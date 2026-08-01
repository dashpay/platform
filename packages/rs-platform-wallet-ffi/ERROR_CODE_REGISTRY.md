# FFI Error-Code Registry

Single source of truth for the integer values of
`PlatformWalletFFIResultCode` (`packages/rs-platform-wallet-ffi/src/error.rs`).

Every value in that enum is **public ABI**. `cbindgen` emits it into the
generated C header, and hosts compare against the integer — Swift
(`packages/swift-sdk/Sources/SwiftDashSDK/PlatformWallet/PlatformWalletResult.swift`)
mirrors it as a `RawRepresentable` enum, Kotlin
(`packages/kotlin-sdk/.../errors/DashSdkError.kt`) branches on it in
`fromPlatformWalletNative`. A shipped host binary that was compiled against
one numbering keeps using that numbering.

This file exists because several feature branches allocate into the same
integer range in parallel. A duplicate discriminant in two branches does **not**
produce a textual merge conflict — the second merge silently misclassifies
errors on every host — so allocations have to be reconciled here, in one place,
rather than in each branch's diff.

## Rules

1. **Claim the next free integer** from the table below — the first value not
   listed as merged, proposed, or reserved. Do not reuse a gap unless this file
   marks it free.
2. **Record the claim in this file in the same PR** that adds the variant. A PR
   that adds a code without a row here is incomplete.
3. **Never renumber a code after it has shipped in a release.** Deprecate
   instead: leave the row, mark it deprecated, and allocate a new integer. Codes
   that are still only proposed (unmerged) may be renumbered to resolve a
   collision; codes on `v4.2-dev` may not.
4. **Do not reuse a retired integer.** Mark it reserved and move on.
5. **Update the mirrors in the same PR**: the Rust enum, the Swift
   `PlatformWalletResultCode` + its `init(result:)` switch, and — where the code
   deserves typed handling — the Kotlin `fromPlatformWalletNative` mapping and
   `DashSdkErrorTest`. Kotlin is allowed to be non-exhaustive: unmapped codes
   fall through to `PlatformWallet.Generic(code, …)`, which preserves the
   integer. Swift is exhaustive; an unmirrored code surfaces as
   `.errorUnknown` there and loses its identity.
6. **Blocks 98–99 are terminal sentinels** (`NotFound`, `ErrorUnknown`) and are
   not an allocation frontier. New codes go after the highest allocated value
   below them.

## Merged allocations (`v4.2-dev`)

These are shipped ABI. Do not renumber.

| Code | Name | Notes |
| ---: | --- | --- |
| 0 | `Success` | |
| 1 | `ErrorInvalidHandle` | |
| 2 | `ErrorInvalidParameter` | |
| 3 | `ErrorNullPointer` | |
| 4 | `ErrorSerialization` | |
| 5 | `ErrorDeserialization` | |
| 6 | `ErrorWalletOperation` | |
| 7 | `ErrorIdentityNotFound` | |
| 8 | `ErrorContactNotFound` | |
| 9 | `ErrorInvalidNetwork` | |
| 10 | `ErrorInvalidIdentifier` | |
| 11 | `ErrorMemoryAllocation` | |
| 12 | `ErrorUtf8Conversion` | |
| 13 | `ErrorArithmeticOverflow` | Reserved slot — declared, no in-tree producer; holds the number for the mapping arriving via #3549 |
| 14 | `ErrorNoSelectableInputs` | |
| 15 | `ErrorWalletAlreadyExists` | |
| 16 | `ErrorShieldedBroadcastFailed` | |
| 17 | `ErrorShieldedBroadcastUnconfirmed` | |
| 18 | `ErrorShieldedSpendUnconfirmed` | |
| 19 | `ErrorShieldedNoRecordedAnchor` | |
| 20 | `ErrorTransactionBroadcastUnconfirmed` | |
| 21 | `ErrorAddressNonceMismatch` | |
| 22 | `ErrorCoreInsufficientFunds` | |
| 23 | `ErrorAssetLockNotTracked` | |
| 24 | `ErrorAssetLockAlreadyConsumed` | |
| 25 | `ErrorAssetLockFundingMismatch` | |
| 26 | `ErrorTransactionBroadcastRejected` | Merged in `9302c62e8b`; took a number several open branches had been treating as free |
| 98 | `NotFound` | Sentinel — `Option` returned as an error |
| 99 | `ErrorUnknown` | Sentinel — unmapped/flattened errors |

**Next free integer: 34** — 27–33 are all claimed in the proposed table below.
(Before the 29/30 resolution landed this line disagreed with the table, which
still showed 30 as unallocated; 30 is now allocated to #4185 and the two agree.)

## Proposed allocations (open PRs)

Not yet ABI. Numbers here may still move; they move by agreement recorded in
this file.

| Code | Name | Owning PR | Status |
| ---: | --- | --- | --- |
| 27 | `ErrorStaleReservationToken` | #4185 | In review (also carried by #4256) |
| 28 | `ErrorReservationTokenConsumed` | #4185 | In review (also carried by #4256) |
| 29 | `ErrorAssetLockInsufficientFunds` | #4184 | In review — **keeps 29** (collision resolved) |
| 30 | `ErrorReservationWalletMismatch` | #4185 | In review — **moved 29 → 30** (collision resolved; #4256 must inherit) |
| 31 | `ErrorSigningKeyUnavailable` | #4183 | In review (also carried by #4204) |
| 32 | `ErrorTransactionBuild` | #4247 | In review (also carried by #4256) |
| 33 | `ErrorTransactionSigning` | #4256 | In review |

Open PRs that touch `rs-platform-wallet-ffi` but claim **no** new code: #4186,
#4191, #4194, #4195, #4240, #4251, #4258.

## Contested and pending

### 29 — RESOLVED: #4184 keeps 29; #4185 moved to 30

Both PR heads defined code 29. Resolution of record: **#4184 keeps
`29 = ErrorAssetLockInsufficientFunds`; #4185 moves `ErrorReservationWalletMismatch`
to 30.**

**This renumber has now landed on #4185's branch**, propagated through every
site: the Rust enum discriminant and its three rustdoc cross-references
(`rs-platform-wallet-ffi/src/error.rs`), the two `signed_payment.rs` doc
references, the JNI rustdoc (`rs-unified-sdk-jni/src/wallet_manager.rs`), Swift
`PlatformWalletResultCode`'s raw value + doc
(`PlatformWalletResult.swift`), and Kotlin's `fromPlatformWalletNative` branch,
class KDoc, code-98 comment (`DashSdkError.kt`), `WalletManagerNative.kt` KDoc,
and the `DashSdkErrorTest` offset assertion.

Both Swift `switch`es are symbolic — `init(ffi:)` matches cbindgen-generated
`PLATFORM_WALLET_FFI_RESULT_CODE_*` constants, so only the enum's raw value
carried the number.

**Still outstanding:** #4256 is stacked downstream and its head still carries the
pre-renumber `29`; it must adopt 30 when it rebases. #4196 likewise (see below).

### 30 — allocated to #4185; the old "consent code" reservation was stale

`ErrorAssetLockCrossDomainConsentRequired` is named as the holder of 30 in
in-tree comments on #4183, #4204, and #4247/#4256's numbering rationale. It is
**not defined anywhere** — #4184, the PR that would have introduced it, does not
contain it after a re-scope.

Verified 2026-08-01 by reading `packages/rs-platform-wallet-ffi/src/error.rs` at
the head of **every one of the 62 open PRs**: no PR anywhere defines a code 30.
30 was therefore genuinely free, and #4185 has taken it. The stale "reserved for
the consent code" comments should be dropped by whichever PR touches them next.

### 27 / 28 — #3968 and #3954 collide with #4185's reservation trio

Found by the same 2026-08-01 sweep; these were missing from the tables above.

- **#3968** (`5931df745a`) numbers `ErrorPersisterTransient = 26`,
  `ErrorPersisterFatal = 27`, `ErrorTransactionBroadcastRejected = 28`. It
  branched before `26 = ErrorTransactionBroadcastRejected` merged, so it both
  contradicts merged ABI at 26 **and** collides with #4185 at 27 and 28.
- **#3954** (`93d0bd49b7`) numbers `ErrorShutdownIncomplete = 27`, colliding with
  #4185's `ErrorStaleReservationToken = 27`.
- **#4259** (`4270d827c2`) carries `ErrorSigningKeyUnavailable = 31` — the same
  number and name as #4183, i.e. inherited rather than a new allocation, like
  #4204.

Both #3968 and #3954 need a rebase onto current `v4.2-dev` and fresh integers
from the frontier below; #4185's 27/28 are the older claim and should stand.

### 26 — `ErrorStaleReservationToken` on #4196 collides with merged ABI

#4196 (stacked on #4185) branched before `26 = ErrorTransactionBroadcastRejected`
merged, and its head numbers the reservation trio **26 / 27 / 28**. Merging it as
it stands would give 26 two meanings and would contradict #4185's own 27 / 28 / 29
for the same three names. #4196 needs a rebase onto current `v4.2-dev` and must
adopt whatever numbering #4185 lands with. No new integers are needed for it.

### 31 vs 33 — two signing-related codes, deliberately distinct

Review on #4256 suggested mapping its signing failure onto 31. #4256 declined and
took 33, on the grounds that 31 (`ErrorSigningKeyUnavailable`, #4183) asserts a
specific contract — the signer holds no usable private key for a requested public
key, restored from a typed signer completion code — whereas #4256's
`BuilderError::SigningFailed` also covers unresolved derivation paths, sighash
failures, and malformed signature encodings. Both codes are currently allocated.
Maintainers may still choose to collapse them; that decision belongs to #4183 and
#4256 jointly and should be recorded here.

## Sibling FFI crates

`rs-sdk-ffi`'s `DashSDKErrorCode` (`packages/rs-sdk-ffi/src/error.rs`) is a
**separate** integer space (0–10, plus `InternalError = 99`) and is not contested
by any of the PRs above — none of them modify it. Do not assume a number means
the same thing in both enums.

## Survey provenance

Compiled 2026-08-01 against `v4.2-dev` at `ed4116b26c` and the following PR
heads: #3954 `93d0bd49b7`, #3968 `5931df745a`, #4183 `2cd948331b`, #4184
`a9e418af50`, #4185 `0b0d5c76d6` (post-renumber), #4259 `4270d827c2`, #4186
`6f7abbadc1`, #4191 `8acb0bd14c`, #4194 `9efc0b7e3a`, #4195 `4f2eb06d64`, #4196
`ea4f783490`, #4204 `7bc8a845c6`, #4240 `9328609a16`, #4247 `72c000dcfd`, #4251
`176f8ed3eb`, #4256 `d8943ccf10`, #4258 `5adfc40032`. Rows describing open PRs
reflect those heads and go stale as the PRs are updated; the merged table does
not.
