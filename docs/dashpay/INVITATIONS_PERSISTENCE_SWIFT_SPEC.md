# Sent-Invitations persistence — Swift/SwiftData half (DIP-13 follow-up)

## 1. Problem

An inviter creates invitations (`create_invitation`), but the iOS **SwiftExampleApp**
has no "Sent invitations" list — a created invitation is invisible in the app after the
share sheet closes. The Rust half already emits the data: `create_invitation` builds an
`InvitationChangeSet` and calls `self.persister.store(PlatformWalletChangeSet { invitations:
Some(cs), .. })`. The app's persister (`FFIPersister`) never forwards that sub-field to the
host, so it never reaches SwiftData or any view.

**Goal:** (a) surface each `InvitationEntry` into SwiftData and render a "Sent invitations" list
(amount, status, expiry, inviter-flag), so the inviter can see what they sent; and (b) let the
inviter **reclaim** an unclaimed voucher — recovering its value as **Platform credits** in an
identity (see §8; the L1 DASH is burned at create-time and cannot return to the wallet).

**Non-goals:** a Rust→Swift *load/rehydrate* path (SwiftData is the UI source; no resume);
cross-device sync; changing the create/claim flows; **any L1 "DASH back to wallet"** (impossible —
the funding is an `OP_RETURN` burn).

## 2. Chosen approach — clone the `asset_locks` push-callback path

The app persists wallet state through a **C callback vtable** (`PersistenceCallbacks`): during
the Rust persister's `store()` round, each `PlatformWalletChangeSet` sub-field is projected
into a flat `#[repr(C)]` struct-per-entry and pushed to the host via an `on_persist_<kind>_fn`
callback, bracketed by `on_changeset_begin`/`on_changeset_end` (one atomic round → one
SwiftData `save()`). `InvitationChangeSet` is structurally identical to `AssetLockChangeSet`
(`BTreeMap<OutPoint, Entry>` upserts + `BTreeSet<OutPoint>` removals), so we mirror the
asset-lock wiring 1:1.

**Rejected alternative — the `dashpay_payments_overlay` pull-getter.** That domain is fetched
on demand off a live `ManagedIdentity` handle precisely because it already round-trips through
identity persistence and wanted *no* new persister callback or SwiftData path. Invitations are
a genuinely new persisted domain flowing through `store()`, so the push-callback route is the
correct one (confirmed by the payments model's own migration note: "the persister doesn't
project payment history").

**Simplification vs. the template.** `InvitationEntry` is all-POD (`out_point`, three `u32`s,
one `u64`, a `bool`, an enum). Unlike `AssetLockEntry` it carries **no owned byte buffers**
(`transaction_bytes`/`proof_bytes`), so the Rust side needs **no parallel `…Storage` Vec** and
**no pointer-lifetime management** — the FFI struct is self-contained POD.

## 3. Interface & data flow

### 3.1 Rust FFI (new `invitation_persistence.rs` + edits to `persistence.rs`, `lib.rs`)

```rust
// Field order is load-bearing: 36+4 lands amount_duffs (u64) on an 8-byte
// boundary, so the struct has ZERO internal padding (size 64, align 8). Do not
// reorder or insert a field without re-checking padding on both sides.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct InvitationEntryFFI {
    pub out_point: [u8; 36],   // 32-byte raw txid ‖ 4-byte LE vout (reuse outpoint_to_bytes)
    pub funding_index: u32,
    pub amount_duffs: u64,
    pub expiry_unix: u32,
    pub created_at_secs: u32,
    pub has_inviter: u8,       // bool → 0/1 — u8 NOT bool on purpose (a memcpy'd byte
                               // ∉ {0,1} is instant UB for Rust bool). Do not "clean up".
    pub status: u8,            // Created=0, Claimed=1, Reclaimed=2 (status_to_u8)
}
// No `unsafe impl Send/Sync` — InvitationEntryFFI has no pointer fields (unlike
// AssetLockEntryFFI). Do not cargo-cult the asset-lock unsafe impls.

// Appended at the END of PersistenceCallbacks (after the feature-gated shielded
// fields, so layout stays stable). Mirrors on_persist_asset_locks_fn exactly.
on_persist_invitations_fn: Option<unsafe extern "C" fn(
    context: *mut c_void,
    wallet_id: *const u8,
    upserts_ptr: *const InvitationEntryFFI, upserts_count: usize,
    removed_ptr:  *const [u8; 36],          removed_count: usize,
) -> i32>,   // return is round-global (see §4 T3): mirror the template — always 0.
```

- `build_invitation_entries(&[&InvitationEntry]) -> Vec<InvitationEntryFFI>` — POD projection,
  **no storage Vec** (nothing to keep alive). Reuse `outpoint_to_bytes` for `out_point`.
- `status_to_u8(&InvitationStatus) -> u8` — a **wildcard-free exhaustive** match (no `_ =>`), so
  a future variant is a compile error. Pinned by a unit test (0/1/2).
- In `FFIPersister::store()`, next to the asset-lock block, add
  `if let Some(ref inv_cs) = changeset.invitations { … }` — with the **same non-empty guard**
  the asset-lock block uses (`if !upserts.is_empty() || !removed.is_empty()`), bind the
  `Vec<InvitationEntryFFI>` and removals to `let` locals (never `.as_ptr()` on a temporary),
  pass `std::ptr::null()` when a count is 0 (an empty `Vec::as_ptr()` is dangling-nonnull), fire
  the callback, capture its `i32`, then `drop(upserts); drop(removed)` for parity with the
  template.
- Register the module: `mod invitation_persistence;` in `lib.rs`; make the struct/fn `pub`.
- `PersistenceCallbacks::Default` is a **manual impl** (not derived) — add
  `on_persist_invitations_fn: None` there; omitting it is a compile error (fail-loud).

### 3.2 Swift ingestion (`PlatformWalletPersistenceHandler.swift`)

- `@convention(c)`-compatible free function `persistInvitationsCallback(context, walletIdPtr,
  upsertsPtr: UnsafePointer<InvitationEntryFFI>?, upsertsCount, removedPtr:
  UnsafePointer<FFIByteTuple36>?, removedCount) -> Int32`. Recover the handler via
  `Unmanaged.fromOpaque(context).takeUnretainedValue()`, **deep-copy every FFI row into owned
  Swift values before returning** (Rust frees the buffers on return), build
  `[InvitationEntrySnapshot]` + `[Data]` removals, call `handler.persistInvitations(...)`.
  **Consume the cbindgen-regenerated `InvitationEntryFFI`** from the header — never a hand-written
  Swift mirror. Mirror the template's error discipline: wrap in `try?`, **always return 0** (see
  §4 T3 — the return is round-global; a failed invitation write must NOT abort the whole round).
- **Outpoint key (T1 — the critical seam):** compute `outPointHex =
  PersistentAssetLock.encodeOutPoint(rawBytes:)` **once** in the shim for each upsert snapshot,
  and store that exact display string (`<reversed-txid-hex>:<vout>`) as
  `PersistentInvitation.outPointHex` (the `@Attribute(.unique)` key). The removal path derives the
  identical string from the same `encodeOutPoint`. Both paths MUST use `encodeOutPoint` verbatim —
  never hand-roll the vout decode (ARM64 misaligned-load trap; `encodeOutPoint` already byte-copies
  into an aligned local).
- Wire `cb.on_persist_invitations_fn = persistInvitationsCallback` in `makeCallbacks()`.
  **(Feasibility-review's #1 silent-failure mode: forgetting this line compiles clean and the app
  runs — invitations just never appear. The sim acceptance test in §5 is the only gate that catches
  it.)**
- `persistInvitations(walletId:, upserts:, removed:)` runs entirely inside `onQueue` (serial
  queue confining `backgroundContext`), body **inline** — do NOT call any public `onQueue`-wrapping
  method from inside it (recursive `serialQueue.sync` deadlocks). Upsert = fetch by unique
  `outPointHex` → mutate fields incl. **`walletId` on BOTH the insert and the update branch**
  (the view's `@Query` filters on it) + `updatedAt = Date()`, else insert; remove =
  `encodeOutPoint(rawBytes:)` then fetch-and-delete. `outPointHex` is globally unique (unscoped by
  wallet) — correct here because on-chain outpoints are globally unique (unlike 20-byte address
  hashes that force a wallet-scoped predicate elsewhere). **No `save()` here** — `endChangeset`
  commits the round.

### 3.3 SwiftData model (`PersistentInvitation.swift`) + registration

```swift
@Model final class PersistentInvitation {
    #Index<PersistentInvitation>([\.walletId])
    @Attribute(.unique) var outPointHex: String     // "<display-txid-hex>:<vout>" — the T1 key
    var rawOutPoint: Data                            // 36B txid‖vout(LE) verbatim from
                                                     // InvitationEntryFFI.out_point; commit-2
                                                     // reclaim reads it directly to build the
                                                     // OutPointFFI (no reverse-encodeOutPoint
                                                     // decode / no migration). Default empty Data.
    var walletId: Data
    var fundingIndexRaw: Int
    var amountDuffs: Int64
    var expiryUnix: Int
    var createdAtSecs: Int
    var hasInviter: Bool
    var statusRaw: Int                               // enums as Int for #Predicate
    var createdAt: Date
    var updatedAt: Date
}
```

Append `PersistentInvitation.self` to `DashModelContainer.modelTypes`. Reuse
`PersistentAssetLock.encodeOutPoint` (ARM64 misaligned-load-safe) for the outpoint key.

### 3.4 UI (`InvitationsView.swift`, linked from `DashPayTabView`)

`@Query`-filtered list (mirror `ContactRequestsView`): filter by `walletId`, sort by
`createdAtSecs` desc, render short outpoint + amount + a status badge + expiry. The
`shortOutPointDisplay` / `statusLabel` helpers live **inline in `InvitationsView.swift`** (a
private extension) — not a separate `…Display.swift` file; extract to a shared file only if a
second consumer appears (the asset-lock display file was extracted precisely because it had
multi-view duplication, which invitations don't yet). The status→label switch maps an unknown
`statusRaw` to an explicit `.unknown` case (the Swift `Int` side has no compiler exhaustiveness,
unlike the Rust match). Entry point: a "Sent invitations" `NavigationLink` in `DashPayTabView`.

## 4. Failure modes & mitigations

| # | Risk | Mitigation |
|---|---|---|
| **T1** | **Outpoint key-form mismatch (highest-risk, latent).** If the upsert keys `outPointHex` on anything other than the `encodeOutPoint` display form, a future reclaim/status-sync delete (which looks up via `encodeOutPoint`) silently matches nothing → orphaned rows. Latent because reclaim is a v1 non-goal, so it passes all v1 testing. | Shim computes `outPointHex = encodeOutPoint(rawBytes:)` **once** for the upsert; the unique key IS that string; removal derives the identical string from the same fn. Test the seam now: add a create→reclaim→row-deleted round-trip test even though reclaim isn't shipped. |
| **T2** | **Round overlap.** One shared `backgroundContext` + one `inChangeset` bool; if two `store()` rounds ran concurrently, one round's `save()` would commit the other's half-applied writes. Inherited by all 8 existing kinds. | State + rely on the invariant: `store()` rounds are serialized per persister (never concurrent), and invitations introduces **no** new `store()`-driving path. If that invariant doesn't hold in `platform_wallet`, it's a **pre-existing** bug to file separately — not fixed here. |
| **T3** | **Round-global rollback.** The callback's `i32` is round-global: a non-zero return rolls back the **entire** round (discarding unrelated asset-lock/identity writes) and makes `store()` return `Err`. | Mirror the template: handler uses `try?`, shim **always returns 0**; a failed invitation write is silently skipped, never a round abort. (There is no per-kind rollback.) |
| — | `ModelContext` not thread-safe; callbacks on Tokio threads | All reads/writes through `onQueue`; body **inline**, never re-enter `onQueue`; never `save()` in the per-kind handler. |
| — | Rust frees FFI buffers on return | Shim deep-copies every row to owned Swift values **before** returning. Trivial (all POD). `let`-bound Vecs + null-when-empty on the Rust side. |
| — | ARM64 misaligned load on vout @offset 32 | Reuse `encodeOutPoint` verbatim (byte-copies into an aligned local); never hand-roll vout decode. |
| — | New `@Model` breaks the store | Additive migration: new type + non-optional columns with defaults; no `DashSchemaV1` version bump (dev stores recreate), matching the file's documented precedent. |
| **T4** | **Status drift.** Two independent encodings (FFI `u8`, sqlite `status_str`); the Swift `Int` side has no exhaustiveness. | Rust `status_to_u8` is wildcard-free (future variant = compile error); Swift maps unknown `statusRaw` → `.unknown`; unit test pins 0/1/2. (Not "one source of truth" — two encodings, each guarded.) |

## 5. Test / verification plan

- **Rust:** unit test `build_invitation_entries` round-trips each field (POD projection) + a
  test pinning `status_to_u8` values (0/1/2, wildcard-free). `cargo test -p platform-wallet-ffi`
  green; `clippy --all-features` + `fmt` clean. Note: **the Rust tests exercise the projection in
  isolation only — they cannot catch a broken FFI wire-up or a stale header.**
- **Swift build:** `build_ios.sh` (rebuild the xcframework so the regenerated header carries
  `InvitationEntryFFI` + the new vtable field — mandatory, not just for the symbol; a stale header
  = vtable mismatch = crash) + SwiftExampleApp `xcodebuild` (iPhone 17, arm64) green.
- **Sim verification — REQUIRED acceptance gate (not optional).** This is the *only* check that
  catches the two most likely failures (forgetting the `makeCallbacks()` wiring → invitations
  silently never appear; stale header → crash). Create an invitation in the app → assert a row in
  `ZPERSISTENTINVITATION` via `sqlite3` on the SwiftData store **and** in `InvitationsView`, with
  the outpoint hex matching the created voucher. Then drive a second `store()` touching the same
  outpoint (or re-create) to confirm **upsert-in-place**, not a duplicate row. Reuse the proven
  DP-14 flow.
- **T1 seam test:** even though reclaim is a v1 non-goal, add a create→(simulated
  reclaim/removal)→row-deleted assertion so the upsert-key ↔ removal-key form is exercised before
  reclaim ships. Otherwise the seam ships untested and bites when reclaim lands.
- **QA rows:** add DP-16 ("Sent invitations list reflects a created invitation; upsert-in-place on
  status change") to `TEST_PLAN.md` §4.10.

## 6. Delivery

**Same PR — #4041 / branch `feat/dip15-dashpay-invitations`** (owner-decided 2026-07-09: the
feature is cohesive create→claim→see-sent→reclaim, and #4041 isn't merged yet, so fragmenting it
into a stacked PR is needless ceremony). Trade-off accepted: this re-triggers CI + the review
bots on the new diff (fine — the PR is awaiting approval anyway). FFI + Swift split with the
swift-rust-ffi engineer: Rust FFI projection/callback + reclaim primitive (my side), Swift model
+ shim + handler + views (ffi-swift). Requires a `build_ios.sh` window (new FFI symbols), so
builds are coordinated to avoid contention.

## 7. Resolved decisions (owner sync, 2026-07-09/10)

1. **Base branch:** same PR #4041 (see §6).
2. **Scope:** display **and reclaim** (§8).
3. **Rehydrate:** push-only, no Rust→Swift load path — the already-decided architecture (the Rust
   storage layer states "the production load path does not re-hydrate invitations into the Rust
   manager; the Swift SwiftData mirror is the UI source",
   `packages/rs-platform-wallet-storage/src/sqlite/schema/invitations.rs:92-93`). A SwiftData wipe
   loses only list *visibility*, never funds or key re-derivability (`funding_index` still derives
   the voucher key).
4. **Reclaim semantics:** recover value as **Platform credits**, not L1 DASH (the DASH is burned
   at create). Confirmed acceptable; UI copy must say "recover as identity credits."
5. **Reclaim target:** user **chooses at reclaim time** — top up an existing identity OR register
   a new one from the voucher.

---

## 8. Reclaim an unclaimed voucher

### 8.1 What reclaim is (and isn't)

The invitation's DASH is **burned into an `OP_RETURN`** at create time (asset-lock special-tx:
the on-chain output is a single OP_RETURN carrying the total; the credit output — P2PKH to the
one-time key — exists only in the tx *payload* as a Platform-side authorization, never as an L1
UTXO). So there is **nothing on L1 to spend back**. "Reclaim" therefore means: **the inviter
consumes the still-unclaimed voucher into a Platform identity of their own, recovering the value
as credits.** Mechanically it's "claim your own invitation." (Evidence: OP_RETURN burn at
`transaction_builder.rs:352-356`, pinned by `asset_lock_builder.rs:713-723`; credit-output P2PKH
built at `build.rs:80-84`; stored outpoint `(txid, 0)` at `build.rs:324-325`.)

### 8.2 Primitive (reuses existing building blocks)

Consume the tracked voucher lock via `AssetLockFunding::FromExistingAssetLock { out_point }`
(the invitation's stored funding outpoint). The **inviter's own wallet signer** re-derives the
voucher key at `m/9'/coin'/5'/3'/funding_index'` itself (`resume_asset_lock` →
`rederive_credit_output_path`, `recovery.rs:371-453`) — **no key export/import** (unlike the
invitee's claim). Two targets (user picks):

- **Top up an existing identity:** `top_up_identity_with_funding(identity_id,
  FromExistingAssetLock { out_point }, asset_lock_signer, settings)` (`registration.rs:388`).
- **Register a new identity:** `register_identity_with_funding(FromExistingAssetLock { out_point },
  …)` (`registration.rs:121`) — the exact helper the existing "Resumable Registrations" flow uses.

New Rust surface is thin: a `reclaim_invitation(out_point, target, identity_signer,
asset_lock_signer, now_unix, settings)` dispatcher in `network/invitation.rs` that calls the
right helper and returns the resulting `Identity`. No new *core* mechanic.

### 8.3 FFI + Swift

- **FFI:** one new `platform_wallet_reclaim_invitation(wallet, out_point[36], target_kind: u8
  {0=topup,1=register}, identity_id[32] (topup) | identity_index: u32 (register), signer,
  now_unix, settings, out_identity_id[32], out_handle)`.
  - The `register` arm can mirror the existing
    `platform_wallet_resume_identity_with_existing_asset_lock_signer`
    (`identity_registration_funded_with_signer.rs:162`) verbatim, pointed at the invitation's
    outpoint.
  - The `topup` arm is **net-new** at the FFI layer — no existing FFI tops up from an *existing*
    asset lock (only `platform_wallet_top_up_from_addresses_with_signer`, which funds a NEW lock).
    The Rust primitive exists; wrap it.
- **Swift:** a "Reclaim" action on each `Created` sent-invitations row → a small sheet: **"Recover
  this invitation's value as credits"** with the target choice (pick an existing identity to top
  up, or "register a new identity"). On success, **the Swift side flips its own
  `PersistentInvitation.statusRaw` to `Reclaimed`** (SwiftData is the UI source — no Rust re-emit
  needed; the display-half persistence bridge is only for the create-time push). Copy is explicit:
  "recovered as identity credits", never "DASH returned".

### 8.4 Failure modes (reclaim-specific)

| Risk | Mitigation |
|---|---|
| **Consume race** — invitee claims the same voucher at the same moment | No L1 double-spend (no shared UTXO). Platform records consumed outpoints and deterministically rejects the second consume with `IdentityAssetLockTransactionOutPointAlreadyConsumed` (`verify_is_not_spent/v0/mod.rs:37-55`). Loser wastes a small ST fee, no funds lost. On that error the Swift side sets the row to `Claimed` (someone claimed it) and shows a benign "already claimed by your friend" message. |
| **User expects L1 DASH back** | UI copy says "recover as identity credits"; the reclaim sheet states the value returns as credits, not spendable DASH. |
| **Reclaim after app restart** (the common case — inviter reclaims days later) | Works: `FromExistingAssetLock` resumes the tracked lock; if the in-memory IS proof was lost on restart it falls back to SPV re-derivation (slower, still correct). **Verify** the SQLite `asset_locks` load re-attaches the proof (spike open-item #1) — perf only, not correctness. |
| **No expiry gate** | Reclaim is allowed anytime (protocol has no timelock; expiry is advisory). Product choice whether to nudge "wait until expiry"; default: allow immediately, since an invitee can claim past expiry anyway. |
| **Partial consumption remainder** | Invitation amounts are small and a single consume takes the whole value; verify the topup consumes the full voucher (no stranded remainder) — spike open-item #3. |
| **Status lifecycle now has a real emitter** | `Reclaimed`/`Claimed` are written by the Swift UI on the local row (not through the Rust changeset), so the `InvitationChangeSet::merge` insert-vs-tombstone hazard (§4-adjacent) stays latent — create is still the only Rust emitter. |

### 8.5 Reclaim test plan (adds to §5)

- **Rust:** unit-test `reclaim_invitation` dispatch (topup vs register) selects the right helper +
  `FromExistingAssetLock`. FFI marshaling test for `platform_wallet_reclaim_invitation`
  (out-param sentinels, target-kind dispatch, nullable id/index).
- **Testnet e2e (DP-17):** create an invitation → do NOT claim it → reclaim it into (a) an existing
  identity (topup: assert the identity's credit balance rises by ~the voucher value) and, in a
  second run, (b) a new identity (register: assert a new identity funded by the voucher). Verify
  on-chain via platform-explorer that the outpoint is consumed. Then attempt a second reclaim/claim
  of the same outpoint → assert the deterministic `AlreadyConsumed` rejection surfaces as the
  benign "already claimed" state. Row status → `Reclaimed`.
- **QA rows:** DP-17 (reclaim topup), DP-18 (reclaim register-new), DP-19 (reclaim-vs-claim race →
  AlreadyConsumed) in `TEST_PLAN.md` §4.10.
