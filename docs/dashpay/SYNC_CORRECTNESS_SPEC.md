# DashPay contact-request sync — incremental, paginated, skew-safe (mirror Android)

Status: **DRAFT** (awaiting multi-agent spec review before implementation)
Owner: rs-sdk / platform-wallet
Priority: **FIRST** of the DashPay-privacy/correctness track (ahead of the
contactInfo format migration and Block).

This is **not** an optimization — our current fetch is a **correctness bug**, and
the reference Android wallet (`dash-wallet`) already does it the right way. This
spec mirrors that proven design.

---

## 1. Problem — our fetch is wrong, not just slow

`packages/rs-sdk/src/platform/dashpay/contact_request_queries.rs::fetch_received_contact_requests`:

```rust
where toUserId == me, order_by $createdAt, limit: 100, start: None
```

`start: None` + fixed `limit: 100`, re-run every ~60 s by `DashPaySyncManager`, means:

- **Re-fetches the same first page from the beginning every sweep** — pays the
  full fetch + GroveDB proof-verify each time for data we already have.
- **Truncates at 100 and never paginates** — with ≥100 requests, newer (or, by
  `$createdAt asc`, older) legitimate requests are **never fetched**. A spammer (or
  just a popular identity) **buries real requests permanently**.
- **No durable high-water / cursor** — no notion of "what's new since last sweep."

## 2. The reference — Android `dash-wallet` does it correctly

Verified 2026-06 against `github.com/dashpay/dash-wallet` (the Android app) +
`github.com/dashpay/kotlin-platform` (the *current* JVM platform lib —
`org.dashj.platform:dash-sdk-*`, which dash-wallet depends on; **not** the stale
`android-dashpay`, last pushed 2024-01). The `ContactRequests.get` query is
identical in both, so the design is long-standing/stable:

- **`PlatformSyncService.kt`** — `TickerFlow(UPDATE_TIMER_DELAY = 15.seconds)` →
  `updateContactRequests()`, re-entrancy-guarded (`updatingContacts` AtomicBoolean).
- **High-water from the local store** (`DashPayContactRequestDao.kt:50-54`):
  ```sql
  SELECT MAX(timestamp) FROM dashpay_contact_request WHERE toUserId = :userId  -- received
  SELECT MAX(timestamp) FROM dashpay_contact_request WHERE userId   = :userId  -- sent
  ```
- **10-minute overlap rewind** for clock-skew safety (`PlatformSyncService.kt:351-368`):
  `if (lastTs < now - 10min) lastTs else lastTs - 10min` — re-fetch the last 10 min
  so a request whose `$createdAt` is slightly behind real arrival isn't missed.
- **Incremental + fully-paginated fetch** both directions
  (`ContactRequests.kt:98-130`, `PlatformSyncService.kt:101-147`):
  ```kotlin
  documentQuery.where("$createdAt", ">", afterTime)
               .orderBy("$createdAt", true)
               .startAfter(startAfter)        // cursor
  limit = if (retrieveAll) -1 else DOCUMENT_LIMIT   // retrieveAll => paginate ALL
  ```
- Then `updateContactProfiles(newUserIds)` + a `checkDatabaseIntegrity` /
  `FixMissingProfiles` completeness pass.

## 3. Goal

Make our contact-request sync **incremental, fully-paginated, high-water-tracked,
and skew-safe**, for **both** directions (received + sent), matching the Android
design. No truncation; each request fetched ~once; a flood can't bury anything.

## 4. Design

### 4.1 Durable high-water (the one model difference from Android)

Android keeps **every** contact request row in `dashpay_contact_request` and
derives `MAX(timestamp)` from it. **Our model collapses** requests into
`established_contacts` / `incoming_contact_requests` / tombstones, so we can't
reliably `MAX()` over a raw-request table. Therefore we persist a **dedicated
per-identity, per-direction high-water cursor**:

```
dashpay_sync_cursor(wallet_id, owner_id, direction, last_created_at_ms)
```

(direction: 0 = received `toUserId==me`, 1 = sent `$ownerId==me`). Updated at the
end of each sweep to the max `$createdAt` actually ingested. Restored at load
(same restore discipline as contacts/payments/tombstones — a lost cursor just
means a one-time full re-fetch, not a correctness break). SwiftData mirror +
FFI-restore parallel to the existing arrays.

### 4.2 The query (rs-sdk)

`fetch_received_contact_requests` / `fetch_sent_contact_requests` gain an
`after_created_at: Option<u64>` + cursor pagination:

```rust
where: [ toUserId == me, $createdAt > (high_water - OVERLAP_MS) ]
order_by: $createdAt asc
start: After(last_doc_cursor)   // paginate until a short page (< limit) is returned
```

Loop pages (`start_after` the last doc id) until exhausted — **retrieve all**, not
first-100-and-stop. `OVERLAP_MS = 10 * 60_000` (copy Android's window; tunable).

### 4.3 Sweep flow (platform-wallet `sync_contact_requests`)

1. Read `high_water_received` / `high_water_sent` (cursor table; `0`/None ⇒ full).
2. Fetch received `> (hw_received − OVERLAP)`, paginated; fetch sent likewise.
3. Ingest via the existing path — `newest_received_per_sender` collapse, rejected/
   blocked suppression, auto-establish. **Idempotency is load-bearing**: the
   10-min overlap re-delivers already-seen docs every sweep, so ingest MUST be a
   fixpoint (it already is — that's the M1/review work).
4. Advance each cursor to the max `$createdAt` ingested this sweep.

### 4.4 Interactions (must be specified, not discovered)

- **Reject/Block tombstones:** an overlap re-fetch re-delivers a rejected request;
  `is_request_rejected` / `is_sender_blocked` must still suppress it (they do).
- **Unblock resync (Q1 from BLOCK_SPEC):** unblock must **rewind the received
  cursor** for that sender's `$createdAt` (or clear the cursor) so their on-chain
  requests refetch — otherwise the high-water has already passed them and unblock
  silently does nothing. This spec is the home for that mechanism.
- **contactInfo-before-contactRequests ordering:** DIP-15 §"Fetching Contact Info"
  says fetch contactInfo first (so contacts don't pop in/out on `displayHidden`).
  Out of scope here (we already sync contactInfo in a later step), but note the
  ordering for when both are incremental.

## 5. Non-goals

- Profile-sync incrementality + the `checkDatabaseIntegrity` self-healing pass
  (Android has both) — worth a follow-up, not this spec.
- Changing cadence (60 s is fine).

## 6. Implementation surface

- `packages/rs-sdk/src/platform/dashpay/contact_request_queries.rs` — `after_created_at`
  + cursor pagination loop; drop the bare `limit:100, start:None`.
- `packages/rs-platform-wallet/.../network/contact_requests.rs::sync_contact_requests`
  — read/advance cursors; pass the high-water.
- New cursor state on `ManagedIdentity` + changeset + both persisters + FFI restore
  (mirror the rejected-tombstone plumbing).
- `unblock` cursor-rewind hook (ties to BLOCK_SPEC Q1).

## 7. Test plan

- **Incremental:** two sweeps; second issues a `$createdAt > hw` query and ingests
  only the delta (assert no re-fetch of old docs beyond the overlap).
- **No-bury:** 150 requests; assert all are eventually fetched via pagination
  (today's `limit:100` drops 50).
- **Skew window:** a request with `$createdAt` just below the prior high-water is
  still fetched (the 10-min overlap).
- **Idempotency:** the overlap re-delivery does not create phantom rows / duplicate
  changeset writes (fixpoint).
- **Cursor restore:** cursor survives relaunch; a wiped cursor triggers exactly one
  full re-fetch then resumes incremental.

## 8. Open questions

- **Q-a:** cursor stored as dedicated state (recommended) vs computed `MAX($createdAt)`
  over a retained raw-request table (the Android shape — bigger model change).
- **Q-b:** `OVERLAP_MS` = 10 min (copy Android) vs derive from observed platform
  time-skew bounds.
- **Q-c:** does `start: After(docId)` pagination interact with the "verified
  absence proof" trap we hit before (the `ORDER BY $createdAt` requirement)? Verify
  the paginated query still binds to the `userIdCreatedAt` index and proves cleanly.
