# requiredSince

`requiredSince` lets a contract update add a property that every new document must hold. Documents already stored were written without it and stay valid: the property is required only of documents written under the contract version the annotation names, or a later one. Reach for it when an app needs a new mandatory field and the contract is already in use.

| | |
|---|---|
| **Where** | A top-level property listed in the document type's `required` |
| **Value** | A contract version: an integer from 1 to 4294967295 |
| **Default** | Absent: a property listed in `required` is required of every document |
| **Since** | protocol version 14 |
| **On update** | May only appear on a property the update adds, equal to the contract version the update creates (`DataContractInvalidRequiredFieldsUpdateError`, 10276). An existing annotation may not be added, changed or removed (`IncompatibleDocumentTypeSchemaError`, 10246). |
| **Errors** | `DataContractInvalidRequiredFieldsUpdateError` (10276), `InvalidContractStructure` (10231), `IncompatibleDocumentTypeSchemaError` (10246), all at registration; `JsonSchemaError` (10101) for a new document without the property |

## Example

Version 2 of a contract has a `post` type with one property, `text`. The update to version 3 adds a required `language`:

```json
"post": {
  "type": "object",
  "properties": {
    "text": { "type": "string", "maxLength": 280, "position": 0 },
    "language": { "type": "string", "minLength": 2, "maxLength": 8, "requiredSince": 3, "position": 1 }
  },
  "required": ["text", "language"],
  "additionalProperties": false
}
```

Every post created or replaced under version 3 or later must have a `language`. Posts written under versions 1 and 2 have none, and remain valid as they are.

## How it works

From protocol version 14, every create and replace stamps the document with the version of the contract it was written under. A transfer or a purchase keeps the stamp the document had, since it does not rewrite the document's properties. See [the contract version stamp](../serialization/document-serialization.md#the-contract-version-stamp-v3).

- **New writes are held to the new schema.** A create must include the property. A replace sends the whole document again, so replacing a document written before the update must add the property too; the document is then stamped with the current version. Documents catch up one at a time, as they are replaced.
- **Older documents are left alone.** A document stamped below the property's `requiredSince` may lack it. It can still be read, transferred, sold and deleted. A document last written before protocol version 14 has no stamp, and counts as older than every annotation.
- **Storage follows the stamp.** A required property is stored without the presence byte an optional one carries. A property with `requiredSince` is stored as required in documents stamped at or above its version, and as optional in older ones. The latest contract alone therefore tells how to read every stored document.
- **Readers must expect gaps.** An app that reads the type should handle documents without the property: every document stamped below the annotation may lack it.
- **No index on it.** A contract update cannot add an index to an existing document type (see [Indexes](indexes.md)), so a property added this way cannot be indexed on that type.

## Rules at registration

- `requiredSince` sits only on a top-level property, and only on one listed in `required`. A nested property, or one that is not required, is refused (`InvalidContractStructure`, 10231). It cannot go on the elements of a typed array.
- The value is never higher than the contract's own version (10276): a property cannot be scheduled to become required later.
- On a new contract, which is version 1, the value may only be 1 (10276).

On an update to an existing document type, which always creates the version one above the current one:

- A property the update adds may be required only if it carries `requiredSince` equal to that new version. Without the annotation, or with any other value, the update is refused (10276).
- An existing property may not become required, with or without the annotation (10276).
- No property may leave `required` (10276).
- A property's existing `requiredSince` may not be changed or removed, and one may not be added to an existing property (`IncompatibleDocumentTypeSchemaError`, 10246).

A document type that the update adds has no older documents. Every `requiredSince` in it must still equal the version the update creates (10276).

## See also

- [Evolving a Contract: Adding Required Fields](../data-model/data-contracts.md#evolving-a-contract-adding-required-fields)
- [The contract version stamp](../serialization/document-serialization.md#the-contract-version-stamp-v3), for how the stamp decides each property's layout
- [Document Shape](document-shape.md#required), for the other rules of `required`
