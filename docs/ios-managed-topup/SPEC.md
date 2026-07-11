# Spec — Wire iOS managed identity top-up from asset lock (#4092)

Branch: `feat/ios-managed-identity-topup` (worktree `platform.worktrees/ios-topup`), based on `v4.1-dev` (`f7d7c8d348`).
Target: standalone PR against `v4.1-dev`.

## 1. Problem

Topping up an existing identity from a Core asset lock has **three fragmented paths** today
(see issue #4092 for the full table), and iOS has **no one-call managed path**:

- The DPP-SDK primitive `dash_sdk_identity_topup_with_instant_lock` is IS-only, takes a
  caller-built InstantSend proof + a **raw asset-lock private key**, and — though wired through the
  Swift SDK chain `topUpIdentity → identityTopUp` — has **zero app callers**. The example-app
  handler `executeIdentityTopUp` is a `notImplemented` stub.
- The managed **from-wallet-balance** export `platform_wallet_top_up_identity_with_funding_signer`
  exists only in the Kotlin-SDK PR #3999 (Android-only).
- The managed **from-existing-asset-lock** export exists only on the DIP-15 invitations branch.

The correct "from asset lock" abstraction is the **managed orchestrator**
`IdentityWallet::top_up_identity_with_funding`, which is **already on `v4.1-dev`**
(`packages/rs-platform-wallet/src/wallet/identity/network/registration.rs:388`) and does the full
lifecycle: resolve/build the asset lock, **IS→CL fallback**, retries, persist the new balance.

The only missing pieces on `v4.1-dev` are (a) the thin FFI export over that orchestrator, and
(b) the Swift SDK wrapper + example-app UI to call it.

## 2. Chosen approach

Extract #3999's already-written FFI export into `v4.1-dev` verbatim, then mirror the **existing,
proven** iOS registration-from-funding path for top-up. Nothing is reimplemented; the orchestrator,
the `MnemonicResolver` core-signer plumbing, and the cbindgen→xcframework header flow all already
exist and are exercised by `registerIdentityWithFunding` on iOS today.

### 2.1 Rust FFI exports (canonical, owned by this PR)

Two managed exports, so top-up has the same **register + resume** pair registration already has:

- **`platform_wallet_top_up_identity_with_funding_signer`** (`FromWalletBalance`) — the primary path,
  build a new lock from wallet balance.
- **`platform_wallet_topup_identity_with_existing_asset_lock_signer`** (`FromExistingAssetLock`) — the
  crash-recovery / stuck-lock path, consume an already-tracked lock. Extracted **verbatim** from the
  DIP-15 branch (`feat/dip15-dashpay-invitations`, `identity_registration_funded_with_signer.rs:268`,
  where it is the invitation-reclaim primitive); it wraps
  `top_up_identity_with_funding(&identity_id, AssetLockFunding::FromExistingAssetLock { out_point },
  &asset_lock_signer, None)` and returns the new balance via `out_new_balance`. Signature:
  `(wallet_handle, out_point: *const OutPointFFI, identity_id: *const [u8;32],
  core_signer_handle: *mut MnemonicResolverHandle, out_new_balance: *mut u64)`. Place it in
  `identity_registration_funded_with_signer.rs` **exactly where DIP-15 has it** (next to the
  registration resume twin `platform_wallet_resume_identity_with_existing_asset_lock_signer`, whose
  `OutPointFFI` import + marshalling it reuses) — so #4041's later rebase dedups cleanly too.

#### FromWalletBalance export

Add `platform_wallet_top_up_identity_with_funding_signer` to
`packages/rs-platform-wallet-ffi/src/identity_top_up.rs` — **exactly where #3999 places it**, next to
the existing `platform_wallet_top_up_from_addresses_with_signer`. Take the **whole `7a1d04792f`
version of the file** so the merge is byte-exact (the sibling `top_up_from_addresses_with_signer`
function is already identical between #3999 and `v4.1-dev` HEAD — zero diff — so this is a clean
wholesale adoption). The delta is: a module-doc-header rewrite, **one merged import line** (adding
`MnemonicResolverCoreSigner, MnemonicResolverHandle` to the existing
`rs_sdk_ffi::{SignerHandle, VTableSigner}`) **plus one new** `use platform_wallet::AssetLockFunding;`,
and the ~100-line function (`Identifier` is already imported). All dependencies already compile on
`v4.1-dev` — the registration twin `identity_registration_funded_with_signer.rs` uses the identical set.

Signature (verbatim from #3999):

```rust
pub unsafe extern "C" fn platform_wallet_top_up_identity_with_funding_signer(
    wallet_handle: Handle,
    identity_id: *const [u8; 32],
    amount_duffs: u64,
    account_index: u32,
    core_signer_handle: *mut MnemonicResolverHandle,
    out_new_balance: *mut u64,
) -> PlatformWalletFFIResult
```

Body: guard the pointers + `amount_duffs != 0`; round-trip `core_signer_handle` through `usize`
(so the spawned future's capture is `Send`); `PLATFORM_WALLET_STORAGE.with_item(...)` →
`block_on_worker(async { identity_wallet.top_up_identity_with_funding(&identity_id,
AssetLockFunding::FromWalletBalance { amount_duffs, account_index }, &asset_lock_signer, None).await })`;
write `*out_new_balance`. Note vs. registration: **no** identity-key signer and **no** pubkeys array —
the `IdentityTopUp` transition is signed entirely by the asset lock's Core-side key, so only
`core_signer_handle` is needed.

Add the `MIN_TOP_UP_DUFFS` guard (§2.5) to this export's parameter checks.

#### FromExistingAssetLock export

Append `platform_wallet_topup_identity_with_existing_asset_lock_signer` (body above) to
`identity_registration_funded_with_signer.rs`, taking the DIP-15 function body. Its symbols are present
on v4.1-dev **except `Identifier`** — the registration resume twin never takes an `identity_id` input,
so the file has no `Identifier` import. **Add `use dpp::prelude::Identifier;`** or the verbatim body
fails `error[E0433]`. Everything else (`OutPointFFI`, `AssetLockFunding`, `MnemonicResolverCoreSigner`,
`MnemonicResolverHandle`, `PLATFORM_WALLET_STORAGE`, `block_on_worker`, `check_ptr!`,
`unwrap_option_or_return!`, `dashcore::{OutPoint, Txid}`, `Handle`, `PlatformWalletFFIResult`) is
already imported. It has no amount parameter (the lock's value is fixed), so the §2.5 floor does not
apply to it.

**Rewrite the doc header.** DIP-15's header describes the invitation-voucher/OP_RETURN-burn reclaim
use case, which is misleading on generic v4.1-dev top-up code. Replace it with a self-contained
description of the stuck-lock / crash-recovery top-up (per the timeless-comments rule — no voucher,
no invitation narrative).

Header auto-surfaces for both: `platform-wallet-ffi` is in `build_ios.sh`'s `INCLUDED_CRATES`; cbindgen
regenerates the header at build time (not committed) and the umbrella `DashSDKFFI.h` picks it up.
**No manual C-header work.**

### 2.2 Swift SDK wrappers

Add two methods to `packages/swift-sdk/Sources/SwiftDashSDK/PlatformWallet/ManagedPlatformWallet.swift`
— the register/resume pair for top-up, mirroring `registerIdentityWithFunding` (`:3549`) +
`resumeIdentityWithAssetLock` (`:3656`) but **simpler** (no `KeychainSigner`, no pubkeys, return a
balance instead of a `ManagedIdentity`):

```swift
public func topUpIdentityWithFunding(          // FromWalletBalance
    identityId: Data,          // 32 bytes
    amountDuffs: UInt64,
    accountIndex: UInt32
) async throws -> UInt64       // new credit balance

public func resumeTopUpWithAssetLock(          // FromExistingAssetLock (crash recovery)
    identityId: Data,          // 32 bytes
    outPoint: Data             // 36 bytes: 32-byte txid + u32 vout
) async throws -> UInt64       // new credit balance
```

`resumeTopUpWithAssetLock` mirrors `resumeIdentityWithAssetLock` (`:3656`) exactly — same `OutPointFFI`
marshalling (`:3705`: `var outPoint = OutPointFFI(txid: txidTuple, vout: ...)`), same
`withExtendedLifetime(coreSigner)` + `Task.detached`, calling the new
`platform_wallet_topup_identity_with_existing_asset_lock_signer` and returning `outNewBalance`.

`topUpIdentityWithFunding` implementation, mirroring the registration scaffolding:
- `let coreSigner = MnemonicResolver()` (default `WalletStorage()`), as registration does at `:3570`.
- Marshal `identityId` (32 bytes) into a pointer to a 32-byte tuple for the `*const [u8;32]` param
  **by copying the sibling `topUpFromAddresses` wrapper in the same file**
  (`ManagedPlatformWallet.swift:502-551`): build a zeroed 32-byte tuple, fill from
  `identityId.prefix(32)`, pass via `withUnsafePointer(to: &idTuple) { idPtr in ... }`. This is the
  exact `*const [u8;32]` **input** precedent (sharper than registration's `*mut` out-param).
- `var outNewBalance: UInt64 = 0`.
- Run the blocking FFI call in `Task.detached(priority: .userInitiated)` wrapped in
  `withExtendedLifetime(coreSigner)` (the registration doc at `:3586` warns `_ = coreSigner` is unsafe
  under `-O`) — the `MnemonicResolver` ctx is `passUnretained`, so it MUST outlive the call.
- `try result.check()`; return `outNewBalance`.

### 2.3 Example-app UI — implement `executeIdentityTopUp`

Wire the stub at `TransitionDetailView.swift:611` following the managed pattern of
`executeIdentityUpdate` (`:644`):
1. Resolve `ownerIdentity` from `selectedIdentityId`; resolve its wallet via
   `ownerIdentity.wallet?.walletId` → `walletManager.wallet(for: walletId)`.
2. Read the funding **amount** (and optionally **accountIndex**) from `formInputs`.
3. `let newBalance = try await wallet.topUpIdentityWithFunding(identityId:amountDuffs:accountIndex:)`.
4. Update the local `PersistentIdentity` balance + `modelContext.save()` (as
   `executeIdentityCreditTransfer` does at `:855`), return a `[String: Any]` result dict.

Catalog change (`StateTransitionDefinitions.swift:49`): replace the single `assetLockProof` textarea
input (which the stub never read) with an **amount** field and an optional **`accountIndex`** field
(default `"0"`). The identity picker is already provided by `needsIdentitySelection`.

**Amount is Core-side duffs, NOT platform credits.** Do *not* mirror `identityCreditTransfer`'s
`"Amount (credits)"` field (`StateTransitionDefinitions.swift:100`) — that would mislabel the unit by
the credit-per-duff scale. Mirror the funding-amount UX of `CreateIdentityView` (DASH-denominated text
field with `duffsPerDash` conversion), or at minimum label the field **"Amount (duffs)"**, and feed it
into `amount_duffs`. Enforce the minimum-amount floor (§2.5) before submit — the same shape as
`CreateIdentityView`'s `currentMinFundingDuffs` gate.

Account selection is a plain numeric field defaulting to 0 (minimal, correct for the QA app). The
richer balance-validated BIP44-funding-account `Picker` from `CreateIdentityView` (`:617-636`) is
**not** copied — it's purely additive Swift-side work later (the FFI takes a raw `account_index: u32`
regardless), so deferring forces no rework. See §4.

**Resume UI (crash-recovery path).** Add a minimal explicit entry to invoke `resumeTopUpWithAssetLock`
— an outpoint (txid hex + vout) input that recovers a stuck lock into the selected identity, styled
like the existing explicit-input QA flows (the two-step `TopUpIdentityFromAddressesView` already takes
raw hex inputs). Add a **confirmation step** ("top up identity X with lock Y?") before invoking —
resume directs a tracked lock at whatever identity is selected, and while cross-wallet consumption is
structurally impossible (the lock must be tracked by *this* wallet), a stray lock could still land on
the wrong *self-owned* identity, which is not undoable. The elaborate auto-detect-tracked-lock
coordinator that registration wraps around `resumeIdentityWithAssetLock` is **not** copied — the SDK
wrapper + explicit-outpoint UI is enough to exercise and prove funds recovery in the QA app;
auto-detection is a follow-up (§7).

**Already-consumed error classification.** A double-resume (lock already burned on Platform) surfaces
as an opaque `PlatformWalletError::Sdk(...)` consensus rejection, not a friendly message — the same
class the DIP-15 reclaim had to special-case with an `isAlreadyConsumed("already completely used")`
classifier. The resume UI should detect and message this ("asset lock already consumed") rather than
dumping the raw SDK error. (A typed FFI error code is a follow-up; a narrow string-match classifier
matches the existing DIP-15 precedent.)

### 2.4 Decision — RETIRE `dash_sdk_identity_topup_with_instant_lock`

**Decided: RETIRE.** The managed path supersedes it and the app has zero callers, so remove the
raw-private-key surface rather than keep it. Blast radius (verified — fully self-contained, no
Kotlin/Android/Java/rs-sdk callers):

- **Rust:** delete `packages/rs-sdk-ffi/src/identity/topup.rs` entirely — it contains only
  `dash_sdk_identity_topup_with_instant_lock` (`:21`) and `..._and_wait` (`:94`). In
  `packages/rs-sdk-ffi/src/identity/mod.rs`, remove **both** the `mod topup;` declaration (`:16`)
  **and** the full 3-line re-export block (`:47-49`: `pub use topup::{ ... };`). **Keep** the rs-sdk
  `TopUpIdentity` trait (`top_up_identity::TopUpIdentity`) — it's only `use`d locally by topup.rs, not
  defined there, and the managed orchestrator uses it too. topup.rs's helper imports
  (`create_instant_asset_lock_proof`, `parse_private_key`, `convert_put_settings`) are heavily used
  elsewhere — deleting the file orphans nothing.
- **Swift:** delete `identityTopUp(...)` (`StateTransitionExtensions.swift:329`, incl. the C call at
  `:352`) and its sole convenience caller `topUpIdentity(...)` (`:2596`). No other callers.
- Header regenerates automatically (symbols vanish from the cbindgen output).

The `identityTopUp` **catalog key** stays — it now routes to the new managed `executeIdentityTopUp`.

### 2.5 Minimum top-up amount floor (funds-safety — requires a decision)

`amount_duffs == 0` is guarded in the export, but that is **not** the real floor. Platform enforces a
consensus-side minimum for `IdentityTopUp` via `IdentityTopUpTransition::calculate_min_required_fee`.
Under the **active fee version (v1)** — `calculate_min_required_fee_on_identity_top_up_transition: 1`
in `dpp_state_transition_versions/v3.rs:31`, used by `STATE_TRANSITION_VERSIONS_V3` (protocol v11+) —
that minimum is `identity_topup_base_cost` (500_000 credits = **500 duffs**) **plus**
`required_asset_lock_duff_balance_for_processing_start_for_identity_top_up` (**50_000 duffs**) =
**50_500 duffs** (`identity_topup_transition/state_transition_estimated_fee_validation.rs:39-50`;
`CREDITS_PER_DUFF = 1000`). Enforced on-chain by `tx_out_credit_value < required_balance` in
`rs-drive-abci/.../identity_top_up/transform_into_action/v0/mod.rs:59`. An amount **between dust and
50_500 duffs** does the dangerous thing: it **builds and broadcasts a real asset lock (spending Core
UTXOs), which Core accepts, then Platform rejects** — leaving the user's DASH committed in a tracked
asset lock that can never complete this top-up. This is the same footgun class as the DIP-15
invitations sub-floor defect (`MIN_INVITATION_DUFFS`). (The bare 50_000 value is only the fee-v0
minimum; the v0→v1 fee-calc bump adds the base cost.)

**Decided: UI gate + export guard (defense in depth).**
- **Export guard:** in `platform_wallet_top_up_identity_with_funding_signer`, replace the
  `amount_duffs == 0` check with `amount_duffs < MIN_TOP_UP_DUFFS → ErrorInvalidParameter`. Define
  `const MIN_TOP_UP_DUFFS: u64 = 50_500;` in the FFI (the active v1 fee minimum — base cost + asset-lock
  floor; see above). This guards **all** callers of the export, including Android. (The
  `FromExistingAssetLock` export has no amount param and is unaffected.)
- **UI gate:** in the example app, disable submit + show a "minimum …" hint until the entered amount
  ≥ the floor (mirroring `CreateIdentityView.currentMinFundingDuffs`), so no sub-floor lock is ever
  broadcast. The hard floor is the consensus 50_500 duffs.

### 2.6 Crash-recovery / stuck-lock resume for top-up (funds-safety — requires a decision)

The orchestrator's `AssetLockFunding` enum already has a `FromExistingAssetLock { out_point }` variant
for resuming a lock that confirmed on Core but never reached Platform (app killed / network drop
between broadcast and submit). Registration exposes this symmetrically —
`platform_wallet_resume_identity_with_existing_asset_lock_signer` → Swift `resumeIdentityWithAssetLock`
(`ManagedPlatformWallet.swift:3656`), explicitly the "crash recovery" path. This PR wires **only**
`FromWalletBalance` for top-up, so an interrupted top-up strands the confirmed Core lock with **no
in-app recovery**. (The DIP-15 invitations branch already carries a `FromExistingAssetLock` top-up
export, so the primitive exists.)

**Decided: include now.** Wire the `FromExistingAssetLock` top-up export
(`platform_wallet_topup_identity_with_existing_asset_lock_signer`, §2.1), the Swift
`resumeTopUpWithAssetLock` wrapper (§2.2), and a minimal explicit-outpoint resume UI (§2.3). This gives
top-up the same register+resume symmetry registration has and closes the stuck-lock funds-safety gap
in this PR. (Auto-detection of tracked stuck locks — registration's coordinator — remains a follow-up,
§7.)

## 3. Interface / data flow

```
Example-app  executeIdentityTopUp (TransitionDetailView.swift)
   selectedIdentityId + formInputs[amount] (+ accountIndex)
      │  resolve wallet via walletManager.wallet(for: ownerIdentity.wallet.walletId)
      ▼
Swift SDK    ManagedPlatformWallet.topUpIdentityWithFunding(identityId, amountDuffs, accountIndex)
      │  MnemonicResolver() core signer; Task.detached + withExtendedLifetime
      ▼
C ABI        platform_wallet_top_up_identity_with_funding_signer(wallet, id, amt, acct, coreSigner, &out)
      │  (cbindgen-generated header, auto-surfaced)
      ▼
Rust FFI     PLATFORM_WALLET_STORAGE.with_item + block_on_worker
      ▼
Orchestrator IdentityWallet::top_up_identity_with_funding(
                id, AssetLockFunding::FromWalletBalance{amount_duffs, account_index}, coreSigner, None)
      │  build asset lock from wallet balance → IS→CL fallback → submit IdentityTopUp → persist balance
      ▼  returns u64 new balance  →  out_new_balance  →  Swift UInt64  →  UI balance update
```

## 4. Alternatives considered / rejected

- **Wire the IS-only `dash_sdk_identity_topup_with_instant_lock` into the UI.** Rejected: pushes
  asset-lock creation, proof acquisition, and raw-private-key handling onto the UI; no IS→CL fallback;
  contradicts the managed security model (raw key never crosses FFI in the managed path).
- **Build a dedicated `TopUpFromCoreView` mirroring `CreateIdentityView`** (BIP44 funding-account
  picker + a registration-style coordinator that survives sheet dismissal). Rejected for this PR as
  over-scoped: the generic `TransitionDetailView` already handles async execution + result/error
  presentation, and a numeric account field is adequate for the QA app. Can be a follow-up if a
  production-grade UX is wanted.
- **Reimplement the orchestrator in the FFI layer.** Rejected — the orchestrator is already upstream;
  the export must be a thin delegate (acceptance criterion).
- **Add the export to `identity_registration_funded_with_signer.rs`.** Rejected in favor of
  `identity_top_up.rs` (where #3999 has it) to keep the eventual #3999 dedup a clean file-level match.

## 5. Failure modes

- **Insufficient Core UTXO balance** in the chosen account → orchestrator errors; surfaced via
  `PlatformWalletFFIResult` → Swift throw → UI error panel. (Same as registration.)
- **Sub-floor amount (dust < amount < 50_500 duffs)** → the asset lock is **accepted Core-side
  and broadcast**, then **rejected Platform-side**, leaving funds committed in a stuck tracked lock.
  This is the funds-safety gap addressed by the §2.5 floor — NOT a clean Core-side failure. (Contrary
  to registration's UI, which *does* gate on a computed `currentMinFundingDuffs` floor in
  `CreateIdentityView` — registration is not floor-free at the UI layer.)
- **Interrupted top-up (app killed between Core confirmation and Platform submit)** → recover via the
  §2.6 resume path (`resumeTopUpWithAssetLock`).
- **Resume of an untracked / consumed / foreign-wallet outpoint** → the orchestrator rejects cleanly
  *before* any broadcast: untracked → `"not tracked"`; already-consumed locally → `"already Consumed —
  nothing to resume"`; consumed on Platform → deterministic consensus rejection on resubmit. No
  double-spend / double-credit / cross-wallet consumption is possible. The consumed-on-Platform case
  is opaque (`Sdk(...)`) and should be classified in the resume UI (§2.3).
- **IS timeout** → orchestrator's IS→CL fallback handles it (the whole reason to use the managed path).
- **`MnemonicResolver` lifetime** → mandatory `withExtendedLifetime`; a dropped resolver mid-call is a
  use-after-free. Directly mirrored from registration.
- **Wrong/unset identity index** → orchestrator returns `IdentityIndexNotSet`; propagated as an error.
- **Local balance bookkeeping failure after Platform accepted** → orchestrator logs, does not fail the
  call (Platform already accepted). UI shows the returned balance.

## 6. Test / verification plan

Honest scoping: the rs-platform-wallet-ffi test suite is **entirely in-process** (contact requests,
handle lifecycle, null-pointer guards); **none** of it exercises the funded registration/top-up path,
because that needs a live Core+Platform network (asset locks, IS/CL). So "mirror the registration
coverage" realistically means:

1. **Rust guard unit tests** (hermetic, mirrorable from the existing null-pointer tests): for the
   `FromWalletBalance` export, assert `ErrorInvalidParameter` on `amount_duffs < MIN_TOP_UP_DUFFS`
   (including the old `== 0` and a sub-floor value like `49_999`), and the null-pointer error on null
   `identity_id` / `core_signer_handle` / `out_new_balance`. For the `FromExistingAssetLock` export,
   assert the null-pointer errors on null `out_point` / `identity_id` / `core_signer_handle` /
   `out_new_balance`. Cheap, real, regression-proof.
2. **Compile/link gates:** `cargo check -p platform-wallet-ffi`; then `./build_ios.sh` regenerates the
   header and the SwiftExampleApp builds against the new symbol (this is the real "does it surface"
   proof).
3. **Funded happy-path + IS→CL fallback: testnet UAT via SwiftExampleApp** (device/simulator). Top up a
   funded identity by N duffs; assert the credit balance rises by ~N (minus fee). Consistent with how
   the funded registration path and the DashPay funded flows are verified in this repo. Not a hermetic
   unit test — stated explicitly, not implied. **Also exercise a near/sub-floor amount** (e.g. just
   under the §2.5 floor) to confirm the UI floor blocks it *before* any asset lock is broadcast (so no
   funds are stranded) — this pins the §2.5 mitigation, not just the `== 0` guard.
4. **Resume-path UAT (testnet):** deliberately interrupt a `FromWalletBalance` top-up after the Core
   lock confirms but before Platform accepts (or reuse a lock the app already tracks), then recover it
   via `resumeTopUpWithAssetLock` and assert the balance rises. Proves the stuck-lock recovery.
5. **Retire regression:** `cargo check -p rs-sdk-ffi` and the SwiftExampleApp build must still pass
   after deleting `topup.rs` + the Swift `identityTopUp`/`topUpIdentity` wrappers — confirms nothing
   else referenced them.
6. **Regression:** the existing two-step address route
   (`topUpAddressFromAssetLock` → `topUpIdentityFromAddresses`) must still build and work — it is
   independent of the new exports.

## 7. Out of scope / follow-ups

- **#3999 dedup:** after this merges, #3999 rebases and **drops its copy** of
  `platform_wallet_top_up_identity_with_funding_signer` (`identity_top_up.rs` +110), keeping only its
  Android JNI/Kotlin wiring. This PR owns the canonical export. Note in the PR description.
- **#4041 dedup:** the DIP-15 invitations PR similarly drops its copy of
  `platform_wallet_topup_identity_with_existing_asset_lock_signer` once this lands. Note in both PRs.
- Dedicated production-grade `TopUpFromCoreView` with a balance-validated funding-account picker + a
  registration-style coordinator, **and auto-detection of tracked stuck locks** to offer resume
  proactively (this PR ships only the explicit-outpoint resume entry).

**Ops note:** the *local* `v4.1-dev` ref is stale (points at `9f9092cc91`, v4.0.0); this branch sits
exactly on `origin/v4.1-dev` (`f7d7c8d348`) with zero commits. Diff/verify against `origin/v4.1-dev`,
not the local ref, or you'll see an 80k-line phantom diff. All spec line numbers match worktree HEAD.

## 8. Acceptance criteria (from the issue)

- [ ] `platform_wallet_top_up_identity_with_funding_signer` (`FromWalletBalance`) **and**
  `platform_wallet_topup_identity_with_existing_asset_lock_signer` (`FromExistingAssetLock`) on
  `v4.1-dev`, both delegating to `top_up_identity_with_funding` (no reimplementation).
- [ ] iOS tops up an existing identity from a Core asset lock in one managed call (build lock from
  wallet balance, IS→CL fallback) via wired UI — `executeIdentityTopUp` no longer `notImplemented`.
- [ ] iOS can recover a stuck/tracked lock into an identity via `resumeTopUpWithAssetLock` (explicit
  outpoint UI).
- [ ] Sub-floor amounts are rejected before broadcast (export `MIN_TOP_UP_DUFFS` guard + UI gate, §2.5).
- [ ] `dash_sdk_identity_topup_with_instant_lock`(+`_and_wait`) and its Swift wrappers are removed
  (§2.4); rs-sdk-ffi + SwiftExampleApp still build.
- [ ] iOS + Android share the same FFI exports.
- [ ] Tests cover happy path + IS→CL fallback + resume + sub-floor rejection (per §6 scoping); two-step
  address route still works.
- [ ] #3999 and #4041 dedup follow-ups noted (§7).
