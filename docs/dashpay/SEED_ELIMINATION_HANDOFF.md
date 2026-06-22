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
3. **§4.6 persistence — emit DONE; storage round-trip REMAINING.** Both the
   enqueue (`ea6a1ea753`, add-delta) and the drain (`ecd288c735`, clear-delta)
   now emit `pending_contact_crypto_*` through the persister. **Remaining (storage
   crate, multi-layer):** a new refinery migration (table keyed by
   `(wallet_id, owner, contact, kind)`), writer (insert added / delete cleared,
   mirror `schema/accounts.rs::apply_registrations`), bulk reader, persister
   dispatch (`persister.rs` ~984/1057), and a **new** restore path into
   `PlatformWalletInfo.pending_contact_crypto` (NB: account registrations restore
   into the key_wallet `Wallet`, not `PlatformWalletInfo`, so this is a fresh
   load hook, not a mirror; `load.rs:97` currently inits empty). Round-trip test
   in the storage crate. The app's FFI/Swift persister is the env-blocked twin.
4. **§4.5 accountReference — HELD (speculative).** Its only Rust callers are the
   `dip14` tests; the production consumer is the env-blocked send-flow ECDH
   collapse, so adding the rs-sdk-ffi `ecdh_shared_secret_and_account_reference`
   / `unmask_account_reference` methods now would be dead code (Rule 2). Build it
   alongside that wiring. The move itself (DIP-15 `calculate/unmask_account_reference`
   → `platform-encryption`, re-exported from `dip14`; the `extract_ask28` test
   moves too) is the prerequisite.
5. ~~§4.5 contactInfo~~ **DONE** (`45f903dc38`). The drain's `ContactInfoDecrypt`
   op (calls `contact_info_open` via an extended `DrainCryptoProvider` +
   re-fetches owned docs — network-dependent) is the remaining drain piece.
6. ~~§4.8 wrong-seed self-check~~ **DONE** (`65f0c5cceb`): `verify_binds_to_xpub`
   + `WrongSeed`. The glue crate calls it at first use with the wallet's
   persisted account-xpub (env-blocked wiring).

## Remaining — environment-blocked (FFI + Swift + on-device)

Need Xcode + iOS simulator runtime / `aarch64-apple-ios` target (absent here).
Implement + regenerate the cbindgen header (`build_ios.sh`), then verify in Xcode.

- **ECDH FFI + Swift** — entry point wrapping `ecdh_shared_secret` (mirror
  `dash_sdk_sign_with_mnemonic_resolver_and_path`); Swift exposes it; the glue
  crate builds the `EcdhProvider::ClientSide { get_shared_secret }` closure.
- **`EcdhProvider::SdkSide → ClientSide` collapse** end-to-end in `sdk_writer`
  (send path) + the `register_external` decrypt closure (drain/accept). Delete
  `derive_encryption_private_key` + `SendContactRequestParams.ecdh_private_key`.
- **Convert sites 2/2b/4–7** through the FFI: `send_contact_request` /
  `accept_contact_request` FFI gain the wallet-HD resolver handle (alongside the
  doc signer); Swift passes the two handles it already holds.
- **§4.6 persistence over FFI** — `IdentityKeyEntryFFI`-style carry of the queue
  deltas through the FFI persister + the Swift persister callback (the SQLite
  path lands in Rust core #3; the FFI/Swift persister is here).
- **Drain FFI + Swift** — `platform_wallet_drain_pending_contact_crypto(wallet,
  core_signer)` wired into the Keychain-unlock path that previously called
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
