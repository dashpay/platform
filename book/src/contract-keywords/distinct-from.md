# distinctFrom

`distinctFrom` says that an identifier property must hold a different identity (or document) than another identifier of the same document, or than the document's owner. Reach for it when a document names two parties that must not be the same: a delegate who is not the delegator, a buyer who is not the seller, a team member who is not the team's leader. The check reads only the document being written, so it costs no state reads.

| | |
|---|---|
| **Where** | An identifier property, at the top level or inside an object; or the `items` of a typed array of identifiers, where it binds every element |
| **Value** | `"$ownerId"`, or the dotted path of another identifier property of the same document type (`"meta.reviewerId"` for a nested one), 1 to 256 characters |
| **Default** | Absent: no rule |
| **Since** | protocol version 14 |
| **On update** | Fixed: adding, removing or changing it is refused (`IncompatibleDocumentTypeSchemaError`, 10246) |
| **Errors** | `DocumentPropertyNotDistinctError` (10419) on a document; at registration `JsonSchemaError` (10101) or `InvalidContractStructure` (10231) |

An identifier here is a 32-byte id, declared as a byte array with the identifier `contentMediaType` (see [Property Schemas](property-schemas.md)).

## Example

```json
"delegation": {
  "type": "object",
  "properties": {
    "delegateId": {
      "type": "array", "byteArray": true, "minItems": 32, "maxItems": 32,
      "contentMediaType": "application/x.dash.dpp.identifier",
      "distinctFrom": "$ownerId",
      "position": 0
    },
    "backupId": {
      "type": "array", "byteArray": true, "minItems": 32, "maxItems": 32,
      "contentMediaType": "application/x.dash.dpp.identifier",
      "distinctFrom": "delegateId",
      "position": 1
    }
  },
  "required": ["delegateId"],
  "additionalProperties": false
}
```

An identity cannot delegate to itself: `delegateId` must differ from the document's owner. The optional `backupId` must differ from `delegateId`; a delegation without a backup passes, since there is nothing to compare.

On a typed array the keyword goes on `items`, and every element must differ from the named value. The moderation charters system contract uses this for a team's members, none of whom may be the leader who writes the document:

```json
"members": {
  "type": "array",
  "minItems": 0,
  "maxItems": 15,
  "uniqueItems": true,
  "items": {
    "type": "array", "byteArray": true, "minItems": 32, "maxItems": 32,
    "contentMediaType": "application/x.dash.dpp.identifier",
    "distinctFrom": "$ownerId"
  },
  "position": 2
}
```

## How it works

- **Create and replace.** After the document's properties pass the JSON schema, each declaring property is compared with what it names: the other property's value in the same document, or the writer's identity for `$ownerId`. Equal values refuse the transition with `DocumentPropertyNotDistinctError` (10419), which names the document type, the property and what it collided with. For a typed array each element is compared on its own.
- **Absent values pass.** When the declaring property or the property it names is left out of the document, there is nothing to compare, and the rule holds. Add the property to `required` when it must be there.
- **Transfer and purchase.** These change the owner and nothing else. The stored document is judged against its new owner, so a transfer to, or a purchase by, the identity held in a property that must differ from `$ownerId` is refused with the same error. A delegation, say, cannot be transferred to its own delegate. A price update changes neither owner nor data and is not judged.
- **Deletes** are never judged.

The check reads the transition (or, for a transfer or purchase, the stored document) and never looks anything else up, and a type without declarations pays nothing for it.

## Rules at registration

- The keyword is allowed only on an identifier property or on the identifier `items` of a typed array. On any other property, the typed array itself included, the meta-schema refuses it (`JsonSchemaError`, 10101).
- The value must be `"$ownerId"` or name a property of the same document type. No other system property is accepted.
- A named property must exist, must itself be an identifier (an identifier that carries `refersTo` counts), must not be an object, and must not be the declaring property.

The parser refuses a value that breaks the last two rules with `InvalidContractStructure` (10231).

## See also

- [Distinct Identifier Properties](../data-model/documents.md#distinct-identifier-properties), the deep dive
- [Typed Arrays](typed-arrays.md), for keywords on `items`
- [propertyConstraints](property-constraints.md): a rule such as `{ "notEqual": ["buyerId", "sellerId"] }` says the same, and can be combined with other conditions
- [Writer and Creator References](owner-refers-to.md), for rules that look the writer up in state
- [Contract Keywords overview](../contract-keywords.md), for the conventions of these tables
