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
| 13 | `ErrorArithmeticOverflow` | Produced in-tree by `shielded_send.rs` (the shielded-send amount/fee overflow guard). The variant's own rustdoc still calls it a reserved slot with no producer and credits #3549 with the eventual mapping — that comment is stale and should be corrected by whichever PR touches it next |
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
| 98 | `NotFound` | Sentinel — `Option` returned as an error |
| 99 | `ErrorUnknown` | Sentinel — unmapped/flattened errors |

**Next allocatable integer: 38** — 27–37 are all claimed (27 merged; 29 and
31–37 in the proposed table below; 28 and 30 reserved). **28 and 30 are
RESERVED, not free**: #4185 and #4256 vacated them when the reservation trio
moved to 34–36, but they are deliberately left unclaimed rather than
back-filled, so that the trio stays contiguous and no number is reused within a
single review cycle. Rule 1's "do not reuse a gap unless this file marks it
free" applies — this file does **not** mark 28 or 30 free, so the frontier is
the only allocation source and a new code takes 38.

## Proposed allocations (open PRs)

Not yet ABI. Numbers here may still move; they move by agreement recorded in
this file.

| Code | Name | Owning PR | Status |
| ---: | --- | --- | --- |
| 28 | *(reserved — vacated)* | — | Vacated by #4185/#4256 on 2026-08-02; RESERVED, not reissuable — the next-free frontier is the only allocation source |
| 29 | `ErrorAssetLockInsufficientFunds` | #4184 | In review — **keeps 29** (collision resolved) |
| 30 | *(reserved — vacated)* | — | Vacated by #4185/#4256 on 2026-08-02; RESERVED, not reissuable — the next-free frontier is the only allocation source |
| 31 | `ErrorSigningKeyUnavailable` | #4183 | In review (also carried by #4204, #4259) |
| 32 | `ErrorTransactionBuild` | #4247 | In review (also carried by #4256) |
| 33 | `ErrorTransactionSigning` | #4256 | In review |
| 34 | `ErrorStaleReservationToken` | #4185 | In review — **moved 27 → 34** (also carried by #4256; #4196 inherits on restack) |
| 35 | `ErrorReservationTokenConsumed` | #4185 | In review — **moved 28 → 35** (also carried by #4256) |
| 36 | `ErrorReservationWalletMismatch` | #4185 | In review — **moved 30 → 36** (also carried by #4256) |
| 37 | `ErrorShieldedInviteAlreadyClaimed` | #4204 | In review — **moved 32 → 37** (collided with #4247's `ErrorTransactionBuild`; see below) |

Open PRs that touch `rs-platform-wallet-ffi` but claim **no** new code, verified
2026-08-03 against each PR's file list and the `error.rs` at its head:
`#3417`, `#3549`, `#3992`, `#4186`, `#4191`, `#4194`, `#4195`, `#4243`.

Four entries this list used to carry have been removed, each for a different
reason, so they are not silently re-added:

| Was listed | Why it is gone |
| --- | --- |
| `#4240` | Its head touches no file under `rs-platform-wallet-ffi` at all |
| `#4251` | Same — no file under this crate |
| `#4258` | Merged into `v4.2-dev` on 2026-08-03 (`ce8233edb7`); it claimed no code, so the merged table is unchanged |
| `#4264` | Closed. Its `error.rs` change (mapping new wallet errors onto the existing `ErrorInvalidParameter`) is carried by `#4243`, which is still open and is listed above |

`#4243` is worth naming explicitly: it *does* modify `error.rs`, but only to map
new wallet errors onto the **existing** `ErrorInvalidParameter`. Touching
`error.rs` is not the same as claiming an integer, and this list tracks the
latter.

Two more PRs carry a code they did not allocate, inherited from the PR they are
stacked on rather than claimed fresh — they must not be read as a second claim
on the number:

| Code | Name | Carried by | Allocated to |
| ---: | --- | --- | --- |
| 31 | `ErrorSigningKeyUnavailable` | #4204, #4259 | #4183 |

PR `#4196` also claims no new integer: it adds a token-less
`PlatformWalletError::StaleReservation` variant and deliberately routes it
through the **existing** `ErrorStaleReservationToken`, so it allocates nothing
and only has to follow that code's number (see below).

### Non-conforming allocations (withdraw and reissue)

These branches allocate into the same range from a stale base. They are listed
here rather than in the proposed table because their numbers cannot stand as
written — each row is a claim to be **withdrawn and reissued**, not an
allocation of record.

A rebase is a precondition, not the remedy. `#3968`'s head does contain the
2026-08-01 base `ed4116b26c` (merge commit `debf67bdae` brought it in), so it is
not simply an un-rebased branch; it is behind the *current* base `5d68612a45`,
which is where #4268's merged `ErrorShutdownIncomplete = 27` lives. Rebasing
picks that up, but rebasing alone will not resolve anything below, because git
sees no conflict in any of it — the branch has to **edit its own enum**.

| Code | Name | Owning PR | Conflict |
| ---: | --- | --- | --- |
| 26 | `ErrorPersisterTransient` | #3968 | Contradicts **merged ABI** — 26 is `ErrorTransactionBroadcastRejected` |
| 27 | `ErrorPersisterFatal` | #3968 | Contradicts **merged ABI** — 27 is #4268's `ErrorShutdownIncomplete` (was a #4185 collision until 2026-08-02) |
| 28 | `ErrorTransactionBroadcastRejected` | #3968 | **Renumbers a shipped code** 26 → 28 — forbidden by rule 3 |

PR `#3954`'s `ErrorShutdownIncomplete = 27` used to sit in this table. It is
gone because that claim **won**: #3954 was closed and superseded by **#4268**,
which merged 27 into `v4.2-dev` on 2026-08-02. See the collision history below.

PR `#3968` is the serious one: rule 3 forbids renumbering a code that has
shipped, and `ErrorTransactionBroadcastRejected = 26` is merged ABI. Moving it
to 28 would silently reinterpret every 26 an already-compiled host returns.
PR #3968 must keep 26 where it is and take fresh integers **from the frontier
(38+)** for its two persister codes. Its 27 is now doubly wrong: 27 is merged
ABI (`ErrorShutdownIncomplete`), so rule 3 protects it too. Note that 28 is
reserved, not free — it is not available to #3968 either.

## Contested and pending

### 32 — RESOLVED: #4204 moved to 37 (first collision this file actually caught)

Found 2026-08-03 while assembling the `v41int13` QA integration. #4204's head
commit `b6992a5dbc` — a review round, not the original feature work — added
`ErrorShieldedInviteAlreadyClaimed = 32` with **no row in this file**, in
direct violation of rule 2. 32 is allocated to `ErrorTransactionBuild` (#4247,
also carried by #4256).

Unlike every other entry in this section, this one was not a paper conflict:
merging #4204 into an integration that already carried
`ErrorReservationWalletMismatch = 32` produced a hard
`error[E0081]: discriminant value 32 assigned more than once`. Resolution of
record: **#4204 moves 32 → 37**, the frontier. `ErrorTransactionBuild` keeps 32.

The numbering was the lesser half of the defect. The code was **unmirrored on
both hosts** — absent from Swift's `PlatformWalletResultCode` and from Kotlin's
`fromPlatformWalletNative`. Per rule 5 that means Swift rendered it
`.errorUnknown` (identity lost), while Kotlin fell through to `Generic(32, …)`
— and in any tree carrying #4185's `ErrorReservationWalletMismatch = 32`,
Kotlin actively **misclassified** "shielded invite already claimed" as
"reservation wallet mismatch". That is the exact silently-wrong-error-on-every-host
failure this file's preamble describes, and it landed on the shielded-invite
claim-recovery path (the error is raised from four sites in
`wallet/shielded/operations.rs`, three of them inside the recovery function).

**Partially** fixed on #4204 together with the renumber. Landed at head
`d78b940a03`: the typed Kotlin
`PlatformWallet.ShieldedInviteAlreadyClaimed` (terminal, `isRetryable = false`),
the Swift `PlatformWalletResultCode.errorShieldedInviteAlreadyClaimed = 37` raw
case with its `init(ffi:)` arm, and a `DashSdkErrorTest` assertion that pins 37
so a future move off the frontier fails the suite instead of the hosts.

**Still missing on #4204, and it does not compile without it:** Swift's
`PlatformWalletError` has no `.shieldedInviteAlreadyClaimed` case, and its
`init(result:)` switches exhaustively over `PlatformWalletResultCode` with no
`default:`. Adding the raw case without the matching `init(result:)` arm makes
that switch non-exhaustive, so the Swift package fails to build at
`d78b940a03`. Rule 5's Swift clause is therefore not yet satisfied — treat the
Swift mirror as incomplete until #4204 adds the typed error case and its
conversion arm.

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

**Still outstanding:** #4196 (see below).

### 30 — vacated, then RESERVED (not free)

`ErrorAssetLockCrossDomainConsentRequired` is named as the holder of 30 in
in-tree comments on #4204 and on #4247/#4256's numbering rationale. It is
**not defined anywhere** — #4184, the PR that would have introduced it, does not
contain it after a re-scope.

Verified 2026-08-01 by reading `packages/rs-platform-wallet-ffi/src/error.rs` at
the head of **every one of the 62 open PRs**: no PR anywhere defines a code 30.
30 was therefore genuinely free at that moment, and #4185 took it — then vacated
it again on 2026-08-02 when the trio moved to 34–36. Vacating is not the same as
freeing: **30 is now RESERVED and must not be reissued** (see the collision
history below for why). The stale "reserved for the consent code" comments
should be dropped by whichever PR touches them next.

Three branches have now done so:

* **#4256** — as of `8febac177c` its `ErrorTransactionSigning` rationale names
  #4268 as the owner of 27 and records where the trio went.
* **#4183** — its enum comment no longer describes 27–28 as reserved for the
  trio; on the 2026-08-03 rebase it was rewritten to say 28 and 30 are reserved
  and 29 belongs to #4184, and to point here.
* **#4184** — same rebase, same correction. Its note used to read "Codes 27-28
  are reserved" while naming **three** codes, which was correct only while the
  trio sat at 27/28/29. It now says 28 is skipped, 28 and 30 are reserved, and
  the trio is at 34–36. The discriminant itself
  (`ErrorAssetLockInsufficientFunds = 29`) never moved and remains the
  resolution of record.

The equivalent stale comment on **#4204** is still there.

### 27 / 28 — #3968 still collides; #3954's claim merged as #4268

Found by the same 2026-08-01 sweep. These now have rows — see **Non-conforming
allocations** above for #3968 and #3954, and the inherited-code table for #4259.
The detail behind those rows:

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
* **#4259** (`9336bdbb71`) carries `ErrorSigningKeyUnavailable = 31` — the same
  number and name as #4183, i.e. inherited rather than a new allocation, like
  #4204. No conflict; recorded so the number is not double-counted.

PR `#3968` needs a rebase onto current `v4.2-dev` **and** fresh integers from
the frontier (**38+**). It must leave 26 alone; 27 is no longer available to it
either (merged ABI now), and neither are the reserved 28 and 30.

### 26 — #4196's trio collides with merged ABI (and is now two moves behind)

PR #4196 (stacked on #4185) branched before `26 = ErrorTransactionBroadcastRejected`
merged, and its head still numbers the reservation trio **26 / 27 / 28**. It is
now two moves behind: merging it as it stands would give 26 two meanings, give 27
two meanings against #4268's merged `ErrorShutdownIncomplete`, and contradict
the #4185 numbering of **34 / 35 / 36** for the same three names. It needs a
rebase and must adopt whatever numbering #4185 lands with. No new integers are
needed for it.

**All three of those numbers come from the copy of #4185 that #4196 carries, not
from #4196's own commits.** Restacking onto #4185's head therefore fixes the
trio for free — including the two moves #4196 never had to make itself. The one
number #4196 does own is a doc reference: its `StaleReservation` variant and the
matching Kotlin KDoc both cite `ErrorStaleReservationToken` as **26**, and that
becomes **34** post-restack. So the number #4196 must chase is 34.

**The restack is not mechanical — it is blocked on a redesign.** Rebasing
the three commits #4196 owns (`2d29451d06`, `c64af1a6eb`, `ea4f783490`) onto
the #4185 head (`6c37e8679e` when this was measured; now `8813e98533`) conflicts
in three files (10 hunks): `error.rs` (3),
`wallet/core/broadcast.rs` (1), `wallet/signed_payment_registry.rs` (6). The
`error.rs` hunks are genuinely mechanical. The other two are not, because #4185
redesigned the registry underneath #4196 after it branched:

* `registered_height` changed from `Option<u32>` to a mandatory `u32`. #4196's
  age guard is built around the `None` case meaning "guard disabled"; that case
  no longer exists.
* #4185 added a `SignedPaymentError::WalletRemoved` variant and an
  owner-stamped `funding_reservation_token` field. #4196 predates both.
* #4196 wants to *move* `RESERVATION_MAX_AGE_BLOCKS` and `reservation_expired`
  into `wallet/reservations.rs` so the V2 handle path can share them. #4185 has
  since rewritten both in place, with new generation-binding rationale.
* #4196's V2 guard documents "leave the stale reservation for the TTL rather
  than release by outpoint". #4185 now releases by owner-guarded *token*, which
  changes that rationale rather than conflicting with it textually.

Resolving this means re-deriving #4196's age guard against the new registry
shape, with real semantic decisions to make (does the V2 guard now release by
owner token? what replaces the `None`-disables-the-guard branch?). That is
author work, not conflict resolution, and it is why this was left rather than
forced through. shumkov's 07-24 request to restack onto #4185's post-renumber
head is actionable in the sense that the base now exists — but the restack
itself needs #4196's author.

### 31 vs 33 — two signing-related codes, deliberately distinct

Review on #4256 suggested mapping its signing failure onto 31. #4256 declined and
took 33, on the grounds that 31 (`ErrorSigningKeyUnavailable`, #4183) asserts a
specific contract — the signer holds no usable private key for a requested public
key, restored from a typed signer completion code — whereas #4256's
`BuilderError::SigningFailed` also covers unresolved derivation paths, sighash
failures, and malformed signature encodings. Both codes are currently allocated.
Maintainers may still choose to collapse them; that decision belongs to #4183 and
to #4256 jointly and should be recorded here.

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
* 31 `ErrorSigningKeyUnavailable` (#4183/#4204/#4259)
* 32 `ErrorTransactionBuild` (#4247/#4256)
* 33 `ErrorTransactionSigning` (#4256)

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

### Known mirror gap on #4256 (not a numbering issue)

Noted while grepping the mirrors for this move: #4256 declares
`ErrorTransactionBuild = 32` and `ErrorTransactionSigning = 33` in Rust and maps
both in Kotlin, but its Swift `PlatformWalletResultCode` declares **neither** —
no `case`, and no arm in `init(ffi:)`, so both fall into that switch's
`default:` and reach Swift hosts as `.errorUnknown`, losing their identity. That
is rule 5's Swift clause. Left for #4256's author rather than folded into the
renumber; it is a missing mirror, not a wrong number.

## Sibling FFI crates

`rs-sdk-ffi`'s `DashSDKErrorCode` (`packages/rs-sdk-ffi/src/error.rs`) is a
**separate** integer space (0–10, plus `InternalError = 99`) and is not contested
by any of the PRs above — none of them modify it. Do not assume a number means
the same thing in both enums.

## Survey provenance

Compiled 2026-08-01 against `v4.2-dev` at `ed4116b26c`, re-verified 2026-08-02
against `v4.2-dev` at `5d68612a45` (where `ErrorShutdownIncomplete = 27`,
PR #4268 `429667e723`, entered the merged table), and **re-verified again
2026-08-03 against the same base `5d68612a45`**, which is still `v4.2-dev`'s
head.

The 2026-08-03 pass re-read the added discriminants directly at the *current*
head of every open PR that touches `error.rs`, `DashSdkError.kt` or
`PlatformWalletResult.swift`, and separately checked each PR's file list to
decide whether it belongs in the no-new-code inventory above. It confirmed:

* 32 and 33 are still #4247/#4256's, which is why the trio sits at 34–36;
* 37 is #4204's post-renumber `ErrorShieldedInviteAlreadyClaimed`, mirrored in
  Kotlin and half-mirrored in Swift (see the code-32 section);
* nothing in flight has taken 28, 30, or 38;
* #4247's head now also carries the 34/35/36 trio, inherited from #4185 (which
  it is stacked on) rather than claimed a second time.

PR heads of record, all read on 2026-08-03:

| PR | Head | Note |
| --- | --- | --- |
| #3954 | `93d0bd49b7` | Closed; superseded by #4268 |
| #3968 | `5931df745a` | Contains base `ed4116b26c` but **not** `5d68612a45` |
| #4183 | `8387858016` | Rebased onto `5d68612a45` on 2026-08-03; keeps 31 |
| #4184 | `5bdae75391` | Rebased onto `5d68612a45` on 2026-08-03; keeps 29 |
| #4185 | `8813e98533` | Post-34/35/36 move |
| #4186 | `951260520c` | |
| #4191 | `8acb0bd14c` | Touches only `dashpay.rs` in this crate |
| #4194 | `1d812c7297` | |
| #4195 | `c471dc9fe6` | |
| #4196 | `ea4f783490` | Still 26/27/28; restack blocked, see above |
| #4204 | `d78b940a03` | Post-renumber 32 → 37; pre-#4268 base |
| #4240 | `3c19977a5a` | No file under this crate |
| #4243 | `f4be5b32f0` | Modifies `error.rs`, claims no integer |
| #4247 | `540def16a0` | Carries #4185's trio |
| #4251 | `176f8ed3eb` | No file under this crate |
| #4256 | `a456664278` | Post-34/35/36 move |
| #4258 | `ce8233edb7` | **Merged** 2026-08-03; claimed no code |
| #4259 | `9336bdbb71` | Carries #4183's 31 |
| #4264 | `bf88c92b85` | Closed; work carried by #4243 |

Rows describing open PRs reflect those heads and go stale as the PRs are
updated; the merged table does not.

That churn is the point of dating the table. An earlier revision of this list
carried `#4185 0b0d5c76d6 (post-renumber)`, which was wrong twice over:
`0b0d5c76d6` is the *parent* of the renumber commit `d854debb`, so it was
pre-renumber, and the branch had already moved on. Every head above was read
from GitHub on the date given, not copied forward from a previous revision of
this file, and the claims attributed to #3968, #3954, #4204 and #4259 were
confirmed by reading `error.rs` at each of those heads directly.
