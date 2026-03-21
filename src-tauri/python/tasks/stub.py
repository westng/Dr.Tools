from datetime import datetime, timezone


def run_task_stub(task_type: str, payload: dict) -> dict:
    return {
        "task_type": task_type,
        "echo": payload,
        "processed_at": datetime.now(timezone.utc).isoformat(),
        "message": "stub task completed",
    }
