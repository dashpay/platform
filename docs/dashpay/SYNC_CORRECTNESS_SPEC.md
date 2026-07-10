# DashPay sync correctness — contact requests **and** profiles (mirror Android `PlatformSyncService`)

Status: **IMPLEMENTED (2026-06-18)** — both stages shipped on
`feat/dashpay-m1-sync-correctness` (PR #3841), 5-lens review (§9) folded in first.
Stage 1 = paginated retrieve-all + per-identity high-water cursor + 10-min overlap
at a 15s cadence (`network/contact_requests.rs`); stage 2 = id-keyed
`contact_profiles` cache for established + pending senders (`network/contact_info.rs`,
`accessors.rs`). Both are surfaced in the UI and **durably persisted** through the
changeset pipeline to *both* backends (SQLite persister + SwiftData); the high-water
cursor stays in-memory by design (a cold restore does one safe full re-fetch).
Owner: rs-sdk / platform-wallet
Priority: **FIRST** of the DashPay correctness track (ahead of the contactInfo
format migration and the ignore feature).

This spec covers **two consecutive stages of the same Android sync loop**:

| Stage | Android (`PlatformSyncService`) | Us before | Delivered |
|-------|--------------------------------|-----------|-----------|
| 1. Contact-request fetch | `updateContactRequests()` — incremental, paginated, high-water | present but **broken** (truncated at 100, no high-water) | **fixed** — retrieve-all + high-water cursor |
| 2. Contact-profile fetch | `updateContactProfiles(userIds)` — batch `whereIn $ownerId` | **absent** (synced only our *own* profile) | **added** — id-keyed cache, established + pending senders |

Neither is an optimization: stage 1 is a **correctness bug** (real requests are
permanently buried) and stage 2 is a **missing feature** (contacts have no name
or avatar in the UI). The Android wallet (`dash-wallet`, on `kotlin-platform`)
already does both; this spec mirrors that proven design. Delivered as **two
commits** (stage 1, then stage 2) on one branch.

---

## 1. Problem

### 1.1 Stage 1 — our contact-request fetch is wrong, not just slow

`packages/rs-sdk/src/platform/dashpay/contact_request_queries.rs`:

```rust
where toUserId == me, order_by $createdAt, limit: 100, start: None
```

`start: None` + a fixed `limit: 100`, re-run every sweep:

- **Re-fetches the first page from the beginning every sweep** — pays the full
  fetch + GroveDB proof-verify each time for data we already have.
- **Truncates at 100 and never paginates** — with ≥100 requests, newer (or, by
  `$createdAt asc`, older) legitimate requests are **never fetched**. A spammer
  (or a popular identity) **buries real requests permanently**.
- **No durable high-water / cursor** — no notion of "what's new since last sweep".

### 1.2 Stage 2 — contact-profile sync is entirely absent

`packages/rs-platform-wallet/.../network/profile.rs::sync_profiles` runs over
`identity_manager.all_identities()` — only **managed** identities (our own),
never contacts (`manager/accessors.rs:54`). So we publish and refresh **our own**
profile but **never fetch a contact's** displayName / avatar / publicMessage. The
UI shows only a raw identity id (or a local alias). Neither `EstablishedContact`
nor any incoming-request sender has a cached profile anywhere.

## 2. The reference — Android `PlatformSyncService`

Verified 2026-06 against `github.com/dashpay/dash-wallet` +
`github.com/dashpay/kotlin-platform` (the current JVM platform lib,
`org.dashj.platform:dash-sdk-*`; **not** the stale `android-dashpay`). One
re-entrancy-guarded ticker (`TickerFlow(15.seconds)`) runs, in order:

```
updateContactRequests()              // stage 1: incremental, paginated, high-water
  → discovers userIds (contacts + pending senders)
updateContactProfiles(userIds)       // stage 2: batch whereIn $ownerId, cache by userId
checkDatabaseIntegrity()/FixMissingProfiles()  // self-heal missing profiles
```

- Stage 1 high-water: `SELECT MAX(timestamp)` per direction; **10-min overlap
  rewind**; incremental `$createdAt > afterTime` + `startAfter` cursor +
  `limit(-1)` = retrieve-all.
- Stage 2 fetches profiles for the userIds drawn from contact-request rows
  (**including pending incoming senders** — that's how the request UI shows a
  requester's name/avatar), keyed in a `dashpay_profile` table by `userId`,
  independent of relationship state.

## 3. Goal

1. Make our **contact-request** sync incremental, fully-paginated,
   high-water-tracked, and skew-safe, for **both** directions — no truncation,
   each request fetched ~once, a flood can't bury anything.
2. Add **contact-profile** sync: fetch established contacts' **and pending
   incoming senders'** profiles in batches, cache them (id-keyed) so the UI shows
   name + avatar on both the contacts and the requests screens, refresh, and
   self-heal any missing profile without unbounded re-querying.

## 4. Design

### 4.1 High-water cursor (stage 1)

**Storage (resolves Q-a).** Android keeps every contact-request row and derives
`MAX(timestamp)`. Our model collapses requests, so we can't `MAX()` a raw table.
We persist **two scalar fields on `ManagedIdentity`** — `high_water_received_ms:
Option<u64>` and `high_water_sent_ms: Option<u64>` — riding the existing
`IdentityEntry` snapshot (changeset → both persisters → FFI restore), **not** a
separate table. Two integers per identity need no relational shape.

**The advance invariant (the heart of stage-1 correctness).** Get this wrong and
we reintroduce the burying bug. The cursor:

1. **Advances only on a fully-exhausted, error-free paginate** of that direction.
   "Exhausted" = a page returned `< limit` docs (possibly empty); a final page of
   exactly `limit` requires one more fetch to confirm. **Any** fetch/proof error
   mid-loop ⇒ **do not advance that direction's cursor this sweep** (leave it at
   the prior value; the overlap re-fetches next sweep).
2. **Advances to `max($createdAt)` over every doc *fetched* this sweep** —
   *including* docs that ingest then parse-skips, collapses
   (`newest_received_per_sender`), or suppresses (ignore/tombstone). The cursor
   records **fetch-completeness, not ingest-success**. Ignore `unwrap_or(0)`
   sentinels: advance to the max of *present* (`Some`) timestamps only, and never
   below the current value.
3. **Never stamps to wall-clock `now`.** On a zero-doc fetch the cursor is left
   unchanged. States: `Absent` ⇒ query `$createdAt > 0` (full); `Present(t)` ⇒
   query `$createdAt > (t − OVERLAP_MS)`.

**Why cursor-loss is safe (the written contract):** every collapsed / suppressed
doc is, by construction, deterministically reproducible from a full re-fetch of
the immutable on-chain set. So **under-shoot is free** (a lost/low cursor just
triggers one full re-fetch; ingest is a fixpoint) and **over-shoot buries**.
Therefore **restore tolerates only under-shoot**: on any restore-consistency
doubt, clamp the cursor to `min(persisted, max($createdAt) over restored contact
rows)`, or reset to `0`. A restored-too-high cursor is a correctness bug.

**`OVERLAP_MS` is correctness-load-bearing, not cosmetic.** The lower bound is
exclusive (`>`) and the `userIdCreatedAt` index is non-unique on `$createdAt`, so
multiple requests can share a `$createdAt` at a page boundary. The overlap is
what re-includes them; **`OVERLAP_MS = 0` is an invalid configuration**, not a
tuning knob. Default `10 * 60_000` (copy Android).

### 4.2 The request query (rs-sdk, stage 1)

`fetch_received_contact_requests` / `fetch_sent_contact_requests` gain
`after_created_at: Option<u64>` + cursor pagination:

```rust
where:    [ toUserId == me, $createdAt > (high_water − OVERLAP_MS) ]
order_by: $createdAt asc        // REQUIRED — binds the userIdCreatedAt index and
                                // avoids the "verified-absent" proof trap
start:    StartAfter(last_doc_id)   // ephemeral, per-loop pagination cursor
```

Two distinct cursors, do not conflate: within-sweep pagination uses
`Start::StartAfter(last_document_id)` (a 32-byte doc id, per-loop); the **durable
high-water** persists `max($createdAt)` (cross-sweep, §4.1). Loop pages until
exhausted (§4.1 rule 1). **Precondition (Q-c, stage 1):** before replacing the
working `limit:100` query, verify on testnet that the paginated `$createdAt > t`
+ `StartAfter` form returns a known existing doc (not a verified-absent empty
proof) — the current query's `order_by` comment documents this exact trap.

### 4.3 Request sweep flow (platform-wallet `sync_contact_requests`)

1. Read `high_water_received` / `high_water_sent` (Absent ⇒ full).
2. Fetch received `> (hw_received − OVERLAP)`, paginated; fetch sent likewise.
3. Ingest via the existing path — `newest_received_per_sender` collapse, ignore
   suppression, auto-establish. **Idempotency is load-bearing**: the overlap
   re-delivers seen docs every sweep, so ingest MUST be a fixpoint (it is).
4. **Per direction, iff its paginate exhausted without error** (§4.1 rule 1):
   advance the cursor to the max `$createdAt` *fetched* this sweep (§4.1 rule 2).
   On any error, skip the advance for that direction.

### 4.4 Contact-profile fetch (rs-sdk + platform-wallet, stage 2)

**Query (resolves Q-c stage 2 + Q-cap).** New `fetch_profiles_for(owner_ids)`:

```rust
where:    [ $ownerId In [id0, id1, …] ]   // ≤ IN_CAP ids per query
order_by: []                              // EMPTY — unique ownerId index, no trap
start:    None                            // each owner yields ≤1 profile; no pagination
```

The `profile` doctype has a **unique single-property `ownerId` index**, so an
`In $ownerId` set lookup proves presence/absence cleanly with **empty
`order_by`** and **no pagination** (mirrors the working `profile.rs` point query,
Equal→In). `IN_CAP = 100` is a **hard cap** enforced at query-build
(`rs-drive/src/query/conditions.rs:361`); the `In` array **rejects duplicates**
(`:368`) and **rejects empty** (`:355`). So the caller **dedups** the id set and
**skips** the query entirely when a chunk (or the whole target set) is empty.

**Target set (resolves the §4.3-vs-§4.4 contradiction): iterate the FULL set
every sweep, not "touched ids".** Each sweep, collect:
`{ established_contacts[].contact_identity_id } ∪ { incoming_contact_requests[].sender }`
across managed identities, **dedup**, and **skip ids that are themselves managed
identities on this wallet** (their profile is their own `dashpay_profile`, which
is authoritative — see §4.7). The stage-1 "touched this sweep" set is at most a
*fetch-these-first hint*, never the iteration set — the existing aggregator
discards `sync_contact_requests`'s return value anyway, and a "touched-only" set
would break both self-heal and first-run backfill (every pre-existing contact is
uncached but untouched).

**Filter, then chunk, then fetch:**

1. Drop ids that are **cached and fresh**, and ids that are **confirmed-absent
   and checked recently** (negative cache, see below). What remains is the fetch
   set — on first run after upgrade this is *every* contact (the dominant,
   expected first-sweep cost; bounded by contact count).
2. Chunk the remaining ids into groups of `IN_CAP`, run one `In` query per chunk.
3. **Per-chunk log-and-continue isolation:** a chunk's fetch/proof failure logs
   and continues to the next chunk; the freshness/checked markers advance **only
   for ids in successfully-fetched chunks**, never sweep-wide on partial failure.
   A persistently-failing chunk must not starve the others.

**Self-heal & the no-profile negative cache.** A contact may have **no `profile`
document on-platform** (profiles are optional). The `In` query simply omits them.
Without a guard, "cached? false" stays true forever and they're re-queried every
sweep — the unbounded-retry pathology `payment_channel_broken` (G1c) exists to
avoid. So record a **confirmed-absent marker with a checked-at timestamp**; the
fetch set targets "no cached profile **and** not checked within the backoff
window". Self-heal then *is* the normal path (an uncached/expired contact re-enters
the fetch set) — no separate `FixMissingProfiles` loop.

### 4.5 Profile storage — **Option B** (id-keyed cache)

A new map on `ManagedIdentity`:

```rust
pub contact_profiles: BTreeMap<Identifier, ContactProfileEntry>,
// ContactProfileEntry = { profile: Option<DashPayProfile>, checked_at_ms: u64 }
//   profile: Some(..) = fetched & present; None = confirmed-absent (negative cache)
//   checked_at_ms: last fetch attempt, drives the self-heal backoff
```

Chosen over a field on `EstablishedContact` because the cache must serve **every
relationship state** — established contacts, **pending incoming-request senders**
(requests screen), and **ignored senders** (future Ignored list) — none of which
share one struct. This is the product decision (§4.6) and matches Android's
relationship-independent `dashpay_profile` table. Plumbing (the `dashpay_payments`
5-site pattern; **the two most-forgotten are the merge rule and the store-side
apply** — miss either and contacts silently vanish on relaunch):

1. field on `ManagedIdentity`;
2. `IdentityEntry` field + `from_managed` (`changeset.rs`);
3. **merge rule** in `IdentityChangeSet::merge` — per-key last-write-wins (the
   `dashpay_payments` merge at `changeset.rs:489-495` is the template);
4. FFI: a **contact-keyed** accessor (distinct from the existing identity-keyed
   own-profile one), e.g. `platform_wallet_get_contact_profile(wallet,
   owner_identity_id, contact_identity_id) -> profile?`, + an
   `IdentityRestoreEntryFFI` field + `restore_contact_profiles` fn
   (mirror `restore_dashpay_payments`, `persistence.rs`);
5. SwiftData `PersistentDashpayProfile` keyed by `(ownerId, contactId)` (mirror
   `PersistentDashpayPayment`) + the **store-side write/apply**.

**Boundary invariant:** `contact_profiles` holds **only the five public profile
fields** parsed from the on-chain `profile` document. It must never receive any
field derived from the encrypted `contactInfo.privateData` path (which carries
private relationship state — alias/note/hidden/ignore). Keep these two stores
distinct so the contactInfo migration (Spec 1) can't accidentally cross them.

### 4.6 Scope & privacy

**Scope (product decision): established contacts + pending incoming-request
senders now; ignored senders ride the same cache when the Ignored list lands.**
This matches Android's observable behavior (requester names in the request UI).

**Privacy posture (resolves Q-scope).** Fetching a *pending* sender's profile is a
public read, but issuing `whereIn $ownerId [sender_ids]` right after their
requests land is a query-pattern an observer could correlate with your inbound
set. We **accept** this because the marginal leak is small: the contact-request
documents are *already public* (indexed by `[toUserId, $createdAt]`), and the
DAPI node serving our `toUserId == me` request query — which we must run —
**already learns the entire inbound set**. Fetching those public profiles adds
little. This is materially weaker than the R1 leak (which *creates a new on-chain
document* about a non-contact). Documented and accepted; the R1 track may later
minimize query-pattern metadata if desired.

### 4.7 Cache write semantics

- **Full-REPLACE, not merge.** A fetched profile document is the authoritative
  *complete* state for that owner; storing it **overwrites** the cached entry via
  `profile_from_properties` (full parse). This is the **opposite** of the
  own-profile *update* path (`merge_profile_properties`, read-modify-write) — do
  **not** reuse that helper here, or a contact who *removes* `avatarUrl` would
  keep showing a stale avatar forever.
- **All-empty parse ⇒ confirmed-absent, not cached-present.** A doc that parses to
  an all-`None` profile is treated as a negative-cache hit (§4.4), not a fresh
  empty profile, so self-heal keeps it honest.
- **Persist only on change.** Compare the fetched profile to the cached one before
  writing; emit no changeset when unchanged. This keeps the deferred-Q-inc
  "refetch-all each sweep" first cut a **persistence fixpoint** — no write
  amplification, the same discipline stage 1 enforces.
- **`avatarUrl` validation at insert.** Validate before caching: **`https://`
  scheme only**, length-capped (state the contract's max). Treat the cached url as
  **untrusted** input downstream — it is attacker-controlled and the UI will load
  it (an unsanitized `http:`/`file:`/`javascript:` url is an SSRF / tracking-pixel
  vector; a tracking url tied to your IP confirms "you have this contact").
- **own-vs-contact authority.** If a target id is itself a managed identity on this
  wallet, skip the contact fetch; that identity's own `dashpay_profile` wins.

### 4.8 Driver wiring (`dashpay_sync.rs`)

Add `sync_contact_profiles()` as a **distinct** step **between** the existing
`sync_profiles()` (own identities) and `sync_contact_infos()`. It is
**log-and-continue, not error-returning** (matches `sync_contact_infos` /
`reconcile_incoming_payments`): a contact-profile fetch failure degrades *display*
only and must never change the sweep's pass/fail outcome. **Do not** fold it into
`sync_profiles` — that function is scoped to `all_identities()` (own) and writes a
different store. Ordering: it must run **after** `sync_contact_requests` so a
contact established this sweep is fetched the same tick.

### 4.9 Interactions (specify, don't discover)

- **Un-ignore resync (deferred to the ignore refactor, but constrained here):**
  un-ignore must re-fetch the un-ignored sender's requests. The
  ignore/reject tombstone is keyed by `(sender, accountReference)` and **does not
  store `$createdAt`**, so a *precise* "rewind the cursor past their `$createdAt`"
  is **not implementable from the tombstone alone**. Therefore: **un-ignore ⇒
  clear (reset to Absent) the received cursor** → one full re-fetch (cheap, safe
  per §4.1). If a targeted rewind is ever wanted, add `$createdAt` to the tombstone
  first. The ignore work owns the call site; this is the mechanism constraint.
- **contactInfo-before-contactRequests ordering:** DIP-15 says fetch contactInfo
  first (so contacts don't flicker on `displayHidden`). Out of scope here; noted.
- **Cursor as at-rest metadata:** the high-water timestamps are derived from public
  on-chain `$createdAt`, but they are a session-activity residue at rest — exclude
  the cursor (and the whole DashPay store) from iCloud backup, since a device-local
  ignore/blocklist and activity residue should not sync to backup.

## 5. Non-goals

- Changing the sweep cadence.
- The **account** half of `checkDatabaseIntegrity` (we already rebuild contact
  accounts) — only the **profile** half is in scope (§4.4 self-heal).
- **Avatar image bytes / rendering** — we cache the fields (`avatarUrl` +
  hashes); downloading/showing the image is app-layer (but the url is validated
  at cache insert, §4.7).
- **Per-profile `$updatedAt`-incremental refetch (Q-inc).** The composite
  `$ownerId In […] AND $updatedAt > marker` is **not provable in one query** (an
  `In` on the first index field plus a range on the second isn't a contiguous
  index range). The first cut refetches all contact profiles each sweep (bounded
  by contact count, and a persistence fixpoint per §4.7); a real incremental would
  be per-owner equality (loses the batch) or client-side staleness — a follow-up.

## 6. Implementation surface

**Stage 1 (commit 1):**
- `rs-sdk/.../dashpay/contact_request_queries.rs` — `after_created_at` + the
  `StartAfter(doc_id)` pagination loop; drop `limit:100, start:None`.
- `platform-wallet/.../network/contact_requests.rs::sync_contact_requests` —
  read/advance cursors per §4.1/§4.3 (advance gated on exhaustion + no error).
- `ManagedIdentity` gains `high_water_received_ms` / `high_water_sent_ms`
  (`Option<u64>`) + `IdentityEntry` + merge + both persisters + FFI restore.

**Stage 2 (commit 2):**
- `rs-sdk/.../dashpay/` — `fetch_profiles_for(owner_ids)` (empty `order_by`, `In`,
  dedup, chunk at `IN_CAP=100`, skip-empty).
- `platform-wallet/.../network/profile.rs` — `sync_contact_profiles` (full-set
  target, negative cache, per-chunk isolation, full-replace, persist-on-change,
  `avatarUrl` validation); reuse `profile_from_properties`.
- `ManagedIdentity.contact_profiles` + the 5 plumbing sites (§4.5), incl. the
  contact-keyed FFI accessor + `PersistentDashpayProfile` SwiftData model.
- `dashpay_sync.rs` — wire `sync_contact_profiles` per §4.8.
- UI bind in the **real** consumers: `Views/DashPay/ContactsView.swift` (list
  row name/avatar), `ContactDetailView.swift` (header), `ContactRequestsView.swift`
  (requester name/avatar), via the existing `DashPayContactMeta` / `DashPayProfileView`.
  (There is **no** `FriendsView`.)

## 7. Test plan

**Stage 1:**
- **Incremental:** two sweeps; second issues `$createdAt > hw` and ingests only the
  delta (no re-fetch beyond the overlap).
- **No-bury:** 150 requests → all eventually fetched via pagination.
- **Equal-timestamp page boundary:** N>limit requests sharing one `$createdAt`
  straddling a page cut → all eventually ingested (pins the overlap as
  correctness, not just skew).
- **Partial-page failure:** inject a page-2 error → the cursor does **not** advance
  and the next sweep re-fetches from the old high-water.
- **Collapsed-doc reachability:** after a cursor wipe, an older-ref doc that was
  collapsed away reappears (proves cursor-loss safety / under-shoot).
- **Restore over-shoot guard:** a restored cursor higher than the restored contact
  rows still re-fetches the missing contacts (over-shoot clamped to under-shoot).
- **Idempotency:** overlap re-delivery creates no phantom rows / duplicate writes.

**Stage 2:**
- **Batch/chunk + dedup:** N>IN_CAP contacts (with a duplicate id) → ⌈N/IN_CAP⌉
  chunked queries, deduped, all cached.
- **First-run backfill:** a wallet restored with M established contacts and zero
  cached profiles fetches all M on the first sweep even though stage 1 ingests no
  new request.
- **Pending-sender profile:** a pending incoming-request sender's profile is
  fetched and reachable via the contact-keyed FFI accessor.
- **No-profile negative cache:** a contact with no on-platform profile is fetched
  at most once per backoff window, not every sweep.
- **Chunk isolation:** chunk 2 of 3 fails → chunks 1 & 3 cache, chunk 2's contacts
  retried next sweep (not marked done).
- **Shrinking profile (full-replace):** cache a full profile, then ingest a doc
  missing `avatarUrl` → cached `avatar_url` becomes `None`.
- **Persist-on-change fixpoint:** a steady-state sweep with unchanged profiles
  writes zero changesets.
- **avatarUrl validation:** a profile with a non-`https` url is rejected/sanitized
  at cache insert.
- **own-vs-contact:** a contact that is also a managed identity resolves to the
  own `dashpay_profile`, not a duplicate contact fetch.
- **Round-trip:** a contact profile survives relaunch (changeset → persister →
  restore), like `dashpay_payments`.

## 8. Open questions (most resolved by the review)

- **Resolved — Q-a** (cursor storage): two scalar `Option<u64>` fields on
  `ManagedIdentity` (not a table).
- **Resolved — Q-store:** Option B (id-keyed `contact_profiles`), per the
  product decision (§4.5/§4.6).
- **Resolved — Q-scope:** established + pending senders; privacy accepted (§4.6).
- **Resolved — Q-c:** stage-1 keeps `order_by $createdAt`; stage-2 uses empty
  `order_by` on the unique `ownerId` index, no pagination. (Stage-1 paginated
  form still needs the one-time testnet proof check, §4.2.)
- **Resolved — Q-cap:** `IN_CAP = 100`, dedup, skip-empty.
- **Resolved — Q-inc:** not provable as a single batch query; deferred (§5).
- **Open — Q-b:** `OVERLAP_MS = 10 min` (copy Android) — keep, but confirm it
  comfortably exceeds observed platform time-skew; **must stay > 0** (§4.1).
- **Open — Q-backoff:** the no-profile negative-cache recheck interval (§4.4) —
  propose "once per N sweeps" or a wall-clock window; pick during impl.
- **Open — Q-checked-clock:** the `checked_at_ms` backoff may use wall-clock
  (acceptable — it gates re-query cost, not cursor correctness) vs a sweep
  counter; decide during impl.

## 9. Review resolutions (traceability)

Folded in from the 5-lens review (feasibility / scope / adversarial / security /
flow). The load-bearing changes vs the first draft:

- **Cursor advance invariant rewritten** (§4.1) — advance only on error-free
  *exhausted* pagination, over docs *fetched* (not *applied*), never wall-clock,
  under-shoot-only on restore, overlap mandatory. Closes the two CRITICAL burying
  holes (advance-past-failed-page, advance-past-collapsed-doc).
- **Cursor storage simplified** to two scalar fields, not a table (Q-a).
- **Stage-2 query shape resolved from the contract indices** (§4.4) — unique
  `ownerId` index ⇒ empty `order_by`, no pagination, `IN_CAP=100`, dedup,
  skip-empty (Q-c, Q-cap). Q-inc shown unprovable as a batch.
- **Stage-2 negative cache + per-chunk isolation + full-replace +
  persist-on-change** added (§4.4/§4.7) — closes infinite-refetch, partial-failure
  starvation, stale-field, and write-amplification holes.
- **Target set = full set (established + pending), every sweep** (§4.4) — closes
  the §4.3-vs-§4.4 contradiction and the first-run-backfill gap.
- **Storage = Option B** with the full 5-site plumbing called out (merge rule +
  store-apply emphasized), public-data boundary (§4.5).
- **avatarUrl validation** + **privacy posture for pending-sender fetch** (§4.6/4.7).
- **Driver hook pinned** as a distinct log-and-continue step (§4.8); **UI surface
  corrected** to the real views (no `FriendsView`).
- **Un-ignore = clear-cursor** because the tombstone lacks `$createdAt` (§4.9).
