# Time-Range Indexes

`timeRange` groups an index's documents into time windows by one of their timestamps, so that "the most used hashtags this hour" or "posts per day" is a question about one window rather than a scan over time. Each window is `range` seconds long and a new one starts every `step` seconds; when windows overlap, a document belongs to each window that contains its timestamp. Combined with the count and ranking keywords, a time-range index serves trending lists and leaderboards per window, with proofs. It costs one set of index entries per window a document falls in, and a `ttl` lets old windows expire so that this data does not stay in state forever.

| | |
|---|---|
| **Where** | index; buckets the index's first property |
| **Value** | object: `on`, `range`, `step` (required), `phase`, `ttl` |
| **Default** | absent: the timestamp is indexed as it is |
| **Since** | protocol version 14 (`ttl` included) |
| **On update** | Fixed, like every index (`DataContractInvalidIndexDefinitionUpdateError`, 10217) |
| **Errors** | `DuplicateUniqueIndexError` (40105) on a unique time-range index |

## Example

```json
"post": {
  "type": "object",
  "indices": [
    {
      "name": "hourlyByHashtag",
      "properties": [{ "$createdAt": "asc" }, { "hashtag": "asc" }],
      "timeRange": { "on": "$createdAt", "range": 3600, "step": 900, "ttl": 86400 },
      "countable": "countable",
      "rangeCountable": true
    },
    {
      "name": "onePerDay",
      "properties": [{ "$createdAt": "asc" }, { "$ownerId": "asc" }],
      "unique": true,
      "timeRange": { "on": "$createdAt", "range": 86400, "step": 86400 }
    }
  ],
  "properties": {
    "hashtag": { "type": "string", "minLength": 1, "maxLength": 63, "position": 0 },
    "text": { "type": "string", "maxLength": 280, "position": 1 }
  },
  "required": ["$createdAt", "hashtag", "text"],
  "additionalProperties": false
}
```

`hourlyByHashtag` keeps one-hour windows starting every 15 minutes, so each post is counted in four windows, and the counts per hashtag in the window that covers the last hour are one query. Its entries expire a day after their window starts. `onePerDay` has non-overlapping daily windows and is unique, so an identity may post once per day: a second post in the same day is refused with `DuplicateUniqueIndexError` (40105).

## The keys

| Key | Value | What it does |
|---|---|---|
| `on` | `"$createdAt"`, `"$updatedAt"` or `"$transferredAt"`, required | The timestamp to bucket. It must be the index's first property and be listed in the type's `required`, since a timestamp that is not required is never recorded. |
| `range` | integer seconds, at least 1, required | The length of each window. An exact multiple of `step`. |
| `step` | integer seconds, at least 1, required | The time between the starts of two windows. |
| `phase` | integer seconds, default `0` | Moves the window boundaries: windows start at `phase + k * step` from the Unix epoch. Less than `step` and less than one year (31536000). Daily windows cut at 06:00 UTC take `"phase": 21600`. |
| `ttl` | integer seconds, at least 1 | How long the index's entries live after their window starts. At least `range` and at most 604800 (one week) at protocol version 14. Absent, entries live forever. |

## How it works

**Buckets.** For each document, the index stores the start of every window that contains the document's timestamp, in milliseconds, in place of the timestamp itself. With `range` equal to `step` the windows do not overlap and a document is in exactly one; otherwise it is in `range / step` of them, the index's **overlap factor**, and costs that many sets of entries. The overlap factor is at most 24 at protocol version 14. Windows are declared in seconds because block time, which the timestamps come from, moves in steps of about five seconds.

A source that changes, `$updatedAt` or `$transferredAt`, moves the document into the current windows each time it changes. Several indexes may bucket the same timestamp with different grids; each grid is stored apart from the others.

**Queries.** A query selects one window of the index with an `IN_TIME_RANGE` clause on the bucketed timestamp:

- `newest`: the window that started most recently, which covers the latest slice of time (up to one `step`).
- `oldest`: the oldest window still open, which covers nearly a full `range` of history: the one for "trending over the last hour".
- `byStart`: the window starting at a given millisecond time on the grid, which reaches past windows.

`newest` and `oldest` are resolved against block time on the server and against the signed time of the response when verified. When the timestamp is bucketed by several grids, the query names the grid. A query may select at most one window. A window with no documents, including one that has not started, is a proved empty answer. Counts, sums and rankings declared on the index are then per window: a ranking below the bucketed timestamp is one leaderboard per window.

**Uniqueness.** On a unique time-range index, two documents conflict when they fall in the same window and agree on the index's other properties. That is only meaningful when each document is in one window and cannot move, so uniqueness needs non-overlapping windows over `$createdAt`.

**Expiry (`ttl`).** An entry can be queried until `ttl` seconds after its window's start; a query for an expired window is refused, so every window a query can reach is complete. The expired entries are removed lazily: each later write into the index removes some of the oldest expired entries, within a bounded budget, at no charge to the writer. An index that stops receiving writes keeps its expired entries.

Everything written under an index with a `ttl` is billed as processing, at an ephemeral-bytes rate, instead of as storage, and nothing is refunded when it is removed. The rate prices a lifetime of at most a week, which is why `ttl` is capped there.

A `ttl` removes entries from this index only. The documents stay, and so do their entries in the type's other indexes. To delete the documents themselves after a time, use the document type's [`ttl`](ttl.md).

On an index-only type, a window with a `ttl` may also declare [`outlivesDelete`](index-only.md#outlivesdelete): a delete of a document then leaves its entries in the window to expire, so the delete needs no `$createdAt`, and the document keeps counting there until the window moves past it.

## Rules at registration

- `on` names `$createdAt`, `$updatedAt` or `$transferredAt`, which is the index's first property and is listed in `required`. A user property cannot be bucketed by time; an integer one can be bucketed by value with [`integerRange`](integer-range.md).
- `range` and `step` are at least 1, `range` is an exact multiple of `step`, and `range / step` is at most 24.
- `phase` is less than `step` and less than 31536000.
- `ttl`, when present, is at least `range` and at most 604800. Two indexes that bucket the same timestamp with the same `range`, `step` and `phase` share their storage and must declare the same `ttl`, or none.
- A unique time-range index has `range` equal to `step` and `on` equal to `$createdAt`.
- The index is not contested, does not set `nullSearchable: false`, and is not `preallocated`.
- A ranking sits below the bucketed timestamp: a single-property time-range index cannot be ranked, and `rankedCountable.at` cannot name the timestamp. See [Ranked Indexes](ranked.md).
- A `refersTo` `findBy` cannot resolve through a time-range index. See [findBy](refers-to-lookup.md).
- On an [index-only type](index-only.md), only `$createdAt` can be bucketed, and a bucketed index cannot serve as the type's proof index.

A broken rule is refused as `InvalidContractStructure` (10231), or by the meta-schema as `JsonSchemaError` (10101). Before protocol version 14 the keyword is unknown and refused.

## See also

- [Time-Range Index TTL](../drive/time-range-ttl.md) for how expired windows are drained, the fee rate and the query gate.
- [Integer-Range Indexes](integer-range.md) for the same windows over an integer property.
- [Counts, Sums and Averages](aggregates.md) and [Ranked Indexes](ranked.md) for totals and leaderboards per window.
- [Time To Live (ttl)](ttl.md) for expiring whole documents.
- [Indexes](indexes.md) for the index keywords every index uses.
