# Index-Only Types

Some documents are nothing but a position: a like says which post, which hashtag and which identity, and nothing else. Stored as an ordinary document, a like pays for a serialized body, a row in the primary tree and a reference in every index, for a fact its index entries already hold. An **index-only** type stores no body and no row: its index entries are its documents. That cuts the storage of a small document by more than half, and makes each index a uniqueness rule. In exchange, its documents can only be created and deleted, every property must live in an index or in the entry's value, and a query returns documents rebuilt from index entries rather than fetched by `$id`.

Seven keywords shape an index-only type: `indexOnly` and `entryPayload` on the document type, and `terminal`, `preallocated`, `summableOffCountIndex`, `skipIfAbsent` and `outlivesDelete` on its indexes. All of them arrived at protocol version 14 and are fixed once the type exists.

## Example

A `like` of a social contract whose `post` type cannot be deleted:

```json
"like": {
  "type": "object",
  "indexOnly": true,
  "documentsMutable": false,
  "canBeDeleted": true,
  "indices": [
    {
      "name": "byHashtagPost",
      "properties": [{ "hashtag": "asc" }, { "postId": "asc" }],
      "countable": "countable",
      "rangeCountable": true,
      "rankedCountable": true,
      "skipIfAbsent": true
    },
    {
      "name": "byPost",
      "properties": [{ "postId": "asc" }],
      "countable": "countable",
      "rangeCountable": true,
      "rankedCountable": true
    },
    { "name": "byLiker", "properties": [{ "$ownerId": "asc" }], "terminal": "postId" }
  ],
  "properties": {
    "hashtag": { "type": "string", "minLength": 1, "maxLength": 63, "position": 0 },
    "postId": {
      "type": "array", "byteArray": true, "minItems": 32, "maxItems": 32,
      "contentMediaType": "application/x.dash.dpp.identifier",
      "refersTo": {
        "type": "permanentDocument",
        "documentType": "post",
        "where": { "hashtag": "hashtag" }
      },
      "position": 1
    }
  },
  "required": ["postId"],
  "additionalProperties": false
}
```

`byPost` holds one entry per post and liker (its terminal defaults to `$ownerId`), so an identity can like a post once, and it counts and ranks posts by likes. `byHashtagPost` ranks the posts under each hashtag, and only likes that carry a hashtag enter it. `byLiker` lists the posts one identity liked. The reference makes sure the post exists and that a like's hashtag is its post's.

## `indexOnly`

| | |
|---|---|
| **Where** | document type |
| **Value** | boolean |
| **Default** | `false` |
| **Since** | protocol version 14 |
| **On update** | Fixed (`DocumentTypeUpdateError`, 40212) |
| **Errors** | `DuplicateUniqueIndexError` (40105), `DocumentNotFoundError` (40101), `InvalidDocumentTransitionActionError` (10404) |

`true` stores the type's documents only as index entries. Each entry sits under the index's values, is keyed by the terminal's values in place of the document id, and holds a 32-byte **row commitment**: a hash over all of the document's values that ties its entries in the different indexes together as one document.

**Create.** A create writes one entry into each index. If any of those entries already exists the create is refused with `DuplicateUniqueIndexError` (40105), so every index is a uniqueness rule over its properties and its terminal. `refersTo` and the other property checks run as on any document.

**Delete.** A document has no id to delete by. It is deleted with an `indexOnlyDelete` transition that carries all of its values (and `$createdAt` when the type requires it); every entry those values produce must exist and carry the matching row commitment, or the delete is refused with `DocumentNotFoundError` (40101). Deleting by id on an index-only type, or with `indexOnlyDelete` on an ordinary type, is refused with `InvalidDocumentTransitionActionError` (10404). The owner can only ever reach its own entries, since every index that keeps entries holds `$ownerId`; a [`summableOffCountIndex`](#summableoffcountindex) counter moves only with the entry the delete removes from its source. `canBeDeleted: false` forbids deletes as on any type.

**No other action.** A document cannot be replaced, transferred, sold or repriced.

**Queries.** A query goes through one index and returns documents rebuilt from its entries: the index's properties, the terminal, and `$ownerId` and `$createdAt` where the index holds them. A query through an index that holds only some of the properties yields only those with a proof; without one it is refused (`Unsupported`), since the documents could not be serialized, and so is a chained or composite read of them. The rebuilt `$id` is a hash of the entry's position and addresses nothing, so there is no fetch by `$id` and no `startAt` cursor; a query pages by the terminal instead (`postId > <last seen>`, with a limit). A query that sets the terminal with an equality can put an `in` on the index's last property, with a limit of at least its number of values; a range on an index property in such a query is refused, because its pages could hold fewer rows than exist. List the equality-bound properties first in an index, and page by a range on its terminal instead. The proof that a create or delete took effect is the presence or absence of its entry in the **proof index**, an index that involves no `$createdAt`, does not skip and keeps entries.

Rules at registration:

- `documentsMutable: false`; `transferable` `0`; `tradeMode` `0`; no `documentsKeepHistory` or `keeps*History`; no `transient` properties.
- None of the document type aggregate keywords (`documentsCountable` and the others): the type has no primary tree. The index keywords of [Counts, Sums and Averages](aggregates.md) and [Ranked Indexes](ranked.md) are allowed.
- At least one index. No index is `unique` or `contested`, and none sets `nullSearchable: false`.
- Every index holds `$ownerId`, as a property or in its terminal, except a [`summableOffCountIndex`](#summableoffcountindex) index, which keeps counters instead of entries.
- The only system properties an index may list are `$ownerId` and `$createdAt`, and an indexed `$createdAt` must be in `required`.
- No index declares [`integerRange`](integer-range.md): rows that differ only in the bucketed integer would claim the same entry in the windows they share.
- At least one index involves no `$createdAt`, does not set `skipIfAbsent` and keeps entries (it is no `summableOffCountIndex` index): the proof index.
- Every property is in `required`, except a skip property of a `skipIfAbsent` index. An object holding an indexed property is required too.
- Every required property appears in at least one index that does not skip and keeps entries, as a property or a terminal component, except the `entryPayload` properties and a property a `summableOffCountIndex` index's source fixes. Every optional property appears in a skip index without a `timeRange` whose skip set is that property alone, except, again, one such a source fixes.
- The type cannot also set [`ttl`](ttl.md) or `moderatorAbilities.delete`, and no document reference can target it: a `refersTo` `findBy` finds no unique index in it, and a reference by id (or with `inList`) is refused when the referring contract is registered (`ReferencedDocumentTypeIndexOnlyError`, 40146), since its documents can not be fetched by `$id`.

A property a `summableOffCountIndex` index's source fixes may sit in no index that keeps entries: in the [`summableOffCountIndex`](#summableoffcountindex) example, a like's `postAuthor`, which the `postId` reference's `where` entry `"$ownerId": "postAuthor"` fixes to the post's owner, sits only in `byAuthorPost`. No query returns such a property: a proved read gives documents without it, required or not, and since no index of the type then holds every property, every documents read of the type without a proof is refused. Its value is the referenced document's (here the post's `$ownerId`), so a client building a delete reads it back from there.

## `entryPayload`

| | |
|---|---|
| **Where** | document type |
| **Value** | array of 1 to 16 distinct property names, each 1 to 64 characters |
| **Default** | absent |
| **Since** | protocol version 14 |
| **On update** | Fixed (40212). The list is read as a set, so reordering it is no change. |

The properties stored in each entry's value, after the row commitment, instead of in a key. They are for data the application reads but never queries by, such as a public key or a ciphertext: they need not be indexed, and they come back with every query result.

```json
"indices": [{ "name": "byRequest", "terminal": ["appEphemeralPubKeyHash", "$ownerId"] }],
"entryPayload": ["walletEphemeralPubKey", "encryptedPayload"]
```

With a flat index keyed by a request hash and the responder, this is a key-value table: a query on the hash returns every responder with its public key and ciphertext.

Rules at registration:

- Only on an `indexOnly` type.
- Each entry names a top-level property that is `required`, a scalar (not an object or an array of values), and bounded: `maxLength` on a string, `maxItems` on a byte array.
- A payload property appears in no index, neither as a property nor in a terminal.
- The largest size each payload property can take, plus two bytes each, adds up to at most 5120 bytes. With several indexes, the payload is repeated in every entry.

## `terminal`

| | |
|---|---|
| **Where** | index of an `indexOnly` type |
| **Value** | a property name, or an array of 1 to 10 distinct names |
| **Default** | `"$ownerId"` |
| **Since** | protocol version 14 |
| **On update** | Fixed, like every index (`DataContractInvalidIndexDefinitionUpdateError`, 10217) |

Where an ordinary index keys each entry by the document id, an index-only index keys it by the terminal's values: the **member key**. There is one entry per index values and member key, so the terminal decides what the index makes unique. `byPost` above, with the default terminal, allows one like per post and owner; `byLiker`, with `postId` as terminal under `$ownerId`, holds the same pairs the other way round.

An array is a composite terminal whose values are joined in order. An index with no `properties` at all is a **flat** index, keyed by its terminal alone, as `byRequest` above is.

A query that fixes every property of the index can test one member key ("did I like this post") or walk the member keys in order, a page at a time.

Rules at registration:

- Only on an `indexOnly` type.
- Each component is `$ownerId` or a property of the type that could be indexed: not an object or an array of values, a string with `maxLength` of at most 63, a byte array with `maxItems` of at most 255. No other system property.
- Every component but the last has a fixed width: a byte array with `minItems` equal to `maxItems`, an identifier, an integer or a boolean. A string can only be last.
- The whole member key is at most 255 bytes. On a flat index, the level key, the component names each preceded by a zero byte, is at most 255 bytes as well.
- A component is not one of the index's `properties`, and not an optional property.
- A flat index takes no count, sum, ranking, `timeRange`, `integerRange`, `skipIfAbsent` or `preallocated` keyword.

## `preallocated`

| | |
|---|---|
| **Where** | index of an `indexOnly` type |
| **Value** | boolean |
| **Default** | `false` |
| **Since** | protocol version 14 |
| **On update** | Fixed (10217) |

The first entry under a new set of values pays for every tree on its path; later ones pay for one entry. When the whole path is decided by a referenced document, `preallocated: true` creates the trees when that document is created, paid by its creator, so every entry costs the same from the first one on. Deleting the last entry keeps the trees, so a post with no likes still shows in the rankings with a count of zero, and a range count, sum or average grouped by the last property returns it with zero, counted in a page's limit, so without an `IN` a short page means the range ended (across an `IN`, a value whose range holds nothing takes a place in the limit too, so there a short page may not be the end). Only documents created once the index exists get the trees: when a contract update adds the referring type, the referenced documents already stored get theirs from their first entry, as without `preallocated`.

In the example, `byHashtagPost` could be preallocated: `postId` is the reference and `hashtag` agrees with the post's. `byLiker` could not, since no post decides who likes it.

Rules at registration:

- Only on an `indexOnly` type.
- Every index property is either a property with a `permanentDocument` or `moderatedDocument` reference to a type of the same contract, or a referring value of that reference's `where`. A `deletableDocument` reference does not qualify, since the trees would outlive a deleted target with nothing left to say what they were keyed by. `$ownerId` may only be the terminal.
- Through a `moderatedDocument` reference, every key of the path must be kept by the referenced document's [removal record](deletion.md#moderatorabilitiesdeletekeepsfields): each `where` entry the index uses compares the referenced `$id`, `$ownerId` or a property the referenced type lists under `moderatorAbilities.deleteKeepsFields` (or one inside an object listed there), never `$creatorId` (`InvalidContractStructure`, 10231). The trees then outlive a removed document the way its record does, and a moderator's restore finds them in place. As through a `permanentDocument` reference, they are keyed by the values the document was created with: a value changed afterwards leaves them empty, and the first entry under the new value builds its own.
- The referenced property of each such `where` entry holds at most 255 bytes, since creating a referenced document makes its value an index key (40126 when the contract is created or updated).
- Not with `timeRange` or `integerRange`.

A referenced document whose agreed value takes more bytes than the referring property can hold preallocates nothing for that index, since no entry could agree with it.

## `summableOffCountIndex`

| | |
|---|---|
| **Where** | index of an `indexOnly` type |
| **Value** | the name of another index of the type, its source |
| **Default** | none |
| **Since** | protocol version 14 |
| **On update** | Fixed (10217) |

A like counted by post, by author and by hashtag is written three times: once in `byPost` and once in each of the other two, which hold nothing `byPost` does not already hold. When every like of a post lands in the same author and the same hashtag, the other two only need to know how many likes each post has. An index with `summableOffCountIndex` keeps exactly that: one counter per group, holding the number of entries its source index keeps for it, in place of an entry per document. A like then adds one to two counters instead of writing two more entries.

Here the like of the [example](#example) also carries its post's author, fixed through the reference's `where` like its hashtag:

```json
"properties": {
  "postId": {
    "type": "array", "byteArray": true, "minItems": 32, "maxItems": 32,
    "contentMediaType": "application/x.dash.dpp.identifier",
    "refersTo": {
      "type": "permanentDocument",
      "documentType": "post",
      "where": { "hashtag": "hashtag", "$ownerId": "postAuthor" }
    },
    "position": 0
  },
  "hashtag": { "type": "string", "minLength": 1, "maxLength": 59, "position": 1 },
  "postAuthor": {
    "type": "array", "byteArray": true, "minItems": 32, "maxItems": 32,
    "contentMediaType": "application/x.dash.dpp.identifier",
    "position": 2
  }
},
"required": ["postId", "postAuthor"]
```

and counts its likes per author and post:

```json
{
  "name": "byAuthorPost",
  "properties": [{ "postAuthor": "asc" }, { "postId": "asc" }],
  "summableOffCountIndex": "byPost",
  "rangeCountable": true,
  "rangeSummable": true,
  "rankedSummable": { "at": ["postAuthor", "postId"] },
  "rankedAverageable": { "at": ["postAuthor"] },
  "preallocated": true
}
```

Each counter adds its entries to the sums of the tree of the last property and, with `rangeCountable`, counts one group in its counts. A count query counts documents, so on this index it reads the sums, which hold the source's entries:

| Query | Reads | `byAuthorPost` |
|---|---|---|
| `count(*)` | the sums | an author's likes, or one post's |
| `sum(byPost)`, named by the source index | the sums | the same |
| `avg(byPost)` | the sums and the group counts | an author's posts and likes, so likes per post |

A ranking at an earlier level orders by these totals: `rankedSummable: { "at": "postAuthor" }` ranks authors by likes, `rankedAverageable: { "at": "postAuthor" }` by likes per post, and `"postId"`, the last property, ranks an author's posts by likes. A count or sum point query may stop at any level from the shallowest sum or average ranking down, and an average point query at any level that also carries counts (from the shallowest average ranking down). Pinned on every property but the last, any of them reads the tree of the last property instead when that property is not ranked: `postAuthor == A` alone gives A's likes, and with `rangeCountable` A's posts for an average. A ranked or `HAVING` count reads the sum rankings the same way: `count(*)` grouped by `postAuthor` ranks authors by likes. So on this index `rankedCountable` declares the same ranking as `rankedSummable`, and needs no `rangeCountable`: `rankedCountable: { "at": "postAuthor" }` ranks authors by likes too. A range count reads the sums over the range too: `count(*)` with `postAuthor == A` and a range on `postId`, grouped by `postId`, gives each of A's posts in the range with its likes (on a preallocated index a post nobody liked comes back with zero, and takes its place in a page's limit), and across an `IN` on `postAuthor` each author's. A range total (no grouping, or one per `IN` value) needs an index whose path passes through no ranked level, as everywhere: grovedb proves a range total only through unranked trees, and the unproven read refuses one too so both agree, so on this index, which ranks `postAuthor` and `postId`, group by `postId` instead. When the index is `preallocated`, every post created once the index exists has a counter from its creation, so a post without likes counts as a post with zero likes (a post stored before a contract update added the like type gets its counter from its first like, and keeps it at zero after); otherwise a post shows once it is liked and leaves with its last like.

Rules at registration:

- Only on an `indexOnly` type, with `rangeSummable: true` (the counters sit in the tree of the last property, which only `rangeSummable` makes a sum tree). No `summable`, `averageable`, `terminal`, `countable: "countableAllowingOffset"`, `timeRange`, `integerRange`, `outlivesDelete`, `unique` or `contested`.
- The source is another index of the type holding every document exactly once: it keeps entries (it is no `summableOffCountIndex` index itself), skips no document (`skipIfAbsent`), keeps no deleted one (`outlivesDelete`) and involves no `$createdAt`.
- One summed value per type: no index of the type declares `summable`, every `summableOffCountIndex` index names the same source, and no property shares the source's name.
- Every property of the source is a property of the index, so a group never counts two source groups.
- Every other property is a referring value of a `where` on a `permanentDocument` or `moderatedDocument` reference by id (no `findBy` or `inList`, whose key could move to another document) to a type of the same contract, held by a property of the source, and the referenced value never changes once written: `$id`, `$creatorId`, an `$ownerId` no transfer or trade changes, or a property the referenced type never lets change (a `where` names only the referenced type's properties, `$id`, `$creatorId` and `$ownerId`). An immutable `deletableDocument` reference counts only when it is required: a replace may clear an optional one once its document is deleted. Through a `moderatedDocument` reference, the value must also stay on the removal record ([`deleteKeepsFields`](deletion.md#moderatorabilitiesdeletekeepsfields)). Every like of one post then lands in one group.
- No other index continues below the index's last property, where the counter stands.
- `rankedSummable` and `rankedAverageable` take the `{ "at": ... }` form only on such an index.

A broken rule is refused as `InvalidContractStructure` (10231), or by the meta-schema as `JsonSchemaError` (10101): the meta-schema refuses the keyword without `rangeSummable: true` or next to `summable`, `averageable` or `terminal`, the `{ "at": ... }` form of `rankedSummable` or `rankedAverageable` without it, and a source name longer than 32 characters.

## `skipIfAbsent`

| | |
|---|---|
| **Where** | index |
| **Value** | `true`, or an array of the index's property names |
| **Default** | `false` |
| **Since** | protocol version 14 |
| **On update** | Fixed (10217) |

The keyword is described in [Indexes](indexes.md#skipifabsent): a document that leaves out a property of the index's skip set writes nothing into the index, and its delete looks for nothing there. It is the one way a property of an index-only type can be optional: in the example, a like without a hashtag is not in `byHashtagPost` and pays nothing for it. A skip property may sit below the first position, which is what lets a windowed ranking skip it:

```json
{
  "name": "byDayHashtagPost",
  "properties": [{ "$createdAt": "asc" }, { "hashtag": "asc" }, { "postId": "asc" }],
  "terminal": "$ownerId",
  "rangeCountable": true,
  "rankedCountable": { "at": ["hashtag", "postId"] },
  "timeRange": { "on": "$createdAt", "range": 86400, "step": 86400, "ttl": 604800 },
  "skipIfAbsent": true
}
```

An untagged like still enters the type's other indexes over the same day window, but writes nothing under `hashtag` in it.

What an index-only type adds to the rules of every type:

- An index path has no representation for a missing value, so every index holding an optional property skips on it: the skip set is every optional property of the index, and an array must name them all.
- Each optional property needs a skip index without a `timeRange` whose skip set is that property alone. A document carrying one optional property but missing another skips every index holding both, and its value would otherwise be written nowhere; a windowed index keeps it only until its windows drain, where document queries do not read it.
- An optional property is never a terminal.
- At least one index that involves no `$createdAt` does not skip: the proof index.

## `outlivesDelete`

| | |
|---|---|
| **Where** | `timeRange` index of an `indexOnly` type |
| **Value** | boolean |
| **Default** | `false` |
| **Since** | protocol version 14 |
| **On update** | Fixed (10217) |

A delete of an index-only document carries its values and removes the entries they address. A time window is keyed by the document's `$createdAt`, so without this keyword a delete must carry the exact timestamp to find the window's entries. With `outlivesDelete: true`, a delete leaves the index's entries where they are, and they expire with their window:

```json
{
  "name": "byTrendPost",
  "properties": [{ "$createdAt": "asc" }, { "postId": "asc" }],
  "terminal": "$ownerId",
  "countable": "countable",
  "timeRange": { "on": "$createdAt", "range": 259200, "step": 86400, "ttl": 604800 },
  "outlivesDelete": true
}
```

- **A delete carries no `$createdAt`** when every index involving it outlives deletes: the rows commit to no timestamp, and a delete that carries one is refused (`InvalidDocumentTransitionActionError`). An unlike needs only the like's other values, which any device knows.
- **A create writes over an entry already there.** When the same owner writes the same values again while a deleted document's entry still stands in a window, the create writes its own entry over it: the owner counts once, and the create is not refused as a duplicate. Only the index's other entries decide whether a create is a duplicate.
- **A deleted document keeps counting in the window** until the window moves past it. On a trending window, an unliked like still counts there for up to the window's `range`.
- **The executed-transition proof** runs against an index that does not outlive deletes, so a delete still proves the document gone.

Rules at registration:

- Only on an `indexOnly` type, on an index with a `timeRange` carrying a `ttl`, so the entries a delete leaves expire.
- Not with a sum (`summable`), and not on a type with `entryPayload`: a kept entry holds the amount or payload of the document that wrote it.
- Every schema property must also sit in an index that neither skips nor outlives deletes, which a delete checks.
- The index's key (its properties but `$createdAt`, and its terminal) must hold the whole key of an index a delete clears and that skips nothing, so no two documents in state share one of its entries. `[$createdAt, postId] → $ownerId` holds `byPost`'s `[postId] → $ownerId`; `[$createdAt] → $ownerId` holds no such key and is refused.
- The proof index may not outlive deletes.

## See also

- [Index-Only Document Types](../drive/index-only-document-types.md) for the entry layout, the row commitment, the full constraint list and the query surface.
- [Time Range](time-range.md) for the windows an `outlivesDelete` index needs.
- [Indexes](indexes.md), [Counts, Sums and Averages](aggregates.md) and [Ranked Indexes](ranked.md) for the index keywords an index-only type uses.
- [References (refersTo)](refers-to.md) for `permanentDocument` and `moderatedDocument` references and `where`, which `preallocated` relies on.
- [Mutability](mutability.md), [Deletion](deletion.md) and [Creation, Transfers and Trading](ownership-and-trading.md) for the flags an index-only type must set.
