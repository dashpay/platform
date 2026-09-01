# Time-Range Index TTL

Design document. Status: **accepted, in implementation** — platform side
first, against the grovedb primitive specified in
[dashpay/grovedb#848](https://github.com/dashpay/grovedb/issues/848)
(placeholder-implemented until it lands; see
[the dependency section](#grovedb-dependency-detach-and-sweep)).

## Problem

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

## Proposal

A `timeRange` index may declare a **time to live**:

```json
"timeRange": { "on": "$createdAt", "range": 3600, "step": 3600, "ttl": 604800 }
```

Semantics, in one paragraph: entries under this index exist for at most
`ttl` seconds past their bucket's start. Everything written under the
index's grid-qualified level is billed as **processing, not storage** —
including the transitional bytes — at an ephemeral-bytes rate. Expired
buckets are dropped **lazily, on write**: the state transition whose
document creates a *new* bucket also drops up to a capped number of
buckets whose start has fallen behind `block_time − ttl`. Nothing about
the query surface changes: an expired window is provably absent, exactly
like a window that never held documents.

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

**Trigger** — deterministic and write-amortized: when the insert walker
creates a bucket value tree that did not exist before (it already knows —
the tree-insert reports whether it inserted), and the transform declares
a TTL, the same batch drops expired buckets: children of the grid level
whose bucket start is `< block_time − ttl`, oldest first, **capped at
`SystemLimits::max_time_range_expired_bucket_drops_per_write` per
triggering write**.

Steady state is one-for-one: one new bucket per `step` means one bucket
crossing the horizon per `step`, so the triggering writer pays for a
single drop. After a quiet spell the backlog is bounded by
`ttl / step` buckets and the cap amortizes catch-up across subsequent
bucket-creating writes rather than dumping a week of demolition on the
first like after a lull.

**Residue** — an index that never receives another write keeps its final
`ttl` of buckets indefinitely. This is bounded garbage that owes nobody
a refund. If it ever matters, the backstop is an epoch-transition sweep
riding the existing scheduled-cleanup pattern
(`check_for_ended_vote_polls` / `clean_up_after_vote_polls_end`);
deliberately **out of scope for v1**.

**User deletes and updates of expired documents** — a document older
than the TTL horizon has no entries left under the TTL'd index, so the
delete and update walkers **skip that index** for any bucket key whose
start is behind the horizon. The skip is deterministic on every node:
it derives from the carried `$createdAt` and block time, the same two
inputs the write that created the entries used.

**Per-index semantics** — TTL removes entries from *this index only*.
An indexOnly like whose windowed entries expire keeps counting in the
all-time ranked `byPost` and in `byLiker`; permanence lives where the
contract declares it. Ranked per-window secondaries die with their
bucket — which also caps live leaderboard state at ~`ttl / step` windows
per index.

## grovedb dependency: detach-and-sweep

Dropping a bucket must cost **O(1) in consensus, independent of the
bucket's contents** — a viral window may hold millions of entries, and a
drop whose cost scales with contents can neither be paid by the
triggering writer nor fit in a block. The existing `clear_subtree` is
explicitly not this (costs marked not-yet-correct, indexed primaries
rejected, nested subtrees enumerated element-by-element).

The primitive, specified in the grovedb issue:

1. **Detach (consensus, O(1))** — remove the bucket element from the
   grid-level Merk. The root hash is immediately correct and the window
   is provably absent.
2. **Sweep (budgeted, off the critical path)** — every subtree lives
   under its own storage prefix and every per-axis secondary under a
   derived prefix; reclamation is prefix range-deletes driven from a
   small deletion queue with a per-block budget. Because TTL'd bytes are
   never refundable, the sweep needs no per-entry consensus accounting.

Until it lands, the platform implementation performs the drop through
grovedb's recursive element delete (which does sweep an indexed tree's
axes) behind a single `Drive` helper — correct, wrong cost class — so
the primitive is a drop-in swap.

## Fee mechanics

Write operations targeting a TTL'd index's subtrees are classified
**ephemeral**: their added bytes bill to processing at an
ephemeral-bytes rate (a fee-version constant) instead of to storage, and
the elements carry no storage flags. Deletion (both the TTL drop and a
user delete of a not-yet-expired document) generates no refunds — there
is nothing to refund. Cost estimation mirrors the same classification so
estimated and actual fees stay in the same class.

## Queries

Unchanged. An expired window is a provable empty answer through every
surface (document, count/sum/avg, ranked, having-range). One documented
consequence: on a TTL'd index, `byStart` addresses historic windows
*within the TTL horizon* — beyond it, absence is the (correct, provable)
answer.

## Versioning

Everything rides the still-unreleased PV14 grammar: the `ttl` key joins
the meta-schema v3 `timeRange` map, the two limits join a new
`SystemLimits` version, and the fee constant joins the PV14 fee table.
No migration story exists or is needed.
