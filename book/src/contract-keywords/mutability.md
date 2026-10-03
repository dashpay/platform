# Mutability

These two keywords decide what a replace may change once a document exists. A replace is the transition an owner sends to overwrite a document with a new version of it. `documentsMutable` turns replaces on or off for the whole document type. `immutable` freezes chosen properties while the rest of the document stays editable: from the moment the document is created, or only while a condition holds, such as five minutes after creation, once the document is published, or once a value has been filled in.

Neither governs deletion, transfers or trading: see [Deletion](deletion.md) and [Creation, Transfers and Trading](ownership-and-trading.md).

## `documentsMutable`

Whether the owner of a document may replace it. Set it to `false` for records that must never change after they are written: votes, receipts, name registrations.

| | |
|---|---|
| **Where** | document type |
| **Value** | boolean |
| **Default** | the contract config's `documentsMutableContractDefault`, which is `true` unless the contract says otherwise |
| **Since** | protocol version 1 |
| **On update** | Fixed (`DocumentTypeUpdateError`, 40212). Adding or removing the key without changing its value is refused too, as a schema change (`IncompatibleDocumentTypeSchemaError`, 10246). |
| **Errors** | `InvalidDocumentTransitionActionError` (10404) for a replace of a type set to `false` |

### Example

```json
"vote": {
  "type": "object",
  "documentsMutable": false,
  "canBeDeleted": false,
  "properties": {
    "proposalId": {
      "type": "array",
      "byteArray": true,
      "minItems": 32,
      "maxItems": 32,
      "contentMediaType": "application/x.dash.dpp.identifier",
      "position": 0
    },
    "choice": { "type": "string", "enum": ["yes", "no", "abstain"], "position": 1 }
  },
  "required": ["proposalId", "choice", "$createdAt"],
  "additionalProperties": false
}
```

A vote is cast once and stays as cast: no replace is accepted, and with `canBeDeleted: false` its owner cannot take it back either.

### How it works

- With `true`, the document's owner may replace it. The replace carries the whole new document, which is validated against the schema as a create is, and a `$revision` one higher than the stored one (`InvalidDocumentRevisionError`, 40106). Anyone other than the owner is refused (`DocumentOwnerIdMismatchError`, 40102). When the type lists `$updatedAt` in `required`, the replace sets it to the block's time, and likewise `$updatedAtBlockHeight` and `$updatedAtCoreBlockHeight` to the block heights.
- With `false`, every replace is refused (`InvalidDocumentTransitionActionError`, 10404).
- A type whose documents cannot be replaced may still let them be transferred or sold (`transferable`, `tradeMode`). Such a document changes owner and price, but its owner can never edit its properties. The DPNS `domain` type works this way.
- A document stores a `$revision` when its type allows a replace, a transfer or trading, or keeps fields its moderators write ([`moderatorAbilities.changeFields`](moderator-abilities.md#changefields)), whose changes raise it even on a type whose documents cannot be replaced. A type that allows none of them stores none. See [System Properties](system-properties.md).

### Rules at registration

- A contested index needs a type whose documents cannot be replaced (`ContestedUniqueIndexOnMutableDocumentTypeError`, 10248). See [Contested Indexes](contested.md).
- An `indexOnly` type must set `documentsMutable: false`. See [Index-Only Types](index-only.md).
- `immutable` is only accepted when the type's documents are mutable (`InvalidContractStructure`, 10231).
- `moderatorAbilities.deleteWithin` on a mutable type needs `$updatedAt` in `required`. See [Deletion](deletion.md).

## `immutable`

The top-level properties a replace may not change, on a type whose documents can otherwise be replaced. Each entry is either a property name, frozen when the document is created, or a property with a condition, frozen for any replace the condition holds for. Reach for the first when part of a document is a commitment: the shop an order was placed with, the author of a post. Reach for the second when a value may change for a while and then must stand: a post's text for five minutes after it is published, an article's body once it is out of draft, a tracking code once it is filled in.

| | |
|---|---|
| **Where** | document type, on a type with `documentsMutable: true` |
| **Value** | array whose entries are a top-level property name, or `{ "property": <name>, "when": <condition> }`; each property listed once |
| **Default** | empty: every property may change |
| **Since** | protocol version 14 |
| **On update** | May gain entries, with a condition or without. A property listed with a condition keeps it, or loses it to be listed without one. A property listed without a condition stays, and no property is dropped (`DocumentTypeUpdateError`, 40212) |
| **Errors** | `DocumentImmutablePropertyChangedError` (40128) for a replace that changes, adds or removes a frozen property |

### Example

```json
"post": {
  "type": "object",
  "documentsMutable": true,
  "properties": {
    "author": { "type": "string", "maxLength": 63, "position": 0 },
    "text": { "type": "string", "maxLength": 500, "position": 1 },
    "status": { "type": "string", "enum": ["draft", "published"], "position": 2 },
    "body": { "type": "string", "maxLength": 5000, "position": 3 },
    "trackingCode": { "type": "string", "maxLength": 40, "position": 4 },
    "pinned": { "type": "boolean", "position": 5 }
  },
  "required": ["author", "text", "status", "$createdAt", "$updatedAt"],
  "immutable": [
    "author",
    {
      "property": "text",
      "when": { "greaterThan": [{ "subtract": ["$updatedAt", "$createdAt"] }, 300000] }
    },
    {
      "property": "body",
      "when": { "equal": ["$old.status", { "const": "published" }] }
    },
    { "property": "trackingCode", "when": { "present": "$old.trackingCode" } }
  ],
  "additionalProperties": false
}
```

`author` never changes. `text` can be corrected for five minutes after the post is created: during a replace `$updatedAt` is the replace's block time, so the difference is the post's age in milliseconds. `body` stays editable while the stored post is a draft, including in the replace that publishes it, and is frozen after that. `trackingCode` can be filled in by one replace while the stored post has none, and is frozen once it holds one. `pinned` is not listed, so it can change at any time.

### How it works

- On every replace, each property that differs from the stored document is checked against the list. A property listed by name is refused with `DocumentImmutablePropertyChangedError` (40128). A property listed with a condition is refused with the same error when its condition holds for this replace. "Differs" covers a changed value, a value the stored document did not have, and a value the replace leaves out.
- A condition takes the grammar of a [`propertyConstraints`](property-constraints.md) rule: comparisons, arithmetic, `in`, `present`, `absent`, `anyOf`, `allOf`, `not`, `ifThen`, `ifThenElse`, the system times and heights the type records, and `$ownerId`. It is judged, as a rule judges a replace, on the document the replace writes: its properties as the replace sets them, the stored `$createdAt` and `$transferredAt`, and the replace's block as `$updatedAt`.
- A path starting with `$old.` reads the stored document instead: `$old.status` is the status before the replace. Only a condition of `immutable`, or a type's [`retractedWhen`](deletion.md#retractedwhen), may read it, and only a schema property through it.
- A property is frozen while its condition holds. If what the condition reads can change back, the property can become editable again: with `body` frozen by `$old.status`, a replace setting `status` back to `draft` still reads the stored `published`, so `body` stays frozen in that replace, and the next replace, reading the stored `draft`, may change it. For a freeze that lasts, list what the condition reads as well, or use a condition that cannot turn back, as a document's age or a filled-in value cannot.
- A condition is evaluated only when its property changes. A condition that faults, dividing by zero or overflowing, counts as holding: a fault never frees a property.
- Values are compared by their data, not their bytes: the order of an object's members and the width an integer is stored in do not count as changes. Freezing an object freezes everything inside it.
- One change is always allowed: a replace may clear a `deletableDocument` reference by id, listed without a condition, once the document it points to has been deleted. Every replace checks such a reference again, so without this the document could never be replaced again. See [References](refers-to.md).
- A replace judged by `checkTx` shortly before a time condition starts holding may still land in a block after that, and is then refused there and pays its fee, like any other state check.
- Transfers, price updates and purchases carry no property values, so the list does not affect them.

### Rules at registration

All refusals below are `InvalidContractStructure` (10231). The shape of the list, and what each condition reads, are checked on every parse. The other rules are checked when a contract is registered or updated.

- Only on a type whose documents are mutable. On a type with `documentsMutable: false` every property is already frozen.
- Every entry is a string or an object with exactly `property` and `when`, and no property is listed twice.
- Every listed property is a declared top-level property. System properties (`$ownerId`, `$createdAt` and the rest) are refused, since the platform manages them. Nested paths such as `meta.author` are refused: list the object that contains them.
- No listed property may be `transient`: it is never stored, so every replace that supplies it while it is frozen would count as a change.
- A condition reads what a `propertyConstraints` rule may read of the type: declared properties of the right kind, neither transient nor inside a transient object, and system times and heights the type lists in `required`. It stays within the node limit of a rule, and lists no condition twice.
- A condition may not read a `countOf` or `sumOf` total: it reads the document alone, so a replace judges it without reading state.
- `immutableAllowSetting`, which earlier builds used to let a frozen property be filled in once, is refused and names the replacement: `{ "property": "p", "when": { "present": "$old.p" } }`.
- An immutable property may not hold a `deletableDocument` reference that a replace could not clear: a typed array of them, one inside an object, or one found by `findBy` whose key no function computes. A single reference by id held directly by the property is allowed, but only without a condition: once cleared, a replace the condition leaves free could set it to another document.
- On a type whose documents can be transferred or traded, an immutable property may not hold a `contract` reference whose `contractRequirements` has an `owner` requirement: after a change of owner the new owner could neither meet it nor repoint it.
- A listed property, with a condition or without, may not be one of the fields only moderators write ([`moderatorAbilities.changeFields`](moderator-abilities.md#changefields)).
- A property listed with a condition is not fixed once written, so a `findBy` key, an `inList` list, a value read beside a `findBy` function, or a value another type indexes through a reference may not rely on it. See [References](refers-to.md).

### On update

What `immutable` freezes may only tighten. A property may be added, with a condition or without, and documents already stored are held to it from then on. A property listed with a condition may lose it, which freezes it whatever the condition said. A condition cannot change: whether one condition holds wherever another does cannot be told in general. A property listed without a condition stays, and no property is dropped, since documents were written on the promise that those properties would not change. The comparison is of the parsed entries, so reordering them is no change.

## See also

- [Immutable Properties on Mutable Document Types](../data-model/documents.md#immutable-properties-on-mutable-document-types), for how the replace compares values
- [Deletion](deletion.md), [Creation, Transfers and Trading](ownership-and-trading.md) and [History](history.md), the other keywords on what may happen to a document
- [System Properties](system-properties.md), for `$revision` and `$updatedAt`
- [transient](transient.md) and [References](refers-to.md), for the properties `immutable` refuses
- [propertyConstraints](property-constraints.md), for the grammar of a condition
- [Contract Keywords](../contract-keywords.md#reading-the-chapters), for how the summary tables read
