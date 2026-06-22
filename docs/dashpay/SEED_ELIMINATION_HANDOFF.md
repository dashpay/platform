# Seed-Elimination — Implementation Handoff / Task Tracker

Companion to `SIGNER_SEED_ELIMINATION_SPEC.md` (design + rationale). This file
is the **actionable remaining-work list**: the Rust cores still to implement
(verifiable in this repo) and the **environment-blocked FFI + Swift + on-device**
tasks that need Xcode + an iOS simulator runtime / device target (absent in the
headless dev env — same wall Phase 1 hit).

Keep this current as cores land.

## Status — landed (branch `feat/dashpay-m1-sync-correctness`)

| Commit | Spec | Summary |
|---|---|---|
| `d309a4b4` | Phase 1 | `send_payment` + `extended_public_key` via Keychain signer; dead read-APIs deleted; `WipingXprv` zeroize guard |
| `c79f95e9` | — | import-bug confirmed already-fixed (carry-scalar), de-blocked |
| `301fc436` | §4.7 | 3-state error classification (`Unavailable` ≠ `Permanent`); `has_seed()` readiness gate — no channel-kill/churn |
| `2e7eae1a` | §4.6 | deferred-crypto queue types (`PendingContactCrypto*`) + changeset add/clear deltas + `upsert_pending_contact_crypto` |
| `508b3edd` | §4.6 | in-memory enqueue on the seedless sweep (`PlatformWalletInfo.pending_contact_crypto`) |
| `d944245204` | §4.5 | `MnemonicResolverCoreSigner::ecdh_shared_secret` (parity-pinned); design = inherent methods + closures (no trait/crate) |
| `93fe4eac12` | §4.6 | `register_external_contact_account` takes `precomputed_shared_key: Option<[u8;32]>` (drain's decrypt core) + Some-path test |
| `6832a52c31` | §4.6 | `register_contact_account` takes `precomputed_account_xpub: Option<ExtendedPubKey>` (drain's RegisterReceiving core) + Some-path test |
| `4b6a6f7934` | §4.6 | deferred-crypto drain framework + `DrainCryptoProvider` trait + RegisterReceiving op + test |
| `ecd288c735` | §4.6 | drain RegisterExternal op (ECDH path + contact fetch + provider + register) + deferral-safety test |
| `45f903dc38` | §4.5 | contactInfo seal/open host primitive (2 hardened-child AES keys, reuses platform_encryption) + round-trip/parity test |
| `65f0c5cceb` | §4.8 | wrong-seed self-check `verify_binds_to_xpub` (+ `WrongSeed` error) + accept/reject test |
| `ea6a1ea753` | §4.6 | enqueue persists the `pending_contact_crypto_added` delta (symmetric with the drain's clear-delta) |
| `97a9a99f22` | §4.6 | drain FFI `platform_wallet_drain_pending_contact_crypto` + `ResolverDrainProvider` glue (iOS-cross-compiled, in the regenerated header) |
| `79ca6a1c2c` | §4.6 | SQLite storage for the queue: migration + writer + reader + store dispatch + round-trip test |
| `3bce45939b` | §4.5 | move `calculate/unmask_account_reference` (+ `extract_ask28` + 5 tests) into `platform-encryption`; `dip14` re-exports — single-sources the HMAC+masking for the Keychain signer |
| `7a23fa114e` | §4.5 | signer `account_reference` / `unmask_account_reference` methods on `MnemonicResolverCoreSigner` (Zeroizing scalar, parity+round-trip test) — the in-signer accountReference for seedless send |
| `9d532c2aba` | §4.6 | send via `EcdhProvider::ClientSide` — SDK no longer receives a private key; `SendContactRequestParams` carries the precomputed `shared_secret` + `expected_recipient_pubkey` guard (still resident-derived; seedless swap next) |
| `88b7fb7671` | §4.6 | generalize `DrainCryptoProvider` → `ContactCryptoProvider` + `account_reference`/`unmask_account_reference`; glue impl wires the signer methods (serves drain AND send) |
| `07e1821526` | §4.6 | **seedless send + accept** — xpub/ECDH/accountReference all via the provider; send/accept gain a `crypto` param; both FFI gain `core_signer_handle` + build the resolver provider. Verified host + `aarch64-apple-ios-sim` |

**Locked design decisions**
- Raw-secret ops are **inherent methods on `MnemonicResolverCoreSigner`** (in
  `rs-sdk-ffi`, which already binds the wallet signer); platform-wallet consumes
  them via **closures** (`EcdhProvider::ClientSide` + per-op closure params). No
  `WalletKeyProvider` trait, no new crate (rs-sdk-ffi and platform-wallet don't
  depend on each other; the closure seam already exists).
- Seedless-capable register fns take `Option<precomputed>` — `None` = resident
  path (unchanged), `Some` = drain (signer-derived, scalar never in this crate).
- Queue = persisted add/clear **deltas** on the changeset; in-memory mirror on
  `PlatformWalletInfo`; dedup by `(owner, contact, kind)`, latest payload wins.
- `register_external` resident path stays gated by §4.7 `has_seed()` →
  `Unavailable` defers (enqueue), never kill/churn.

## Remaining — Rust cores (verifiable here; do in this order)

1. ~~`register_contact_account` precomputed-xpub~~ **DONE** (`6832a52c31`). Both
   drain ops now have their seedless core (RegisterExternal=`93fe4eac12`,
   RegisterReceiving=`6832a52c31`).
2. ~~The drain method~~ **DONE** (`4b6a6f7934` framework + RegisterReceiving;
   `ecd288c735` RegisterExternal). `drain_pending_contact_crypto(provider)` on
   `IdentityWallet`, generic over the platform-wallet-local `DrainCryptoProvider`
   trait (`receiving_xpub` + `ecdh_shared_secret`; the glue crate impls it over
   the resolver signer). Snapshots the queue, runs each op, removes completed
   entries (in-memory + persisted clear delta), marks-broken on permanent
   faults, leaves transient/unavailable for next drain. **Remaining:** the
   `ContactInfoDecrypt` op (needs the §4.5 contactInfo primitive). End-to-end
   RegisterExternal success path (real fetch + provider) verifies on-device.
3. **§4.6 persistence — emit + SQLite storage DONE; restore BLOCKED upstream.**
   Enqueue (`ea6a1ea753`) and drain (`ecd288c735`) emit the deltas; the SQLite
   persister now writes + reads them (`79ca6a1c2c`: migration + writer + reader +
   dispatch + round-trip test). **Remaining:** the read-back into
   `PlatformWalletInfo.pending_contact_crypto` is blocked on the upstream
   per-wallet state restore (`persister.rs` `LOAD_UNIMPLEMENTED:
   ClientStartState::wallets`); the reader is `cfg(test)`-gated until then.
   The app's **FFI/Swift persister twin** (carry the deltas through the FFI
   persister + Swift callback) is env-blocked.
4. ~~**§4.5 accountReference**~~ **DONE.** The move landed (`3bce45939b`) and the
   signer methods landed (`7a23fa114e`): `MnemonicResolverCoreSigner::account_reference`
   / `unmask_account_reference` derive the scalar in-signer (Zeroizing) and call
   `platform-encryption`, so the raw scalar never returns to platform-wallet. The
   "speculative" hold was wrong — the consumer is the send-flow FFI (Rust, builds
   here), not Swift. Wired into `ContactCryptoProvider` (`88b7fb7671`); the send
   path consumes them in C2 below.
5. ~~§4.5 contactInfo~~ **DONE** (`45f903dc38`). The drain's `ContactInfoDecrypt`
   op (calls `contact_info_open` via an extended `DrainCryptoProvider` +
   re-fetches owned docs — network-dependent) is the remaining drain piece.
6. ~~§4.8 wrong-seed self-check~~ **DONE** (`65f0c5cceb`): `verify_binds_to_xpub`
   + `WrongSeed`. The glue crate calls it at first use with the wallet's
   persisted account-xpub (env-blocked wiring).

## Remaining — Rust + FFI (verifiable here; do in this order)

These compile + cross-compile in this repo (the glue crate is Rust; the
ClientSide seam + provider already landed). Earlier handoffs wrongly filed these under
"environment-blocked" by conflating FFI (Rust) with Swift (`.swift`).

**C2 — seedless send + accept — DONE** (`07e1821526`). Send + accept source the
friendship xpub, ECDH secret, and accountReference through `ContactCryptoProvider`;
both FFI take `core_signer_handle`. (The new C ABI param breaks the `.swift`
callers — expected; that's the Swift task below.) Verified host + iOS-sim.

**C3 — delete the resident ECDH path. BLOCKED on the sweep conversion.**
`derive_encryption_private_key` is NOT yet dead: the background sync sweep
(`build_contact_accounts`, the `register_external_contact_account(..., None)`
caller) still uses the resident-derive path during migration — it has no signer
in the background, so it either derives now (resident) or enqueues (seedless,
the §4.7 `BuildReadiness` gate). To eliminate the resident path entirely:
1. **Convert the sweep to always-enqueue** (`build_contact_accounts`): drop the
   `Ready`→derive-now branch so the sweep ALWAYS enqueues the `RegisterExternal`
   op; the drain (on unlock, with a signer) does the crypto. This is the
   behaviour the acceptance test wants ("background-discover an inbound contact
   then unlock → it becomes payable"). Careful change — re-read the
   `BuildReadiness` gate first; do NOT rush.
2. Then `register_external_contact_account`'s `precomputed_shared_key: Option`
   has only `Some` callers (drain + accept) → make it non-`Option`
   `shared_key: [u8; 32]`, delete the `None` resident branch (steps 2–4), drop
   `Unavailable` from its local `use`, renumber its step comments.
3. Then `derive_encryption_private_key` (`identity_handle.rs`) has only test
   callers → delete it + the `ecdh_key_derivation_tests` module (its
   purpose-agnostic/index-driven property is now structural in
   `identity_auth_derivation_path`, which takes `key_id` not purpose).
Confirm no `derive_contact_xpub(wallet)` / `derive_extended_private_key` survive
on DashPay send/accept paths.

**§4.6 persistence over FFI** — `IdentityKeyEntryFFI`-style carry of the queue
deltas through the FFI persister + the Swift persister callback (the SQLite
path landed in `79ca6a1c2c`; the FFI persister twin is Rust-doable, the Swift
callback is below).

### Ordering: why C3 / §4.9 / sweep-conversion can't land before the Swift wiring

Grounded in the actual sweep code (`build_contact_accounts`):
- The `BuildReadiness::Ready` (resident `has_seed`) branch is a **deliberate
  migration fast-path**: a wallet that still holds a resident seed registers
  contacts immediately during the background sweep; a seedless wallet enqueues
  for the drain. Deleting this branch makes the sweep *always* defer to the
  drain — which needs a Keychain signer. A migration wallet that has a seed but
  no wired signer-drain would then **never** register contacts → regression.
- The platform-wallet test helpers (`make_wallet` etc. in `payments.rs`) call
  `attach_wallet_seed` to set up resident-seed wallets. Deleting `attach_wallet_seed`
  breaks every test that exercises a resident path; they'd first need a **real
  seedless test `ContactCryptoProvider`** (derive from a test seed via `key_wallet`,
  which platform-wallet already depends on — the existing `CannedProvider`/
  `UnusedProvider` only return stubs).
- The spec's own ordering: delete `attach_wallet_seed` "only after the sweep is
  seedless-safe AND the drain replaces the Keychain-unlock re-attach" — and that
  re-attach→drain swap is in `.swift` (env-blocked).

So the correct sequence is: **Swift signer wiring (drain-on-unlock) FIRST**, then
the wholesale end-state change (seedless test harness → sweep always-enqueue →
C3 delete resident `register_external` branch + `derive_encryption_private_key`
→ §4.9 delete `attach_wallet_seed`). Doing the Rust deletions before the Swift
wiring leaves a Rust-green-but-app-broken intermediate and risks regressing the
recurring sync (this branch's whole purpose).

## Remaining — environment-blocked (Swift + on-device)

Need Xcode + iOS simulator runtime (absent here). After the Rust/FFI lands,
regenerate the cbindgen header (`build_ios.sh` — the header lives inside the
xcframework build artifact, so it can't be hand-regenerated meaningfully without
the cross-compile + packaging) and update the `.swift` callers. The two contact
FFI now take an extra `core_signer_handle` the Swift side already holds (the same
handle it passes to the drain); Swift passes it and, on Keychain unlock, calls
`platform_wallet_drain_pending_contact_crypto` instead of the deleted re-attach.
- **Drain FFI — Rust side DONE** (`97a9a99f22`, iOS-cross-compiled + in header).
  **Remaining (Swift):** call `platform_wallet_drain_pending_contact_crypto(wallet,
  core_signer)` from the Keychain-unlock path that previously called
  `unlockWalletFromKeychain` (replacing the deleted re-attach), + opportunistic
  drain on signer-present actions + a UI "needs unlock" marker.
- **contactInfo FFI + Swift** — seal/open entry points; the sync path enqueues
  `ContactInfoDecrypt` instead of the silent `SkippedWatchOnly`.
- **Delete the dead `dash_sdk_dashpay_*` ClientSide FFI surface** + regen header.
- **§4.9** — delete `attach_wallet_seed` + the FFI export +
  `unlockWalletFromKeychain` re-attach + the legacy `KeychainSigner.sign(...)->Data?`
  nil-swallow; rework test helpers to inject a test signer. **Only after** the
  sweep is seedless-safe (queue + drain landed).
- **On-device acceptance** — clean wipe → import → send/accept contact request,
  send payment, publish profile + contactInfo; **background-discover an inbound
  contact then unlock → it becomes payable**; `git grep attach_wallet_seed`
  empty; no surviving `derive_extended_private_key` / `build_signed(wallet)` on
  DashPay paths.
