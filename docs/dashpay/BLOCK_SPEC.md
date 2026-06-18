# DashPay "Block sender" — design spec

Status: **SUPERSEDED (2026-06-18) — replaced by the implemented local-only Ignore.**
This single-device design is 4-lens reviewed (§0 R1–R10) and stays valid as the
reference, but the **shipped** feature is the simpler per-sender, reversible
**Ignore** (= block) in `docs/dashpay/SYNC_CORRECTNESS_SPEC.md` / Spec 2 — see the
TODO. The intermediate plan to carry block/reject **via `contactInfo`** for
cross-device sync was **REJECTED**: R1 found that a `contactInfo` about a
*non-established* sender leaks *who* you ignored (its public `$createdAt`/`$updatedAt`
correlate with the inbound `contactRequest`'s timestamp via the public indexes), and
the DIP-15 ≥2-contacts ambiguity gate doesn't cover a fresh non-established sender.
What actually landed:
1. **`CONTACTINFO_FORMAT_SPEC.md`** — privateData CBOR→DIP-15 varint. **IMPLEMENTED**
   (carries alias/note/displayHidden/acceptedAccounts; does NOT carry ignore — R1).
2. **Ignore = per-sender, reversible, LOCAL-ONLY.** **IMPLEMENTED** across all layers
   (changeset, FFI, SwiftData, *and* the SQLite persister's `ignored_senders` table).
3. **Cross-device ignore** — deferred to a future **encrypted field on the `profile`
   document** (contract / governance track), whose update timing is conflated with
   normal profile edits so it doesn't leak the per-sender existence/count. Not built.

Owner: platform-wallet / swift-sdk
Relates to: `docs/dashpay/SPEC.md` (G5 rejection), the existing per-request
Decline flow.

---

## 0. Review resolutions (authoritative — supersedes inline text where they conflict)

Folded from a 4-lens multi-agent spec review (feasibility / scope / adversarial /
security-privacy). Ordered by severity.

**R1 (CRITICAL, security+feasibility) — Cross-device via `contactInfo`-reuse is OUT.**
Creating a `contactInfo` *about a non-contact* breaks the DIP-15 ≥2-contacts
unlinkability gate: the doc's public existence + `$createdAt` correlates with the
inbound `contactRequest` (public `userIdCreatedAt` index) to re-identify *who* you
blocked, even though `encToUserId` is encrypted. The "displayHidden is precedent"
claim is a false equivalence — displayHidden rides a doc that exists anyway; a
block-of-a-non-contact *creates* the leaking doc. It's also mechanically blocked:
`set_contact_info_with_external_signer`→`set_contact_metadata` hard-requires an
established contact (returns `false`→error otherwise), and the apply side drops
non-established. **Resolution:** if cross-device ever ships, it is a **single
owner-scoped, self-encrypted blocklist document** (leaks only "a blocklist exists"
+ edit-count, not one-doc-per-victim) — a contract change on the later track.
§11.2's contactInfo-reuse is struck as the primary plan; open-question-g resolves
to **"yes, it leaks — demonstrably."**

**R2 (CRITICAL, security) — Unblock vs Phase-2 incremental fetch contradiction.**
§2/§8 promise "unblock → requests reappear next sync," but Phase-2 high-water
`$createdAt` will have advanced past the blocked request, so it never refetches —
unblock silently does nothing after Phase 2. **Resolution:** `unblock_sender` must
**rewind the identity's received-request high-water** (so the sender's docs
refetch) — OR the guarantee changes to "unblock does not resurface already-passed
requests" and the UI says so. *Decision needed (see Q1 below); default = rewind.*

**R3 (CRITICAL, adversarial+feasibility) — Gate the auto-establish paths, not just
the read loop.** The §4.2 gate must sit at the **top of the per-sender loop body,
before the `tracked_reference`/rotation dispatch** — otherwise a blocked
established sender's rotation runs `apply_rotated_incoming_request`, which clears
`payment_channel_broken` and **reactivates a payment channel to someone you
blocked.** Additionally, `is_sender_blocked` must be consulted inside the
state-layer establish methods (`add_sent_contact_request`,
`add_incoming_contact_request`, `apply_rotated_incoming_request`), and
`block_sender` must clear/guard `sent_contact_requests[sender]`. The read-loop gate
alone is necessary-but-not-sufficient.

**R4 (CRITICAL, adversarial) — Rust `apply.rs` + `Merge` must handle the new
fields.** `ContactChangeSet` is destructured exhaustively (no `..`) in
`apply.rs`, so adding `blocked`/`unblocked` breaks the build until handled —
specify: apply inserts `blocked` into `blocked_senders`, removes `unblocked`
keys, **applies `unblocked` after `blocked`** (latest-action-wins), extend
`Merge::merge`/`is_empty`, and `block`/`unblock` never emit both for the same
sender in one changeset. The Rust apply-restore is **separate from and additional
to** the FFI/Swift restore, not optional.

**R5 (HIGH, all three) — Do NOT drop reject tombstones on block.** There is no
`removed_rejected` changeset channel (rejected is upsert-only); adding one is a
cross-layer change. And dropping them creates a lost-request hole under the
`limit:100` truncation + resurrects previously-declined refs on unblock. They're
**inert** under the sender-superset block (the gate short-circuits first).
**Resolution:** keep them; delete §3's "drop on block" + §4.3 step 3. Simpler AND
correct.

**R6 (HIGH, security) — §11.1 countable index leaks the inbound social graph.**
Count proofs are **public, not recipient-private**, and return cleartext
`{sender_id → count}`. A countable `[toUserId, $ownerId]` lets *anyone* scrape
"who contacted R, with counts" in O(log n). **Resolution:** drop the per-sender
`GROUP BY` axis; keep at most an aggregate `COUNT(*) WHERE toUserId == me` (a
single number) for a badge; carry the graph-exposure analysis into the DIP.

**R7 (HIGH, adversarial) — Persist-failure atomicity.** `block_sender` mutates
in-memory then stores; a store failure leaves memory=blocked / disk=unblocked.
**Resolution:** persist-first, mutate-`blocked_senders`-after-success (clean
rollback on failure). (Less severe now that R5 stops dropping tombstones.)

**R8 (HIGH, adversarial) — Multi-device shared identity: blocked-becomes-established.**
Device A blocks B; device B accepts → A reconciles the sent reciprocal → A holds
`blocked + established` (which §8 calls impossible). **Resolution (Q2):** on sync,
if a blocked sender becomes established, **auto-lift the block + UI notice**
(accept wins) — default — or suppress-while-blocked. *Decision needed.*

**R9 (HIGH, security) — Threat-model honesty (§9 additions).** State plainly: the
economic gate prices *identity creation* (one-time), NOT recurring same-identity
requests (per-cheap-doc — that's *why* Block exists); the inbound edge is
**permanent public chain metadata** Block can't retract; v1 is per-device so a
harasser reappears on un-blocked devices.

**R10 (HIGH, security) — Data-at-rest / residue (§9 checklist).** "Wiped with the
wallet" is overstated. Required: `blocked_contacts.sender_id` at-rest encryption
status named; **no `sender_id` in logs above debug**; FFI restore-buffer lifetime
bounded; and the sleeper — **the SwiftData container may be iCloud-backed**, so a
"local" block list can sync to iCloud backup. Confirm/exclude that.

**Cleanups (low-risk):** migration → fold `blocked_contacts` into **V001** (the
storage crate has no released consumers; the V002→V001 squash set the precedent);
`blocked`/`unblocked` extend the **existing** `on_persist_contacts_fn` callback,
not a new vtable slot; there is **no production SQLite reader** for the rejected
precedent (restore is FFI/Swift only) — match that, don't invent a loader; fix
`blocked_at_ms` (ms) vs SQL `unixepoch()` (seconds); broaden the self-block guard
to **any wallet-owned identity**, enforced at the `blocked_senders` insertion
boundary.

**Open decisions for you:**
- **Q1 (R2):** unblock **rewinds the high-water** (requests reappear; default) vs
  **doesn't** (unblock is "fresh start", UI says so)?
- **Q2 (R8):** blocked-then-established-via-other-device → **auto-lift block**
  (default) vs suppress-the-established-contact-while-blocked?

---

## 1. Problem & motivation

Today's "Reject / Decline" is keyed by `(sender_id, account_reference)` — it
suppresses **one specific request**. `contactRequest` documents are immutable
and never deleted on-chain, so the tombstone's real job is to stop that exact
request from re-appearing in the incoming list on every sync sweep.

It is **not** a block: a previously-rejected sender can re-request with a
bumped `accountReference` (a DIP-15 rotation) and the new request reaches the
user, because `is_request_rejected` matches only the exact reference. That is
deliberate (a sender who rotated keys should be able to reconnect), but it
means there is no way for a user to say *"I never want to hear from this
person again."*

This spec adds a **per-sender Block** alongside the per-request Decline.

### What Block can and cannot be

Block is necessarily a **local mute**, not protection:

- The chain has no block-list and does not stop anyone from *creating* a
  `contactRequest` addressed to your identity. Any block is a filter applied
  in your own wallet on read/ingest.
- So Block = "auto-hide every request from this sender id, regardless of
  `accountReference`, on this device." It stops the nagging; it does not stop
  the sender from writing documents.

We will be explicit about this in the UI copy (see §7) so it is not mistaken
for true protection.

## 2. Goals / non-goals

**Goals**
- A `block(sender_id)` action that suppresses **all** incoming requests from
  that sender — current and future, across rotations.
- An `unblock(sender_id)` that lifts it; the sender's still-on-chain requests
  reappear on the next sync.
- Durable across relaunch, on **both** persisters (SQLite + SwiftData), with
  restore-at-load — i.e. no resurrection bug (mirror the rejected-tombstone
  restore we just added).
- Correct wallet-wipe behaviour (no plaintext rows left on disk).
- UI to Block from an incoming-request row + a "Blocked" list to Unblock.

**Non-goals (v1)**
- Cross-device sync of the block list (see §6 — no on-chain home for it).
- Blocking an **established** contact (that is "remove + suppress" — a
  different flow; see §8).
- Any on-chain or consensus-level enforcement.

## 3. Semantics — Block vs Decline

| | **Decline** (exists) | **Block** (new) |
|---|---|---|
| Key | `(sender, accountReference)` | `sender` only |
| Suppresses | that one request | every request from the sender |
| Rotation (new ref) | gets through | stays suppressed |
| Intent | "dismiss this request" | "mute this person" |
| Reversible | n/a (a new ref is a new request) | yes — `unblock` |
| Scope | local | local |

Block **supersedes** Decline for a sender: once blocked, the per-reference
reject tombstones for that sender are redundant. On block we drop them (tidy;
the sender check covers everything). On unblock we do **not** restore them —
a fresh start, the user re-decides per request.

## 4. Architecture (Rust — `platform-wallet`)

### 4.1 State (`ManagedIdentity`)

Add, mirroring `rejected_contact_requests`:

```rust
/// Senders this identity has BLOCKED (G5 stage 3). Every incoming request
/// from a key in this map is suppressed regardless of accountReference —
/// the per-sender superset of `rejected_contact_requests`. Local-only.
pub blocked_senders: BTreeMap<Identifier, BlockedContact>,
```

```rust
pub struct BlockedContact {
    pub owner_id: Identifier,
    pub sender_id: Identifier,
    pub blocked_at_ms: u64, // local wall-clock; UI "blocked on …", not consensus
}
```

Accessor: `is_sender_blocked(&self, sender_id) -> bool`.

### 4.2 Ingest suppression

In `sync_contact_requests`, the received-doc loop already calls
`is_request_rejected(sender, ref)`. Add a **sender-level** gate *before* it:

```rust
if managed.is_sender_blocked(&sender_id) { continue; } // drop silently
```

This covers rotations automatically (no `accountReference` in the check) and
short-circuits both new-request and rotation handling.

### 4.3 Operations + changeset

Extend `ContactChangeSet` with two fields:

```rust
pub blocked:   BTreeMap<Identifier, BlockedContact>, // upserts
pub unblocked: BTreeSet<Identifier>,                 // tombstones
```

`block_sender(&mut self, sender_id, persister) -> ContactChangeSet`:
1. `blocked_senders.insert(sender_id, BlockedContact{…})`.
2. `incoming_contact_requests.remove(sender_id)` + emit `removed_incoming`
   (so the persisted `state='received'` row is DELETEd on **both** backends —
   the exact two-backend consistency lesson from the reject fix).
3. Drop any `rejected_contact_requests` entries for that sender (and emit the
   matching `removed`/no-op — they become redundant under the sender block).
4. `cs.blocked.insert(sender_id, …)`.

`unblock_sender(&mut self, sender_id, persister) -> ContactChangeSet`:
1. `blocked_senders.remove(sender_id)`.
2. `cs.unblocked.insert(sender_id)`.
3. Next sync re-ingests the sender's on-chain requests as fresh incoming.

Both go through the persister exactly like reject. Failures **propagate** (the
reject path's C1 lesson: a swallowed block-persist would silently un-block on
restart).

## 5. Persistence (both backends + restore)

### 5.1 Rust SQLite (`platform-wallet-storage`)

New table:

```sql
CREATE TABLE blocked_contacts (
    wallet_id  BLOB NOT NULL,
    owner_id   BLOB NOT NULL,
    sender_id  BLOB NOT NULL,
    blocked_at INTEGER NOT NULL DEFAULT (unixepoch()),
    PRIMARY KEY (wallet_id, owner_id, sender_id),
    FOREIGN KEY (wallet_id) REFERENCES wallet_metadata(wallet_id) ON DELETE CASCADE
);
```

Writer: `cs.blocked` → upsert; `cs.unblocked` → `DELETE`. Loader rehydrates
`blocked_senders`. **Migration placement is an open decision** (§12.a): fold
into `V001` (consistent with the recent V002→V001 squash, if the crate is
still pre-release) or add `V002`.

### 5.2 SwiftData (example app)

- New model `PersistentDashpayBlockedContact` (mirror of the rejected model):
  `(networkRaw, ownerIdentityId, senderIdentityId)` unique, cascade-owned by
  `PersistentIdentity` via a new `dashpayBlockedContacts` relationship.
- `persistContacts` upserts on `cs.blocked`, deletes on `cs.unblocked`.
- **Restore at load:** add a `blocked` FFI array on `IdentityRestoreEntryFFI`
  (reuse the `ContactRequestRejectionFFI` plumbing pattern) +
  `restore_dashpay_blocked` → rebuilds `blocked_senders`. Without this the
  block resurrects-as-unblocked on relaunch (the bug class we just fixed).
- **Wallet-wipe PHASE 1:** add `dashpayBlockedContacts` to the pre-delete
  loop — it has a non-optional `owner`, so omitting it re-introduces the
  mid-wipe fatal we just fixed.
- **Storage Explorer:** add the model to all three explorer views (the
  `check-storage-explorer.sh` CI gate requires it).

## 5.5 Fetch model, spam & DoS (the part that actually matters)

The received-request query today is, every sweep:

```rust
where toUserId == me, order_by $createdAt, limit: 100, start: None
```

`start: None` ⇒ **non-incremental**: we re-fetch the same first page from the
beginning each sweep and re-verify its proofs. Two consequences:

- **Truncation:** beyond 100 requests we never paginate forward — a flood of
  ≥100 junk requests can **bury** legitimate ones so they're never fetched.
- **Repeated cost:** every sweep pays fetch + GroveDB proof-verify for the
  whole page again.

**Threat: a sender (or a funded Sybil swarm) creates many requests.** Invalid
ones are the worst — they fail parse/validation but still cost fetch +
proof-verify + parse each sweep. The only built-in deterrent is **economic**:
each `contactRequest` document costs the sender platform credits. Spam isn't
free, but it isn't prevented.

**What Block can and cannot do here.** It CANNOT cut fetch cost: the index is
keyed by recipient (`toUserId == me`), there is no `sender NOT IN (…)` index,
and Sybil senders are unpredictable — so block is a **local read-filter after
fetch**, full stop.

**The real lever — incremental fetch (high-water).** Track the high-water
`$createdAt` (or core height) of the newest request seen per identity and
query `WHERE toUserId == me AND $createdAt > high_water`, paginating forward.
Then:

- each request is fetched **exactly once**, never re-fetched;
- pagination goes *forward*, so legit requests can't be buried past 100;
- block/decline become **one-time-on-first-sight** — suppress once, never see
  it again (no re-suppress-every-sweep).

This is the actual DoS/cost mitigation and is **worth doing independently of
Block.** It bounds steady-state work to O(new requests per sweep). It does not
stop the *first* fetch of a request from a new sender (impossible without
server-side sender exclusion), but nothing in the protocol can.

> **Scope note (sequencing decided):** incremental fetch touches
> `sync_contact_requests` and the received query, and changes reject/block from
> "re-suppress each sweep" to "suppress once." It is **Phase 2** (after
> single-device Block — see §12). Block works correctly without it (it just
> re-suppresses each sweep until Phase 2 lands); Phase 2 is the efficiency win.

## 6. Cross-device sync — local v1, self-encrypted blocklist as the real fix

Today reject **and** block are per-device local state. `displayHidden`
(contactInfo) syncs, but only for **established** contacts gated by the
≥2-contacts privacy rule — a blocked *sender* is not an established contact,
so no existing document carries it.

**v1: local-only** (simplest, ship-able). You re-block per device.

**v2 (recommended target): a self-encrypted owner-private blocklist
document.** One document the owner encrypts to themselves (same key family as
contactInfo `privateData`, but a single owner-scoped list, not per-contact and
not gated by the 2-contact rule). Every device reads + applies it, so block
(and optionally decline) apply everywhere. Costs: each edit is a document
write (credits); it reveals encrypted *that* a blocklist exists, never its
contents. This is the honest fix for "I shouldn't have to block on every
device" — promoted from "future" to a v2 decision (§12.b), because the user
explicitly wants cross-device behaviour.

## 7. UI (example app)

- **Incoming-request row** (`ContactRequestsView`): add **Block** beside
  Accept / Decline, behind a confirm ("Block <id>? You won't see future
  requests from them on this device. This doesn't stop them on-chain.").
- **Blocked list:** a screen listing `blocked_senders` with **Unblock**.
- Optimistic overlay identical to accept/reject (in-flight set + error row).

## 8. Edge cases & interactions

- **Rotation:** blocked sender bumps `accountReference` → still suppressed
  (per-sender check). ✔ the whole point.
- **Block then unblock:** on unblock, the sender's on-chain requests reappear
  on next sync as fresh incoming (we do not restore old reject tombstones).
- **Established contact:** Block is offered only on incoming-request rows in
  v1. Blocking someone you're already contacts with = "remove contact +
  suppress" — a distinct flow (delete the `EstablishedContact`, its accounts,
  then add the block). Deferred; called out so the UI doesn't offer Block on
  established rows yet.
- **Self-block:** guard against blocking your own identity id (no-op + log).
- **Decline + Block ordering:** Block supersedes; declining first then
  blocking is fine (block drops the tombstone).

## 9. Security / abuse / limits

- Block is a **local mute**, not protection — say so in the UI. A determined
  sender keeps writing on-chain docs; we just never surface them.
- **Storage growth:** the block list grows with user action only (bounded,
  benign). No index needed — same access pattern as contacts (load per
  owner, filter in memory); no query filters by `sender_id` alone beyond the
  PRIMARY KEY.
- **Privacy:** the block list reveals who you blocked; it is local + wiped
  with the wallet (hence the PHASE 1 requirement).

## 10. Test / verification plan

Rust (`platform-wallet`):
- `block_sender_suppresses_all_references` — block, then ingest a request with
  a *different* accountReference → not ingested (rotation can't bypass).
- `block_emits_removed_incoming` — block drops the pending row on both
  backends (red→green like the reject fix).
- `unblock_then_sync_reingests` — after unblock, the sender's request ingests
  again.
- `block_supersedes_reject_tombstones` — blocking clears per-ref tombstones.

`platform-wallet-storage`:
- `blocked_contacts` round-trip + loader rehydrates `blocked_senders`.
- migration-applies test (whichever placement §12.a lands on).

`platform-wallet-ffi`:
- `restore_blocked_rows_rebuilds_block_set` (mirror the rejected-restore test).

Swift / example app:
- wipe test: a wallet with a blocked contact wipes cleanly (no fatal).
- `check-storage-explorer.sh` passes (model in all three views).
- UAT: block a sender → relaunch → still blocked; unblock → request returns.

## 11. DashPay data-contract improvements (network-governance track)

These enable the features above more efficiently, but they are **changes to the
registered `dashpay` data contract** — a system contract on-chain. Adding an
index or a document type is a **contract update / DIP-level coordination**, not
a wallet-side change. So this track is separate from the wallet phases; the
wallet ships what works on today's contract (Phase 0), and these are proposals
for the contract maintainers. Each added index also makes **every**
`contactRequest` write a bit more expensive (more index trees to maintain) —
a system-wide cost borne at document creation, so additions must earn their keep.

### 11.0 What needs NO contract change

**Incremental fetch (Phase 0) is free.** The existing `userIdCreatedAt`
index `[toUserId, $createdAt]` already serves `WHERE toUserId == me AND
$createdAt > high_water` (range-after-equality). Ship it on today's contract.
*Don't* add an index for this.

### 11.1 Countable index → O(1) counts + spam detection (GROUP BY sender)

Today, to know "how many pending requests do I have" or "is one sender
flooding me", we must **fetch the documents**. Platform's count/group-by
queries (see `book/src/drive/count-index-group-by-examples.md`) answer those
from a proof **without enumerating documents** — but only over a `countable`
index.

Proposal: a new **countable** index on the recipient→sender axis:

```
byRecipientSender = [{ toUserId: asc }, { $ownerId: asc }]   // countable
```

(`$ownerId` of a `contactRequest` *is* the sender; the existing `ownerIdUserId`
is the reverse order and can't serve a `toUserId ==`-prefixed group-by.)

Unlocks, all as O(1)/O(log n) **count proofs, no doc fetch**:
- `COUNT(*) WHERE toUserId == me` → a pending-request **badge** for free.
- `GROUP BY $ownerId` → **requests-per-sender**; with a `HAVING count > N`-style
  threshold, cheaply **flag a spammer** (a sender who created many requests to
  you) and auto-suggest Block — *before* paying to fetch their docs.

Caveat: GROUP BY returns **counts, not documents**. It tells you *who* and *how
many*, not the request contents — you still fetch (incrementally) the ones you
want to act on. So it's a detection/triage layer on top of Phase 0, not a
replacement for fetch.

### 11.2 Cross-device block — reuse `contactInfo` `privateData` (no new doc type, no contract change)

Cross-device is **out of scope for v1** (single-device, decided). When we do
it, the chosen mechanism is **not** a new document type — it **reuses the
`contactInfo` we already have**:

- `contactInfo.privateData` is an **opaque encrypted blob at the contract
  level** (the contract only enforces 48–2048 bytes). The CBOR array inside —
  `[aliasName, note, displayHidden]` — is **our client convention**, so adding
  a positional element (e.g. `relationshipState: active|declined|blocked`, or a
  bare `blocked: bool`) is a **client-side change with NO contract update**.
- `displayHidden` is already the precedent: a per-contact, self-encrypted,
  cross-device-syncing hide flag. Block is the same idea, generalised.
- It rides the codec + sync + restore we already built (`fetch_decrypted_contact_infos`,
  `encode/decode_private_data`), so a device applies blocks during the existing
  contactInfo sweep.

Two things to resolve before building it (tracked, not v1):

1. **Non-established targets.** `contactInfo` is created today only for
   *established* contacts. Blocking a non-contact means creating a `contactInfo`
   *about* a non-contact. The contract allows it (no link to a `contactRequest`),
   but it interacts with the **≥2-contacts privacy gate** and doc-existence
   metadata — needs a privacy pass (does an extra `contactInfo` for a
   non-contact leak anything an observer can use? `encToUserId` is encrypted, so
   *who* is hidden; the *count* is not).
2. **Block vs Decline granularity.** Sync the deliberate **Block** (rare); keep
   ephemeral **Decline** local (frequent — a doc write per decline is too
   noisy). For *established* contacts, `displayHidden` already covers
   cross-device hide.

A standalone `contactBlock` document type is the **fallback** only if reusing
`contactInfo` runs into the privacy gate — and it would then be a real contract
change (§11.3). Reuse is strictly cheaper, so it's the primary plan.

### 11.3 Versioning / rollout reality

Two tiers, sequenced honestly:

- **Client-only, no contract change:** incremental fetch (§11.0, the existing
  index serves it) and the `contactInfo.privateData` block flag (§11.2, opaque
  blob). These ship through the normal wallet path.
- **Contract update (DIP / maintainer coordination):** the countable
  `[toUserId, $ownerId]` index (§11.1) and any *query-level* filter-out-blocked
  / standalone blocklist doc. `dashpay-contract` is registered on-chain, so
  these affect the whole network + need backward-compat for existing documents.
  **TODO / later track**, proposed separately from this wallet work.

## 12. Roadmap (decided) + remaining open questions

**Decided sequencing:**
1. **Phase 1 — single-device Block** (this spec → multi-agent review → implement).
   Local per-sender suppression, both persisters + restore + wipe + explorer + UI.
   No cross-device.
2. **Phase 2 — incremental fetch** (high-water `$createdAt`; client-only, no
   contract change). Bounds steady-state fetch + stops the ≥100 burying.
3. **Later / TODO (contract track):** real query-level DoS protection — filter
   blocked/rejected senders out *before* fetching. Requires a contract change
   → DIP / maintainer coordination. (NB: the once-proposed countable
   `[toUserId, $ownerId]` index is **struck** per R6 — its public count proof
   leaks the inbound social graph; at most an aggregate `COUNT(*)` total.)
4. **Cross-device (later):** **per R1, NOT via `contactInfo`-reuse** (a
   `contactInfo` about a non-contact breaks DIP-15 unlinkability). If it ships,
   it is a **single owner-scoped, self-encrypted blocklist document** — a
   contract change on this later track, with the metadata-leak analysis done up
   front.
5. **TODO (contactInfo format — DashPay-wide, NOT block-specific):** migrate
   `contactInfo.privateData` from the CBOR-array encoding to the **DIP-15
   versioned-varint** format DIP-15 actually defines (`version` + aliasName +
   note + displayHidden + acceptedAccounts), and **reconcile the deployed
   contract's field description** (currently says "cbor") to match DIP-15.
   Rationale: DIP-15 is the interop authority and its description currently
   contradicts the contract's; **no client implements contactInfo yet**, so we
   can fix it cleanly now. The DIP-15 `version` field is the forward-compat
   lever — append-only fields behind a version bump, with **tolerant decoders**
   (read known fields, ignore trailing) so older readers don't break (a *strict*
   decoder would). Replaces our current `encode/decode_private_data` codec.
   Also fixes the internal doc inconsistency (`research/01` wrongly says "CBOR
   per DIP-0015"; the contract validates `privateData` by **length only** — its
   "array in cbor" description is advisory, not enforced — so we follow DIP-15
   varint, the authoritative format).
   **Verified 2026-06 against github.com/dashpay:** no client decodes
   `contactInfo.privateData` today — `android-dashpay` has no `ContactInfo`
   class (the schema is bundled as JSON only; its Kotlin handles `contactRequest`),
   and `dash-wallet` has only a `// TODO` comment. So we have **full format
   freedom now**; the window to align to DIP-15 is *before* `dash-wallet`
   implements its TODO (it will follow DIP-15 = varint, not our CBOR → otherwise
   the two won't interop). Format discipline once readers exist: **version =
   breaking changes only; for additive changes do NOT bump — append + tolerant
   decode** so older readers ignore trailing fields.

**Remaining open questions for the Phase-1 review:**
- a. **Migration placement** — fold `blocked_contacts` into `V001` (matches the
  recent V002→V001 squash) or a new `V002`? Pending whether
  `platform-wallet-storage` is considered released.
- c. **State shape** — `BTreeMap<Identifier, BlockedContact>` (carries
  `blocked_at` for UI) vs bare `BTreeSet<Identifier>`. Recommend the map.
- d. **Established-contact block** — confirmed **out of v1** ("remove + suppress"
  is a separate flow).
- e. **Naming** — "Block" vs "Decline" (vs "Ignore"); UI copy must make the
  local-mute nature explicit.
- g. **Privacy of a `contactInfo` for a non-contact** (Phase-4 prerequisite) —
  does it leak anything? Resolve before cross-device.
