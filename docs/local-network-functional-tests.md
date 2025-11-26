# Running wasm/js Evo functional tests against a local dashmate network

Context and gaps to close before we can point the wasm-sdk and js-evo-sdk functional suites at a dashmate `local` network instead of public testnet.

## What exists on dashmate local

- Platform boots with system contracts already present (e.g., DPNS contract `GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec`, withdrawals contract) once `waitForNodeToBeReady` passes.
- Local preset exposes HTTPS DAPI on port `2443` with a self-signed certificate (`platform.gateway.listeners.dapiAndDrive.port` in the local config).
- LLMQs are formed automatically via the `local_seed` miner node; no external quorum endpoints.

## Current blockers

- SDK network support is limited to `mainnet`/`testnet`:
  - `rs-sdk-trusted-context-provider` rejects `Network::Regtest` and always fetches quorums from `quorums.{mainnet,testnet}.networks.dash.org`.
  - wasm-sdk and js-evo-sdk builders only accept `mainnet`/`testnet` and default to hardcoded testnet address lists.
- Functional tests hardcode testnet data:
  - js-evo fixtures (`packages/js-evo-sdk/tests/fixtures/testnet.mjs`) contain testnet identities, contracts, tokens, proTx hash, epoch numbers, usernames, etc., and tests call `EvoSDK.testnetTrusted()`.
  - wasm-sdk functional tests all call `WasmSdkBuilder.testnetTrusted()` and assert against the same testnet IDs.
- TLS: local DAPI uses a self-signed cert; the wasm/JS surface does not currently expose a way to inject a CA or bypass verification when custom addresses are used.

## What needs to change

1) **Add local/regtest network path**
   - Extend `rs-sdk-trusted-context-provider` to handle `Network::Regtest` (or add a “local” mode) with a local quorum source or a way to skip trusted prefetch for proofs.
   - Update wasm-sdk/js-evo-sdk builders and type unions to accept `local/regtest`, and allow constructing builders without hitting testnet quorum endpoints when custom addresses are provided.

2) **Allow local DAPI connectivity**
   - Dashmate already issues self-signed certificates for the local preset; we only need to point the SDKs to the local HTTPS endpoint (e.g., `https://127.0.0.1:2443`) without custom CA plumbing.
   - Prefer env-driven address overrides (e.g., `EVO_DAPI_ADDRESSES`) so CI/local runs can switch networks without code edits.

3) **Seed test data on local**
   - System contracts are present, but testnet-only fixtures (identity `5DbL…`, token contract `H7FR…`, group contract `49PJ…`, usernames, epoch numbers, proTx hash) do not exist on a fresh local network.
   - Add a seeding step/script (can reuse platform-test-suite helpers) that, after dashmate start, creates:
     - A funded identity with keys.
     - DPNS names, known documents.
     - Token/group contracts and balances needed by assertions.
   - Emit a generated fixture file (e.g., `fixtures/local.mjs`) consumed by both wasm-sdk and js-evo-sdk tests.
   - Use the existing `SDK_TEST_DATA=true yarn start` mechanism to have dashmate seed test data automatically during network startup, then extract the IDs into the local fixtures for wasm/js-evo functional suites.
   - For wasm-sdk specifically, required IDs/contracts are summarized in `packages/wasm-sdk/tests/functional/fixtures/requiredTestData.mjs`; ensure the seeding path populates equivalents on local. The `SDK_TEST_DATA=true` hook seeds sample identities/contracts used by SDK tests—align local fixtures to those outputs.

4) **Make tests local-first**
   - Replace hardcoded `testnet` builders with a local/regtest option and env-driven address selection; functional suites should target the local dashmate network when run from this repo.
   - Replace fixed IDs/proTx hashes/epoch numbers in assertions with the seeded fixture values or live queries against the local node.

## Trusted quorum options for local

- **Run a local “quorums.*” HTTP service (mirroring testnet)**  
  Would mimic `https://quorums.testnet.networks.dash.org` locally. This means building and packaging an extra service (or dashmate plugin) to scrape quorum info from the local network and expose `/quorums`/`/previous-quorums` endpoints, plus wiring TLS/ports. This is the closest analogue to testnet behavior but adds deployment and maintenance overhead.
- **Quorum list server repo**  
  The public endpoints are served from <https://github.com/dashpay/quorum-list-server>. To support a local mirror, we’ll need to extend that repo with a Dockerfile and CI pipeline so dashmate (or CI) can spin up a local quorum-list instance against the local network.
- **Add a local/regtest path in the trusted context provider (or skip trusted quorums on local)**  
  Instead of hitting a quorums endpoint, add network-specific logic so that when `network=local/regtest` the SDK/context skips trusted quorum fetching (or uses a minimal local source) and still functions for read-only queries. This avoids needing any quorum-list service but would mean proofs/trusted flows may be limited or disabled on local.

### Decision: use sidecar quorum service
- Implement and publish a Dockerized `quorum-list-server` (see <https://github.com/dashpay/quorum-list-server>) and integrate it into dashmate’s `local` preset docker-compose (wired to Core RPC inside the network; expose HTTPS for SDKs).
- Update SDKs/providers to accept a local quorum base URL (or auto-point `network=local` to the dashmate service) so trusted/proof flows work unchanged on local.
- Keep the skip-path as a fallback only if the sidecar is unavailable; primary path is proof-parity via the local quorum service.

### Dashmate integration notes (in progress)
- We will ship the quorum-list server as a dashmate-managed service, backed by the new Docker image (build almost done). It must be **disabled by default** and only turned on for the `local_seed` node when `dashmate setup local` runs, so regular presets/users are unaffected.
- Add the service to the local preset docker-compose, sourcing Core RPC credentials/host from the existing `local_seed` config and exposing an HTTPS port to the host for SDKs/tests to hit. Keep other nodes unaware of it unless explicitly enabled.
- Provide a config toggle/flag (e.g., `--enable-quorum-list` or a preset-scoped setting) so CI/local flows can opt in; `dashmate status` should surface whether the service is running when the local preset is active.
- When enabled, ensure the local SDK network config points to this service by default (either via preset defaults or env var wiring) so wasm/js functional suites don’t need manual URLs.
- Config toggle landed: set `platform.quorumList.enabled=true` (profile `platform-quorum`, defaults to port `2444` bound to `127.0.0.1`; the sidecar binds internally to `0.0.0.0`). Local setup now enables it on `local_seed` and wires Core RPC credentials from the dedicated `quorum_list` RPC user. Compose service name: `quorum_list`.

## Temporary guidance

- Until the above changes are implemented, do **not** run the functional suites in `packages/js-evo-sdk` or `packages/wasm-sdk` when invoking their `test` scripts; they assume public testnet data and will fail against a local dashmate network.

## Suggested bootstrap flow

1. Start dashmate local (`yarn dashmate setup local && yarn dashmate start`) and wait for readiness.
2. Run a seeding script to create identity/contracts/documents and write `fixtures/local.mjs` (and equivalent for wasm tests).
3. Export env vars (addresses, CA/insecure flag, fixture path) and run the wasm/js-evo functional suites with the new `local` network option.
