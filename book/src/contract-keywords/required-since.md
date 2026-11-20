# requiredSince

`requiredSince` lets a contract update add a property that every new document must hold. Documents already stored were written without it and stay valid: the property is required only of documents written under the contract version the annotation names, or a later one. Reach for it when an app needs a new mandatory field and the contract is already in use.

| | |
|---|---|
| **Where** | A top-level property listed in the document type's `required`; from protocol version 15 also a member of an object, listed in that object's `required` (or the `required` beside its `$ref`) |
| **Value** | A contract version: an integer from 1 to 4294967295 |
| **Default** | Absent: a property listed in `required` is required of every document |
| **Since** | protocol version 14 |
| **On update** | May only appear on a property or object member the update adds, equal to the contract version the update creates (`DataContractInvalidRequiredFieldsUpdateError`, 10276). An existing annotation may not be added, changed or removed (`IncompatibleDocumentTypeSchemaError`, 10246; `IncompatibleDataContractSchemaError`, 10213, in a `$defs` definition). |
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

## Members of an object

From protocol version 15 an update may also add a required member to an object the type already has and writes inline. The member goes after the object's existing members, in its `properties` map and at a position above theirs, carries `requiredSince` equal to the version the update creates, and is listed in the object's own `required`. An object that takes its members from a `$ref` gains them only as optional: the `required` beside the `$ref` stays as it is, since the definition may be shared, and the definition's own lists are fixed:

```json
"post": {
  "type": "object",
  "properties": {
    "text": { "type": "string", "maxLength": 280, "position": 0 },
    "meta": {
      "type": "object",
      "position": 1,
      "properties": {
        "author": { "type": "string", "maxLength": 64, "position": 0 },
        "language": { "type": "string", "minLength": 2, "maxLength": 8, "requiredSince": 3, "position": 1 }
      },
      "required": ["author", "language"],
      "additionalProperties": false
    }
  },
  "required": ["text", "meta"],
  "additionalProperties": false
}
```

- **New writes are held to it, older documents are left alone,** as for a top-level property: every `meta` created or replaced under version 3 or later must hold a `language`, and documents written earlier may lack it.
- **Storage does not follow the stamp.** Such a member is stored with the presence byte an optional member carries, in every document whatever its stamp, so a document written before the update reads with the member absent.
- **Listed in the object's own `required`.** A dotted name in the type's `required` (`"meta.language"`) cannot carry it: no `required` list gains a dotted name on update. A dotted name may leave a list only while another list still requires the member it names, which repairs a type whose dotted name no document could meet (see [Document Shape](document-shape.md#required)).
- **Not through a `$ref`.** On update the `required` beside a `$ref` can not change (10276), and a `$defs` definition's own `required`, at any depth, is not read for storage, so a member it alone lists cannot carry `requiredSince` (10231). Through a `$ref` members can only be added as optional. At registration a definition's member may be required through the `required` beside each `$ref` that uses it; every property that references the definition must then list the member there, or its parse refuses the `requiredSince`, and a property an update adds may reference such a definition whatever version its member names, since no stored document holds the property.
- **Not past `maxProperties`.** An object's `required` may not name more members than its `maxProperties` allows, which no update can change: such an update is refused (10276).
- **Not where a stored document could not add it.** A replace sends the whole document again, so a document stored before the update would have to add the member. Such a member can only be added as optional (10276) under a property listed in [`immutable`](mutability.md) (with a condition or without), and inside an object declaring a whole-object `const` or `enum`, which a stored document holding the member may not meet (an update that drops the `const` or `enum` may add it as required). It is also added only as optional in a type whose moderators change fields (`moderatorAbilities.changeFields`): a moderator's change checks the whole document again, and can not add the member to a document stored before.
- **Not past the contract's version.** A member inside an object the update adds may be required outright, or carry a `requiredSince` up to the version the update creates, never past it (10276).

## Rules at registration

- `requiredSince` sits only on a property listed as required: a top-level property in the type's `required`, or, from protocol version 15, an object's member in that object's `required` or the `required` beside its `$ref`. A property that is not required is refused (`InvalidContractStructure`, 10231), and so is a nested one before protocol version 15. It cannot go on the elements of a typed array.
- The value is never higher than the contract's own version (10276): a property cannot be scheduled to become required later.
- On a new contract, which is version 1, the value may only be 1 (10276).

On an update to an existing document type, which always creates the version one above the current one:

- A property the update adds may be required only if it carries `requiredSince` equal to that new version. Without the annotation, or with any other value, the update is refused (10276).
- An existing property may not become required, with or without the annotation (10276).
- No property may leave `required` (10276).
- A property's existing `requiredSince` may not be changed or removed, and one may not be added to an existing property (`IncompatibleDocumentTypeSchemaError`, 10246). From protocol version 15 the same holds for an object's member (10276).
- These rules judge the update that adds the property. A later update may still list it in [`immutable`](mutability.md), whose list may grow, and so may it list an object holding a member with `requiredSince`. The documents written before that version can then no longer be replaced: a replace would have to add the property, which changes what `immutable` freezes. This is the same for a top-level property and for an object's member.

A document type that the update adds has no older documents. Every `requiredSince` on its own properties must still equal the version the update creates (10276). The members of an object, in a type the update adds or an object it adds to an existing type, may be required outright, since no stored document holds the object, or carry `requiredSince` up to the contract's version.

## See also

- [Evolving a Contract: Adding Required Fields](../data-model/data-contracts.md#evolving-a-contract-adding-required-fields)
- [The contract version stamp](../serialization/document-serialization.md#the-contract-version-stamp-v3), for how the stamp decides each property's layout
- [Document Shape](document-shape.md#required), for the other rules of `required`
