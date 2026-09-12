// Run the differential scenario with the PINNED upstream JS client (its TypeScript source, via bun).
// `run.py --js <Agents-for-js checkout>` copies this file into the client package and runs it there.
//
// The JsonOnly step is skipped for JS: `eventsource-client` reconnects forever on a non-SSE 200
// (the JS client has no JSON fallback), so the scenario would never finish.
import { writeFileSync } from 'fs'
import { Activity } from '@microsoft/agents-activity'
import { ConnectionSettings, CopilotStudioClient } from '@microsoft/agents-copilotstudio-client'

const [base, out] = [process.argv[2], process.argv[3]]

function dump(a: Activity) {
  // The Activity class keeps `channelId` in a private `_channelId` behind a getter; JSON.stringify
  // emits the field, not the getter. Dump the wire view.
  const o = JSON.parse(JSON.stringify(a))
  for (const k of Object.keys(o)) if (k.startsWith('_')) delete o[k]
  if (a.channelId !== undefined) o.channelId = a.channelId
  return o
}
async function collect(gen: AsyncGenerator<Activity>) {
  const r: unknown[] = []
  for await (const a of gen) r.push(dump(a))
  return r
}
const settings = (bot: string) => new ConnectionSettings({ directConnectUrl: `${base}/bots/${bot}` })
const steps: Record<string, unknown> = {}

let client = new CopilotStudioClient(settings('Main'), 'tok-shared')
steps.start = await collect(client.startConversationStreaming(true))
steps.conversation_id_after_start = (client as any).conversationId
steps.ask = await collect(client.sendActivityStreaming(Activity.fromObject({ type: 'message', text: 'Hello?', conversation: { id: (client as any).conversationId } })))
steps.execute = await collect(client.executeStreaming(Activity.fromObject({ type: 'message', text: 'run this' }), 'explicit-B'))
steps.conversation_id_after_execute = (client as any).conversationId
const sub: unknown[] = []
for await (const e of client.subscribeAsync('conv-A', 'evt-0')) sub.push({ event_id: e.eventId ?? null, activity: dump(e.activity) })
steps.subscribe = sub
steps.start_with_request = await collect(client.startConversationStreaming({ emitStartConversationEvent: false, locale: 'fr-FR', conversationId: 'conv-A' }))

client = new CopilotStudioClient(settings('NoHeader'), 'tok-shared')
steps.headerless_start = await collect(client.startConversationStreaming(true))
steps.conversation_id_after_headerless_start = (client as any).conversationId
steps.headerless_ask = await collect(client.sendActivityStreaming(Activity.fromObject({ type: 'message', text: 'which?', conversation: { id: (client as any).conversationId } })))

steps.json_only_start = 'skipped: eventsource-client reconnects forever on a non-SSE 200'
steps.conversation_id_after_json_only = 'skipped'

client = new CopilotStudioClient(settings('Broken'), 'tok-shared')
try {
  await collect(client.startConversationStreaming(true))
  steps.broken_start = 'no error'
} catch (e) {
  steps.broken_start = `${(e as Error).name}: ${(e as Error).message}`
}
writeFileSync(out, JSON.stringify(steps, null, 2))
