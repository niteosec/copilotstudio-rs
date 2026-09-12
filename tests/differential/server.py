"""Recording Direct-to-Engine stand-in used by the differential parity harness.

Serves a fixed scenario (see SCENARIO) and records every request it receives. Both the pinned
upstream clients and the Rust client are pointed at it; `run.py` diffs the recordings.
"""

import json
from aiohttp import web

API_VERSION = "2022-03-01-preview"


def sse(*events):
    """events: (event_type, data_json, id) tuples → SSE body, terminated by the `end` event the
    service emits after every turn (the JS client only stops on it; .NET/Python stop on close)."""
    out = []
    for ev in events:
        event_type, data, event_id = ev
        if event_id:
            out.append(f"id: {event_id}")
        out.append(f"event: {event_type}")
        out.append(f"data: {json.dumps(data, separators=(',', ':'))}")
        out.append("")
    out += ["event: end", "data: done", ""]
    return "\n".join(out) + "\n"


def msg(text, conv, **extra):
    return {"type": "message", "text": text, "conversation": {"id": conv}, **extra}


# path suffix (after the bot base) → (method, status, headers, content_type, body)
SCENARIO = {
    # Step 1: start. Header carries the conversation id; the stream mixes activity + non-activity events.
    ("POST", "/bots/Main/conversations"): (
        200,
        {"x-ms-conversationid": "conv-A"},
        "text/event-stream",
        sse(
            ("activity", {"type": "typing"}, None),
            ("activity", {"type": "event", "name": "startConversation", "conversation": {"id": "conv-A"}}, None),
            ("activity", msg("Welcome to the differential agent.", "conv-A", textFormat="markdown",
                             suggestedActions={"to": [], "actions": [{"type": "imBack", "title": "Help", "value": "help"}]}), None),
        ),
    ),
    # Step 2: ask in conv-A → streamed chunks then the final message.
    ("POST", "/bots/Main/conversations/conv-A"): (
        200,
        {"x-ms-conversationid": "conv-A"},
        "text/event-stream",
        sse(
            ("activity", {"type": "typing", "text": "Hel", "conversation": {"id": "conv-A"},
                          "entities": [{"type": "streaminfo", "streamType": "streaming", "streamId": "s1", "streamSequence": 1}]}, None),
            ("activity", {"type": "typing", "text": "lo there", "conversation": {"id": "conv-A"},
                          "entities": [{"type": "streaminfo", "streamType": "streaming", "streamId": "s1", "streamSequence": 2}]}, None),
            ("activity", msg("Hello there", "conv-A", id="m-2", channelId="pva-studio",
                             entities=[{"type": "streaminfo", "streamType": "final", "streamId": "s1"}],
                             attachments=[{"contentType": "application/vnd.microsoft.card.adaptive",
                                           "content": {"type": "AdaptiveCard", "version": "1.5", "body": []}}],
                             channelData={"custom": True}, value={"k": [1, 2]}), None),
        ),
    ),
    # Step 3: execute in an explicit conversation.
    ("POST", "/bots/Main/conversations/explicit-B"): (
        200,
        {},
        "text/event-stream",
        sse(("activity", msg("Executed in B", "explicit-B"), None)),
    ),
    # Step 4: subscribe with resumption; ids on some events only.
    ("GET", "/bots/Main/conversations/conv-A/subscribe"): (
        200,
        {},
        "text/event-stream",
        sse(
            ("activity", msg("sub 1", "conv-A"), "evt-1"),
            ("activity", {"type": "typing", "conversation": {"id": "conv-A"}}, None),
            ("activity", msg("sub 2", "conv-A"), "evt-2"),
        ),
    ),
    # Step 5: start without the header; two messages with different conversation ids.
    ("POST", "/bots/NoHeader/conversations"): (
        200,
        {},
        "text/event-stream",
        sse(("activity", msg("first", "from-first"), None), ("activity", msg("second", "from-second"), None)),
    ),
    ("POST", "/bots/NoHeader/conversations/from-first"): (200, {}, "text/event-stream", sse(("activity", msg("ok first", "from-first"), None))),
    ("POST", "/bots/NoHeader/conversations/from-second"): (200, {}, "text/event-stream", sse(("activity", msg("ok second", "from-second"), None))),
    # Step 6: a JSON (non-streamed) start response.
    ("POST", "/bots/JsonOnly/conversations"): (
        200,
        {"x-ms-conversationid": "conv-J"},
        "application/json",
        json.dumps({"activities": [msg("json welcome", "conv-J")], "conversationId": "conv-J"}),
    ),
    # Step 7: an error.
    ("POST", "/bots/Broken/conversations"): (403, {}, "application/json", json.dumps({"error": {"code": "D2EAccessDenied"}})),
}

records = []


async def handle(request: web.Request):
    body = await request.read()
    try:
        body_json = json.loads(body) if body else None
    except ValueError:
        body_json = body.decode("utf-8", "replace")
    records.append({
        "method": request.method,
        "path": request.path,
        "query": dict(request.query),
        "headers": {k.lower(): v for k, v in request.headers.items()},
        "body": body_json,
    })
    key = (request.method, request.path)
    if key not in SCENARIO:
        return web.Response(status=404, text=f"no scenario for {key}")
    status, headers, content_type, payload = SCENARIO[key]
    return web.Response(status=status, headers=headers, content_type=content_type, text=payload)


async def dump(_request):
    return web.json_response(records)


async def reset(_request):
    records.clear()
    return web.Response(text="ok")


def make_app():
    app = web.Application()
    app.router.add_get("/__records", dump)
    app.router.add_post("/__reset", reset)
    app.router.add_route("*", "/{tail:.*}", handle)
    return app


if __name__ == "__main__":
    import sys
    port = int(sys.argv[1]) if len(sys.argv) > 1 else 8765
    web.run_app(make_app(), host="127.0.0.1", port=port, print=None)
