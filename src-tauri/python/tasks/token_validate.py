import asyncio
import json
from datetime import datetime, timezone

import httpx


DOUYIN_AUTH_KEYS = ("sessionid", "sessionid_ss", "passport_csrf_token", "uid_tt")
TIKTOK_AUTH_KEYS = ("sessionid", "sessionid_ss", "sid_tt", "uid_tt", "passport_csrf_token")


def run_token_validate(payload: dict) -> dict:
    platform = str(payload.get("platform", "")).strip()
    cookie = str(payload.get("cookie", "")).strip()
    checked_at = datetime.now(timezone.utc).isoformat()

    if platform not in {"douyin", "tiktok"}:
        raise ValueError("platform must be douyin or tiktok")

    if not cookie:
        return {
            "platform": platform,
            "checkedAt": checked_at,
            "status": "invalid",
            "message": "Cookie 不能为空",
        }

    if platform == "douyin":
        status, message = _validate_douyin_cookie(cookie)
    else:
        status, message = _validate_tiktok_cookie(cookie)

    return {
        "platform": platform,
        "checkedAt": checked_at,
        "status": status,
        "message": message,
    }


def _validate_douyin_cookie(cookie: str) -> tuple[str, str]:
    if not _has_auth_key(cookie, DOUYIN_AUTH_KEYS):
        return "invalid", "Cookie 缺少抖音登录关键字段"

    try:
        response = asyncio.run(_validate_douyin_cookie_async(cookie))
    except Exception as exc:
        message = str(exc).strip() or exc.__class__.__name__
        lowered = message.lower()
        if "login" in lowered or "cookie" in lowered or "unauthorized" in lowered:
            return "expired", f"Cookie 可能已失效：{message}"
        return "unchecked", f"校验失败，请稍后重试：{message}"

    status_code = _read_nested(response, ["status_code"], 0)
    status_msg = str(_read_nested(response, ["status_msg"], "") or "")
    response_text = json.dumps(response, ensure_ascii=False)

    if status_code == 0:
        return "valid", "Cookie 校验成功，可用于抖音请求"

    lowered = f"{status_msg} {response_text}".lower()
    if "login" in lowered or "expired" in lowered or "登录" in response_text or "失效" in response_text:
        return "expired", status_msg or "Cookie 已过期，请重新更新"

    return "invalid", status_msg or "Cookie 校验未通过"


async def _validate_douyin_cookie_async(cookie: str) -> dict:
    from f2.apps.douyin.crawler import DouyinCrawler
    from f2.apps.douyin.model import UserCollection

    kwargs = {"cookie": cookie}
    async with DouyinCrawler(kwargs) as crawler:
        params = UserCollection(cursor=0, count=1)
        return await crawler.fetch_user_collection(params)


def _validate_tiktok_cookie(cookie: str) -> tuple[str, str]:
    if not _has_auth_key(cookie, TIKTOK_AUTH_KEYS):
        return "invalid", "Cookie 缺少 TikTok 登录关键字段"

    headers = {
        "Cookie": cookie,
        "User-Agent": (
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) "
            "AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36"
        ),
        "Referer": "https://www.tiktok.com/",
    }

    try:
        with httpx.Client(headers=headers, follow_redirects=True, timeout=15.0) as client:
            response = client.get("https://www.tiktok.com/")
    except Exception as exc:
        message = str(exc).strip() or exc.__class__.__name__
        return "unchecked", f"校验失败，请稍后重试：{message}"

    final_url = str(response.url)
    body = response.text[:4000]
    lowered = body.lower()

    if "/login" in final_url or "login-context" in lowered:
        return "expired", "Cookie 可能已失效，请重新登录后更新"

    if response.status_code == 200:
        return "valid", "Cookie 基础请求成功，可继续用于 TikTok 下载"

    return "unchecked", f"校验失败，请稍后重试：HTTP {response.status_code}"


def _has_auth_key(cookie: str, keys: tuple[str, ...]) -> bool:
    lowered = cookie.lower()
    return any(f"{key.lower()}=" in lowered for key in keys)


def _read_nested(data: dict, path: list[str], default):
    current = data
    for key in path:
        if not isinstance(current, dict) or key not in current:
            return default
        current = current[key]
    return current
