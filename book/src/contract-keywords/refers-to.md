# References (refersTo)

An identifier (a 32-byte id) can hold any value. `refersTo` says what it points at, and Platform then checks, whenever a document is created or replaced, that the thing it names exists: an identity, a data contract, a token, a document, or one key of an identity. Reach for it when a document only makes sense next to something else: a reply needs its post, a join request needs the proposal it joins, an encrypted message needs the key it was encrypted to. The check runs when the document is written. Nothing checks the reference again when its target changes later, and nothing resolves it for a reader.

| | |
|---|---|
| **Where** | An identifier property, at the top level or inside an object; the `items` of a typed array of identifiers, where every element is checked; for one form of `identityPublicKey`, an integer key id property; and a string or byte array property whose value `findBy` reads, as is (see [A string or byte array key](refers-to-lookup.md#a-string-or-byte-array-key)) or through a function (see [Commit and reveal](refers-to-lookup.md#commit-and-reveal)). `ownerRefersTo` and `creatorRefersTo` carry the same declaration at the document type level. |
| **Value** | An object: `type`, naming one [target](#targets), with the [keys](#keys) that target takes; or an object holding only `anyOf` or only `allOf` (see [Expressions](refers-to-expressions.md)). |
| **Default** | Absent: the identifier is not checked against anything. |
| **Since** | protocol version 14 |
| **On update** | Fixed: adding, removing or changing any part of a declaration is refused (`IncompatibleDocumentTypeSchemaError`, 10246). |
| **Errors** | `ReferencedEntityNotFoundError` (40120) when the target does not exist; the full list is under [Errors](#errors). |

## Example

```json
"reply": {
  "type": "object",
  "documentsMutable": true,
  "canBeDeleted": true,
  "properties": {
    "postId": {
      "type": "identifier",
      "refersTo": { "type": "deletableDocument", "documentType": "post" },
      "position": 0
    },
    "mentions": {
      "type": "array", "minItems": 0, "maxItems": 5, "uniqueItems": true,
      "items": {
        "type": "identifier",
        "refersTo": { "type": "identity" }
      },
      "position": 1
    },
    "text": { "type": "string", "minLength": 1, "maxLength": 280, "position": 2 }
  },
  "required": ["postId", "text"],
  "additionalProperties": false
}
```

`postId` must be the id of a `post` document of this contract that exists when the reply is written. Posts can be deleted, so the reference is a `deletableDocument` one. Each of the up to five `mentions` must be the id of an existing identity. The type carries six references at most: one for `postId` and five for `mentions`.

## Targets

`type` names what the value points at.

| `type` | The value must be | Keys it takes |
|---|---|---|
| `identity` | the id of an existing identity | none |
| `contract` | the id of an existing data contract | `contractRequirements` |
| `token` | the id of an existing token | none |
| `permanentDocument` | the id of an existing document of a type whose documents can never disappear; with `findBy`, one part of the key that finds it; with `inList`, one of the identifiers a list on it holds | `documentType` (required), `contractId`, `findBy`, `where`, `inList`, and beside a `findBy` function `constraints` (`minimumAgeBlocks`) |
| `moderatedDocument` | the id of an existing document of a type whose documents disappear only when the contract's moderators remove them, on the record | `documentType` (required), `contractId`, `where` |
| `deletableDocument` | the id of an existing document of a type whose documents can disappear in any other way; with `findBy`, one part of the key that finds it | `documentType` (required), `contractId`, `findBy`, `where`, and beside a `findBy` function `consume` and `constraints` (`minimumAgeBlocks`, `chooseFirst`) |
| `identityPublicKey` | an identity key that exists and is not disabled | `keyIdProperty` or `identityProperty` (one of them, required), `keyRequirements` |

A key that belongs to another target is refused when the contract is registered.

### `identity`

The value is the id of an identity that exists. The check reads the identity's revision. Identities are never removed, so a validated reference never dangles.

### `contract`

The value is the id of a data contract that exists. [`contractRequirements`](#contractrequirements) can ask more of it: that it declares elected moderation, has reached a certain age, belongs to the writer, and so on. Contracts are never deleted.

### `token`

The value is the id of a token. The check reads the token's record, which is written when its contract is registered and never removed.

### `permanentDocument`

The value is the id of a document of `documentType`, in this contract or in the one `contractId` names. The referenced type must be one whose documents can never disappear: `canBeDeleted: false` (not `"onlyWhenConsumed"`), no `moderatorAbilities.delete` and no `ttl`. None of those can change on a contract update and document types are never removed, so a validated permanent reference never dangles.

```json
"reasons": {
  "type": "array", "minItems": 0, "maxItems": 64, "uniqueItems": true,
  "items": {
    "type": "identifier",
    "refersTo": { "type": "permanentDocument", "documentType": "reason" }
  },
  "position": 2
}
```

This is the moderation charters contract's `submittedCharter.reasons`: every element must be the id of a `reason` document, a type that is immutable and can never be deleted. With [`findBy`](refers-to-lookup.md) the value is instead one part of a unique index key that finds the document, and with [`inList`](refers-to-list-element.md) one of the identifiers a list on the document holds.

### `moderatedDocument`

The same for a type whose documents disappear only when the contract's moderators remove them, and never without a trace: `canBeDeleted: false`, no `ttl`, and [`moderatorAbilities.delete`](moderator-abilities.md) with `deleteKeepsRecord` left at its default, `true`. Every removal then leaves a [removal record](../data-model/contract-moderation.md#deleting-documents) under the contract, holding the document's id, its owner, the moderator, the time, the reason and a hash of the document, and nothing ever deletes it. So a validated moderated reference always resolves: to the document, or to the record of its removal, from which a moderator can [restore](../data-model/contract-moderation.md#restoring-documents) the document as it was.

```json
"postId": {
  "type": "identifier",
  "refersTo": { "type": "moderatedDocument", "documentType": "post" },
  "position": 0
}
```

A reply to a `post` its author can not delete, which the moderators can take down. The post must exist when the reply is written. Once a moderator removes it, the reply keeps pointing at it, and a replace of the reply is checked as if the reference were permanent: an edit of the reply's text passes, and the reference now resolves to the removal record. What a replace can not do is point a reply at a removed post, or write a new reply to one: a value the write sets must name a document in state.

A removed document has no values left but those its record keeps: its id, its owner, and the fields its type lists under [`moderatorAbilities.deleteKeepsFields`](deletion.md#moderatorabilitiesdeletekeepsfields). A `where` entry the replace checks again (its referring property changed, or it is a writer gate valued `"$ownerId"`) is checked against the record when it compares the referenced `$ownerId`, `$id` or a property the type keeps (a kept path, or one inside a kept object), with the same absence rule as against a document in state, and refused with `ReferencedDocumentRemovedError` (40145) when it compares any other property; the replace can repoint the reference at a document in state, or leave that property unchanged until the post is restored. A writer gate (a `where` entry valued `"$ownerId"`, or a `notEqual` bounded by it) is checked on every replace, so on a `moderatedDocument` reference it may compare only the referenced `$ownerId` or `$id` (`InvalidContractStructure`, 10231, otherwise), the only values sure to be in every record: a gate on anything else could refuse every replace once the post is removed. An ordering bounded by `$updatedAt` or `$transferredAt` (or one of their heights), and a `contains` of `"$ownerId"`, are checked on every replace too, and compare only a property the type keeps (40126 otherwise). A [rule reading the post](property-constraints.md#values-of-a-referenced-document) reads only its `$ownerId`, its `$id` and the properties its type keeps, for the same reason. The value is kept as long as it is the one the stored document held, whatever else the replace changed in the same object or list. The value is always the document's id: `findBy`, `inList` and operands of an [expression](refers-to-expressions.md) are refused, since a removal frees the unique index keys a `findBy` finds by.

A chained or composite query that joins through a moderated reference proves each removed document's record beside the documents it joins, and reports the removal with it.

An index of an `indexOnly` type may be [`preallocated`](index-only.md#preallocated) through a moderated reference when the removal record keeps every key of its path: each `where` entry the index uses compares the referenced `$id`, `$ownerId` or a property the type keeps. The trees then outlive a removed post the way its record does, and a restore finds them in place.

### `deletableDocument`

The same for a type whose documents can disappear without a record: deleted by their owner (`canBeDeleted`), deleted by a create that consumes them (`canBeDeleted: "onlyWhenConsumed"`), removed by the contract's moderators when `deleteKeepsRecord` is `false`, or removed by the platform when their `ttl` passes. A type whose documents only moderators remove, keeping records, is a `moderatedDocument` target instead, and a `deletableDocument` reference to it is refused. The document must exist when the referring document is written, and may be deleted afterwards.

Because the target may be gone, every replace of the referring document checks the reference again, whether or not the replace touched it. Once the target is deleted, the replace has to point the property at a document that exists or remove it. A required property cannot be removed, so a document whose required reference has lost its target can be replaced only after it is repointed; it can still be deleted. A single `deletableDocument` reference held by an `immutable` top-level property may be removed by a replace once its target is gone, an exception to the immutability rule.

### `identityPublicKey`

One key of one identity, which must exist and not be disabled. Identity keys can be disabled but never removed, so a validated key reference never dangles, while a disabled key refuses new writes. The declaration comes in two forms, which differ in which property carries it:

- **On the identity property.** The identifier holds the identity's id and [`keyIdProperty`](#keyidproperty-and-identityproperty) names the sibling integer property holding the key id. The moderation charters contract's `joinRequest.recipientId`:

  ```json
  "recipientId": {
    "type": "array", "byteArray": true, "minItems": 32, "maxItems": 32,
    "contentMediaType": "application/x.dash.dpp.identifier",
    "refersTo": {
      "type": "identityPublicKey",
      "keyIdProperty": "recipientKeyId",
      "keyRequirements": { "purpose": "decryption", "boundTo": "submittedCharter" }
    },
    "position": 1
  }
  ```

- **On the key id property.** The integer holds the key id and [`identityProperty`](#keyidproperty-and-identityproperty) names whose key it is. The property must declare exactly the range of a key id, `"minimum": 0` and `"maximum": 4294967295`. The same contract's `joinRequest.senderKeyId`, a key of the writer:

  ```json
  "senderKeyId": {
    "type": "integer", "minimum": 0, "maximum": 4294967295,
    "refersTo": {
      "type": "identityPublicKey",
      "identityProperty": "$ownerId",
      "keyRequirements": { "purpose": "encryption", "boundTo": "joinRequest" }
    },
    "position": 3
  }
  ```

A key reference pairs the value with one key id, so it is refused on the elements of a typed array, as an operand of an expression and in `ownerRefersTo` or `creatorRefersTo`.

## Keys

### `documentType`

The name of the referenced document type, 1 to 64 letters, digits or underscores. Required on `permanentDocument`, `moderatedDocument` and `deletableDocument`, and refused on the other targets. For `permanentDocument` the type's documents must never disappear; for `moderatedDocument` they must disappear only through a moderator's recorded removal; for `deletableDocument` they must be able to disappear otherwise. With `inList` it is the type holding the list.

### `contractId`

The contract holding `documentType`, as a base58 string or an array of 32 bytes. Absent means the declaring contract; naming the declaring contract's own id means the same. Only on the two document targets. A reference into another contract costs a billed fetch of that contract when a document is written, once per declaration: the elements of a typed array and the operands of an expression share it.

### `where`

States what the referenced document must hold once found. It takes 1 to 10 entries, on `permanentDocument`, `moderatedDocument` and `deletableDocument` references, each keyed by a property of the referenced document. An entry valued with a referring value, `{ "<referenced property>": "<referring value>" }`, must hold as an equality when the referring document is written; an entry valued with an object holds a literal, an inequality, a list or an ordering instead (see [Conditions](#conditions)). Two typed arrays compare as lists (see [Typed arrays](#typed-arrays)). `where` never finds the document: the value does, as its id, or [`findBy`](refers-to-lookup.md) does. `where` is checked against the document found.

- **The key** is the referenced side: a property of the referenced document type, a dotted path for a nested one, or one of the referenced document's own identifiers: `$ownerId` (its current owner, which follows it through transfers), `$creatorId` (its creator, which never changes, on types that record it, see [System Properties](system-properties.md)) or `$id` (its id). A system name needs an identifier on the referring side.
- **The value** is the referring side: a property of the declaring document type, a dotted path for a nested one, or `"$ownerId"`, the writer. An entry whose value is `"$ownerId"` is a write gate: only an identity whose id equals the referenced side may create or replace the document. A referring value may appear once among the equalities.
- **Absence counts.** Both sides absent agree; one side absent is a mismatch, as a different value is.
- **Values, not keys.** The two sides compare as values of their type: strings as text (so `""` is not `"\0"`), byte arrays and identifiers as bytes (an identifier equals the same 32 bytes, however it is carried), integers as numbers whatever width they are carried at, floats exactly, bit for bit (so `-0.0` is not `0.0`). A value of any length compares, past the 255 bytes of an index key too.
- The comparison reads the document already fetched for the existence check, so it costs nothing more. An entry that does not hold refuses the write with `ReferencedDocumentPropertyMismatchError` (40127); no document found is still `ReferencedEntityNotFoundError` (40120).
- **No `"."` and no functions.** They find the document, so they belong in [`findBy`](refers-to-lookup.md), where a function is the key of a [commit and reveal](refers-to-lookup.md#commit-and-reveal).

```json
"submittedCharterId": {
  "type": "identifier",
  "refersTo": {
    "type": "permanentDocument",
    "documentType": "submittedCharter",
    "where": {
      "$ownerId": "$ownerId",
      "targetContractId": "targetContractId"
    }
  },
  "position": 1
}
```

This is the moderation charters contract's `electedCharter.submittedCharterId`: the proposal it names must be owned by the writer, and must be for the same `targetContractId` as the elected charter. The same contract's `joinRequest.submittedCharterId` declares `"where": { "$ownerId": "recipientId" }`: the proposal's owner must be the join request's `recipientId`. Other patterns: `{ "$ownerId": "authorId" }` makes a like carry its post's owner in `authorId`, and `{ "$creatorId": "$ownerId" }` lets only the referenced document's creator write.

Read as SQL, `documentType` is the `FROM`, the value (or `findBy`) the part of the `WHERE` that finds the row, and `where` the rest of the `WHERE`, checked on the row found:

```sql
SELECT * FROM submittedCharter
WHERE $id = :submittedCharterId                  -- the value, an id reference
  AND $ownerId = :writer                         -- where
  AND targetContractId = :targetContractId       -- where
```

At registration both sides must exist and hold the same type of value (for integers, the same stored integer type). Neither side may be an object, a typed array pairs only with a typed array (see [Typed arrays](#typed-arrays)), the referring side may not be the reference property itself, and the referenced side may not be `transient` (no stored document carries it; the referring side may be). An entry through which a [`preallocated`](index-only.md#preallocated) index is keyed needs a referenced property whose every value fits an index key, at most 255 bytes: a string of at most 63 characters or with `maxBytes` at most 255, or a byte array of at most 255 bytes. For a pair of typed arrays, every element must fit. A condition's referenced side must hold a single value a literal is compared with (an integer, a boolean, a string, an identifier or a byte array, not a float); every literal must read as its kind and be a value the property can hold (one of its `enum` values, of a length or size it takes, inside its `minimum` and `maximum`); an `in` must list values distinct once read (`"cafe"` and `"CAFE"` are one byte array); a `notEqual` compares two values of one kind (the writer an identifier), never the reference property itself, whose value found the document; and an ordering compares an integer with an integer bound, a time or height the declaring type records, an integer bound leaving some value of the property on its side. An entry breaking one of these is refused with `ReferencedDocumentPropertyAgreementInvalidError` (40126), whose referring property names the condition as written.

#### Conditions

An entry may hold an object with one key instead of a referring value:

| Entry | Holds when the referenced side |
|---|---|
| `{ "const": v }` | equals the literal `v`. A side the document leaves out equals none |
| `{ "notEqual": b }` | differs from `b`: a property of the declaring type, `"$ownerId"` (the writer) or a `{ "const": v }`. It is the negation of an equality, so two absent sides are equal and one absent side differs |
| `{ "in": [v1, v2, ...] }` | equals one of two to thirty distinct literals of one kind. A side left out is in none |
| `{ "greaterThan": b }`, `{ "greaterThanOrEqual": b }`, `{ "lessThan": b }`, `{ "lessThanOrEqual": b }` | is an integer that compares with `b` this way: an integer property of the declaring type, an integer `{ "const": n }`, or a time or height the declaring type lists in `required` (`"$createdAt"`, `"$updatedAtBlockHeight"`, ...). It holds only when both sides have a value |

A literal is an integer, `true` or `false`, or a string, and it reads as the referenced property it is compared with: a string for a string property, a base58 identifier for an identifier (or `$ownerId`, `$creatorId`, `$id`), hex digits for a byte array (two a byte). The rule that a referring value appears once applies to the equalities only, so `"minPrice": { "lessThanOrEqual": "amount" }, "maxPrice": { "greaterThanOrEqual": "amount" }` bounds a bid by both ends of a range.

```json
"auctionId": {
  "type": "identifier",
  "refersTo": {
    "type": "permanentDocument",
    "documentType": "auction",
    "where": {
      "status": { "in": ["open", "extended"] },
      "reservePrice": { "lessThanOrEqual": "amount" },
      "endsAt": { "greaterThan": "$updatedAt" },
      "$ownerId": { "notEqual": "$ownerId" }
    }
  },
  "position": 0
}
```

A bid names an auction that is open or extended, whose reserve its `amount` meets, that has not ended at the block of the write (the bid type lists `$updatedAt` in `required`), and that its writer does not own. A time or height reads as the referring document records it: a create's block for all of them, and on a replace the stored creation and transfer and the replace's block as the update. `$createdAt` and its heights never change, while `$updatedAt` and `$transferredAt` and their heights do without the referring document changing (a replace, a transfer), so an entry bounded by one of them is checked again on every replace, as a writer gate is, and so is a `notEqual` bounded by `"$ownerId"`.

Conditions are checked when the reference is, against the same document, with the same refusal, 40127, whose referring property names the condition as written, in its declared order, a string quoted as JSON writes it (`{ "in": ["open", "extended"] }`). They work on every form of document reference: the elements of a typed array, a leaf of an [expression](refers-to-expressions.md), [`findBy`](refers-to-lookup.md) and [`inList`](refers-to-list-element.md). They never key a [`preallocated`](index-only.md#preallocated) index. Beside a `findBy` function, the property a condition is bounded by must be fixed once written or transient, as an equality's referring property must, and no condition may be bounded by `$updatedAt` or `$transferredAt` or their heights, which a later write changes while such a reference is judged on the create alone.

A condition reads absence as a `where` entry does, not as a [`propertyConstraints`](property-constraints.md#values-of-a-referenced-document) rule reading the same value: a `notEqual` bounded by a property finds two absent sides equal and refuses, where the rule's `notEqual` holds for a string or an identifier left out; an ordering needs both sides, where the rule reads an integer left out as 0. Moving a check from one to the other changes what a document leaving the value out may do.

Through a `moderatedDocument` reference a [`propertyConstraints`](property-constraints.md#values-of-a-referenced-document) rule reads, every replace checks the whole `where`, against the removal record once a moderator removes the document, so registration keeps every entry of such a reference, equality or condition, to `$id`, `$ownerId` and the fields the type lists under `deleteKeepsFields`.

#### Typed arrays

A typed array on the referenced side is compared as a list, in one of three ways:

| Entry | Holds when the referenced typed array |
|---|---|
| `"<referenced array>": "<referring array>"` | has the same elements as the referring typed array, in the same order |
| `{ "contains": b }` | holds an element equal to `b`: a property of the declaring type holding one value of the element kind, or `"$ownerId"` (the writer) for identifier elements |
| `{ "containsAll": b }` | holds every element of `b`, a typed array of the declaring type of the same element kind, in any order |

- **A list left out is the empty one**, as an indexed typed array without elements is absent. Two lists left out, or one left out and one empty, are equal; every list holds all of an empty `containsAll`; a list left out holds nothing, and a `contains` value the document leaves out is held by no list, as a value left out is `in` no list.
- **Elements compare by value**, exactly as two single values do (see **Values, not keys** above): an identifier equals its 32 bytes however a transition carries them, and an integer is the same whatever width it is carried at.
- **Billed by the element.** Unlike a comparison of single values, an entry comparing typed arrays is billed before it compares them, `where_list_credits_per_element` of the fee schedule (100 credits at protocol version 14) for each element it reads: both lists for an equality and a `containsAll`, the referenced list for a `contains`, a list left out none. A `containsAll` looks each element up in a set of the referenced ones, so its work grows with the lists' length, not its square.
- **At registration** (40126): the referenced side is a stored typed array whose elements are integers, booleans, strings, identifiers or byte arrays, never floats; the referring typed array of an equality or a `containsAll` is stored (not `transient`) and its elements share the referenced elements' value kind; a `contains` value is of the element kind; neither is the reference property itself. `const`, `in`, `notEqual` and the orderings compare single values and refuse a typed array.
- **Keying an index by the elements.** An equality between two typed arrays fixes the referring list to the referenced one, so it can key a [`preallocated`](index-only.md#preallocated) or [`summableOffCountIndex`](index-only.md#summableoffcountindex) index of an index-only type by the elements of the list, each element fitting an index key (see [Counting per element](index-only.md#counting-per-element)). `contains` and `containsAll` fix no element and never key one.
- A `contains` of `"$ownerId"` is a writer gate: only an identity the list holds may create or replace the document, and every replace checks it again. Through a `moderatedDocument` reference its list must be one the type keeps under `deleteKeepsFields`, since once a moderator removes the document the check reads the removal record.

```json
"postId": {
  "type": "identifier",
  "refersTo": {
    "type": "moderatedDocument",
    "documentType": "post",
    "where": {
      "tags": "tags",
      "$ownerId": "postAuthor"
    }
  },
  "position": 0
}
```

A like copies its post's `tags` (a typed array of at most 4 distinct strings) and its author, so a like can be found by tag without reading the post. `"members": { "contains": "$ownerId" }` lets only a member the post lists write, and `"tags": { "containsAll": "tags" }` lets a document name some of its post's tags in any order. `containsAny`, which the [document query](typed-arrays.md) has, is not a `where` entry.

The keyword `propertyAgreement`, whose pairs were keyed the other way (`{ "<referring property>": "<referenced property>" }`), was replaced by `where` before protocol version 14 shipped. A declaration still carrying it is refused on every parse, with a message saying to key `where` by the referenced document's property. `"propertyAgreement": { "recipientId": "$ownerId" }` is now `"where": { "$ownerId": "recipientId" }`, and a symmetric pair such as `"$ownerId": "$ownerId"` reads the same.

### `contractRequirements`

What a `contract` reference requires of the referenced contract beyond existing. It holds at least one of these keys:

| Key | Value | Met when |
|---|---|---|
| `moderation` | `"elected"` | the contract declares an elected moderation team |
| | `"electionOpen"` | the contract declares an elected team, and its own `electionDelay`, counted from the contract's creation, has passed at the block time of the write (or it declares no delay) |
| `minimumAgeSeconds` | integer, 1 to 4294967295 | the contract's recorded creation time is at least that many seconds before the block time of the write |
| `minimumSecondsSinceUpdate` | integer, 1 to 4294967295 | the same, counted from the later of its creation and its last update |
| `owner` | `"self"` | the contract is owned by the writer of the referring document |
| | `"other"` | the contract is owned by anyone else |
| `readonly` | `true` | the contract's config is `readonly`: it can never be updated again |
| `keepsHistory` | `true` | the contract's config keeps history |
| `ownerProtected` | `true` or `false` | the contract's elected moderation protects, or does not protect, its owner from the team; a contract without elected moderation meets neither |

A contract with no recorded creation time never meets `minimumAgeSeconds` or `minimumSecondsSinceUpdate`. The requirements are judged against the contract already fetched for the existence check, the writer and the block time, so they cost no further read. The first unmet one refuses the write with `ReferencedContractRequirementNotMetError` (40135); a contract that does not exist is still `ReferencedEntityNotFoundError` (40120).

The moderation charters contract uses both moderation values: a proposal (`submittedCharter.targetContractId`) needs a target that is `elected`, so teams can form while the target's election delay runs, and the charter that opens the election (`electedCharter.targetContractId`) needs it `electionOpen`:

```json
"refersTo": { "type": "contract", "contractRequirements": { "moderation": "electionOpen" } }
```

`owner` is the one requirement judged against the writer, and a transfer or a purchase changes the owner without a write. On a type whose documents can be transferred or traded, a reference carrying `owner` is therefore checked again on every replace, so a new owner has to repoint it at a contract that meets the requirement for them, or remove it where it is optional. Such a reference may not sit under an `immutable` property of such a type, which could never be repointed. See [Elected Moderation](../data-model/contract-moderation.md#elected-moderation) for the moderation values.

### `keyIdProperty` and `identityProperty`

The two forms of `identityPublicKey`. A declaration takes exactly one of them.

- **`keyIdProperty`**, on the identity property: the path of the integer property of the same document type holding the key id. It must exist, be an integer, and not carry a key reference of its own. If the identity is set and the key id is not, the write is refused with `ReferencedKeyIdPropertyInvalidError` (40125).
- **`identityProperty`**, on the key id property: whose key the value is.
  - `"$ownerId"`: the writer. The writer's existence is already proven, so the key fetch is the only read.
  - `"$creatorId"`: the document's creator, only on a type that records creator ids. A document written before its type recorded them has none, and setting the key id on it is refused (40125).
  - The path of an identifier property of the same type, which must exist, be an identifier and not carry an `identityPublicKey` reference of its own. A key id set while that property is not set is refused (40125).

A stored key id may not be paired with a `transient` identity, since the key id alone names no key. Each rule is checked at registration and refused with `ReferencedKeyIdPropertyInvalidError` (40125).

### `keyRequirements`

What an `identityPublicKey` reference requires of the key beyond existing and not being disabled. It holds at least one of:

- `purpose`: the key's purpose, one of `authentication`, `encryption`, `decryption`, `transfer`, `voting` or `owner`.
- `boundTo`: a document type of the declaring contract. The key must be bound to exactly this contract and that document type; a key bound to the whole contract or to a contract group does not meet it.

At registration `boundTo` must name a document type of the contract, and one a key of the required purpose can be bound to: only authentication, encryption and decryption keys carry a document type bound, an encryption key only where the type declares `requiresIdentityEncryptionBoundedKey`, and a decryption key only where it declares `requiresIdentityDecryptionBoundedKey` (see [Signing and Keys](signing-keys.md)). The requirements are judged against the key already fetched, and the first unmet one refuses the write with `ReferencedIdentityKeyRequirementNotMetError` (40136). A missing key is still 40123 and a disabled one 40124.

### `findBy`, `inList`, `anyOf` and `allOf`

- [`findBy`](refers-to-lookup.md) finds a `permanentDocument` or `deletableDocument` through the unique index of its type over exactly the properties it names, with the value as one part of the key. One entry may be a function, for a [commit and reveal](refers-to-lookup.md#commit-and-reveal), beside which the `refersTo` may declare `constraints` (`minimumAgeBlocks`, and on a `deletableDocument` [`chooseFirst`](refers-to-lookup.md#choosing-the-first-commitment-choosefirst)) and, on a `deletableDocument`, `consume`.
- [`inList`](refers-to-list-element.md), on a `permanentDocument` whose `findBy` is `{ "$id": <property> }`, names the list on that document the value must be in.
- [`anyOf` and `allOf`](refers-to-expressions.md) combine several targets in one declaration.

## How it works

### On create

When a document is created, every reference is checked against the current state: the `ownerRefersTo` or `creatorRefersTo` declaration first, then each property's. The first check that fails refuses the write with that check's error, naming the property by its path (`postId`, `meta.charterId`), an element by its list path (`mentions[2]` for the third) and the writer or creator reference as `$ownerId` or `$creatorId`. A refused write is still charged for the reads it made.

- A reference property the document leaves out is not checked. Whether it may be left out is up to `required`.
- Each check is a billed read: the identity, the contract (none for the declaring contract, which is already loaded), the token's record, the document by id or by `findBy`, or the key. One write fetches a given document by id once, however many references name it.
- On a typed array each element is checked in list order as a single reference would be. An element repeating an earlier one is not checked twice.

### On replace

A replace checks a reference again only when its outcome could have changed:

| Declaration | Checked again on a replace when |
|---|---|
| `identity`, `token`, `contract` | the value changed |
| `contract` with an `owner` requirement, on a type whose documents can be transferred or traded | every replace |
| `permanentDocument`, `moderatedDocument` | the value changed, or a referring property a `where` entry reads changed (an equality's referring side, a condition's bound) |
| any document reference with a `where` entry whose value is `"$ownerId"`, a `notEqual` or `contains` bounded by `"$ownerId"`, or an ordering bounded by `$updatedAt`, `$transferredAt` or one of their heights | every replace |
| a document reference a `propertyConstraints` rule reads through ([`referenced`](property-constraints.md#values-of-a-referenced-document)) | every replace: every replace judges every rule, and the rule reads the document as it is at the replace |
| `permanentDocument` with `findBy`, `inList` included | also when a property `findBy` reads changed |
| `deletableDocument`, by id or with `findBy` | every replace |
| a document reference with a `findBy` function | never: the whole declaration, its `where` included, is judged on the create alone (see [Commit and reveal](refers-to-lookup.md#commit-and-reveal)); a type whose documents are never replaced has no replace at all |
| `identityPublicKey` with `keyIdProperty` | the identity or the key id changed |
| key id with `identityProperty: "$ownerId"` | every replace |
| key id with `identityProperty: "$creatorId"` | the key id changed |
| key id with an identity property path | the key id or that property changed |
| `anyOf` or `allOf` | one of its operands would be checked again; the whole expression is then checked |

A value changed when the replace set it differently, added it or removed it. Changes are tracked per top-level property, so a change anywhere in an object checks again every reference inside that object. When a typed array changed, only the elements the stored list did not hold are checked, unless a rule above that applies to every element does (a referring property a `where` entry reads changed, a `where` entry checked on every replace, an `owner` requirement on such a type, or a `deletableDocument` target); then every element is checked. A reference checked again is checked whole: its existence and every `where` entry, against the document as it is at the replace. The rules for `ownerRefersTo` and `creatorRefersTo` are in [Writer and Creator References](owner-refers-to.md).

The "every replace" rows exist because something the reference depends on can change without a write to the referring document: the writer after a transfer or purchase, the time of the write, the target's existence for a deletable one, or, for a reference a rule reads through, the target's own values (a `permanentDocument` target may be replaced since). Checking such a reference again costs the read its first check made, billed the same way: by id, or through the `findBy` index. A moderated target that a moderator removed is not one of them: the reference resolves to its removal record, as described [above](#moderateddocument).

### Transfers, purchases, deletes and restores

- A transfer or a purchase checks no reference. A reference governs writing, not holding: a new owner meets the writer gates (a `where` entry valued `"$ownerId"`, `identityProperty: "$ownerId"`, an `owner` requirement) on their first replace. Nor does it judge a rule reading a referenced document; registration keeps such a rule, on a type whose documents can be transferred or traded, from reading what a transfer, a purchase or a price update changes.
- Deleting a referring document checks nothing. Deleting a referenced document does not look for documents referring to it: a `permanentDocument` target cannot be deleted at all, a `moderatedDocument` target is removed by a moderator on the record the reference then resolves to, and a `deletableDocument` reference meets its missing target on the referring document's next replace. A `deletableDocument` target can still refuse its owner's delete while documents refer to it: a [`deleteConstraints`](deletion.md#deleteconstraints) rule counting them by the target's own id, `{ "equal": [{ "countOf": ["vote", { "pollId": "$id" }] }, 0] }`, needs a countable index on the referring property, and holds the target in place until the last of them is gone.
- A document a moderator removed and later restores comes back as it was, without its references being checked again (see [Restoring Documents](../data-model/contract-moderation.md#restoring-documents)), and without the rules reading a referenced document judged. A moderator's change of the fields a type keeps for its moderators judges none of them either, and a rule reading a referenced document may not read such a field.

## The reference budget

Every reference is a billed read when a document is written, so a document type may carry at most **256** references per document. Registration counts:

- one for each property declaring `refersTo`, whether an identifier or a key id;
- `maxItems` for each typed array whose elements declare it;
- one for the type's `ownerRefersTo` or `creatorRefersTo`;
- each of these multiplied by the number of leaves when the declaration is an expression.

The `reply` above counts 6. A typed array of `maxItems` 15 whose elements declare an `anyOf` of two targets counts 30. A type over the budget is refused at registration with `InvalidContractStructure` (10231).

## Rules at registration

A contract's declarations are checked when it is registered, and again for the whole contract on every update. A declaration that is malformed is refused by the meta-schema (`JsonSchemaError`, 10101) or by the parser (`InvalidContractStructure`, 10231). A declaration that is well formed but cannot hold is refused with the reference errors below: those are judged against the contract itself for its own document types, and against the stored contract for another contract's.

- `refersTo` sits on an identifier property or on the `items` of a typed array of identifiers. On the array itself it is refused: the declaration belongs on its `items`. The one exception is the key id form of `identityPublicKey`, on an integer property with exactly `"minimum": 0` and `"maximum": 4294967295`.
- A declaration holds `type` and the keys its target takes, or a single `anyOf` or `allOf`.
- A referenced `documentType` must exist (`ReferencedDocumentTypeNotFoundError`, 40121). The three document references are disjoint, each type admitting exactly one: its documents must never disappear for `permanentDocument` (`ReferencedDocumentTypeDeletableError`, 40122), disappear only through a moderator's recorded removal for `moderatedDocument` (`ReferencedDocumentTypeNotModeratedError`, 40143), and be able to disappear otherwise for `deletableDocument` (`ReferencedDocumentTypeNotDeletableError`, 40131, for a type whose documents never disappear; `ReferencedDocumentTypeModeratedError`, 40144, for one whose documents only moderators remove on the record). The last is refused at registration only: a `deletableDocument` reference promises less than such a type keeps, and a contract registered on a network before `moderatedDocument` existed may hold one, which stays writable. A `permanentDocument` with `inList` whose list is in the declaring contract is the exception: the parser checks its type and reports a deletable one as `InvalidContractStructure` (10231).
- A reference that finds its document by the document's id (`permanentDocument`, `moderatedDocument` or `deletableDocument` without `findBy`, or with `inList`, whose list's document is read by its id) may not name an [`indexOnly`](index-only.md) type (`ReferencedDocumentTypeIndexOnlyError`, 40146): its documents exist only as index entries and can not be fetched by `$id`, so no write could check the reference. This holds for a type of the declaring contract and of another one alike. A `findBy` into such a type is refused by its own rules instead, since the type has no unique index.
- Every `where` entry must be one that can hold (40126), every key reference must fit the document type (40125), every `boundTo` must name a type a key can be bound to (10231), and every [`findBy`](refers-to-lookup.md#rules-at-registration) and [list](refers-to-list-element.md#rules-at-registration) must resolve. A rule reading a document of another contract's type through a reference must read what that type gives (40148).
- An `immutable` property may not hold a `deletableDocument` reference that a replace could not remove: one inside an object, a typed array of them, or any `deletableDocument` found by `findBy`, except one whose key a `findBy` function computes, which is checked on the create alone. A single `deletableDocument` reference by id that is itself an immutable top-level property is allowed, but only listed without a condition: a replace the condition left free could set it to another document once it was cleared. A `contract` reference with an `owner` requirement may not sit under an immutable property of a type whose documents can be transferred or traded. All refused with `InvalidContractStructure` (10231); see [Mutability](mutability.md).
- The type stays within the [reference budget](#the-reference-budget).
- The earlier spellings `lookup`, `propertyAgreement` and `type: "listElement"`, replaced by `findBy`, `where` and `inList` before protocol version 14 shipped, are refused: by the meta-schema on registration and update (10101), and by the parser on every parse (10231), with a message naming what replaced each.
- On an update, every existing declaration must be unchanged (10246). A document type the update adds may declare any reference.

## Errors

| Error | Code | When |
|---|---|---|
| `ReferencedEntityNotFoundError` | 40120 | Write: the identity, contract, token or document does not exist, `findBy` finds no document, or a value is not in its list. |
| `ReferencedDocumentTypeNotFoundError` | 40121 | Registration: `documentType` does not exist, or `contractId` names no contract. |
| `ReferencedDocumentTypeDeletableError` | 40122 | Registration: a `permanentDocument` reference, with `inList` or without, names a type whose documents can disappear. |
| `ReferencedIdentityKeyNotFoundError` | 40123 | Write: the identity has no key with that id, or the identity does not exist. |
| `ReferencedIdentityKeyDisabledError` | 40124 | Write: the key is disabled. |
| `ReferencedKeyIdPropertyInvalidError` | 40125 | Registration: `keyIdProperty` or `identityProperty` names a property that does not fit, `$creatorId` on a type that records no creator ids, or a stored key id paired with a transient identity. Write: a key id without its identity, or an identity without its key id. |
| `ReferencedDocumentPropertyAgreementInvalidError` | 40126 | Registration: a `where` entry names a missing, transient or object property, pairs a typed array with something other than a typed array of the same element kind, compares typed arrays of floats, or two properties of different types, or `$creatorId` on a type that does not record it, or keys a preallocated index by a property, or a typed array with an element, that can hold more than 255 bytes; or a [condition](#conditions) holds a literal of another kind than its property, compares two values of different kinds, orders something that is no integer, or is bounded by a time or height the declaring type does not record. |
| `ReferencedDocumentPropertyMismatchError` | 40127 | Write: the document found does not meet a `where` entry; for a condition, the referring property is the condition as written. |
| `ReferencedDocumentTypeNotDeletableError` | 40131 | Registration: a `deletableDocument` reference names a type whose documents can never disappear. |
| `ReferencedContractRequirementNotMetError` | 40135 | Write: the referenced contract exists but does not meet a `contractRequirements` entry. |
| `ReferencedIdentityKeyRequirementNotMetError` | 40136 | Write: the key exists and is enabled but does not meet a `keyRequirements` entry. |
| `ReferencedDocumentLookupInvalidError` | 40137 | Registration: a `findBy` into another contract's document type cannot resolve, for example because no unique index of it is over exactly the properties `findBy` names. |
| `ReferencedDocumentListInvalidError` | 40138 | Registration: an `inList` list on another contract's document type does not qualify. |
| `ReferencedDocumentTypeNotModeratedError` | 40143 | Registration: a `moderatedDocument` reference names a type whose documents can disappear otherwise than through a moderator's recorded removal, or never disappear. |
| `ReferencedDocumentTypeModeratedError` | 40144 | Registration: a `deletableDocument` reference names a type whose documents disappear only through a moderator's recorded removal. |
| `ReferencedDocumentRemovedError` | 40145 | Write: a replace kept a `moderatedDocument` reference whose document a moderator removed, and a `where` entry checked again compares a property the removal record does not keep. |
| `ReferencedDocumentTypeIndexOnlyError` | 40146 | Registration: a reference that finds its document by id (no `findBy`, or with `inList`) names an `indexOnly` type, whose documents can not be fetched by id. |
| `ReferencedDocumentRuleReadInvalidError` | 40148 | Registration: a [`propertyConstraints` rule](property-constraints.md#values-of-a-referenced-document) reads, through a reference into another contract's document type, a field that type can not give it. A rule reading a type of the declaring contract is refused by the contract parse instead (10231). |
| `ReferencedDocumentPrecededError` | 40149 | Write: the commitment a [`chooseFirst`](refers-to-lookup.md#choosing-the-first-commitment-choosefirst) reference found is preceded, in its `by` order, by a valid, unexpired commitment of another identity holding the same value at `among`. |

The registration errors name the declaration as `<documentType>.<property>`, `<documentType>.<property>[]` for typed array elements, `<documentType>.$ownerId` or `<documentType>.$creatorId` for the writer and creator references, and add the operand for a leaf of an expression (`resignation.memberId.anyOf[1]`). When a document is written, the referenced document type is looked up and its kind checked again as a safeguard (40121, 40122, 40131, 40143), but a registered contract cannot fail those checks later: contracts and document types are never removed, and neither the deletion flags nor `ttl` nor `moderatorAbilities` can change. Nor can `indexOnly`, so the indexOnly rule (40146) is checked at registration only. The other codes in the range (40128 to 40130, 40132 to 40134, 40139 to 40142, 40147) belong to other keywords. See [Error Codes](../error-handling/error-codes.md).

## More forms of reference

- [findBy](refers-to-lookup.md): a `permanentDocument` or `deletableDocument` found through the unique index of its type over exactly the properties `findBy` names, with the value as one part of the key. The value can be an identity (a member, an author) instead of a document id.
- [Expressions](refers-to-expressions.md): `anyOf` and `allOf`, several targets combined in one declaration.
- [List Elements](refers-to-list-element.md): `inList` on a `permanentDocument`, a value that must be one of the identifiers a list on another document holds.
- [Writer and Creator References](owner-refers-to.md): `ownerRefersTo` and `creatorRefersTo`, the same declaration applied to the document's writer or creator instead of a property.
- [References on the elements](../data-model/documents.md#references-on-the-elements) of a typed array: a declaration on `items`, checked for every element.

## See also

- [Document References](../data-model/documents.md#document-references-refersto) in the Documents chapter, with the internals.
- [Elected Moderation](../data-model/contract-moderation.md#elected-moderation) for the moderation charters contract, the first user of most reference forms.
- [Typed Arrays](typed-arrays.md), [System Properties](system-properties.md), [Mutability](mutability.md), [Deletion](deletion.md), [Time To Live (ttl)](ttl.md) and [Creation, Transfers and Trading](ownership-and-trading.md) for the keywords references read.
- [distinctFrom](distinct-from.md), which requires an identifier to differ from another, and [encryptedFor](encrypted-for.md), which names the key references an encrypted property was made with.
- [Values of Referenced Documents](derived-index-properties.md), where an index holds a value of the document a `permanentDocument` or `moderatedDocument` reference points at.
- [propertyConstraints](property-constraints.md#values-of-a-referenced-document), whose rules read the document a reference points at through `{ "referenced": [reference, field] }`.
- [Contract Keywords](../contract-keywords.md) for the conventions these pages use.
