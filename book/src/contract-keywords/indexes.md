# Indexes (indices)

A document type's `indices` list says which queries its documents can answer and which values must be unique. Without an index, a query can only address a document by its `$id`. With one, Drive keeps the documents sorted by the index's properties, so a query that fixes those properties reaches the matching documents directly instead of reading the whole type. Every index costs storage and processing on each write, and an index can never be added, removed or changed once the document type exists, so a contract author decides them before the document type is registered.

This chapter covers the four keywords every index uses: `name`, `properties`, `unique` and `nullSearchable`. The other index keywords, for contests, counts, sums, rankings, time windows and index-only types, have their own chapters (see [More index keywords](#more-index-keywords)).

## Example

```json
"post": {
  "type": "object",
  "indices": [
    { "name": "byOwnerTime", "properties": [{ "$ownerId": "asc" }, { "$createdAt": "asc" }] },
    { "name": "bySlug", "properties": [{ "$ownerId": "asc" }, { "slug": "asc" }], "unique": true },
    { "name": "byTopic", "properties": [{ "meta.topic": "asc" }], "nullSearchable": false }
  ],
  "properties": {
    "slug": { "type": "string", "minLength": 1, "maxLength": 63, "position": 0 },
    "text": { "type": "string", "maxLength": 280, "position": 1 },
    "meta": {
      "type": "object",
      "properties": {
        "topic": { "type": "string", "minLength": 1, "maxLength": 32, "position": 0 }
      },
      "additionalProperties": false,
      "position": 2
    }
  },
  "required": ["$createdAt", "slug", "text"],
  "additionalProperties": false
}
```

`byOwnerTime` lists one author's posts in the order they were written. `bySlug` lets each author use a slug once: a second post by the same owner with the same slug is refused. `byTopic` finds posts by a property nested inside `meta`, and leaves out posts that have no topic.

## `indices`

| | |
|---|---|
| **Where** | document type |
| **Value** | array of 1 to 10 index objects |
| **Default** | absent: the type has no index, and its documents can only be addressed by `$id` |
| **Since** | protocol version 1 |
| **On update** | Fixed: an index may not be added, removed or changed (`DataContractInvalidIndexDefinitionUpdateError`, 10217) |
| **Errors** | `DuplicateUniqueIndexError` (40105) |

Each entry of `indices` is one index. Drive builds a tree for it when the contract is registered and maintains it on every create, replace, transfer, purchase, price update and delete of a document of the type.

A query uses an index when its `where` clauses fix the index's leading properties, in order, and it orders by the properties that follow. An index on `[a, b]` answers `a == x`, `a == x AND b == y`, and `a == x` ordered by `b`, but not `b == y` alone. The query picker and the tree layout are described in [Indexes](../drive/indexes.md#query-traversal).

Rules at registration:

- At most 10 indexes, and at least one when the key is present (an empty `indices` array is refused by the meta-schema).
- No two indexes may have the same properties in the same order (`DuplicateIndexError`, 10201).
- At most one index may be contested, and a type with a contested index may have no other unique index (`ContestedUniqueIndexWithUniqueIndexError`, 10249). See [Contested Indexes](contested.md).

## `name`

| | |
|---|---|
| **Where** | index |
| **Value** | string, 1 to 32 characters |
| **Default** | none: required |
| **Since** | protocol version 1 |
| **On update** | Fixed: indexes are compared by name, so renaming an index is a removal plus an addition (10217) |
| **Errors** | `DuplicateIndexNameError` (10211) at registration |

The name identifies the index in queries that name one, in error messages and in the contract update rule. Two indexes of one document type may not share a name (`DuplicateIndexNameError`, 10211).

## `properties`

| | |
|---|---|
| **Where** | index |
| **Value** | array of 1 to 10 objects, each `{ "<property path>": "asc" }` with exactly one key |
| **Since** | protocol version 1 |
| **On update** | Fixed (10217) |
| **Errors** | `UndefinedIndexPropertyError` (10209), `SystemPropertyIndexAlreadyPresentError` (10208), `InvalidIndexPropertyTypeError` (10206), `InvalidIndexedPropertyConstraintError` (10205), all at registration |

The indexed properties, in order. The order matters: a query uses the index through a prefix of the list. The only sort order the meta-schema accepts is `"asc"`; a query may still walk an index in descending order.

What may be indexed:

- **A top-level property** of the type, by its name.
- **A property inside an object**, by its dotted path. DPNS indexes `records.identity`, the `identity` property of a domain's `records` object.
- **System properties**: `$ownerId`, `$createdAt`, `$updatedAt`, `$transferredAt`, their `*BlockHeight` and `*CoreBlockHeight` variants, `$creatorId` on a type that records it, and `$moderatedAt` and `$moderatedBy` on a type that lists `moderatorAbilities.changeFields`, in a non-unique index, from protocol version 14 (see [System Properties](system-properties.md)). A timestamp or block height is only recorded when the type lists it in `required`; an index on one that is not required holds every document under null.
- **Not `$id`**, which the document type's primary tree already indexes (`SystemPropertyIndexAlreadyPresentError`, 10208).

What each indexed property must be, because its value becomes a GroveDB key of at most 255 bytes:

- A property the type defines (`UndefinedIndexPropertyError`, 10209).
- Not an object, an array of values or a typed array (`InvalidIndexPropertyTypeError`, 10206). A byte array, including an identifier, is fine.
- A string must declare `maxLength` of at most 63, since a character can take four bytes. A byte array must declare `maxItems` of at most 255. A missing or larger bound is refused (`InvalidIndexedPropertyConstraintError`, 10205). An index with a ranking has tighter bounds (see [Ranked Indexes](ranked.md)).
- From protocol version 14, not a transient property, nor one inside a transient object: its value is never stored, so the index would never hold it. See [transient](transient.md).

## `unique`

| | |
|---|---|
| **Where** | index |
| **Value** | boolean |
| **Default** | `false` |
| **Since** | protocol version 1 |
| **On update** | Fixed (10217) |
| **Errors** | `DuplicateUniqueIndexError` (40105) |

On a unique index no two documents may hold the same values for all of the index's properties. A create, replace, transfer, purchase or price update that would make a second document with the same values is refused with `DuplicateUniqueIndexError` (40105); so is a moderator's restore of a deleted document whose values have been taken since. A replace that leaves the indexed values as they were is not held against the document itself.

A document that leaves any of the indexed properties out is not held to the index: uniqueness cannot be decided on a missing value, so two such documents may coexist. Make the properties `required` when every document must be unique.

Uniqueness is per index. `bySlug` above is unique over the pair (`$ownerId`, `slug`), so two authors may use the same slug; a unique index on `slug` alone would make each slug global.

A unique index with a [`timeRange`](time-range.md) treats two documents in the same window as equal on the bucketed property. A unique index cannot carry a ranking.

## `nullSearchable`

| | |
|---|---|
| **Where** | index |
| **Value** | boolean |
| **Default** | `true` |
| **Since** | protocol version 1 |
| **On update** | Fixed (10217) |

With the default, a document whose indexed properties are all missing is still entered in the index, under null, so a query for the null value finds it. `false` leaves such a document out of this index: it exists, and other indexes and `$id` still reach it, but this index does not. A document with only some of the properties missing is entered either way.

DPNS sets `"nullSearchable": false` on its `records.identity` index, so domains that point at no identity take no room in it.

`nullSearchable: false` is refused on an index with a ranking or a `timeRange`, and on an index of an index-only type.

## Null handling

A property is null in an index when the document leaves it out. Putting the rules above together:

| The document's indexed values | Entered in the index? | Held to `unique`? |
|---|---|---|
| all present | yes | yes |
| some missing | yes, under null for the missing ones | no |
| all missing | only if `nullSearchable` is `true` | no |

The storage shape of each case is in [Null Handling](../drive/indexes.md#null-handling).

## Changing indexes

Drive builds an index's trees when the document type is registered and never backfills them, so an index added later would miss every existing document, and a removed or changed one would leave orphaned trees. A contract update may therefore not add, remove or change any index of an existing document type. From protocol version 14 the update is refused with `DataContractInvalidIndexDefinitionUpdateError` (10217), naming the first index that differs. Indexes are compared by name: reordering the `indices` array is no change, and renaming an index is a removal plus an addition. Earlier protocol versions refused every such change as well, though not always with this error.

A document type that the update adds may declare any indexes, as a new contract may.

## Limits

| Limit | Value |
|---|---|
| Indexes per document type | 10 |
| Properties per index | 10 |
| Characters in an index name | 32 |
| Characters in an index property path | 256 |
| `maxLength` of an indexed string | 63 (lower on a ranked index) |
| `maxItems` of an indexed byte array | 255 (lower on a ranked index) |
| Contested indexes per document type | 1 |

## More index keywords

An index entry may carry more keywords, each with its own chapter:

- [Contested Indexes](contested.md): `contested` turns a unique index into a scarce resource that masternodes award by vote, the way DPNS gives out names.
- [Counts, Sums and Averages](aggregates.md): `countable`, `summable`, `averageable` and their `range*` forms keep totals per indexed value, so counts, sums and averages are read without walking the documents.
- [Ranked Indexes](ranked.md): `rankedCountable`, `rankedSummable` and `rankedAverageable` order the indexed values by those totals, for "top 10" queries with proofs.
- [Time-Range Indexes](time-range.md): `timeRange` groups documents into time windows, for "trending this hour" queries.
- [Index-Only Types](index-only.md): `terminal`, `preallocated` and `skipIfAbsent` shape the indexes of a type whose documents live only in their indexes.

## See also

- [Indexes](../drive/indexes.md) in the Drive part: the index trie, the GroveDB layout and the query picker.
- [Contested Indexes](contested.md), [Counts, Sums and Averages](aggregates.md), [Ranked Indexes](ranked.md), [Time-Range Indexes](time-range.md), [Index-Only Types](index-only.md).
- [System Properties](system-properties.md) for what `$ownerId`, `$createdAt` and the others hold.
- [Contract Keywords](../contract-keywords.md) for the conventions of these chapters.
