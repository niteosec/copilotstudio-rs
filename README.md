# copilotstudio-rs

**Unofficial Rust port of Microsoft's Copilot Studio client** — the client that ships in the
open-source [Microsoft 365 Agents SDK](https://github.com/microsoft/Agents) as
`Microsoft.Agents.CopilotStudio.Client` (.NET), `microsoft-agents-copilotstudio-client` (Python)
and `@microsoft/agents-copilotstudio-client` (JavaScript).

It talks to published Copilot Studio agents over the **Direct-to-Engine** protocol through the
Power Platform API, authenticated with an Entra token for the `CopilotStudio.Copilots.Invoke`
permission. Crate: `copilotstudio-client`.

> This project is not affiliated with or endorsed by Microsoft. It mirrors the official clients'
> public surface and wire behaviour at a **pinned upstream version** (below). The official clients
> are MIT-licensed; see [`NOTICE`](NOTICE) for attribution.

## Pinned upstream

| Client | Tag | Commit | Role |
|---|---|---|---|
| [`microsoft/Agents-for-net`](https://github.com/microsoft/Agents-for-net) | `v1.8.77` | `d2493a5f` | primary reference |
| [`microsoft/Agents-for-python`](https://github.com/microsoft/Agents-for-python) | `v1.5.0` | `d3f427d8` | cross-check |
| [`microsoft/Agents-for-js`](https://github.com/microsoft/Agents-for-js) | `v1.8` | `8f07a596` | cross-check |

Protocol: Power Platform API `api-version=2022-03-01-preview`. The full pin is in
[`UPSTREAM.toml`](UPSTREAM.toml) and exposed as `copilotstudio_client::UPSTREAM`. Pin bumps follow
the procedure in [`docs/DESIGN.md`](docs/DESIGN.md#10-pin-bump-procedure).

## Usage

```rust
use copilotstudio_client::{ConnectionSettings, CopilotClient};
use futures_util::TryStreamExt;

let settings = ConnectionSettings::new("<environment id>", "<agent schema name>");

// 1. Acquire a token OUTSIDE the crate for this audience
//    (Entra: Power Platform API → CopilotStudio.Copilots.Invoke, delegated).
let scope = copilotstudio_client::scope_from_settings(&settings)?;   // https://api.powerplatform.com/.default
let token = acquire_with_msal_or_obo(&scope).await?;                  // your code

// 2. Drive the agent. Every call is a stream of Activities; drop the stream to cancel.
let client = CopilotClient::new(settings, token);
let mut start = client.start_conversation(true);
while let Some(activity) = start.try_next().await? {
    println!("{}: {}", activity.r#type, activity.text.as_deref().unwrap_or(""));
}
let mut reply = client.ask_question("What can you do?", None);
while let Some(activity) = reply.try_next().await? {
    if let Some(text) = activity.text.as_deref() { println!("agent> {text}"); }
}
```

Surface (1:1 with the upstream names): `start_conversation`, `start_conversation_with_request`,
`ask_question`, `send_activity`, `execute`, `subscribe`, `scope_from_settings`, `scope_from_cloud`,
`connection_url`, `ConnectionSettings::from_env`. A `TokenProvider` trait (called per request with
the request URL, like the .NET `tokenProviderFunction`) replaces the fixed token when you need
refresh or exchange; a caller-supplied `reqwest::Client` replaces the default one. The client
contract is also a trait, `CopilotClientApi` (`ICopilotClient` / `CopilotClientProtocol` upstream),
so consumers can hold `Arc<dyn CopilotClientApi>` and substitute a test double.

### Authentication

The crate never acquires tokens. Supported flows, all in the caller:

- **Delegated (interactive)** — MSAL public client, scope = `scope_from_settings(...)`.
- **On-Behalf-Of** — a service exchanges a user's inbound token for one with the Power Platform
  audience (`grant_type=urn:ietf:params:oauth:grant-type:jwt-bearer`,
  `requested_token_use=on_behalf_of`, `scope=https://api.powerplatform.com/.default`) and hands the
  result to this crate. The user's identity flows through to the agent.
- **App-only (S2S)** — client-credentials token; upstream documents this as private preview and
  limited to agents with no end-user authentication.

Tokens are never logged; `Debug` output redacts them.

### Examples and the live run

`examples/console.rs` is the twin of the upstream console samples: device-code sign-in (or a
pre-acquired `COPILOTSTUDIO_TOKEN`), then a chat loop. `examples/token.rs` signs in and prints only
the token, for the smoke test. Tenant setup (agent + a public-client app registration with the
delegated `CopilotStudio.Copilots.Invoke` permission) is in [`docs/LIVE.md`](docs/LIVE.md).

```sh
ENVIRONMENT_ID=… SCHEMA_NAME=… TENANT_ID=… APP_CLIENT_ID=… cargo run --example console
```

## Status

v0 — the surface needed to drive an agent: connection settings, URL/audience resolution for every
Power Platform cloud, start/continue a conversation, send activities as the authenticated user,
receive the agent's activities as a live stream (SSE, with the .NET JSON fallback), `subscribe`,
and the Bot Framework Activity schema. Not ported: the Microsoft-internal `OrchestratedClient`,
the browser WebChat adapter. Direct Line as a fallback transport is on the roadmap.

Deliberate divergences from upstream are enumerated in
[`docs/DESIGN.md` §6](docs/DESIGN.md#6-deliberate-divergences); anything else that differs from the
pinned .NET behaviour is a bug — please file it.

## Testing

```sh
cargo test                                          # parity vectors, activity round-trips, mock-server behaviour
cargo test --test live -- --ignored --nocapture     # live smoke test; needs COPILOTSTUDIO_TOKEN + agent settings
python3 tests/differential/run.py <Agents-for-python> [--js <Agents-for-js>]   # the pinned upstream clients vs this one
```

Activity fixtures under `tests/fixtures/` are generated from the upstream Python model
(`tests/fixtures/generate.py`), so round-trip tests prove parity against the reference itself.
`tests/differential/` goes further: it runs the pinned Python and JS clients and this client
through one scenario against one recording server and diffs every request and every yielded
activity; only the divergences enumerated in the design are allowed.

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.
Derived from `microsoft/Agents`, © Microsoft Corporation, MIT License — see [`NOTICE`](NOTICE).
