# Create Identity from Core Funds — Plan (Draft 9)

Status: **iter 1 + 2 + 4 done. Testnet validation hit an SPV event-routing concern (separate investigation). iter 3 (SwiftData mirror) and iter 5 (resume picker) still pending.**
Branch: `feat/swift/funding-with-asset-lock`
Target: SwiftExampleApp, testnet validation first.

**Draft 7 update (mid-iter-1 discovery)**: testnet validation of iter 1
revealed the asset-lock builder dies with `"Cannot sign with watch-only
wallet"` — misleading error from
`key-wallet::asset_lock_builder.rs:188` which collapses both
`WalletType::WatchOnly` AND `WalletType::ExternalSignable` errors into
the same `WatchOnlyWallet` variant. Investigation confirmed:

- `persistence.rs:1688` already correctly creates wallets as
  `ExternalSignable` on reload (the comment about "watch-only" elsewhere
  is stale).
- The real gap: `build_asset_lock_transaction` calls the soft-only
  `build_asset_lock` instead of `build_asset_lock_with_signer`. No Core
  signer is plumbed through any layer of platform-wallet / FFI / Swift.
- The existing single `signer` parameter on
  `register_identity_with_funding_external_signer` is only for Platform
  state-transition signing — never reaches Core asset-lock signing.

**Iter 2 is now "Core signer plumbing"** — was "SwiftData mirror".
SwiftData mirror moves to iter 3. See § Iter 2 — Core signer plumbing.

This document captures the plan for wiring "Create a Platform
identity using an asset-lock proof, funded from Core wallet
UTXOs" in `CreateIdentityView`. Delivered in seven incremental
iterations — each is testable end-to-end on testnet before the
next one starts, so we can stop, redirect, or expand scope
after every step.

The ordering prioritizes user-visible progress: iter 1 ships a
working (if minimal) feature, iter 2 + 3 layer on the SwiftData
mirror and the stage-aware progress bar, then iter 4 does the
Rust refactor (which is invisible to users but fixes a leak and
unlocks resume). Iter 5 ships resume.

## Goal

User opens the app, picks a Core account from a wallet that has
testnet DASH UTXOs, picks an identity registration index, hits
"Create". Rust builds the asset-lock funding tx, broadcasts it,
waits for the instant-send lock, registers the identity on
Platform, persists the new `PersistentIdentity` + identity auth
keys. Swift only marshals the call, mirrors the tracked asset
lock to SwiftData (from iter 2 onward), and surfaces stage
progress + errors (from iter 3 onward).

Two related modes covered (split across iter 1 and iter 5):

- **Fund from wallet** — build a new asset lock from wallet
  UTXOs. Delivered iter 1.
- **Fund from unused asset lock** — resume a previously built
  asset lock by outpoint (recovery path after a crash, network
  error, or dismissed flow that left funds locked). Delivered
  iter 5, depends on the Rust refactor in iter 4.

## Testnet prerequisite

Wallet inside the app must hold testnet DASH. Two options:

1. **Fresh wallet**: create one inside the app (network defaults
   to testnet — `AppState.swift:51`), copy first receive
   address, fund from <https://testnet-faucet.dashevo.org/>.
2. **Existing funded testnet mnemonic**: import via the wallet-
   import flow. Needs ≥ `assetLockMinimum + fee + headroom`
   spendable duffs.

The mnemonic never crosses the FFI boundary into Swift (see
`packages/swift-sdk/CLAUDE.md`). All derivation, UTXO
selection, tx construction, broadcast, instant-lock wait, and
identity registration happen Rust-side.

## Rust pipeline (post-iter-4 shape)

Iter 1, 2, 3 use the function as it exists today
(`register_identity_with_funding_external_signer`). Iter 4
refactors it into the two-layer factoring below and adds
resume + cleanup. From iter 4 onward, the pipeline shape is:

**L2 — `register_identity_with_funding`** (renamed wallet-managed function):

```rust
async fn register_identity_with_funding<S: Signer<...>>(
    funding: IdentityFunding,    // FromWalletBalance | FromExistingAssetLock | UseAssetLock
    identity_index: u32,
    keys_map: BTreeMap<u32, IdentityPublicKey>,
    signer: &S,
    settings: Option<PutSettings>,
) -> Result<Identity, PlatformWalletError>
```

Dispatch:

- `IdentityFunding::FromWalletBalance { amount_duffs }` →
  `asset_locks.create_funded_asset_lock_proof()` builds tx →
  broadcasts → waits for IS-lock → returns `(proof, key,
  out_point)`.
- `IdentityFunding::FromExistingAssetLock { out_point }` →
  `asset_locks.resume_asset_lock()` picks up at whatever stage
  the tracked lock left off at and re-derives the one-time
  private key.
- `IdentityFunding::UseAssetLock { proof, private_key }` →
  passes through (no asset-lock build/resume; outpoint derived
  via `out_point_from_proof` for cleanup).

All three paths submit via L1 (`register_identity_with_signer`)
with IS→CL fallback, then add to `IdentityManager`, then
`remove_asset_lock` on success.

### Stages (for the progress bar)

Defined as `AssetLockStatus` at
`packages/rs-platform-wallet/src/wallet/asset_lock/tracked.rs:17`:

| Stage | Description | Persister fired |
|---|---|---|
| `Built` | Tx constructed locally | ✅ `build.rs:316` |
| `Broadcast` | Sent to Core network | ✅ `build.rs:330` |
| `InstantSendLocked` | IS-locked (usable to register) | ✅ `build.rs:353` |
| `ChainLocked` | Mined into a chain-locked block | ✅ `recovery.rs:238` |
| `RegisteringOnPlatform` | Platform state-transition in flight | — (UI-only label, not in Rust enum) |
| `Done` | Identity registered + tracked lock removed | — (signaled by row deletion) |

Every status transition emits a changeset to the persister.
FFI snapshot: `platform_wallet_tracked_asset_locks_list()` at
`packages/rs-platform-wallet-ffi/src/manager_diagnostics.rs:258`;
Swift wrapper: `PlatformWalletManagerDiagnostics.swift:162`
(`trackedAssetLocks(for:)` → `[TrackedAssetLockSnapshot]`).

## Persistence on the Swift side — the gap

Rust persists tracked asset locks robustly. Swift currently does
**not** mirror them to SwiftData — queryable only via the FFI
snapshot. Implications:

- Cross-launch recovery works (Rust reloads the lock on wallet
  init), but the SwiftUI explorer surfaces don't see a tracked
  lock until something queries the FFI.
- The progress-bar UI cannot reactively follow a stage
  transition via `@Query`.
- The explorers have no row for tracked asset locks beyond a
  count.

**Resolved in iter 2.** From iter 2 onward, a `PersistentAssetLock`
SwiftData model mirrors `TrackedAssetLock` via a new FFI
persister callback.

## Existing UI to extend

| Surface | File:line | Current state |
|---|---|---|
| `CreateIdentityView` — Source Wallet + Account picker | `CreateIdentityView.swift` whole file | Platform Payment path wired; Core / CoinJoin / walletless stubbed |
| `CreateIdentityView` — "Fund from unused Asset Lock" picker entry | `CreateIdentityView.swift:200-201` | Picker exists; submit path stub |
| `StorageExplorerView` | `StorageExplorerView.swift:5,27-78` | 18 Persistent* models listed; no AssetLock row |
| `WalletMemoryExplorerView` — count only | `WalletMemoryExplorerView.swift:166,368` | Shows "N asset locks" count; no drill-down |

---

## Delivery iterations

Each iteration ships independently. Iter 1 is the smallest path
to "wallet-balance identity creation works"; refactors and
polish layer on after.

### Iter 1 — Wire existing function from Swift (Swift only, no Rust changes)

**Goal**: prove the wallet-balance path works end-to-end on
testnet without touching Rust. Smallest possible change.

The existing `registerIdentityWithFunding(amountDuffs:identityIndex:identityPubkeys:signer:)`
at `ManagedPlatformWallet.swift:2356` already wraps
`platform_wallet_register_identity_with_funding_signer`, which
calls `register_identity_with_funding_external_signer`. That
function's `FundWithWallet { amount_duffs }` branch is exactly
the wallet-balance path. All the infrastructure exists.

**Steps**:

1. Detect when the chosen `PersistentAccount` is a Core /
   CoinJoin account vs Platform Payment in
   `CreateIdentityView.submit()`.
2. Add a funding-amount UI field (default 100,000 duffs / 0.001
   DASH). Validate against the selected account's spendable
   balance.
3. On submit:
   - `prePersistIdentityKeysForRegistration(identityIndex:,
     keyCount: 3, network:)` → `[(path, pubkeyBytes)]`.
   - Map to `[ManagedPlatformWallet.IdentityPubkey]`: first key
     `securityLevel = .master`, remaining `.high`.
   - `registerIdentityWithFunding(amountDuffs:, identityIndex:,
     identityPubkeys:, signer: KeychainSigner)`.
4. Generic in-flight spinner with "Registering identity…"
   message — no stage UI yet.
5. Disable the "Fund from unused Asset Lock" picker option
   (resume support arrives iter 4 + 5).
6. Manual testnet validation: fund wallet, create identity at
   index 0 with 100,000 duffs, verify on
   <https://testnet.platform-explorer.com/>.

**Known limitations (deliberately deferred)**:

- **Tracked asset locks leak in Rust state on success.**
  `_out_point` is dropped at `registration.rs:105`. Silent in
  iter 1 (no Swift mirror), surfaces as clutter in iter 2-3
  (rows accumulate but progress bar still works), fixed in
  iter 4.
- **No stage progress bar.** Generic spinner only. Fixed in
  iter 3.
- **No resume path.** Picker option exists but submit is
  stubbed. Fixed in iter 5.
- **No crash recovery UI.** If the app dies mid-flow, Rust
  retains the tracked lock internally but Swift cannot see or
  act on it. Fixed in iter 5 (resume picker becomes the
  recovery affordance).

---

### Iter 2 — Core signer plumbing (unblocks asset-lock signing for ExternalSignable wallets)

**Goal**: pipe Core ECDSA signing through KeychainSigner so that
asset-lock-funded identity operations work on wallets where the seed
isn't in Rust (every wallet reloaded from persister state — today's
default via `Wallet::new_external_signable`).

**Two distinct Core-side signing operations** in the asset-lock-funded
identity flow:

1. **BUILD phase** — sign each UTXO input of the asset-lock tx. Today
   uses soft `build_asset_lock(wallet, …)` which calls
   `wallet.root_extended_priv_key()` and fails for ExternalSignable.
   Fix: switch to `build_asset_lock_with_signer(wallet, …, signer)` and
   pass our Core signer.
2. **CONSUME phase** — sign the asset-lock-proof's credit-spending
   signature on the IdentityCreate state transition. Today uses
   `state_transition.sign_by_private_key(asset_lock_proof_private_key,
   ECDSA_HASH160, bls)` at `rs-dpp::identity_create_transition/v0/
   v0_methods.rs:78` — requires raw `&[u8]` private key. Fix: add a
   sibling `sign_with_signer(path, signer)` to `StateTransition` and
   route through it.

The CONSUME-phase fix lives in rs-dpp + rs-sdk and requires upstream
additions there (purely additive — old paths stay).

**The same gap exists in IdentityTopUp via asset-lock.** Same shape
fix applied to `top_up_identity` family in rs-sdk + rs-dpp's top-up
transition. ~80 LoC additional, same PR.

**Naming convention** (sibling functions, not rename of existing):

| Layer | Existing function (now explicit) | New sibling |
|---|---|---|
| rs-dpp `IdentityCreateTransitionV0` | `try_from_identity_with_signer_and_private_key` (renamed from `try_from_identity_with_signer`) | `try_from_identity_with_signers` (plural — both args are signers) |
| rs-dpp `IdentityTopUpTransitionV0` | same rename pattern | new `_with_signers` sibling |
| rs-dpp `StateTransition` | `sign_by_private_key` (keep — already explicit) | new `sign_with_signer<S: key_wallet::Signer>` |
| rs-sdk `PutIdentity::put_to_platform` | `put_to_platform_with_private_key` | `put_to_platform_with_signer` (singular — only one new signer added) |
| rs-sdk `BroadcastNewIdentity::broadcast_request_for_new_identity` | `..._with_private_key` | `..._with_signer` |
| rs-sdk internal `put_identity_with_asset_lock` | `put_identity_with_asset_lock_and_private_key` | `put_identity_with_asset_lock_and_signer` |

**Plural at rs-dpp, singular at rs-sdk** because at rs-dpp both
parameters are visible signers; at rs-sdk the identity signer is
implicit (always present) and we're adding one new signer.

**Reuse the existing mnemonic-resolver pattern — no new Swift signer.**
KeychainSigner already vends a `MnemonicResolverHandle` for Platform-
address signing today. The Core-side signer (`MnemonicResolverCoreSigner`,
Rust-only) wraps that same handle and implements
`key_wallet::signer::Signer`. Each signing call is atomic — derive +
sign + zero inside a single FFI round-trip, identical security profile
to today's Platform-address signing. **No private key ever lives in
Rust memory across operations.**

**The gap surfaced during iter 1 testnet validation**: the asset-lock
builder uses the soft-only `build_asset_lock` (`asset_lock/build.rs:82`)
which requires `wallet.root_extended_priv_key()`. That returns an error
for both `WatchOnly` and `ExternalSignable` wallet types — collapsed
into the misleading "Cannot sign with watch-only wallet" error from
`key-wallet::asset_lock_builder.rs:188`. The sibling
`build_asset_lock_with_signer` exists upstream but has zero callers in
this repo.

**Steps**:

**Step 1 — Delete `VTableCoreSigner`** ✅ **DONE** (`cargo check -p rs-sdk-ffi` clean)
- The trampoline at `packages/rs-sdk-ffi/src/core_signer.rs` (671 LoC,
  7 tests) was an over-engineered generic vtable bridge. We don't need
  generic external signer flexibility — KeychainSigner-via-mnemonic-
  resolver is the only signer we have.
- Delete the file, remove module export in `lib.rs`.

**Step 1b — Add `MnemonicResolverCoreSigner`** ✅ **DONE** (332 LoC, 5 tests passing) at
`packages/rs-platform-wallet-ffi/src/mnemonic_resolver_core_signer.rs`:
- Holds a `MnemonicResolverHandle` (raw FFI handle into the Swift
  KeychainSigner's resolver) + network.
- Implements `key_wallet::signer::Signer` with:
  - `supported_methods()` returns `&[SignerMethod::Digest]`.
  - `sign_ecdsa(path, sighash)`: resolves mnemonic via handle, derives
    Core priv key at `path`, signs the digest with
    `dpp::dashcore::signer::sign(sighash, &secret_bytes)`, zeroes the
    key buffer, returns `(Signature, PublicKey)`. Mirrors the body of
    `dash_sdk_sign_with_mnemonic_resolver_and_path` lines 149-234.
  - `public_key(path)`: same flow but skips signing — just returns the
    derived compressed pubkey.
- **Atomic per-call**: each method call is one derive + (sign|peek) +
  zero round-trip. No priv key persists across method calls or across
  FFI boundary.

**Step 2 — `rs-dpp`: add `StateTransition::sign_with_signer`** ✅ **DONE** at `state_transition/mod.rs:1291`. Byte-parity test at `:3257` passing. `cargo check -p dpp` clean across all feature combos. 908 lib tests pass.
- Sibling to `sign_by_private_key` at
  `packages/rs-dpp/src/state_transition/mod.rs:1206`.
- Signature:
  ```rust
  pub async fn sign_with_signer<S: key_wallet::signer::Signer>(
      &mut self,
      path: &DerivationPath,
      signer: &S,
  ) -> Result<(), ProtocolError>
  ```
- Body: compute `signable_bytes()`, apply same digest pre-image as
  existing ECDSA path, call `signer.sign_ecdsa(path, digest)`,
  serialize the resulting signature and call `set_signature(...)`.
- **Correctness check up front**: verify what `signer::sign(&data,
  private_key)` (currently used at line 1226) does to `data` before
  signing (raw / SHA256 / double_sha256). New `sign_with_signer` MUST
  apply the same transform so signatures verify byte-identically.

**Step 3 — `rs-dpp::IdentityCreateTransitionV0`: rename + new sibling** ✅ **DONE** at `v0_methods.rs:38` (renamed) and `:90` (new). Trait + outer-enum dispatcher updated.
- File: `packages/rs-dpp/src/state_transition/state_transitions/identity/identity_create_transition/v0/v0_methods.rs:38`.
- Rename `try_from_identity_with_signer` →
  `try_from_identity_with_signer_and_private_key` (no behavior change).
- Add new `try_from_identity_with_signers<IS, AS>(...)`:
  - `IS: Signer<IdentityPublicKey>` — same as old, signs per-key witnesses.
  - `AS: key_wallet::signer::Signer` — signs the asset-lock-proof line.
  - New parameter: `asset_lock_proof_path: DerivationPath` — where AS
    should sign.
  - Body: identical to legacy until line 78; replace
    `state_transition.sign_by_private_key(asset_lock_proof_private_key,
    ECDSA_HASH160, bls)?` with
    `state_transition.sign_with_signer(&asset_lock_proof_path,
    asset_lock_signer).await?`.
- Update the trait `IdentityCreateTransitionMethodsV0` and its
  dispatcher in `methods/mod.rs` to expose both functions.

**Step 4 — `rs-dpp::IdentityTopUpTransitionV0`: same shape fix** ✅ **DONE** at `v0_methods.rs:29` (renamed → `_with_private_key`) and `:61` (new `_with_signer`).
- Top-up has the same asset-lock-private-key signing pattern. Mirror
  Step 3 in that transition module.

**Step 5 — `rs-sdk::PutIdentity` + helpers** ✅ **DONE** in `put_identity.rs`: renamed legacy methods to `_with_private_key`, added `put_to_platform_with_signer` (`:71`/`:187`) + `_and_wait_for_response_with_signer` (`:90`/`:212`). Helper `put_identity_with_asset_lock_and_signer` at `:308`. Also propagated through `rs-sdk-ffi`, `wasm-sdk`, `rs-drive-abci`, `strategy-tests` (50+ call-site renames). Tests: 117/117 pass.
- File: `packages/rs-sdk/src/platform/transition/put_identity.rs`.
- Rename existing trait methods `put_to_platform` /
  `put_to_platform_and_wait_for_response` →
  `put_to_platform_with_private_key` / `..._and_wait_for_response_with_private_key`.
- Add new sibling methods `put_to_platform_with_signer` /
  `put_to_platform_and_wait_for_response_with_signer`. Same shape but
  take asset-lock signer + path instead of `asset_lock_proof_private_key`.
- Rename internal helper `put_identity_with_asset_lock` →
  `put_identity_with_asset_lock_and_private_key`. Add new
  `put_identity_with_asset_lock_and_signer`.

**Step 6 — `rs-sdk::BroadcastNewIdentity` mirror** ✅ **DONE** in `broadcast_identity.rs`: `..._with_private_key` (rename) at `:97`/`:129`, new `..._with_signer` at `:114`/`:153`. Also propagated TopUpIdentity in `top_up_identity.rs` (`top_up_identity_with_signer` at `:39`/`:79`).
- File: `packages/rs-sdk/src/platform/transition/broadcast_identity.rs`.
- Rename `broadcast_request_for_new_identity` →
  `broadcast_request_for_new_identity_with_private_key`. Add new
  `broadcast_request_for_new_identity_with_signer`.
- Both top-up entry points get the same treatment.

**Step 7 — `rs-platform-wallet::asset_lock/build.rs`** ✅ **DONE**:
- Change `build_asset_lock_transaction` (line 40) to take a Core
  signer parameter (`&S where S: key_wallet::signer::Signer`).
- Replace soft `build_asset_lock(wallet, account_index, fundings, fee)`
  at line 82 with `build_asset_lock_with_signer(wallet,
  account_index, fundings, fee, signer)`.
- Result type changes: `AssetLockCreditKeys::Public(Vec<(PublicKey,
  DerivationPath)>)` instead of `Private(Vec<[u8;32]>)`. Plumb the
  `(pubkey, path)` tuple through the call chain.
- Update the existing soft-path branch handling (delete it — soft path
  cannot work for ExternalSignable, which is now the universal mode).

**Step 8 — `rs-platform-wallet::wallet/core/broadcast.rs`** ✅ **DONE**:
- `send_to_addresses` (line 38) takes a Core signer too. Swap
  `build_signed(wallet, …)` (line 120) → `build_signed(signer, …)`.

**Step 9 — `rs-platform-wallet::identity/network/registration.rs`** ✅ **DONE**:
- `register_identity_with_funding_external_signer` (line 59) takes
  asset-lock signer + identity-registration path. Routes through new
  `put_identity_with_asset_lock_and_signer`.
- Same for top-up siblings.

**Step 10 — `rs-platform-wallet-ffi`** ✅ **DONE**:
- Extend `platform_wallet_register_identity_with_funding_signer`
  (`identity_registration_funded_with_signer.rs`) with a
  `mnemonic_resolver_handle: *mut MnemonicResolverHandle` parameter
  (NOT a new SignerHandle). Same handle type as Platform-address
  signing already uses. Inside the FFI, construct
  `MnemonicResolverCoreSigner` from the handle and pass to the
  platform-wallet API.
- Same for `core_wallet_send_to_addresses` (used by SendViewModel).

**Step 11 — Swift `KeychainSigner`** ✅ **DONE** — confirmed no new code needed. `MnemonicResolver` (the existing class at `MnemonicResolverAndPersister.swift`) already vends the handle the FFI consumes. Just pass `MnemonicResolver().handle` through to the FFI.
- `KeychainSigner` already vends `mnemonicResolverHandle: MnemonicResolverHandle`
  used by Platform-address signing. No new property, no new vtable.
- The architectural rule "no private keys outside Swift" is preserved
  because every `MnemonicResolverCoreSigner` method does atomic
  derive+sign+zero — same security profile as today's
  Platform-address path.

**Step 12 — Swift call sites** ✅ **DONE**:
- `ManagedPlatformWallet.registerIdentityWithFunding(...)`: pass the
  existing `keychainSigner.mnemonicResolverHandle` to the extended
  FFI alongside the existing identity signer handle.
- `SendViewModel`: pass the same handle to the Core-side send FFI.
- `CreateIdentityView.submitCoreFunded` updates call site
  mechanically.

**Step 13 — Validation** ⚠️ **PARTIAL** — Core signer pipeline validated end-to-end on testnet (asset-lock tx confirmed on chain, our signer correctly signed UTXO inputs), but the wait-for-proof poll loop times out because **SPV→wallet event routing isn't delivering IS-lock / chain-lock context updates to `bip44_account.transactions()`**. The Swift app shows the tx as "Confirmed" but the asset-lock manager's tracked status stays at `Broadcast`. Diagnosed as either testnet masternode silence or a regression in dash-spv → wallet integration (likely from recent rust-dashcore bumps). **Iter 4's auto IS→CL fallback was triggered but ALSO timed out** because the chain-lock event never propagated to our poll either. See § Iter 5 / SPV event-routing follow-up.
- `cargo check` clean across rs-dpp, dash-sdk, platform-wallet,
  platform-wallet-ffi after each layer lands.
- xcframework rebuild.
- Re-run testnet Core-funded identity creation → succeeds.
- Test normal Core send → succeeds.
- Optionally: test top-up via asset-lock if iter 1 isn't blocked on it.

**Step 14 — Fix the misleading error string** (upstream, follow-up)
- `key-wallet::asset_lock_builder.rs:188` collapses both `WatchOnly`
  and `ExternalSignable` errors into `WatchOnlyWallet`. Distinguish
  them. Cosmetic / debuggability. Out of this PR.

---

### Review findings (post-implementation, pre-testnet)

Three reviewers audited the iter 2 implementation: a crypto/security
auditor, an adversarial reviewer, and a Rust quality engineer. Summary:

**No critical findings.** Byte-parity of `sign_with_signer` vs the
legacy private-key path is confirmed (same `double_sha`, same
RFC6979, same low-s, same compact-65 framing — pinned by the test
at `rs-dpp::state_transition::mod.rs:3257`). Path equality between
build-phase and consume-phase is structurally enforced. Recovery-id
brute force is correct. No double-hashing. Swift mnemonic plaintext
is XOR-masked + `memset_s`-zeroed.

#### ✅ Fixed in this session

- **Adversarial P0 #4** — `_ = coreSigner` lifetime folklore.
  Replaced with `withExtendedLifetime((signer, coreSigner)) { ... }`
  in `ManagedPlatformWallet.swift:2376`. `_ = x` is not a Swift
  language guarantee; the optimizer may release in `-O` builds
  causing UAF in the vtable callback.

#### 🔥 Post-testnet (mechanical fixes worth landing in this PR)

- **Crypto H3** — ✅ **DONE in iter 4** — `register_identity_with_funding`
  (new merged L2) calls `remove_asset_lock` on success. Hygiene
  contract restored.

- **Rust-quality #2** — `MnemonicResolverCoreSigner::Error` is
  `String`. Replace with a typed `enum
  MnemonicResolverSignerError { NullHandle, NotFound,
  BufferTooSmall, ResolverFailed(i32), InvalidUtf8,
  InvalidMnemonic, DerivationFailed(String), InvalidScalar }` so
  callers can discriminate "user has no mnemonic in Keychain"
  from "FFI buffer overflow".

- **Rust-quality #3** — `StateTransition::sign_with_signer` maps
  signer errors to `ProtocolError::Generic`. Should use a more
  specific variant (e.g. new `ProtocolError::ExternalSignerError`)
  so the recovery-id-mismatch case (invariant violation by a
  conformant signer) is distinguishable from a real signing
  failure.

- **Rust-quality #9** — Inconsistent `Send + Sync` bounds across
  `register_identity_with_funding_external_signer`,
  `register_identity_with_signer`, `funded_register_identity` in
  `rs-platform-wallet::registration.rs`. Spell `Send + Sync` on
  both signer generic params everywhere for forward-compat with
  future `spawn`-driven refactors.

- **Crypto H4** — `MnemonicResolverCoreSigner.network` field is
  decorative (derivation is path-driven, not network-driven). Add
  a debug assertion in the constructor that `network ==
  wallet.network()`. Free safety net for the FFI call site, which
  already pulls both values.

#### 🛠 Polish (worth a follow-up PR, not blocking)

- **Adversarial P1 #6** — `build.rs:117-132` uses
  `keys.drain(..).next()` on `AssetLockCreditKeys::Public(keys)`.
  Latent bug if a future change emits >1 credit output: the first
  path is silently used, signature mismatch on the others, cascade
  lands as a consensus rejection. Add `match keys.len() { 1 =>
  ..., n => return Err(...) }` boundary check.

- **Rust-quality #6** — `try_from_identity_with_signer_and_private_key`
  and `try_from_identity_with_signers` bodies are near-duplicates.
  Extract a shared helper for the per-key-witness signing logic
  (lines 47-76 / 104-133 are identical). Reduces drift.

- **Rust-quality #11** — `MnemonicResolver()` in
  `ManagedPlatformWallet.registerIdentityWithFunding` should accept
  an injectable `storage: WalletStorage = WalletStorage()` parameter
  for test parity with `prePersistIdentityKeysForRegistration`
  (which already does this).

#### 📌 Architectural / longer-term

- **Adversarial P0 #1** — Wallet-manager write lock is held across
  the entire signer-driven build path
  (`rs-platform-wallet::asset_lock/build.rs:66-102`). The signer
  calls back into Swift Keychain, which on iOS can block for tens
  of seconds (Face ID prompts). The lock blocks SPV sync, balance
  reads, persister flushes, top-up flows. Architectural fix:
  release the write lock before invoking the signer; pass derived
  material into the inner builder. **Defer until iter 1
  testnet-validation succeeds** — fixing it changes a load-bearing
  control-flow shape mid-test cycle.

- **Adversarial P1 #3** — Funding-account derivation index
  advances inside `peek_next_funding_address` before the asset-lock
  build can fail. If the signer errors (Keychain locked, user
  cancels biometric), the wallet leaks a derivation slot per
  attempt, drifting toward gap-limit (~20). Verify whether
  `next_address(_, false)` is idempotent for not-yet-used entries;
  if strictly-advancing, add transactional rollback.

- **Adversarial P1 #7** — `block_on_worker` at
  `rs-platform-wallet-ffi::runtime.rs:55` uses `.expect("tokio
  worker panicked")` on `JoinError`. Pre-existing, but the new
  resolver surface widens the panic-source space (user-driven
  Keychain failures, malformed mnemonic, etc.). Replace with
  explicit JoinError → `PlatformWalletFFIResult::err` mapping, or
  set `panic = "abort"` in the FFI crate's release profile so
  unwinding across `extern "C"` is impossible.

- **Crypto H1** — `ExtendedPrivKey` intermediates in
  `MnemonicResolverCoreSigner::sign_ecdsa` aren't `ZeroizeOnDrop`
  (dashcore upstream issue). 32-byte scalar sits on the stack
  briefly until naturally overwritten. Microsecond window;
  fixable upstream via a `ZeroizeOnDrop` impl on dashcore's
  `ExtendedPrivKey`.

- **Crypto H2** — `MnemonicResolverCoreSigner` lifetime is
  comment-managed via `usize`-stored pointer. Soundness relies on
  the FFI caller honoring the doc-comment contract. Type-system
  fix: `PhantomData<&'a MnemonicResolverHandle>` + borrowed
  constructor. Needs `Arc<MnemonicResolverHandle>` at the
  boundary for `Send + 'static` future capture. Document or
  implement.

#### 🧪 Recommended additional regression tests (from auditor)

1. Recovery-id corner case — hand-craft a fixed-seed test that
   forces each recid ∈ {0, 1, 2, 3} to be the matching id, pin
   byte-identical output across all four.
2. Path equality pin — build asset-lock with fake signer, recover
   credit-output script_pubkey's pubkey hash, ask same signer for
   `public_key(returned_path)`, hash it, assert
   `Hash160(pk) == script_pubkey_hash`.
3. Cleanup parity — assert
   `register_identity_with_funding_external_signer` removes the
   tracked lock on success (currently fails — see H3).
4. Concurrent registrations — two `registerIdentityWithFunding`
   flows on the same wallet, verify Keychain serialization +
   `wallet_manager` write-lock interaction.

---

### Iter 3 — SwiftData mirror + persister callback (was iter 2)

**Goal**: make tracked asset locks visible to SwiftUI via
`@Query`. Unblocks the progress bar (iter 3) and resume picker
(iter 5).

The FFI persister callback table at `persistence.rs:104` has
**no asset-lock callback** today. Rust's `manager.rs:85` queues
asset-lock changesets internally but the FFI bridge drops them.
Also: `persistence.rs:1994` hardcodes `unused_asset_locks:
BTreeMap::new()` on wallet load, so even out-of-band persistence
would not rehydrate on launch.

**Steps**:

1. **FFI: add `on_persist_asset_locks_fn`** to
   `PlatformWalletPersistenceCallbacks` (`persistence.rs:64+`)
   carrying `(wallet_id, upserts: *const AssetLockEntryFFI,
   upserts_count, removed: *const [u8;36], removed_count)`.
2. **FFI: add `AssetLockEntryFFI`** `#[repr(C)]` mirror of
   `AssetLockEntry` (`changeset.rs:680-701`). Bincode-serialize
   the optional `AssetLockProof`.
3. **FFI: wire dispatcher** in `persistence.rs::store()` around
   the existing per-kind blocks.
4. **FFI: extend `WalletRestoreEntryFFI`** with
   `tracked_asset_locks: *const AssetLockEntryFFI` + `count`;
   populate `unused_asset_locks` at `persistence.rs:1994` from
   the restored rows so wallet-load rehydrates from SwiftData.
5. **SwiftData: add `PersistentAssetLock`** at
   `Sources/SwiftDashSDK/Persistence/Models/PersistentAssetLock.swift`:

   ```swift
   @Model
   final class PersistentAssetLock {
       #Index<PersistentAssetLock>([\.walletId])
       @Attribute(.unique) var outPointHex: String   // txid:vout
       var walletId: Data
       var transactionBytes: Data
       var fundingTypeRaw: Int
       var identityIndexRaw: Int32
       var amountDuffs: Int64
       var statusRaw: Int                // 0..3 = Built/Broadcast/IS/CL
       var proofBytes: Data?
       var createdAt: Date
       var updatedAt: Date
   }
   ```

6. **Register the model** in `DashModelContainer.modelTypes`
   (`DashModelContainer.swift:32`).
7. **Hook the persister handler**: extend
   `PlatformWalletPersistenceHandler` with an asset-lock case
   following the existing upsert pattern (e.g.
   `persistAddressBalances` at lines 88-113) — fetch by predicate,
   mutate-or-insert, defer save inside changeset bracket.
8. **Add a row to `StorageExplorerView`** for
   `PersistentAssetLock`, matching the pattern at
   `StorageExplorerView.swift:27-78`. **SwiftData-backed**, not
   FFI-backed — proves the persister round-trip works end-to-end
   before later iterations rely on it.

**Validation**: trigger an identity registration (the iter 1
flow), watch `StorageExplorerView`. A row should appear at
`Built`, advance through `Broadcast` → `InstantSendLocked`,
**then stay** (because iter 4's cleanup fix hasn't shipped yet).
Manual screen refresh OK; no progress bar yet.

**Known accumulation**: every successful registration leaves a
stale row at `InstantSendLocked` until iter 4 ships. Clutter in
StorageExplorer, harmless in normal user flow (the slot is
consumed via `PersistentIdentity`, so the row is unreachable
from `CreateIdentityView`).

---

### Iter 3 — Stage progress bar + RegistrationCoordinator

**Goal**: replace iter 1's generic spinner with a 5-step
stage-aware progress bar. Survive view dismissal.

**Stage source**: SwiftData `@Query` on `PersistentAssetLock`
(from iter 2) plus a Swift-side `ObservableObject` controller
for the bookend phases.

**Matching rule**: Swift cannot know the outpoint *before* the
FFI call returns. Match by `(walletId, identityIndex)` instead
— `TrackedAssetLock` already carries `identity_index`
(`tracked.rs:27`), and the UI enforces one in-flight registration
per `(walletId, identityIndex)` slot via `unusedIdentityIndices`.

**5-step UI**:

| Step | Driven by |
|---|---|
| 1. Preparing identity keys | controller `phase == .preparingKeys` |
| 2. Building asset-lock tx | `activeLock.statusRaw == 0` (Built) |
| 3. Broadcasting & waiting for instant-lock | `activeLock.statusRaw == 1` (Broadcast) |
| 4. Submitting to Platform | `activeLock.statusRaw == 2 or 3` AND controller still `.inFlight` |
| 5. Identity registered | controller `.completed` (controller-driven, **not** row deletion — iter 4 introduces the cleanup) |

**Failure semantics**: errors set controller to
`.failed(message:)`. From iter 4 onward the tracked lock row
stays on failure only (success removes it). Until iter 4 ships,
the row stays on both success and failure — UI is unaffected
because step 5 is controller-driven.

**Steps**:

1. **`IdentityRegistrationController`** (per repo convention
   from Swift arch review: `ObservableObject` + `@Published`,
   **NOT** `@Observable` which the codebase doesn't use):

   ```swift
   @MainActor
   final class IdentityRegistrationController: ObservableObject {
       enum Phase: Equatable {
           case idle
           case preparingKeys
           case inFlight
           case completed(identityId: Data)
           case failed(String)
       }
       @Published var phase: Phase = .idle

       func submit(/* walletId, identityIndex, funding, signer */) async { ... }
   }
   ```

2. **`RegistrationCoordinator`** singleton, hosted on
   **`PlatformWalletManager`** (per Swift arch review: AppState
   is a bootstrap host, PlatformWalletManager is the per-network
   operational hub and survives view dismissals). Keyed by
   `(walletId, identityIndex)`, stores active controllers,
   single-flights per slot.

3. **`CreateIdentityView`** binds to
   `walletManager.registrationCoordinator.startRegistration(
   walletId:, identityIndex:, funding:, …)` — reuses an existing
   controller for the slot, or creates a new one.

4. **`RegistrationProgressView`** reads
   `controller.phase` + `@Query` `activeLock.first?.statusRaw`
   to compute `currentStep`, renders 5 step rows with
   done/active/pending/failed states. `@Query` filtered by
   `walletId + identityIndexRaw == identityIndex`.

5. **"Pending registrations" row** on home / identities tab —
   lists active controllers from the coordinator, so dismissed
   flows remain reachable. Empty when the map is empty.

6. **Retention**: on `.completed`, keep the entry in the
   coordinator map briefly (~30s) before purging. On `.failed`,
   keep indefinitely until the user manually dismisses.

7. **Disable network toggle** while the coordinator map is
   non-empty (per adversarial review — switching testnet↔mainnet
   mid-flight tears down the SDK).

**Iter-3 call site**: this iteration's call into the Swift
wrapper still uses the iter-1 signature
`registerIdentityWithFunding(amountDuffs:identityIndex:identityPubkeys:signer:)`.
Iter 4 updates the wrapper to take `funding: IdentityFunding`;
the controller's call site updates mechanically at that point.

---

### Iter 4 — Rust refactor + cleanup fix + resume support ✅ **DONE**

**Outcome**: L1/L2 merge complete; auto IS→CL fallback wired at registration layer; H3 cleanup-on-success; multi-wallet Keychain isolation; typed signer errors (`MnemonicResolverSignerError` + `ProtocolError::ExternalSignerError`); consistent Send+Sync. All cargo tests green (122 + 78 + 3436). Testnet validation confirmed asset-lock build + broadcast + signing path works end-to-end, but exposed an SPV event-routing concern — see § SPV event-routing follow-up below.



**Goal**: collapse the three overlapping registration functions
into a two-layer factoring, fix the asset-lock leak, add
resume capability to the funding enum. After iter 4, the wallet-
balance path still works (re-validate iter 1's happy path) but
the function shape is what later iterations build on.

#### Target shape

| Layer | Function | Responsibility |
|---|---|---|
| **L1** | `register_identity_with_signer` (existing, signature updated) | Pure submit primitive. Takes `keys_map`, raw proof, raw key, signer. Builds placeholder Identity internally so callers don't repeat boilerplate. Calls `put_to_platform_and_wait_for_response`. No retry, no funding, no cleanup, no bookkeeping. |
| **L2** | `register_identity_with_funding` (renamed from `register_identity_with_funding_external_signer`) | Full orchestration. Takes `keys_map` + `IdentityFunding` + `identity_index` + signer. Pre-flight + funding dispatch + L1-submit + IS→CL fallback (re-submits via L1) + `IdentityManager` bookkeeping + `remove_asset_lock` cleanup. |
| — | `funded_register_identity` | **Deleted.** All useful behavior absorbed into L2. |

#### Steps

**Step 1 — Rust enum** (`types/funding.rs`):

- Keep `IdentityFunding` (`:27-42`); add a third variant
  `UseAssetLock { proof: AssetLockProof, private_key: PrivateKey }`
  mirroring the retired `IdentityFundingMethod::UseAssetLock`.
  No live consumer today but the variant + match arm cost ~3
  lines and future consumers (walletless paste, evo-tool
  import) get to wire up without a Rust schema change.
- Delete `IdentityFundingMethod` (`:47-68`).
- Delete `TopUpFundingMethod` (`:71-86`) **only if grep confirms
  no other consumers** — otherwise leave intact and migrate
  top-up separately.
- Update the file header comment (`:1-13`).

Final enum:

```rust
pub enum IdentityFunding {
    FromWalletBalance { amount_duffs: u64 },
    FromExistingAssetLock { out_point: OutPoint },
    UseAssetLock { proof: AssetLockProof, private_key: PrivateKey },
}
```

**Step 2 — L1 `register_identity_with_signer`** (`registration.rs:240`):

Change signature to take `keys_map: BTreeMap<u32, IdentityPublicKey>`
instead of pre-built `Identity`. Build the placeholder Identity
internally:

```rust
pub async fn register_identity_with_signer<S: Signer<...>>(
    &self,
    keys_map: BTreeMap<u32, IdentityPublicKey>,
    asset_lock_proof: AssetLockProof,
    asset_lock_private_key: &dashcore::PrivateKey,
    signer: &S,
    settings: Option<PutSettings>,
) -> Result<Identity, dash_sdk::Error> {
    let identity = Identity::V0(IdentityV0 {
        id: Identifier::default(),
        public_keys: keys_map,
        balance: 0,
        revision: 0,
    });
    identity
        .put_to_platform_and_wait_for_response(...)
        .await
}
```

Pre-flight checks (`keys_map` non-empty, key 0 = MASTER+AUTH)
stay at L2, not here — L1 is a primitive that trusts its
caller. Currently called only by `funded_register_identity`
(`:348`, `:369`), which is being deleted in Step 4. After the
merge L2 becomes the new caller.

**Step 3 — L2 `register_identity_with_funding`** (renamed from
`register_identity_with_funding_external_signer`, `:59`):

- Rename.
- Change `funding: IdentityFundingMethod` → `funding: IdentityFunding`.
- Keep pre-flight checks (`:70-98`).
- Replace the 2-arm funding match (`:101-116`) with three arms,
  **each capturing `tracked_out_point: Option<OutPoint>`**
  (currently dropped at `:105` as `_out_point` — the leak):
  - `FromWalletBalance { amount_duffs }` — existing body
    (`create_funded_asset_lock_proof`); keep returned
    `out_point` as `Some(out_point)`.
  - `UseAssetLock { proof, private_key }` — existing body, plus
    `Self::out_point_from_proof(&proof)` for tracked outpoint.
  - `FromExistingAssetLock { out_point }` — **new arm**, calls
    `self.asset_locks.resume_asset_lock(&out_point,
    Duration::from_secs(300))`; pass outpoint through as
    `Some(out_point)`.
- Remove inline Identity construction (`:119-124`) — now in L1.
- Submit via L1 (`self.register_identity_with_signer(keys_map,
  ...)`) instead of inline `put_to_platform_and_wait_for_response`.
  Both the initial call and the IS→CL fallback retry go through
  L1.
- IS→CL fallback wrapping (`:140-178`) stays.
- IdentityManager bookkeeping (`:181-220`) stays.
- **Add `remove_asset_lock` cleanup** after successful
  bookkeeping (post `:220`):
  ```rust
  if let Some(out_point) = tracked_out_point {
      self.asset_locks.remove_asset_lock(&out_point).await;
  }
  ```

Open tension: the cleanup-on-success path makes a `Registered`
variant on `AssetLockStatus` impossible (no row to hold it).
See Open Questions.

**Step 4 — Delete `funded_register_identity`** (`:311+`).

All behavior is now in L2:
- Funding dispatch ✅ (Step 3, with full 3-variant support)
- IS→CL fallback ✅ (Step 3 via L1)
- Cleanup ✅ (Step 3)
- IdentityManager bookkeeping (L2 always does this; the old
  function deliberately skipped it).

**Step 5 — FFI** (`identity_registration_funded_with_signer.rs`):

Extend the existing `platform_wallet_register_identity_with_funding_signer`
entry point. Change `amount_duffs: u64` parameter to a tagged
`IdentityFundingFFI` struct. Pattern: flat `#[repr(C)]` struct
with a `kind: u8` discriminator + per-variant fields (precedent:
`identity_registration_with_signer.rs:111-127`, NOT a C union):

```c
typedef struct IdentityFundingFFI {
    uint8_t kind;                // 0 / 1 / 2
    uint64_t amount_duffs;       // kind == 0
    uint8_t txid[32];            // kind == 1
    uint32_t vout;               // kind == 1
    const uint8_t *proof_bytes;  // kind == 2 (bincode-serialized)
    uintptr_t proof_len;         // kind == 2
    uint8_t private_key[32];     // kind == 2
} IdentityFundingFFI;
```

The FFI body dispatches on `kind`, constructs the matching
`IdentityFunding` variant, calls the new L2.

**Step 6 — Swift wrapper** (`ManagedPlatformWallet.swift:2356`):

Replace the current
`registerIdentityWithFunding(amountDuffs:identityIndex:identityPubkeys:signer:)`
with a funding-typed version:

```swift
public enum IdentityFunding {
    case fromWalletBalance(amountDuffs: UInt64)
    case fromExistingAssetLock(outPoint: OutPoint)
    case useAssetLock(proof: Data, privateKey: Data)
}

public func registerIdentityWithFunding(
    funding: IdentityFunding,
    identityIndex: UInt32,
    identityPubkeys: [IdentityPubkey],
    signer: KeychainSigner
) async throws -> (Identifier, ManagedIdentity)
```

Marshals to the tagged FFI struct. Update
`CreateIdentityView.submit()` from iter 1 to use the new
signature — call site change is mechanical (`amountDuffs: X` →
`funding: .fromWalletBalance(amountDuffs: X)`).

**Step 7 — Re-validate iter 1's happy path**

Build, run on testnet, register an identity. Confirm the
tracked-lock cleanup is now happening (no leak).

**Pre-existing bug, out of scope**: if `IdentityManager::add_identity`
at `:199` fails *after* successful Platform submission, the
function returns early via `?` — the identity is on Platform
but the wallet doesn't know it, AND the tracked asset lock stays
in storage. Predates our changes. Follow-up issue, not this PR.

---

### Iter 5 — "Fund from unused Asset Lock" picker + crash recovery

**Goal**: enable resuming a tracked asset lock when the
previous registration didn't complete. Validate crash recovery
end-to-end.

**Resume picker semantics**: an "unused" lock is one at status
`InstantSendLocked` or `ChainLocked` for which **no
`PersistentIdentity` exists** at the same `(walletId,
identityIndex)`. (Not `identityIndex == nil` — that field is
always set on a tracked lock.)

**Steps**:

1. **Resume-picker `@Query`** on `PersistentAssetLock` filtered
   by `walletId == selectedWalletId AND statusRaw >= 2 AND no
   matching PersistentIdentity at (walletId, identityIndexRaw)`.
   Compound query — may need a post-fetch filter for the
   anti-join.

2. **Update `CreateIdentityView`** so picking
   `.unusedAssetLock` and a specific tracked lock from the list
   wires through to `registrationCoordinator.startRegistration(
   walletId:, identityIndex: lock.identityIndexRaw, funding:
   .fromExistingAssetLock(outPoint: lock.outPointHex), …)`.

3. **Crash-recovery validation**: trigger a registration, kill
   the app between `Broadcast` and Platform submission. Re-launch.
   Verify the tracked lock appears in `StorageExplorerView` /
   `WalletMemoryExplorerView`. Open CreateIdentity → "Fund from
   unused Asset Lock" → submit → identity registers, tracked
   lock removed.

---

### Iter 6 — Explorer drill-downs

**Goal**: full explorer visibility for tracked asset locks
beyond the StorageExplorer row delivered in iter 2.

**Steps**:

1. **`StorageExplorerView` detail view** for
   `PersistentAssetLock`: list locks with `outPointHex`,
   `status`, `amountDuffs`, `identityIndexRaw`, `createdAt`,
   `updatedAt`. SwiftData-backed.

2. **`WalletMemoryExplorerView` drill-down**: expand the
   existing "N asset locks" count (`:368`) into a sub-section
   per wallet showing the live FFI snapshot
   (`trackedAssetLocks(for: walletId)`). Follow the
   `walletsSection` pattern at `:325`. FFI-backed (this view is
   for *in-memory* wallet state, not SwiftData).

---

### Iter 7 (optional) — Walletless paste flow

Out of scope unless explicitly requested. Lets the user paste a
raw asset-lock proof + private key + identity pubkeys and
register an identity with no wallet derivation. Uses the
`IdentityFunding::UseAssetLock` variant added in iter 4 — the
funding plumbing exists, only the UI is missing.

---

### SPV event-routing follow-up — RESOLVED (2026-05-13)

End-to-end Core-funded identity registration validated on testnet. Three causes, all landed:

- **Root cause**: in trusted-SDK mode the app set `masternodeSyncEnabled=false`, which disabled `dash-spv`'s `ChainLockManager` + `InstantSendManager`. The SPV client connected to masternode peers and received `CLSig`/`ISLock` P2P messages, but with no manager subscribed, `MessageDispatcher` dropped them — `LockNotifyHandler` never saw a single IS/CL event, `wait_for_proof` slept the full 300 s. Fix: hardcode `enable_masternodes = true` in `platform_wallet_manager_spv_start`; drop the FFI knob. Commit `885a1be3`.
- **Wallet record promotion**: upstream `WalletInterface` had no `process_chain_lock` until [rust-dashcore#756](https://github.com/dashpay/rust-dashcore/pull/756) merged, so records were stuck at `TransactionContext::InBlock(_)` after a chainlock. Bumped pin from `53130869` → `5297d61a` and added match arms for the new `WalletEvent::TransactionsChainlocked` variant in `core_bridge` + `balance_handler`. Commit `4184a425`.
- **Platform funding floor**: the v0 `200_000` duff minimum doesn't cover v1's per-key creation cost. With `defaultKeyCount = 3` the real floor is `221_500` duffs (`identity_create_base_cost + asset_lock_base * CREDITS_PER_DUFF + identity_key_in_creation_cost * 3`). Bumped `minIdentityFundingDuffs` to `221_500` and `defaultCoreFundingDuffs` to `250_000`. Commit `3d16a31a`.

## Open questions

- **Default funding amount**: 100,000 duffs (0.001 DASH)?
- **Asset-lock minimum constant**: name + value, verify <
  testnet faucet typical payout (per adversarial review W8).
- **Key count**: stick with `defaultKeyCount = 3` (1 master + 2
  high), or expose a picker?
- **`AssetLockStatus` extension** (iter 4 vs later): adding a
  `RegisteringOnPlatform` variant Rust-side would make step 4
  of the progress bar crisp (Rust signals when it moves from
  "waiting for IS-lock" to "submitting to Platform"). Without
  it, step 4 fires the instant IS-lock arrives, which may show
  "Submitting" prematurely if Rust internally retries. A
  `Registered` variant is **not possible** because the row is
  removed on cleanup. Defer the `RegisteringOnPlatform`
  decision until iter 4.

## Out of scope (explicitly)

- Mnemonic creation / import flow (already exists).
- SPV / BLAST sync changes (already exists).
- `top_up_identity_with_funding` migration to `IdentityFunding`
  (separate cleanup — only delete `TopUpFundingMethod` in iter 4
  if grep confirms no live consumers).
- Manual asset-lock proof construction beyond iter 7's optional
  paste UI.

## Architectural constraints (must follow)

From `packages/swift-sdk/CLAUDE.md`:

- Swift SDK does three things only: persist data, load data,
  bridge.
- No mnemonic / seed / derivation path construction in Swift.
- No iteration / gap-limit walks / policy loops in Swift.
- Decisions live Rust-side. If Rust doesn't expose a single
  call for what we need, add the helper in `platform-wallet`
  first.

The one allowed exception is iOS Keychain writes (Rust derives
the bytes, Swift persists). `prePersistIdentityKeysForRegistration`
is the precedent we follow.

From the Swift architectural review:

- Use `ObservableObject` + `@Published`, **not** `@Observable`
  (zero `@Observable` usage in this codebase).
- Host coordinators on `PlatformWalletManager`, **not**
  `AppState`. Operations are per-wallet hence per-network;
  `PlatformWalletManager` is the natural per-network hub and
  survives view dismissals.
- Register every new SwiftData `@Model` in
  `DashModelContainer.modelTypes`. Add `#Index` for
  query-heavy scalar fields.
- Use `walletId: Data` (denormalized scalar) for filtering
  predicates rather than relationships — the existing
  `PersistentTxo` pattern, more reliable than the
  `PersistentIdentity` relationship-based approach.
