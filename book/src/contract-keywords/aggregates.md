# Counts, Sums and Averages

Counting documents normally means fetching them and counting what comes back, which grows with the number of documents and proves every one of them. The keywords in this chapter make Drive keep running totals inside its trees instead, so a count, a sum or an average is read without visiting the documents, and proved with a short proof. The document type keywords keep totals over all of a type's documents. The index keywords keep totals per indexed value, and with their `range*` forms over ranges of values. An average is never stored as such: the platform returns the count and the sum of the same documents, and the client divides.

Every total is updated by every write that touches it, so each flag adds to the cost of writing documents of the type. The flags choose the layout of the type's trees, so all of them are fixed when the document type is created.

## Example

```json
"tip": {
  "type": "object",
  "documentsMutable": false,
  "documentsAverageable": "amount",
  "indices": [
    { "name": "byRecipient", "properties": [{ "recipient": "asc" }], "averageable": "amount" },
    { "name": "byDay", "properties": [{ "day": "asc" }], "summable": "amount", "rangeSummable": true }
  ],
  "properties": {
    "recipient": {
      "type": "array", "byteArray": true, "minItems": 32, "maxItems": 32,
      "contentMediaType": "application/x.dash.dpp.identifier",
      "position": 0
    },
    "amount": { "type": "integer", "minimum": 1, "maximum": 4294967295, "position": 1 },
    "day": { "type": "integer", "minimum": 0, "maximum": 4294967295, "position": 2 }
  },
  "required": ["recipient", "amount", "day"],
  "additionalProperties": false
}
```

`documentsAverageable` keeps the number of tips and their total, so the average tip over the whole type is one read. `byRecipient` keeps the same pair per recipient, so "how many tips did this identity get, and how much on average" is one read. `byDay` keeps the sum per day and, with `rangeSummable`, answers "total tipped between day 100 and day 130" without visiting each day.

## `documentsCountable`

| | |
|---|---|
| **Where** | document type |
| **Value** | boolean |
| **Default** | `false` |
| **Since** | protocol version 12 |
| **On update** | Fixed (`DocumentTypeUpdateError`, 40212) |

Keeps the number of the type's documents in its primary tree, so a count query with no `where` clause is one read, with a proof. Without it, such a query is refused rather than answered slowly.

## `documentsSummable`

| | |
|---|---|
| **Where** | document type |
| **Value** | the name of an integer property, 1 to 64 characters |
| **Default** | absent |
| **Since** | protocol version 12 |
| **On update** | Fixed (40212) |

Keeps the sum of the named property over all the type's documents, so a sum query with no `where` clause is one read. On a type with `documentsKeepHistory`, the sum counts each document's current revision only.

The property must exist on the type, be listed in `required` (a missing value would leave the sum wrong when the document is deleted), and hold values that fit a signed 64-bit sum. Integers are stored in the smallest width their bounds allow when the contract uses [`sizedIntegerTypes`](contract-config.md#sizedintegertypes), and a property that becomes an unsigned 64-bit integer is refused: `"minimum": 0` with no `maximum` does, so add a `maximum` of at most 4294967295, or give no bounds at all.

## `documentsAverageable`

| | |
|---|---|
| **Where** | document type |
| **Value** | the name of an integer property, 1 to 64 characters |
| **Default** | absent |
| **Since** | protocol version 12 |
| **On update** | Fixed (40212) |

Shorthand for `documentsCountable: true` plus `documentsSummable` on the named property: the count and the sum an average is computed from. The storage is exactly that of the two flags. When `documentsSummable` is also written it must name the same property, and an explicit `documentsCountable: false` beside it is refused as a contradiction.

## Document type `rangeCountable`, `rangeSummable`, `rangeAverageable`

| | |
|---|---|
| **Where** | document type |
| **Value** | boolean each |
| **Default** | `false` |
| **Since** | protocol version 12 |
| **On update** | Fixed (40212) |

These keep the count, the sum, or both at every node of the primary tree rather than only at its root, which is what range and offset aggregates over the primary key need. They are rarely useful: a range count or sum is almost always wanted over an indexed property, which is what the [index keywords of the same names](#index-rangecountable-rangesummable-rangeaverageable) give.

- `rangeCountable` implies `documentsCountable`.
- `rangeSummable` needs `documentsSummable` or `documentsAverageable`.
- `rangeAverageable` is shorthand for `rangeCountable` plus `rangeSummable`, and needs `documentsAverageable`. An explicit `false` for either of the two beside it is refused as a contradiction.

## `countable`

| | |
|---|---|
| **Where** | index |
| **Value** | `"notCountable"`, `"countable"`, `"countableAllowingOffset"`, or a boolean (`true` is `"countable"`, `false` is `"notCountable"`) |
| **Default** | `"notCountable"` |
| **Since** | protocol version 12 |
| **On update** | Fixed, like every index (`DataContractInvalidIndexDefinitionUpdateError`, 10217) |

Keeps a document count for each value of the index, so "how many documents have these values" is one read per value, with a proof. A count query is served by a countable index whose properties exactly match its `where` clauses, each an equality or an `in`: a countable index on `[brand, color]` answers `brand == "a" AND color == "red"`, and `color in [...]` under a fixed brand with one count per color, but not `brand == "a"` alone. Declare one countable index per shape of count the application needs.

`"countableAllowingOffset"` keeps a count at every node of the index's trees, not only per value. It costs more on every write and prepares the index for offset queries; use `"countable"` unless you need it. The boolean form is kept for contracts written before the string form existed.

On a unique index the flag changes almost nothing, since each value holds at most one document; it only counts the documents that leave an indexed property out.

## `summable`

| | |
|---|---|
| **Where** | index |
| **Value** | the name of an integer property, 1 to 64 characters |
| **Default** | absent |
| **Since** | protocol version 12 |
| **On update** | Fixed (10217) |

Keeps the sum of the named property for each value of the index, the way `countable` keeps a count, and serves sum queries under the same exact-match rule. The property follows the rules of `documentsSummable`: it exists, is required, and fits a signed 64-bit sum.

## `averageable`

| | |
|---|---|
| **Where** | index |
| **Value** | the name of an integer property, 1 to 64 characters |
| **Default** | absent |
| **Since** | protocol version 12 |
| **On update** | Fixed (10217) |

Shorthand for `countable: "countable"` plus `summable` on the named property, which is what an average query per value needs. When `summable` is also written it must name the same property. An explicit `countable: "countableAllowingOffset"` beside it is kept; an explicit `"notCountable"` (or `false`) is refused as a contradiction.

## Index `rangeCountable`, `rangeSummable`, `rangeAverageable`

| | |
|---|---|
| **Where** | index |
| **Value** | boolean each |
| **Default** | `false` |
| **Since** | protocol version 12 |
| **On update** | Fixed (10217) |

These answer aggregates over a **range** of the index's last property: "how many reviews between grade 60 and 80", "total tipped from day 100 to day 130". The count or sum is kept at every node of the tree of that property's values, so a range total is read by walking the edges of the range, with a proof whose size grows with the logarithm of the number of values rather than with the documents. The same index also returns one total per distinct value in a range. The query fixes the properties before the last one, with equalities or an `in`.

- `rangeCountable` makes the index countable: an omitted `countable` becomes `"countable"`, an explicit `"countableAllowingOffset"` is kept, and an explicit `"notCountable"` is refused. Before protocol version 14, `countable` had to be written out beside it.
- `rangeSummable` needs `summable` or `averageable`.
- `rangeAverageable` is shorthand for `rangeCountable` plus `rangeSummable`, and needs `averageable`. An explicit `false` for either of the two beside it is refused.

A range total costs more on each write than a per-value total, since every node of the tree carries it. The ranked keywords build on these flags: see [Ranked Indexes](ranked.md).

## How they combine

- **Count and sum on one tree.** An index with both a count and a sum (by `averageable`, or by `countable` plus `summable`) keeps both in one tree, and one proof returns the pair. The same holds for the document type flags.
- **One summed property per type.** Every `summable`, `averageable`, `documentsSummable` and `documentsAverageable` of a document type must name the same property: the sums share their trees, which have no room to tell two properties apart. To sum two properties, use two document types.
- **Type and index flags are independent.** `documentsCountable` gives the unfiltered total; a countable index gives filtered ones. A type may have both.
- **Index-only types** take the index keywords but refuse the document type ones, since they have no primary tree. See [Index-Only Types](index-only.md).
- **Queries must match.** A count or sum query that no index serves exactly is refused; there is no slow fallback. Pick the indexes for the queries the application will make.

## Rules at registration

- A summed property exists on the type, is an integer that fits a signed 64-bit sum (not an unsigned 64-bit integer), and is listed in `required`.
- All summed properties of a type are the same property.
- From protocol version 14, on a type with a [contested index](contested.md), the summed property declares a `minimum` of at least -134217728 and a `maximum` of at most 134217728 (±2^27).
- From protocol version 14, on a type with a [`ttl`](ttl.md), the summed property declares a `minimum` of at least 0, or a `minimum` of at least -134217728 and a `maximum` of at most 134217728 (±2^27).
- On other types, each value only has to fit a signed 64-bit integer.
- The shorthands agree with their longhand where both are written, and no explicit `false` or `"notCountable"` contradicts them.
- Each `range*` flag has its prerequisite, as listed above.
- The meta-schema refuses a malformed value (`JsonSchemaError`, 10101).

## Choosing what to set

| The application needs | Set |
|---|---|
| The number of documents of the type | `documentsCountable: true` |
| The sum, or the average, of a property over the whole type | `documentsSummable` or `documentsAverageable` |
| A count per value (`where author == x`) | `countable: "countable"` on an index whose properties are exactly those of the query |
| A sum or an average per value | `summable` or `averageable` on such an index |
| A count, sum or average over a range (`where grade > 60`) | the matching `range*` flag on an index whose last property is the ranged one |
| The top or bottom values by count, sum or average | the `range*` flags plus a ranking: see [Ranked Indexes](ranked.md) |

## See also

- [Document Count Trees](../drive/document-count-trees.md) and [Document Sum Trees](../drive/document-sum-trees.md) for the tree variants, the query endpoints and what each flag costs.
- [Count Index Examples](../drive/count-index-examples.md), [Sum Index Examples](../drive/sum-index-examples.md) and [Average Index Examples](../drive/average-index-examples.md) for worked queries and proof sizes.
- [Indexes](indexes.md) for the index keywords every index uses.
- [Ranked Indexes](ranked.md) for ordering values by these totals.
