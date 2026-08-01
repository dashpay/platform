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

**Next free integer: 34** (see the proposed table; 27–33 are claimed).

## Proposed allocations (open PRs)

Not yet ABI. Numbers here may still move; they move by agreement recorded in
this file.

| Code | Name | Owning PR | Status |
| ---: | --- | --- | --- |
| 27 | `ErrorStaleReservationToken` | #4185 | In review (also carried by #4256) |
| 28 | `ErrorReservationTokenConsumed` | #4185 | In review (also carried by #4256) |
| 29 | `ErrorReservationWalletMismatch` | #4185 | **Collision** — see below |
| 29 | `ErrorAssetLockInsufficientFunds` | #4184 | **Collision** — see below |
| 30 | — | — | **Unallocated.** Reserved in sibling comments only; see below |
| 31 | `ErrorSigningKeyUnavailable` | #4183 | In review (also carried by #4204) |
| 32 | `ErrorTransactionBuild` | #4247 | In review (also carried by #4256) |
| 33 | `ErrorTransactionSigning` | #4256 | In review |

Open PRs that touch `rs-platform-wallet-ffi` but claim **no** new code: #4186,
#4191, #4194, #4195, #4240, #4251, #4258.

## Contested and pending

### 29 — `ErrorReservationWalletMismatch` (#4185) vs `ErrorAssetLockInsufficientFunds` (#4184)

Both PR heads define code 29. This is the known collision: review on #4185
directed that PR to keep #4184's `29 = ErrorAssetLockInsufficientFunds` and move
`ErrorReservationWalletMismatch` to 30. That renumber has not landed on #4185's
head, and #4256 (stacked downstream) carries the pre-renumber `29`.

Resolution of record: **#4184 keeps 29; #4185 moves to 30**, propagated through
the Rust enum, the FFI `From` mapping, Swift `PlatformWalletResult`, Kotlin
`DashSdkError` (+ `DashSdkErrorTest`), and the JNI rustdoc — plus #4256, which
inherits the value.

### 30 — reserved in comments for a variant that no longer exists

`ErrorAssetLockCrossDomainConsentRequired` is named as the holder of 30 in
in-tree comments on #4183, #4204, and #4247/#4256's numbering rationale. It is
**not defined anywhere** — #4184, the PR that would have introduced it, does not
contain it after a re-scope. 30 is therefore free, and is the slot the #4185
renumber above should take. The stale "reserved for the consent code" comments
should be dropped by whichever PR touches them next.

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
heads: #4183 `2cd948331b`, #4184 `a9e418af50`, #4185 `7d85953c2a`, #4186
`6f7abbadc1`, #4191 `8acb0bd14c`, #4194 `9efc0b7e3a`, #4195 `4f2eb06d64`, #4196
`ea4f783490`, #4204 `7bc8a845c6`, #4240 `9328609a16`, #4247 `72c000dcfd`, #4251
`176f8ed3eb`, #4256 `d8943ccf10`, #4258 `5adfc40032`. Rows describing open PRs
reflect those heads and go stale as the PRs are updated; the merged table does
not.
