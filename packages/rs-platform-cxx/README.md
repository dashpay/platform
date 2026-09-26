# dash-platform-cxx

Dash Platform for C++ embedders, as a thin [`cxx`](https://cxx.rs) shell over
`dash-sdk`. The embedder supplies what only it knows: the evonode endpoints
from its masternode list, the Platform quorum keys from its LLMQ store, its
best ChainLock height, and signatures from its wallet. `dash-sdk` supplies
everything else: query construction, transport, retries with address
banning, proof verification, the signed-time window, protocol-version
tracking, contact-request minting. Dash Core's platform GUI is the first
consumer.

## Trust model

Every response byte comes from an untrusted node; nothing in the closure
fetches from a trusted third-party service (no `rs-sdk-trusted-context-provider`,
no HTTP client, no default seed list; CI asserts it). Trust rests on inputs
the embedder pushes:

- **Quorum keys** (`set_quorum_keys`, the full active set, hashes in the
  embedder's internal byte order). The provider (`src/provider.rs`) hands the
  verifier a key only if the proof names the network's Platform LLMQ type and
  a pushed hash; anything else fails signature verification.
- **ChainLock anchor** (`set_chainlock_height`, monotonic). Until the first
  push no proved read is dispatched (`Unavailable`). A proof whose signed
  core-chain-locked height trails the anchor by more than 288 blocks is
  refused as stale; there is no ceiling, since a node one ChainLock ahead of
  the embedder is honest.
- **Chain id** (`Config.tenderdash_chain_id`). After the SDK verified the
  quorum signature and its signed-time window, the shell (`src/ops.rs`)
  compares the signed `chain_id` (`ChainIdMismatch`), then applies its own
  monotonic Platform-height watermark (tolerance 3 blocks, `Rejected`), then
  records the verified protocol version the builders use, then flags a
  `protocol_version` above what this build knows
  (`UnsupportedProtocolVersion`, the value is still returned). The SDK's own
  height watermark is off and the shell keys its watermark and its verified
  version after the chain-id check, so a validly signed proof from another
  chain moves nothing the shell decides on (the SDK's internal version
  ratchet, which runs inside verification, may move upward on it; nothing
  reads that for building).

Absence is proven, never inferred: `ProvenAbsent` comes only after the SDK
verified a proof of it, and carries the same verified metadata as a value
(also under an unsupported protocol version, which `meta` then shows).
Every failure is classified by error variant into `Status.kind`, never by
message text: a node that definitively refuses a request (a gRPC code the
SDK would not retry, including a response above the 4 MiB decoding bound)
is `Rejected`; no answer at all is `Unavailable`. Every bridge entry point runs under `catch_unwind`, so a panic
on hostile input is a `rust::Error` or an `Internal` status, never an
abort; the crate refuses to build with `panic = "abort"`.

## API

Namespace `platform_ffi`; the full declaration is `src/lib.rs`, the
generated header `dash/platform/ffi.h`.

- **Lifecycle**: `new_platform_client(Config{network, tenderdash_chain_id,
  platform_llmq_type})`, `set_endpoints(&[String])` (`https://` only),
  `set_quorum_keys`, `set_chainlock_height`, `shutdown()` (aborts the
  in-flight request, stops the runtime; idempotent, also run on drop).
  The SDK keeps a pooled connection per evonode it has talked to, and only
  dropping the SDK closes them: an empty endpoint set drops it (no socket
  stays open while the embedder has disabled networking), a set that
  removes endpoints rebuilds it over the updated list (retained entries
  keep their ban state), a set that only adds updates it in place. The
  quorum keys, the ChainLock anchor, the height watermark and the verified
  protocol version belong to the client and survive every rebuild.
- **Proved reads**, one SDK request each, returning `Verified*{status, meta,
  value | items + page}`: `get_identity`, `get_identity_by_pubkey_hash`,
  `get_identity_contract_nonce` (masked to the 40-bit value; `ProvenAbsent`
  = the identity has not used the contract, i.e. 0), `resolve_name`,
  `search_names` (prefix of 1 to 63 normalized characters, limit clamped to
  1..=100), `names_of_identity`, `get_profile`, `get_contact_requests` (to
  or from an identity, after a time, oldest first),
  `get_contested_vote_state`. Paged reads return one page of up to their
  limit; `page.has_more` is set on a page that fills it (which may still be
  the last one) and `page.next_start_after` is the cursor for the next call
  (all zero = from the start). Queries use this build's latest compiled-in
  contracts, which decode every older document; the SDK's tracked version
  only matters for building. At protocol version 13 Drive answers a
  continuation of `names_of_identity` with an empty page, so only its first
  100 names are reachable there and a continuation is `Unavailable` rather
  than a last page.
- **Broadcast**: `broadcast(&[u8]) -> BroadcastResult{status}`, advisory and
  typed: `Ok`, `AlreadyExists`, `Consensus` with `consensus_code`, `Rejected`
  (a definitive non-consensus refusal), `Unavailable` (no answer). Every
  write is confirmed by a proved re-query.
- **Builders**, no network, returning `Built{bytes, hash, object_id}`:
  `build_identity_create`, `build_dpns_preorder`, `build_dpns_domain` (the
  caller supplies and persists the salt), `build_profile` (create, or
  replace the `Profile` a `get_profile` returned at its revision + 1: a
  replace carries the whole document, so the avatar and payment-address
  fields another wallet set are carried over unedited),
  `build_contact_request` (minted by `Sdk::create_contact_request` with the
  embedder's ECDH secret). A create carries the document id
  `dash-sdk`'s `put_to_platform` would derive at the network's version
  (from the entropy alone up to protocol version 13, also from the identity
  contract nonce from 14), and a property the network's contract does not
  have yet (DashPay's payment addresses before 14) is refused before
  anything is signed. Transition rules change across protocol
  versions, so every builder (and `contested_vote_fund_credits`) needs the
  version a verified read has shown the network to run: before the first
  read the shell accepted they fail with a `rust::Error`, except on a
  devnet, whose floor is the latest version (regtest's is not: one read
  first). Once a read has shown a version this build does not know, they
  fail too, until the embedder is updated. The version only moves up, on
  each accepted read; the nonce read right before a build refreshes it.
- **Pure helpers**: `normalize_label`, `is_valid_username`,
  `is_contested_username`, `credits_per_duff`, `system_contract_id`, and the
  DIP-15 pieces that need only 32-byte inputs: `dip15_decrypt_xpub`,
  `dip15_account_reference_from_mac`,
  `dip15_unmask_account_reference_from_mac`, `dip15_select_recipient_key`,
  `dip15_receive_keys_acceptable`. The infallible ones answer `false`, `0`
  or empty on a (contained) panic, which reads as a refusal.

`Status.kind` is one of `Ok, ProvenAbsent, AlreadyExists, Consensus,
Unavailable, Rejected, ChainIdMismatch, UnsupportedProtocolVersion,
Internal`; a `Verified*` value is meaningful only under `Ok` and
`UnsupportedProtocolVersion`.

## Threading and signing

- Reads and the broadcast block the calling thread until the SDK request
  completes or `shutdown` aborts it; the embedder serializes them on its own
  worker. One tokio runtime (`src/runtime.rs`, two worker threads with
  16 MiB stacks for GroveDB proof replay) is owned per client. No single
  call issues more than one SDK request.
- Builders run to completion on the calling thread: dpp's async builders are
  driven by a local executor, no runtime is entered, and the `WalletSigner`
  is only ever invoked on the thread that called the builder (tested). The
  embedder's `WalletSigner` (`include/dash/platform/signer.h`) must
  nonetheless be callable from any thread (take the wallet's own lock, no
  thread-local state); that contract, stated in the header, is what its
  `Send + Sync` impls rest on.
- `SignForKey(key_id, signable)` receives the full signable preimage of the
  transition, so the wallet hashes it (double SHA256) itself and can check
  what it signs; the first byte is the `StateTransition` variant index
  (`test_data/state_transition_first_byte.json`: 2 = batch, 3 = identity
  create). It answers with a 65-byte compact recoverable ECDSA signature,
  exactly `dashcore::signer::sign` over the raw key.
  `SignAssetLockSighash(sighash)` is the one digest path: the asset lock's
  outpoint key signs the 32-byte double SHA256 the builder computed; the
  public key is recovered from the signature (compressed-key header,
  31 + recovery id; anything else is refused), so it is never exported.
  Private keys never cross the FFI; the shell never derives keys.

## Building and testing

An ordinary workspace member: `cargo build -p dash-platform-cxx --release`.
`build.rs` stages `dash/platform/ffi.h`, `dash/platform/signer.h` and
`rust/cxx.h` under `target/<profile>/include/`; install that tree and
`libdash_platform_cxx.a`, link with `-lpthread -lm` (`-ldl` on Linux,
`-framework Security -framework CoreFoundation` on macOS for the rustls
trust store). `scripts/cxx-smoke.sh` compiles and runs `tests/cxx_smoke.cc`
against exactly that interface.

`cargo test -p dash-platform-cxx` runs the unit tests, `tests/replay.rs`
(a real Drive state proved and BLS-signed by a test quorum, replayed through
`dash-sdk`'s mock transport so the SDK runs the full GroveDB replay and
signature check: the freshness matrix, absence, paging, broadcast
classification) and `tests/builders.rs` (every builder byte for byte against
dpp's in-process private-key constructions, the document ids and data Drive
validates, signer refusals, the no-reactor invariant, and
`test_data/state_transition_first_byte.json` against the transitions it
builds; `UPDATE_TEST_VECTORS=1` rewrites that file). Both suites run every
test at protocol version 13 (what testnet and mainnet run) and at this
build's latest, the fixture state generated at each.
