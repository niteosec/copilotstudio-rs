#!/usr/bin/env python3
"""Regenerate the activity parity fixtures from the *upstream* Python model.

Usage:
    python3 tests/fixtures/generate.py /path/to/Agents-for-python   # checked out at the pinned tag

Requires `pydantic`, `pyjwt` and `aiohttp` importable (e.g. `uv venv && uv pip install pydantic pyjwt aiohttp`).
The fixtures are what `microsoft_agents.activity` serialises with
`model_dump_json(exclude_unset=True, by_alias=True)` — the exact bytes the Python client puts on
the wire — so the Rust round-trip tests prove parity against the reference, not against a reading
of it. Re-run on every pin bump (docs/DESIGN.md §10).
"""

import json
import sys
from pathlib import Path

repo = Path(sys.argv[1])
sys.path.insert(0, str(repo / "libraries" / "microsoft-agents-activity"))
sys.path.insert(0, str(repo / "libraries" / "microsoft-agents-copilotstudio-client"))

from microsoft_agents.activity import (  # noqa: E402
    Activity,
    Attachment,
    CardAction,
    ChannelAccount,
    ChannelId,
    ConversationAccount,
    ConversationReference,
    SuggestedActions,
    TextHighlight,
)
from microsoft_agents.activity.entity import Entity, StreamInfo  # noqa: E402
from microsoft_agents.copilotstudio.client import ExecuteTurnRequest, StartRequest  # noqa: E402

out = Path(__file__).parent


def dump(name: str, model) -> None:
    text = model.model_dump_json(exclude_unset=True, by_alias=True)
    (out / name).write_text(json.dumps(json.loads(text), indent=2, sort_keys=True) + "\n")
    print(f"wrote {name}")


# 1. The minimal message the upstream tests use.
dump(
    "activity_minimal_message.json",
    Activity(type="message", text="Hello, world!", conversation={"id": "1234567890"}),
)

# 2. A rich activity exercising aliases, nested models, entities (known + unknown), opaque JSON.
rich = Activity(
    type="message",
    id="act-1",
    timestamp="2026-09-12T10:11:12.1234567Z",
    local_timestamp="2026-09-12T12:11:12.1234567+02:00",
    local_timezone="Europe/Zurich",
    service_url="https://example.invalid/service",
    channel_id=ChannelId("msteams:sub"),
    from_property=ChannelAccount(id="agent-1", name="Agent", role="bot"),
    recipient=ChannelAccount(id="user-1", aad_object_id="00000000-0000-0000-0000-000000000001", tenant_id="t-1"),
    conversation=ConversationAccount(id="conv-1", conversation_type="personal", tenant_id="t-1"),
    text_format="markdown",
    attachment_layout="list",
    locale="en-US",
    text="**Hello**",
    speak="Hello",
    input_hint="acceptingInput",
    summary="summary",
    suggested_actions=SuggestedActions(
        to=["user-1"],
        actions=[CardAction(type="imBack", title="Yes", value="yes"), CardAction(type="openUrl", title="Docs", value="https://example.invalid")],
    ),
    attachments=[
        Attachment(
            content_type="application/vnd.microsoft.card.adaptive",
            content={"type": "AdaptiveCard", "version": "1.5", "body": [{"type": "TextBlock", "text": "hi"}]},
            name="card",
        )
    ],
    entities=[
        StreamInfo(stream_type="final", stream_id="stream-1", stream_sequence=3, stream_result="success"),
        Entity.model_validate({"type": "https://schema.org/Message", "@type": "Message", "citation": [{"position": 1}]}),
        Entity(type="customEntity", foo="bar", nested={"a": [1, 2]}),
    ],
    channel_data={"streamType": "legacy", "custom": True},
    reply_to_id="act-0",
    value_type="application/vnd.custom",
    value={"k": "v", "n": 1},
    name="op",
    relates_to=ConversationReference(
        activity_id="act-0",
        user=ChannelAccount(id="user-1"),
        agent=ChannelAccount(id="agent-1"),
        conversation=ConversationAccount(id="conv-1"),
        channel_id=ChannelId("msteams"),
        service_url="https://example.invalid/service",
    ),
    importance="normal",
    delivery_mode="normal",
    listen_for=["yes", "no"],
    text_highlights=[TextHighlight(text="Hello", occurrence=1)],
    caller_id="urn:botframework:azure",
)
dump("activity_rich.json", rich)

# 3. A streamed typing chunk as the service emits it.
dump(
    "activity_typing_stream_chunk.json",
    Activity(
        type="typing",
        id="chunk-2",
        conversation=ConversationAccount(id="conv-1"),
        text="Hello, wor",
        entities=[StreamInfo(stream_type="streaming", stream_id="stream-1", stream_sequence=2)],
    ),
)

# 4. The wire bodies the client sends.
dump(
    "execute_turn_request.json",
    ExecuteTurnRequest(activity=Activity(type="message", text="Test message", conversation={"id": "456"})),
)
dump("start_request_full.json", StartRequest(emit_start_conversation_event=True, locale="en-US", conversation_id="test-123"))
dump("start_request_default.json", StartRequest(emit_start_conversation_event=True))
