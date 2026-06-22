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

1. **`register_contact_account` precomputed-xpub** — mirror commit `93fe4eac12`.
   Add `precomputed_account_xpub: Option<ExtendedPubKey>`: `None` = derive at
   `contacts.rs:186` (resident); `Some` = use it (drain). Ripple: ~7 callers
   pass `None` (`contact_requests.rs:338/965/1287`, `payments.rs` tests). This
   is the drain's *RegisterReceiving* core. Test: Some-path builds the receiving
   account from a supplied xpub.
2. **The drain method** (`drain_pending_contact_crypto`) on `IdentityWallet`.
   Generic over two provider closures supplied by the glue crate:
   `xpub_at(path) -> ExtendedPubKey` (→ `register_contact_account(Some(..))`) and
   `ecdh(path, peer) -> [u8;32]` (→ `register_external(.., Some(..))`); contactInfo
   later. For each persisted `PendingContactCrypto`: run the op; on `Ok` →
   push the key to `pending_contact_crypto_cleared` + remove from the in-memory
   mirror; on `Permanent` → mark broken + clear; on `Unavailable`/`Transient` →
   leave for next drain. Persist the clears via one changeset `store`. Tests:
   queue with a RegisterExternal entry + a closure returning a known secret →
   account built + entry cleared; a Permanent error clears + marks broken; an
   Unavailable error leaves the entry.
3. **§4.6 persistence round-trip (storage crate)** — SQLite table +
   writer/reader for `pending_contact_crypto_added/_cleared` (mirror
   `schema/accounts.rs`), persister dispatch (`persister.rs` ~984/1057), and
   restore into `PlatformWalletInfo.pending_contact_crypto` via the start-state
   path (`load.rs:97` currently inits empty). Tests: storage round-trip
   (persist add/clear → load → queue restored).
4. **§4.5 accountReference** — move DIP-15 `calculate_account_reference` /
   `unmask_account_reference` from `dip14.rs` to `platform-encryption` (the
   DIP-15 crypto crate, reachable from rs-sdk-ffi; update platform-wallet
   callers), then add `ecdh_shared_secret_and_account_reference` /
   `unmask_account_reference` inherent methods on `MnemonicResolverCoreSigner`.
   Parity tests vs the resident path.
5. **§4.5 contactInfo** — `contact_info_seal` / `contact_info_open` inherent
   methods (2 hardened-child keys via key_wallet + AES via platform-encryption;
   `root_path` passed in). Parity tests; the DIP-15 wire codec stays in
   `crypto/contact_info.rs` (plaintext-only).
6. **§4.8 wrong-seed self-check** — `MnemonicResolverCoreSigner` derives BIP44
   account-0 xpub; the glue crate compares to the wallet's persisted account-0
   xpub at first use; mismatch fails loud. (Replaces the dual gate removed with
   `attach_wallet_seed`.)

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
