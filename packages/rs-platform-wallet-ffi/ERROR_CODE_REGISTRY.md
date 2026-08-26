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
integer range in parallel, and a duplicate integer across two branches never
produces a textual merge conflict. What happens after the merge depends on the
shape of the duplication, and only one of the two shapes is caught by a
compiler:

* **Two different variant names on the same integer.** The merged Rust enum has
  two variants with one discriminant, so `rustc` refuses it with
  `error[E0081]: discriminant value N assigned more than once`. Loud, but only
  *after* someone actually merges both branches into one tree — neither
  branch's own CI can see it, because neither branch contains both variants.
  This is how the code-32 collision below was finally caught.
* **The same meaning moving to a different integer**, or a host mirror left
  un-updated. Nothing fails to compile. A shipped host binary keeps the
  numbering it was built against, so it silently reads the new integer as
  whatever the old one meant — or, for an unmirrored code, loses the identity
  entirely (`.errorUnknown` in Swift). This is the failure this file mainly
  exists to prevent, and nothing in either branch's diff shows it.

So allocations have to be reconciled here, in one place, rather than in each
branch's diff.

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
5. **Update the mirrors in the same PR.** Swift needs **three** edits, not one,
   and they fail in different ways:
   1. `PlatformWalletResultCode` — the raw case.
   2. `PlatformWalletResultCode.init(ffi:)` — the arm mapping the generated C
      constant. This switch has a `default:` that yields `.errorUnknown`, so
      omitting the arm compiles fine and silently loses the code's identity
      *before* any typed handling sees it.
   3. `PlatformWalletError` — the typed case, **and** its `init(result:)` arm.
      That switch is exhaustive with no `default:`, so adding a raw case in (1)
      without the matching arm here makes it non-exhaustive and the Swift
      package stops compiling.

   Then, where the code deserves typed handling, the Kotlin
   `fromPlatformWalletNative` mapping and `DashSdkErrorTest`. Kotlin is allowed
   to be non-exhaustive: unmapped codes fall through to
   `PlatformWallet.Generic(code, …)`, which preserves the integer.
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
| 13 | `ErrorArithmeticOverflow` | Produced in-tree by `shielded_send.rs` (the shielded-send amount/fee overflow guard). Its rustdoc was corrected by #4360 (`e0b8baa850`), which documents the `PlatformWalletError::InputSumOverflow` mapping — the stale no-producer comment this row used to track is gone |
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
| 27 | `ErrorShutdownIncomplete` | Merged 2026-08-02 by **#4268** (`429667e723`). A quiesce/drain barrier missed its budget. **Took the number #4185 had held since before this file existed** — see the collision history below |
| 31 | `ErrorSigningKeyUnavailable` | Merged 2026-08-04 by **#4183** (merge commit `189a3abb1c`, stacked on #4191). The signer holds no usable private key for a requested public key. Landed complete in that one commit: the Rust C-facing discriminant, Swift's `errorSigningKeyUnavailable = 31` raw case *and* its `init(ffi:)` arm *and* the typed `PlatformWalletError` case with its `init(result:)` arm, and Kotlin's `31 -> PlatformWallet.SigningKeyUnavailable`. Rule 3 now protects it — see the 31-vs-33 note below |
| 34 | `ErrorStaleReservationToken` | Merged 2026-08-06 by **#4308** (`438153da39`) — the reservation trio landed with the split build/broadcast surface (successor of fork-era #4185's claim) |
| 35 | `ErrorReservationTokenConsumed` | Merged 2026-08-06 by **#4308** (`438153da39`) |
| 36 | `ErrorReservationWalletMismatch` | Merged 2026-08-06 by **#4308** (`438153da39`) |
| 37 | `ErrorDocumentNotForSale` | Merged 2026-08-09 by **#4348** (`6373e00f0c`). **Took the number the fork-era shielded-invite claim held** — see the proposed table's 37 note |
| 38 | `ErrorDocumentPriceChanged` | Merged 2026-08-09 by **#4348** (`6373e00f0c`) |
| 39 | `ErrorInsufficientIdentityCredits` | Merged 2026-08-09 by **#4348** (`6373e00f0c`) |
| 40 | `ErrorContestedNameNotTradable` | Merged 2026-08-09 by **#4348** (`6373e00f0c`) |
| 41 | `ErrorShieldedInsufficientBalance` | Merged 2026-08-11 by **#4360** (`e0b8baa850`) |
| 42 | `ErrorMasternodeWithdrawalUnconfirmed` | Merged 2026-08-22 by **#4451** (masternode-credit claiming). **Took the number active #4356 had claimed** for `ErrorAssetLockInputConflict` — see the proposed table's 42 note; #4356 renumbers via the frontier |
| 46 | `ErrorMasternodeListUnavailable` | Merged 2026-08-24 by **#4465** (`8dd964277`). Initially minted as 43 (already held by active #4313's `ErrorShieldedInviteAlreadyClaimed` across all three layers) — collision flagged in review and renumbered to the then-frontier same day, Rust and Swift together |
| 98 | `NotFound` | Sentinel — `Option` returned as an error |
| 99 | `ErrorUnknown` | Sentinel — unmapped/flattened errors |

**Next allocatable integer: 48** — 27–47 are all claimed (27, 31, 34–42, 46
merged; 29 proposed by active #4361; 43–45
proposed by active #4313 at head `0302b188ab`; 47 proposed by active #4356
(renumbered from 42 — see its row below); 28, 30,
32 and 33 reserved). **28, 30,
32 and 33 are RESERVED, not free**: 28 and 30 were vacated when the
reservation trio moved to 34–36; 32 and 33 lapsed when their in-repo owners
(#4310, #4311) closed without merging. All four are deliberately left
unclaimed rather than back-filled, so no number is reused within a single
review cycle. Rule 1's "do not reuse a gap unless this file marks it free"
applies — this file does **not** mark any of them free, so the frontier is
the only allocation source and a new code takes 48. (42 is a cautionary tale:
merged #4451 minted it while active #4356 held the claim — merged ABI wins,
the open PR renumbers. 46's near-miss went the other way: caught in review,
renumbered before merge.)

## Proposed allocations (open PRs)

Not yet ABI. Numbers here may still move; they move by agreement recorded in
this file.

**Ownership migrated 2026-08-11, then largely settled the same week.** The
fork-era PRs that originally held these allocations (#4184, #4185, #4204,
plus #4247 and #4256) were closed and recreated in-repository per repo
policy. Of the successors: #4308 **merged** (the trio, 34–36 — now in the
merged table); three others — #4316, #4310 and #4311 — **closed without
merging** (32 and 33 lapse to RESERVED; 29 is carried live by #4361, which
holds the typed shortfall today); and #4313 (the shielded-invite
claim) lost 37 to merged #4348, revived, and now holds 43–45 from the
frontier at head `0302b188ab` — see its three rows below.
Fork-era numbers remain in the collision history, which is immutable record.

| Code | Name | Owning PR | Status |
| ---: | --- | --- | --- |
| 28 | *(reserved — vacated)* | — | Vacated by #4185/#4256 on 2026-08-02; RESERVED, not reissuable — the next-free frontier is the only allocation source |
| 29 | `ErrorAssetLockInsufficientFunds` | #4361 | In review — **keeps 29**. Lineage: fork-era #4184 → #4316 (closed unmerged) → carried live by #4361's typed asset-lock shortfall (`ErrorAssetLockInsufficientFunds = 29` at its head). **Rule 5 is satisfied as of `15aa2caea1`, and was not before it.** Kotlin has mirrored 29 since the branch's binding commit `a711c55eca` (`fromPlatformWalletNative` plus a `DashSdkErrorTest` pin); Swift carried none of rule 5's three edits until `15aa2caea1`, so 29 fell to `init(ffi:)`'s `default:` and lost its identity as `.errorUnknown` — the rule-5 failure this file exists to catch, one host typed and the other blind. That commit adds the raw case, the `init(ffi:)` arm, and the typed `PlatformWalletError` case with its `init(result:)` arm, plus an `ErrorHandlingTests` case pinning the raw value. Its Rust sibling `2eac8a897e` is what lets the code reach either host on the exact-amount funding path, which flattened the variant to `ErrorWalletOperation` (6) in `map_asset_lock_funding_result` before the blanket `From` arm could run |
| 47 | `ErrorAssetLockInputConflict` | #4356 | Proposed — **47 is reserved for this active PR, but the three-layer renumber is still PENDING.** Merged #4451 took 42 for `ErrorMasternodeWithdrawalUnconfirmed` on 2026-08-22, and merged ABI wins. At the cited #4356 head `7d9be71a08`, Rust still defines and tests `ErrorAssetLockInputConflict = 42`, Swift still declares `errorAssetLockInputConflict = 42`, and Kotlin still maps and tests native 42 — #4356 must move all three layers and their tests together to 47 before it can merge. Rule 1 makes 47 unavailable to any other contributor while #4356 is active |
| 30 | *(reserved — vacated)* | — | Vacated by #4185/#4256 on 2026-08-02; RESERVED, not reissuable — the next-free frontier is the only allocation source |
| 32 | *(reserved — lapsed)* | — | Owner #4310 (successor of fork-era #4247) closed without merging; RESERVED, not reissuable |
| 33 | *(reserved — lapsed)* | — | Owner #4311 (successor of fork-era #4256) closed without merging; RESERVED, not reissuable |
| 43 | `ErrorShieldedInviteAlreadyClaimed` | #4313 | In review — **ACTIVE; the former "on hold — holds no number" row is obsolete.** The branch revived and renumbered to the frontier exactly as that row prescribed. Lineage: fork-era #4204's 32 → 37 move, then 37 **taken by merged #4348** (`ErrorDocumentNotForSale = 37`, ABI since 2026-08-09), then 37 → 43 on revival. `ErrorShieldedInviteAlreadyClaimed = 43` at head `0302b188ab`. **Rule 5 is satisfied at that head**: Swift carries all three edits — the raw case, the `init(ffi:)` arm, and the typed `PlatformWalletError.shieldedInviteAlreadyClaimed` case with its arm in `init(code:message:)` (which `init(result:)` delegates to) — plus `errorDescription`; Kotlin has the typed terminal `PlatformWallet.ShieldedInviteAlreadyClaimed`, the `43 ->` arm in `fromPlatformWalletNative`, and a `DashSdkErrorTest` pin on 43. Swift's 43 mirror predates `0302b188ab` on the branch; the raw-value test pin for 43 is Kotlin's (Swift's `ErrorHandlingTests` pins 44 and 45 only) |
| 44 | `ErrorShieldedScanBudgetExhausted` | #4313 | In review — claimed from the frontier; carries the #4306 scan-budget semantics (retryable — progress is checkpointed). **Rule 5 is satisfied as of `0302b188ab`, and was not before it.** At that commit's parent Kotlin already mirrored 44 (typed `ShieldedScanBudgetExhausted`, the `fromPlatformWalletNative` arm, a `DashSdkErrorTest` pin) while Swift carried none of rule 5's three edits, so 44 fell to `init(ffi:)`'s `default:` and lost its identity as `.errorUnknown` — one host typed, the other blind, the same failure shape as row 29's. `0302b188ab` adds the raw case, the `init(ffi:)` arm, the typed case with its `init(code:message:)` arm and `errorDescription`, and an `ErrorHandlingTests` pin of raw value 44 |
| 45 | `ErrorShieldedLifecycleBusy` | #4313 | In review — claimed from the frontier. A shielded lifecycle operation refused because teardown/clear holds the wallet (retryable — nothing consumed); the FFI remove path passes the refusal through as 45 instead of flattening it to `ErrorWalletOperation` (6). Same rule-5 history as 44: Kotlin mirrored 45 at the parent commit already; Swift's three edits and an `ErrorHandlingTests` pin of raw value 45 landed in `0302b188ab`. **Rule 5 is satisfied at that head** |

**Code 31 left this table on 2026-08-04.** `ErrorSigningKeyUnavailable` sat here
as #4183's proposal until #4183 merged (`189a3abb1c`); it is now in the merged
table above and rule 3 applies to it in full. It has since had company: the
reservation trio (34–36, #4308) and the 37–41 block (#4348, #4360) also
merged out of proposal — see the merged table, and take the current
frontier only from the frontier note above (46 as of 2026-08-19).

PRs that touch `rs-platform-wallet-ffi` but claim **no** new code, verified
2026-08-04 against each PR's file list and the `error.rs` at its head (a
fork-era snapshot; #4186, #4194 and #4195 have since been closed and
recreated in-repository, carrying the same no-code property, while #4243
remains open at its fork-era head):
`#3417`, `#3549`, `#3992`, `#4186`, `#4194`, `#4195`, `#4243`.

Five entries this list used to carry have been removed, each for a stated
reason, so they are not silently re-added:

| Was listed | Why it is gone |
| --- | --- |
| `#4240` | Its head touches no file under `rs-platform-wallet-ffi` at all — still true at `a167afe84c` |
| `#4251` | Was listed as touching no file under this crate; **merged into `v4.2-dev` on 2026-08-04** (`7afc8a8ff3`) having touched none, so nothing here changes |
| `#4258` | Merged into `v4.2-dev` on 2026-08-03 (`ce8233edb7`); it claimed no code, so the merged table is unchanged |
| `#4264` | Closed. Its `error.rs` change (mapping new wallet errors onto the existing `ErrorInvalidParameter`) is carried by `#4243`, which is still open and is listed above |
| `#4191` | **Merged into `v4.2-dev` on 2026-08-04** (`0e2282b586`). In this crate it only ever touched `dashpay.rs`; it claimed no code, so the merged table gained nothing from it — though #4183, which was stacked on it, did |

`#4186`, `#4194`, `#4195` and `#4243` are worth naming explicitly: each *does*
modify `error.rs`, but none of them adds a discriminant — `#4243`'s change maps
new wallet errors onto the **existing** `ErrorInvalidParameter`. Touching
`error.rs` is not the same as claiming an integer, and this list tracks the
latter.

**`#4277` is the merged precedent for that distinction.** It merged into
`v4.2-dev` on 2026-08-04 (`6704a41a85`) with a change to this crate's
`error.rs`, and it claimed no integer: it routes
`PlatformWalletError::TxMetadataPayloadTooLarge` onto the existing
`ErrorInvalidParameter`, with an in-line comment saying it does so deliberately
"so no new numeric code churns the Swift/Kotlin mirror enums". The merged table
is unchanged by it.

The **inherited-code table** has been reduced to one row and retained for
provenance:

| Code | Name | Carried by | Allocated to |
| ---: | --- | --- | --- |
| 31 | `ErrorSigningKeyUnavailable` | #4204, #4259 | #4183 |

`#4183` merged on 2026-08-04, so 31 is no longer an allocation anyone can inherit
— it is trunk. Every branch that has rebased onto current `v4.2-dev` carries it
from the base, which is not a claim and cannot be double-counted. Verified
2026-08-04 at the heads of #4184, #4185, #4186, #4194, #4195, #4196, #4204,
`#4240`, #4247, #4256 and #4259: all eleven show `ErrorSigningKeyUnavailable = 31`
inherited from the merged base.

PR `#4196` also claims no new integer: it adds a token-less
`PlatformWalletError::StaleReservation` variant and deliberately routes it
through the **existing** `ErrorStaleReservationToken`, so it allocates nothing
and only has to follow that code's number. As of 2026-08-04 (head `9909f77546`)
it is restacked onto #4185 and follows 34 (see below).

### Non-conforming allocations (withdraw and reissue)

These branches allocate below the frontier without a row here. They are listed
here rather than in the proposed table because their numbers cannot stand as
written — each row is a claim to be **withdrawn and reissued**, not an
allocation of record.

**#3968's branch state, re-read 2026-08-26 at its current head
`396977be4e7c`.** The stale-base half of its old defect is cured: the branch
merged `v4.2-dev` in on 2026-08-20 (merge commit `9d0dd5a49d`) and again on
2026-08-25 (`c86d237117`, `4dbf38f5da`), so `ErrorTransactionBroadcastRejected`
is back on its merged-ABI 26, `ErrorShutdownIncomplete = 27` comes in from the
base, and the rule-3 renumber (26 → 28) this table used to record is gone.
But the 2026-08-20 merge also renumbered the two persister codes to **42 and
43** — below the frontier, with no row here (rule 2), and onto numbers this
file records as taken. This file's earlier prediction stands
half-vindicated: rebasing cured what the base could cure, and the enum edit
that was always required was made — onto the wrong integers.

| Code | Name | Owning PR | Conflict |
| ---: | --- | --- | --- |
| 42 | `ErrorPersisterTransient` | #3968 | Contradicts **merged ABI** — 42 is #4451's `ErrorMasternodeWithdrawalUnconfirmed` (merged 2026-08-22). Not a paper conflict: since the 2026-08-25 base merges, #3968's **own tree** carries both variants — a hard E0081 in `error.rs` (`= 42` at both variants) and a duplicate raw value 42 in Swift's `PlatformWalletResultCode` — so the branch does not compile as-is |
| 43 | `ErrorPersisterFatal` | #3968 | Collides with **active #4313**, whose recorded claim is `ErrorShieldedInviteAlreadyClaimed = 43` (see its proposed row). The silent shape: nothing conflicts textually and neither tree carries both variants, so only this file shows it |

PR `#3954`'s `ErrorShutdownIncomplete = 27` used to sit in this table. It is
gone because that claim **won**: #3954 was closed and superseded by **#4268**,
which merged 27 into `v4.2-dev` on 2026-08-02. See the collision history below.

The 42 is the instructive half, because its timeline shows why "check the
table on the day you allocate" is not enough. When `9d0dd5a49d` minted
`ErrorPersisterTransient = 42` on 2026-08-20, 42 was not yet merged ABI — it
was **#4356's recorded claim** in this file's proposed table, so the mint
violated rule 1 against a *proposed* allocation. Two days later #4451 merged
42 as `ErrorMasternodeWithdrawalUnconfirmed` and it became unrenumberable ABI
(the same event that pushed #4356 to 47); three days after that, #3968's own
base merges imported the merged variant and turned the paper conflict into an
E0081 in its own tree. A registry row filed with the mint (rule 2) would have
been challenged on day one. Both persister codes must now take fresh integers
**from the frontier note above, which is the single canonical source; no
number is copied here because any copy goes stale the moment another PR
merges** (as the original "46+" copy in this paragraph did when #4465 shipped
46 — the frontier note reads 48 as of 2026-08-26, so a pair claimed today
takes 48 and 49, recording the claim there and here in the same PR). 26 and
27 need nothing: they are the merged base's own values, correctly inherited,
and rule 3 keeps them where they are.

## Contested and pending

### 32 — RESOLVED: #4204 moved to 37 (first collision this file actually caught)

Found 2026-08-03 while assembling the `v41int13` QA integration. #4204's head
commit `b6992a5dbc` — a review round, not the original feature work — added
`ErrorShieldedInviteAlreadyClaimed = 32` with **no row in this file**, in
direct violation of rule 2. 32 is allocated to `ErrorTransactionBuild` (#4247,
also carried by #4256).

Unlike every other entry in this section, this one was not a paper conflict:
merging #4204 into an integration that already carried
`ErrorTransactionBuild = 32` produced a hard
`error[E0081]: discriminant value 32 assigned more than once`. Resolution of
record: **#4204 moves 32 → 37**, the frontier. `ErrorTransactionBuild` keeps 32.

The numbering was the lesser half of the defect. The code was **unmirrored on
both hosts** — absent from Swift's `PlatformWalletResultCode` and from Kotlin's
`fromPlatformWalletNative`. Per rule 5 that means Swift rendered it
`.errorUnknown` (identity lost), while Kotlin fell through to `Generic(32, …)`
— and in any tree also carrying the competing #4247 mapping for 32, Kotlin
actively **misclassified** "shielded invite already claimed" as "transaction
build failed". `ErrorReservationWalletMismatch` was never the peer here: its
allocation history runs 26/28 → 30 → 36 and never passes through 32.

The three host outcomes therefore differ, and only the third is a
misclassification: Swift loses the code's identity as `.errorUnknown`, Kotlin
on #4204 alone preserves `Generic(32, …)`, and Kotlin misclassifies only where
the competing code-32 mapping is also present. That is the cross-host ABI
failure this file's preamble describes, and it landed on the shielded-invite
claim-recovery path (the error is raised from four sites in
`wallet/shielded/operations.rs`, three of them inside the recovery function).

Fixed on #4204 together with the renumber. Landed at head
`d78b940a03`: the typed Kotlin
`PlatformWallet.ShieldedInviteAlreadyClaimed` (terminal, `isRetryable = false`),
the Swift `PlatformWalletResultCode.errorShieldedInviteAlreadyClaimed = 37` raw
case with its `init(ffi:)` arm, and a `DashSdkErrorTest` assertion that pins 37
so a future move off the frontier fails the suite instead of the hosts.

The Swift half of that fix was **incomplete at `d78b940a03`, and is complete
now.** At that head `PlatformWalletError` had no `.shieldedInviteAlreadyClaimed`
case even though `init(result:)` switches exhaustively over
`PlatformWalletResultCode` with no `default:` — adding the raw case without the
matching arm makes that switch non-exhaustive, so the Swift package did not
build. **Closed since.** Verified 2026-08-04 at #4204's current head
`4efecd5b71`: the raw case, its `init(ffi:)` arm, the typed
`PlatformWalletError.shieldedInviteAlreadyClaimed` case and its `init(result:)`
arm are all present. Rule 5's Swift clause is satisfied. Kept here because the
sequence is the lesson — one of rule 5's three Swift edits landed a full review
round after the other two.

**Lesson for rule 2:** the violation entered on a *review-round* commit, well
after the PR's numbering had been reviewed and recorded as settled. Re-check
discriminants on every push that touches `error.rs`, not only at first review.

### 29 — RESOLVED: #4184 keeps 29 (#4185 moved away, twice)

Both PR heads defined code 29. Resolution of record: **#4184 keeps
`29 = ErrorAssetLockInsufficientFunds`; #4185 moves `ErrorReservationWalletMismatch`
to 30.**

**#4184's 29 is settled and has not moved.** #4185's third code moved to 30 to
clear it, and then — with the rest of the trio — to **36** when #4268 merged 27
(see the collision history above), which is why 30 is reserved rather than
free. Nothing about this section's resolution changed: 29 is #4184's.

Note that neither #4184 nor #4256 was ever blocked by CI on this. Both are
MERGEABLE with green checks, because two branches assigning the same
discriminant produce no textual conflict — the collision surfaces only as an
E0081 after a textual merge, or silently as a wrong error code on the host.
That is the whole reason this file exists.

**#4196 closed out 2026-08-03** — restacked onto the trio at 34/35/36 (see
"26 — RESOLVED" below); nothing remains outstanding from this collision.

### 30 — vacated, then RESERVED (not free)

`ErrorAssetLockCrossDomainConsentRequired` was named as the holder of 30 in
in-tree comments on #4183 and #4204, and in #4256's pre-renumber numbering
rationale. It is **not defined anywhere** — #4184, the PR that would have
introduced it, does not contain it after a re-scope. Those comments are all gone
now: grepping `packages` at `v4.2-dev` `97904ed2fc` on 2026-08-04 returns no
occurrence of the name, and neither does #4204's head `4efecd5b71`.

Verified 2026-08-01 by reading `packages/rs-platform-wallet-ffi/src/error.rs` at
the head of **every one of the 62 open PRs**. Stated precisely, because the
unqualified version of this sentence is false: **no PR unrelated to #4185
defines a code 30.** #4185 itself, and #4256 downstream of it, did define
`ErrorReservationWalletMismatch = 30` at their surveyed heads — that was the
allocation, not a competing claim. So nothing contested 30, #4185's claim stood,
and the stale consent-code reservation never conflicted with it.

PR #4185 then vacated 30 on 2026-08-02 when the trio moved to 34–36. Vacating
is not the same as freeing: **30 is now RESERVED and must not be reissued** (see
the collision history below for why). The stale "reserved for the consent code"
comments should be dropped by whichever PR touches them next.

Three branches have now done so:

* **#4256** — as of `8febac177c` its `ErrorTransactionSigning` rationale names
  #4268 as the owner of 27 and records where the trio went.
* **#4183** — its enum comment no longer describes 27–28 as reserved for the
  trio; on the 2026-08-03 rebase it was rewritten to say 28 and 30 are reserved
  and 29 belongs to #4184, and to point here. **#4183 merged on 2026-08-04**, so
  that corrected comment is now the in-tree text at `v4.2-dev` — the reservation
  of 28 and 30 and #4184's claim on 29 are recorded in `error.rs` itself, not
  only here.
* **#4184** — same rebase, same correction. Its note used to read "Codes 27-28
  are reserved" while naming **three** codes, which was correct only while the
  trio sat at 27/28/29. It now says 28 is skipped, 28 and 30 are reserved, and
  the trio is at 34–36. The discriminant itself
  (`ErrorAssetLockInsufficientFunds = 29`) never moved and remains the
  resolution of record.

The equivalent stale comment on **#4204** is gone too. It was not fixed by hand:
`#4204` rebased onto the merged base, and its `error.rs` at `4efecd5b71` now
carried #4183's corrected 28/29/30 note verbatim from trunk. All four branches
that ever held the stale reservation text — #4256, #4183, #4184, #4204 — were
clear at those surveyed heads, and #4183's version of the note became trunk.

**The stale wording has since RETURNED to trunk.** Merged #4308
(`438153da39`) introduced allocation comments that call 28 and 30 *free* —
`error.rs` ("28 (free — vacated by this PR)", "30 (free — …)", "28 and 30 are
nominally free") and the Swift mirror ("28 and 30 are free") — contradicting
rule 1 and the primary table, which RESERVE both values. Until a follow-up
corrects those in-tree comments, this registry is the authority: 28 and 30
are reserved, not allocatable.

### 27 / 28 — #3968's original collision, since moved to 42 / 43; #3954's claim merged as #4268

Found by the same 2026-08-01 sweep. #3968's live rows — now at 42 and 43 —
are in **Non-conforming allocations** above; #3954's story is there too, and
the inherited-code table covers #4259. The detail below is the 2026-08-01
record of the original 26 / 27 / 28 layout, kept because the branch's later
history (see the closing paragraph) repeated the same mistake against a fresh
pair of numbers:

* **#3968** (`5931df745a`) numbers `ErrorPersisterTransient = 26`,
  `ErrorPersisterFatal = 27`, `ErrorTransactionBroadcastRejected = 28`. It
  branched before `26 = ErrorTransactionBroadcastRejected` merged, so it both
  contradicts merged ABI at 26 **and** collides with #4185 at 27 and 28.

  The 28 is the more serious half and is easy to miss, because it does not look
  like an allocation at all: #3968 is not claiming 28 for something new, it is
  *moving a code that has already shipped* out of the way of its own 26. Rule 3
  forbids that outright. A host compiled against merged ABI returns 26 for a
  broadcast rejection; after #3968 the same condition returns 28, and 26 means
  a transient persister failure. Nothing in either branch's diff shows the
  contradiction. #3968 must leave 26 alone and take fresh integers for both
  persister codes.
* **#3954** (`93d0bd49b7`) numbered `ErrorShutdownIncomplete = 27`. This file
  previously called that a proposed-vs-proposed collision and said #4185's older
  claim should stand. **That was wrong, and it resolved the other way.** #3954
  was closed; its work landed as **#4268**, which merged 27 into `v4.2-dev` on
  2026-08-02. #4185 and #4256 moved their trio to 34–36 in response. Merging
  decides an ABI number; being the older open claim does not.
* **#4259** carries `ErrorSigningKeyUnavailable = 31` — the same number and name
  as #4183, i.e. inherited rather than a new allocation, like #4204. No
  conflict; recorded so the number is not double-counted. This is now moot:
  #4183 merged on 2026-08-04, so at #4259's current head `5b77dfd8f1` the 31 is
  simply the merged base's, and there is no second claim to reconcile.

That 26 / 27 / 28 layout is now history, resolved the only way half of it
could be: by the branch merging its base. At head `396977be4e7c`
(re-read 2026-08-26) the 2026-08-20 merge `9d0dd5a49d` and the 2026-08-25
follow-ups have `ErrorTransactionBroadcastRejected` back on 26,
`ErrorShutdownIncomplete = 27` inherited from trunk, and the rule-3 renumber
gone. But the same 2026-08-20 merge moved the persister pair to
`ErrorPersisterTransient = 42` / `ErrorPersisterFatal = 43` — numbers already
spoken for: 42 was the recorded claim of #4356 (since merged out from under
it by #4451) and 43 was held in-tree by #4313. So the branch now collides at
42 (hard:
E0081 plus a duplicate Swift raw value in its own tree, since the 08-25 base
merges imported merged 42) and at 43 (silent, against an active proposal).
The full account, timeline included, is in **Non-conforming allocations**
above. Both persister codes still owe fresh integers from the frontier note —
the single canonical source; 48 as of 2026-08-26 — and a registry row in the
same PR (rule 2).

### 26 — RESOLVED: #4196 restacked onto #4185 and is on 34 / 35 / 36

**Closed out 2026-08-03.** PR #4196 (stacked on #4185) branched before
`26 = ErrorTransactionBroadcastRejected` merged, and for most of this file's
life its head still numbered the reservation trio **26 / 27 / 28** — two moves
behind, in a state where merging it would have given 26 two meanings, given 27
two meanings against #4268's merged `ErrorShutdownIncomplete`, and contradicted
the **34 / 35 / 36** of #4185 for the same three names.

That is no longer the case. At head `12492e8c54` the restack is done:
`ErrorStaleReservationToken = 34`, `ErrorReservationTokenConsumed = 35`,
`ErrorReservationWalletMismatch = 36`, `ErrorShutdownIncomplete = 27` present
from the merged base, #4185's head `8813e98533` is an ancestor, and the PR is
MERGEABLE against `v4.2-dev`. It still allocates no integer of its own.

The numeric references #4196 owns were carried along with it. Verified at
`12492e8c54`:

* `DashSdkError.kt` — the `StaleReservationToken` KDoc reads "native code 34",
  and `fromPlatformWalletNative` maps `34 -> PlatformWallet.StaleReservationToken`.
* `ManagedCoreWallet.kt` — its V2 broadcast KDoc reads "native code 34, shared
  with the deferred-token surface"; the remaining mentions are symbolic
  `[StaleReservationToken]` links carrying no number.
* `PlatformWalletError::StaleReservation` — refers to the FFI code symbolically
  and has never contained a number, so it needed no update.

**Kept for the record, because the delay was the interesting part.** The restack
was not mechanical. Rebasing the three commits #4196 owned onto #4185's head
conflicted in three files (10 hunks): `error.rs` (3),
`wallet/core/broadcast.rs` (1), `wallet/signed_payment_registry.rs` (6). Only
the `error.rs` hunks were mechanical, because #4185 had redesigned the registry
underneath #4196 after it branched:

* `registered_height` changed from `Option<u32>` to a mandatory `u32`, and
  #4196's age guard was built around the `None` case meaning "guard disabled".
* #4185 added a `SignedPaymentError::WalletRemoved` variant and an
  owner-stamped `funding_reservation_token` field, both of which #4196 predated.
* #4196 wanted to *move* `RESERVATION_MAX_AGE_BLOCKS` and `reservation_expired`
  into `wallet/reservations.rs`; #4185 had since rewritten both in place.
* #4196's V2 guard documented "leave the stale reservation for the TTL rather
  than release by outpoint", while #4185 now releases by owner-guarded *token*.

Re-deriving the age guard against the new registry shape was author work, not
conflict resolution — which is why this sat for as long as it did rather than
being forced through by whoever was maintaining this file.

### 31 vs 33 — two signing-related codes, deliberately distinct

Review on #4256 suggested mapping its signing failure onto 31. #4256 declined and
took 33, on the grounds that 31 (`ErrorSigningKeyUnavailable`, #4183) asserts a
specific contract — the signer holds no usable private key for a requested public
key, restored from a typed signer completion code — whereas #4256's
`BuilderError::SigningFailed` also covers unresolved derivation paths, sighash
failures, and malformed signature encodings.

**That question is now fully settled — by merging and by closure, not by
agreement.** #4183 merged on 2026-08-04, so **31 is ABI** and rule 3 forbids
renumbering or retiring it; it ships with complete Swift and Kotlin mirrors.
The other side of the decision died with its owners: #4256 closed unmerged on
2026-08-06 and its in-repository successor #4311 closed unmerged on
2026-08-10, so no open PR holds the `BuilderError::SigningFailed` mapping and
33 sits RESERVED in the allocation table above. A future PR that revives that
mapping chooses fresh: route onto the existing 31 where the contract fits, or
claim a new code from the frontier — 33 itself is not reissuable.

## Collision history — the 27 / 28 / 30 → 34 / 35 / 36 move

Recorded because the reservation trio has now been renumbered three times, and
because the reason it kept moving is the failure mode this file exists to catch.

| When | Trio numbering | Why it moved |
| --- | --- | --- |
| original (#4185, #4196) | 26 / 27 / 28 | — |
| 2026-07 | 27 / 28 / 30 | `26 = ErrorTransactionBroadcastRejected` merged (`9302c62e8b`); 29 went to #4184 by agreement, so the third code took 30 |
| **2026-08-02** | **34 / 35 / 36** | **#4268 merged `ErrorShutdownIncomplete = 27` into the `v4.2-dev` ABI** |

The third move is the instructive one. On 2026-08-01 this file recorded
the `ErrorShutdownIncomplete = 27` of #3954 as a *non-conforming* claim that
had to be withdrawn, on the reasoning that #4185's 27 was the older claim and
should stand. That reasoning was wrong in the only way that matters: seniority among
open PRs does not decide an ABI number — **merging does**. #3954 was closed and
its work landed as #4268, which merged 27 first. An unmerged claim, however old,
has no standing against merged ABI (rule 3, read from the other side).

So the trio moved again, and this time it moved **above every number claimed by
anything** — merged or proposed — rather than into the next free gap:

* 27 `ErrorShutdownIncomplete` (merged, #4268)
* 29 `ErrorAssetLockInsufficientFunds` (#4184)
* 31 `ErrorSigningKeyUnavailable` (**merged 2026-08-04, #4183**; was proposed when the trio jumped it)
* 32 `ErrorTransactionBuild` (#4247/#4256)
* 33 `ErrorTransactionSigning` (#4256)

Two of those five are now shipped ABI rather than proposals: 27 already was when
the trio moved, and 31 has merged since. That is the argument for the move made
retroactively — a number that looked merely "claimed by an open PR" on
2026-08-02 is unrenumberable ABI two days later, and anything sitting on it
would now be stuck there.

Taking 34–36 rather than back-filling the vacated 28 and 30 costs two integers
in a space that is nowhere near exhausted, and buys two things: the trio reads
as one contiguous family, and it cannot be hit again by anything currently in
flight. **28 and 30 are therefore RESERVED, not free.** Do not reissue them in
this review cycle — a reviewer who saw the earlier numbering would otherwise
find a familiar number attached to an unfamiliar meaning. Rule 1 only permits
reusing a gap this file marks free, and this file marks neither of them free.

The move landed on both branches on 2026-08-02: **#4185** (`3dec774929`) and
**#4256** (`8febac177c`), each across the Rust enum discriminants and every
rustdoc cross-reference, the two `signed_payment.rs` doc references, the JNI
rustdoc (`rs-unified-sdk-jni/src/wallet_manager.rs`), the Swift
`PlatformWalletResultCode` raw values, and Kotlin's `fromPlatformWalletNative`
branches, class KDoc, `WalletManagerNative.kt` KDoc and the `DashSdkErrorTest`
offset assertions. Both `switch`es in Swift are symbolic — `init(ffi:)` matches
cbindgen `PLATFORM_WALLET_FFI_RESULT_CODE_*` constants — so only the enum's raw
values carried a number there.

Neither branch's CI could have caught the collision, for the reason given at the
top of this file: a duplicate integer across two branches produces no textual
conflict, and neither branch's tree contains both variants, so neither
compiler ever sees the E0081. Both were MERGEABLE and green throughout.

### Mirror gap on #4256 — CLOSED (was never a numbering issue)

Noted while grepping the mirrors for this move: #4256 declared
`ErrorTransactionBuild = 32` and `ErrorTransactionSigning = 33` in Rust and
mapped both in Kotlin, but its Swift `PlatformWalletResultCode` declared
**neither** — no `case`, and no arm in `init(ffi:)`, so both fell into that
switch's `default:` and would have reached Swift hosts as `.errorUnknown`,
losing their identity. That is rule 5's Swift clause. It was left for #4256's
author rather than folded into the renumber, since it was a missing mirror and
not a wrong number.

**Fixed.** Verified 2026-08-04 at #4256's head `862036b18d`:
`errorTransactionBuild = 32` and `errorTransactionSigning = 33` raw cases, both
`init(ffi:)` arms, the typed `PlatformWalletError.transactionBuild` /
`.transactionSigning` cases and their `init(result:)` arms are all present. Both
of this file's outstanding Swift mirror gaps — this one and #4204's — closed
between the 2026-08-03 and 2026-08-04 passes.

## Sibling FFI crates

`rs-sdk-ffi`'s `DashSDKErrorCode` (`packages/rs-sdk-ffi/src/error.rs`) is a
**separate** integer space (0–10, plus `InternalError = 99`) and is not contested
by any of the PRs above — none of them modify it. Do not assume a number means
the same thing in both enums.

## Survey provenance

Compiled 2026-08-01 against `v4.2-dev` at `ed4116b26c`, re-verified 2026-08-02
against `v4.2-dev` at `5d68612a45` (where `ErrorShutdownIncomplete = 27`,
PR #4268 `429667e723`, entered the merged table), re-verified 2026-08-03 against
that same base, and **re-verified again 2026-08-04 against `v4.2-dev` at
`97904ed2fc`** (the head on that date; the branch has advanced substantially
since — this whole section is the dated survey, and its merged/frontier
conclusions are superseded by the merged table and frontier note above).

The base moved on 2026-08-04, which is what made that pass necessary: four PRs
merged into `v4.2-dev` that day — **#4191** (`0e2282b586`), **#4183**
(`189a3abb1c`), **#4277** (`6704a41a85`) and **#4251** (`7afc8a8ff3`) — and one
of them, #4183, moved a number out of this file's proposed table and into the
ABI. Each of those four SHAs is the **merge commit on `v4.2-dev`**, confirmed by
`git merge-base --is-ancestor` against `97904ed2fc`, not a PR head SHA.

The 2026-08-04 pass re-read the discriminants directly, in-tree and at the
*current* head of every open PR that touches `error.rs`, `DashSdkError.kt` or
`PlatformWalletResult.swift`, and separately re-checked each PR's file list. It
confirmed — a **2026-08-04 verification snapshot, retained as record** (the
merged table and frontier note above are the current state; 34–41 have merged
since and the frontier has moved on — see the frontier note):

* in-tree at `97904ed2fc`, `error.rs` runs 0–27 contiguously and then **31**,
  with 28, 29, 30 and everything from 32 up absent. So 31 is the only number
  this file had listed as proposed that is now merged, and the frontier is
  unchanged at 38;
* `31 = ErrorSigningKeyUnavailable` has **complete** host mirrors on `v4.2-dev`
  — Swift's raw case, its `init(ffi:)` arm, the typed `PlatformWalletError` case
  and its `init(result:)` arm, and Kotlin's
  `31 -> PlatformWallet.SigningKeyUnavailable` — all introduced by
  `189a3abb1c` itself, so rule 5 was satisfied in the merging commit;
* 29 is still #4184's; 32 and 33 are still #4247/#4256's, which is why the trio
  sits at 34–36; 37 is still #4204's;
* nothing in flight has taken 28, 30, or 38;
* #3968 is unchanged and still numbers 26 / 27 / 28;
* the two Swift mirror gaps this file was tracking — #4204's typed-case gap and
  #4256's missing raw cases — have both been closed.

PR heads of record — a **historical snapshot, read 2026-08-04 (fork era)**
and retained as record. Most non-merged PRs below have since been closed and,
where still needed, recreated in-repository (see the migration note above) —
the exceptions being **#3968 and #4243, which GitHub still reports OPEN** (each
row notes its live status):

| PR | Head | Note |
| --- | --- | --- |
| #3954 | `31e22d5a90` | Closed; superseded by #4268 |
| #3968 | `5931df745a` | Head unchanged since 2026-08-03; still numbers 26 / 27 / 28. Contains base `ed4116b26c` but not `5d68612a45`, and so not `97904ed2fc` either |
| #4183 | `189a3abb1c` | **Merged 2026-08-04** (merge commit); 31 is now ABI |
| #4184 | `11c3677b1c` | Keeps 29 |
| #4185 | `326cd3eab6` | Trio at 34/35/36 |
| #4186 | `1fcdfd6b37` | Modifies `error.rs`; adds no discriminant |
| #4191 | `0e2282b586` | **Merged 2026-08-04** (merge commit); claimed no code |
| #4194 | `560f66a31d` | Modifies `error.rs`; adds no discriminant |
| #4195 | `7d20a638e5` | Modifies `error.rs`; adds no discriminant |
| #4196 | `9909f77546` | Restacked onto #4185; trio 34/35/36; allocates nothing of its own |
| #4204 | `4efecd5b71` | 37; rebased onto the merged base, so it now carries 27 and 31 from trunk; Swift mirror complete |
| #4240 | `a167afe84c` | No file under this crate |
| #4243 | `f4be5b32f0` | Head unchanged; modifies `error.rs`, claims no integer |
| #4247 | `8541073247` | 32, plus #4185's trio |
| #4251 | `7afc8a8ff3` | **Merged 2026-08-04** (merge commit); no file under this crate |
| #4256 | `862036b18d` | 32 / 33 plus the trio; Swift mirror now complete |
| #4258 | `ce8233edb7` | Merged 2026-08-03; claimed no code |
| #4259 | `5b77dfd8f1` | Carries 31 from the merged base |
| #4264 | `bf88c92b85` | Closed; work carried by #4243 |
| #4277 | `6704a41a85` | **Merged 2026-08-04** (merge commit); modifies `error.rs`, claims no integer |

Rows describing open PRs reflect those heads and go stale as the PRs are
updated; the merged table does not.

That churn is the point of dating the table. An earlier revision of this list
carried `#4185 0b0d5c76d6 (post-renumber)`, which was wrong twice over:
`0b0d5c76d6` is the *parent* of the renumber commit `d854debb`, so it was
pre-renumber, and the branch had already moved on. Every head above was read
from GitHub on the date given, not copied forward from a previous revision of
this file, and the claims attributed to #3968, #3954, #4204 and #4259 were
confirmed by reading `error.rs` at each of those heads directly.
