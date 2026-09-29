# Mutability

These three keywords decide what a replace may change once a document exists. A replace is the transition an owner sends to overwrite a document with a new version of it. `documentsMutable` turns replaces on or off for the whole document type. `immutable` freezes chosen properties while the rest of the document stays editable, and `immutableAllowSetting` lets some of those frozen properties be filled in once, later, when they were left empty at creation.

None of the three governs deletion, transfers or trading: see [Deletion](deletion.md) and [Creation, Transfers and Trading](ownership-and-trading.md).

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
- `immutable` and `immutableAllowSetting` are only accepted when the type's documents are mutable (`InvalidContractStructure`, 10231).
- `moderatorAbilities.deleteWithin` on a mutable type needs `$updatedAt` in `required`. See [Deletion](deletion.md).

## `immutable`

The top-level properties that are frozen when a document is created, on a type whose documents can otherwise be replaced. Reach for it when most of a document is editable but some of it is a commitment: the shop an order was placed with, the author of a post, the item that was ordered.

| | |
|---|---|
| **Where** | document type, on a type with `documentsMutable: true` |
| **Value** | array of top-level property names, no repeats |
| **Default** | empty: every property may change |
| **Since** | protocol version 14 |
| **On update** | May gain entries, never lose one (`DocumentTypeUpdateError`, 40212) |
| **Errors** | `DocumentImmutablePropertyChangedError` (40128) for a replace that changes, adds or removes a listed property |

### Example

```json
"order": {
  "type": "object",
  "documentsMutable": true,
  "properties": {
    "shop": {
      "type": "array",
      "byteArray": true,
      "minItems": 32,
      "maxItems": 32,
      "contentMediaType": "application/x.dash.dpp.identifier",
      "position": 0
    },
    "item": { "type": "string", "maxLength": 100, "position": 1 },
    "status": { "type": "string", "enum": ["open", "paid", "shipped"], "position": 2 },
    "trackingCode": { "type": "string", "maxLength": 40, "position": 3 }
  },
  "required": ["shop", "item", "status"],
  "immutable": ["shop", "item", "trackingCode"],
  "immutableAllowSetting": ["trackingCode"],
  "additionalProperties": false
}
```

The buyer can move `status` along as often as needed, but never change which shop or which item the order is for. `trackingCode` is left out when the order is placed, may be filled in by one later replace, and is frozen from then on.

### How it works

- On every replace, each listed property of the new document is compared with the stored one. A property that differs is refused with `DocumentImmutablePropertyChangedError` (40128). "Differs" covers a changed value, a value the stored document did not have (unless `immutableAllowSetting` allows it, below), and a value the replace leaves out.
- Values are compared by their data, not their bytes: the order of an object's members and the width an integer is stored in do not count as changes.
- Freezing an object freezes everything inside it.
- One change is always allowed: a replace may clear a listed `deletableDocument` reference by id once the document it points to has been deleted. Every replace checks such a reference again, so without this the document could never be replaced again. See [References](refers-to.md).
- Transfers, price updates and purchases carry no property values, so the list does not affect them.

### Rules at registration

All refusals below are `InvalidContractStructure` (10231).

- Only on a type whose documents are mutable. On a type with `documentsMutable: false` every property is already frozen.
- Every entry names a declared top-level property. System properties (`$ownerId`, `$createdAt` and the rest) are refused, since the platform manages them. Nested paths such as `meta.author` are refused: list the object that contains them.
- No entry may be a `transient` property: it is never stored, so every replace that supplies it would count as a change.
- An immutable property may not hold a `deletableDocument` reference that a replace could not clear once its target is gone: a typed array of them, one inside an object, or one found through a `lookup`. A single reference by id held directly by the property is allowed.
- On a type whose documents can be transferred or traded, an immutable property may not hold a `contract` reference whose `contractRequirements` has an `owner` requirement: after a change of owner the new owner could neither meet it nor repoint it.

### On update

The list may grow: an update may freeze a property that was editable, and documents already stored keep the values they have. It may never shrink, since documents were written on the promise that those properties would not change. The comparison is of the parsed lists, so reordering them is no change.

## `immutableAllowSetting`

The `immutable` properties that a replace may still set while the stored document has no value for them. It is for optional values that are not known when the document is created, like a tracking code or a closing date, and must not change once they are known.

| | |
|---|---|
| **Where** | document type, next to `immutable` |
| **Value** | array of property names, each also listed in `immutable`, no repeats |
| **Default** | empty |
| **Since** | protocol version 14 |
| **On update** | May lose entries at any time. May gain an entry only for a property that becomes immutable in the same update (`DocumentTypeUpdateError`, 40212). |
| **Errors** | `DocumentImmutablePropertyChangedError` (40128) for a replace that changes or removes the property once it holds a value |

### How it works

- While the stored document has no value for the property, a replace may set it. That first value is then frozen like the rest of the `immutable` list: it can neither change nor be removed.
- It only means something for an optional property. A required one always has a value from creation.

### Rules at registration

- Every entry must also be in `immutable` (`InvalidContractStructure`, 10231).
- An entry may not be a `deletableDocument` reference by id (`InvalidContractStructure`, 10231). Such a reference may be cleared once its target is deleted, and the next replace could then set it again to a different document.

### On update

Dropping an entry tightens the rule and is always allowed. Adding one to a property that was already immutable would let documents change what they were promised to keep, so it is only allowed together with making the property immutable in the same update.

## See also

- [Immutable Properties on Mutable Document Types](../data-model/documents.md#immutable-properties-on-mutable-document-types), for how the replace compares values
- [Deletion](deletion.md), [Creation, Transfers and Trading](ownership-and-trading.md) and [History](history.md), the other keywords on what may happen to a document
- [System Properties](system-properties.md), for `$revision` and `$updatedAt`
- [transient](transient.md) and [References](refers-to.md), for the properties `immutable` refuses
- [Contract Keywords](../contract-keywords.md#reading-the-chapters), for how the summary tables read
