# maxBytes

`maxBytes` caps how many bytes a string may take when it is encoded as UTF-8, which is how Platform stores it. JSON Schema's `maxLength` counts characters, and one character takes from one to four bytes, so `maxLength` alone does not bound the stored size. Reach for `maxBytes` when the size of a document matters: to keep storage fees predictable, or to stay under the 5120 bytes any one stored value may take.

| | |
|---|---|
| **Where** | A string property, at the top level or inside an object; or the `items` of a typed array of strings, where it bounds every element |
| **Value** | An integer from 1 to 65535, no lower than the property's `minLength` |
| **Default** | Absent: only `maxLength` and the 5120-byte cap on every value apply |
| **Since** | protocol version 14 |
| **On update** | May be raised or removed; adding it or lowering it is refused (`IncompatibleDocumentTypeSchemaError`, 10246) |
| **Errors** | `DocumentPropertyMaxBytesExceededError` (10421) on a document; at registration `JsonSchemaError` (10101) or `InvalidContractStructure` (10231) |

## Example

```json
"description": {
  "type": "string",
  "minLength": 1,
  "maxLength": 4096,
  "maxBytes": 4096,
  "position": 1
}
```

This is the `description` of a proposal in the moderation charters system contract. `maxLength` admits 4096 characters, which could take up to 16384 bytes (more than the 5120-byte cap on a stored value); `maxBytes` holds the stored text to 4096 bytes, so a description in plain ASCII can use every character and one written only in four-byte characters, such as most emoji, a quarter of them.

On a typed array the keyword goes on `items`:

```json
"tags": {
  "type": "array",
  "maxItems": 8,
  "items": { "type": "string", "minLength": 1, "maxLength": 32, "maxBytes": 64 },
  "position": 2
}
```

Each of up to 8 tags is at most 32 characters and at most 64 bytes.

## How it works

- The check runs wherever a document's properties are validated: every create and replace in consensus, and every client that validates a document before sending it.
- It runs after the JSON schema validation. A value the schema refuses (too many characters, the wrong type) is reported with the schema's error, not this one.
- Every string the document holds for a property that declares `maxBytes` is measured in UTF-8 bytes. One that is longer refuses the transition with `DocumentPropertyMaxBytesExceededError` (10421), which names the property and both lengths. For a typed array the error names the element, as in `tags[2]`.
- A property the document leaves out is not checked.

The cap bounds the value only; `maxLength` and `minLength` still apply in characters. Setting both is normal: `maxLength` says what a user may type, `maxBytes` what the platform stores.

## Rules at registration

- The keyword is allowed only on a string property or on the string `items` of a typed array. On any other property, the typed array itself included, the meta-schema refuses it (`JsonSchemaError`, 10101).
- The value is an integer from 1 to 65535 (10101).
- It may not be lower than `minLength`: a string of `minLength` characters takes at least that many bytes, so a lower cap would refuse every value (`InvalidContractStructure`, 10231).

## See also

- [Byte Caps on Strings](../data-model/documents.md#byte-caps-on-strings-maxbytes), the deep dive
- [Property Schemas](property-schemas.md), for `maxLength` and `minLength`
- [Typed Arrays](typed-arrays.md), for keywords on `items`
- [Contract Keywords overview](../contract-keywords.md), for the 5120-byte value limit and the conventions of these tables
