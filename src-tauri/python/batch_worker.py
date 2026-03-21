import json
import os
import sqlite3
import subprocess
import sys
from concurrent.futures import ThreadPoolExecutor, as_completed
from contextlib import contextmanager
from datetime import datetime, timezone
from pathlib import Path
from uuid import uuid4

from tasks.video_download import run_video_download


KEY_EXPORT_DIR = "settings.export_dir"
KEY_MAX_CONCURRENT_DOWNLOADS = "settings.max_concurrent_downloads"
KEY_DOWNLOAD_NOTIFICATIONS_ENABLED = "settings.download_notifications_enabled"
KEY_DOUYIN_COOKIE = "settings.douyin_cookie"
KEY_TIKTOK_COOKIE = "settings.tiktok_cookie"


def now_iso() -> str:
    return datetime.now(timezone.utc).isoformat()


def open_db(db_path: str) -> sqlite3.Connection:
    conn = sqlite3.connect(db_path, timeout=30)
    conn.row_factory = sqlite3.Row
    return conn


def append_runtime_log(app_dir: Path, message: str) -> None:
    path = app_dir / "batch-worker.log"
    with path.open("a", encoding="utf-8") as handle:
        handle.write(f"{now_iso()} {message}\n")


def get_meta(db_path: str, key: str) -> str | None:
    conn = open_db(db_path)
    try:
        row = conn.execute("SELECT value FROM app_meta WHERE key = ?", (key,)).fetchone()
        return None if row is None else str(row["value"])
    finally:
        conn.close()


def read_bool(value: str | None, default: bool) -> bool:
    if value is None:
        return default
    normalized = value.strip().lower()
    if normalized in {"true", "1", "yes", "on"}:
        return True
    if normalized in {"false", "0", "no", "off"}:
        return False
    return default


def read_int(value: str | None, default: int) -> int:
    if value is None:
        return default
    try:
        parsed = int(value.strip())
    except ValueError:
        return default
    return parsed if parsed > 0 else default


def list_queued_tasks(db_path: str, batch_id: str) -> list[dict]:
    conn = open_db(db_path)
    try:
        rows = conn.execute(
            """
            SELECT id, input_json
            FROM tasks
            WHERE task_type = 'video.download' AND status = 'queued'
            ORDER BY created_at ASC
            """
        ).fetchall()
    finally:
        conn.close()

    tasks: list[dict] = []
    for row in rows:
        try:
            payload = json.loads(row["input_json"])
        except json.JSONDecodeError:
            continue
        if str(payload.get("batchId", "")).strip() != batch_id:
            continue
        tasks.append(
            {
                "taskId": row["id"],
                "platform": str(payload.get("platform", "")).strip(),
                "sourceUrl": str(payload.get("sourceUrl", "")).strip(),
                "downloadCover": bool(payload.get("downloadCover", False)),
                "downloadMusic": bool(payload.get("downloadMusic", False)),
                "downloadDescription": bool(payload.get("downloadDescription", False)),
                "downloadLyric": bool(payload.get("downloadLyric", False)),
            }
        )
    return tasks


def reclaim_running_tasks(db_path: str, batch_id: str) -> int:
    conn = open_db(db_path)
    try:
        rows = conn.execute(
            """
            SELECT id, input_json
            FROM tasks
            WHERE task_type = 'video.download' AND status = 'running'
            ORDER BY created_at ASC
            """
        ).fetchall()

        recovered_ids: list[str] = []
        for row in rows:
            try:
                payload = json.loads(row["input_json"])
            except json.JSONDecodeError:
                continue
            if str(payload.get("batchId", "")).strip() != batch_id:
                continue
            recovered_ids.append(str(row["id"]))

        now = now_iso()
        for task_id in recovered_ids:
            conn.execute(
                "UPDATE tasks SET status = 'queued', updated_at = ? WHERE id = ?",
                (now, task_id),
            )
            conn.execute(
                "INSERT INTO task_logs (id, task_id, level, message, ts) VALUES (?, ?, ?, ?, ?)",
                (str(uuid4()), task_id, "warning", "task recovered from stale running state", now),
            )

        conn.commit()
        return len(recovered_ids)
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
            "UPDATE tasks SET status = ?, output_json = ?, error_text = NULL, updated_at = ? WHERE id = ?",
            ("success", json.dumps(output, ensure_ascii=False), now_iso(), task_id),
        )
        conn.commit()
    finally:
        conn.close()


def update_task_failure(db_path: str, task_id: str, error_text: str) -> None:
    conn = open_db(db_path)
    try:
        conn.execute(
            "UPDATE tasks SET status = ?, error_text = ?, updated_at = ? WHERE id = ?",
            ("failed", error_text, now_iso(), task_id),
        )
        conn.commit()
    finally:
        conn.close()


def append_log(db_path: str, task_id: str, level: str, message: str) -> None:
    conn = open_db(db_path)
    try:
        conn.execute(
            "INSERT INTO task_logs (id, task_id, level, message, ts) VALUES (?, ?, ?, ?, ?)",
            (str(uuid4()), task_id, level, message, now_iso()),
        )
        conn.commit()
    finally:
        conn.close()


def finish_batch_task(db_path: str, batch_id: str, succeeded: bool) -> dict | None:
    conn = open_db(db_path)
    try:
        conn.execute("BEGIN IMMEDIATE")
        row = conn.execute(
            "SELECT platform, total_count, success_count, failed_count, completion_handled FROM download_batches WHERE id = ?",
            (batch_id,),
        ).fetchone()
        if row is None:
            conn.commit()
            return None

        success_increment = 1 if succeeded else 0
        failed_increment = 0 if succeeded else 1
        now = now_iso()
        conn.execute(
            """
            UPDATE download_batches
            SET success_count = success_count + ?,
                failed_count = failed_count + ?,
                updated_at = ?
            WHERE id = ?
            """,
            (success_increment, failed_increment, now, batch_id),
        )
        updated = conn.execute(
            "SELECT platform, total_count, success_count, failed_count, completion_handled FROM download_batches WHERE id = ?",
            (batch_id,),
        ).fetchone()
        assert updated is not None
        total_count = int(updated["total_count"])
        success_count = int(updated["success_count"])
        failed_count = int(updated["failed_count"])
        completion_handled = int(updated["completion_handled"])
        is_complete = success_count + failed_count >= total_count

        summary = None
        if is_complete and completion_handled == 0:
            conn.execute(
                """
                UPDATE download_batches
                SET completion_handled = 1,
                    completed_at = COALESCE(completed_at, ?),
                    updated_at = ?
                WHERE id = ?
                """,
                (now, now, batch_id),
            )
            summary = {
                "batchId": batch_id,
                "platform": str(updated["platform"]),
                "totalCount": total_count,
                "successCount": success_count,
                "failedCount": failed_count,
            }

        conn.commit()
        return summary
    finally:
        conn.close()


def platform_label(platform: str) -> str:
    if platform == "douyin":
        return "抖音"
    if platform == "tiktok":
        return "TikTok"
    return platform


def send_batch_notification(summary: dict) -> None:
    platform = platform_label(str(summary["platform"]))
    success_count = int(summary["successCount"])
    failed_count = int(summary["failedCount"])
    total_count = int(summary["totalCount"])

    if success_count == 0:
        title = "批次下载失败"
        body = f"{platform} · 全部失败，共 {failed_count} 条"
    elif failed_count == 0:
        title = "批次下载完成"
        body = f"{platform} · 全部成功，共 {success_count} 条"
    else:
        title = "批次下载完成"
        body = f"{platform} · 成功 {success_count} 条，失败 {failed_count} 条，共 {total_count} 条"

    if sys.platform == "darwin":
        script = f'display notification "{body.replace(chr(34), chr(92) + chr(34))}" with title "{title}"'
        subprocess.Popen(["osascript", "-e", script], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)


def build_task_payload(db_path: str, task: dict) -> dict:
    platform = task["platform"]
    cookie_key = KEY_DOUYIN_COOKIE if platform == "douyin" else KEY_TIKTOK_COOKIE
    export_dir = get_meta(db_path, KEY_EXPORT_DIR)
    if not export_dir:
        export_dir = str(Path.home() / "Downloads")

    return {
        "taskId": task["taskId"],
        "platform": platform,
        "sourceUrl": task["sourceUrl"],
        "downloadCover": task["downloadCover"],
        "downloadMusic": task["downloadMusic"],
        "downloadDescription": task["downloadDescription"],
        "downloadLyric": task["downloadLyric"],
        "outputDir": export_dir,
        "cookie": get_meta(db_path, cookie_key) or "",
    }


def process_task(db_path: str, batch_id: str, app_dir: Path, task: dict, notifications_enabled: bool) -> None:
    task_id = task["taskId"]
    append_runtime_log(app_dir, f"worker task start task_id={task_id}")
    update_task_status(db_path, task_id, "running")
    append_log(db_path, task_id, "info", "video download task running")

    try:
        output = run_video_download(build_task_payload(db_path, task))
        update_task_success(db_path, task_id, output)
        append_log(db_path, task_id, "info", "video download task success")
        summary = finish_batch_task(db_path, batch_id, True)
    except Exception as exc:
        message = f"{exc.__class__.__name__}: {exc}"
        update_task_failure(db_path, task_id, message)
        append_log(db_path, task_id, "error", f"video download task failed: {message}")
        summary = finish_batch_task(db_path, batch_id, False)

    if notifications_enabled and summary is not None:
        send_batch_notification(summary)


@contextmanager
def batch_lock(app_dir: Path, batch_id: str):
    lock_dir = app_dir / "batch-worker-locks"
    lock_dir.mkdir(parents=True, exist_ok=True)
    lock_path = lock_dir / f"{batch_id}.lock"
    handle = None
    try:
        handle = os.open(lock_path, os.O_CREAT | os.O_EXCL | os.O_WRONLY)
        os.write(handle, str(os.getpid()).encode("utf-8"))
        yield True
    except FileExistsError:
        yield False
    finally:
        if handle is not None:
            os.close(handle)
            try:
                lock_path.unlink()
            except FileNotFoundError:
                pass


def main() -> int:
    if len(sys.argv) != 3:
        raise SystemExit("usage: batch_worker.py <db_path> <batch_id>")

    db_path = sys.argv[1]
    batch_id = sys.argv[2]
    app_dir = Path(db_path).resolve().parent

    with batch_lock(app_dir, batch_id) as acquired:
        if not acquired:
            append_runtime_log(app_dir, f"worker skipped batch_id={batch_id} reason=lock-exists")
            return 0

        append_runtime_log(app_dir, f"worker start batch_id={batch_id}")
        recovered = reclaim_running_tasks(db_path, batch_id)
        if recovered > 0:
            append_runtime_log(app_dir, f"worker recovered stale running tasks batch_id={batch_id} count={recovered}")
        tasks = list_queued_tasks(db_path, batch_id)
        if not tasks:
            append_runtime_log(app_dir, f"worker stop batch_id={batch_id} reason=no-queued-tasks")
            return 0

        notifications_enabled = read_bool(get_meta(db_path, KEY_DOWNLOAD_NOTIFICATIONS_ENABLED), True)
        max_workers = read_int(get_meta(db_path, KEY_MAX_CONCURRENT_DOWNLOADS), 3)
        max_workers = max(1, max_workers)

        with ThreadPoolExecutor(max_workers=max_workers) as executor:
            futures = [
                executor.submit(process_task, db_path, batch_id, app_dir, task, notifications_enabled)
                for task in tasks
            ]
            for future in as_completed(futures):
                future.result()

        append_runtime_log(app_dir, f"worker completed batch_id={batch_id}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
