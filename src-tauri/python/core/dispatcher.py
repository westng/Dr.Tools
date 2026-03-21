from tasks.recording_account_resolve import run_recording_account_resolve
from tasks.recording_live_status_check import run_recording_live_status_check
from tasks.stub import run_task_stub
from tasks.token_validate import run_token_validate
from tasks.video_download import run_video_download


def handle_request(method: str, params: dict):
    if method == "ping":
        return {"message": "pong"}

    if method == "run_task":
        task_type = params.get("task_type", "")
        payload = params.get("payload", {})
        if task_type == "video.download":
            return run_video_download(payload)
        if task_type == "token.validate":
            return run_token_validate(payload)
        if task_type == "recording.account.resolve":
            return run_recording_account_resolve(payload)
        if task_type == "recording.live_status.check":
            return run_recording_live_status_check(payload)
        return run_task_stub(task_type, payload)

    raise ValueError(f"unsupported method: {method}")
