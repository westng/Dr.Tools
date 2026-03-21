import asyncio


def run_recording_account_resolve(payload: dict) -> dict:
    platform = str(payload.get("platform", "")).strip()
    source_url = str(payload.get("sourceUrl", "")).strip()
    cookie = str(payload.get("cookie", "") or "").strip()

    if platform not in {"douyin", "tiktok"}:
        raise ValueError("platform must be douyin or tiktok")

    if not source_url.startswith(("http://", "https://")):
        raise ValueError("sourceUrl must be an http or https url")

    if not cookie:
        raise ValueError("cookie is required")

    if platform == "douyin":
        return asyncio.run(_resolve_douyin_account(source_url, cookie))

    return asyncio.run(_resolve_tiktok_account(source_url, cookie))


async def _resolve_douyin_account(source_url: str, cookie: str) -> dict:
    from f2.apps.douyin.crawler import DouyinCrawler
    from f2.apps.douyin.filter import UserLive2Filter, UserProfileFilter
    from f2.apps.douyin.model import UserLive2, UserProfile
    from f2.apps.douyin.utils import ClientConfManager, SecUserIdFetcher

    sec_user_id = await SecUserIdFetcher.get_sec_user_id(source_url)
    kwargs = _build_f2_request_kwargs(
        cookie=cookie,
        headers=ClientConfManager.headers(),
        proxies=ClientConfManager.proxies(),
    )

    async with DouyinCrawler(kwargs) as crawler:
        response = await crawler.fetch_user_profile(UserProfile(sec_user_id=sec_user_id))

    user = UserProfileFilter(response)
    account_name = _first_non_empty(user.nickname, user.nickname_raw)
    account_uid = _first_non_empty(user.uid, user.sec_user_id, user.unique_id)
    account_room_id = _optional_string(user.room_id)
    account_web_rid = None

    if not account_name or not account_uid:
        raise RuntimeError("failed to resolve douyin account profile")

    if account_room_id:
        try:
            async with DouyinCrawler(kwargs) as crawler:
                live_response = await crawler.fetch_live_room_id(UserLive2(room_id=account_room_id))
            live = UserLive2Filter(live_response)
            account_web_rid = _optional_string(live.web_rid)
            account_room_id = _first_non_empty(account_room_id, live.room_id)
        except Exception:
            account_web_rid = None

    return {
        "platform": "douyin",
        "accountInput": source_url,
        "accountName": account_name,
        "accountUid": str(account_uid),
        "accountAvatarUrl": _optional_string(user.avatar_url),
        "accountRoomId": account_room_id,
        "accountWebRid": account_web_rid,
        "accountSecUserId": _optional_string(user.sec_user_id),
        "accountUniqueId": _optional_string(user.unique_id),
    }


async def _resolve_tiktok_account(source_url: str, cookie: str) -> dict:
    from f2.apps.tiktok.crawler import TiktokCrawler
    from f2.apps.tiktok.filter import UserLiveFilter, UserProfileFilter
    from f2.apps.tiktok.model import UserLive, UserProfile
    from f2.apps.tiktok.utils import ClientConfManager, SecUserIdFetcher

    sec_uid = await SecUserIdFetcher.get_secuid(source_url)
    unique_id = await SecUserIdFetcher.get_uniqueid(source_url)
    kwargs = _build_f2_request_kwargs(
      cookie=cookie,
      headers=ClientConfManager.headers(),
      proxies=ClientConfManager.proxies(),
    )

    async with TiktokCrawler(kwargs) as crawler:
        response = await crawler.fetch_user_profile(UserProfile(secUid=sec_uid, uniqueId=unique_id))

    user = UserProfileFilter(response)
    account_name = _first_non_empty(user.nickname, user.nickname_raw)
    account_uid = _first_non_empty(user.uid, user.secUid, user.uniqueId)
    account_room_id = None

    if not account_name or not account_uid:
        raise RuntimeError("failed to resolve tiktok account profile")

    resolved_unique_id = _first_non_empty(unique_id, user.uniqueId)
    if resolved_unique_id:
        try:
            async with TiktokCrawler(kwargs) as crawler:
                live_response = await crawler.fetch_user_live(UserLive(uniqueId=resolved_unique_id))
            live = UserLiveFilter(live_response)
            account_room_id = _optional_string(live.live_room_id)
        except Exception:
            account_room_id = None

    return {
        "platform": "tiktok",
        "accountInput": source_url,
        "accountName": account_name,
        "accountUid": str(account_uid),
        "accountAvatarUrl": _extract_tiktok_avatar_url(response),
        "accountRoomId": account_room_id,
        "accountWebRid": None,
        "accountSecUserId": _optional_string(user.secUid),
        "accountUniqueId": _optional_string(user.uniqueId),
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


def _extract_tiktok_avatar_url(response: dict) -> str | None:
    user = response.get("userInfo", {}).get("user", {})
    avatar = user.get("avatarLarger")
    if isinstance(avatar, dict):
        url_list = avatar.get("urlList")
        if isinstance(url_list, list):
            for item in url_list:
                normalized = _optional_string(item)
                if normalized:
                    return normalized

    return _optional_string(avatar)
