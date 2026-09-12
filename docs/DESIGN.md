# copilotstudio-rs — Design

An **unofficial Rust port** of Microsoft's Copilot Studio client from the open-source
Microsoft 365 Agents SDK (`microsoft/Agents`). The port talks to published Copilot Studio agents
over the **Direct-to-Engine (D2E) protocol** exposed through the Power Platform API, authenticated
with an Entra token for the `CopilotStudio.Copilots.Invoke` permission.

This document is the contract for the crate: what is mirrored, at which upstream version, how the
reference structure maps onto Rust, where the port deliberately diverges, and how parity is tested.
It is written *before* the implementation and updated with every pin bump.

---

## 1. Positioning

| | |
|---|---|
| **What** | A faithful port of the official client's public surface and wire behaviour — activities in, activities out, streamed. |
| **What not** | Not a redesigned SDK, not an agent framework adapter, not a Direct Line client (yet), not a reverse-engineering of the service. Everything here is derived from the three open-source reference clients. |
| **Spec** | The reference clients' source at the pinned tags (§2). Where the three clients disagree, §6 records which behaviour is mirrored and why. |
| **Auth** | Tokens are supplied by the caller. The crate never acquires tokens (no MSAL, no OBO exchange inside the crate). |
| **Licence** | MIT OR Apache-2.0. The upstream is MIT (Microsoft Corporation); attribution is carried in `README.md`, `NOTICE`, and this file. |

Prior art: [`agent-framework-copilotstudio`](https://crates.io/crates/agent-framework-copilotstudio)
(community, part of `agent-framework-rs`) wraps the same endpoints behind an Agent-Framework-shaped
`run_once` API. It confirms the URL algorithm independently but does not mirror the client surface
(no activity streaming API, no `execute`/`subscribe`, no Activity schema). This crate is the
client-level port that such adapters could sit on.

---

## 2. Upstream pin

The official client ships in three repositories. All three are pinned; the **.NET client is the
primary reference** because it is the origin implementation the other two port from (their
docstrings say so), and it carries behaviours the others lack (JSON fallback, header constants).
Python and JS are cross-checked for every behaviour and used to break ties where .NET is silent.

| Client | Repository | Tag | Commit | Date | Path |
|---|---|---|---|---|---|
| .NET (**primary**) | `microsoft/Agents-for-net` | `v1.8.77` | `d2493a5feebaaf742a456d24f8caa9878d3f2373` | 2026-08-24 | `src/libraries/Client/Microsoft.Agents.CopilotStudio.Client/` |
| Python | `microsoft/Agents-for-python` | `v1.5.0` | `d3f427d81b6d5fdd40bcb8ad16b78dbeb4530631` | 2026-08-26 | `libraries/microsoft-agents-copilotstudio-client/` |
| JavaScript | `microsoft/Agents-for-js` | `v1.8` | `8f07a596a54b59219435af4a574d0740c814d36b` | 2026-08-27 | `packages/agents-copilotstudio-client/` |

Activity schema reference: `microsoft-agents-activity` (Python, same tag) and
`Microsoft.Agents.Core/Models` (.NET, same tag).

Pin metadata is also machine-readable in `UPSTREAM.toml` at the repo root; the crate's
`UPSTREAM` constant exposes it at runtime.

**Pin-bump procedure** (§10) is the only sanctioned way to change protocol behaviour.

---

## 3. The wire surface to mirror

Everything below is transcribed from the pinned sources. Nothing is inferred from network traffic.

### 3.1 Endpoint construction (`PowerPlatformEnvironment`)

Two modes. `DirectConnectUrl` wins when set; otherwise the URL is derived from
`(cloud, environment_id, schema_name, agent_type)`.

**Environment host** — `GetEnvironmentEndpoint(cloud, environment_id, cloud_base_address)`:

```
normalized = environment_id.lower().replace("-", "")
n          = 2 if cloud in {Prod, FirstRelease} else 1
host       = f"{normalized[:-n]}.{normalized[-n:]}.environment.{endpoint_suffix(cloud)}"
```

**Endpoint suffix per cloud** (`GetEndpointSuffix`):

| `PowerPlatformCloud` | suffix |
|---|---|
| `Local` | `api.powerplatform.localhost` |
| `Exp` | `api.exp.powerplatform.com` |
| `Dev` | `api.dev.powerplatform.com` |
| `Prv` | `api.prv.powerplatform.com` |
| `Test` | `api.test.powerplatform.com` |
| `Preprod` | `api.preprod.powerplatform.com` |
| `FirstRelease`, `Prod` | `api.powerplatform.com` |
| `GovFR`, `Gov` | `api.gov.powerplatform.microsoft.us` |
| `High` | `api.high.powerplatform.microsoft.us` |
| `DoD` | `api.appsplatform.us` |
| `Mooncake` | `api.powerplatform.partner.microsoftonline.cn` |
| `Ex` | `api.powerplatform.eaglex.ic.gov` |
| `Rx` | `api.powerplatform.microsoft.scloud` |
| `Other` | the caller-supplied `custom_power_platform_cloud` / `cloud_base_address` |
| `Unknown` | error |

**Path** (`CreateUri`, standard mode):

```
agent_path = "dataverse-backed" if agent_type == Published else "prebuilt"
/copilotstudio/{agent_path}/authenticated/bots/{schema_name}/conversations
/copilotstudio/{agent_path}/authenticated/bots/{schema_name}/conversations/{conversation_id}
/copilotstudio/{agent_path}/authenticated/bots/{schema_name}/conversations/{conversation_id}/subscribe
?api-version=2022-03-01-preview
```

**Direct-connect mode** (`CreateUri(baseaddress, …)`): take the supplied absolute URL, strip a
trailing `/` or `\`, cut the path at the first `/conversations`, then append
`/conversations[/{id}[/subscribe]]` and set the query to `api-version=2022-03-01-preview`.

**Resolution rules** (`GetCopilotStudioConnectionUrl`):

- Standard mode requires `environment_id` and `schema_name` (else error).
- `settings.cloud` overrides the default `Prod` unless it is `Unknown`.
- `cloud == Other` requires an absolute `cloud_base_address`, or a `custom_power_platform_cloud`
  that parses as a URI (relative or absolute), else error.
- `settings.copilot_agent_type` overrides the default `Published`.
- Direct-connect mode requires an absolute URL, else error.

Test vectors (from `CopilotClientTests.VerifyConnectionUrl`, `env = A47151CF-4F34-488F-B377-EBE84E17B478`):

| cloud | type | custom | conv | expected |
|---|---|---|---|---|
| Other | Published | `foo.api.com` | – | `https://a47151cf4f34488fb377ebe84e17b47.8.environment.foo.api.com/copilotstudio/dataverse-backed/authenticated/bots/Bot01/conversations?api-version=2022-03-01-preview` |
| Preprod | Published | – | – | `https://a47151cf4f34488fb377ebe84e17b47.8.environment.api.preprod.powerplatform.com/…/conversations?api-version=…` |
| Prod | Published | – | – | `https://a47151cf4f34488fb377ebe84e17b4.78.environment.api.powerplatform.com/…/conversations?api-version=…` |
| FirstRelease | Published | – | `1234` | `https://a47151cf4f34488fb377ebe84e17b4.78.environment.api.powerplatform.com/…/conversations/1234?api-version=…` |
| Prod | Prebuilt | – | `1234` | `https://a47151cf4f34488fb377ebe84e17b4.78.environment.api.powerplatform.com/copilotstudio/prebuilt/authenticated/bots/Bot01/conversations/1234?api-version=…` |
| Other | Prebuilt | `Blah+1_ Blah` | `1234` | **error** |

Path-segment encoding: the JS client percent-encodes `schema_name` and `conversation_id` as path
segments (`encodeURIComponent`); .NET/Python interpolate raw. The port encodes (§6).

### 3.2 Token audience (`GetTokenAudience` / `ScopeFromSettings`)

```
standard mode:      https://{endpoint_suffix(cloud)}/.default
direct-connect:     decode cloud from the URL host (table below); if Unknown, fall back to
                    settings.cloud / explicit cloud; if that is Other or Unknown → error.
```

Host → cloud decode (`DecodeCloudFromURI`): the same suffix table, matched on the lower-cased host,
except `Ex`/`Rx`/`Other` are never decoded and `api.gov.powerplatform.microsoft.us` decodes to
`GovFR`. Anything else → `Unknown`.

Vectors (`VerifyAgentScopeTest`): Prod → `https://api.powerplatform.com/.default`; Preprod →
`https://api.preprod.powerplatform.com/.default`; Mooncake →
`https://api.powerplatform.partner.microsoftonline.cn/.default`; FirstRelease →
`https://api.powerplatform.com/.default`; Other + `fido.com` → `https://fido.com/.default`;
Unknown → error. Direct-connect `https://api.powerplatform.com/copilotstudio/bots/test-bot` →
`https://api.powerplatform.com/.default`.

The audience is what the caller passes to MSAL/Entra as the scope. The Entra permission behind it is
**Power Platform API → CopilotStudio → `CopilotStudio.Copilots.Invoke`** (delegated for user /
OBO flows, application for app-only S2S).

### 3.3 HTTP requests

| Operation | Method | URL | Body | Extra headers |
|---|---|---|---|---|
| Start conversation | `POST` | `…/conversations` | `StartRequest` | `x-ms-conversation-id: {id}` when `StartRequest.conversation_id` is set (.NET) |
| Send activity / execute turn | `POST` | `…/conversations/{id}` | `ExecuteTurnRequest` | – |
| Subscribe (Microsoft-internal at this time) | `GET` | `…/conversations/{id}/subscribe` | – | `Last-Event-ID: {id}` when resuming |

Common request headers on every call:

```
Authorization: Bearer {token}
Accept:        text/event-stream
Content-Type:  application/json          (POST only)
User-Agent:    CopilotStudioClient.agents-sdk-{lang}/{version} {runtime}/{version} {os}/{version}
```

Response headers consumed:

| Header | Meaning |
|---|---|
| `x-ms-conversationid` | The conversation id assigned by the service; becomes the client's current conversation. |
| `x-ms-d2e-experimental` | Island-specific "experimental" URL. Captured **only** when `use_experimental_endpoint` is set **and** no `direct_connect_url` was configured; it then becomes the effective direct-connect URL for the rest of the client's life. |

Other header names defined upstream (`CopilotStudioHeaderNames`) but not used by the D2E client at
this pin: `x-ms-client-request-id`, `x-ms-correlation-id`, `x-cci-agent-version`,
`Accept-Language`. They are exposed as constants for parity, not sent.

### 3.4 Request bodies

```jsonc
// StartRequest
{ "emitStartConversationEvent": true, "locale": "en-US", "conversationId": "…" }   // locale/conversationId omitted when unset

// ExecuteTurnRequest
{ "activity": { …Activity… } }
```

### 3.5 Responses

**Expected path — `Content-Type: text/event-stream`.** Standard Server-Sent Events. Only events with
`event: activity` are surfaced; their `data:` is one JSON `Activity`. The `id:` field (when present)
is the SSE event id, surfaced by `subscribe` for resumption. Any other event type is ignored.

```
event: activity
data: {"type":"typing","conversation":{"id":"…"}, …}

event: activity
data: {"type":"message","text":"Hello","conversation":{"id":"…"}, …}
```

**Fallback path — anything else.** .NET logs a warning and parses the whole body as JSON:
`StartResponse { activities, conversationId? }`, `ExecuteTurnResponse { activities }`,
`SubscribeResponse { activities }`, each yielding `activities` in order. Python/JS have no fallback.
The port implements the .NET fallback.

**Errors.** Any non-2xx status is an error carrying the status and the response body text (.NET
reads the body; Python/JS surface the status only).

### 3.6 Conversation-id tracking

The client keeps a "current conversation id" so callers can omit it:

1. `x-ms-conversationid` on any response sets it.
2. While streaming, the first `message` activity sets it **only if still empty** (.NET/JS; Python
   overwrites on every message — not mirrored).
3. `start_conversation(…)` resets it to `StartRequest.conversation_id` or empty before the request
   (JS; .NET does not reset — the port mirrors JS, see §6).
4. `execute(conversation_id, activity)` forces `activity.conversation.id = conversation_id`
   (.NET/Python) and makes it current (JS).
5. `send_activity(activity)` uses `activity.conversation.id` when present, else the current id.
6. `ask_question(text, conversation_id?)` builds a `message` activity with the explicit or current id.

### 3.7 Streaming text ("streaminfo")

Agents that stream tokens emit `typing` activities carrying a `streaminfo` entity
(`{ "type": "streaminfo", "streamType": "streaming", "streamId": "…", "streamSequence": n }`) — or,
in the legacy shape, the same fields in `channelData` — followed by a final `message`. The JS
client accumulates chunk text by `(streamId, streamSequence)` and rewrites `activity.text`; .NET and
Python yield activities untouched. The port yields activities untouched (primary behaviour) and
exposes `Activity::stream_info()` so callers can accumulate if they want to.

### 3.8 Activity schema (Bot Framework "Activity Protocol")

Mirrored from `microsoft_agents.activity.Activity` / `Microsoft.Agents.Core.Models.Activity`.
Serialisation rules that matter on the wire:

- camelCase property names; `from_property` ↔ `from`; `ConversationReference.agent` ↔ `bot`.
- Absent/null fields are omitted (.NET `WhenWritingNull`, Python `exclude_unset`).
- `channelId` is a string `"channel[:subChannel]"`.
- `entities[]` are open objects keyed by `type`; well-known types (`streaminfo`, `mention`, …)
  are typed views over the same JSON. Unknown entity types are preserved verbatim.
- Unknown top-level properties are preserved (.NET `Properties` extension data).
- `channelData`, `value`, `attachments[].content` are opaque JSON.

Fields: `type` (required), `id`, `timestamp`, `localTimestamp`, `localTimezone`, `serviceUrl`,
`channelId`, `from`, `conversation`, `recipient`, `textFormat`, `attachmentLayout`,
`membersAdded`, `membersRemoved`, `reactionsAdded`, `reactionsRemoved`, `topicName`,
`historyDisclosed`, `locale`, `text`, `speak`, `inputHint`, `summary`, `suggestedActions`,
`attachments`, `entities`, `channelData`, `action`, `replyToId`, `label`, `valueType`, `value`,
`name`, `relatesTo`, `code`, `expiration`, `importance`, `deliveryMode`, `listenFor`,
`textHighlights`, `semanticAction`, `callerId`.

Nested: `ChannelAccount { id, name, aadObjectId, role, agenticUserId, agenticAppId, tenantId, +extra }`,
`ConversationAccount { isGroup, conversationType, id, name, aadObjectId, role, tenantId, properties }`,
`ConversationReference { activityId, user, bot, conversation, channelId, locale, serviceUrl }`,
`Attachment { contentType, contentUrl, content, name, thumbnailUrl }`,
`SuggestedActions { to, actions }`, `CardAction { type, title, image, text, displayText, value, channelData, imageAltText }`,
`MessageReaction { type }`, `TextHighlight { text, occurrence }`, `SemanticAction { id, entities, state }`,
`Entity { type, +extra }`, `StreamInfo`.

Well-known string sets are exposed as constants modules (`activity_types`, `input_hints`,
`delivery_modes`, `end_of_conversation_codes`, `text_format_types`, `attachment_layout_types`,
`role_types`, `entity_types`); `type` itself is a closed-plus-`Other(String)` enum for ergonomic
matching.

---

## 4. Auth model

The reference client is **token-agnostic**: .NET takes a `Func<string /*request url*/, Task<string>>`
token provider (or relies on an `HttpClient` handler), Python/JS take a bearer string. Token
acquisition (MSAL public client, confidential client, OBO) lives entirely in the samples. The port
keeps that boundary:

```rust
pub trait TokenProvider: Send + Sync {
    /// Called once per request with the fully-resolved request URL (mirrors .NET).
    fn access_token(&self, request_url: &str) -> BoxFuture<'_, Result<String, BoxError>>;
}
pub struct StaticToken(String);   // Python / JS constructor shape
```

Supported flows (all outside the crate):

| Flow | Token | Agent auth mode | Notes |
|---|---|---|---|
| Delegated, interactive | user token for `https://api.powerplatform.com/.default` (or `…/CopilotStudio.Copilots.Invoke`) | any | Upstream console samples. |
| **Delegated, On-Behalf-Of** | service exchanges the user's inbound token for one with the Power Platform audience via the Entra OBO grant (`grant_type=urn:ietf:params:oauth:grant-type:jwt-bearer`, `requested_token_use=on_behalf_of`, `scope=https://api.powerplatform.com/.default`) | integrated auth | Upstream `MsalAuth.acquire_token_on_behalf_of`. The exchanged token is what the caller hands to this crate. The user's identity flows through; the service must be shared on the agent (viewer) and the user must have access. |
| App-only (S2S) | client-credentials token | "No authentication" only | Private preview upstream; the client is identical, only the token differs. |

Constraints inherited from upstream docs: same-tenant only; identity must stay consistent across
turns of one conversation (`CallerIdentityMismatch` / `CallerIdentityTypeMismatch` otherwise).

The crate provides `scope_from_settings(&ConnectionSettings) -> Result<String>` and
`scope_from_cloud(PowerPlatformCloud) -> Result<String>` so the caller asks Entra for exactly the
audience the client will hit — the only auth-related code that belongs in the client.

Tokens are treated as secrets: never logged (diagnostics redact `Authorization`), `Debug` on the
client and on `StaticToken` prints `[REDACTED]`.

---

## 5. Crate layout

Single library crate `copilotstudio-client` (lib target `copilotstudio_client`). Module map from
the reference structure:

| Rust module | Reference | Contents |
|---|---|---|
| `lib.rs` | `__init__.py` / `index.ts` | Re-exports; `UPSTREAM` pin constant. |
| `power_platform_cloud` | `PowerPlatformCloud` | Enum + `FromStr`/`Display`; string values as upstream (`Prod`, `GovFR`, …). |
| `agent_type` | `AgentType` | `Published` / `Prebuilt`. |
| `connection_settings` | `ConnectionSettings` | Settings struct + builder + `from_env()`. |
| `power_platform_environment` | `PowerPlatformEnvironment` | Pure URL / audience functions (§3.1–3.2). No I/O. |
| `headers` | `CopilotStudioHeaderNames` | Header-name constants. |
| `user_agent` | `UserAgentHelper` | `User-Agent` string. |
| `token` | .NET `tokenProviderFunction` | `TokenProvider`, `StaticToken`. |
| `models` | `Models/*` | `StartRequest`, `ExecuteTurnRequest`, `StartResponse`, `ExecuteTurnResponse`, `SubscribeResponse`, `SubscribeEvent`. |
| `sse` | `System.Net.ServerSentEvents` / `eventsource-client` | Byte-stream → SSE event parser. |
| `client` | `CopilotClient` | The client. |
| `error` | `errors/`, `errorHelper.ts` | `Error` + `SettingsError`. |
| `activity` | `microsoft-agents-activity` | Activity schema (§3.8). Candidate for a separate crate once it grows beyond what the client needs. |

Not ported (documented so it is a decision, not an omission):

- `OrchestratedClient` / `IOrchestratedClient` (.NET) — "ExternalOrchestration API … intended for
  internal use only". Different endpoint family (`/powervirtualagents/orchestrated/…`).
- `CopilotStudioWebChat` (JS) — a Bot Framework WebChat adapter, browser-only.
- Telemetry/OpenTelemetry spans (JS `observability/`) — replaced by `tracing` spans.
- `populate_from_environment` / `loadCopilotStudioConnectionSettingsFromEnv` — ported as
  `ConnectionSettings::from_env()` with the Python variable names.

Dependencies: `tokio` (runtime), `reqwest` (HTTP, `stream` + `json`, rustls), `serde`/`serde_json`,
`futures-core`/`futures-util`, `async-stream`, `bytes`, `url`, `thiserror`, `tracing`, `http`
(re-exported `StatusCode`/`HeaderName`). No WebSocket dependency: D2E is HTTP+SSE only; WebSocket
arrives with the Direct Line transport (§11).

---

## 6. Deliberate divergences

| # | Topic | Upstream | Port | Why |
|---|---|---|---|---|
| D1 | Conversation id reset on start | .NET keeps the previous id; JS resets to `StartRequest.conversation_id` / empty | JS | With .NET's "only set if empty" rule, a second start on the same client keeps a stale id when the response carries no header. JS fixed this; Python is unaffected only because it overwrites on every message. |
| D2 | Path-segment encoding | JS `encodeURIComponent`; .NET/Python raw | encode | Raw interpolation of `/`, `?`, `#` produces a different resource. Encoding is a no-op for well-formed schema names and ids. |
| D3 | `subscribe` method | .NET `POST {}`; Python/JS `GET` | `GET` | Newer clients; SSE resumption via `Last-Event-ID` is a GET idiom. .NET marks the API obsolete/internal. |
| D4 | Streaming text accumulation | JS mutates `typing` activities' `text`; .NET/Python don't | don't; expose `stream_info()` | Mutating activities hides the raw protocol; accumulation is a caller-side concern. |
| D5 | Timestamps | typed `datetime` / `DateTimeOffset` | `String` (RFC 3339 as sent) | A strict typed parse would fail whole activities on any format drift; a typed accessor can be added without a wire change. |
| D6 | Diagnostics | `EnableDiagnostics` prints URL + all response headers | same, via `tracing::debug!`, `Authorization` redacted | Never log bearer tokens. |
| D7 | `StartRequest.conversation_id` transport | .NET body + `x-ms-conversation-id` header; JS URL path; Python body only | .NET | Origin implementation; Python's model mirrors it. |
| D8 | HTTP error detail | .NET includes the body; Python/JS status only | .NET | Strictly more information. |
| D9 | Non-SSE response | .NET JSON fallback; Python/JS parse SSE regardless | .NET | Origin implementation. |
| D10 | Final SSE event without trailing blank line | spec (and .NET parser) discards; Python's line parser emits | emit at EOF | Superset that matches Python; the service terminates streams cleanly either way. |
| D11 | `ask_question` conversation account | .NET may send `conversation: {}` when no id is known | always send the resolved id (Python) | Strictly more informative; identical when an id is known. |
| D12 | `error` codes | Python `-650xx`, JS `-1400xx` numeric codes | none; typed enum variants with upstream messages | Codes differ between clients; the enum is the Rust idiom. |

Anything not listed here that differs from the pinned .NET behaviour is a bug.

---

## 7. Public API (Rust)

```rust
// Settings
let settings = ConnectionSettings::new("env-id", "schema-name")
    .cloud(PowerPlatformCloud::Prod)              // default Prod
    .agent_type(AgentType::Published)             // default Published
    .custom_power_platform_cloud("api.example")   // only with Cloud::Other
    .direct_connect_url("https://…")              // overrides everything
    .use_experimental_endpoint(false)
    .enable_diagnostics(false);
let settings = ConnectionSettings::from_env()?;   // ENVIRONMENT_ID, SCHEMA_NAME|AGENT_IDENTIFIER, CLOUD, …

// Pure helpers
let scope: String = scope_from_settings(&settings)?;
let url:   Url    = connection_url(&settings, Some("conv-id"))?;

// Client
let client = CopilotClient::new(settings, "eyJ…");                     // static token
let client = CopilotClient::builder(settings).token_provider(p).http_client(c).build();

// Conversation — every call returns a stream of activities
let mut s = client.start_conversation(true);
let mut s = client.start_conversation_with_request(StartRequest { locale: Some("en-US".into()), ..Default::default() });
let mut s = client.ask_question("hello", None);
let mut s = client.send_activity(activity);
let mut s = client.execute("conv-id", activity);
let mut s = client.subscribe("conv-id", None);     // Stream<Item = Result<SubscribeEvent>>
while let Some(activity) = s.try_next().await? { … }

client.conversation_id() -> Option<String>
client.settings() -> &ConnectionSettings

pub type ActivityStream<'a> = Pin<Box<dyn Stream<Item = Result<Activity, Error>> + Send + 'a>>;
```

Naming maps 1:1 to the upstream names in snake_case (`start_conversation`, `ask_question`,
`send_activity`, `execute`, `subscribe`, `scope_from_settings`, `scope_from_cloud`). The
deprecated aliases (`ask_question_with_activity`, JS `*Async` non-streaming forms) are not ported;
collecting a stream is `s.try_collect::<Vec<_>>().await`.

---

## 8. Async & concurrency model

- `tokio` runtime, `reqwest` client (connection pool shared through the builder or a caller-supplied
  `reqwest::Client`, mirroring `IHttpClientFactory` / `client_session_settings`).
- Each operation is one HTTP request whose body is consumed lazily as a `Stream`. The stream is
  `Send + 'a` (borrows the client) so it can be driven from spawned tasks via `Arc<CopilotClient>`.
- **Cancellation = drop.** Dropping the stream aborts the request; no cancellation-token parameter.
- The client is `Send + Sync`; mutable state (current conversation id, captured experimental URL)
  sits behind a `std::sync::Mutex` that is never held across an `await`. Concurrent turns on one
  client are allowed, as upstream, and share the conversation-id slot exactly as upstream does.
- The SSE parser is incremental: it consumes `bytes::Bytes` chunks as they arrive and yields events
  as soon as a blank line (or EOF) completes them, so streamed `typing` chunks surface live.

---

## 9. Error model

Enums are exhaustive (no `#[non_exhaustive]`) so callers get compile-time coverage; adding a
variant is a breaking change and bumps the 0.x minor.

```rust
pub enum Error {
    Settings(SettingsError),                   // URL/audience resolution failed (pure)
    InvalidArgument(&'static str),             // e.g. "conversation_id cannot be empty"
    Token(BoxError),                           // TokenProvider failed
    Http(reqwest::Error),                      // transport
    RequestFailed { status: StatusCode, body: String },   // non-2xx
    Decode { context: &'static str, source: serde_json::Error },   // SSE `data:` or JSON fallback body
}

pub enum SettingsError {
    CloudBaseAddressRequired, EnvironmentIdRequired, AgentIdentifierRequired,
    CustomCloudOrBaseAddressRequired, SettingsOrCloudRequired, InvalidDirectConnectUrl,
    UnableToResolveCloudFromDirectConnectUrl, InvalidCloudCategory(PowerPlatformCloud),
}
```

Messages are the upstream messages verbatim so logs line up across SDKs.

---

## 10. Pin-bump procedure

1. Choose the new tags for all three repos (they release independently; pick the latest of each).
2. Diff the three client directories between old and new tags; list every behavioural change.
3. Update §2, `UPSTREAM.toml`, and the `UPSTREAM` constant.
4. Port each change; add or update the parity test that pins it; update §6 if a divergence is
   introduced or resolved.
5. Regenerate the parity vectors that come from upstream tests (§11.1).
6. Changelog entry `Upstream: .NET vX → vY, Python …, JS …`.

---

## 11. Test strategy

### 11.1 Mirror-parity tests (pure, no network)

- URL construction: every `VerifyConnectionUrl` and JS strategy vector (§3.1), plus the Python
  direct-connect normalisation cases (`/conversations` stripping, subscribe links).
- Audience: every `VerifyAgentScopeTest` vector and the JS direct-connect audience vector (§3.2).
- Cloud decode: `api.powerplatform.com` → Prod, `api.gov.powerplatform.microsoft.us` → GovFR,
  unknown host → Unknown.
- Activity round-trip: fixtures serialised by the upstream Python model (`exclude_unset`) must
  deserialise and re-serialise byte-identically (modulo key order) — proves camelCase/omission rules,
  `from`/`bot` aliases, entity preservation, extension data.
- Request bodies: `StartRequest` / `ExecuteTurnRequest` JSON matches the upstream shapes.
- SSE parser: spec cases (multi-line `data`, comments, `\r\n`, chunk boundaries splitting a line,
  `id` handling, non-`activity` events ignored, EOF flush).

### 11.2 Client behaviour tests (mock HTTP server, `wiremock`)

- Start → SSE stream yields activities; `x-ms-conversationid` header captured; subsequent
  `ask_question` posts to `/conversations/{id}` with the expected body and headers (`Authorization`,
  `Accept`, `Content-Type`, `User-Agent`).
- Conversation-id precedence (header > first message; reset on start; `execute` forces).
- Experimental URL capture: captured only when enabled and no direct URL configured (the three
  Python tests, ported).
- JSON fallback bodies for start / execute / subscribe.
- Non-2xx → `RequestFailed { status, body }`.
- Token provider invoked per request with the request URL; provider error surfaces as `Error::Token`.
- `subscribe` sends `GET` + `Last-Event-ID` and surfaces `event_id`.

### 11.3 Live smoke test (opt-in, `#[ignore]`)

`tests/live.rs` runs only when `COPILOTSTUDIO_ENVIRONMENT_ID`, `COPILOTSTUDIO_SCHEMA_NAME` and
`COPILOTSTUDIO_TOKEN` are set: start a conversation, expect at least one activity, send one message,
expect at least one `message` activity back with a non-empty `text`. `examples/console.rs` mirrors
the upstream console samples (device-code login through Entra to obtain the token, then a REPL) and
doubles as the manual smoke test.

---

## 12. Roadmap (post-v0)

- **Direct Line fallback transport** (REST + WebSocket streaming) behind a `Transport` seam, for
  agents published to Azure Bot Service instead of D2E. Adds the WebSocket dependency.
- Split `activity` into its own crate if it grows towards the full Bot Framework schema (cards,
  Teams extensions).
- `time`-typed timestamp accessors (D5) once real-world formats are confirmed.
- `StreamAccumulator` helper for streamed text (D4).
- Optional `retry`/backoff layer — deliberately absent from upstream; if added it stays opt-in.
