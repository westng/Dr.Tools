import asyncio
import json
import re
import shutil
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path


VIDEO_EXTENSIONS = {".mp4", ".mov", ".m4v", ".webm", ".mkv"}
IMAGE_EXTENSIONS = {".jpg", ".jpeg", ".png", ".webp"}


def run_video_download(payload: dict) -> dict:
    task_id = str(payload.get("taskId", "")).strip()
    platform = str(payload.get("platform", "")).strip()
    source_url = str(payload.get("sourceUrl", "")).strip()
    output_dir = str(payload.get("outputDir", "")).strip()
    cookie = str(payload.get("cookie", "") or "").strip()
    download_cover = bool(payload.get("downloadCover", False))
    download_music = bool(payload.get("downloadMusic", False))
    download_description = bool(payload.get("downloadDescription", False))
    download_lyric = bool(payload.get("downloadLyric", False))

    if platform not in {"douyin", "tiktok"}:
        raise ValueError("platform must be douyin or tiktok")

    if not source_url.startswith(("http://", "https://")):
        raise ValueError("sourceUrl must be an http or https url")

    if not output_dir:
        raise ValueError("outputDir is required")

    if platform != "douyin":
        download_lyric = False

    output_root = Path(output_dir).expanduser().resolve() / "DrToolsDownloads"
    temp_dir = output_root / ".tmp" / platform / (task_id or "manual")
    temp_dir.mkdir(parents=True, exist_ok=True)

    app_code = "dy" if platform == "douyin" else "tk"
    command = [
        sys.executable,
        "-m",
        "f2",
        app_code,
        "-M",
        "one",
        "-u",
        source_url,
        "-p",
        str(temp_dir),
        "-f",
        "False",
        "-n",
        "{aweme_id}",
        "-m",
        "True" if download_music else "False",
        "-d",
        "True" if download_description else "False",
        "-v",
        "True" if download_cover else "False",
        "-l",
        "en_US",
    ]
    if platform == "douyin":
        command.extend(["-L", "True" if download_lyric else "False"])
    if cookie:
        command.extend(["-k", cookie])

    metadata_command = list(command)
    if cookie:
        metadata_command[-1] = "***"

    completed = subprocess.run(
        command,
        cwd=str(temp_dir),
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        timeout=600,
        check=False,
    )

    stdout = completed.stdout.strip()
    stderr = completed.stderr.strip()

    if completed.returncode != 0:
        message = stderr or stdout or f"f2 exited with code {completed.returncode}"
        raise RuntimeError(message)

    video_path = _find_first_file(temp_dir, VIDEO_EXTENSIONS)
    cover_path = _find_first_file(temp_dir, IMAGE_EXTENSIONS) if download_cover else None

    if video_path is None:
        message = stderr or stdout or "f2 completed but no video file was produced"
        raise RuntimeError(message)

    author_name = _resolve_author_name(video_path, platform)
    resolved_author_name, author_uid = _resolve_author_profile(
        source_url=source_url,
        platform=platform,
        cookie=cookie,
    )
    if resolved_author_name:
        author_name = resolved_author_name
    download_date = datetime.now().astimezone().strftime("%Y-%m-%d")
    final_dir = output_root / _platform_display_name(platform) / "视频" / download_date / _safe_path_name(author_name)
    final_dir.mkdir(parents=True, exist_ok=True)

    final_video_path = _move_to_final_path(video_path, final_dir)
    final_cover_path = _move_to_final_path(cover_path, final_dir) if cover_path else None

    metadata = {
        "taskId": task_id,
        "platform": platform,
        "sourceUrl": source_url,
        "authorName": author_name,
        "authorUid": author_uid,
        "downloadDate": download_date,
        "downloadCover": download_cover,
        "downloadMusic": download_music,
        "downloadDescription": download_description,
        "downloadLyric": download_lyric,
        "outputPath": str(final_video_path),
        "coverPath": str(final_cover_path) if final_cover_path else None,
        "executedAt": datetime.now(timezone.utc).isoformat(),
        "stdout": stdout,
        "stderr": stderr,
        "command": metadata_command,
        "tempDir": str(temp_dir),
    }

    metadata_path = final_dir / f"{final_video_path.stem}.metadata.json"
    metadata_path.write_text(json.dumps(metadata, ensure_ascii=False, indent=2), encoding="utf-8")
    shutil.rmtree(temp_dir, ignore_errors=True)

    return {
        "task_type": "video.download",
        "platform": platform,
        "sourceUrl": source_url,
        "downloadCover": download_cover,
        "downloadMusic": download_music,
        "downloadDescription": download_description,
        "downloadLyric": download_lyric,
        "authorName": author_name,
        "authorUid": author_uid,
        "outputPath": str(final_video_path),
        "metadataPath": str(metadata_path),
        "coverPath": str(final_cover_path) if final_cover_path else None,
        "processedAt": metadata["executedAt"],
        "message": "video download completed via f2 CLI",
        "stdout": stdout,
    }


def _find_first_file(root: Path, allowed_suffixes: set[str]) -> Path | None:
    files = sorted(
        (
            path
            for path in root.rglob("*")
            if path.is_file() and path.suffix.lower() in allowed_suffixes
        ),
        key=lambda path: path.stat().st_mtime,
        reverse=True,
    )
    return files[0] if files else None


def _resolve_author_name(video_path: Path, platform: str) -> str:
    parent_name = video_path.parent.name.strip()
    if not parent_name or parent_name in {platform, "one"}:
        return "未知用户"
    return parent_name


def _resolve_author_profile(source_url: str, platform: str, cookie: str) -> tuple[str | None, str | None]:
    try:
        return asyncio.run(
            asyncio.wait_for(
                _fetch_author_profile(source_url=source_url, platform=platform, cookie=cookie),
                timeout=8,
            )
        )
    except Exception:
        return None, None


async def _fetch_author_profile(source_url: str, platform: str, cookie: str) -> tuple[str | None, str | None]:
    if platform == "douyin":
        from f2.apps.douyin.crawler import DouyinCrawler
        from f2.apps.douyin.filter import PostDetailFilter
        from f2.apps.douyin.model import PostDetail
        from f2.apps.douyin.utils import AwemeIdFetcher, ClientConfManager

        kwargs = _build_f2_request_kwargs(
            cookie=cookie,
            headers=ClientConfManager.headers(),
            proxies=ClientConfManager.proxies(),
        )
        aweme_id = await AwemeIdFetcher.get_aweme_id(source_url)
        async with DouyinCrawler(kwargs) as crawler:
            response = await crawler.fetch_post_detail(PostDetail(aweme_id=aweme_id))
        video = PostDetailFilter(response)
        return (
            _first_non_empty(getattr(video, "nickname", None), getattr(video, "nickname_raw", None)),
            _first_non_empty(
                getattr(video, "uid", None),
                getattr(video, "sec_user_id", None),
                getattr(video, "unique_id", None),
            ),
        )

    if platform == "tiktok":
        from f2.apps.tiktok.crawler import TiktokCrawler
        from f2.apps.tiktok.filter import PostDetailFilter
        from f2.apps.tiktok.model import PostDetail
        from f2.apps.tiktok.utils import AwemeIdFetcher, ClientConfManager

        kwargs = _build_f2_request_kwargs(
            cookie=cookie,
            headers=ClientConfManager.headers(),
            proxies=ClientConfManager.proxies(),
        )
        aweme_id = await AwemeIdFetcher.get_aweme_id(source_url)
        async with TiktokCrawler(kwargs) as crawler:
            response = await crawler.fetch_post_detail(PostDetail(itemId=aweme_id))
        video = PostDetailFilter(response)
        return (
            _first_non_empty(getattr(video, "nickname", None), getattr(video, "nickname_raw", None)),
            _first_non_empty(
                getattr(video, "uid", None),
                getattr(video, "secUid", None),
                getattr(video, "uniqueId", None),
            ),
        )

    return None, None


def _build_f2_request_kwargs(cookie: str, headers: dict | None, proxies: dict | None) -> dict:
    request_headers = dict(headers or {})
    return {
        "cookie": cookie,
        "headers": request_headers,
        "proxies": proxies or {"http://": None, "https://": None},
    }


def _first_non_empty(*values: object) -> str | None:
    for value in values:
        if isinstance(value, str):
            normalized = value.strip()
            if normalized:
                return normalized
    return None


def _platform_display_name(platform: str) -> str:
    if platform == "douyin":
        return "抖音"
    if platform == "tiktok":
        return "TikTok"
    return platform


def _safe_path_name(value: str) -> str:
    normalized = re.sub(r'[\\/:*?"<>|\r\n\t]+', "_", value).strip(" .")
    normalized = re.sub(r"\s+", " ", normalized)
    if not normalized:
        return "未知用户"
    return normalized[:80]


def _move_to_final_path(source: Path | None, final_dir: Path) -> Path | None:
    if source is None:
        return None

    safe_name = _safe_file_name(source.name)
    destination = _unique_destination(final_dir / safe_name)
    shutil.move(str(source), str(destination))
    return destination


def _safe_file_name(value: str) -> str:
    name = re.sub(r'[\\/:*?"<>|\r\n\t]+', "_", value).strip(" .")
    if not name:
        return "unnamed"
    return name[:180]


def _unique_destination(path: Path) -> Path:
    if not path.exists():
        return path

    stem = path.stem
    suffix = path.suffix
    counter = 2
    while True:
        candidate = path.with_name(f"{stem}_{counter}{suffix}")
        if not candidate.exists():
            return candidate
        counter += 1
