# generatedFrom

`generatedFrom` says that the platform generates a string property's value with a system function of other properties of the same document, its params. The functions change the case of a string (`lowercase`, `uppercase`, `capitalize`, `camelCase`, `snakeCase`), or fold a name so that names differing only by case or by look-alike characters read the same (`homographSafeASCII`): reach for that one when a unique index must treat `Bob`, `BOB` and `B0B` as one name. A client may leave the property out, and the platform generates it when the document arrives; a client that sends it has it checked. The check reads only the document being written, so it costs no state reads.

| | |
|---|---|
| **Where** | A string property, at the top level or inside an object. Not on a typed array or its `items`, and not beside `$ref` |
| **Value** | `{ "function": <name>, "params": [<path>, ...] }`: a system function, and as many params as it takes, each the dotted path of a property of the same document type (`"profile.display"` for a nested one), 1 to 256 characters (ASCII, as property names are, so as many bytes) |
| **Default** | Absent: no rule |
| **Since** | protocol version 14 |
| **On update** | Fixed: adding, removing or changing it is refused (`IncompatibleDocumentTypeSchemaError`, 10246); a property an update adds may declare it only when one of its params is new too (`DocumentTypeUpdateError`, 40212) |
| **Errors** | `DocumentPropertyNotGeneratedError` (10424) on a document; at registration `JsonSchemaError` (10101) or `InvalidContractStructure` (10231); on update `IncompatibleDocumentTypeSchemaError` (10246) or `DocumentTypeUpdateError` (40212) |

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

This is the rule the DPNS `domain` type's data trigger checks today for `normalizedLabel` and `normalizedParentDomainName`, written into the schema.

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

A function refuses nothing. Which characters a value may hold is the job of each param's `pattern`: `homographSafeASCII` resists look-alike names only where that pattern admits ASCII alone, as DPNS's does. A contract that admits other scripts can hold two names that look alike but generate different values.

## Params

For now a param is always a property of the same document, written as its dotted path. The form leaves room to add, in a later protocol version, literals (`{ "const": ... }`), system values such as `"$ownerId"` and nested calls, without changing what parses today.

## How it works

- **Generated on arrival.** When a document create or replace, or the values of an [index-only](index-only.md) delete, leaves the property out and supplies every param, the platform writes the generated value into the document before anything reads it: contest detection, the schema validation, the indexes and the stored document all see it. A property the document sends is left as sent. A generated value then goes through the property's own schema like a sent one, so any bound it declares, such as `maxLength`, should admit every value the function can generate from its params.
- **Checked after the JSON schema.** Wherever a document's properties are validated, on every create and replace included and in a client that validates a document before sending it, the property must hold what the function generates from its params, and must be absent when a param is. A document that repeats a key in an object on the way to the property or to a param is refused too, since the value it holds there would be ambiguous. A property that breaks this refuses the transition with `DocumentPropertyNotGeneratedError` (10424), which names the document type, the property, the function and its params. A schema error on any of the values is reported first.
- **Absent params.** A property one of whose params is absent must be absent too. To make the params required in effect, list the generated property in `required`: a document without them then fails the schema.
- **Replace.** A replace is judged on the whole new document. Leave the property out to have it generated from the new params; a stale value sent with changed params is refused.
- **Transfers, purchases and deletes by id** do not change the data and are not judged. An index-only delete is: its values are generated and checked like a create's, so a stale value, or one without its params, refuses it.

The SDK's transition builders generate the property from the document's params, replacing any value the document holds and leaving it out when a param is absent, so a transition built from a document carries the value the platform would generate, and contest detection sees it. A document fetched, edited and sent back through them therefore carries the value of its new params, not the stale one. The property-constraint pre-checks of the JavaScript and FFI SDKs judge the document the same way. A client that validates a document it built, before building a transition, should generate it first (in Rust, `DocumentTypeBasicMethods::regenerate_generated_properties`) or set the value; otherwise the local check reports the property missing. The proof a client verifies after a create or replace is checked against the document with the generated value, as the platform stored it.

## Rules at registration

- The keyword is allowed only on a string property, and not beside `$ref`, whose definition replaces every keyword written next to it (declare it in the definition instead). On any other property, a typed array and its `items` included, the meta-schema refuses it (`JsonSchemaError`, 10101).
- `function` must name a system function, and `params` must list 1 to 16 params, as many as the function takes. The meta-schema refuses an unknown function or an empty or overlong list (`JsonSchemaError`, 10101).
- Every param must name another string property of the same document type (not an object, not a system property, not the declaring property), and that property may not be generated itself.
- Neither the declaring property nor a param may be [transient](transient.md) or sit inside a transient object: a transient value is never stored.
- Every param must sit inside every object that holds the declaring property: a top-level property may take any param, but `profile.normalized` must take params inside `profile`. A document that supplies the params then always holds the object the platform writes the value into.
- On a contract update, a new property may declare `generatedFrom` only when one of its params is new too. Documents stored before the update were never generated, so a new generated property whose params all existed is refused (`DocumentTypeUpdateError`, 40212).

The meta-schema refuses the shape errors of the first two rules with `JsonSchemaError` (10101). The parser refuses a function with the wrong number of params, and a declaration that breaks the param rules (the third to fifth), with `InvalidContractStructure` (10231). The update rule refuses with `DocumentTypeUpdateError` (40212).

## See also

- [Generated Properties](../data-model/documents.md#generated-properties-generatedfrom), the deep dive
- [Property Schemas](property-schemas.md), for `pattern` and `required`
- [Indexes (indices)](indexes.md), for the unique index that usually reads the generated property
- [Contract Keywords overview](../contract-keywords.md), for the conventions of these tables
