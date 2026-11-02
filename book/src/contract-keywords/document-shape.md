# Document Shape

A document type's schema is a JSON Schema object with some Platform keywords added. The keywords in this chapter give a document its outline: it is an object, it has these properties and no others, some of them must be present, and a few JSON Schema rules hold over the document as a whole. What a single property may hold is described in [Property Schemas](property-schemas.md); what may happen to a document (replace, delete, transfer) has chapters of its own.

## Example

The DashPay `profile` type, with its indexes left out:

```json
"profile": {
  "type": "object",
  "properties": {
    "avatarUrl": { "type": "string", "format": "uri", "minLength": 1, "maxLength": 2048, "position": 0 },
    "avatarHash": { "type": "array", "byteArray": true, "minItems": 32, "maxItems": 32, "position": 1 },
    "avatarFingerprint": { "type": "array", "byteArray": true, "minItems": 8, "maxItems": 8, "position": 2 },
    "publicMessage": { "type": "string", "minLength": 1, "maxLength": 140, "position": 3 },
    "displayName": { "type": "string", "minLength": 1, "maxLength": 25, "position": 4 }
  },
  "minProperties": 1,
  "dependentRequired": {
    "avatarUrl": ["avatarHash", "avatarFingerprint"],
    "avatarHash": ["avatarUrl", "avatarFingerprint"],
    "avatarFingerprint": ["avatarUrl", "avatarHash"]
  },
  "required": ["$createdAt", "$updatedAt"],
  "additionalProperties": false
}
```

A profile holds at least one of its five properties, and never any other. The three avatar properties come together or not at all. No property of its own is required, but the platform records the time each profile was created and last updated, because `required` lists `$createdAt` and `$updatedAt`.

## `type`

| | |
|---|---|
| **Where** | The top of a document type. Required. |
| **Value** | `"object"`, the only value allowed |
| **Since** | protocol version 1 |
| **On update** | Fixed (`IncompatibleDocumentTypeSchemaError`, 10246) |
| **Errors** | `JsonSchemaError` (10101) at registration |

A document is always an object: a set of named properties. Properties inside a document have their own `type`, described in [Property Schemas](property-schemas.md#type).

## `properties`

| | |
|---|---|
| **Where** | The top of a document type. Required. |
| **Value** | An object mapping 1 to 100 property names to property schemas |
| **Since** | protocol version 1 |
| **On update** | Properties may be added, never removed (`IncompatibleDocumentTypeSchemaError`, 10246). An added property is optional, or required with [`requiredSince`](required-since.md). |
| **Errors** | `MissingPositionsInDocumentTypePropertiesError` (10411), `JsonSchemaError` (10101), both at registration |

`properties` declares every property a document of the type may hold. Each value is the property's schema: its type, its bounds and the Platform keywords that apply to it (see [Property Schemas](property-schemas.md)).

Rules at registration:

- A document type has 1 to 100 properties. A nested object has the same limit.
- A property name is 1 to 64 letters, digits or underscores (`^[a-zA-Z0-9_]{1,64}$`). Protocol versions 1 to 13 also admitted `-`; from protocol version 14 it is refused.
- Every property has a [`position`](property-schemas.md#position), a number. The top-level positions must run 0, 1, 2 and so on, with no gap and no number used twice (`MissingPositionsInDocumentTypePropertiesError`, 10411). A document is stored with its properties in `position` order and without their names, so the numbers are what tie the stored bytes to the schema.

On update, a new property takes the next free position. It is optional unless it is listed in `required` with `requiredSince` set to the version the update creates. A property that already exists can never be removed, since stored documents may hold it.

## `additionalProperties`

| | |
|---|---|
| **Where** | The top of a document type (required), and every property of type `object` |
| **Value** | `false`, the only value allowed |
| **Since** | protocol version 1 |
| **On update** | Fixed (10246) |
| **Errors** | `JsonSchemaError` (10101): a document holding a property its type does not declare |

A document holds only the properties its type declares. Writing `additionalProperties: false` says so; Platform accepts no other value.

## `required`

| | |
|---|---|
| **Where** | The top of a document type. A property of type `object` has its own `required` for its members (see [Objects](property-schemas.md#objects)). |
| **Value** | An array of names, none repeated: properties of the type, and the system timestamps and block heights |
| **Default** | Absent: every property is optional and no timestamp is recorded |
| **Since** | protocol version 1 |
| **On update** | May gain only a property the same update adds, annotated with `requiredSince`; may lose nothing (`DataContractInvalidRequiredFieldsUpdateError`, 10276) |
| **Errors** | `JsonSchemaError` (10101): a created or replaced document leaves out a required property |

`required` names two kinds of thing:

- **The type's own properties.** Every created or replaced document must hold them. A required property is also stored without the presence byte an optional one carries (unless it is [transient](transient.md)), which is why the list is so hard to change later.
- **System timestamps and block heights**: `$createdAt`, `$updatedAt`, `$transferredAt`, and their `BlockHeight` and `CoreBlockHeight` forms. The writer does not supply these. Listing one tells the platform to record it on every document of the type; one that is not listed is never recorded. See [System Properties](system-properties.md).

Some keywords only work with a timestamp in the list: [`ttl`](ttl.md) needs `$createdAt`, and a [time-range index](time-range.md) needs the timestamp it buckets.

On a contract update, from protocol version 14:

- A property the update adds may be required if it carries `requiredSince` equal to the contract version the update creates. See [requiredSince](required-since.md).
- An existing optional property may not become required.
- Nothing may be removed from the list.
- A system timestamp or block height may not be added. So the set of values a type records is fixed once the type exists.

Each of these is refused with `DataContractInvalidRequiredFieldsUpdateError` (10276). The `required` list of a nested object is fixed (`IncompatibleDocumentTypeSchemaError`, 10246).

## `minProperties` and `maxProperties`

| | |
|---|---|
| **Where** | The top of a document type; also on properties of type `object` |
| **Value** | An integer, 0 or more |
| **Since** | protocol version 1 (declared in the meta-schema from 12) |
| **On update** | Fixed (`IncompatibleDocumentTypeSchemaError`, 10246) |
| **Errors** | `JsonSchemaError` (10101) |

These are the JSON Schema keywords: a document must hold at least `minProperties` and at most `maxProperties` of its own properties. System properties are not counted. The DashPay `profile` above uses `minProperties: 1` so that an empty profile cannot be written.

Meta-schema v0, which covered protocol versions 1 to 11, did not list these keywords at the top of a document type, but it did not refuse keys it did not know either. A contract of that time could use them, and its documents were validated against them: the DashPay contract, registered at protocol version 1, uses `minProperties` and `dependentRequired`. From protocol version 12 the meta-schema lists them and checks their values.

## `dependentRequired`

| | |
|---|---|
| **Where** | The top of a document type; also on properties of type `object` |
| **Value** | An object mapping a property name to an array of property names |
| **Since** | protocol version 1 (declared in the meta-schema from 12) |
| **On update** | Entries, and names within an entry, may be removed, and so may the whole keyword; nothing may be added (`IncompatibleDocumentTypeSchemaError`, 10246) |
| **Errors** | `JsonSchemaError` (10101) |

The JSON Schema keyword: when a document holds the property named by a key, it must also hold every property in that key's array. In the example, a profile with an `avatarUrl` must also have an `avatarHash` and an `avatarFingerprint`. Removing an entry only lets more documents through, which is why removal is the only change allowed.

## `$comment` and `description`

| | |
|---|---|
| **Where** | The top of a document type; also on any property |
| **Value** | A string |
| **Since** | protocol version 1 |
| **On update** | Free: may be added, changed or removed |
| **Errors** | none |

Notes for people reading the contract. Consensus does not act on them.

## `$schema` and `$defs`

| | |
|---|---|
| **Where** | Added by the platform; a document type does not write them |
| **Value** | `$schema`: the document meta-schema's URL. `$defs`: the contract's `schemaDefs`. |
| **Since** | protocol version 1 |
| **Errors** | `InvalidContractStructure` (10231): a document type that writes either key |

When Platform reads a document type, it adds two keys before validating the schema:

- `$schema`, the URL of the document meta-schema, so the schema is checked against it.
- `$defs`, holding the contract's `schemaDefs`: definitions shared by every document type of the contract. A property then refers to one with [`$ref`](property-schemas.md#ref), as `"$ref": "#/$defs/<name>"`.

A document type that writes either key itself is refused. Definitions live only at the contract level, in `schemaDefs`, and are checked as part of each document type: 1 to 100 of them, each named like a property and each a property schema. A contract update may add definitions but not remove them, and a change to a definition follows the update rules of the keywords it holds (`IncompatibleDataContractSchemaError`, 10213). See [Contract-Level Keys and config](contract-config.md).

## Limits on the whole type

- **Name.** A document type's name, its key in `documentSchemas`, is 1 to 64 letters, digits or underscores (`InvalidDocumentTypeNameError`, 10415). Up to protocol version 13 it could also hold `-`.
- **Nesting.** The schema, with the definitions it reaches through `$ref`, nests at most 256 levels of objects and arrays (`DataContractMaxDepthExceedError`, 10200). A `$ref` that does not resolve, or that leads back to itself, is refused (`InvalidJsonSchemaRefError`, 10207).
- **Known keys only.** From protocol version 12 a key the meta-schema does not know is refused at the top of a document type (`JsonSchemaError`, 10101). Before 12 such a key was accepted without being checked. The keys are listed in the [overview](../contract-keywords.md#every-keyword).

## See also

- [Property Schemas](property-schemas.md), for what goes inside `properties`
- [System Properties](system-properties.md), for the timestamps `required` can record
- [requiredSince](required-since.md), for adding a required property in an update
- [Evolving a Contract: Adding Required Fields](../data-model/data-contracts.md#evolving-a-contract-adding-required-fields)
- [Document Serialization](../serialization/document-serialization.md#user-defined-properties), for how `position` and `required` shape the stored bytes
