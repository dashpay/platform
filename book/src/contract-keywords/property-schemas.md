# Property Schemas

Each entry of a document type's `properties` is a property schema: JSON Schema (draft 2020-12), limited to the keywords in this chapter, plus three Platform keywords that say how a value is stored (`position`, `byteArray` and `contentMediaType`). The schema is checked when the contract is registered, and every created or replaced document is validated against it. Platform keywords with more to them, such as [`maxBytes`](max-bytes.md), [`refersTo`](refers-to.md), [`distinctFrom`](distinct-from.md), [`encryptedFor`](encrypted-for.md), [`generatedFrom`](generated-from.md), [`requiredSince`](required-since.md) and a typed array's [`items`](typed-arrays.md), have chapters of their own.

| Keyword | Applies to | On update |
|---|---|---|
| [`type`](#type) | every property | Fixed |
| [`position`](#position) | every property | Fixed |
| [`minLength`, `maxLength`, `pattern`, `format`](#strings) | strings | Loosened or removed only |
| [`minimum`, `maximum`, `exclusiveMinimum`, `exclusiveMaximum`, `multipleOf`](#numbers) | integers and numbers | Loosened or removed only, keeping an integer's width; `multipleOf` fixed |
| [`enum`, `const`](#enum-and-const) | any | `enum` may gain values; `const` may be removed |
| [`byteArray`, `contentMediaType`](#byte-arrays-and-identifiers) | byte arrays | Fixed |
| [`minItems`, `maxItems`, `uniqueItems`, `contains`](#arrays) | arrays | Loosened or removed only; `contains` fixed |
| [`properties`, `required`, `additionalProperties`, `minProperties`, `maxProperties`, `dependentRequired`](#objects) | objects | Members may be added; the rest fixed, except `dependentRequired` may lose entries |
| [`$ref`](#ref) | any | Fixed |
| [`$id`, `$comment`, `description`, `examples`](#annotations) | any | Free (`$id` may only be added) |

Each section gives the error a contract update gets for breaking its rules. Most are refused with `IncompatibleDocumentTypeSchemaError` (10246), from the comparison of the old and new schemas; a change to how a value is stored is refused with `DocumentTypeUpdateError` (40212).

## How property schemas are checked

When a contract is registered or updated:

- The schema is checked against the document meta-schema. A keyword the meta-schema does not allow where it is written, or a value of the wrong shape, is refused with `JsonSchemaError` (10101).
- The parser reads each property's type and bounds, and refuses what the meta-schema cannot express (`InvalidContractStructure`, 10231).
- The schema is compiled for validating documents. A `pattern` that is not a valid regular expression, or a `format` the validator does not know, is refused here (`JsonSchemaError`, 10101).

When a document is created or replaced:

1. Every string and byte array value is at most 5120 bytes, whatever the schema allows (`DocumentFieldMaxSizeExceededError`, 10417). From protocol version 14 a value nested more than 256 levels deep is refused as well (`ValueError`, 10103).
2. The document's properties are validated against the schema. Each failure is reported as a `JsonSchemaError` (10101) naming the keyword and the property.
3. Platform's own checks, which JSON Schema cannot express, come next: [`maxBytes`](max-bytes.md) and [`propertyConstraints`](property-constraints.md) among them.

A transfer, a price update and a purchase carry no property values, so they are not validated against the schema again.

The schema also decides how each value is stored, since a stored document holds no property names or type tags:

| Property | Stored as |
|---|---|
| `integer` | 1, 2, 4 or 8 bytes, chosen by its bounds (see [Numbers](#numbers)) |
| `number` | 8 bytes, a 64-bit floating point number |
| `boolean` | 1 byte |
| `string` | a length prefix, then the UTF-8 bytes |
| byte array with `minItems` equal to `maxItems` | the bytes, with no prefix |
| any other byte array | a length prefix, then the bytes |
| identifier | 32 bytes |
| `object` | a length prefix, then its members |
| typed array | an element count, then the elements (see [Typed Arrays](typed-arrays.md)) |

An optional property adds one byte in front that says whether it is present. See [Document Serialization](../serialization/document-serialization.md#value-encoding-by-type) for the exact encoding.

## `type`

| | |
|---|---|
| **Where** | Every property; also the elements of a typed array |
| **Value** | One of `"string"`, `"integer"`, `"number"`, `"boolean"`, `"object"`, `"array"` |
| **Since** | protocol version 1 |
| **On update** | Fixed (`IncompatibleDocumentTypeSchemaError`, 10246) |
| **Errors** | `JsonSchemaError` (10101): a document value of another type |

`type` is a single name. A list of types, and `"null"`, are refused at registration: every stored value needs one known encoding.

An `array` is one of two things:

- a **byte array**, with `byteArray: true`: a string of bytes, such as a hash or an [identifier](#byte-arrays-and-identifiers);
- from protocol version 14, a **typed array**, with an `items` schema: a list of values of one scalar type. See [Typed Arrays](typed-arrays.md).

An array that is neither is refused.

## `position`

| | |
|---|---|
| **Where** | Every property, at every level. Not on the elements of a typed array. |
| **Value** | An integer, 0 or more |
| **Since** | protocol version 1 |
| **On update** | Fixed (10246) |
| **Errors** | `MissingPositionsInDocumentTypePropertiesError` (10411) at registration |

A document is stored with its values one after another and no property names. `position` is the property's place in that sequence.

- The top-level properties of a document type must use the positions 0, 1, 2 and so on, with no gap and no number used twice (10411). A top-level property without a `position` is refused.
- The members of an object need a `position` too. Number them from 0 within the object; only the top level is checked for gaps.
- A property that takes its schema from a [`$ref`](#ref) writes its `position` next to the `$ref`.
- A property added by a contract update takes the next free position. An existing position can never change, or stored documents would be read in the wrong order.

## Strings

| | |
|---|---|
| **Keywords** | `minLength`, `maxLength`, `pattern`, `format` |
| **Where** | Properties of type `string`, and the string elements of a typed array |
| **Value** | `minLength`, `maxLength`: an integer, 0 or more, counting characters. `pattern`: a regular expression. `format`: the name of a JSON Schema format, such as `"uri"` |
| **Since** | protocol version 1 |
| **On update** | `maxLength` may be raised or removed, and `minLength` lowered or removed. `pattern` and `format` may be removed. None of them may be added, and no other change is allowed (10246). |
| **Errors** | `JsonSchemaError` (10101) |

```json
"username": {
  "type": "string",
  "minLength": 3,
  "maxLength": 63,
  "pattern": "^[a-zA-Z0-9_]+$",
  "position": 0
}
```

A `username` is 3 to 63 letters, digits or underscores.

- `minLength` and `maxLength` count characters, and a character takes 1 to 4 bytes in UTF-8. To cap the stored size, add [`maxBytes`](max-bytes.md). Whatever `maxLength` says, no single string may exceed 5120 bytes (10417).
- `pattern` is written in the syntax of Rust's `regex` crate, which has no lookaround and no backreferences. A pattern that does not compile is refused at registration (10101).
- `format` is checked on every document. The validator knows `date-time`, `date`, `time`, `email`, `idn-email`, `hostname`, `ipv4`, `ipv6`, `uri` and `regex`. Any other format, `uuid` and `uri-reference` included, is refused at registration (10101).
- A string with `pattern` or `format` must declare a `maxLength` of at most 50000, so that matching stays cheap.
- A string used in an index needs a `maxLength` of at most 63 (`InvalidIndexedPropertyConstraintError`, 10205). See [Indexes](indexes.md).

## Numbers

| | |
|---|---|
| **Keywords** | `minimum`, `maximum`, `exclusiveMinimum`, `exclusiveMaximum`, `multipleOf` |
| **Where** | Properties of type `integer` or `number`, and such elements of a typed array |
| **Value** | A number. On an integer element of a typed array, `minimum` and `maximum` are integers. |
| **Since** | protocol version 1 |
| **On update** | `maximum` and `exclusiveMaximum` may be raised or removed, and `minimum` and `exclusiveMinimum` lowered or removed, unless that changes how an integer is stored (`DocumentTypeUpdateError`, 40212). None may be added, and `multipleOf` is fixed (10246). |
| **Errors** | `JsonSchemaError` (10101) |

```json
"rating": { "type": "integer", "minimum": 1, "maximum": 5, "position": 2 }
```

A `rating` is a whole number from 1 to 5, and is stored in one byte.

A `number` is always stored in 8 bytes. An `integer` is stored in the smallest width its `minimum` and `maximum` allow, when the contract's config has `sizedIntegerTypes` on (the default; see [Contract-Level Keys and config](contract-config.md)):

| `minimum` | `maximum` | Stored as |
|---|---|---|
| 0 or more | up to 255 | 1 byte, unsigned |
| 0 or more | up to 65535 | 2 bytes, unsigned |
| 0 or more | up to 4294967295 | 4 bytes, unsigned |
| 0 or more | higher | 8 bytes, unsigned |
| below 0 | both bounds within -128 to 127 | 1 byte, signed |
| below 0 | both bounds within -32768 to 32767 | 2 bytes, signed |
| below 0 | both bounds within -2147483648 to 2147483647 | 4 bytes, signed |
| below 0 | otherwise | 8 bytes, signed |

With only a `minimum`, the integer takes 8 bytes, unsigned when the minimum is 0 or more. With only a `maximum`, it takes the unsigned width the maximum gives, so an integer that may be negative needs a `minimum` too. With neither, an `enum` of integers picks the width from its smallest and largest members; without one, the integer takes 8 bytes, signed. `exclusiveMinimum` and `exclusiveMaximum` do not affect the width. With `sizedIntegerTypes` off, every integer takes 8 bytes, signed.

Stored documents and index entries hold each integer at its width, so a contract update may not change the width or the sign. Raising `maximum` past the width, lowering `minimum` below 0, removing a bound, or adding an `enum` value outside the width is refused with `DocumentTypeUpdateError` (40212); so is turning `sizedIntegerTypes` on when it would change an existing integer's width. A change that keeps the width is accepted.

## `enum` and `const`

| | |
|---|---|
| **Where** | Any property. `enum` also on the string, integer, number and boolean elements of a typed array; `const` never on elements. |
| **Value** | `enum`: an array of one or more values, none repeated. `const`: one value. |
| **Since** | protocol version 1 |
| **On update** | `enum` may gain values but not lose one; the keyword may be removed, not added. `const` may be removed, not added or changed (10246). An `enum` value that changes an integer's width is refused (`DocumentTypeUpdateError`, 40212). |
| **Errors** | `JsonSchemaError` (10101) |

```json
"status": { "type": "string", "enum": ["open", "closed", "archived"], "position": 3 }
```

`enum` lists the values a property may take and `const` the one value it must take. Both only narrow what documents may hold, so an update may widen them (more `enum` values, or no keyword at all) but never narrow them, which could leave stored documents invalid. On an integer without bounds, `enum` also sets the stored width (see [Numbers](#numbers)).

## Byte arrays and identifiers

| | |
|---|---|
| **Keywords** | `byteArray`, `contentMediaType` |
| **Where** | Properties of type `array`, and the array elements of a typed array |
| **Value** | `byteArray`: `true`, the only value. `contentMediaType`: `"application/x.dash.dpp.identifier"` makes the byte array an identifier. |
| **Since** | protocol version 1 |
| **On update** | Fixed (10246). The length bounds follow the rules of [Arrays](#arrays), but a byte array may not switch between a fixed and a variable length, or change its fixed length (`DocumentTypeUpdateError`, 40212). |
| **Errors** | `JsonSchemaError` (10101) |

```json
"authorId": {
  "type": "array",
  "byteArray": true,
  "minItems": 32,
  "maxItems": 32,
  "contentMediaType": "application/x.dash.dpp.identifier",
  "position": 1
}
```

`authorId` is an identifier: the 32-byte id of an identity, a document, a contract or a token.

- `byteArray: true` makes an array a string of bytes. Its `minItems` and `maxItems` count bytes. When the two are equal the bytes are stored as they are; otherwise they carry a length prefix.
- `contentMediaType: "application/x.dash.dpp.identifier"` makes a byte array an identifier. It must come with `byteArray: true`, `minItems: 32` and `maxItems: 32`, and it may not carry `uniqueItems`. An identifier is shown in base58, and it is the kind of property [`distinctFrom`](distinct-from.md) and most [`refersTo`](refers-to.md) targets are declared on.
- A byte array used in an index needs a `maxItems` of at most 255 (`InvalidIndexedPropertyConstraintError`, 10205).
- On a typed array, `contentMediaType` belongs on the `items`, not on the array.

## Arrays

| | |
|---|---|
| **Keywords** | `minItems`, `maxItems`, `uniqueItems`, `contains` |
| **Where** | Properties of type `array`. `minItems` and `maxItems` also on byte array elements of a typed array. |
| **Value** | `minItems`, `maxItems`: an integer, 0 or more. `uniqueItems`: a boolean. `contains`: a schema. |
| **Since** | protocol version 1 |
| **On update** | `maxItems` may be raised or removed and `minItems` lowered or removed (10246), within the byte array rule above (40212); a typed array keeps its `maxItems`. `uniqueItems` may be removed or set to `false`, not added (10246). `contains` is fixed (10246). |
| **Errors** | `JsonSchemaError` (10101) |

- `minItems` and `maxItems` count bytes on a byte array and elements on a typed array. A typed array must declare `maxItems`, at most 1024.
- `uniqueItems: true` on a typed array refuses a document that repeats an element. On a plain byte array it refuses a repeated byte. It is refused on an identifier, and on the elements of a typed array.
- `contains` is the JSON Schema keyword: at least one element must match the schema it holds.

## Objects

| | |
|---|---|
| **Keywords** | `properties`, `required`, `additionalProperties`, `minProperties`, `maxProperties`, `dependentRequired` |
| **Where** | Properties of type `object` |
| **Value** | The same as at the top of a document type: see [Document Shape](document-shape.md) |
| **Since** | protocol version 1 |
| **On update** | Members may be added, never removed; `required` and `additionalProperties` are fixed; `dependentRequired` may lose entries, not gain them (10246). `minProperties` and `maxProperties` are fixed (10246). |
| **Errors** | `JsonSchemaError` (10101) |

```json
"records": {
  "type": "object",
  "properties": {
    "identity": {
      "type": "array",
      "byteArray": true,
      "minItems": 32,
      "maxItems": 32,
      "contentMediaType": "application/x.dash.dpp.identifier",
      "position": 0
    }
  },
  "minProperties": 1,
  "additionalProperties": false,
  "position": 5
}
```

An object groups members under one property, as DPNS groups a name's records.

- An object declares `properties`, 1 to 100 members each with a `position`, and `additionalProperties: false`, unless it takes its schema from a [`$ref`](#ref).
- Its `required` names the members every value of the object must hold. It cannot change after the type exists, so a member added by an update is optional.
- An object is stored with its members inline. An object cannot be indexed, but a member can: an index names it by its dotted path, such as `records.identity`.

## `$ref`

| | |
|---|---|
| **Where** | Any property, except the elements of a typed array |
| **Value** | `"#/$defs/<name>"`: a definition in the contract's `schemaDefs` |
| **Since** | protocol version 1 |
| **On update** | Fixed (10246) |
| **Errors** | `InvalidJsonSchemaRefError` (10207) at registration |

`$ref` lets several document types share one schema, written once in the contract's `schemaDefs`:

```json
"schemaDefs": {
  "address": {
    "type": "object",
    "properties": {
      "street": { "type": "string", "maxLength": 100, "position": 0 },
      "city": { "type": "string", "maxLength": 50, "position": 1 }
    },
    "required": ["street", "city"],
    "additionalProperties": false
  }
}
```

A property of any document type of the contract then reads `"shippingAddress": { "$ref": "#/$defs/address", "position": 2 }`.

- Only local references, starting with `#`, are allowed. The platform places `schemaDefs` under `$defs` in every document type (see [`$schema` and `$defs`](document-shape.md#schema-and-defs)).
- A reference that does not resolve, or that leads back to itself, is refused (10207).
- The property keeps its own `position`; the definition supplies everything else.
- The `$ref` itself cannot change on update. The definition it points at can, under the rules of the keywords it holds (`IncompatibleDataContractSchemaError`, 10213).

## Annotations

| | |
|---|---|
| **Keywords** | `$id`, `$comment`, `description`, `examples` |
| **Where** | Any property. `$comment` and `description` also on the elements of a typed array. |
| **Value** | `$id`: a string starting with `#`. `$comment`, `description`: a string. `examples`: an array of values. |
| **Since** | protocol version 1 |
| **On update** | `$comment`, `description` and `examples`: free. `$id`: may be added, not removed or changed (10246). |
| **Errors** | none |

Notes for people and tools reading the contract. They do not change what a document may hold.

## See also

- [Document Shape](document-shape.md), for the keywords at the top of a document type
- [Typed Arrays](typed-arrays.md), for arrays of values
- [maxBytes](max-bytes.md), for capping a string's size in bytes
- [Document Serialization](../serialization/document-serialization.md), for the stored form of every type
- [Indexes](indexes.md), for the limits on indexed properties
- [Error Codes](../error-handling/error-codes.md)
