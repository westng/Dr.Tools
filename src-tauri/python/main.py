import contextlib
import json
import sys
import traceback
from pathlib import Path

from core import handle_request


def process_request(request: dict) -> dict:
    response = {"id": None, "result": None, "error": None}
    try:
        request_id = int(request.get("id", 0))
        method = str(request.get("method", ""))
        params = request.get("params", {})

        response["id"] = request_id
        with contextlib.redirect_stdout(sys.stderr):
            response["result"] = handle_request(method, params)
    except Exception as exc:
        response["error"] = f"{exc.__class__.__name__}: {exc}"
        if response["id"] is None:
            response["id"] = -1
    return response


def run_file_request(request_path: str, response_path: str) -> int:
    response = {"id": -1, "result": None, "error": "unknown error"}
    try:
        request = json.loads(Path(request_path).read_text(encoding="utf-8"))
        response = process_request(request)
    except Exception as exc:
        response = {"id": -1, "result": None, "error": f"{exc.__class__.__name__}: {exc}"}

    Path(response_path).write_text(json.dumps(response, ensure_ascii=False), encoding="utf-8")
    return 0


def main() -> int:
    if len(sys.argv) == 3:
        return run_file_request(sys.argv[1], sys.argv[2])

    for raw in sys.stdin:
        line = raw.strip()
        if not line:
            continue

        response = process_request(json.loads(line))

        sys.stdout.write(json.dumps(response, ensure_ascii=False) + "\n")
        sys.stdout.flush()

    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception:
        traceback.print_exc(file=sys.stderr)
        raise
