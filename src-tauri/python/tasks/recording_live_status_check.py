from datetime import datetime, timezone
import asyncio


def run_recording_live_status_check(payload: dict) -> dict:
    platform = str(payload.get("platform", "")).strip()
    cookie = str(payload.get("cookie", "") or "").strip()

    if platform not in {"douyin", "tiktok"}:
        raise ValueError("platform must be douyin or tiktok")

    if not cookie:
        raise ValueError("cookie is required")

    if platform == "douyin":
        return asyncio.run(_check_douyin_live_status(payload, cookie))

    return asyncio.run(_check_tiktok_live_status(payload, cookie))


async def _check_douyin_live_status(payload: dict, cookie: str) -> dict:
    from f2.apps.douyin.crawler import DouyinCrawler
    from f2.apps.douyin.filter import UserLive2Filter, UserProfileFilter
    from f2.apps.douyin.model import UserLive2, UserProfile
    from f2.apps.douyin.utils import ClientConfManager, SecUserIdFetcher

    sec_user_id = _optional_string(payload.get("accountSecUserId"))
    source_url = _optional_string(payload.get("sourceUrl"))
    if not sec_user_id:
        if not source_url:
            raise ValueError("douyin accountSecUserId or sourceUrl is required")
        sec_user_id = await SecUserIdFetcher.get_sec_user_id(source_url)

    kwargs = _build_f2_request_kwargs(
        cookie=cookie,
        headers=ClientConfManager.headers(),
        proxies=ClientConfManager.proxies(),
    )

    async with DouyinCrawler(kwargs) as crawler:
        profile_response = await crawler.fetch_user_profile(UserProfile(sec_user_id=sec_user_id))

    profile = UserProfileFilter(profile_response)
    room_id = _optional_string(profile.room_id)
    live_status = _optional_string(profile.live_status)
    web_rid = _optional_string(payload.get("accountWebRid"))
    live_title = None

    if room_id:
        try:
            async with DouyinCrawler(kwargs) as crawler:
                live_response = await crawler.fetch_live_room_id(UserLive2(room_id=room_id))
            live = UserLive2Filter(live_response)
            room_id = _first_non_empty(room_id, live.room_id)
            web_rid = _first_non_empty(_optional_string(live.web_rid), web_rid)
            live_title = _first_non_empty(live.live_title_raw, live.live_title)
            live_status = _first_non_empty(_optional_string(live.live_status), live_status)
        except Exception:
            pass

    return {
        "platform": "douyin",
        "status": "live" if str(live_status) == "2" else "not-live",
        "accountRoomId": room_id,
        "accountWebRid": web_rid,
        "liveTitle": live_title,
        "checkedAt": _now_iso(),
        "errorMessage": None,
    }


async def _check_tiktok_live_status(payload: dict, cookie: str) -> dict:
    from f2.apps.tiktok.crawler import TiktokCrawler
    from f2.apps.tiktok.filter import UserLiveFilter
    from f2.apps.tiktok.model import UserLive
    from f2.apps.tiktok.utils import ClientConfManager, SecUserIdFetcher

    unique_id = _optional_string(payload.get("accountUniqueId"))
    source_url = _optional_string(payload.get("sourceUrl"))
    if not unique_id:
        if not source_url:
            raise ValueError("tiktok accountUniqueId or sourceUrl is required")
        unique_id = await SecUserIdFetcher.get_uniqueid(source_url)

    kwargs = _build_f2_request_kwargs(
        cookie=cookie,
        headers=ClientConfManager.headers(),
        proxies=ClientConfManager.proxies(),
    )

    async with TiktokCrawler(kwargs) as crawler:
        live_response = await crawler.fetch_user_live(UserLive(uniqueId=unique_id))

    live = UserLiveFilter(live_response)
    has_live = bool(live.has_live)
    live_status = _optional_string(live.live_status)

    return {
        "platform": "tiktok",
        "status": "live" if has_live and str(live_status) == "2" else "not-live",
        "accountRoomId": _optional_string(live.live_room_id),
        "accountWebRid": None,
        "liveTitle": _first_non_empty(live.live_title_raw, live.live_title),
        "checkedAt": _now_iso(),
        "errorMessage": None,
    }


def _build_f2_request_kwargs(cookie: str, headers: dict | None = None, proxies: dict | None = None) -> dict:
    return {
        "cookie": cookie,
        "headers": dict(headers or {}),
        "proxies": dict(proxies or {"http://": None, "https://": None}),
    }


def _first_non_empty(*values):
    for value in values:
        normalized = _optional_string(value)
        if normalized:
            return normalized
    return None


def _optional_string(value) -> str | None:
    if value is None:
        return None

    normalized = str(value).strip()
    return normalized or None


def _now_iso() -> str:
    return datetime.now(timezone.utc).isoformat()
