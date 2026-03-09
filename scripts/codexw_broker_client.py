#!/usr/bin/env python3
"""Small broker-style client fixture for the codexw connector prototype."""

from __future__ import annotations

import argparse
import json
import sys
import urllib.error
import urllib.parse
import urllib.request
from typing import Any


def _json_loads(data: bytes) -> Any:
    if not data:
        return None
    return json.loads(data.decode("utf-8"))


class BrokerClient:
    def __init__(
        self,
        base_url: str,
        agent_id: str,
        client_id: str | None = None,
        lease_seconds: int | None = None,
    ) -> None:
        self.base_url = base_url.rstrip("/")
        self.agent_id = agent_id
        self.client_id = client_id
        self.lease_seconds = lease_seconds

    def _url(self, path: str) -> str:
        return f"{self.base_url}/v1/agents/{self.agent_id}{path}"

    def _headers(self, extra: dict[str, str] | None = None) -> dict[str, str]:
        headers: dict[str, str] = {"Accept": "application/json"}
        if self.client_id:
            headers["X-Codexw-Client-Id"] = self.client_id
        if self.lease_seconds is not None:
            headers["X-Codexw-Lease-Seconds"] = str(self.lease_seconds)
        if extra:
            headers.update(extra)
        return headers

    def request(
        self,
        method: str,
        path: str,
        payload: dict[str, Any] | None = None,
        headers: dict[str, str] | None = None,
    ) -> tuple[int, Any]:
        body = None
        merged_headers = self._headers(headers)
        if payload is not None:
            body = json.dumps(payload).encode("utf-8")
            merged_headers["Content-Type"] = "application/json"

        request = urllib.request.Request(
            self._url(path),
            data=body,
            headers=merged_headers,
            method=method,
        )
        try:
            with urllib.request.urlopen(request) as response:
                return response.status, _json_loads(response.read())
        except urllib.error.HTTPError as exc:
            return exc.code, _json_loads(exc.read())

    def get(self, path: str) -> tuple[int, Any]:
        return self.request("GET", path)

    def post(self, path: str, payload: dict[str, Any] | None = None) -> tuple[int, Any]:
        return self.request("POST", path, payload)

    def stream_events(
        self,
        session_id: str,
        last_event_id: str | None = None,
        limit: int = 5,
    ) -> list[dict[str, Any]]:
        headers = self._headers({"Accept": "text/event-stream"})
        if last_event_id is not None:
            headers["Last-Event-ID"] = last_event_id
        request = urllib.request.Request(
            self._url(f"/sessions/{session_id}/events"),
            headers=headers,
            method="GET",
        )
        items: list[dict[str, Any]] = []
        current: dict[str, str] = {}
        with urllib.request.urlopen(request) as response:
            for raw_line in response:
                line = raw_line.decode("utf-8").rstrip("\r\n")
                if not line:
                    if current:
                        item: dict[str, Any] = {}
                        if "id" in current:
                            item["id"] = current["id"]
                        if "event" in current:
                            item["event"] = current["event"]
                        if "data" in current:
                            try:
                                item["data"] = json.loads(current["data"])
                            except json.JSONDecodeError:
                                item["data"] = current["data"]
                        items.append(item)
                        current = {}
                        if len(items) >= limit:
                            break
                    continue
                if line.startswith(":") or ":" not in line:
                    continue
                field, value = line.split(":", 1)
                current[field] = value.lstrip(" ")
        return items


def _print_json(status: int, payload: Any) -> None:
    print(json.dumps({"status": status, "body": payload}, indent=2, sort_keys=True))


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Broker-style client fixture for the codexw connector prototype."
    )
    parser.add_argument(
        "--base-url",
        default="http://127.0.0.1:4317",
        help="Connector base URL. Default: %(default)s",
    )
    parser.add_argument(
        "--agent-id",
        default="codexw-lab",
        help="Broker-visible agent id. Default: %(default)s",
    )
    parser.add_argument("--client-id", help="Optional broker client identity.")
    parser.add_argument(
        "--lease-seconds",
        type=int,
        help="Optional lease duration projected through connector headers.",
    )

    sub = parser.add_subparsers(dest="command", required=True)

    create = sub.add_parser("session-create", help="Create a broker-style session.")
    create.add_argument("--thread-id", required=True)

    attach = sub.add_parser("session-attach", help="Attach to an existing thread.")
    attach.add_argument("--session-id", required=True)
    attach.add_argument("--thread-id", required=True)

    renew = sub.add_parser("attachment-renew", help="Renew the current attachment lease.")
    renew.add_argument("--session-id", required=True)
    renew.add_argument("--lease-seconds", type=int, required=True)

    release = sub.add_parser("attachment-release", help="Release the current attachment.")
    release.add_argument("--session-id", required=True)

    turn = sub.add_parser("turn-start", help="Submit a turn through the connector.")
    turn.add_argument("--session-id", required=True)
    turn.add_argument("--prompt", required=True)

    sub.add_parser("sessions", help="List sessions.")

    session_get = sub.add_parser("session-get", help="Inspect one session.")
    session_get.add_argument("--session-id", required=True)

    transcript = sub.add_parser("transcript", help="Fetch transcript snapshot.")
    transcript.add_argument("--session-id", required=True)

    events = sub.add_parser("events", help="Read a small number of SSE events.")
    events.add_argument("--session-id", required=True)
    events.add_argument("--last-event-id")
    events.add_argument("--limit", type=int, default=5)

    for name, suffix in (
        ("orchestration-status", "/orchestration/status"),
        ("orchestration-workers", "/orchestration/workers"),
        ("orchestration-dependencies", "/orchestration/dependencies"),
        ("shells", "/shells"),
        ("services", "/services"),
        ("capabilities", "/capabilities"),
    ):
        cmd = sub.add_parser(name, help=f"GET {suffix}")
        cmd.add_argument("--session-id", required=True)

    shell_start = sub.add_parser("shell-start", help="Start a shell job.")
    shell_start.add_argument("--session-id", required=True)
    shell_start.add_argument("--shell-command", required=True)
    shell_start.add_argument("--intent")
    shell_start.add_argument("--label")

    shell_detail = sub.add_parser("shell-detail", help="Inspect one shell job.")
    shell_detail.add_argument("--session-id", required=True)
    shell_detail.add_argument("--job-ref", required=True)

    for name in ("shell-poll", "shell-terminate"):
        cmd = sub.add_parser(name, help=f"{name.replace('-', ' ')} for one shell job.")
        cmd.add_argument("--session-id", required=True)
        cmd.add_argument("--job-ref", required=True)

    shell_send = sub.add_parser("shell-send", help="Send stdin to one shell job.")
    shell_send.add_argument("--session-id", required=True)
    shell_send.add_argument("--job-ref", required=True)
    shell_send.add_argument("--text", required=True)

    service_detail = sub.add_parser("service-detail", help="Inspect one service by job ref.")
    service_detail.add_argument("--session-id", required=True)
    service_detail.add_argument("--job-ref", required=True)

    cap_detail = sub.add_parser("capability-detail", help="Inspect one capability.")
    cap_detail.add_argument("--session-id", required=True)
    cap_detail.add_argument("--capability", required=True)

    service_attach = sub.add_parser("service-attach", help="Attach to a service.")
    service_attach.add_argument("--session-id", required=True)
    service_attach.add_argument("--job-ref", required=True)

    service_wait = sub.add_parser("service-wait", help="Wait for service readiness.")
    service_wait.add_argument("--session-id", required=True)
    service_wait.add_argument("--job-ref", required=True)
    service_wait.add_argument("--timeout-ms", type=int)

    service_run = sub.add_parser("service-run", help="Run a service recipe.")
    service_run.add_argument("--session-id", required=True)
    service_run.add_argument("--job-ref", required=True)
    service_run.add_argument("--recipe", required=True)
    service_run.add_argument("--args-json", help="Optional JSON object for recipe args.")

    for name in ("service-provide", "service-depend"):
        cmd = sub.add_parser(name, help=f"{name.replace('-', ' ')} for one service.")
        cmd.add_argument("--session-id", required=True)
        cmd.add_argument("--job-ref", required=True)
        cmd.add_argument("--values-json", required=True, help="JSON array or null.")

    service_contract = sub.add_parser("service-contract", help="Update service contract.")
    service_contract.add_argument("--session-id", required=True)
    service_contract.add_argument("--job-ref", required=True)
    service_contract.add_argument(
        "--contract-json",
        required=True,
        help="JSON object with protocol/endpoint/attachHint/readyPattern/recipes.",
    )

    service_relabel = sub.add_parser("service-relabel", help="Relabel a service.")
    service_relabel.add_argument("--session-id", required=True)
    service_relabel.add_argument("--job-ref", required=True)
    service_relabel.add_argument("--label", required=True)

    return parser


def main() -> int:
    args = build_parser().parse_args()
    client = BrokerClient(
        base_url=args.base_url,
        agent_id=args.agent_id,
        client_id=args.client_id,
        lease_seconds=args.lease_seconds,
    )

    cmd = args.command
    status: int
    payload: Any

    if cmd == "session-create":
        status, payload = client.post("/sessions", {"thread_id": args.thread_id})
    elif cmd == "session-attach":
        status, payload = client.post(
            f"/sessions/{args.session_id}/attach", {"thread_id": args.thread_id}
        )
    elif cmd == "attachment-renew":
        status, payload = client.post(
            f"/sessions/{args.session_id}/attachment/renew",
            {"lease_seconds": args.lease_seconds},
        )
    elif cmd == "attachment-release":
        status, payload = client.post(f"/sessions/{args.session_id}/attachment/release", {})
    elif cmd == "turn-start":
        status, payload = client.post(
            f"/sessions/{args.session_id}/turns", {"prompt": args.prompt}
        )
    elif cmd == "sessions":
        status, payload = client.get("/sessions")
    elif cmd == "session-get":
        status, payload = client.get(f"/sessions/{args.session_id}")
    elif cmd == "transcript":
        status, payload = client.get(f"/sessions/{args.session_id}/transcript")
    elif cmd == "events":
        _print_json(
            200,
            client.stream_events(args.session_id, args.last_event_id, args.limit),
        )
        return 0
    elif cmd == "orchestration-status":
        status, payload = client.get(f"/sessions/{args.session_id}/orchestration/status")
    elif cmd == "orchestration-workers":
        status, payload = client.get(f"/sessions/{args.session_id}/orchestration/workers")
    elif cmd == "orchestration-dependencies":
        status, payload = client.get(
            f"/sessions/{args.session_id}/orchestration/dependencies"
        )
    elif cmd == "shells":
        status, payload = client.get(f"/sessions/{args.session_id}/shells")
    elif cmd == "services":
        status, payload = client.get(f"/sessions/{args.session_id}/services")
    elif cmd == "capabilities":
        status, payload = client.get(f"/sessions/{args.session_id}/capabilities")
    elif cmd == "shell-start":
        body: dict[str, Any] = {"command": args.shell_command}
        if args.intent:
            body["intent"] = args.intent
        if args.label:
            body["label"] = args.label
        status, payload = client.post(f"/sessions/{args.session_id}/shells", body)
    elif cmd == "shell-detail":
        ref = urllib.parse.quote(args.job_ref, safe="")
        status, payload = client.get(f"/sessions/{args.session_id}/shells/{ref}")
    elif cmd == "shell-poll":
        ref = urllib.parse.quote(args.job_ref, safe="")
        status, payload = client.post(f"/sessions/{args.session_id}/shells/{ref}/poll", {})
    elif cmd == "shell-terminate":
        ref = urllib.parse.quote(args.job_ref, safe="")
        status, payload = client.post(
            f"/sessions/{args.session_id}/shells/{ref}/terminate", {}
        )
    elif cmd == "shell-send":
        ref = urllib.parse.quote(args.job_ref, safe="")
        status, payload = client.post(
            f"/sessions/{args.session_id}/shells/{ref}/send", {"text": args.text}
        )
    elif cmd == "service-detail":
        ref = urllib.parse.quote(args.job_ref, safe="")
        status, payload = client.get(f"/sessions/{args.session_id}/services/{ref}")
    elif cmd == "capability-detail":
        ref = urllib.parse.quote(args.capability, safe="")
        status, payload = client.get(f"/sessions/{args.session_id}/capabilities/{ref}")
    elif cmd == "service-attach":
        ref = urllib.parse.quote(args.job_ref, safe="")
        status, payload = client.post(f"/sessions/{args.session_id}/services/{ref}/attach", {})
    elif cmd == "service-wait":
        ref = urllib.parse.quote(args.job_ref, safe="")
        body: dict[str, Any] = {}
        if args.timeout_ms is not None:
            body["timeout_ms"] = args.timeout_ms
        status, payload = client.post(f"/sessions/{args.session_id}/services/{ref}/wait", body)
    elif cmd == "service-run":
        ref = urllib.parse.quote(args.job_ref, safe="")
        body: dict[str, Any] = {"recipe": args.recipe}
        if args.args_json:
            body["args"] = json.loads(args.args_json)
        status, payload = client.post(f"/sessions/{args.session_id}/services/{ref}/run", body)
    elif cmd == "service-provide":
        ref = urllib.parse.quote(args.job_ref, safe="")
        status, payload = client.post(
            f"/sessions/{args.session_id}/services/{ref}/provide",
            {"capabilities": json.loads(args.values_json)},
        )
    elif cmd == "service-depend":
        ref = urllib.parse.quote(args.job_ref, safe="")
        status, payload = client.post(
            f"/sessions/{args.session_id}/services/{ref}/depend",
            {"dependsOnCapabilities": json.loads(args.values_json)},
        )
    elif cmd == "service-contract":
        ref = urllib.parse.quote(args.job_ref, safe="")
        contract = json.loads(args.contract_json)
        if not isinstance(contract, dict):
            raise SystemExit("--contract-json must decode to an object")
        status, payload = client.post(
            f"/sessions/{args.session_id}/services/{ref}/contract",
            contract,
        )
    elif cmd == "service-relabel":
        ref = urllib.parse.quote(args.job_ref, safe="")
        status, payload = client.post(
            f"/sessions/{args.session_id}/services/{ref}/relabel",
            {"label": None if args.label == "none" else args.label},
        )
    else:
        raise SystemExit(f"unsupported command: {cmd}")

    _print_json(status, payload)
    return 0


if __name__ == "__main__":
    sys.exit(main())
