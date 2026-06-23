# Seed-Elimination — Implementation Handoff / Task Tracker

Companion to `SIGNER_SEED_ELIMINATION_SPEC.md` (design + rationale). This file
is the **actionable remaining-work list**: the Rust cores still to implement
(verifiable in this repo) and the **environment-blocked FFI + Swift + on-device**
tasks that need Xcode + an iOS simulator runtime / device target (absent in the
headless dev env — same wall Phase 1 hit).

Keep this current as cores land.

## SCOPE CORRECTION (2026-06-23, after re-reading the spec §2 inventory)

Earlier entries below over-scoped discovery as a "deep storage/signing rewrite
that drops verified_scalar." **That was wrong.** The spec (§1, §2, §7) is
explicit: the **carry-scalar (`verified_scalar`) is the ACCEPTED, KEPT fix** for
the imported-identity zero-keys bug (RESOLVED, on-device 23/23). Seed elimination
removes the **resident** seed (`attach_wallet_seed`'s `mem::swap` graft), NOT the
stored per-key scalars. Per the spec's exhaustive §2 inventory, sites #1–#6 are
**done** (the contact-request flow); the only remaining seed-dependent path is
**#7 contactInfo**. Discovery is NOT a §4.9 blocker as a "rewrite" — it just has a
`ResidentWallet` derivation fallback to eliminate (route through `Master(transient
xpriv)`; carry-scalar stays).

**Corrected remaining work for §4.9 (delete `attach_wallet_seed`):**
1. **#7 contactInfo** — publish (write) via a `ContactCryptoProvider`
   `contact_info_seal`/`open` seam (signer primitives exist + parity-tested,
   `45f903dc38`); sync (read) via the `ContactInfoDecrypt` deferred op (testnet).
2. **Discovery/loading `ResidentWallet` fallback** — `discover()` (discovery.rs:189)
   + `load_identity_by_index()` (loading.rs:123) pass `ResidentWallet`; route the
   transient master xpriv through (the `Master` variants exist:
   `discover_from_master` 216) so import/unlock derive from the transient mnemonic,
   then remove the `ResidentWallet` variants. Carry-scalar unchanged.
3. **Swift** — drain-on-unlock replacing `unlockWalletFromKeychain`'s re-attach +
   test-helper rework.
4. **Delete** `attach_wallet_seed` + FFI export + impl + dual-gate/`mem::swap` +
   the `KeychainSigner.sign(...)->Data?` nil-swallow.

Environment (verified 2026-06-23): Xcode 26.5 + iOS-sim SDK present (BUILDS work);
NO sim runtime installed (`simctl` 0 disk images — user installing via Xcode);
testnet acceptance feasible (user has a funded testnet seed). Plan: implement +
build-verify (cargo + build_ios.sh + SwiftExampleApp) here; run testnet acceptance
once the runtime lands.

## SUPERSEDED — earlier HOLD decision (2026-06-23, now reversed by the scope correction above)

The seedless **contact-request flow** (send/accept/drain/always-enqueue sweep)
and the resident-ECDH-path deletion (C3) are **DONE + verified** (292/292).
The user **decided to HOLD** the remaining seed-elimination — deleting
`attach_wallet_seed` (§4.9), the discovery key-storage/signing rewrite, and the
Swift Keychain-signer wiring — for an **iOS-capable session**, because their
correctness is only observable against the iOS Keychain signer (env-blocked) and
discovery is the most safety-critical path (a wrong key-storage change locks
users out of signing). This is a deliberate deferral, not an oversight:
`attach_wallet_seed` is intentionally retained while its remaining production
callers (discovery, contactInfo sweep decrypt) still need it. Do NOT delete it
headless. Resume the held work in an environment with Xcode + a live network.

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
| `5f1f75a9f9` | test | seedless `SeedCryptoProvider` test harness (derives from a test seed via `key_wallet`) — the deletion-rework prerequisite |
| `9082d35aad` | §4.7 | drain `RegisterExternal` runs `validate_contact_request` (closes the deferred-path validation gap; makes always-enqueue validation-safe) |
| `1b88d5a6ca` | §4.6 | **sweep always-enqueues** — removed `build_contact_accounts`' resident fast-path; the signerless sweep defers everything to the drain (−205 lines) |
| `14566d96bd` | §4.9 | **C3** — delete the resident ECDH path: `register_external` non-`Option` (dead key-index params removed), `derive_encryption_private_key` + tests gone, `RegisterExternalError::Unavailable` removed |

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

**Sweep always-enqueue + C3 — DONE** (`1b88d5a6ca` sweep, `9082d35aad` drain
validation, `5f1f75a9f9` seedless test harness, `14566d96bd` C3). The signerless
sweep now defers everything to the drain; the resident `register_external` branch,
`derive_encryption_private_key`, the 3 resident classification tests, and the
`RegisterExternalError::Unavailable` machinery are all deleted. No DashPay
**send/accept/sweep** path derives from a resident seed.

**§4.6 persistence over FFI** — `IdentityKeyEntryFFI`-style carry of the queue
deltas through the FFI persister + the Swift persister callback (the SQLite
path landed in `79ca6a1c2c`; the FFI persister twin is Rust-doable, the Swift
callback is below).

### §4.9 (delete `attach_wallet_seed`) is blocked on 3 remaining raw-secret paths

Grounded in `git grep "wallet.derive_extended_private_key"` (production, non-test):
deleting `attach_wallet_seed` makes `has_seed()` always false, which would break
these paths that still derive from the **resident seed** and have no provider seam
yet. Each needs the same treatment send/accept got (route through a signer method
via a provider closure/seam) BEFORE `attach_wallet_seed` can go:

1. **contactInfo** (`crypto/contact_info.rs` `derive_contact_info_keys`) — the
   resident twin of the signer's `contact_info_seal`/`contact_info_open` (which
   exist + are parity-tested, `45f903dc38`). **Not cleanly separable:** the
   derivation is reached through a shared helper `fetch_decrypted_contact_infos`
   used by BOTH the signer-present publish (`set_contact_info_with_external_signer`)
   AND the **signerless sweep** (`sync_contact_infos`). The sweep side can't
   decrypt without a signer → it needs the `ContactInfoDecrypt` deferred op, which
   re-fetches the owner's docs (**network-blocked here**). So contactInfo can't go
   fully seedless until the sweep decrypt is deferred over a live network. (The
   publish-only encrypt could be threaded through a provider, but it shares the
   helper, so a clean conversion does both at once.)
2. **auto-accept proof** (`crypto/auto_accept.rs`) — **production-dead:**
   `generate_auto_accept_proof` / `derive_auto_accept_private_key` have no
   production caller (the send flow always passes `auto_accept_proof: None`); only
   their own `#[cfg(test)]` tests exercise the seed. So this is NOT a production
   seed dependency — when `attach_wallet_seed` goes, only these tests need rework
   (drop them or derive via a test seed directly).
3. **`derive_identity_auth_keypair`** (`network/identity_handle.rs`) — returns the
   raw `ExtendedPrivKey`; used by the identity-discovery scan (`discovery.rs`) +
   the FFI key preview + registration. This is the **documented deep blocker**
   (memory `dashpay-imported-identity-zero-candidate-discovery`). Root cause,
   confirmed in `discovery.rs::breadcrumb_decisions`: discovery doesn't just
   *match* keys — it carries the **verified private scalar** (`KeyWithBreadcrumb.
   verified_scalar`) to the client so it "stores the bytes directly instead of
   re-deriving from a mnemonic." **That stored scalar IS the resident-seed
   posture.** Seedless discovery requires an architectural change:
   - verify ownership via **public-key** derivation (signer `extended_public_key`
     at the hardened candidate path, compare to the on-chain compressed pubkey) —
     no scalar needed for the match;
   - stop carrying/storing `verified_scalar` (remove the field; changes the
     changeset + the `identity_keys` persistence + the FFI/Swift that stores it);
   - route signing through the Keychain signer using only the breadcrumb
     `(wallet_id, identity_index, key_id)` — the env-blocked Swift signing path.
   This spans storage + FFI + Swift, not a localized Rust conversion. Resolve the
   storage/signing model (and wire the Swift Keychain signing) before deleting the
   resident derive.

   **Decisive fact (traced):** `verified_scalar` → `IdentityKeyEntry.private_key`
   is a **Rust→FFI→Swift hand-off** — NO Rust signing path reads it (Rust signs
   via the external `VTableSigner`; `git grep` finds no Rust consumer of the stored
   scalar for signing). Its terminal consumer is the Swift Keychain persister /
   signer (14 `rs-platform-wallet-ffi` files handle identity-key private material).
   So the Rust-side change (verify-via-pubkey, drop the scalar) cannot be
   **validated** here — only the Swift signer that consumes the breadcrumb proves
   it works, and that's env-blocked. Combined with this being the single most
   safety-critical path (a wrong key-storage change = users locked out of signing),
   this is the one conversion that must NOT be rushed headless: do it in an
   iOS-capable session where the Swift signing can be exercised end-to-end.

**Net:** `attach_wallet_seed` still has live production callers — the contactInfo
sweep decrypt (network-blocked) and discovery (deep storage/signing change +
env-blocked Swift). Deleting it now regresses background sync + identity
discovery, which the spec's own §4.9 ordering forbids ("only after the sweep is
seedless-safe"). The contact-request flow is fully seedless; the wallet-wide
posture needs the above before the resident-seed API can go.

Only after those are seedless does `attach_wallet_seed` have no production caller.
Then §4.9 deletes: `manager/attach_seed.rs` (+ its tests), the
`platform_wallet_manager_attach_wallet_seed_from_mnemonic` FFI (+ 4 tests in
`manager.rs`), and the `make_wallet` test helpers' attach calls (→ seedless,
using `SeedCryptoProvider`). The `.swift` `unlockWalletFromKeychain` re-attach →
`drain` swap is env-blocked.

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
