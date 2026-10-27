# generatedFrom

`generatedFrom` says that the platform generates a string property's value with a system function of other properties of the same document, its params. The functions change the case of a string (`lowercase`, `uppercase`, `capitalize`, `camelCase`, `snakeCase`), or fold a name so that names differing only by case or by look-alike characters read the same (`homographSafeASCII`): reach for that one when a unique index must treat `Bob`, `BOB` and `B0B` as one name. A client may leave the property out, and the platform generates it when the document arrives; a client that sends it has it checked. The check reads only the document being written, so it costs no state reads.

From protocol version 15 an integer property can be generated too, with `sys.tokens.decimals`: the decimals of the token an identifier property names, read from the token's contract. A value read from state is generated when the document is created and kept after (see [Functions that read state](#functions-that-read-state)).

| | |
|---|---|
| **Where** | A string property, at the top level or inside an object; from protocol version 15 also an integer property, for `sys.tokens.decimals`. Not on a typed array or its `items`, and not beside `$ref` |
| **Value** | `{ "function": <name>, "params": [<path>, ...] }`: a system function, and as many params as it takes, each the dotted path of a property of the same document type (`"profile.display"` for a nested one), 1 to 256 characters (ASCII, as property names are, so as many bytes) |
| **Default** | Absent: no rule |
| **Since** | protocol version 14; `sys.tokens.decimals` protocol version 15 |
| **On update** | Fixed: adding, removing or changing it is refused (`IncompatibleDocumentTypeSchemaError`, 10246); a property an update adds may declare it only when one of its params is new too (`DocumentTypeUpdateError`, 40212) |
| **Errors** | `DocumentPropertyNotGeneratedError` (10424) on a document, and from protocol version 15 `ReferencedEntityNotFoundError` (40120) for a token that does not exist; at registration `JsonSchemaError` (10101) or `InvalidContractStructure` (10231); on update `IncompatibleDocumentTypeSchemaError` (10246) or `DocumentTypeUpdateError` (40212) |

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
      "type": "string", "maxLength": 63,
      "generatedFrom": {
        "function": "sys.stringTransformations.homographSafeASCII",
        "params": ["label"]
      },
      "position": 1
    }
  },
  "required": ["label", "normalizedLabel"],
  "additionalProperties": false
}
```

A client creates `{ "label": "Bob" }` and the stored document holds `{ "label": "Bob", "normalizedLabel": "b0b" }`. A second handle `{ "label": "B0B" }` generates the same `b0b` and is refused by the unique index. A client that sends `{ "label": "Bob", "normalizedLabel": "b0b" }` gets the same result; one that sends `"normalizedLabel": "bob"` is refused with `DocumentPropertyNotGeneratedError`.

The generated property needs no `pattern` of its own. Every value it can hold is what the function generates from params that passed their own patterns: here `label` admits ASCII letters, digits and `-`, so `normalizedLabel` can only ever hold `a` to `z` without `i`, `l` and `o`, digits and `-`. It keeps `maxLength` because it is indexed, and an indexed string declares one of at most 63.

The DPNS `domain` type declares this rule for `normalizedLabel` and `normalizedParentDomainName` from protocol version 15 (DPNS contract v3). Before it, the type's create data trigger checked the same rule.

## Functions

Functions are system functions, built into the platform and named under `sys.`; any other name is refused at registration. The prefix keeps them apart from functions a contract may bring in a later protocol version. They are grouped in namespaces, and each one declares how many params it takes. The `sys.stringTransformations` functions each take one string:

| Function | Returns | Example |
|---|---|---|
| `sys.stringTransformations.lowercase` | The string with `A` to `Z` lowercased | `Hello World` to `hello world` |
| `sys.stringTransformations.uppercase` | The string with `a` to `z` uppercased | `Hello World` to `HELLO WORLD` |
| `sys.stringTransformations.capitalize` | The first character uppercased and every other lowercased | `hELLO wORLD` to `Hello world` |
| `sys.stringTransformations.camelCase` | The words joined, the first lowercased and every later one capitalized | `Hello World`, `hello_world` and `HelloWorld` to `helloWorld` |
| `sys.stringTransformations.snakeCase` | The words lowercased and joined with `_` | `Hello World`, `helloWorld` and `hello-world` to `hello_world` |
| `sys.stringTransformations.homographSafeASCII` | The string with `A` to `Z` lowercased, then `o` turned into `0` and `i` and `l` into `1` | `Lil-Olive` to `111-011ve` |

The string transformations change ASCII characters only and keep every other character as it is (`Olé` becomes `01é` under `homographSafeASCII`, `OLÉ` becomes `olÉ` under `lowercase`). They use no Unicode tables: Unicode case mappings change between releases of the tools a node is built with, and two nodes that lowercased a character differently would disagree about a document. On ASCII, `homographSafeASCII` is exactly what the DPNS trigger computes.

`camelCase` and `snakeCase` split the string into words. Every ASCII character that is neither a letter nor a digit (a space, `-`, `_`, `.` and so on) separates words and is dropped. A word also starts at an ASCII uppercase letter that follows anything other than another ASCII uppercase letter (`helloWorld` is `hello`, `World`), or that follows one and is followed by an ASCII lowercase letter (`XMLHttpRequest` is `XML`, `Http`, `Request`, so it becomes `xmlHttpRequest` or `xml_http_request`). Characters outside ASCII belong to the word they are in. Every transformation gives back its own output unchanged.

`lowercase`, `uppercase`, `capitalize` and `homographSafeASCII` keep the length of the value, and `camelCase` never lengthens it, but `snakeCase` can: `aBcDeF` becomes `a_bc_de_f`. Give a generated property a `maxLength` that admits what its function can generate from its params' longest value.

The `sys.tokens` functions (protocol version 15) each take one identifier, a token's id, and read the token's configuration:

| Function | Returns | Property |
|---|---|---|
| `sys.tokens.decimals` | The token's decimals, from its conventions, 0 to 255 | An integer property whose `minimum` and `maximum`, when declared, admit 0 to 255, with no `enum` |

A function refuses nothing. Which characters a value may hold is the job of each param's `pattern`: `homographSafeASCII` resists look-alike names only where that pattern admits ASCII alone, as DPNS's does. A contract that admits other scripts can hold two names that look alike but generate different values.

### Functions that read state

A token's decimals live in its contract and can change (a `Conventions` update by whoever controls them). So a property a `sys.tokens` function generates is generated when the document is created and kept after:

```json
"order": {
  "type": "object",
  "properties": {
    "tokenId": {
      "type": "array", "byteArray": true, "minItems": 32, "maxItems": 32,
      "contentMediaType": "application/x.dash.dpp.identifier",
      "position": 0
    },
    "price": { "type": "integer", "minimum": 1, "position": 1 },
    "decimals": {
      "type": "integer", "minimum": 0, "maximum": 255,
      "generatedFrom": { "function": "sys.tokens.decimals", "params": ["tokenId"] },
      "position": 2
    }
  },
  "required": ["tokenId", "price", "decimals"],
  "additionalProperties": false
}
```

- **Create.** The platform reads the token named by `tokenId` and writes its decimals into a document that leaves `decimals` out; a sent value must equal them. A token that does not exist refuses the document with `ReferencedEntityNotFoundError` (40120), naming the param.
- **Replace.** While the replace keeps `tokenId`, the document keeps the decimals it holds, whatever the token's decimals are now: a left-out value is the stored one, and a sent one must equal it. A replace that names another token reads that token's decimals, as a create does.
- **Cost.** Each read is billed to the write: the token's contract info, and its contract unless it is the document's own.
- **Clients.** A client holds no state, so it leaves the property out: the SDK's transition builders remove it, the document checks do not refuse it left out, and a proof verifier takes the value from the proved document, where the platform judged it.

Here `price` is per whole token at the order's `decimals`, so a later change of the token's decimals does not reprice the order.

## Params

For now a param is always a property of the same document, written as its dotted path: a string property for the string transformations, an identifier property (one carrying `refersTo` included) for the token functions. The form leaves room to add, in a later protocol version, literals (`{ "const": ... }`), system values such as `"$ownerId"` and nested calls, without changing what parses today.

## How it works

- **Generated on arrival.** When a document create or replace, or the values of an [index-only](index-only.md) delete, leaves the property out and supplies every param, the platform writes the generated value into the document before anything reads it: contest detection, the schema validation, the indexes and the stored document all see it. A property the document sends is left as sent. A generated value then goes through the property's own schema like a sent one, so any bound it declares, such as `maxLength`, should admit every value the function can generate from its params.
- **Checked after the JSON schema.** Wherever a document's properties are validated, on every create and replace included and in a client that validates a document before sending it, the property must hold what the function generates from its params, and must be absent when a param is. A document that repeats a key in an object on the way to the property or to a param is refused too, since the value it holds there would be ambiguous. A property that breaks this refuses the transition with `DocumentPropertyNotGeneratedError` (10424), which names the document type, the property, the function and its params. A schema error on any of the values is reported first.
- **Absent params.** A property one of whose params is absent must be absent too. To make the params required in effect, list the generated property in `required`: a document without them then fails the schema.
- **Replace.** A replace is judged on the whole new document. Leave the property out to have it generated from the new params; a stale value sent with changed params is refused.
- **Read from state.** A `sys.tokens` value is written or checked while the platform builds the write, before the rest, and kept from the document's creation: see [Functions that read state](#functions-that-read-state).
- **Transfers, purchases and deletes by id** do not change the data and are not judged. An index-only delete is: its values are generated and checked like a create's, so a stale value, or one without its params, refuses it.

The SDK's transition builders generate the property from the document's params, replacing any value the document holds and leaving it out when a param is absent, so a transition built from a document carries the value the platform would generate, and contest detection sees it. A document fetched, edited and sent back through them therefore carries the value of its new params, not the stale one. The property-constraint pre-checks of the JavaScript and FFI SDKs judge the document the same way. A client that validates a document it built, before building a transition, should generate it first (in Rust, `DocumentTypeBasicMethods::regenerate_generated_properties`) or set the value; otherwise the local check reports the property missing. The proof a client verifies after a create or replace is checked against the document with the generated value, as the platform stored it.

## Rules at registration

- The keyword is allowed only on a string property (from protocol version 15 also an integer property, for `sys.tokens.decimals`, whose `minimum` and `maximum` admit 0 to 255 and which declares no `enum`), and not beside `$ref`, whose definition replaces every keyword written next to it (declare it in the definition instead). On any other property, a typed array and its `items` included, the meta-schema refuses it (`JsonSchemaError`, 10101).
- `function` must name a system function, and `params` must list 1 to 16 params, as many as the function takes. The meta-schema refuses an unknown function or an empty or overlong list (`JsonSchemaError`, 10101).
- Every param must name another string property of the same document type (an identifier property for a `sys.tokens` function; not an object, not a system property, not the declaring property), and that property may not be generated itself.
- A `sys.tokens` function is refused on an [indexOnly](index-only.md) type, whose deletes send their values again where a kept value could not be checked, and on a property in a contested index, since a client, which leaves the property out, could not tell whether its document joins a contest.
- Neither the declaring property nor a param may be [transient](transient.md) or sit inside a transient object: a transient value is never stored.
- Every param must sit inside every object that holds the declaring property: a top-level property may take any param, but `profile.normalized` must take params inside `profile`. A document that supplies the params then always holds the object the platform writes the value into.
- On a contract update, a new property may declare `generatedFrom` only when one of its params is new too. Documents stored before the update were never generated, so a new generated property whose params all existed is refused (`DocumentTypeUpdateError`, 40212).

The meta-schema (from protocol version 15, the native schema check) refuses the shape errors of the first two rules with `JsonSchemaError` (10101). The parser refuses a function with the wrong number of params, an integer property that cannot hold every value `sys.tokens.decimals` generates, and a declaration that breaks the param rules (the third to sixth), with `InvalidContractStructure` (10231). The update rule refuses with `DocumentTypeUpdateError` (40212).

## See also

- [Generated Properties](../data-model/documents.md#generated-properties-generatedfrom), the deep dive
- [Property Schemas](property-schemas.md), for `pattern` and `required`
- [Indexes (indices)](indexes.md), for the unique index that usually reads the generated property
- [Contract Keywords overview](../contract-keywords.md), for the conventions of these tables
