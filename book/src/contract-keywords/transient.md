# transient

`transient` lists properties that a transition must carry but a stored document never holds. The value is validated with the rest of the document, can be read by the checks that run on the transition, and is then dropped. Reach for it when a write has to prove something with a value that should not be kept, such as a secret salt.

| | |
|---|---|
| **Where** | The top of a document type |
| **Value** | An array of names of top-level properties |
| **Default** | Absent: every property is stored |
| **Since** | protocol version 1. A replace drops the values from protocol version 14. |
| **On update** | Fixed (`IncompatibleDocumentTypeSchemaError`, 10246). The list is compared as a set, so reordering or repeating a name is no change. |
| **Errors** | `InvalidContractStructure` (10231) at registration |

## Example

The DPNS `domain` type, trimmed to two properties:

```json
"domain": {
  "type": "object",
  "documentsMutable": false,
  "properties": {
    "label": { "type": "string", "minLength": 3, "maxLength": 63, "position": 0 },
    "preorderSalt": {
      "type": "array",
      "byteArray": true,
      "minItems": 32,
      "maxItems": 32,
      "description": "Salt used in the preorder document",
      "position": 1
    }
  },
  "required": ["label", "preorderSalt"],
  "transient": ["preorderSalt"],
  "additionalProperties": false
}
```

Registering a name takes two steps. A `preorder` document first commits to a hash of the salt and the name, and the `domain` document then reveals both. Every domain create must carry its 32-byte `preorderSalt`, and DPNS checks it against the preorder. Once the domain is created the salt has done its job, and the stored domain does not hold it.

## How it works

- **Checked like any other value.** A transient value is on the transition when the document is validated, so the schema holds it to its rules: a required transient property must be present, and its bounds apply. `maxBytes` and `distinctFrom` apply to it too, and so do checks such as the DPNS one above.
- **Dropped before storing.** After validation, a create drops the transient values before the document is written. From protocol version 14 a replace drops them the same way. Before 14 a replace stored the values it carried, so a replaced document could hold values its create had dropped.
- **Dropped by top-level name.** Only top-level properties can be transient. When a transient property is an object, the whole object is dropped with everything in it.
- **Never stored, never found.** No stored document holds a transient value, so a query cannot find one and a later reader cannot see it.
- **Sent again on replace.** A replace is validated as a whole document, so a required transient property must be carried by every replace, not only the create.
- **Storage.** A transient property is stored as optional, with a presence byte, even when it is required: in a stored document it is always absent.

## Rules at registration

From protocol version 14, a document type is refused (`InvalidContractStructure`, 10231) when:

- an entry does not name a top-level property of the type. A nested path, a system property or an undeclared name is refused; to drop a nested value, list the object around it;
- an index reads a transient property, or a property inside a transient object. Every stored document would lack the value, so the index could find nothing and a unique index would enforce nothing;
- a reference reads one where the value would have to be stored: a `refersTo` `findBy` may not read one on either side (the params of a `findBy` function may, being read from the create), a `where` may not name one on its referenced side, a key reference may not store its key id with a transient identity, and an `inList` reference may not find its list's document through one. The referring side of a `where` entry may be transient: it is checked on the transition;
- `encryptedFor` names one as its recipient or key id;
- `generatedFrom` sits on one or names one as a param;
- `immutable` lists one. A transient property is always absent from the stored document, so every replace that carries it would count as changing it;
- a `propertyConstraints` rule reads one;
- the type is `indexOnly` and declares any transient property.

Before protocol version 14 none of these was checked.

On a contract update the list is fixed. It decides which values stored documents hold and how every property is encoded, so documents written under one list could not be read under another.

## See also

- [Transient Properties](../data-model/documents.md#transient-properties) in the Documents chapter
- [Document Serialization](../serialization/document-serialization.md#user-defined-properties), for the presence byte a transient property always takes
- [Document Shape](document-shape.md#required), for `required`
- [References (refersTo)](refers-to.md), [encryptedFor](encrypted-for.md), [generatedFrom](generated-from.md), [Mutability](mutability.md), [propertyConstraints](property-constraints.md) and [Index-Only Types](index-only.md), whose rules refuse transient properties
