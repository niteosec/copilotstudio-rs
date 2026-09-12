# Live run — talking to a real Copilot Studio agent

What the mock tests cannot prove: that the service accepts what we send and that we decode what it
streams. This is the runbook for the opt-in live smoke test and the console example. Every portal
step below is the same one the upstream console samples' READMEs prescribe
(`Agents-for-net/src/samples/CopilotStudioClient/CopilotStudioClient/README.md`).

## 1. Tenant prerequisites (once)

- A Microsoft 365 tenant with a **Copilot Studio** license or trial (the standalone Copilot Studio
  trial is enough; Microsoft 365 Copilot alone does not publish agents to the native-app channel).
- Permission to create an **app registration** in that tenant's Entra ID.
- The **Power Platform API** must be visible under *APIs my organization uses*. If it is not,
  register its service principal once (tenant admin, PowerShell):
  `New-MgServicePrincipal -AppId 8578e004-a5c6-46e7-913e-12f58912df43` — see
  [Power Platform API authentication, step 2](https://learn.microsoft.com/power-platform/admin/programmability-authentication-v2#step-2-configure-api-permissions).

## 2. The agent (Copilot Studio, ~5 min)

1. <https://copilotstudio.microsoft.com> → **Create** an agent (any template; a blank agent with the
   default greeting topic is enough).
2. **Publish** it.
3. **Settings → Advanced → Metadata**: note **Schema name** and **Environment Id**.
4. Settings → Security → Authentication: leave **Authenticate with Microsoft** (the default). The
   signed-in user's identity then flows through; they must be the agent's owner or have it shared
   with them. ("No authentication" also works over D2E — the transport is Entra-only regardless.)

## 3. The app registration (Entra ID, ~5 min)

1. Entra ID → App registrations → **New registration**: any name; *Accounts in this organizational
   directory only*; platform **Public client/native (mobile & desktop)**; redirect URI
   `http://localhost` (HTTP, as upstream prescribes). Register.
2. Overview: note the **Application (client) ID** and **Directory (tenant) ID**.
3. **API permissions → Add a permission → APIs my organization uses → Power Platform API →
   Delegated permissions → CopilotStudio → `CopilotStudio.Copilots.Invoke`** → Add. Then
   **Grant admin consent** (otherwise the first sign-in shows a consent prompt, which also works if
   users may consent).
4. **Authentication → Advanced settings → Allow public client flows → Yes** (the device-code flow
   the examples use; upstream calls this "enable the following mobile and desktop flows").

That is the entire auth footprint: a public client with one delegated permission. No secret exists.

## 4. Run

```sh
export ENVIRONMENT_ID='…'  SCHEMA_NAME='…'  TENANT_ID='…'  APP_CLIENT_ID='…'
# optional: CLOUD=Prod|Preprod|Gov|… (default Prod), COPILOT_AGENT_TYPE=Published|Prebuilt, LOCALE=en-US, ENABLE_DIAGNOSTICS=true

# Interactive chat (device-code sign-in, then a REPL) — the twin of the upstream console samples:
cargo run --example console

# Or the smoke test: sign in once, capture the token, run the ignored test:
export COPILOTSTUDIO_TOKEN=$(cargo run -q --example token)
cargo test --test live -- --ignored --nocapture
```

`token` prints the sign-in instructions on stderr ("open https://microsoft.com/devicelogin and enter
the code …") and the bearer token — nothing else — on stdout. Tokens are short-lived (about an
hour) and never written by the crate; keep them in the environment of the process that needs them.

The smoke test starts a conversation, expects at least one activity and a conversation id, sends
one message, and expects a `message` activity with text back. With `ENABLE_DIAGNOSTICS=true` the
request URLs and response headers (never `Authorization`) are logged at debug level.

## 5. What can go wrong

| Symptom | Cause |
|---|---|
| `AADSTS65001` / consent prompt on sign-in | Admin consent not granted for `CopilotStudio.Copilots.Invoke`; grant it or let the user consent. |
| `AADSTS7000218` or "public client" errors | *Allow public client flows* is off (§3.4). |
| `AADSTS500011` / resource not found | Power Platform API service principal missing in the tenant (§1). |
| `401` | Token audience ≠ the agent's cloud; compare `scope_from_settings` (printed by `token`) with `CLOUD`. |
| `403` with `D2EAccessDenied` | The signed-in user cannot access the agent (not owner / not shared), or token tenant ≠ agent tenant. |
| `404` | Wrong `ENVIRONMENT_ID` / `SCHEMA_NAME` / `CLOUD` / agent type, or the agent is not published. |
| `CallerIdentityMismatch` mid-conversation | A different user or app continued a conversation someone else started (§S2S doc guardrails). |
