# Time-Range Index TTL

Architecture reference for the `ttl` key of `timeRange` indexes: what it
means, how expired windows are drained, and the invariants every walker
shares. The storage primitive underneath is grovedb's flat-subtree drop
([dashpay/grovedb#848](https://github.com/dashpay/grovedb/issues/848),
landed in grovedb PR #849); see
[the storage section](#grovedb-dependency-flat-subtree-drop).

## Motivation

A `timeRange` index stores every document once per containing window, and
a ranked one additionally rewrites a per-window secondary on every write.
All of those bytes are billed as **storage** — a price that prepays
~perpetual retention through the epoch-distribution model — even though
windowed data is intrinsically ephemeral: a "posts liked this hour"
bucket is worthless once the trending surface has moved past it. The
result is that the flagship use case (likes feeding a trending index)
pays perpetuity prices for state with a useful life measured in days,
multiplied by the grid's overlap factor.

Nobody cleans this up, either. Deletion costs the deleter processing,
refunds accrue to owners who have no reason to come back for entries this
small, and the state lingers forever.

## Semantics

A `timeRange` index may declare a **time to live**:

```json
"timeRange": { "on": "$createdAt", "range": 3600, "step": 3600, "ttl": 604800 }
```

In one paragraph: entries under this index exist for at most `ttl`
seconds past their bucket's start, plus a bounded drainage lag. Expired
buckets are drained **lazily, on write**: every state transition that
writes into the index continues draining the oldest expired bucket,
deepest-first, under a per-write operation budget. A fully drained
window is provably absent, exactly like a window that never held
documents; during the drainage lag an expired-but-not-yet-drained window
can still serve its remaining contents to an absolute (`byStart`) query
— correct answers about current state, within the "at most `ttl` plus
lag" lifetime. Everything written under the index's grid-qualified level
bills as **processing, not storage** — including the transitional bytes
— at an ephemeral-bytes rate.

### Why the fee reclassification is honest, not a subsidy

Storage fees prepay retention distributed across future epochs — decades
of it. A byte that provably lives at most one week consumes on the order
of **1/2,600th** of that retention. The real resource cost of a TTL'd
write is compute and write amplification (already processing) plus a
week of disk occupancy, which a flat per-byte processing surcharge covers
safely *because `ttl` is capped*. Version 1 caps it at **one week**
(`SystemLimits::max_time_range_ttl_seconds = 604 800`).

The load-bearing simplification: **TTL'd subtrees never create
refundable storage.** No `StorageFlags`, no owner/epoch refund entries.
That single property pays off three times:

1. the fee reroute needs no refund-ledger reconciliation;
2. cleanup owes nobody anything;
3. deletion needs no byte metering for consensus — which is what makes
   O(1) bucket drops possible at all (see the grovedb dependency).

## Grammar and validation

- `ttl` is an optional key of the `timeRange` map, in seconds, parsed
  into the transform. It is **not part of the grid identity**:
  [`TimeRangeTransform::storage_key`] excludes it, so declaring or
  changing a TTL never forks the storage level, and query-side grid
  matching ([`TimeRangeGridSpec`]) continues to compare
  `(range, step, phase)` only.
- **`ttl ≥ range`.** `$createdAt` is consensus-assigned from block time,
  so writes only ever target windows containing *now*; this invariant
  guarantees no bucket that can still receive entries (or serve as the
  `oldest` selector's window) is ever dropped.
- **`ttl ≤ SystemLimits::max_time_range_ttl_seconds`** (one week in v1).
  The cap is what makes the flat ephemeral-byte rate safe.
- **One TTL per grid per field.** Two indexes bucketing the same field
  with the same grid share one storage level; a differing `ttl` would
  give the shared subtree two conflicting lifecycles. Rejected at
  contract validation.
- Composes with everything the grid already composes with: `countable`,
  the range axes, ranked levels below the bucket, `unique`
  (`range == step`, `$createdAt`), indexOnly document types.
  `preallocated` stays banned with `timeRange` for the pre-existing
  structural reason.

## Cleanup

**Trigger** — deterministic and write-amortized: **every write** into a
TTL'd index continues drainage of the oldest expired bucket (start
`< block_time − ttl`), deepest-first, spending at most
`SystemLimits::max_time_range_ttl_drop_operations_per_write` O(1) drop
operations and resuming exactly where the previous write's budget ran
out. When nothing is expired, the check is a single bounded range read.
The operation count of a full bucket scales with its distinct groups,
and write volume scales with group volume, so drainage keeps pace
roughly one window behind; after a quiet spell the backlog amortizes
across subsequent writes instead of dumping a week of demolition on the
first like after a lull.

**Residue** — an index that never receives another write keeps its final
`ttl` of buckets indefinitely. This is bounded garbage that owes nobody
a refund. If it ever matters, the backstop is an epoch-transition sweep
riding the existing scheduled-cleanup pattern
(`check_for_ended_vote_polls` / `clean_up_after_vote_polls_end`);
deliberately **out of scope for v1**.

**User deletes and updates of expired documents** — handled at
**full-path granularity**, because a bucket drains piecewise: an entry
whose bucket (or whose group's trees inside a standing bucket) the drain
already took is skipped as cleanly removed; one whose trees still stand
is removed normally, so a not-yet-drained expired bucket never carries
dangling references. Every check is deterministic — it reads consensus
state plus the carried `$createdAt` and block time. Writes never target
expired windows, so an update of a fully expired document simply leaves
it without entries under the TTL'd index.

**Per-index semantics** — TTL removes entries from *this index only*.
An indexOnly like whose windowed entries expire keeps counting in the
all-time ranked `byPost` and in `byLiker`; permanence lives where the
contract declares it. Ranked per-window secondaries die with their
bucket — which also caps live leaderboard state at ~`ttl / step` windows
per index.

## grovedb dependency: flat-subtree drop

Dropping a bucket must never put user-scaled work on the consensus path.
The primitive that landed (grovedb PR #849) is the **flat-subtree drop**:
O(1) consensus removal of a subtree *declared to contain no child
subtrees* — an ordinary parent-Merk element delete whose cost is
independent of the subtree's contents — staging a durable redo record
(atomically, outside the root hash) that names every storage prefix the
drop orphaned: the subtree's own and, for indexed primaries, its three
per-axis secondary prefixes. Reclamation is DB-level range tombstones,
drained by `GroveDb::flush_pending_prefix_drops` — idempotent,
crash-safe, snapshot-correct, and never part of consensus cost.

A time-range bucket is *not* flat, so the platform drains it
**deepest-first, one flat unit at a time** (`drain_expired_time_range_buckets`):

1. each group's `[0]` reference tree — flat by construction, and where
   the mass lives — is flat-dropped;
2. the emptied group value tree leaves through the flat drop — or, under
   a ranked (indexed-primary) property-name tree, through grovedb's
   dedicated indexed-tree delete, which mirrors the group out of the
   ranking secondary;
3. the drained property-name tree is flat-dropped (dooming its secondary
   prefixes when ranked);
4. the emptied bucket is flat-dropped.

Every step is O(1); the *number* of steps scales with the window's
distinct groups, and that count is what
`SystemLimits::max_time_range_ttl_drop_operations_per_write` bounds.
**Every write** into a TTL'd index continues drainage where the previous
budget stopped (when nothing is expired, the check is one bounded range
read); write volume scales with group volume, so drainage keeps pace
roughly one window behind. Between writes a bucket may stand partially
drained — within TTL semantics (entries live *at most* `ttl`) — and the
removal walkers handle those states at full-path granularity: a
document whose group the drain already took deletes as a clean skip,
one whose group still stands is removed normally.

The flat-drop path-reuse contract (never re-create a dropped path before
its record drains) holds by construction: bucket paths embed their
window start, and writes never target expired windows. The host side:
drive-abci calls `flush_pending_prefix_drops` after committing each
block's transaction and once at startup, completing reclamation a crash
may have interrupted.

## Fee mechanics

Write operations targeting a TTL'd index's subtrees are classified
**ephemeral**: the walkers route them into a separate operation batch
(`LowLevelDriveOperation::EphemeralGroveOperation`), applied after the
standing batch, whose captured cost is consumed on its own terms — added
bytes bill to **processing** at
`FeeStorageVersion::ttl_ephemeral_disk_usage_credit_per_byte`
(270 credits/byte, 1% of the storage rate, ~27× a pro-rata week of
retention) and the storage fee contribution is **zero**. The elements
carry no storage flags, so deletion — the TTL drain or a user delete of
a not-yet-expired document — is basic removal with no refund entries:
there is nothing to refund, which is also where TTL writers collectively
pre-pay the drainage described below. Cost estimation routes through the
same split, so estimated and actual fees stay in the same class.

Drainage itself and the walkers' TTL bookkeeping reads are **unbilled**:
their costs go to scratch accounting, never to the triggering user. That
is load-bearing for the `estimated >= actual` fee invariant — the
estimation dry run cannot read state and therefore cannot price
state-dependent drainage, so billing it only on execution would let a
transition pass validation and then overdraw on apply. The unbilled work
is bounded: a capped count of O(1) drop operations plus a handful of
bounded reads per write.

## Queries

Unchanged in shape. A **drained** window is a provable empty answer
through every surface (document, count/sum/avg, ranked, having-range).
Two documented consequences: on a TTL'd index, `byStart` addresses
historic windows *within the TTL horizon* — beyond it, absence is the
(correct, provable) eventual answer; and during the bounded drainage lag
an expired-but-standing window may still serve its remaining, possibly
partially drained contents. Those are correct, provable answers about
what is currently stored — TTL promises entries live *at most* `ttl`
plus the lag, not that they vanish at the horizon instant. The relative
selectors (`newest` / `oldest`) can never address an expired window at
all.

## Versioning

Everything rides the still-unreleased PV14 grammar: the `ttl` key joins
the meta-schema v3 `timeRange` map, the two limits join a new
`SystemLimits` version, and the ephemeral-bytes rate joins the PV14 fee
table (`FEE_VERSION3`, which keeps `fee_version_number: 1` — the number
tags the refund algorithm, which is unchanged). No migration story
exists or is needed.
