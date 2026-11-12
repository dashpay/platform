# List Elements (inList)

A `permanentDocument` reference with `inList` says the value must be one of the identifiers a list on another document holds. `inList` names the list, and `findBy: { "$id": <property> }` names the document holding it, by the id another property of the referring document holds. Reach for it when membership is written down once as a list on a document that never changes, such as the members elected with a charter, and other documents must name one of them.

| | |
|---|---|
| **Where** | `refersTo` on an identifier property, or on the `items` of a typed array of identifiers, where every element must be listed; an operand of an [expression](refers-to-expressions.md); or [`ownerRefersTo` and `creatorRefersTo`](owner-refers-to.md), where the writer or the creator must be listed. |
| **Value** | `inList`, the path of a typed array of identifiers on `documentType`, in a `"type": "permanentDocument"` declaration with `documentType` (the type holding the list), `findBy` (exactly `{ "$id": "<the identifier property holding the document's id>" }`), optionally `where` (up to 10 entries) and `contractId`. |
| **Since** | protocol version 14 |
| **On update** | Fixed, like the rest of `refersTo` (`IncompatibleDocumentTypeSchemaError`, 10246). |
| **Errors** | `ReferencedEntityNotFoundError` (40120) when the value is not in the list or the list's document is not found; `ReferencedDocumentPropertyMismatchError` (40127) for a `where` entry; at registration `ReferencedDocumentListInvalidError` (40138) for a list of another contract. |

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
        "where": { "$ownerId": "$ownerId" }
      },
      "position": 0
    },
    "memberId": {
      "type": "array", "byteArray": true, "minItems": 32, "maxItems": 32,
      "contentMediaType": "application/x.dash.dpp.identifier",
      "distinctFrom": "$ownerId",
      "refersTo": {
        "type": "permanentDocument",
        "documentType": "electedCharter",
        "findBy": { "$id": "electedCharterId" },
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

Read as SQL, with the list as a table of its elements:

```sql
SELECT * FROM electedCharter
WHERE $id = :electedCharterId      -- findBy
  AND :memberId IN (members)       -- inList
```

## How it works

When the referring document is created or replaced:

1. The document holding the list is the one whose `$id` equals the value of the property `findBy` reads. It is fetched by id, once per write: in the example the `electedCharterId` reference and the list reference share one fetch. The list is collected once, so checking many values against it costs one read.
2. The `where` entries, if any, are checked against that document, as for any document reference (`ReferencedDocumentPropertyMismatchError`, 40127).
3. The value must be in the list. On a typed array every element must be; on the writer's or creator's reference, the writer or creator must be.

A value not in the list, a list document that does not exist, or a value set while the `$id` property is not, refuses the write with `ReferencedEntityNotFoundError` (40120), naming the property, or an element by its list path (`memberIds[1]` for the second):

```text
electedCharter 7kX...: members [Alice, Bob]
removedModerator { electedCharterId: 7kX..., memberId: Alice }  -> accepted
removedModerator { electedCharterId: 7kX..., memberId: Carol }  -> refused, 40120:
  referenced list element (members of the electedCharter document electedCharterId names)
  <Carol> not found for path memberId
```

A replace checks the reference again when its value changed, or when a property `findBy` or `where` reads changed: the `$id` property among them, since it may now name another document, whose list is then checked against every value. A `where` entry whose value is `"$ownerId"` is checked on every replace. When only a typed array of values changed, the elements the stored list already held are not checked again. Nothing else can make a validated value unlisted: the list's document is never deleted and its list never changes.

## Rules at registration

- `inList` is only allowed on a `permanentDocument` reference, and needs `findBy`.
- `findBy` is exactly `{ "$id": "<property>" }`, and `{ "$id": ... }` is allowed only with `inList`. The property is an identifier property of the referring document type: not `$ownerId` (no document has the writer's id), not `"."` (the value is the list element) and not a typed array (one document holds the list). It must be stored, so it and every object around it are not `transient`, and a reader can tell from the stored document which list the value was checked against. It may be optional. It needs no `refersTo` of its own; if it has one, that must be a reference by id to `documentType` in the list's contract, or the value could never be in the list. Refused with `InvalidContractStructure` (10231) otherwise.
- `where` may not compare `$id` again, nor the property `findBy` reads. Its entries follow the rules of every [`where`](refers-to.md#where) (`ReferencedDocumentPropertyAgreementInvalidError`, 40126).
- `documentType` must exist (`ReferencedDocumentTypeNotFoundError`, 40121), and its documents must never disappear: `canBeDeleted: false`, no `moderatorAbilities.delete` and no `ttl`.
- `inList` is a property path (a dotted one for a nested list, never a `$` name) of a stored typed array of identifiers on `documentType`, and the list must be fixed once a document is written: the type is immutable (`documentsMutable: false`), or the list's top-level property is listed under `immutable` without a condition. A list frozen only under a condition does not qualify: while the condition does not hold, a replace could change it.
- The checks on `documentType` and `inList` are refused with `InvalidContractStructure` (10231) for a document type of the declaring contract. For one of another contract, a deletable type is refused with `ReferencedDocumentTypeDeletableError` (40122) and a list that does not qualify with `ReferencedDocumentListInvalidError` (40138).
- Each value counts one against the [reference budget](refers-to.md#the-reference-budget), `maxItems` for a typed array.

The type `listElement`, which found the document holding the list through a `propertyAgreement` pair with `$id` on its referenced side, was replaced by `inList` on a `permanentDocument` before protocol version 14 shipped. A declaration still using it is refused on every parse, with a message saying to declare a `permanentDocument` with `findBy: { "$id": ... }` and `inList`.

## See also

- [References (refersTo)](refers-to.md) for `documentType`, `contractId`, [`where`](refers-to.md#where) and the replace rules.
- [findBy](refers-to-lookup.md), which finds a referenced document by the values of a unique index instead of by its id.
- [Expressions](refers-to-expressions.md) and [Writer and Creator References](owner-refers-to.md), where a list reference is a common operand or target.
- [An element of a list](../data-model/documents.md#an-element-of-a-list-inlist) in the Documents chapter, with the internals.
- [Typed Arrays](typed-arrays.md) and [Mutability](mutability.md) for the list and the rules that keep it fixed.
