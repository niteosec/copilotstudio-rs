# Changelog

All notable changes to this crate are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions follow SemVer (0.x: minor bumps may break).

## [Unreleased]

### Added
- Initial port of the Copilot Studio Direct-to-Engine client.
  Upstream: .NET `v1.8.77` (primary), Python `v1.5.0`, JS `v1.8`.
- `ConnectionSettings` (+ `from_env`), `PowerPlatformCloud`, `AgentType`.
- URL and token-audience resolution for every Power Platform cloud, direct-connect mode, subscribe links.
- `CopilotClient`: `start_conversation`, `start_conversation_with_request`, `ask_question`,
  `send_activity`, `execute`, `subscribe`; SSE streaming with the .NET JSON fallback;
  `x-ms-conversationid` / `x-ms-d2e-experimental` handling; `TokenProvider` / `StaticToken`.
- `CopilotClientApi` trait (`ICopilotClient` / `CopilotClientProtocol`) for substitution in tests.
- Bot Framework `Activity` schema with open entities, `streaminfo` view, extension data.
- Parity tests transcribed from the upstream test suites; activity fixtures generated from the
  upstream Python model; mock-server behaviour tests; a differential harness that runs the pinned
  Python and JS clients against the same recording server; opt-in live smoke test with a runbook
  (`docs/LIVE.md`); `console` and `token` examples.
