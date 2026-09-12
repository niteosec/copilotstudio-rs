#!/usr/bin/env python3
"""Differential parity harness: the pinned upstream clients vs the Rust client, same server.

Usage:
    python3 tests/differential/run.py /path/to/Agents-for-python [--js /path/to/Agents-for-js]

Both checkouts must be at the pinned tags. The JS half needs `bun` and a one-off build of the
workspace (see tests/differential/README.md).

Starts the recording server, runs both scenarios, then diffs (a) every request each client sent —
method, path, query, the protocol headers, JSON body — and (b) every activity each client yielded.
Known, documented divergences (docs/DESIGN.md §6) are listed in EXPECTED and reported separately;
anything else is a parity failure and exits non-zero.
"""

import json
import os
import re
import socket
import subprocess
import sys
import time
import urllib.request
from pathlib import Path

HERE = Path(__file__).parent
ROOT = HERE.parent.parent
PY = sys.executable
repo = sys.argv[1]
js_repo = sys.argv[sys.argv.index("--js") + 1] if "--js" in sys.argv else None

COMPARED_HEADERS = ["accept", "content-type", "authorization", "x-ms-conversation-id", "last-event-id"]

# (step or request index, field) → reason. Keyed by a short label produced by `describe`.
# Request order: Main start(0) ask(1) execute(2) subscribe(3) start_with_request(4);
#                NoHeader start(5) ask(6); JsonOnly start(7); Broken start(8).
# Known, documented divergences between the Rust port and each reference (docs/DESIGN.md §6).
EXPECTED = {
    "python": {
        "request[4].headers.x-ms-conversation-id": "D7: .NET sends x-ms-conversation-id on a start request naming a conversation; Python sends the body only",
        "step.json_only_start": "D9: .NET JSON fallback; Python has none and yields nothing",
        "step.conversation_id_after_headerless_start": "§3.6: Python overwrites the current id on every message; .NET/JS keep the first",
        "request[6].path": "§3.6 consequence: the follow-up turn goes to the id each client kept",
        "request[6].body.activity.conversation.id": "§3.6 consequence: the follow-up turn goes to the id each client kept",
        "step.headerless_ask": "§3.6 consequence: the follow-up turn goes to the id each client kept",
        "step.broken_start": "D8: error text — .NET includes the body, Python the status only; the Rust type follows .NET",
    },
    "js": {
        "request[2].body": "D16: JS executeStreaming sends conversationId on the wrapper; .NET/Python force activity.conversation.id",
        "request[3].headers.content-type": "D3: .NET/Python send Content-Type on subscribe; JS does not",
        "request[4].path": "D7: JS posts a start naming a conversation to the conversation URL; .NET uses the collection URL + header + body",
        "request[4].headers.x-ms-conversation-id": "D7",
        "request[4].body.conversationId": "D7",
        "step.start_with_request": "D7 consequence: the server answers the conversation URL with the turn fixture, so JS yields different activities",
        "request[7]": "D9: JS reconnects forever on a non-SSE 200 (step skipped); .NET falls back to JSON",
        "step.json_only_start": "D9",
        "step.conversation_id_after_json_only": "D9",
        "step.ask[1].text": "D4: JS rewrites typing chunks with the accumulated stream text; .NET/Python (and the port) yield the raw chunk",
        "step.broken_start": "D8: error text differs per client; the Rust type follows .NET",
    },
}


def free_port():
    with socket.socket() as s:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]


def fetch(url):
    with urllib.request.urlopen(url) as r:
        return json.loads(r.read())


def reset(base):
    urllib.request.urlopen(urllib.request.Request(f"{base}/__reset", method="POST")).read()


def normalise_request(r):
    return {
        "method": r["method"],
        "path": r["path"],
        "query": r["query"],
        "headers": {h: r["headers"].get(h) for h in COMPARED_HEADERS},
        # `CopilotStudioClient.agents-sdk-{lang}/{version} …` — compare everything but the language.
        "user_agent_shape": re.sub(r"^(CopilotStudioClient\.agents-sdk-)[a-z]+/.*$", r"\1*", r["headers"].get("user-agent") or ""),
        "body": r["body"],
    }


def diff(a, b, prefix, out):
    if isinstance(a, dict) and isinstance(b, dict):
        for k in sorted(set(a) | set(b)):
            diff(a.get(k), b.get(k), f"{prefix}.{k}", out)
    elif isinstance(a, list) and isinstance(b, list):
        for i, (x, y) in enumerate(zip(a, b)):
            diff(x, y, f"{prefix}[{i}]", out)
        if len(a) != len(b):
            out.append((f"{prefix}.length", len(a), len(b)))
    elif a != b:
        out.append((prefix, a, b))


def compare(label, ref_requests, ref_steps, rust_requests, rust_steps):
    differences = []
    if len(ref_requests) != len(rust_requests):
        differences.append(("request.count", len(ref_requests), len(rust_requests)))
    diff(ref_requests, rust_requests, "request", differences)
    for step in sorted(set(ref_steps) | set(rust_steps)):
        diff(ref_steps.get(step), rust_steps.get(step), f"step.{step}", differences)

    expected = EXPECTED[label]
    unexpected = []
    print(f"\n=== {label}: {len(ref_requests)} requests recorded ({len(rust_requests)} by rust); {len(ref_steps)} scenario steps")
    for where, ref, rs in differences:
        key = next((k for k in expected if where.startswith(k)), None)
        tag = "expected " if key else "UNEXPECTED"
        print(f"[{tag}] {where}\n    {label:<6}: {json.dumps(ref)[:200]}\n    rust  : {json.dumps(rs)[:200]}")
        if key:
            print(f"    ↳ {expected[key]}")
        else:
            unexpected.append(where)
    if not differences:
        print("identical")
    print(f"--- {label}: {len(differences) - len(unexpected)} expected divergence(s), {len(unexpected)} unexpected")
    return unexpected


def run_js(base, out_dir):
    pkg = Path(js_repo) / "packages" / "agents-copilotstudio-client"
    script = pkg / "scenario_js.ts"
    script.write_text((HERE / "scenario_js.ts").read_text())
    try:
        subprocess.run(["bun", "run", str(script), base, str(out_dir / "js_steps.json")], cwd=pkg, check=True, timeout=120)
    finally:
        script.unlink(missing_ok=True)


def main():
    port = free_port()
    base = f"http://127.0.0.1:{port}"
    server = subprocess.Popen([PY, str(HERE / "server.py"), str(port)])
    try:
        for _ in range(50):
            try:
                fetch(f"{base}/__records")
                break
            except Exception:
                time.sleep(0.1)
        out_dir = ROOT / "target" / "differential"
        out_dir.mkdir(parents=True, exist_ok=True)

        env = dict(os.environ, DIFFERENTIAL_BASE_URL=base, DIFFERENTIAL_OUT=str(out_dir / "rust_steps.json"))
        subprocess.run(["cargo", "test", "-q", "--test", "differential", "--", "--ignored"], cwd=ROOT, env=env, check=True)
        rust_requests = [normalise_request(r) for r in fetch(f"{base}/__records")]
        rust_steps = json.loads((out_dir / "rust_steps.json").read_text())
        (out_dir / "rust_requests.json").write_text(json.dumps(rust_requests, indent=2))
        reset(base)

        subprocess.run([PY, str(HERE / "scenario_python.py"), repo, base, str(out_dir / "python_steps.json")], check=True)
        python_requests = [normalise_request(r) for r in fetch(f"{base}/__records")]
        python_steps = json.loads((out_dir / "python_steps.json").read_text())
        (out_dir / "python_requests.json").write_text(json.dumps(python_requests, indent=2))
        reset(base)
        unexpected = compare("python", python_requests, python_steps, rust_requests, rust_steps)

        if js_repo:
            run_js(base, out_dir)
            js_requests = [normalise_request(r) for r in fetch(f"{base}/__records")]
            js_requests.insert(7, {"skipped": "JsonOnly (D9)"})  # keep indices aligned with the Rust run
            js_steps = json.loads((out_dir / "js_steps.json").read_text())
            (out_dir / "js_requests.json").write_text(json.dumps(js_requests, indent=2))
            unexpected += compare("js", js_requests, js_steps, rust_requests, rust_steps)

        sys.exit(1 if unexpected else 0)
    finally:
        server.terminate()


if __name__ == "__main__":
    main()
