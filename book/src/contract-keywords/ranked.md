# Ranked Indexes

A ranked index answers "which values score highest": the five restaurants with the best average grade, the ten hashtags with the most posts, the three sellers with the largest sales. The range aggregates of [Counts, Sums and Averages](aggregates.md) keep a count or a sum for every value of an index, but in the order of the values, so finding the top five means reading every value. A ranking adds a second, ordered view of the same totals, and a "top K" query reads K entries from one end of it, with a proof that grows with K rather than with the number of values. Each ranking costs one more tree that every write under the index updates, so a contract declares only the rankings it will query.

The index's **groups** are the distinct values of its last property. The three keywords rank the groups by document count, by the sum of the summed property, or by its average.

## Example

```json
"review": {
  "type": "object",
  "indices": [
    {
      "name": "byRestaurant",
      "properties": [{ "restaurantId": "asc" }],
      "averageable": "grade",
      "rangeAverageable": true,
      "rankedAverageable": true,
      "rankedCountable": true
    }
  ],
  "properties": {
    "restaurantId": { "type": "string", "minLength": 1, "maxLength": 32, "position": 0 },
    "grade": { "type": "integer", "minimum": 0, "maximum": 100, "position": 1 }
  },
  "required": ["restaurantId", "grade"],
  "additionalProperties": false
}
```

`averageable` and `rangeAverageable` keep the count and the sum of `grade` per restaurant; `rankedAverageable` orders the restaurants by average grade and `rankedCountable` by number of reviews. The query

```sql
SELECT avg(grade) FROM review GROUP BY restaurantId ORDER BY avg(grade) DESC LIMIT 3
```

returns the three best-rated restaurants with their counts and sums, proved.

## `rankedCountable`

| | |
|---|---|
| **Where** | index |
| **Value** | boolean, or `{ "at": <property name or array of 1 to 10 names> }` |
| **Default** | `false` |
| **Since** | protocol version 14 |
| **On update** | Fixed, like every index (`DataContractInvalidIndexDefinitionUpdateError`, 10217) |

Ranks groups by how many documents they hold. Needs `rangeCountable: true` on the index, or `rangeAverageable: true`, which implies it.

`true` ranks the values of the index's last property. On a compound index, the ranking is kept separately for each value of the properties before it: on `[city, restaurantId]` each city has its own ranking of restaurants, and a query names the city.

The object form `{ "at": ... }` places the ranking at another level of the index. Naming an earlier property ranks that property's values by the number of documents beneath them, whatever the later properties hold:

```json
{
  "name": "byHashtagPost",
  "properties": [{ "hashtag": "asc" }, { "postId": "asc" }],
  "countable": "countable",
  "rangeCountable": true,
  "rankedCountable": { "at": ["hashtag", "postId"] }
}
```

`"hashtag"` in `at` ranks hashtags by their total posts across all post ids; `"postId"`, the last property, is the same as `true` and ranks the posts under one hashtag. An array declares several rankings on one index; each level named costs one more ordered tree to maintain on every write beneath it. A query addresses a ranking by the property it groups by, with every property before that one fixed.

`at` names only the index's own properties, each once. The object form cannot be combined with `rankedSummable` or `rankedAverageable`: a ranking at an earlier level is fed by a chain of counts that cannot also carry a sum. A [`summableOffCountIndex`](index-only.md#summableoffcountindex) index is the exception: its counters feed a chain of sums, and of counts too from the shallowest average ranking down, so it takes all three at any level. An average reads a level only where that chain carries counts: above the shallowest average ranking its value trees sum but count nothing. There a document count is its sums (likes, not posts), so `rankedCountable` declares the same ranking as `rankedSummable`, which a ranked `count(*)` and a ranked `sum` of the source both read, and it needs no `rangeCountable`.

## `rankedSummable`

| | |
|---|---|
| **Where** | index |
| **Value** | boolean, or `{ "at": <property name or array of 1 to 10 names> }` on a `summableOffCountIndex` index |
| **Default** | `false` |
| **Since** | protocol version 14 |
| **On update** | Fixed (10217) |

Ranks the groups of the index's last property by the sum of the index's `summable` property: the recipients who received the most, the products that sold the most units. Needs `rangeSummable: true`, or `rangeAverageable: true`. On a [`summableOffCountIndex`](index-only.md#summableoffcountindex) index the sum is the source's entries, and `at` ranks an earlier level by it: authors by the likes their posts received. A ranked `count(*)` over such an index reads this ranking too, since its sums are its document counts.

## `rankedAverageable`

| | |
|---|---|
| **Where** | index |
| **Value** | boolean, or `{ "at": <property name or array of 1 to 10 names> }` on a `summableOffCountIndex` index |
| **Default** | `false` |
| **Since** | protocol version 14 |
| **On update** | Fixed (10217) |

Ranks the groups of the index's last property by the average of the `averageable` property. On a [`summableOffCountIndex`](index-only.md#summableoffcountindex) index the average is entries per group, and `at` ranks an earlier level by it: authors by likes per post. Needs both range totals: `rangeAverageable: true`, or `rangeCountable: true` with `rangeSummable: true`.

The three keywords are independent. `rankedAverageable` does not imply `rankedCountable` or `rankedSummable`, unlike `averageable`, which is shorthand for a count and a sum. Declare each ranking the application will query, and no other.

## Queries

A ranked query names one aggregate, groups by the ranked property, orders by the aggregate and takes a limit, with an optional offset. On a compound index such as `[city, restaurantId]`:

```sql
SELECT count(*) FROM review WHERE city == "London" GROUP BY restaurantId ORDER BY count(*) DESC LIMIT 5
```

Every property before the grouped one must be fixed with an equality; at most one of them may instead be an `in` of 2 to 10 values, whose rankings are merged. There is no ranking across different values of those properties. A ranked index still answers the per-value range queries the same `range*` flags answer (one entry per value in the range). A range total, one count, sum or average over a whole range, is not available through an index whose path passes through a ranked level, one it ranks itself or one another index ranks at a level the two share: the ranked trees are indexed trees, which grovedb neither totals a range over nor proves a range total through, and an unproven total through a ranked level is refused as well so that a proved and an unproven read agree. Such a query is refused with a hint to group by the last property.

The request shape, ties, offsets and proofs are described in [Ranked Index Examples](../drive/ranked-index-examples.md).

## Rules at registration

- **Its range totals.** Each ranking needs the range flags listed above, checked both by the meta-schema and by the parser. A written-out `false` needs nothing.
- **A non-unique index.** Every group of a unique index holds at most one document, so a ranking on one is refused, and a contested index, which is unique, cannot have one either.
- **`nullSearchable` left at `true`.** With `false`, documents missing the property would still leave an empty group in the ranking.
- **A shorter key.** The ranked property's value becomes part of the ordered tree's key, behind an 8-byte sort key (16 bytes when `rankedAverageable` ranks that property), so its worst case must fit in what is left of the 255-byte limit. A ranked string takes `maxLength` of at most 61, or 59 at a property `rankedAverageable` ranks; a ranked byte array takes `maxItems` of at most 247, or 239. A larger bound is refused (`InvalidIndexedPropertyConstraintError`, 10205). Identifiers, integers and other fixed-width values always fit. The bound applies to each ranked property, by the rankings at that property: the last one when a ranking is declared with `true`, and each property named in `at`.
- **Time and value windows.** On a [`timeRange`](time-range.md) or [`integerRange`](integer-range.md) index, a ranking must sit below the bucketed property, which gives one ranking per window. A single-property bucketed index cannot be ranked, and `at` cannot name the bucketed property.
- **Other indexes of the type.** A compound ranked index `[p1, ..., pn]` is refused when another countable or summable index ends at exactly `[p1, ..., pn-1]`. A ranking at an earlier level (`at`) also restricts the other indexes that reach that level; the full table is in [Shape Restrictions](../drive/document-ranked-trees.md#shape-restrictions).
- **One index per property list.** Two indexes with the same properties are a `DuplicateIndexError` (10201), so the rankings of one property list go on one index.
- **Protocol version 14.** Earlier versions do not know the keywords and refuse them.

A broken rule other than the key length is refused as `InvalidContractStructure` (10231), or by the meta-schema as `JsonSchemaError` (10101).

## Costs

Each ranking is one ordered tree, rewritten whenever a document under it is created, changed or deleted, on top of the range totals it is built from. Two rankings on one index cost two rewrites per write; a fully ranked `at` array costs one per ranked level. A ranking at the index's first property gets its tree when the contract is registered; a ranking at a deeper level gets one tree per value above it, as documents arrive.

## See also

- [Document Ranked Trees](../drive/document-ranked-trees.md) for the tree variants, prefix-level rankings and how the rankings are maintained.
- [Ranked Index Examples](../drive/ranked-index-examples.md) for worked queries and proofs.
- [Counts, Sums and Averages](aggregates.md) for the range totals a ranking builds on.
- [Time-Range Indexes](time-range.md) for rankings per time window.
