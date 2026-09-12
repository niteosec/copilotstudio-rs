# Differential parity harness

Runs the **pinned upstream clients themselves** and the Rust client against one recording server
(`server.py`), then diffs every request each client sent (method, path, query, protocol headers,
JSON body) and every activity each client yielded. Documented divergences (`docs/DESIGN.md` §6) are
listed in `EXPECTED` inside `run.py` and reported as such; anything else fails the run.

This is the parity evidence behind the port: fixtures and transcribed vectors prove the pure layers,
this proves the client behaviour against the reference implementations. Re-run on every pin bump.

## Python reference (no network needed)

```sh
git clone --branch v1.5.0 https://github.com/microsoft/Agents-for-python /tmp/msagents/Agents-for-python
uv venv /tmp/msagents/venv && uv pip install --python /tmp/msagents/venv/bin/python pydantic pyjwt aiohttp
/tmp/msagents/venv/bin/python tests/differential/run.py /tmp/msagents/Agents-for-python
```

`scenario_python.py` imports the client from the checkout's `libraries/` — the exact tagged source,
not a wheel.

## JavaScript reference (needs `bun`; one-off workspace build)

```sh
git clone --branch v1.8 https://github.com/microsoft/Agents-for-js /tmp/msagents/Agents-for-js
cd /tmp/msagents/Agents-for-js
bun install --ignore-scripts
node_modules/.bin/tsc --build packages/agents-telemetry/tsconfig.json packages/agents-activity/tsconfig.json packages/agents-copilotstudio-client/tsconfig.json
# bun copies the `file:` dependency without its build output; point it at the built package:
rm -rf packages/agents-telemetry/node_modules/@microsoft/agents-activity
ln -s ../../../agents-activity packages/agents-telemetry/node_modules/@microsoft/agents-activity
cd -
/tmp/msagents/venv/bin/python tests/differential/run.py /tmp/msagents/Agents-for-python --js /tmp/msagents/Agents-for-js
```

`scenario_js.ts` is copied into the client package and run with `bun` against the package's
TypeScript source. The JSON-only step is skipped for JS: `eventsource-client` reconnects forever on
a non-SSE 200 (the JS client has no JSON fallback).

## .NET reference

Not automated: the .NET client needs the `dotnet` SDK. The scenario is the same; a port of
`scenario_python.py` to a console app referencing `Microsoft.Agents.CopilotStudio.Client 1.8.77`
would slot into `run.py` the same way. Until then, .NET behaviour is transcribed line-by-line from
`CopilotClient.cs` (§3 of the design) and cross-checked through the two references above.

## Outputs

`target/differential/{rust,python,js}_{requests,steps}.json` — inspect these when a diff fires.
