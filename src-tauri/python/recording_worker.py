import asyncio
import json
import os
import signal
import sqlite3
import subprocess
import sys
import urllib.request
from datetime import datetime, timezone
from pathlib import Path
from uuid import uuid4
import re


KEY_EXPORT_DIR = "settings.export_dir"
KEY_DOUYIN_COOKIE = "settings.douyin_cookie"
STOP_REQUESTED = False
CURRENT_FFMPEG = None
DEFAULT_SEGMENT_SECONDS = 1800


def now_iso() -> str:
    return datetime.now(timezone.utc).isoformat()


def open_db(db_path: str) -> sqlite3.Connection:
    conn = sqlite3.connect(db_path, timeout=30)
    conn.row_factory = sqlite3.Row
    return conn


def get_meta(db_path: str, key: str) -> str | None:
    conn = open_db(db_path)
    try:
        row = conn.execute("SELECT value FROM app_meta WHERE key = ?", (key,)).fetchone()
        return None if row is None else str(row["value"])
    finally:
        conn.close()


def get_task_payload(db_path: str, task_id: str) -> dict:
    conn = open_db(db_path)
    try:
        row = conn.execute("SELECT input_json FROM tasks WHERE id = ?", (task_id,)).fetchone()
        if row is None:
            raise RuntimeError("recording task not found")
        return json.loads(str(row["input_json"]))
    finally:
        conn.close()


def update_task_status(db_path: str, task_id: str, status: str) -> None:
    conn = open_db(db_path)
    try:
        conn.execute(
            "UPDATE tasks SET status = ?, updated_at = ? WHERE id = ?",
            (status, now_iso(), task_id),
        )
        conn.commit()
    finally:
        conn.close()


def update_task_success(db_path: str, task_id: str, output: dict) -> None:
    conn = open_db(db_path)
    try:
        conn.execute(
            "UPDATE tasks SET status = 'success', output_json = ?, error_text = NULL, updated_at = ? WHERE id = ?",
            (json.dumps(output, ensure_ascii=False), now_iso(), task_id),
        )
        conn.commit()
    finally:
        conn.close()


def update_task_failure(db_path: str, task_id: str, error_text: str) -> None:
    conn = open_db(db_path)
    try:
        conn.execute(
            "UPDATE tasks SET status = 'failed', error_text = ?, updated_at = ? WHERE id = ?",
            (error_text, now_iso(), task_id),
        )
        conn.commit()
    finally:
        conn.close()


def append_task_log(db_path: str, task_id: str, level: str, message: str) -> None:
    conn = open_db(db_path)
    try:
        conn.execute(
            "INSERT INTO task_logs (id, task_id, level, message, ts) VALUES (?, ?, ?, ?, ?)",
            (str(uuid4()), task_id, level, message, now_iso()),
        )
        conn.commit()
    finally:
        conn.close()


def append_account_log(db_path: str, account_id: str, level: str, message: str) -> None:
    conn = open_db(db_path)
    try:
        conn.execute(
            "INSERT INTO recording_account_logs (id, account_id, level, message, ts) VALUES (?, ?, ?, ?, ?)",
            (str(uuid4()), account_id, level, message, now_iso()),
        )
        conn.commit()
    finally:
        conn.close()


def is_account_enabled(db_path: str, account_id: str) -> bool:
    conn = open_db(db_path)
    try:
        row = conn.execute("SELECT enabled FROM recording_accounts WHERE id = ?", (account_id,)).fetchone()
        return bool(row and int(row["enabled"]) != 0)
    finally:
        conn.close()


def mark_account_recording(db_path: str, account_id: str) -> None:
    conn = open_db(db_path)
    try:
        ts = now_iso()
        conn.execute(
            "UPDATE recording_accounts SET status = 'recording', last_checked_at = ?, last_error = NULL, updated_at = ? WHERE id = ?",
            (ts, ts, account_id),
        )
        conn.commit()
    finally:
        conn.close()


def mark_account_finished(db_path: str, account_id: str, error_text: str | None) -> None:
    conn = open_db(db_path)
    try:
        ts = now_iso()
        status = "watching" if is_account_enabled(db_path, account_id) else "idle"
        conn.execute(
            "UPDATE recording_accounts SET status = ?, last_checked_at = ?, last_recorded_at = ?, last_error = ?, updated_at = ? WHERE id = ?",
            (status, ts, ts, error_text, ts, account_id),
        )
        conn.commit()
    finally:
        conn.close()


def sanitize_path(value: str, fallback: str) -> str:
    normalized = re.sub(r'[\\/:*?"<>|\r\n\t]+', "_", value or "").strip(" .")
    normalized = re.sub(r"\s+", " ", normalized)
    if not normalized:
        return fallback
    return normalized[:80]


def resolve_output_root(db_path: str) -> Path:
    export_dir = (get_meta(db_path, KEY_EXPORT_DIR) or "").strip()
    if not export_dir:
        export_dir = str(Path.home() / "Downloads")
    return Path(export_dir).expanduser().resolve() / "DrToolsDownloads"


async def fetch_douyin_live_info(room_id: str, web_rid: str | None, cookie: str) -> dict:
    from f2.apps.douyin.crawler import DouyinCrawler
    from f2.apps.douyin.filter import UserLive2Filter, UserLiveFilter
    from f2.apps.douyin.model import UserLive2, UserLive
    from f2.apps.douyin.utils import ClientConfManager

    kwargs = {
        "cookie": cookie,
        "headers": dict(ClientConfManager.headers()),
        "proxies": dict(ClientConfManager.proxies()),
    }

    async with DouyinCrawler(kwargs) as crawler:
        if room_id:
            response = await crawler.fetch_live_room_id(UserLive2(room_id=room_id))
            live = UserLive2Filter(response)
            stream_map = live.flv_pull_url or live.hls_pull_url or {}
            resolved_web_rid = _first_non_empty(live.web_rid, web_rid)
            return {
                "status": "live" if str(live.live_status) == "2" else "not-live",
                "room_id": _first_non_empty(live.room_id, room_id),
                "web_rid": resolved_web_rid,
                "title": _first_non_empty(live.live_title_raw, live.live_title),
                "nickname": _first_non_empty(live.nickname_raw, live.nickname),
                "cover": _optional_string(live.cover),
                "stream_url": _pick_stream_url(stream_map),
            }

        if not web_rid:
            raise ValueError("room_id or web_rid is required")

        response = await crawler.fetch_live(UserLive(web_rid=web_rid, room_id_str=""))
        live = UserLiveFilter(response)
        stream_map = live.flv_pull_url or live.m3u8_pull_url or {}
        return {
            "status": "live" if str(live.live_status) == "2" else "not-live",
            "room_id": _optional_string(live.room_id),
            "web_rid": web_rid,
            "title": _first_non_empty(live.live_title_raw, live.live_title),
            "nickname": _first_non_empty(live.nickname_raw, live.nickname),
            "cover": _optional_string(live.cover),
            "stream_url": _pick_stream_url(stream_map),
        }


def _pick_stream_url(value) -> str | None:
    if isinstance(value, str):
        return value.strip() or None
    if not isinstance(value, dict):
        return None

    preferred_keys = ["FULL_HD1", "FULL_HD", "HD1", "SD1", "SD2", "ORIGION", "origin"]
    for key in preferred_keys:
        candidate = _optional_string(value.get(key))
        if candidate:
            return candidate

    for candidate in value.values():
        normalized = _optional_string(candidate)
        if normalized:
            return normalized
    return None


def _optional_string(value) -> str | None:
    if value is None:
        return None
    normalized = str(value).strip()
    return normalized or None


def _first_non_empty(*values) -> str | None:
    for value in values:
        normalized = _optional_string(value)
        if normalized:
            return normalized
    return None


def download_cover_if_needed(url: str | None, destination: Path) -> Path | None:
    if not url:
        return None
    try:
        with urllib.request.urlopen(url, timeout=15) as response:
            data = response.read()
        destination.write_bytes(data)
        return destination
    except Exception:
        return None


def build_ffmpeg_command(stream_url: str, output_path: Path, split_recording: bool) -> list[str]:
    ffmpeg_bin = os.environ.get("DRTOOLS_FFMPEG_BIN", "").strip() or "ffmpeg"
    command = [
        ffmpeg_bin,
        "-y",
        "-hide_banner",
        "-nostdin",
        "-loglevel",
        "error",
        "-rw_timeout",
        "15000000",
        "-i",
        stream_url,
        "-c",
        "copy",
    ]
    if split_recording:
        command.extend([
            "-f",
            "segment",
            "-segment_time",
            str(DEFAULT_SEGMENT_SECONDS),
            "-reset_timestamps",
            "1",
            str(output_path),
        ])
        return command

    command.append(str(output_path))
    return command


def wait_process(process: subprocess.Popen[str]) -> int:
    global STOP_REQUESTED
    while True:
        if STOP_REQUESTED:
            try:
                process.terminate()
            except Exception:
                pass
        code = process.poll()
        if code is not None:
            return int(code)
        try:
            process.wait(timeout=1)
        except subprocess.TimeoutExpired:
            continue


def handle_stop_signal(signum, frame):
    global STOP_REQUESTED, CURRENT_FFMPEG
    STOP_REQUESTED = True
    if CURRENT_FFMPEG is not None:
      try:
          CURRENT_FFMPEG.terminate()
      except Exception:
          pass


def run_recording(db_path: str, task_id: str) -> dict:
    global STOP_REQUESTED, CURRENT_FFMPEG
    payload = get_task_payload(db_path, task_id)
    account_id = str(payload.get("accountId", "")).strip()
    account_name = str(payload.get("accountName", "")).strip() or "未知主播"
    platform = str(payload.get("platform", "")).strip()
    room_id = str(payload.get("accountRoomId", "")).strip()
    web_rid = _optional_string(payload.get("accountWebRid"))
    retry_on_disconnect = bool(payload.get("retryOnDisconnect", False))
    split_recording = bool(payload.get("splitRecording", False))
    save_snapshot = bool(payload.get("saveSnapshot", False))

    if platform != "douyin":
        raise ValueError("recording currently supports douyin only")
    if not account_id:
        raise ValueError("accountId is required")
    if not room_id and not web_rid:
        raise ValueError("accountRoomId or accountWebRid is required")

    cookie = (get_meta(db_path, KEY_DOUYIN_COOKIE) or "").strip()
    if not cookie:
        raise ValueError("douyin cookie is required for live recording")

    output_root = resolve_output_root(db_path)
    download_date = datetime.now().astimezone().strftime("%Y-%m-%d")
    account_dir = output_root / "抖音" / "直播录制" / download_date / sanitize_path(account_name, "未知主播")
    account_dir.mkdir(parents=True, exist_ok=True)

    append_task_log(db_path, task_id, "info", f"live recording output directory ready: {account_dir}")
    append_account_log(db_path, account_id, "info", f"直播录制输出目录：{account_dir}")
    mark_account_recording(db_path, account_id)

    live_info = asyncio.run(fetch_douyin_live_info(room_id, web_rid, cookie))
    if live_info.get("status") != "live":
        raise RuntimeError("直播已结束，未能启动录制")

    title = _optional_string(live_info.get("title")) or account_name
    base_name = f"{datetime.now().astimezone().strftime('%Y%m%d_%H%M%S')}_{sanitize_path(title, sanitize_path(account_name, 'live'))}"

    cover_path = None
    if save_snapshot:
        cover_path = download_cover_if_needed(live_info.get("cover"), account_dir / f"{base_name}_cover.jpg")
        if cover_path is not None:
            append_task_log(db_path, task_id, "info", f"cover snapshot saved: {cover_path.name}")

    if split_recording:
        output_path = account_dir / f"{base_name}_%03d.flv"
        reported_output_path = str(account_dir)
        append_task_log(db_path, task_id, "info", "split recording enabled with default 30 minute segments")
    else:
        output_path = account_dir / f"{base_name}.flv"
        reported_output_path = str(output_path)

    attempt = 0
    while True:
        if STOP_REQUESTED:
            raise InterruptedError("recording interrupted by stop request")

        stream_url = _optional_string(live_info.get("stream_url"))
        if not stream_url:
            raise RuntimeError("未解析到可用直播流地址")

        append_task_log(db_path, task_id, "info", f"ffmpeg recording started (attempt {attempt + 1})")
        append_account_log(db_path, account_id, "info", f"开始录制直播流，第 {attempt + 1} 次尝试。")

        command = build_ffmpeg_command(stream_url, output_path, split_recording)
        process = subprocess.Popen(command, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, text=True)
        CURRENT_FFMPEG = process
        exit_code = wait_process(process)
        CURRENT_FFMPEG = None

        if STOP_REQUESTED:
            raise InterruptedError("recording interrupted by stop request")

        latest_info = asyncio.run(fetch_douyin_live_info(room_id, web_rid, cookie))
        if exit_code == 0 and latest_info.get("status") != "live":
            append_task_log(db_path, task_id, "success", "live recording completed")
            append_account_log(db_path, account_id, "success", "直播录制已完成。")
            return {
                "taskType": "recording.live",
                "platform": platform,
                "accountId": account_id,
                "accountName": account_name,
                "outputPath": reported_output_path,
                "coverPath": str(cover_path) if cover_path else None,
                "processedAt": now_iso(),
                "splitRecording": split_recording,
                "message": "live recording completed",
            }

        if latest_info.get("status") != "live":
            append_task_log(db_path, task_id, "warning", "live ended after stream interruption")
            append_account_log(db_path, account_id, "warning", "直播已结束，录制流程收尾完成。")
            return {
                "taskType": "recording.live",
                "platform": platform,
                "accountId": account_id,
                "accountName": account_name,
                "outputPath": reported_output_path,
                "coverPath": str(cover_path) if cover_path else None,
                "processedAt": now_iso(),
                "splitRecording": split_recording,
                "message": "live recording finished after stream ended",
            }

        if not retry_on_disconnect or attempt >= 2:
            raise RuntimeError(f"ffmpeg exited unexpectedly with code {exit_code}")

        attempt += 1
        live_info = latest_info
        append_task_log(db_path, task_id, "warning", f"stream interrupted, retrying recording (attempt {attempt + 1})")
        append_account_log(db_path, account_id, "warning", f"直播流中断，准备进行第 {attempt + 1} 次重试。")


def main() -> int:
    if len(sys.argv) != 3:
        raise SystemExit("usage: recording_worker.py <db_path> <task_id>")

    signal.signal(signal.SIGTERM, handle_stop_signal)
    signal.signal(signal.SIGINT, handle_stop_signal)

    db_path = sys.argv[1]
    task_id = sys.argv[2]
    payload = get_task_payload(db_path, task_id)
    account_id = str(payload.get("accountId", "")).strip()

    try:
        output = run_recording(db_path, task_id)
        update_task_success(db_path, task_id, output)
        mark_account_finished(db_path, account_id, None)
        return 0
    except InterruptedError as exc:
        message = str(exc) or "recording interrupted"
        update_task_failure(db_path, task_id, message)
        append_task_log(db_path, task_id, "warning", message)
        if account_id:
            append_account_log(db_path, account_id, "warning", "录制任务已被中断。")
            mark_account_finished(db_path, account_id, message)
        return 1
    except Exception as exc:
        message = f"{exc.__class__.__name__}: {exc}"
        update_task_failure(db_path, task_id, message)
        append_task_log(db_path, task_id, "error", message)
        if account_id:
            append_account_log(db_path, account_id, "error", f"直播录制失败：{message}")
            mark_account_finished(db_path, account_id, message)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
