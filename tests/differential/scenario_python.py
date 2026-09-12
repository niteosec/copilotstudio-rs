"""Run the differential scenario with the PINNED upstream Python client and write its recording.

Usage: scenario_python.py <Agents-for-python checkout> <base url> <out.json>
"""

import asyncio
import json
import sys
from pathlib import Path

repo = Path(sys.argv[1])
sys.path.insert(0, str(repo / "libraries" / "microsoft-agents-activity"))
sys.path.insert(0, str(repo / "libraries" / "microsoft-agents-copilotstudio-client"))

from microsoft_agents.activity import Activity  # noqa: E402
from microsoft_agents.copilotstudio.client import ConnectionSettings, CopilotClient, StartRequest  # noqa: E402

base, out = sys.argv[2], sys.argv[3]


def dump(activity):
    return json.loads(activity.model_dump_json(exclude_unset=True, by_alias=True))


async def collect(agen):
    return [dump(a) async for a in agen]


async def main():
    steps = {}

    client = CopilotClient(ConnectionSettings("", "", direct_connect_url=f"{base}/bots/Main"), "tok-shared")
    steps["start"] = await collect(client.start_conversation())
    steps["conversation_id_after_start"] = client._current_conversation_id
    steps["ask"] = await collect(client.ask_question("Hello?"))
    steps["execute"] = await collect(client.execute("explicit-B", Activity(type="message", text="run this")))
    steps["conversation_id_after_execute"] = client._current_conversation_id
    steps["subscribe"] = [{"event_id": e.event_id, "activity": dump(e.activity)} async for e in client.subscribe("conv-A", "evt-0")]
    steps["start_with_request"] = await collect(
        client.start_conversation_with_request(StartRequest(emit_start_conversation_event=False, locale="fr-FR", conversation_id="conv-A"))
    )

    client = CopilotClient(ConnectionSettings("", "", direct_connect_url=f"{base}/bots/NoHeader"), "tok-shared")
    steps["headerless_start"] = await collect(client.start_conversation())
    steps["conversation_id_after_headerless_start"] = client._current_conversation_id
    steps["headerless_ask"] = await collect(client.ask_question("which?"))

    client = CopilotClient(ConnectionSettings("", "", direct_connect_url=f"{base}/bots/JsonOnly"), "tok-shared")
    steps["json_only_start"] = await collect(client.start_conversation())
    steps["conversation_id_after_json_only"] = client._current_conversation_id

    client = CopilotClient(ConnectionSettings("", "", direct_connect_url=f"{base}/bots/Broken"), "tok-shared")
    try:
        await collect(client.start_conversation())
        steps["broken_start"] = "no error"
    except Exception as e:  # noqa: BLE001
        steps["broken_start"] = f"{type(e).__name__}: {e}"

    Path(out).write_text(json.dumps(steps, indent=2, sort_keys=True))


asyncio.run(main())
