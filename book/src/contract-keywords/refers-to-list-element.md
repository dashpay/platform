# List Elements

A `listElement` reference says the value must be one of the identifiers a list on another document holds. `inList` names the list, and the `$id` pair of `propertyAgreement` names the document holding it. Reach for it when membership is written down once as a list on a document that never changes, such as the members elected with a charter, and other documents must name one of them.

| | |
|---|---|
| **Where** | `refersTo` on an identifier property, or on the `items` of a typed array of identifiers, where every element must be listed; an operand of an [expression](refers-to-expressions.md); or [`ownerRefersTo` and `creatorRefersTo`](owner-refers-to.md), where the writer or the creator must be listed. |
| **Value** | `"type": "listElement"` with `documentType` (the type holding the list), `propertyAgreement` (exactly one pair with `$id` on its referenced side, and up to nine other pairs), `inList` (the path of a typed array of identifiers on that type), and optionally `contractId`. |
| **Since** | protocol version 14 |
| **On update** | Fixed, like the rest of `refersTo` (`IncompatibleDocumentTypeSchemaError`, 10246). |
| **Errors** | `ReferencedEntityNotFoundError` (40120) when the value is not in the list or the list's document is not found; `ReferencedDocumentPropertyMismatchError` (40127) for another pair; at registration `ReferencedDocumentListInvalidError` (40138) for a list of another contract. |

## Example

The moderation charters contract lets the leader of a seated team take an elected member off it with a `removedModerator` document:

```json
"removedModerator": {
  "type": "object",
  "documentsMutable": false,
  "canBeDeleted": true,
  "properties": {
    "electedCharterId": {
      "type": "array", "byteArray": true, "minItems": 32, "maxItems": 32,
      "contentMediaType": "application/x.dash.dpp.identifier",
      "refersTo": {
        "type": "permanentDocument",
        "documentType": "electedCharter",
        "propertyAgreement": { "$ownerId": "$ownerId" }
      },
      "position": 0
    },
    "memberId": {
      "type": "array", "byteArray": true, "minItems": 32, "maxItems": 32,
      "contentMediaType": "application/x.dash.dpp.identifier",
      "distinctFrom": "$ownerId",
      "refersTo": {
        "type": "listElement",
        "documentType": "electedCharter",
        "propertyAgreement": { "electedCharterId": "$id" },
        "inList": "members"
      },
      "position": 1
    }
  },
  "required": ["$createdAt", "electedCharterId", "memberId"],
  "additionalProperties": false
}
```

`electedCharterId` must name an elected charter the writer owns, so only the team's leader can write one. `memberId` must be one of the `members` of the `electedCharter` document whose `$id` this document's `electedCharterId` holds, so only an elected member can be removed. `electedCharter` is immutable and can never be deleted, so its `members` never change.

## How it works

When the referring document is created or replaced:

1. The document holding the list is the one whose `$id` equals the value of the `$id` pair's referring property. It is fetched by id, once per write: in the example the `electedCharterId` reference and the list reference share one fetch. The list is collected once, so checking many values against it costs one read.
2. The other `propertyAgreement` pairs are checked against that document, as for any document reference (`ReferencedDocumentPropertyMismatchError`, 40127).
3. The value must be in the list. On a typed array every element must be; on the writer's or creator's reference, the writer or creator must be.

A value not in the list, a list document that does not exist, or a value set while the `$id` property is not, refuses the write with `ReferencedEntityNotFoundError` (40120), naming the property, or an element by its list path (`memberIds[1]` for the second):

```text
electedCharter 7kX...: members [Alice, Bob]
removedModerator { electedCharterId: 7kX..., memberId: Alice }  -> accepted
removedModerator { electedCharterId: 7kX..., memberId: Carol }  -> refused, 40120:
  referenced list element (members of the electedCharter document electedCharterId names)
  <Carol> not found for path memberId
```

A replace checks the reference again when its value changed, or when the referring side of any of its pairs changed: the `$id` property among them, since it may now name another document, whose list is then checked against every value. A pair keyed by `$ownerId` is checked on every replace. When only a typed array of values changed, the elements the stored list already held are not checked again. Nothing else can make a validated value unlisted: the list's document is never deleted and its list never changes.

## Rules at registration

- `propertyAgreement` holds exactly one pair with `$id` on the referenced side. Its referring side is an identifier property of the referring document type: not `$ownerId` (no document has the writer's id) and not a typed array (one document holds the list). It must be stored, so it and every object around it are not `transient`, and a reader can tell from the stored document which list the value was checked against. It may be optional. It needs no `refersTo` of its own; if it has one, that must be a reference by id to `documentType` in the list's contract, or the pair could never hold. Refused with `InvalidContractStructure` (10231) otherwise.
- The other pairs follow the rules of every [`propertyAgreement`](refers-to.md#propertyagreement) (`ReferencedDocumentPropertyAgreementInvalidError`, 40126).
- `documentType` must exist (`ReferencedDocumentTypeNotFoundError`, 40121), and its documents must never disappear: `canBeDeleted: false`, no `moderatorAbilities.delete` and no `ttl`.
- `inList` is a property path (a dotted one for a nested list, never a `$` name) of a stored typed array of identifiers on `documentType`, and the list must be fixed once a document is written: the type is immutable (`documentsMutable: false`), or the list's top-level property is listed under `immutable`. A list that is also under `immutableAllowSetting` qualifies, since it can only be set once on a document that had none, and an absent list accepted no value.
- The checks on `documentType` and `inList` are refused with `InvalidContractStructure` (10231) for a document type of the declaring contract. For one of another contract, a deletable type is refused with `ReferencedDocumentTypeDeletableError` (40122) and a list that does not qualify with `ReferencedDocumentListInvalidError` (40138).
- Each value counts one against the [reference budget](refers-to.md#the-reference-budget), `maxItems` for a typed array.

## See also

- [References (refersTo)](refers-to.md) for `documentType`, `contractId`, `propertyAgreement` and the replace rules.
- [Expressions](refers-to-expressions.md) and [Writer and Creator References](owner-refers-to.md), where a list element is a common operand or target.
- [An element of a list](../data-model/documents.md#an-element-of-a-list-listelement) in the Documents chapter, with the internals.
- [Typed Arrays](typed-arrays.md) and [Mutability](mutability.md) for the list and the rules that keep it fixed.
