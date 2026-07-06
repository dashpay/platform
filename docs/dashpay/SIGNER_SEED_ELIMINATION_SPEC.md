# DashPay Signer-Based Seed Elimination — Spec

Status: DRAFT v3 (revised after a 3-agent deep design review: seedless
background-sync architecture, signer/host-primitive model, security &
failure-mode audit)
Branch: `feat/dashpay-m1-sync-correctness` (PR #3841)
Cross-repo: required key-wallet method (`extended_public_key`) has LANDED;
pinned `rust-dashcore` rev already bumped.

## 1. Problem

PR #3639 ("external signable wallets", v3.1-dev) set this codebase's
posture: a registered/restored wallet holds **no resident seed**
(`WalletType::ExternalSignable`). Private-key work is done by passing a
**Keychain-backed `Signer`** per operation; the seed lives only in the iOS
Keychain.

The DashPay paths added in PR #3841 did not follow that model — they reach
for the resident seed (`send_payment` passes the `Wallet` to `build_signed`;
`derive_contact_xpub` calls `wallet.derive_extended_public_key`; contact
encrypt/decrypt + `accountReference` + contactInfo derive raw secrets off
the `Wallet`). To make them work, `manager/attach_seed.rs::attach_wallet_seed`
re-derives a signing `Wallet` from the Keychain seed and grafts it onto the
loaded wallet via `std::mem::swap`. **That defeats the external-signable
posture** (the seed becomes resident for the whole session) and is a
workaround.

The resolved **import-wallet bug** was the same disease for identity keys
(Swift re-derived identity scalars from the mnemonic during discovery);
fixed by the carry-scalar change (later reworked into the derive-sign-destroy resolver — see `IDENTITY_KEY_SCALAR_ELIMINATION_SPEC.md`).

### Goal

Every DashPay private-key operation runs through a Keychain-backed host
primitive; the wallet seed is **never made persistently resident**;
`attach_wallet_seed` (+ `unlockWalletFromKeychain`'s re-attach + the FFI
export + the dual-gate/`mem::swap`) is **deleted** — but only after the
background sync sweep is seedless-safe (§4.9 ordering constraint).

### Honest scope of the security win (corrected after the security audit)

This does **not** make the seed "never resident." Verified facts, to be
stated plainly so the win is not over-credited:

- **The full BIP-39 64-byte seed + master xprv are reconstructed in one
  contiguous buffer per operation** (`MnemonicResolverCoreSigner::resolve_derived_xprv`
  — `seed: Zeroizing<[u8;64]>`, master xprv on the next lines; sibling
  `sign_with_mnemonic_resolver.rs`). The whole wallet is derivable from that
  buffer for the duration of the op.
- **The read is unlock-gated, not biometric-gated.** The Keychain mnemonic is
  `kSecAttrAccessibleWhenUnlockedThisDeviceOnly` with **no `LAContext` /
  `SecAccessControl`** on the read path (`WalletStorage.swift`). The
  `.biometryCurrentSet` stash exists but is **unused**. So "user present"
  really means "device unlocked" — any in-process code can drive the resolver
  while unlocked.
- **The wipe is best-effort.** Byte buffers use `Zeroizing` (volatile + fence,
  runs on unwind). The two `ExtendedPrivKey` scalars use secp256k1's
  `non_secure_erase` (fills `[1u8;32]`, self-disclaimed as non-secure;
  `ExtendedPrivKey` has no `Drop`/`Zeroize` upstream). There is an **error-
  /unwind-path residue gap** (§4.2 hardening).

The real, defensible benefit is a **smaller in-memory time-window** for the
root secret — per-operation-and-wiped vs `attach_wallet_seed`'s session-long
resident `Wallet` — plus consistency with the #3639 posture. It is a modest,
honest improvement, not "the seed is never in RAM." (The dashj /
dash-shared-core reference clients hold the decrypted seed for the whole
session — see §8 — so there is no off-the-shelf signer-based DashPay to copy.)

### Non-goals

- Changing `downgrade_to_external_signable` (`wallet_lifecycle.rs:251`) — it
  is what makes the seed absent; it stays.
- Re-introducing an in-memory-seed wallet (the dashj model).
- Wiring QR-based auto-accept — tracked in the backlog (dashpay/platform#4020) (helpers KEPT, not
  deleted; §2 note).

### Q2 scope, status & completion criteria (2026-06-23)

**Q2** (the PR reviewer ask) = *remove the assign-seed workaround* = delete
`attach_wallet_seed` (this §; §4.9). Live status: the backlog issue dashpay/platform#4020 (authoritative tracker).

**Done + verified** (branch `feat/dashpay-m1-sync-correctness`; platform-wallet
292/292; glue builds host + `aarch64-apple-ios-sim`): §2 inventory sites **#1–#6**
— the entire seedless contact-request flow (send/accept/drain/always-enqueue
sweep), the `ContactCryptoProvider` seam + signer host primitives (ECDH,
accountReference, contactInfo seal/open, wrong-seed check), the deferred-crypto
queue + SQLite persistence, and C3 (resident ECDH path deleted). Discovery is
NOT a rewrite target — the **carry-scalar fix is kept** (§1).

**Remaining for Q2 (do in this order). Folds in the 4-lens plan review
(feasibility / security / scope-dedup / adversarial), which corrected two
over-optimistic items in an earlier draft of this banner — see the MUST-FIX
notes.**

1. **#7 contactInfo (the load-bearing item).**
   - Add `contact_info_seal` AND **`contact_info_open`** to `ContactCryptoProvider`
     (+ glue impl over the signer primitives + `SeedCryptoProvider` test impl).
     **MUST-FIX (review):** publish is NOT seal-only — it must DECRYPT existing
     owned docs to decide update-vs-create (the doc↔contact binding lives inside
     `encToUserId` ciphertext; `contact_info.rs:435-447`), so it needs `open`.
   - Refactor the shared helper `fetch_decrypted_contact_infos` (resident-hardcoded
     at `:206`/`:450`, used by BOTH publish and the signerless sweep) into a public
     high-water scan (no keys) + a provider-`open` decrypt step. Thread `crypto`
     into `set_contact_info_with_external_signer` (publish) and the FFI
     (`platform_wallet_set_dashpay_contact_info_with_signer` gains a
     `core_signer_handle` — ABI break, like send/accept).
   - **MUST-FIX (security): root-path provenance.** Build the contactInfo root
     path in **Rust** via `identity_auth_derivation_path_for_type(net, ECDSA,
     identity_index, root_key_id)` (never Swift); pin the parity test against that
     REAL path, not `test_path()` — a wrong root silently writes undecryptable
     contactInfo with no on-chain oracle.
   - **MUST-FIX (security): high-water.** Publish is signer-present, so derive the
     high-water `derivationIndex` from a FRESH full decrypt, or refuse to publish —
     NEVER fall back to `0`/stale (collides the unique `($ownerId, rootIndex,
     derivationIndex)` index / reuses a key).
   - Implement the `ContactInfoDecrypt` **drain op** (currently a no-op stub,
     `contact_requests.rs:1440`): re-fetch owned docs + `contact_info_open` via the
     provider + re-run `validate_contact_request`-equivalent. Re-fetch = **testnet
     validated**. `derive_contact_info_keys` (resident twin) is deleted ONLY after
     both publish AND this sweep/drain path no longer call it.
   - **MUST-FIX (security): confused-deputy.** The drain (and opportunistic drains)
     must re-validate the queue entry's `owner_identity_id` is owned by THIS wallet
     before decrypting/registering.

2. **Discovery/loading — NO library change (corrected; was wrong in the earlier
   draft).** The FFI already routes external-signable wallets through
   `discover_from_master` / `load_identity_by_index_from_master` (via
   `resolve_master_from_resolver`), and the `ResidentWallet` variants are the
   **live path for genuine resident-key wallet TYPES** (`WalletType::Mnemonic`/
   `Seed`, e.g. raw-seed imports with no persisted mnemonic) — NOT dead duplicates.
   So **keep** them; do **NOT** attempt the deep `verified_scalar`-drop rewrite
   (that's the env-blocked architectural change, and unnecessary — carry-scalar is
   kept). The only Q2 work here: (a) confirm every discovery/loading caller passes
   a non-null resolver after the deletion makes external-signable the universal
   posture (the FFI already errors on null — `identity_discovery.rs:194`); (b) the
   test-helper rework in step 3. NOTE: the §2 "exhaustive" table is exhaustive over
   *DashPay-contact* paths; the discovery resident derive `derive_identity_auth_keypair`
   (`identity_handle.rs:191`) is a resident-key-TYPE path that legitimately stays.

3. **Test-helper rework + Swift.** ✓ DONE. The test helpers were made seedless
   earlier (`c42d2e413e`). Swift (`70aaf32f9f`): `unlockWalletFromKeychain(_:)`
   now verifies-binds + drains-on-unlock (no seed graft); send/accept/contactInfo
   thread the resolver `core_signer_handle`. Verified by regenerating the
   xcframework header (`build_ios.sh --target sim`) + an arm64-sim SwiftExampleApp
   build (BUILD SUCCEEDED).

4. **Delete** `attach_wallet_seed` + FFI export + dual-gate/`mem::swap` + the dead
   `dash_sdk_dashpay_*` rs-sdk-ffi surface (4 fns, zero Swift callers — confirmed)
   + the legacy `KeychainSigner.sign(...)->Data?` nil-swallow.
   - **MUST-FIX (security): atomic wrong-seed wiring.** ✓ DONE (Rust, `fe3ab74e19`):
     `attach_wallet_seed` (lib + FFI + dual gate) deleted and `verify_seed_binds`
     (`PlatformWallet` method + `platform_wallet_verify_seed_binds_to_wallet` FFI)
     landed in the same commit — signer-derived BIP44-0 xpub vs the persisted one,
     mismatch → `SeedMismatch`. The comparison lives in `verify_seed_binds`, which
     derives the xpub through the `ContactCryptoProvider::receiving_xpub` seam (a
     generic derive-at-path) — no redundant trait method. (The signer's
     `verify_binds_to_xpub` primitive was NOT used and was deleted post-review as
     dead code; the live path reimplements the equality in `verify_seed_binds`.)
     The dead `dash_sdk_dashpay_*` surface was removed earlier (`db18688545`).
     ✓ Swift wiring DONE (`70aaf32f9f`): the verify FFI is called at unlock and the
     `KeychainSigner.sign(...)->Data?` nil-swallow is deleted (see step 3 Swift).
   - **SHOULD-FIX (security): §4.2 sibling-FFI leak.** ✓ DONE (`feb266fd1b`): the
     `WipingXprv` RAII guard now wraps `master`/`derived` in
     `rs-platform-wallet-ffi/src/sign_with_mnemonic_resolver.rs`, scrubbing on the
     error/unwind paths the Ok-only `non_secure_erase` missed.

**Done when:**
- ✓ `git grep attach_wallet_seed` empty (outside docs/regenerated headers).
- ✓ `cargo test -p platform-wallet` green (292) + glue (`platform-wallet-ffi`) green.
- ✓ `build_ios.sh --target sim` regenerates the header (verify FFI present, attach
  gone, send/accept/contactInfo carry `core_signer_handle`) + SwiftExampleApp
  builds clean (arm64 sim, BUILD SUCCEEDED).
- **On-device acceptance — VALIDATED on devnet `paloma` (2026-06-23)** via idb UI
  automation across **two simulators** (sim A = funded `Test_devnet`, 9 DASH, 3
  identities; sim B = freshly created `SimB` wallet + new identity "Eve"). Every
  signing op below ran through the Keychain resolver with **no resident seed**:
  - ✓ App builds, installs, launches, runs full (SDK init, network switch, wallet
    list/detail, DashPay sync) on the new seedless binary — no crash.
  - ✓ **Seedless Core payment + IS-lock** — sim A sent 1 DASH to sim B's address;
    tx signed via the resolver, broadcast, **InstantLock validated** (this is the
    real wrong-seed-would-fail path: the seedless wallet signed a real Core tx).
  - ✓ **Seedless identity registration** — sim B: asset lock → InstantSend proof →
    ChainLock proof → Platform registration, all via the resolver ("Identity
    created" `BfWHEg…`).
  - ✓ **Seedless DPNS name registration** (`eveqaseed1ess2026`) + **profile publish**
    ("Eve") — both Platform writes via the resolver.
  - ✓ **Seedless contactInfo publish** — `setDashPayContactInfo`
    (`core_signer_handle`) published an encrypted contactInfo doc, **create**
    (`updated=false`) **and update** (`updated=true`).
  - ✓ **Seedless contact request SEND** (Alice→Eve) — `send_contact_request_with_signer`
    (`core_signer_handle`): registered the DashpayReceivingFunds account.
  - ✓ **Seedless contact request ACCEPT** (Eve→Alice) —
    `accept_contact_request_with_signer` (`core_signer_handle`): reciprocal request
    + registered DashpayReceivingFunds **and** DashpayExternalAccount (the ECDH +
    accountReference path). Contact established cross-device (sim A shows Eve as
    contact #3).
  - ⏳ NOT exercised on-chain (covered elsewhere): (a) **wrong-seed rejected loud**
    via `verify_seed_binds` at unlock — gated behind the `loadFromPersistor`
    restorable path; cleanly forced only by a destructive wipe+reimport. Covered by
    the high-fidelity unit test (real `key_wallet` wallet, same BIP44-0 path,
    accept/reject) — and the live Core/Platform signing above proves the resolver
    derives correct keys. (b) **cross-device contactInfo decrypt** (same owner on a
    2nd device) — publish validated; decrypt-drain unit-tested.

  **Unrelated finding (not a seed defect):** the DashPay contact-profile chunk fetch
  fails with a DAPI error `missing order by for range error: query must have an
  orderBy field for each range element` ("Failed to fetch a contact-profile chunk;
  will retry next sweep"). Contact establishment still succeeds; contact *profiles*
  may not render. Track + fix separately.

**Out of Q2 scope** (tracked in the backlog (dashpay/platform#4020)): §6b queue restore (upstream
`ClientStartState::wallets` — note the security review's caveat that until restore
works, a contact discovered-then-app-killed-before-unlock never finishes setup);
the §4.8 present-but-zero-keys import caveat; QR auto-accept wiring.

## 2. Inventory of seed-dependent paths (revised; exhaustive over **DashPay-contact** paths)

Verified by grepping every `derive_extended_p*_key` / `build_signed(wallet` /
`.has_seed()` reader, and by tracing reachability from the background sweep.
NOTE (plan review): this table enumerates the DashPay-contact seed paths (#1–#7).
The **identity-discovery** resident derive `derive_identity_auth_keypair`
(`identity_handle.rs:191`) is a separate resident-key-TYPE path — it legitimately
stays for `WalletType::Mnemonic`/`Seed` wallets and is NOT a Q2 deletion target
(external-signable wallets already route discovery through the resolver). See the
Q2 banner step 2.

`bg?` = reachable from the **signerless** recurring sweep
(`DashPaySyncManager` → `dashpay_sync` → `build_contact_accounts` /
`sync_contact_infos`). The sweep FFI takes **no signer handle** and runs with
no user present — so any `bg?`+secret op is a deferral problem (§4.6).

| # | Call site | Capability | bg? | Phase |
|---|---|---|---|---|
| 1 | `send_payment` (`payments.rs`, build site) | ECDSA sign at path | no | **1 — DONE** |
| 2 | `derive_contact_xpub` (`dip14.rs:98`), caller = send-contact-request flow (signer present) | xpub at hardened path | no | **2** (bundled with #4 — same function) |
| 2b | `register_contact_account` → `wallet.derive_extended_public_key` (`contacts.rs:186`) | xpub at hardened path | **yes** | **2** (steady-state no-op — see note) |
| 3 | contact-request / profile / contactInfo **doc signing** | `Signer<IdentityPublicKey>` | — | already done |
| 4 | contact-request xpub **encrypt** — `derive_encryption_private_key` (`identity_handle.rs:476`) → `EcdhProvider::SdkSide` (`sdk_writer.rs:240`) | ECDH | no | **2** |
| 5 | contact xpub **decrypt** — `register_external_contact_account` (`contacts.rs:472/510/514`) | ECDH | **yes** | **2 — DEFERRED via queue** |
| 6 | `accountReference` (`contact_requests.rs:204`; `dip14.rs:262`) | HMAC keyed by the **same** ECDH key | partial | **2** |
| 7 | contactInfo AES keys — `derive_contact_info_keys` (`contact_info.rs:80`) via sync (read) + publish (write) | raw hardened-child bytes as AES-256 key | **yes (read)** | **2 — DEFERRED via queue** |

Reclassification vs v2 (from the review):

- **Sites 2 and 2b moved Phase 1 → Phase 2.** Both live in functions that
  *also* perform Phase-2 ECDH (#4 in `send_contact_request_with_external_signer`;
  #5 in the `register_external` path that the sweep drives). Converting their
  xpub piecemeal in Phase 1 would double-touch the same functions. Bundle each
  xpub conversion with the ECDH conversion of its host function (one
  wallet-HD closure threading per function). Phase 1 stays exactly the
  already-shipped, green slice (#1 + the `extended_public_key` foundation +
  dead-API deletion).
- **2b is a steady-state no-op.** `register_contact_account` has an early-exit
  (`contacts.rs:153–174`): once the receiving account exists it returns before
  the derive at `:186`. The account **is persisted** —
  `AccountRegistrationEntry.account_xpub: ExtendedPubKey` (`changeset.rs:967`)
  bincode round-trips (`persistence.rs:2387`/`2869`) and restores via
  `Account::from_xpub` into a watch-only `new_external_signable` wallet
  (`persistence.rs:2874/2889`), with the address pool + `used` flags
  (`AccountAddressPoolEntry`). Gap-limit refill is pure public `ckd_pub`
  (`KeySource::Public`, key-wallet `managed_account_trait.rs:295`). So the
  derive at `:186` fires **only** on a contact's first-ever registration in a
  session where it was never persisted — the deferral edge case (§4.6).
- **Only #5 (ECDH decrypt) and #7-read (contactInfo decrypt) are genuine
  background blockers.** Everything else the sweep does (request ingest,
  profile sync, address matching, gap-limit refill, payment reconcile) is
  pure public derivation or no derivation at all.

Notes (unchanged from v2):
- **#6 is not a separate key** — `calculate_account_reference` is HMAC-keyed by
  the same ECDH scalar as #4/#5; the ECDH handling covers it.
- **auto-accept (`auto_accept.rs:80`) is KEPT** — real but unwired DIP-15
  feature; converts cleanly to a sign path when wired. Tracked in the backlog (dashpay/platform#4020).
- **Dead read APIs** `contact_xpub` / `contact_payment_addresses` had zero
  callers → **DELETED in Phase 1**.

`attach_wallet_seed` consumers to remove (Phase 2, §4.9): FFI
`platform_wallet_manager_attach_wallet_seed_from_mnemonic` (`manager.rs:430`);
Swift `unlockWalletFromKeychain` (`PlatformWalletManager.swift:468`); test
helpers (`payments.rs`).

## 3. Capabilities the Keychain signer must expose

Two distinct, correctly-separate signer notions exist and stay separate
across the FFI seam (§4.4):

- **Doc-signer** — `dpp::identity::signer::Signer<IdentityPublicKey>`
  (state-transition signing). Impl = `VTableSigner`; FFI handle =
  `SignerHandle`/`VTableSigner` (seed never enters Rust). Already wired.
- **Wallet-HD signer** — `key_wallet::signer::Signer` (ECDSA-at-path,
  `public_key`, `extended_public_key`). Impl = `MnemonicResolverCoreSigner`;
  FFI handle = `MnemonicResolverHandle` (mnemonic transiently enters Rust;
  all crypto runs in FFI-crate Rust and wipes).

Wallet-HD capabilities:

1. **ECDSA sign at path** — EXISTS (`sign_ecdsa`).
2. **public_key at path** — EXISTS.
3. **extended_public_key at path** — DONE (key-wallet method + host primitive).
4. **ECDH** `(path, peer_pubkey) -> shared_secret` — NEW host primitive (§4.5).
5. **contactInfo seal/open** — NEW host primitive (§4.5).

Capabilities 4–5 are added as **inherent methods on `MnemonicResolverCoreSigner`**
(in `rs-sdk-ffi`, where it already lives) and consumed by platform-wallet via
**closures** (the existing `EcdhProvider::ClientSide` seam) — no new trait, no
new crate (§4.4).

## 4. Design

### Phase 1 — sign + xpub (low-risk, SHIPPED green)

#### 4.1 key-wallet change — LANDED

`key_wallet::signer::Signer::extended_public_key` added as a **provided
default that errors** (not a breaking required method); `InMemorySigner` test
impls override it; pinned rev bumped. `TransactionSigner`/`build_signed`
unchanged.

#### 4.2 host primitive: extended_public_key (FFI + Swift) — DONE + 1 hardening

`MnemonicResolverCoreSigner::extended_public_key` reuses the shared
`resolve_derived_xprv` helper, computes `ExtendedPubKey::from_priv`, and wipes
both scalars. Interop-guard test pins signer-xpub == `Wallet::derive_extended_public_key`.

**Phase-1 hardening (from the security audit) — TODO before merge:** wipe the
`master` scalar on the **error/unwind path** of `resolve_derived_xprv`. Today
the explicit `non_secure_erase` runs only in the `Ok` arms of `derive_priv`
and `extended_public_key`; if `master.derive_priv(path)` returns `Err`, or a
panic unwinds between materialization and the wipe, the `master` scalar leaks
(no `Drop`/`Zeroize`). Same gap in `sign_with_mnemonic_resolver.rs`. Preferred
fix: a small RAII wipe-guard around the two `ExtendedPrivKey`s (so all exit
paths wipe), rather than more hand-placed calls — there will be **five** such
sites once §4.5 lands.

**Path provenance (security requirement).** Paths are built in Rust
(`AccountType::…derivation_path()`, `identity_auth_derivation_path_for_type`,
the `dip14`/`contact_info` path builders) and passed **opaquely** through the
FFI; Swift never assembles a path.

#### 4.3 call-site conversions (Phase 1) — DONE

- `send_payment` takes `<S: key_wallet::signer::Signer>` and signs via
  `build_signed(signer, …)`; FFI `platform_wallet_send_dashpay_payment` takes
  a `MnemonicResolverHandle`; Swift threads the resolver under
  `withExtendedLifetime`.
- Dead `contact_xpub` / `contact_payment_addresses` deleted.
- Sites 2 and 2b are **not** converted here — moved to Phase 2 (§2).

### Phase 2 — raw-secret paths + seedless sweep + delete the workaround

#### 4.4 Signer & host-primitive model (no duplicate logic)

**Decision: two FFI handles; raw-secret ops as inherent methods on the
existing wallet signer, consumed via closures — NO new trait, NO new crate.**

- Keep `VTableSigner` (doc-signer) and `MnemonicResolverHandle` (wallet-HD) as
  **two** FFI handles. Merging them would either regress the doc-signer (seed
  currently never enters Rust) or force DIP-15 crypto into Swift — both
  rejected.
- `MnemonicResolverCoreSigner` (in `rs-sdk-ffi`) already **is** the wallet-HD
  binding (impls `key_wallet::signer::Signer`) and is constructed only by the
  `rs-platform-wallet-ffi` glue crate. `rs-sdk-ffi` and `platform-wallet` do not
  depend on each other, so a shared `WalletKeyProvider` *trait* would need a new
  crate or a layering inversion (the external `key-wallet` is ruled out — keep
  the cross-repo PR to one method, and ECDH must not become a `Signer` method).
  Avoid all of that: add the raw-secret capabilities as **inherent methods** on
  `MnemonicResolverCoreSigner` (it gains a `platform-encryption` dep — a leaf
  crypto crate, no cycle), and let platform-wallet consume them via **closures**
  — the `EcdhProvider::ClientSide { get_shared_secret }` seam it already uses,
  plus a closure param on the decrypt/drain path. The glue crate wires the
  closures from the signer's methods (it already owns the signer's construction
  + lifetime). *(An earlier draft proposed a `WalletKeyProvider: Signer`
  extension trait; dropped — no shared home given the crate graph, and the
  closure seam already exists.)*

Inherent methods on `MnemonicResolverCoreSigner` (sync — derivation is
CPU-bound + the resolver call is synchronous; the consuming `ClientSide` closure
wraps each in a future at the FFI seam):

```text
ecdh_shared_secret(path, peer_pubkey) -> Zeroizing<[u8;32]>                       // #4/#5  DONE
ecdh_shared_secret_and_account_reference(path, peer, compact_xpub, account_index, version)
                                        -> (Zeroizing<[u8;32]>, u32)              // #6
unmask_account_reference(path, prior_reference, compact_xpub) -> (u32, u32)       // #6
contact_info_seal(root_path, derivation_index, contact_id, plaintext, iv) -> ContactInfoSealed  // #7
contact_info_open(root_path, derivation_index, enc_to_user_id, blob) -> ContactInfoOpened        // #7
```

- Document-ops conversion: send/accept contact-request already thread a
  `doc_signer` for the state transition; for the xpub/ECDH they additionally
  receive the wallet-HD closure(s) the glue crate builds from the signer.
  `send_payment` keeps only its `key_wallet::Signer`. Swift passes the two
  handles it already holds — **no new Swift class, no new Swift crypto**.
- **Delete the dead `dash_sdk_dashpay_*` ClientSide FFI surface**
  (`rs-sdk-ffi/src/dashpay/contact_request.rs`: the two entry points,
  params/results, `DashSDKEcdhMode`, the four `*_with_{shared_secret,private_key}`
  helpers) and regenerate the cbindgen header. Zero non-Rust callers; it is a
  divergence-prone parallel orchestration of the same `rs-sdk` core, and its
  `SdkSide` raw-scalar ABI contradicts the new posture. The `rs-sdk`
  contact-request core + `EcdhProvider` stay (single source).

**No-duplicate-logic trace:** every host method body is "derive scalar at a
Rust-built path (the shared `resolve_derived_xprv`, scrubbed by the `WipingXprv`
guard on every exit path) → call the existing `platform_encryption` / `dip14`
fn → return the result." Zero new crypto. Single sources stay: DIP-15 ECDH/AES →
`platform-encryption`; accountReference HMAC + masking → `dip14`; contact-request
orchestration → `rs-sdk platform::dashpay::contact_request`; contactInfo wire
codec → `crypto/contact_info.rs` (plaintext-only, runs outside the primitive).

#### 4.5 raw-secret host primitives (option iii) + EcdhProvider collapse

For #4–#7 the host primitive derives the key **and runs the crypto in
FFI-crate Rust**, returning only the result; the raw scalar never reaches
`rs-platform-wallet` or Swift.

- **ECDH (#4/#5):** switch `sdk_writer.rs:240` from
  `EcdhProvider::SdkSide { get_private_key }` to
  `ClientSide { get_shared_secret }` backed by a closure calling
  `MnemonicResolverCoreSigner::ecdh_shared_secret` (DONE; parity-pinned).
  For all DashPay paths the model **collapses to `ClientSide` only**; delete
  `derive_encryption_private_key` (`identity_handle.rs:476`) and
  `SendContactRequestParams.ecdh_private_key` — the two places that
  materialize the raw scalar in `rs-platform-wallet`. Shared secret MUST be
  byte-identical to `platform_encryption::derive_shared_key_ecdh`
  (`SHA256((0x02|y_parity)‖x)`); peer pubkey validated on-curve before ECDH.
- **accountReference (#6):** folded into `ecdh_shared_secret_and_account_reference`
  / `unmask_account_reference` so the encryption scalar is used for both ECDH
  and the `dip14` HMAC in one derivation and never returns raw.
- **contactInfo (#7):** `contact_info_seal` / `contact_info_open` derive the
  two hardened-child AES keys and run encrypt/decrypt via `platform_encryption`,
  returning only ciphertext/plaintext. The DIP-15 wire codec stays in
  `crypto/contact_info.rs` (no key material).

**Interop parity (required):** host-path accountReference, contactInfo blob,
and ECDH secret MUST equal the in-process results for the same seed+path —
each pinned by a test (DIP-15 interop vs dashj/dash-shared-core).

#### 4.6 Seedless background-sync & the deferred-crypto queue (NEW — the core)

The recurring sweep has no signer and no user present (verified:
`DashPaySyncManager` holds no signer; FFI `platform_wallet_sync_contact_requests`
/ `…_dashpay_sync_start` / `…_sync_now` take none). Design rule: every sweep
op is **public-derivable** or **deferred** — never resident-seed.

**Persisted pending-crypto queue.** Add `pending_contact_crypto:
Vec<PendingContactCrypto>` to `PlatformWalletChangeSet`, keyed per
`(owner_identity_id, contact_id)` with an op discriminant:

```
PendingContactCrypto { owner_identity_id, contact_id,
  op: RegisterReceiving                                   // our friendship xpub (2b, first-time only)
    | RegisterExternal { encrypted_public_key, our_decryption_key_index,
                         contact_encryption_key_index }   // #5 ECDH decrypt
    | ContactInfoDecrypt,                                  // #7 (idempotent re-fetch+decrypt)
  enqueued_at_ms }
```

- Stores **only ciphertext + public key indices** (safe to persist). Rides the
  existing `persister.store(...)` changeset pipeline (no side channel);
  restored through `build_wallet_start_state`; a missing/old column restores as
  an empty queue (skip-and-continue convention).
- **Persisted, not in-memory**, because restore-from-Keychain is exactly when
  it's needed and the app may be killed between background discovery and the
  next foreground unlock.

**Enqueue (background, seedless).** In `build_contact_accounts` and
`sync_contact_infos`, when key material is unavailable (the `Unavailable`
classification, §4.7), **enqueue** instead of the current silent skip-and-log
/ retry-forever / channel-kill. Idempotent per `(owner, contact, op-kind)`.

**Drain (foreground, signer present)** — no new signer plumbing into the
background manager:

1. **On-unlock (primary):** a **new FFI** `platform_wallet_drain_pending_contact_crypto(wallet, core_signer)`
   wired into the same Swift code path that previously called
   `unlockWalletFromKeychain` — so deleting the re-attach and adding the drain
   are one change. It runs each entry through the §4.5 host primitives, then
   the public registration, and clears the entry.
2. **Opportunistic:** `send_payment` / `send_contact_request_with_external_signer`
   / `accept_…` / `set_contact_info_with_external_signer` each drain queue
   entries **for the identity they operate on** before their own work — so the
   first user action on a contact resolves its deferred crypto (e.g. tapping
   "Pay" on an inbound-only contact builds its `DashpayExternalAccount`).

Drain is idempotent (each op re-checks its early-exit guard). While queued, the
sweep still does all PUBLIC work for the contact (ingest, profile, address
match, reconcile); the contact is visible but "needs unlock to finish setup."

#### 4.7 Error classification: Transient / Unavailable / Permanent (must-fix)

The live bug behind "is deferral safe": `build_contact_accounts` today maps an
ECDH/derive failure to `Permanent` → `mark_contact_channel_broken`
(irreversible, `warn!`-only). A **locked Keychain is transient** but would
permanently disable payments to a contact. Fix before any Phase-2 coding:

- Introduce a **third** arm, `Unavailable` (signer/key material absent), in
  `RegisterExternalError` (and the contactInfo path), **distinct** from
  `Transient` (retry soon) and `Permanent` (malformed data → mark broken).
- `Unavailable` → **enqueue (§4.6) and pause this contact's build; never kill,
  never churn-retry every 15s.** Only genuinely malformed inputs (bad
  ciphertext, off-curve pubkey) are `Permanent`.
- **Fix the `is_seedless` gate** (`contact_requests.rs:904–921`): it currently
  keys on `identity_index.is_none()`, which a seedless-but-indexed identity
  passes — falling through to the channel-kill. Gate on "can I derive **right
  now**?" (key material available), not "is this a wallet-owned identity?".
- Add a **needs-rebuild / needs-unlock marker** surfaced to the UI for
  deferred contacts.

#### 4.8 wrong-seed safety check (replaces the deleted dual gate)

`attach_wallet_seed`'s dual id/xpub gate also verified the Keychain mnemonic
binds to the loaded wallet. Replace it with a **one-time xpub self-check** at
signer construction / first use: derive BIP44 account-0 xpub from the resolved
mnemonic and compare to the wallet's persisted account-0 xpub; mismatch fails
loud. **Caveat (security audit):** this catches a *wrong* seed but **not** a
*present-but-zero-keys* import (the open imported-identity bug, §7
prerequisite).

#### 4.9 delete the workaround — ORDERING CONSTRAINT

Remove `attach_wallet_seed`, the FFI export, `unlockWalletFromKeychain`'s
re-attach (replaced by the §4.6 drain), the dual gate + `mem::swap`; rework
the test helpers to inject a test `Signer` / pre-seed the queue. Also delete
or make-throwing the legacy `KeychainSigner.sign(identityPublicKey:data:)
-> Data?` swallow (returns reasonless `nil` on any failure).

**Do NOT execute this deletion until the sweep is seedless-safe** (§4.6 queue
+ §4.7 classification landed and tested). Until then the resident seed is what
keeps the signerless sweep working; removing it first turns every background
tick on an unbuilt contact into an irreversible channel-kill.

## 5. Failure modes

- **Signer unavailable mid-sweep (ECDH/xpub build):** classified `Unavailable`
  → enqueued + paused, **not** channel-killed, **not** retried every 15s
  (§4.7). Resolves on next drain.
- **Keychain locked while the tokio sweep ticks:** the loop has no
  scenePhase/BG gating; it MUST hit the `Unavailable`+enqueue path, never the
  kill path.
- **Pending queue never drains:** mitigated by the on-unlock drain wired to the
  old re-attach trigger + opportunistic drain on any signer-present action +
  a UI "needs unlock" marker. Pinned by an on-device test.
- **Poison entry (permanently malformed ciphertext):** `Permanent` → mark
  broken + clear entry (no requeue). Transient → keep for next drain.
- **Partial registration** (persist-ok / in-memory-insert-fail): unchanged
  (store-before-insert; relaunch rebuilds) — but the rebuild must classify a
  locked Keychain as `Unavailable`, not escalate.
- **Restore-from-Keychain / app-reinstall → zero signing keys:** the
  imported-identity bug — **RESOLVED** by carry-scalar (the materialization path no
  longer re-derives from a not-yet-stored mnemonic). Phase 2 only confirms imported
  wallets reach the resolver for xpub/ECDH (mnemonic stored by `createWallet`).
- **Wrong/mis-mapped mnemonic:** §4.8 xpub self-check, fails loud.
- **Read-path:** converted readers propagate signer errors; never return
  empty/zero/stale addresses.

## 6. Test plan

- **key-wallet:** `extended_public_key` on `InMemorySigner` matches
  `derive_extended_public_key` (DONE).
- **platform-wallet:** test `Signer` replaces `attach_wallet_seed`;
  signer-based tests for `send_payment` (DONE: interop-guard) + contact-request
  create/accept (ECDH), contactInfo round-trip; **interop parity** tests
  (accountReference, contactInfo, ECDH byte-equal to in-process).
- **Seedless sweep:** watch-only wallet + persisted xpubs → sweep does all
  PUBLIC ops with no signer; `register_contact_account` early-exit no-op once
  persisted.
- **Deferral queue:** background-discover inbound contact → `Unavailable` →
  enqueue (not kill); drain on unlock → contact payable. A `Permanent` error
  clears the entry AND sets `payment_channel_broken`. A locked-Keychain
  (transient) does **not** mark broken.
- **Per-primitive no-residue:** each inherent host-primitive method wipes scalars
  on Ok **and error/unwind** paths.
- **FFI:** input-validation (null/oversize/bad-path) incl. contactInfo depth-6
  paths (hardened 65536/65537).
- **Acceptance grep:** `git grep attach_wallet_seed` empty; no surviving
  `derive_extended_private_key` / `build_signed(wallet` on DashPay paths.
- **On-device:** clean wipe → import → discover → send/accept contact request,
  send payment, publish profile + contactInfo, **background-discover an
  inbound contact then unlock → it becomes payable** — all with **no**
  re-attach.

## 7. Rollout order (revised)

1. **Phase 1 (SHIPPED, green):** key-wallet method → host `extended_public_key`
   primitive → `send_payment` conversion → delete dead read APIs. **Remaining
   before merge:** the §4.2 error-path wipe hardening + iOS `build_ios.sh`
   verification.
2. **Phase 2 prerequisites (design + fix BEFORE feature code):**
   - §4.7 three-state error classification + `is_seedless` gate fix.
   - §4.6 persisted pending-crypto queue + drain FFI.
   - ~~Resolve the imported-identity zero-signing-keys bug~~ **RESOLVED** by the
     carry-scalar change (commit `c567981c46`; on-device 23/23 signable,
     regression tests green). No longer a blocker. Phase 2 need only confirm an
     imported wallet reaches the resolver for xpub/ECDH (its mnemonic is stored in
     the Keychain by `createWallet`, so the resolver-backed signer works for it).
3. **Phase 2 feature code:** inherent host primitives on
   `MnemonicResolverCoreSigner` (ECDH + accountReference + contactInfo) →
   `EcdhProvider` collapse to `ClientSide` →
   convert #2/#2b/#4–#7 (each xpub bundled with its function's ECDH) → delete
   the dead `dash_sdk_dashpay_*` surface → §4.8 self-check.
4. **Only then §4.9:** delete `attach_wallet_seed` + re-attach + legacy
   `KeychainSigner.sign(...)->Data?`.
5. Build + clippy + tests + on-device acceptance after each phase.

Cross-repo: the key-wallet edit (already landed) needed Claude Code run from
`/Users/ivanshumkov/Projects/dashpay/` (sibling-repo writes). FFI/Swift work
goes through the **swift-rust-ffi-engineer** agent.

## 8. Alternatives rejected

- **In-memory seed (dashj / dash-shared-core).** Reference clients hold the
  decrypted seed in-session — no signer-based DashPay to copy. Rejected:
  abandons the #3639 posture.
- **Phase-1-only (xpub/sign):** does not delete `attach_wallet_seed`. Adopted
  *as Phase 1*, not the end state.
- **Keep the workaround:** rejected by product decision.
- **Unified single signer handle (doc + wallet + ECDH):** rejected — regresses
  the doc-signer (seed never in Rust today) or pushes DIP-15 crypto into Swift.
  Add the wallet-side raw-secret ops as inherent methods on the existing
  signer, consumed via closures, instead (§4.4).
- **§4.5 option (i) (return raw scalar):** rejected for option (iii) — exposes
  the ECDH key raw, defeating the hashing; the carry-scalar precedent
  (write-once Rust→Swift) does not sanction the read-many reverse flow.
- **Skip-and-log deferral (current code):** rejected — silently strands
  contacts and (worse) the `Permanent` path irreversibly kills channels.
  Replaced by the §4.6 persisted queue + §4.7 classification.

## 9. Security must-fixes from the multi-agent review (priority order)

1. **[CRITICAL]** Three-state classification (`Unavailable` ≠ `Permanent`);
   never auto-`mark_contact_channel_broken` on unavailable key material (§4.7).
2. **[CRITICAL]** Give the sweep a seedless-safe path: enqueue+defer, never
   derive-or-die; do not delete `attach_wallet_seed` until this lands (§4.6,
   §4.9 ordering).
3. **[CRITICAL]** Fix the `is_seedless` gate predicate — key-availability, not
   `identity_index.is_none()` (§4.7).
4. **[RESOLVED]** Imported-identity zero-signing-keys bug — fixed by carry-scalar
   (commit `c567981c46`), on-device-verified, regression tests green. No longer a
   Phase-2 blocker; only confirm imported wallets reach the resolver for xpub/ECDH.
   (Note: §4.8's xpub self-check still does not cover a present-but-zero-keys import,
   but the carry-scalar fix means imports now materialize keys, so this is moot.)
5. **[HIGH]** Tighten §1 honest-scope wording (done) + fix the
   `resolve_derived_xprv` error-/unwind-path scalar leak; prefer one RAII
   wipe-guard over five hand-placed `non_secure_erase` calls (§4.2).
6. **[MEDIUM]** Delete / make-throwing the legacy `KeychainSigner.sign(...)->Data?`
   nil-swallow (§4.9).
7. **[MEDIUM]** UI marker for contacts pending an unlock-drain (§4.6/§4.7).

## 10. Resolved review questions

- **key-wallet method:** provided-default-that-errors — §4.1.
- **raw-secret:** option (iii) via inherent host primitives on the wallet
  signer, consumed by closures — §4.4/§4.5.
- **signer surface:** two FFI handles; raw-secret ops as inherent methods +
  `EcdhProvider::ClientSide` closures (no new trait/crate) — §4.4.
- **dead `dash_sdk_dashpay_*` surface:** delete — §4.4.
- **read-API ripple:** dead → deleted — §2.
- **dual-gate deletion:** safe for grafting; wrong-seed detection preserved via
  the §4.8 self-check (but not zero-keys — §7).
- **ECDH placement:** FFI-layer host primitive, not a key-wallet trait method —
  §3/§4.5.
- **"defer site 2b":** safe **only** with §4.6 queue + §4.7 classification +
  §4.9 ordering. Without them it is silent, irreversible channel corruption.
- **pre-check:** Swift uses `platform_wallet_send_contact_request_with_signer`;
  the rs-sdk-ffi ClientSide surface is the reference template for the ECDH
  switch and is deleted afterward.
