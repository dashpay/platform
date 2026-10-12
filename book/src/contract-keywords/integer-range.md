# Integer-Range Indexes

`integerRange` groups an index's documents into windows of an integer property, so that "listings priced 500 to 799 by category" or "players per score band" is a question about one window rather than a scan over values. Each window is `range` units long and a new one starts every `step` units; when windows overlap, a document belongs to each window that contains its value. Combined with the count and ranking keywords, an integer-range index serves counts, sums and leaderboards per window, with proofs. It is the integer counterpart of [`timeRange`](time-range.md), and costs one set of index entries per window a document falls in.

| | |
|---|---|
| **Where** | index; buckets the index's first property |
| **Value** | object: `on`, `range`, `step` (required), `phase` |
| **Default** | absent: the property is indexed as it is |
| **Since** | protocol version 14 |
| **On update** | Fixed, like every index (`DataContractInvalidIndexDefinitionUpdateError`, 10217) |
| **Errors** | `DuplicateUniqueIndexError` (40105) on a unique integer-range index |

## Example

```json
"listing": {
  "type": "object",
  "indices": [
    {
      "name": "byPriceBand",
      "properties": [{ "price": "asc" }, { "category": "asc" }],
      "integerRange": { "on": "price", "range": 300, "step": 100 },
      "countable": "countable",
      "rangeCountable": true
    },
    {
      "name": "oneBidPerBand",
      "properties": [{ "price": "asc" }, { "$ownerId": "asc" }],
      "unique": true,
      "integerRange": { "on": "price", "range": 1000, "step": 1000 }
    }
  ],
  "properties": {
    "price": { "type": "integer", "minimum": 0, "maximum": 1000000, "position": 0 },
    "category": { "type": "string", "minLength": 1, "maxLength": 63, "position": 1 }
  },
  "required": ["price", "category"],
  "additionalProperties": false
}
```

`byPriceBand` keeps windows 300 wide starting every 100, so a listing priced 750 is counted in the windows starting at 500, 600 and 700, and the listings per category between 600 and 899 are one query. `oneBidPerBand` has non-overlapping windows of 1000 and is unique, so an identity may hold one listing per thousand: a second listing priced 1200 next to one priced 1900 is refused with `DuplicateUniqueIndexError` (40105).

## The keys

| Key | Value | What it does |
|---|---|---|
| `on` | property name, required | The integer property to bucket. It must be the index's first property, an integer property of the type of at most 64 bits, and listed in the type's `required`. |
| `range` | integer, at least 1, required | The length of each window, in the property's own units. An exact multiple of `step`. |
| `step` | integer, at least 1, required | The distance between the starts of two windows. |
| `phase` | integer, default `0` | Moves the window boundaries: windows start at `phase + k * step` for every integer `k`, negative ones included. Less than `step`. Windows of 100 cut at 50, 150, 250 take `"phase": 50`. |

## How it works

**Windows.** For each document, the index stores the start of every window that contains the document's value, in place of the value itself, encoded exactly like a value of the property. With `range` equal to `step` the windows do not overlap and a value is in exactly one; otherwise it is in `range / step` of them, the index's **overlap factor**, and the document costs that many sets of entries. The overlap factor is at most 24 at protocol version 14, the same cap as a time-range grid's.

**The bottom window.** The schema's `minimum` and `maximum` decide how many bytes the property takes and whether it can be negative. A property with a non-negative `minimum`, or with only a `maximum`, is unsigned and its lowest possible value is 0. Otherwise it is signed, and its lowest possible value is the smallest the chosen width holds: -128 for `"minimum": -100, "maximum": 100`, and -9223372036854775808 for an integer with no bounds. A window that would start below the lowest value the property's type can hold starts at that value instead. So on an unsigned property with `"phase": 50`, the values 0 to 49 are in a window that starts at 0, and with overlapping windows every window that would start below the lowest value merges into the one that starts at it. Every value the property can hold is in at least one window. When the lowest value is itself a window start, as it is on an unsigned property with no `phase`, those windows add nothing and simply do not exist.

A property that changes value moves the document into the windows of its new value. Several indexes may bucket the same property with different grids; each grid is stored apart from the others.

The bucketed property is required, so it is never a [`skipIfAbsent`](indexes.md#skipifabsent) property, but the index's other properties can be: a document missing one of them writes no entry in any window, and an update that adds or drops it moves the document into or out of all of its windows.

**Queries.** A query selects one window of the index with an `IN_INTEGER_RANGE` clause on the bucketed property, naming the window by its start. The start must be a window start of the grid, or the lowest value of the property's type for a clamped bottom window; any other value is refused rather than rounded. The server and the proof verifier both read the window from the query itself, so no clock is involved. When the property is bucketed by several grids, the query names the grid. A query may select at most one window, of either kind. A window with no documents is a proved empty answer. Counts, sums and rankings declared on the index are then per window: a ranking below the bucketed property is one leaderboard per window.

A plain clause on the bucketed property, such as `price == 700`, never reads an integer-range index: the index holds window starts, not prices. Such a query needs an index that is not bucketed.

In the JavaScript SDK the selection is an `integerRange` entry of the query: `{ field: "price", start: 600 }`, with `grid: { range, step, phase }` when the property has several grids. A start beyond `Number.MAX_SAFE_INTEGER` is given as a decimal string. In the Rust SDK it is `DocumentQuery::with_integer_range("price", 600u64)`.

**Uniqueness.** On a unique integer-range index, two documents conflict when their values fall in the same window and they agree on the index's other properties. That is only meaningful when each value is in one window, so uniqueness needs non-overlapping windows. A document may change its value within its own window; moving into a window where another document holds the same other properties is refused.

## Rules at registration

- `on` names an integer property of the document type of at most 64 bits, which is the index's first property and is listed in `required`. A system property cannot be bucketed by value; for timestamps use [`timeRange`](time-range.md).
- `range` and `step` are at least 1, `range` is an exact multiple of `step`, and `range / step` is at most 24.
- `phase` is less than `step`.
- A unique integer-range index has `range` equal to `step`.
- The index is not contested, does not set `nullSearchable: false`, is not `preallocated`, and does not also declare `timeRange`.
- A ranking sits below the bucketed property: a single-property integer-range index cannot be ranked, and `rankedCountable.at` cannot name the bucketed property. See [Ranked Indexes](ranked.md).
- `on` is not inside an object that is left out of `required`: a document could otherwise omit the object and fall in no window.
- The grid-qualified level key (`on#range#step`, with `#phase` when non-zero) is at most 255 bytes.
- A `refersTo` `findBy` cannot resolve through an integer-range index, and a [`propertyConstraints`](property-constraints.md) count or sum does not read one. See [findBy](refers-to-lookup.md).
- An [index-only type](index-only.md) cannot declare `integerRange`: its entries are keyed by the index's values and terminal, so two rows that differ only in the bucketed integer would claim the same entry in every window they share.

A broken rule is refused as `InvalidContractStructure` (10231), or by the meta-schema as `JsonSchemaError` (10101). Before protocol version 14 the keyword is unknown and refused.

## See also

- [Time-Range Indexes](time-range.md) for windows over timestamps.
- [Counts, Sums and Averages](aggregates.md) and [Ranked Indexes](ranked.md) for totals and leaderboards per window.
- [Indexes](indexes.md) for the index keywords every index uses.
