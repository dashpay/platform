# Typed Arrays

A typed array is a list property: a document holds any number of values, up to a limit, all of one simple type. A post's tags, the scores of a match or the members of a team are typed arrays. The `items` keyword makes an array a typed array and gives the schema every element follows. Before protocol version 14 an array property had to be a byte array.

| | |
|---|---|
| **Where** | A property of type `array`, at the top of a document type or inside an object, in place of `byteArray: true` |
| **Value** | The schema of one element: an integer, a number, a string, a boolean, a byte array or an identifier |
| **Since** | protocol version 14 |
| **On update** | `items` may be neither added nor removed (`IncompatibleDocumentTypeSchemaError`, 10246). The keywords inside it follow their own update rules, and a change to how an element is stored is refused (`DocumentTypeUpdateError`, 40212). |
| **Errors** | `JsonSchemaError` (10101); on elements, `DocumentPropertyMaxBytesExceededError` (10421), `DocumentPropertyNotDistinctError` (10419) and the reference errors of [refersTo](refers-to.md); for an index on the elements, `InvalidIndexedPropertyConstraintError` (10205) and `InvalidContractStructure` (10231) |

## Example

```json
"tags": {
  "type": "array",
  "minItems": 0,
  "maxItems": 10,
  "uniqueItems": true,
  "items": { "type": "string", "minLength": 1, "maxLength": 32, "maxBytes": 64 },
  "position": 3
}
```

A post holds up to 10 tags, none repeated, each 1 to 32 characters and at most 64 bytes.

The elements may also be identifiers that point at other documents. The moderation charters contract lists the reasons a moderation team may act on like this:

```json
"reasons": {
  "type": "array",
  "minItems": 0,
  "maxItems": 64,
  "uniqueItems": true,
  "items": {
    "type": "array",
    "byteArray": true,
    "minItems": 32,
    "maxItems": 32,
    "contentMediaType": "application/x.dash.dpp.identifier",
    "refersTo": { "type": "permanentDocument", "documentType": "reason" }
  },
  "position": 2
}
```

A charter names up to 64 distinct `reason` documents, and each one must exist when the charter is written.

With the [identifier shorthand](property-schemas.md#shorthands-identifier-and-bytes) the same `items` read `{ "type": "identifier", "refersTo": { "type": "permanentDocument", "documentType": "reason" } }`; `{ "type": "bytes", "size": 20 }` makes every element 20 bytes.

## How it works

**On the array**, `minItems` and `maxItems` count elements, and `uniqueItems: true` refuses a document that repeats one. A list that is too long or too short, repeats an element, or holds an element of the wrong type is refused with `JsonSchemaError` (10101).

**On the elements**, the `items` schema is checked for every element, as a property of that schema would be:

- the JSON Schema keywords: an element's `enum`, bounds, length and `pattern` (`JsonSchemaError`, 10101);
- [`maxBytes`](max-bytes.md) on string elements (`DocumentPropertyMaxBytesExceededError`, 10421), the error naming the element, such as `tags[2]`;
- [`distinctFrom`](distinct-from.md) on identifier elements: every element must differ from the named property (`DocumentPropertyNotDistinctError`, 10419);
- [`refersTo`](refers-to.md) on identifier elements: each element is checked as a single reference would be, in list order, and the first one that fails refuses the write with that reference's error, naming the element (`reasons[2]` for the third). An empty or absent list checks nothing.

A replace re-checks the references of the elements the stored list did not hold, and every element when the reference's rules call for it. See [References on the Elements](../data-model/documents.md#references-on-the-elements) for the details.

**Storage.** A typed array is stored inline in the document: a count of its elements, then each element encoded as a required property of the element's type would be. An identifier element takes 32 bytes, an integer element the width its bounds give it (see [Numbers](property-schemas.md#numbers)), a fixed-size byte array element its bytes, and a string element a length prefix and its bytes. Elements never carry a presence byte. The `reasons` list above is therefore one count byte and 32 bytes per reason. The 5120-byte limit on a single value applies to each element, not to the list.

**When a document is read from JSON**, identifier and byte array elements are converted from their string form element by element, as a single identifier or byte array is.

## Indexing the elements

An index may name a typed array, from protocol version 14. The document is then entered once under each of its distinct elements, so "the posts whose `hashtags` hold `dash`" or "the posts that mention this identity" is an index lookup, with a proof:

```json
"hashtags": {
  "type": "array",
  "maxItems": 10,
  "items": { "type": "string", "minLength": 1, "maxLength": 61 },
  "position": 0
},
"mentions": {
  "type": "array",
  "maxItems": 10,
  "items": { "type": "identifier" },
  "position": 1
}
```

```json
"indices": [
  { "name": "byHashtag", "properties": [{ "hashtags": "asc" }, { "$createdAt": "asc" }] },
  { "name": "byMention", "properties": [{ "mentions": "asc" }, { "$createdAt": "asc" }] },
  { "name": "trending", "properties": [{ "hashtags": "asc" }],
    "countable": "countable", "rangeCountable": true, "rankedCountable": true }
]
```

`byHashtag` lists the posts carrying one hashtag, newest first; `byMention` the posts naming one identity; `trending` counts the posts per hashtag and ranks the hashtags by that count.

- **One entry per distinct element.** Each element is keyed exactly as a property of the element's type would be, so elements sort like values of that type. A list that repeats an element is entered under it once; `uniqueItems` is not needed, except by an index of an [index-only type](index-only.md#typed-arrays).
- **An empty or absent list is entered nowhere.** The array is always in the index's skip set (see [`skipIfAbsent`](indexes.md#skipifabsent)), declared or not, and an index naming it is read only with the elements bound.
- **Any position, one array.** The array may be the index's first property or come after others (`[$ownerId, hashtags]`), but an index names at most one typed array.
- **Updates move only what changed.** A replace that keeps an element keeps its entry; an element added or dropped adds or removes one entry; a change to another property of the index moves every element's entry in that index.

Queries read the elements with `contains` and `containsAny`; see [Indexes on typed arrays](indexes.md#typed-arrays) for the operators, the limits and the rules.

## Rules at registration

The array:

- declares `items` and `maxItems`, and not `byteArray`;
- has a `maxItems` of at most 1024, and a `minItems` no higher than its `maxItems`;
- carries no `contentMediaType`, `refersTo`, `distinctFrom` or `maxBytes` of its own: these go on the `items`.

The element schema (`items`):

- is written inline: a `$ref` is refused;
- has a `type` of `integer`, `number`, `string`, `boolean` or `array`, and an `array` element must be a byte array (`byteArray: true`), or one of the [shorthands](property-schemas.md#shorthands-identifier-and-bytes) `identifier` and `bytes` (with `size`). Objects and lists of lists are refused;
- takes these keywords: `type`, `minimum`, `maximum`, `exclusiveMinimum`, `exclusiveMaximum`, `multipleOf`, `minLength`, `maxLength`, `pattern`, `format`, `minItems`, `maxItems` (bytes, on a byte array element), `size` (on a `bytes` element), `enum`, `byteArray`, `contentMediaType`, `maxBytes`, `distinctFrom`, `refersTo`, `$comment` and `description`. Anything else, `position`, `const`, `uniqueItems` and `examples` included, is refused. A one-value `enum` does what `const` would, and an update can still widen it;
- may carry an `enum` only on a string, integer, number or boolean element, and every member must be a value of the element's type;
- on an integer element, has an integer `minimum` and `maximum`, the minimum no higher than the maximum;
- may carry `refersTo` only on an identifier element, and not with the `identityPublicKey` target: its key id is a single sibling property, which cannot pair with many elements.

Where a typed array cannot be used:

- as an index-only type's terminal or entry payload: a terminal keys one entry and an entry payload is one value (10231, or `InvalidIndexPropertyTypeError` 10206 in a terminal). An index of an index-only type may name one, under the rules of [Index-Only Types](index-only.md#typed-arrays);
- inside an object, when an index names it: an indexed typed array is a top-level property (10231);
- in a `where` entry of a reference, other than as a list: two typed arrays of one element kind may be compared as lists (the same elements in the same order), and a referenced typed array may be asked to `contains` a value or `containsAll` of another typed array's elements; a float element is refused, and a typed array is equal to no single value (see [Typed arrays](refers-to.md#typed-arrays) in References). An equality between two lists can key a `preallocated` or `summableOffCountIndex` index of an index-only type by the elements of the list (see [Counting per element](index-only.md#counting-per-element));
- as an operand of a [propertyConstraints](property-constraints.md) rule, other than in a `present` or `absent` test;
- with [`encryptedFor`](encrypted-for.md), which only a byte array takes.

References count against the limit of 256 per document at `maxItems` each: a list of up to 64 references counts as 64. An `immutable` property may not hold a list of `deletableDocument` references (see [Mutability](mutability.md)).

A meta-schema violation is refused with `JsonSchemaError` (10101) and the other rules with `InvalidContractStructure` (10231).

## On update

- `items` may not be added or removed, so a byte array cannot become a typed array or the reverse (10246).
- Inside `items`, each keyword follows its own update rule (see [Property Schemas](property-schemas.md)): for example `maxLength` and `maxBytes` may be raised, and `enum` may gain values. `refersTo` and `distinctFrom` are fixed.
- A change to how an element is stored is refused (`DocumentTypeUpdateError`, 40212): an integer element whose width or sign would change (by its bounds or its `enum`), or a byte array element that would switch between fixed and variable size or change its fixed size.
- On the array, `maxItems` may be raised, up to 1024 and within the reference limit, and `minItems` lowered. `uniqueItems` may be removed, not added, and not removed from an array an index of an index-only type names. An indexed array's `maxItems` stays within the index limits (16, or 4 under a window, and 256 entries per document), and on a type with a [`ttl`](ttl.md), whose documents prepaid their cleanup by it, is not raised at all (`DocumentTypeUpdateError`, 40212).
- The element bounds of an indexed array stay within the index key limits (a string's `maxLength` at most 63, a byte array element's `minItems` at least 1).

## See also

- [Typed Arrays](../data-model/documents.md#typed-arrays) and [References on the Elements](../data-model/documents.md#references-on-the-elements) in the Documents chapter
- [Property Schemas](property-schemas.md), for the keywords an element takes
- [References (refersTo)](refers-to.md), [distinctFrom](distinct-from.md) and [maxBytes](max-bytes.md), which apply to every element
- [Document Serialization](../serialization/document-serialization.md#value-encoding-by-type), for the stored form
