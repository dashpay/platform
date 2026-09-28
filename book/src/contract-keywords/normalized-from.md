# normalizedFrom

`normalizedFrom` says that a string property holds a normalized form of another string property of the same document, its source: a name folded so that names differing only by case or by look-alike characters read the same. Reach for it when a unique index must treat `Bob`, `BOB` and `B0B` as one name. A client may leave the property out, and the platform computes it from the source when the document arrives; a client that sends it has it checked. The check reads only the document being written, so it costs no state reads.

| | |
|---|---|
| **Where** | A string property, at the top level or inside an object. Not on a typed array or its `items` |
| **Value** | `{ "property": <path>, "transform": "homographSafeASCII" }`: the dotted path of the source string property (`"profile.display"` for a nested one), 1 to 256 characters, and the transform |
| **Default** | Absent: no rule |
| **Since** | protocol version 14 |
| **On update** | Fixed: adding, removing or changing it is refused (`IncompatibleDocumentTypeSchemaError`, 10246) |
| **Errors** | `DocumentPropertyNotNormalizedError` (10424) on a document; at registration `JsonSchemaError` (10101) or `InvalidContractStructure` (10231) |

## Example

```json
"handle": {
  "type": "object",
  "indices": [
    { "name": "byNormalizedLabel", "properties": [{ "normalizedLabel": "asc" }], "unique": true }
  ],
  "properties": {
    "label": {
      "type": "string", "pattern": "^[a-zA-Z0-9-]{3,63}$", "maxLength": 63,
      "position": 0
    },
    "normalizedLabel": {
      "type": "string", "pattern": "^[a-hj-km-np-z0-9-]{3,63}$", "maxLength": 63,
      "normalizedFrom": { "property": "label", "transform": "homographSafeASCII" },
      "position": 1
    }
  },
  "required": ["label", "normalizedLabel"],
  "additionalProperties": false
}
```

A client creates `{ "label": "Bob" }` and the stored document holds `{ "label": "Bob", "normalizedLabel": "b0b" }`. A second handle `{ "label": "B0B" }` normalizes to the same `b0b` and is refused by the unique index. A client that sends `{ "label": "Bob", "normalizedLabel": "b0b" }` gets the same result; one that sends `"normalizedLabel": "b1b"` is refused with `DocumentPropertyNotNormalizedError`.

This is the rule the DPNS `domain` type's data trigger checks today for `normalizedLabel` and `normalizedParentDomainName`, written into the schema.

## The transform

`homographSafeASCII` takes the source one character at a time: `A` to `Z` become lowercase, then `o` becomes `0`, and `i` and `l` become `1`. Every other character, ASCII or not, is kept as it is. So `Lil-Olive` becomes `111-011ve`, and `Olé` becomes `01é`.

It uses no Unicode tables. Unicode case mappings change between releases of the tools a node is built with, and two nodes that lowercase a character differently would disagree about a document. On ASCII the transform is exactly what the DPNS trigger computes, and it keeps the byte length of the value.

The transform refuses nothing. Which characters a value may hold is the job of the properties' `pattern`: the transform resists look-alike names only where the pattern admits ASCII alone, as DPNS's does. A contract that admits other scripts can hold two names that look alike but normalize differently.

## How it works

- **Filled on arrival.** When a document create or replace, or the values of an [index-only](index-only.md) delete, leaves the property out and supplies the source, the platform writes the normalized form into the document before anything reads it: contest detection, the schema validation, the indexes and the stored document all see it. A property the document sends is left as sent. A computed value then goes through the property's own schema like a sent one, so the property's `pattern` and `maxLength` should admit the normalized form of every value the source admits (the transform keeps the length).
- **Checked after the JSON schema.** Wherever a document's properties are validated, on every create and replace included and in a client that validates a document before sending it, the property must hold the source's normalized form, and must be absent when the source is. A property that breaks this refuses the transition with `DocumentPropertyNotNormalizedError` (10424), which names the document type, the property, its source and the transform. A schema error on either value is reported first.
- **Absent source.** A property whose source is absent must be absent too. To make the source required in effect, list the normalized property in `required`: a document without the source then fails the schema.
- **Replace.** A replace is judged on the whole new document. Leave the property out to have it computed from the new source; a stale value sent with a changed source is refused.
- **Transfers, purchases and deletes** do not change the data and are not judged.

A document a client builds has not been through the platform's fill, so a client that validates it locally before sending should fill it first (in Rust, `DocumentTypeBasicMethods::fill_normalized_properties`) or send the normalized value; otherwise the local check reports the property missing. The proof a client verifies after a create or replace is checked against the filled document, as the platform stored it.

## Rules at registration

- The keyword is allowed only on a string property. On any other property, a typed array and its `items` included, the meta-schema refuses it (`JsonSchemaError`, 10101).
- `property` must name another string property of the same document type (not an object, not a system property, not the declaring property), and that property may not declare `normalizedFrom` itself.
- Neither the declaring property nor its source may be [transient](transient.md) or sit inside a transient object: a transient value is never stored.
- The source must sit inside every object that holds the declaring property: a top-level property may take any source, but `profile.normalized` must take one inside `profile`. A document that supplies the source then always holds the object the platform writes the normalized form into.

The parser refuses a declaration that breaks these rules with `InvalidContractStructure` (10231).

## See also

- [Normalized String Properties](../data-model/documents.md#normalized-string-properties-normalizedfrom), the deep dive
- [Property Schemas](property-schemas.md), for `pattern` and `required`
- [Indexes (indices)](indexes.md), for the unique index that usually reads the normalized property
- [Contract Keywords overview](../contract-keywords.md), for the conventions of these tables
