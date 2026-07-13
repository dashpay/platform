# Invitations #4041 — Review-Fix Spec

Addresses the 12 actionable findings surfaced on PR #4041 after the legacy-compat
codec rework. Grouped by the fix's nature. Four other threads are **stale** (they
target code the rework removed) and get a reply-and-resolve, not a code change.

Guiding constraint (Ivan, prior sync): we are **not** doing the "auto-persist on
every key-wallet mutation" redesign. Every fix below is a *targeted* change to the
existing explicit-persist path — verified tractable because the FFI persistence
machinery already propagates a per-callback failure end-to-end (see §0).

## 0. The persistence failure-signal chain (already exists — the key enabler)

`ManagedPersister::store()` (`rs-platform-wallet-ffi/src/persistence.rs`) runs one
callback per changeset-kind. For each it checks the returned `i32`:

```
let result = unsafe { cb(ctx, wallet_id, …) };
if result != 0 { eprintln!(…); round_success = false; }
```

and at the end:

```
if !round_success {
    return Err(PersistenceError::backend("one or more persistence callbacks failed; changeset was rolled back"));
}
```

So a **nonzero Swift callback return** already becomes `store() -> Err`, which:
- **B2**: makes `persist_asset_lock_account_pools()` return `Err`, so the pre-broadcast
  gate at `build.rs:415` aborts (for `IdentityInvitation`) *before* the tx hits the wire.
- **B1**: makes the invitation-persist round return `Err` — but the current
  `create_invitation` **swallows that `Err`** (best-effort `warn`, invitation.rs:291-299),
  so the Swift-callback fix alone does not surface the failure. See B1↔B3 coupling below.

`onQueue` is `serialQueue.sync` (synchronous), so the callback body's skip/guard runs
*before* the callback returns — the return value can faithfully reflect it. The whole
round is atomic: `endChangeset` does one `backgroundContext.save()` (commit) or
`rollback()`; a nonzero callback rolls back **everything** staged in the round, and
`store()` returns `Err` *before* the pending-merge, so the Rust caller re-emits the same
changeset next round — nothing is lost (verified: this is the existing contract for
identities/txs/asset-locks).

**Scoping caveat (must-fix from review):** `on_persist_account_address_pools_fn` is the
*same* callback used for ordinary BIP44 address sync (persistence.rs:824/865), invoked up
to 3× per round. Making it strict for *everything* would turn a benign transient
`fetchAccount` miss during ordinary sync — especially a first-registration account staged
but not yet committed in the same round — into a **permanent persistence wedge**. So B2's
strictness is **scoped to the `IdentityInvitation` account type only** (see B2). Those
pools never flow through ordinary sync (the OP_RETURN credit outputs are never SPV-visible),
and the `identity_invitation` account is registered at wallet-setup (build.rs:200-216
requires it to exist), so a miss there is a genuine anomaly — safe to fail on.

---

## B1 — Invitation persist failure reported as success 🔴

**File:** `PlatformWalletPersistenceHandler.swift` — `persistInvitations` (226) +
`persistInvitationsCallback`.

**Now:** on a `fetch` failure `persistInvitations` logs and `continue`s (skips the
upsert); the callback returns `0`. A funded invitation then never reaches SwiftData
(the sole UI source, no rehydrate) yet `create_invitation` reported success → the
voucher is invisible.

**Fix (Swift half):**
1. `persistInvitations` returns `Bool` (accumulate `allPersisted`): `false` if any upsert
   was skipped due to a fetch error, or any removal fetch threw.
2. `persistInvitationsCallback` returns `1` when `persistInvitations` reports a skip,
   `0` otherwise → `round_success = false` → `store()` `Err`. (The invitation round is
   invitation-only — `store(PlatformWalletChangeSet { invitations: Some(cs), ..default })`
   — so this rollback only ever discards an invitation round; low cost.)

**Fix (Rust half — REQUIRED, this is the coupling with B3):** the Swift half alone only
rolls the round back; `create_invitation` currently **swallows** the `store()` `Err`
(invitation.rs:291-299, best-effort `warn`). The user-visible symptom ("create reports
success while the voucher is invisible") is only fixed when the **B3 reorder makes the
invitation persist propagate** — i.e. B1 and B3 are one change: the reordered persist
(B3) returns its `Err` from `create_invitation`, and the Swift callback (B1) makes that
persist actually fail when the row can't be written. Neither half is sufficient alone.

**Effect:** a transient failure rolls the round back (nothing half-committed) AND surfaces
to the caller (via B3) instead of silently dropping a funded invite. Existing `print`
telemetry stays. Removal-path fetch failure (line 288) is best-effort cleanup → include
it in the `allPersisted` flag for symmetry (a nonzero return just retries next round).

---

## B2 — Voucher-index durability gate defeated by the pool callback 🔴

**File:** `PlatformWalletPersistenceHandler.swift` — `persistAccountAddressPoolsCallback`
(~5960, always `return 0`) + `persistAccountAddresses` (2659, guard-returns on
`fetchAccount` miss, `try?` on write, `Void` return).

**Now:** even when nothing is written (missing parent account, fetch/write failure),
the callback returns `0` → `round_success` stays `true` → `store()` `Ok` →
`persist_asset_lock_account_pools()` `Ok` → the `build.rs:415` gate passes → Rust
broadcasts. A restart can then reselect the same `IdentityInvitation` funding index
and **re-export the same bearer voucher key** — the exact class of bug the gate exists
to prevent (cf. commit 55937e15c1).

**The real fallible point (review precision):** SwiftData `insert()`/property mutation
never throw — the only fallible steps are the `fetch()` calls, and the actual hole is the
**silent `guard let account = fetchAccount(...) else { return }`** at `:2665` (and the
platform-payment sub-path's own fetches at `:2760`), which stages *nothing* and so can't
be caught by `save()`. That guard-return is the thing to signal on.

**Fix (scoped to `IdentityInvitation` — do NOT tighten the shared callback for all types):**
1. `persistAccountAddresses` returns `Bool` success. It returns `false` **only when the
   failing pool's account type is `IdentityInvitation`** (the fetchAccount miss, and a
   `do/catch` on the fetches). For every other account type it keeps today's tolerant
   behavior (log + return `true`), so ordinary sync — including a first-registration
   account staged-but-unsaved in the same round — is unaffected.
2. `persistAccountAddressPoolsCallback` **accumulates** `allOk` across the whole
   `for i in 0..<count` pool loop (do NOT early-return on the first failure — that would
   skip persisting the remaining pools), and returns `1` iff any `IdentityInvitation` pool
   failed. The `isPlatformPayment` sub-path (`persistPlatformPaymentAddresses`, :2743) is
   never `IdentityInvitation`, so it stays lenient and needs no change.

Requires the callback to see each pool's account type — it already does (the FFI pool
entry carries the account spec / type; confirm `AccountSpecFFI.accountType` is readable in
`persistAccountAddresses`).

**Effect:** the pre-broadcast gate can finally abort — for `IdentityInvitation`,
`create_funded_asset_lock_proof` returns the "aborted before broadcast" error and no tx is
broadcast, so no index/key is consumed. All other account types (funds sync, topups,
platform-payment) stay best-effort, so no sync-wedge is possible.

**Why no wedge:** `IdentityInvitation` pools never flow through ordinary sync (OP_RETURN
credit outputs are never SPV-visible), and the `identity_invitation` account is registered
at wallet-setup (build.rs:200-216 requires it), committed long before any invite → the
gate's `fetchAccount` always finds it on the happy path; a miss is a real anomaly worth
failing on, never a same-round-pending-visibility flake.

---

## B3 — Funded voucher orphaned by a post-broadcast, pre-persist failure 🔴

**File:** `rs-platform-wallet/src/wallet/identity/network/invitation.rs` —
`create_invitation` (~247-305).

**Now:** after `create_funded_asset_lock_proof` spends the DASH, three paths return
`Err` *before* the `InvitationEntry` is persisted:
- IS→CL fallback rejection (247-253),
- voucher-key export (258),
- `encode_invitation_uri` on an over-long username/display-name/avatar (268).

The Sent-Invitations/reclaim UI lists only persisted `PersistentInvitation` rows, so
the funded voucher is unrecoverable through the one screen that claims it is reclaimable.

**Fix (simple reorder — NOT a skeleton/enrich split):** `expiry_unix` and `inviter`
(→`has_inviter`) are plain function parameters, fully known the instant
`create_funded_asset_lock_proof` returns — they are not derived from any of the three
fallible steps, and there is **no existing "enrichment" path** (the `InvitationEntry` is
constructed exactly once, at :277-305). So move that existing construction + `persister.store()`
block to run **immediately after the IS/CL check passes** (i.e. after :253, before the
voucher-key export at :258 and the URI-encode at :268). One reorder, no placeholder row,
no `has_inviter=false` skeleton, no reconciliation logic.

- Persist **after** the IS/CL check (so an invalid Chain-proof invite is never recorded)
  but **before** export/encode (so those failures leave the funded voucher already
  reclaimable).
- Make this persist **propagate** its `Err` from `create_invitation` (drop the current
  best-effort `warn` for *this* store) — this is the B1 coupling: a funded voucher we
  cannot record is a hard failure to surface, not a silent success.
- Edge: if `funding_index_from_path(&path)` is `None`, mirror the existing warn-skip
  (don't `unwrap`); a row without a recoverable index can't be reclaimed anyway.

**Alternatives rejected:** (a) a two-write skeleton+enrich — invents state the code
doesn't need (all fields already known); (b) moving the fallible steps *before* funding —
`path`/`proof` are only known after `build_asset_lock_transaction` auto-selects the index,
so it'd mean restructuring the builder (beyond scope); (c) unwind/refund — impossible, the
DASH is OP_RETURN-burned, so making the row exist *is* the only recovery.

---

## B4 — Reclaim UI vanishes when the last identity is deleted 🔴

**File:** `DashPayTabView.swift` (~215-225) + `InvitationsView.swift`.

**Now:** the "Sent invitations" paperplane `NavigationLink` is gated on
`if let identity = activeIdentity, let walletId = identity.wallet?.walletId`.
Invitations are wallet-scoped; deleting the last local identity nils `activeIdentity`
while the wallet + its funded invitations survive → the reclaim UI disappears.

**Fix:**
1. Key the paperplane link off `claimWalletId` (the existing fallback that resolves to
   any loaded wallet, 131-134) instead of `activeIdentity`.
2. Make `InvitationsView.identity` optional; the list (reclaim swipe) needs only a
   `walletId`. Gate **only** the "+" create button (`dashpay.invitations.create`) on an
   available identity, since creating an invite needs an inviter identity but reclaiming
   does not. (`identity` is used in exactly one place — `CreateInvitationSheet(identity:)`
   at InvitationsView.swift:77 — so optionalizing is clean; the `.sheet` closure must
   `if let`/`guard` the optional so it compiles.)

**Call-site caveats:** update the DashPayTabView paperplane link (214-221) to pass
`network` (a view-level property, still available when `activeIdentity == nil`). With
multiple loaded wallets the paperplane + its `@Query` bind to whichever wallet
`claimWalletId` resolves to — acceptable for the example app and consistent with the claim
flow's own fallback.

**Effect:** funded invitations stay reclaimable regardless of identity state.

---

## B5 — Bearer key in an interceptable custom URL scheme 🔴 (doc-level)

**File:** `Info.plist` (~36-45) + the inline comment.

**Now:** the one-time voucher key ships inside a `dashpay://` custom scheme, which iOS
does not make exclusive — a co-installed same-scheme app can intercept the link. The
inline comment still says `?data=…` (stale — it's the query form now).

**Fix (minimum, matching the reviewer's accepted bar for example-app code):**
1. Fix the stale `?data=…` comment to the current query form.
2. Add a "known limitation" note pointing at the HTTPS universal-link alternative
   (`invitations.dashpay.io/applink`, already parser-supported) as the preferred
   delivery, and reference the existing infra tracking issue (#4096).

No code behavior change — the custom scheme stays for the example app; the universal-
link transport is externally blocked (Android team creds) and tracked separately.

---

## Minor (🟡 / 💬)

- **S1** `network/invitation.rs:286` — `created_at_secs = expiry_unix.saturating_sub(TTL)`
  back-computes creation time; a non-FFI caller with a custom `expiry_unix` persists a
  1970/garbage `created_at`. **Fix:** thread an explicit `now_unix` (already available at
  the FFI boundary) into `create_invitation` and store it directly as `created_at_secs`.
- **S2** `asset_lock/build.rs:415` — the funding-pool snapshot isn't lock-held across
  build→persist, so concurrent invitation builds could clobber a newer pool with an older
  one (key reuse). **Decision: DROP the code change** (review consensus). The proposed
  per-wallet write lock would **self-deadlock** — `build_asset_lock_transaction` takes
  `wallet_manager.write().await` (build.rs:66) and `persist_asset_lock_account_pools`
  takes `wallet_manager.read().await` (:296) on a non-reentrant tokio `RwLock`. The
  residual is (a) already documented in-code as low-severity + **self-healing on the next
  build** (build.rs:284-288), and (b) already serialized in the shipped UI
  (`CreateInvitationSheet.swift:212` `guard !isCreating`). For a single-user example app
  that's sufficient. Keep the existing code note; reply on the thread explaining the
  deadlock + self-heal rationale. (If ever needed: a dedicated non-reentrant per-wallet
  `Mutex` used *only* around build+persist, never re-entering `wallet_manager`'s lock.)
- **S3** `ReclaimInvitationSheet.swift:189-191` — `reclaimInFlight` is set/saved before
  the pre-broadcast local work (`prePersistIdentityKeysForRegistration`, 207), so a
  local failure leaves the marker set → a later genuine foreign claim can be
  misclassified as self-reclaim. **Fix:** set the marker only immediately before the
  network consume; treat pre-broadcast local failures as unambiguous "did not reclaim"
  (clear/never-set the marker).
- **S4** `ReclaimInvitationSheet.swift:228-255` — the self-reclaim-vs-foreign-claim
  branching is inline; only `isAlreadyConsumed(message:)` is unit-tested. **Fix:**
  extract the terminal-state decision (`hadPriorReclaimInFlight` × `isAlreadyConsumed` →
  Reclaimed/Claimed/error) into a pure `nonisolated static` function and unit-test all
  three outcomes.
- **N1** `ClaimInvitationSheet.swift:202` — `claim()` re-reads `trimmedURI` at submit
  instead of the captured `preview`; the URI field has no `.disabled(isClaiming)`. Not
  exploitable (evaluated synchronously before the first `await`; Rust re-validates), but
  coherence-worthy. **Fix:** freeze `let submittedURI = trimmedURI` at claim start and
  add `.disabled(isClaiming)` to the field.

## Spec-doc updates (📄)

- **M1** `INVITATIONS_PERSISTENCE_SWIFT_SPEC.md:101` — restate to match B1 (persist
  failures are now signaled, not swallowed).
- **M2** `INVITATIONS_PERSISTENCE_SWIFT_SPEC.md:280` — restate to match S3 (marker
  placement disambiguates; update the "classify every AlreadyConsumed" wording).

## Stale — reply-and-resolve, no code (✓)

- **ZAZV** (`crypto/invitation.rs`) — "encoder emits links its own parser rejects."
  Targeted the old base58 payload embedding the *full funding tx* + `MAX_INVITATION_DATA_B58_LEN`;
  the new codec embeds only the txid and fetches the tx at claim. Constant/mechanism gone.
  *(Residual, out of the finding's scope: `islock` hex is length-unchecked at encode vs.
  the 8192 URI cap — only reachable with a ~100-input funding tx; note in the reply, low
  priority.)*
- **umY / DK** (`ClaimInvitationSheet.swift`) — "verify inviter DPNS name↔id before
  bootstrap." The new link carries **no identity id** (`inviter_id` always zeroed); the
  contact prompt is built from `username` only and `sendContact` resolves the id via
  `resolveDpnsName(username)` — displayed name and contacted id are the same DPNS entry.
  Vector gone.
- **umd** (`rs-platform-wallet-ffi/src/invitation.rs`) — "preview violates has_inviter/
  username invariant on interior-NUL usernames." The ABI now *intentionally* emits
  `has_inviter=true` with a null `inviter_username` for du-less metadata-only links
  (`username: Option`); Swift guards `if p.hasInviter, let name = p.inviterUsername`. The
  suggested "drop to has_inviter=false" would wrongly discard display-name/avatar-only
  invites. *(Minor doc nit: the field comment at :333-334 is slightly inaccurate — fix
  in passing.)*

## Test / verification plan

- **B1/B2:** unit-test the callbacks return nonzero on a simulated failure and `0` on
  success; assert `store()` returns `Err` (Rust round_success path already covered).
  - **B2 wedge-avoidance (critical):** the funded regression MUST exercise the
    **first-registration same-round** path — a brand-new account whose registration +
    address pools land in one `store()` round — and assert the pool durably persists
    (proves the non-`IdentityInvitation` lenient path is untouched and doesn't wedge).
  - **B2 accumulation:** unit-test that a mid-loop `IdentityInvitation` pool failure
    surfaces as callback `1` and that a non-invitation pool failure in the same loop does
    *not* (scoping holds); confirm the loop persists all pools (no early return).
- **B3:** integration — force `encode_invitation_uri` to fail (over-long username) after a
  funding success and assert a reclaimable row exists AND `create_invitation` returns `Err`.
- **B4:** UI/logic — with `activeIdentity == nil` but a loaded wallet, the paperplane link
  is present, the list renders, and the "+" create is disabled.
- **S3 (red→green):** a `.register` reclaim whose `prePersistIdentityKeysForRegistration`
  throws (pre-broadcast) must leave `reclaimInFlight == false`, so a subsequent
  foreign-claim "already consumed" classifies as **Claimed(1)**, not Reclaimed(2). Assert
  the OLD marker placement misclassifies (red) and the moved one classifies correctly (green).
- **S4:** unit-test all three `(hadPriorReclaimInFlight × isAlreadyConsumed)` outcomes of
  the extracted pure function.
- **S1/N1:** targeted unit tests (S1: custom `expiry_unix` no longer poisons `created_at`).
- **Full regression:** 24 platform-wallet + 10 ffi invitation tests + the funded sim e2e
  stay green; `fmt --all` + `clippy --workspace --all-features` clean; iOS build.
