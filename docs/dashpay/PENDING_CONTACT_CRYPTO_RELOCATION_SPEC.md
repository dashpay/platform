# Relocate the deferred-crypto queue from the wallet to the identity

**Status:** reviewed (research + 4-lens spec review folded); implementing.
**Scope:** `packages/rs-platform-wallet` (+ a one-line doc-comment in `rs-platform-wallet-storage`).
Rust-only. FFI signatures, Swift, and whole-struct serialization are **unchanged**.

## 1. Problem

`pending_contact_crypto: Vec<PendingContactCrypto>` lives on the **wallet-level** struct
`PlatformWalletInfo` (`wallet/platform_wallet.rs:57`), a sibling of `identity_manager`. Every
*other* DashPay artifact already lives **per-identity** on `ManagedIdentity`
(`state/managed_identity/mod.rs`): `established_contacts`, `sent_contact_requests`,
`incoming_contact_requests`, `dashpay_rescan_triggered`, `auto_accept_verify_failed`,
`dashpay_payments`. The queue is the lone exception, and each `PendingContactCrypto` entry carries
`owner_identity_id` — manually re-storing the exact container key that is *implicit* for the
others. Identity-network code (`IdentityWallet<B>`) reaches *up* into wallet state to touch it.

This is **cleanup, not a bug fix**. The queue is functionally correct where it is; the value is
consistency/maintainability. The risk is a routing regression on a signer-gated DashPay path (a
mis-routed drain → a contact account never gets built → a DashPay payment silently can't resolve
its external account). So it is speced, reviewed, isolated (its own commit/PR), and tested.

## 2. Current architecture (verified in research + review)

- **Type** (`changeset/changeset.rs:1075`): `PendingContactCrypto { owner_identity_id, contact_id,
  op: PendingContactCryptoOp, enqueued_at_ms }`. Dedup key `PendingContactCryptoKey =
  (owner_identity_id, contact_id, kind)`; `upsert_pending_contact_crypto` keeps ≤1 entry per key.
- **Receiver**: methods are on `IdentityWallet<B: TransactionBroadcaster>` (bound to a `wallet_id`,
  NOT one identity); reaches the queue via `wm.get_wallet_info(&self.wallet_id)`.
- **`IdentityManager` has TWO buckets** (`state/manager/mod.rs:69-83`):
  `wallet_identities: BTreeMap<WalletId, BTreeMap<RegistrationIndex, ManagedIdentity>>` and
  `out_of_wallet_identities: BTreeMap<Identifier, ManagedIdentity>`. Today's flat wallet-level queue
  is **bucket-agnostic**. This is the crux of the refactor's one real trap — see §3 D4 / §5 R1.
- **Enqueue** — exactly THREE production sites, each already holding `&mut PlatformWalletInfo` and
  the owner id: `enqueue_pending_auto_accepts` (`contact_requests.rs:1553`),
  `enqueue_deferred_contact_crypto` (`:1734`), `enqueue_contact_info_decrypt`
  (`contact_info.rs:385`). Each pairs the in-memory `upsert_pending_contact_crypto(&mut
  info.pending_contact_crypto, e)` with a changeset `pending_contact_crypto_added: vec![e]` and a
  `persister.store(...)`. (`payments.rs:2757` is a **test**, not a production enqueue.)
- **Drain** (`drain_pending_contact_crypto`, `contact_requests.rs:1781`): read-lock → clone the flat
  queue → **drop lock** → async match over the owned snapshot (each arm routes every side-effect by
  `entry.owner_identity_id`; the loop body never touches the queue) → write-lock → single
  `retain_drained_by_snapshot(&mut info.pending_contact_crypto, &cleared)`.
  `drain_auto_accepts` (`:2160`) is the signer-gated sibling for `AutoAccept` ops; its removal block
  also marks `auto_accept_verify_failed` per owner.
- **Count** (`pending_contact_crypto_count`, `:1763`): `count_account_build_ops` over the flat queue
  (excludes `ContactInfoDecrypt`). Backs the "waiting for unlock" UI banner.
- **Op ownership asymmetry (subtle, load-bearing for R1):** `RegisterReceiving` /
  `RegisterExternal` are owned-only (`build_contact_accounts` gates on `identity_index.is_some()`,
  `contact_requests.rs:1661`); `ContactInfoDecrypt` is owned-only (`contact_info.rs` iterates only
  `wallet_identities`). But `AutoAccept` is **NOT** gated — `enqueue_pending_auto_accepts` runs for
  every identity in the sweep's `all_identities()` loop (both buckets), so an `AutoAccept` op can
  legitimately land on an **out-of-wallet** identity's queue.
- **Changeset** (`PlatformWalletChangeSet.pending_contact_crypto_{added,cleared}`,
  `changeset.rs:1189`): flat top-level Vecs; entries carry the owner.
- **Apply is a no-op for the queue** (`wallet/apply.rs:115`): the in-memory queue is mutated
  *directly* at the enqueue/drain sites; the changeset deltas are for persistence, not in-memory replay.
- **The queue IS durably persisted** — via the `rs-platform-wallet-storage` SQLite backend: table
  `pending_contact_crypto` keyed `(wallet_id, owner_identity_id, contact_id, kind)`
  (`migrations/V001__initial.rs:87`), live writer `apply_pending_contact_crypto`
  (`sqlite/schema/pending_contact_crypto.rs:49`) driven from `apply_changeset_to_tx`
  (`sqlite/persister.rs:1063`), reader `all_pending_contact_crypto` (`:108`), round-trip test
  (`:161`). The **FFI/SwiftData** backend has no callback for it, so on iOS it is not durably
  persisted — but that is one backend, not "the field is vestigial." **The changeset fields are
  load-bearing; nothing here gets deleted.** Because the SQLite writer keys on
  `(wallet_id, owner_identity_id, contact_id, kind)`, moving the *in-memory* field per-identity
  changes **zero** SQLite writes (the owner stays on every row — D2/D5).
- **Not restored on cold load** (`manager/load.rs:102-114`): starts `Vec::new()`; the sweep
  re-enqueues. A restore path is half-wired but blocked upstream — see R6.
- **FFI** (`ffi/dashpay.rs:733, 798`): `platform_wallet_drain_pending_contact_crypto` /
  `_count` take a **wallet** handle and call `wallet.identity().<method>()`. Called from Swift
  (`PlatformWalletManager.swift:640,968`). D4 keeps the `IdentityWallet` method signatures →
  **no FFI or Swift change**.
- **Accessors** (`state/manager/accessors.rs`): `managed_identity(&Identifier)` /
  `managed_identity_mut(&Identifier)` (`:70,75`) already resolve across **both** buckets via
  `location_index`. Enumerators `all_identities() -> Vec<&Identity>` and `identity_ids() ->
  Vec<Identifier>` exist, but **there is no iterator yielding `&ManagedIdentity`** — one is added
  (D3).

## 3. Design

Move the in-memory Vec to `ManagedIdentity`, keyed by the owning identity. Keep
persistence/apply/FFI shapes unchanged to bound the blast radius.

- **D1 — Field placement.** Add `pending_contact_crypto: Vec<PendingContactCrypto>` to
  `ManagedIdentity`; remove it from `PlatformWalletInfo`. Init `Vec::new()` in `ManagedIdentity::new`
  + `new_out_of_wallet` (next to `established_contacts`); drop the 5 `PlatformWalletInfo` init sites.
- **D2 — Keep `PendingContactCrypto` unchanged (keep `owner_identity_id`).** It is the drain's
  routing key (each op's side-effects derive from it) AND the SQLite key column; keeping it holds the
  type, dedup key, changeset, and their tests stable. The in-memory redundancy (owner == container)
  is benign. *Dropping it is out of scope* (§7).
- **D3 — Access by identity.**
  - Enqueue + per-owner removal: existing `managed_identity_mut(&owner)` (spans both buckets).
  - Drain-snapshot + count: **add** `IdentityManager::managed_identities(&self) -> impl Iterator<Item
    = &ManagedIdentity>` chaining `out_of_wallet_identities.values()` with
    `wallet_identities.values().flat_map(|m| m.values())`. Iterate **both buckets** (R1).
- **D4 — Drain/count: flat snapshot → unchanged async loop → per-owner-grouped removal. Both
  buckets. Same signatures, same wallet-wide semantics.** Concretely (do NOT write an outer
  per-identity loop — it borrows `&mut ManagedIdentity`/holds the lock across `.await` and will not
  compile):
  1. **Snapshot:** under a read guard, gather every resident identity's queue into one flat owned
     `Vec<PendingContactCrypto>` (`managed_identities().flat_map(|m| m.pending_contact_crypto.iter()
     .cloned())`), then drop the guard. `count` sums `count_account_build_ops` per identity the same
     way.
  2. **Async loop:** unchanged — it already keys every lookup/side-effect off
     `entry.owner_identity_id` and touches the queue nowhere.
  3. **Removal:** under a write guard, group `cleared_snapshots` by `owner_identity_id` and, per
     owner, `retain_drained_by_snapshot(&mut managed_identity_mut(&owner).pending_contact_crypto,
     &subset)`. Fully synchronous under one guard — nothing crosses `.await`.
     `retain_drained_by_snapshot`'s value-equality (which includes the owner) transfers unchanged.
     For `drain_auto_accepts`, the same per-owner hop also carries the `auto_accept_verify_failed`
     mark (already per-owner today).
  - *Send-drain scope (Q1 resolved → wallet-wide):* keep `payments.rs:575`
    `self.drain_pending_contact_crypto` draining every resident identity, not just the sender. It is
    safe and useful — the Keychain provider is wallet-**seed**-scoped, so one identity's send
    correctly finishes other identities' pending builds as a free, correct side effect; no
    cross-identity dependency exists (accounts are keyed by both ids). Narrowing to sender-only is a
    one-line snapshot filter with only a mild latency/UX argument — deferred.
- **D5 — Changeset + apply + FFI unchanged.** Keep `pending_contact_crypto_{added,cleared}` flat
  (entries carry owner), keep `apply.rs` ignoring them, keep the FFI signatures. Only the *in-memory*
  field + its access sites move. The SQLite writer is unaffected (owner-keyed).
- **D6 — `ManagedIdentity` field is persistence-inert. Do NOT add it to `IdentityEntry::from_managed`**
  (`changeset/changeset.rs:332`), which explicitly enumerates the persisted per-identity fields.
  Leaving it out keeps the queue in-memory-only per identity (like `established_contacts` /
  `dashpay_rescan_triggered`), so it is NOT double-persisted (once via the flat changeset delta,
  never via a snapshot). This preserves D5.

## 4. Alternatives rejected

- **Per-identity changeset routing:** unnecessary — apply ignores the queue deltas and the SQLite
  writer already keys by owner. Adds churn + a migration question for zero benefit.
- **Drop `owner_identity_id`:** forces the changeset/SQLite key to carry the owner another way and
  rewrites the dedup key + tests. Higher risk, separable, deferred (§7).
- **Move only to `IdentityManager`:** leaves the queue one flat list one struct deeper — still not
  keyed by identity. Doesn't achieve the goal.
- **Defer / TODO:** legitimate (a reviewer's call, given this path just absorbed the scalar
  elimination). Decision: proceed now as an isolated, tested, reviewed change so it's bisectable.

## 5. Failure modes & risk register

| ID | Risk | Mitigation |
|----|------|------------|
| R1 | **[Critical]** Drain/count iterate only the owned bucket → an `AutoAccept` op on an *out-of-wallet* identity is silently never drained/counted (auto-accept never fires; banner under-counts). This reproduces the exact silent signer-gated regression this refactor fears. | D3/D4 iterate **both** buckets (`managed_identities()`). Test: an out-of-wallet identity holding an `AutoAccept` entry is still counted and drained. |
| R2 | Drain restructure holds a `&mut ManagedIdentity` or the wallet-manager guard across `.await` → won't compile / deadlock (the register fns re-acquire the non-reentrant manager lock). | D4: flat owned snapshot → drop guard → async loop → re-lock → synchronous per-owner removal. Never a live `values_mut()` borrow across `.await`; snapshot ids/entries into owned Vecs, re-lookup per owner (mirrors the sweep). |
| R3 | Enqueue routes to a wrong/absent identity. | Owner is already in hand at all 3 sites; add `managed_identity_mut(owner)` before upsert. `None` (identity removed in the narrow collect-guard→write-guard window) → log + drop (benign; the identity is gone). Test: enqueue lands on the owner's queue and nowhere else. |
| R4 | `count`/`drain` totals drift (miss an identity). | Same signatures + wallet-wide semantics over both buckets; test with 2 identities each holding entries asserts the aggregate equals the sum. |
| R5 | Auto-accept verify-failure marking regresses. | Marking is already per-owner (`managed_identity_mut(owner).mark_...`); folds into the same removal hop. Keep its test. |
| R6 | Future cold-load restore drops entries (a persisted row whose owner identity isn't applied yet). | The move introduces an ordering constraint the flat queue didn't have: any future restore must apply identities **before** fanning each persisted row out to its owner's queue. Documented here + in the storage doc-comment so whoever finishes the (currently blocked) restore doesn't reintroduce the drop. Not active today (nothing restores). |
| R7 | Identity removal now GC's its queue (dies with the `ManagedIdentity`). | Behavior change vs the flat wallet Vec (entries used to outlive owner residence). **Accepted** — it's orphan cleanup; a transient remove/re-add loses queued ops that the sweep re-enqueues on re-add. Noted, no code needed. |
| R8 | Identity removed between drain snapshot and removal → its keys aren't retained-off/cleared. | Net-identical to today: the op's target is gone and `apply` ignores the `cleared` delta anyway. Noted. |

## 6. Change list (critical files)

- `wallet/platform_wallet.rs` — remove the field + doc.
- `wallet/identity/state/managed_identity/mod.rs` — add the field + doc.
- `wallet/identity/state/managed_identity/identity_ops.rs` — init in `new` + `new_out_of_wallet`; **do not** touch `from_managed`.
- `wallet/identity/state/manager/accessors.rs` — add `managed_identities()` iterator (both buckets).
- `wallet/identity/network/contact_requests.rs` — 2 enqueues (`:1553`, `:1734`); `drain_pending_contact_crypto` (snapshot both buckets, per-owner removal); `pending_contact_crypto_count` (sum both buckets); `drain_auto_accepts`; `empty_info` test helper (`:3254`); the drain/count/auto-accept tests.
- `wallet/identity/network/contact_info.rs` — enqueue (`:385`).
- `wallet/identity/network/payments.rs` — send drain unchanged (`:575`); **re-seed** the drain tests (`:2757`, `:2811`) with real registered identities (out-of-wallet for the `identity_index==None` case).
- `manager/load.rs:114`, `manager/wallet_lifecycle.rs:249`, `wallet/apply.rs:420`, `wallet/platform_wallet_traits.rs:43,56` — drop the `PlatformWalletInfo` init sites.
- `rs-platform-wallet-storage/src/sqlite/schema/pending_contact_crypto.rs:100-106` — update the doc-comment that names `PlatformWalletInfo.pending_contact_crypto` as the restore target (now per-identity fan-out by `owner_identity_id`; see R6).
- `ffi/dashpay.rs` — unchanged; verify it still compiles.

## 7. Explicitly out of scope

- Dropping `owner_identity_id` from `PendingContactCrypto`.
- Deleting/altering the `pending_contact_crypto_{added,cleared}` changeset fields — they are
  persisted by the SQLite backend (§2). NOT vestigial.
- Wiring the cold-load restore (blocked upstream); this change only leaves it a correct ordering note.
- Any Swift / FFI-signature change.

## 8. Test / verification plan

- **R1 (the one that matters):** an out-of-wallet identity holding an `AutoAccept` entry is counted
  by `pending_contact_crypto_count` and processed by the drain — asserts both buckets are iterated.
- **R3:** enqueue lands on the owner identity's queue and no other identity's.
- **R4:** 2 resident identities each holding queue entries → aggregate count + drain == sum.
- **R2:** re-uses `retain_drained_by_snapshot`'s existing value-equality test shape, per-owner.
- **R5:** auto-accept verify-failure marks the right identity.
- Cold-load: a freshly-loaded identity has an empty queue; the sweep re-enqueues (behavior unchanged).
- Keep green (re-seeded where noted): `send_payment_runs_pending_contact_crypto_drain`,
  `drain_completes_register_receiving_and_clears_queue`,
  `drain_leaves_register_external_it_cannot_complete`,
  `account_build_count_excludes_contact_info_decrypt`, the changeset merge/dedup tests.
- `cargo test -p platform-wallet -p platform-wallet-ffi`; `cargo clippy … --all-targets` clean;
  `build_ios.sh --target sim` BUILD SUCCEEDED (FFI unchanged → Swift unaffected).
