# Indexes (indices)

A document type's `indices` list says which queries its documents can answer and which values must be unique. Without an index, a query can only address a document by its `$id`. With one, Drive keeps the documents sorted by the index's properties, so a query that fixes those properties reaches the matching documents directly instead of reading the whole type. Every index costs storage and processing on each write, and an index can never be added, removed or changed once the document type exists, so a contract author decides them before the document type is registered.

This chapter covers the keywords every index can use: `name`, `properties`, `unique`, `nullSearchable` and `skipIfAbsent`. The other index keywords, for contests, counts, sums, rankings, time windows and index-only types, have their own chapters (see [More index keywords](#more-index-keywords)).

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
- **A value of the document a reference points at**, from protocol version 14: `"<reference property>.<field>"`, such as `postId.$ownerId`, read from the referenced document and never stored. See [Values of Referenced Documents](derived-index-properties.md).

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

A unique index with a [`timeRange`](time-range.md) or an [`integerRange`](integer-range.md) treats two documents in the same window as equal on the bucketed property. A unique index cannot carry a ranking.

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

`nullSearchable: false` is refused on an index with a ranking, a `timeRange` or an `integerRange`, on an index of an index-only type, and together with `skipIfAbsent`.

## `skipIfAbsent`

| | |
|---|---|
| **Where** | index |
| **Value** | `true`, or an array of 1 to 10 of the index's property names |
| **Default** | `false` |
| **Since** | protocol version 14 |
| **On update** | Fixed (10217) |

A document that leaves out a property of the index's **skip set** writes nothing into this index, and its delete looks for nothing there. `true` makes the skip set every optional property of the index; an array names it. A [value of a referenced document](derived-index-properties.md#skipping-a-document-without-the-value) is never in the `true` set, since whether it can be absent depends on the referenced type; the array may name one. The skip property can sit anywhere in the index, under a [`timeRange`](time-range.md) or [`integerRange`](integer-range.md) window too. The index then holds only the documents carrying every skip property, and its counts and rankings are "among the documents that have them". A present but empty value is not absent and is indexed.

```json
"post": {
  "type": "object",
  "documentsMutable": true,
  "indices": [
    {
      "name": "byHashtagLanguageTime",
      "properties": [{ "hashtag": "asc" }, { "language": "asc" }, { "$createdAt": "asc" }],
      "skipIfAbsent": ["hashtag"]
    },
    {
      "name": "byDayHashtag",
      "properties": [{ "$createdAt": "asc" }, { "hashtag": "asc" }],
      "rangeCountable": true,
      "rankedCountable": true,
      "timeRange": { "on": "$createdAt", "range": 86400, "step": 86400 },
      "skipIfAbsent": true
    }
  ],
  "properties": {
    "hashtag": { "type": "string", "maxLength": 61, "position": 0 },
    "language": { "type": "string", "maxLength": 8, "position": 1 },
    "text": { "type": "string", "maxLength": 280, "position": 2 }
  },
  "required": ["$createdAt", "text"],
  "additionalProperties": false
}
```

An untagged post writes nothing into either index: no entry under `hashtag`, and no day window in `byDayHashtag`, whose ranking of today's hashtags therefore never has to step over an empty hashtag. A tagged post without a language is entered into `byHashtagLanguageTime` under null for `language`, since the array leaves `language` out of the skip set. A replace that adds or drops the hashtag moves the post into or out of both indexes.

A query only uses a skip index when it constrains every skip property, so it cannot silently miss the documents the index skipped. On a stored type the constraint must be one no missing value can meet: an equality or `in` with values that are neither null nor empty (an empty byte array is keyed like null), a range with a non-empty lower bound, `startsWith` with a non-empty prefix, or ranking by the property. Ordering by the property alone, or a range with only an upper bound, does not count: an index that does not skip would return the documents without the property too (they sort first, under null).

Rules at registration:

- The skip set is not empty: an index whose properties are all required could never skip.
- Each skip property is a top-level property of the type, not a system property, and not listed in `required`, or a [value of a referenced document](derived-index-properties.md#skipping-a-document-without-the-value) that can be absent.
- No `rankedCountable` `at` level sits above the index's deepest skip property: it would count only documents carrying that property, and no query could read it without binding the property.
- The index is not contested, and does not also set `nullSearchable: false` (the skip already leaves out a document with every indexed value missing).
- On a stored type, a skip property that is a byte array sets `minItems` to at least 1 (on the referenced type, for a value read from a referenced document): an empty byte array is keyed like a missing value.
- On a stored type, a ranking at a skip property's level does not share that level with an index that keeps null for the property: that index would create the null value, and the ranking would show it as a group with no documents.
- On an [index-only type](index-only.md#skipifabsent) the skip set holds every optional property of the index, and each optional property needs a skip index of its own, without a `timeRange` (a window keeps the value only until the window drains).

Respelling `true` as the array of the same properties, or the other way round, is no change on a contract update.

## Null handling

A property is null in an index when the document leaves it out. Putting the rules above together:

| The document's indexed values | Entered in the index? | Held to `unique`? |
|---|---|---|
| all present | yes | yes |
| some missing | yes, under null for the missing ones | no |
| all missing | only if `nullSearchable` is `true` | no |

An index with `skipIfAbsent` leaves out a document missing any of its skip properties, whatever the other values are.

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
- [Integer-Range Indexes](integer-range.md): `integerRange` groups documents into windows of an integer property, for counts and rankings per price or score band.
- [Index-Only Types](index-only.md): `terminal`, `preallocated`, `skipIfAbsent` and `outlivesDelete` shape the indexes of a type whose documents live only in their indexes.
- [Values of Referenced Documents](derived-index-properties.md): an index property `"<reference property>.<field>"` holds a value of the document a reference points at, which the document does not store.

## See also

- [Indexes](../drive/indexes.md) in the Drive part: the index trie, the GroveDB layout and the query picker.
- [Contested Indexes](contested.md), [Counts, Sums and Averages](aggregates.md), [Ranked Indexes](ranked.md), [Time-Range Indexes](time-range.md), [Integer-Range Indexes](integer-range.md), [Index-Only Types](index-only.md).
- [System Properties](system-properties.md) for what `$ownerId`, `$createdAt` and the others hold.
- [Contract Keywords](../contract-keywords.md) for the conventions of these chapters.
