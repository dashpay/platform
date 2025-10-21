# rs-dapi TODO & Migration Tracker

This tracker lists remaining work to reach and exceed parity with the legacy JS `packages/dapi` implementation. Items are grouped by area and priority. File paths are included to anchor implementation work.

Legend:
- P0: Required for parity/MVP
- P1: Important for production completeness
- P2: Nice-to-have/cleanup

## P0 — Core gRPC (Layer 1) Parity

- [x] Implement Dash Core RPC client (dashcore-rpc)
  - Files: `src/clients/core_client.rs` (new), `src/config/mod.rs` (Core RPC URL/user/pass)
-  - Implemented so far: `getblockcount`, `getrawtransaction(info)`, `sendrawtransaction`
- [x] Wire Core service methods in `src/services/core_service.rs`
  - [x] `get_best_block_height`
  - [x] `get_block`
  - [x] `get_transaction`
  - [x] `broadcast_transaction`
  - [x] `get_blockchain_status`
  - [x] `get_masternode_status`
  - [x] `get_estimated_transaction_fee`
- [x] Map and standardize error handling to match JS behavior
  - Files: `src/services/core_service.rs`, `src/error.rs`
- [x] Cache immutable Core responses with LRU (invalidate on new block)
  - Files: `src/clients/core_client.rs`, `src/cache.rs`, `src/services/streaming_service/mod.rs`, `src/server.rs`
  - Methods cached inside CoreClient: `get_block_bytes_by_hash(_hex)`; invalidated on ZMQ `hashblock`

## P0 — Platform gRPC (Layer 2) Essentials

- [x] Ensure full Drive-proxy coverage via `drive_method!` in `src/services/platform_service/mod.rs`
  - Cross-check with `packages/dapi-grpc/protos/platform/v0/platform.proto`
- [x] Add caching for `getStatus` with 3-minute TTL and invalidate on new block
  - Files: `src/services/platform_service/get_status.rs`, use ZMQ block notifications or Tenderdash events to invalidate
- [x] Finalize error mapping consistency between `broadcastStateTransition` and `waitForStateTransitionResult`
  - Files: `src/services/platform_service/broadcast_state_transition.rs`, `src/services/platform_service/wait_for_state_transition_result.rs`
  - Align codes/messages with Drive error codes and JS behavior
- [x] Configure gRPC transport robustness (sizes/compression)
  - Increase max inbound message size for large proofs/doc queries
  - Enable compression (e.g., gzip) for client/server
  - Files: `src/clients/drive_client.rs` (client channel), `src/server.rs` (tonic Server builder)

## P0 — Streaming MVP (ZMQ → gRPC)

- [x] Remove panic on ZMQ startup; add retry/backoff and health reporting
  - Files: `src/services/streaming_service/mod.rs`
- [x] Implement historical streaming for `subscribeToBlockHeadersWithChainLocks`
  - Files: `src/services/streaming_service/block_header_stream.rs`
  - Notes: For `count > 0`, stream historical headers (80-byte headers) from Core RPC in chunks and close stream. For `count = 0`, forward live ZMQ Core blocks/chainlocks.
- [x] Implement historical queries for `subscribeToTransactionsWithProofs`
  - Files: `src/services/streaming_service/transaction_stream.rs`
  - Notes: For `count > 0`, fetch blocks from given height/hash, filter transactions via bloom, stream `RawTransactions` plus a block boundary (`RawMerkleBlock` placeholder using raw block), then close. For `count = 0`, optionally backfill to tip then subscribe to live ZMQ.
- [x] Implement basic bloom filter matching + transaction parsing
- [x] Provide initial masternode list diff on subscription
  - Files: `src/services/streaming_service/masternode_list_stream.rs`

## P0 — Protocol Translation Minimums (Parity with JS DAPI)

- [x] JSON-RPC: legacy parity endpoints
  - [x] `getBestBlockHash`
  - [x] `getBlockHash`
  - Files: `src/protocol/jsonrpc_translator.rs`, `src/server.rs` (dispatch)
  - Notes: Translator implemented with tests; server dispatch returns hex strings

## P2 — Protocol Translation (Non-legacy extras)

- [x] JSON-RPC extension: `sendRawTransaction` (not in JS DAPI docs)
  - Files: `src/protocol/jsonrpc_translator.rs`, `src/server.rs`
  - Accepts `hex[, allowHighFees, bypassLimits]`; returns txid string
- [x] JSON-RPC extension: Platform `getStatus` (not in JS DAPI docs)
  - Files: `src/protocol/jsonrpc_translator.rs`, `src/server.rs`

## P1 — Observability & Ops

- [x] gRPC access logging (interceptor) to align with HTTP access logs
  - Files: `src/logging/middleware.rs`, gRPC server builder wiring
- [ ] Prometheus metrics: request counts, latency, errors, subscriber counts
  - Files: `src/server.rs` (`/metrics`), metrics crate integration
- [x] Health check validates upstreams (Drive, Tenderdash RPC, Core RPC) via `/health`
  - Files: `src/server/metrics.rs`, `src/services/platform_service/get_status.rs`

## P1 — Deployment

- [ ] Complete `Dockerfile`, `docker-compose.yml`, and `DOCKER.md`
  - Files: `packages/rs-dapi/docker-compose.yml`, `packages/rs-dapi/DOCKER.md`
- [ ] Provide Envoy/Dashmate integration examples (listeners/clusters/routes)
  - Files: `docs/` or `packages/rs-dapi/doc/`

## P1 — Testing

- [ ] Unit tests for Core and Platform handlers (success + error mapping)
- [ ] Integration tests for Platform broadcast + wait (with/without proofs)
- [ ] Streaming tests: bloom filtering, proofs, subscription lifecycle
- [ ] Protocol translation tests (JSON-RPC ↔ gRPC round-trips)
  - Progress: JSON-RPC translator unit tests added in `src/protocol/jsonrpc_translator.rs`
- [ ] CI workflow to build, test, and lint
- [ ] Drive-proxy smoke tests for all `drive_method!` endpoints
  - Spin up a minimal tonic Platform test server to capture requests and return canned responses
  - Verify passthrough of request/response and metadata; assert cache path hit/miss
- [ ] Proto drift guard (parity check)
  - Add a unit/CI check that enumerates Platform proto RPCs and ensures corresponding service methods exist
  - Fail CI if new RPCs are added to proto without service wiring

## P2 — Documentation

- [ ] Expand README with endpoint matrix and examples
  - Files: `packages/rs-dapi/README.md`
  - Files: `packages/rs-dapi/doc/` (spec + generation notes)
- [ ] Migration guide from JS dapi to rs-dapi, JSON-RPC deprecation scope

## P2 — Cleanup & Consistency

- [ ] Unify error types (`src/error.rs` vs `src/errors/mod.rs`) into one `DapiError`
- [ ] Refactor error conversions: remove `impl From<Box<dyn std::error::Error + Send + Sync>> for DapiError` and map external errors explicitly to `DapiError` across the codebase
  - Files: `src/error.rs`, `src/clients/*`, `src/services/*`, `src/server.rs`
- [ ] Remove remaining `TODO` placeholders or convert into tracked tasks here
- [ ] Harden Tenderdash WebSocket reconnection/backoff
- [ ] Consistent config naming and documentation, align with Dashmate

---

Quick References:
- Core service: `src/services/core_service.rs:1`
- Platform service: `src/services/platform_service/mod.rs:1`
- Streaming service: `src/services/streaming_service/mod.rs:1`
- Protocol translation: `src/protocol/*.rs`
- Server: `src/server.rs:1`
- Config: `src/config/mod.rs:1`
- Clients: `src/clients/*`
