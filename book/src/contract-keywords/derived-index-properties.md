# Values of Referenced Documents

An index may hold a value that the document does not store, read from the document one of its references points at. A reply indexed by `postId.$ownerId` is filed under the owner of the post it replies to, so "every reply to my posts" is one query, and the reply never stores that owner. Drive reads the value from the post whenever it writes or removes the reply's index entries. This saves storing a copy of the value in every document (32 bytes for an identifier). In exchange, every write that moves the entries reads the referenced document again.

## Example

A reply to a post only the moderators take down:

```json
"post": {
  "type": "object",
  "canBeDeleted": false,
  "moderatorAbilities": { "delete": true },
  "properties": {
    "text": { "type": "string", "minLength": 1, "maxLength": 280, "position": 0 }
  },
  "required": ["text"],
  "additionalProperties": false
},
"reply": {
  "type": "object",
  "documentsMutable": true,
  "canBeDeleted": true,
  "immutable": ["postId"],
  "indices": [
    { "name": "toPostOwner", "properties": [{ "postId.$ownerId": "asc" }, { "$createdAt": "asc" }] }
  ],
  "properties": {
    "postId": {
      "type": "array", "byteArray": true, "minItems": 32, "maxItems": 32,
      "contentMediaType": "application/x.dash.dpp.identifier",
      "refersTo": { "type": "moderatedDocument", "documentType": "post" },
      "position": 0
    },
    "body": { "type": "string", "minLength": 1, "maxLength": 280, "position": 1 }
  },
  "required": ["postId", "body", "$createdAt"],
  "additionalProperties": false
}
```

A reply is created with its own properties only, `postId` and `body`. Its author may edit `body` but never repoint `postId`. A query for the replies to one identity's posts, newest first:

```json
{
  "where": [["postId.$ownerId", "==", "<identity id>"]],
  "orderBy": [["$createdAt", "desc"]]
}
```

The replies come back as stored, without the owner: the proof ties them to it through the index path.

A post whose removal record keeps a field can have its replies filed under that field too. With `"documentsMutable": false` and `"moderatorAbilities": { "delete": true, "deleteKeepsFields": ["hashtag"] }` on the post, a reply index on `postId.hashtag` files every reply under the hashtag of the post it replies to, before and after a moderator removes the post.

## The index property

| | |
|---|---|
| **Where** | a property of an index of a document type that is not `indexOnly` |
| **Value** | `"<reference property>.<field>"` |
| **Since** | protocol version 14 |
| **On update** | Fixed, as every index is (10217) |
| **Errors** | `InvalidContractStructure` (10231); for a field an index can not key, `InvalidIndexPropertyTypeError` (10206) or `InvalidIndexedPropertyConstraintError` (10205) |

The name reads through a reference when its first segment names a top-level identifier property that declares a `refersTo`. An identifier has no nested properties, so the name can not mean a nested property. The field is what follows the first `.`:

- `$ownerId`: the owner of the referenced document.
- `$creatorId`: the identity that created the referenced document, on a type that records it (see [System Properties](system-properties.md)).
- A schema property of the referenced document type, by its dotted path, with that property's type.

## What Drive does

- **Create.** The reference is validated as usual, which reads the referenced document, and Drive writes the entries under the values it holds, without reading it again. A referenced document that has no value for the field, or a reference left out, puts the document under null, as a missing property would, or leaves it out of an index that [skips on the property](#skipping-a-document-without-the-value).
- **Replace, transfer, purchase, price update and a moderator's change of fields.** Drive reads the referenced document once, for both versions of the document, since neither can point elsewhere. It reads it even when no index holding a derived value moves: every entry's reference is rewritten in place on an update, under the values it is filed by.
- **Delete, a moderator's removal, and expiry by `ttl`.** Drive reads the referenced document to find the entries to remove.
- **A removed referenced document.** Once a moderator removes a `moderatedDocument` target, Drive reads its owner, and any field its type keeps under [`deleteKeepsFields`](deletion.md#moderatorabilitiesdeletekeepsfields), from the removal record, which keeps them as the document held them. So the entries are found under the values they were written under. A restored document is read again.

Each read is billed as a processing fee with the write that makes it. A dry run prices the reads at their worst case and reads nothing, pricing the entries under a value of the field's type and typical size.

## Rules at registration

A derived value must stay what it was when an entry was written, or Drive could not find the entry again. So:

- The reference is a `permanentDocument` or a `moderatedDocument` one, by id, to a document type of the same contract. A `deletableDocument` target could leave state without a record, and a `findBy`, an `inList` or an operand of an [expression](refers-to-expressions.md) does not name the document by id.
- The reference property is fixed once written: the type's documents are immutable, or the property is listed under [`immutable`](mutability.md) without a condition, and it is not one of the fields only moderators write.
- Through a `moderatedDocument` reference, the field is `$ownerId`, or a schema property the referenced type lists under [`moderatorAbilities.deleteKeepsFields`](deletion.md#moderatorabilitiesdeletekeepsfields), or one inside an object listed there: the removal record keeps these, and Drive reads them from it once the document is removed. Not `$creatorId`, which no record keeps. The list is fixed when the referenced type is registered, so an update adding a referring type can read only what it already lists.
- `$ownerId` only of a type whose documents can not be transferred or traded, and `$creatorId` only of a type that records it.
- A schema property must exist on the referenced type, be stored (not `transient` or inside a transient object), be fixed once written there under the same rule as the reference property, and be one an index can key: not an object or an array, a string of at most 63 characters, a byte array of at most 255 bytes, tighter on a [ranked index](ranked.md).
- Not `$id` of the referenced document, which is the reference property itself: index that instead. Nor any other system property.
- The index is not `unique` or contested: a uniqueness check reads the values the create carries, and a derived value is not among them.
- The derived property is not the source of a `timeRange` or an `integerRange`.
- The type is not `indexOnly`: its entries hold every value of its documents, and its delete carries them.

## Skipping a document without the value

A derived property can be a skip property of [`skipIfAbsent`](indexes.md#skipifabsent). The index then leaves out a document whose reference is absent, or whose referenced document has no value for the field, instead of filing it under null. A reply that may quote a post, filed by the owner of the post it quotes:

```json
{
  "name": "byQuotedOwner",
  "properties": [{ "quoteId.$ownerId": "asc" }, { "$createdAt": "asc" }],
  "skipIfAbsent": ["quoteId.$ownerId"]
}
```

A reply quoting no post writes nothing into this index, and its delete looks for nothing there. Since neither the reference nor the field can change once written, a replace never moves a document into or out of such an index through its derived values.

Only the array form names a derived property. `skipIfAbsent: true` skips on the type's own optional properties: whether a derived value can be absent depends on the referenced type.

At registration, a derived skip property must be able to be absent: its reference property is not in `required`, or the field is a schema property the referenced type does not require, or one inside an object it does not require. `$ownerId` and `$creatorId` are absent only with the reference. A byte array the property reads must set `minItems` to at least 1 on the referenced type, since an empty one is keyed like a missing value. Every other [`skipIfAbsent` rule](indexes.md#skipifabsent) applies as to any skip property.

## Queries

A query uses a derived property as any other index property: `==`, `in`, a range, `orderBy`. Its values encode as the field's type.

A `startAt` or `startAfter` cursor places the page by the values the document it names stores, and a derived value is not among them. A query paging with a cursor must therefore fix every derived property of its index with `==`. Otherwise it is refused, and it can page by a range on another property of the index instead.

A subscription to state transitions filters by the values a transition carries, and a derived value is not among them either, so a subscription filter can not name a derived property. Filter by the reference property instead (`postId`), or query the index.

## See also

- [Indexes (indices)](indexes.md) for the index keywords every index takes.
- [References (refersTo)](refers-to.md) for `permanentDocument` and `moderatedDocument`.
- [Mutability](mutability.md) for `immutable`, and [Moderator Abilities](moderator-abilities.md) for removal records.
