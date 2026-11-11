# A Target the Document Names

A document reference normally names its target in the schema: a `documentType`, and a `contractId` when the type is in another contract. A reference with `contractIdProperty` and `documentTypeProperty` leaves both to the document: two of its own properties say which contract and which document type the value points into. `documentTypeRequirements` then says what that document type must look like. Reach for it when a contract should accept a document of any contract that qualifies: a proposal described by a moderated text someone else hosts, a listing attached to a record of whichever registry issued it.

| | |
|---|---|
| **Where** | `refersTo` on an identifier property, at the top level or inside an object. Not on the `items` of a typed array, not an operand of an [expression](refers-to-expressions.md), not [`ownerRefersTo` or `creatorRefersTo`](owner-refers-to.md). |
| **Value** | `contractIdProperty` and `documentTypeProperty`, both required, and optionally `documentTypeRequirements`, in a `"type": "permanentDocument"` or `"type": "moderatedDocument"` declaration, or one [listing both](#either-kind), that carries nothing else. |
| **Since** | protocol version 15 |
| **On update** | Fixed, like the rest of `refersTo` (`IncompatibleDocumentTypeSchemaError`, 10246). |
| **Errors** | When a document is written: `ReferencedDocumentTypeNotFoundError` (40121), `ReferencedDocumentTypePrivateError` (40150), `ReferencedDocumentTypeIndexOnlyError` (40146), `ReferencedDocumentTypeRequirementNotMetError` (40153), `ReferencedDocumentTypeDeletableError` (40122), `ReferencedDocumentTypeNotModeratedError` (40143), `ReferencedDocumentTypeModeratedError` (40144), `ReferencedEntityNotFoundError` (40120). At registration: `InvalidContractStructure` (10231), `JsonSchemaError` (10101). |

## Example

```json
"note": {
  "type": "object",
  "properties": {
    "descriptionId": {
      "type": "identifier",
      "refersTo": {
        "type": "moderatedDocument",
        "contractIdProperty": "descriptionContractId",
        "documentTypeProperty": "descriptionDocumentType",
        "documentTypeRequirements": {
          "properties": {
            "body":      { "name": "$any", "type": "string" },
            "bodyBytes": { "name": "$any", "type": "array", "byteArray": true }
          },
          "require": { "anyOf": ["body", "bodyBytes"] }
        }
      },
      "position": 0
    },
    "descriptionContractId": { "type": "identifier", "position": 1 },
    "descriptionDocumentType": { "type": "string", "minLength": 1, "maxLength": 64, "position": 2 }
  },
  "required": ["descriptionId", "descriptionContractId", "descriptionDocumentType"],
  "additionalProperties": false
}
```

A note is described by a document its author points at: `descriptionContractId` names a contract, `descriptionDocumentType` a document type of it, and `descriptionId` a document of that type. The type must be one whose documents only the contract's moderators remove, on the record, and it must hold text, as a string or as bytes. Nothing about that contract is in this schema.

## The keys

| Key | Value | What it does |
|---|---|---|
| `contractIdProperty` | the name of a top-level identifier property of the document type, 1 to 64 letters, digits or underscores | The property holding the id of the contract the referenced document type lives in. Takes the place of `contractId`. |
| `documentTypeProperty` | the name of a top-level string property | The property holding the name of the referenced document type. Takes the place of `documentType`. |
| `documentTypeRequirements` | object: `properties` (required), `require` | What the named document type must look like. Absent, the type is held to its kind alone. |

The two properties come together: a declaration names its target in the schema or through the document, never half of each. The declaring contract is named like any other: a document pointing into its own contract writes that contract's id.

## documentTypeRequirements

`properties` holds 1 to 4 named **slots**. Each describes a top-level property the referenced document type must have, and is **filled** by one property of that type matching the description.

| Slot key | Value | Meaning |
|---|---|---|
| `name` | `"$any"`, a property name, or `{ "property": "<a string property of the referring document>" }` | Which property may fill the slot: any property matching the rest; the one called this; or the one the referring document names in that property of its own. |
| `type` | `"string"`, `"identifier"`, `"integer"`, `"boolean"`, or `"array"` with `"byteArray": true` | What the property holds. A byte array slot is not filled by an identifier, nor an identifier slot by a byte array. |
| `maxBytes` | integer, 1 to 4294967295 | On a string or byte array slot: the property can never hold more bytes than this. A string's bound is its `maxBytes`, or four times its `maxLength`; a byte array's is its `maxItems`. A property declaring no bound meets none. |
| `distinctFrom` | a slot name, or a list of slot names | This slot is filled by a different property than each of them. Saying it on one side is enough. |
| `when` | a condition | What the referenced type must say of the property: one the condition does not hold for does not fill the slot. See [A condition on the property](#a-condition-on-the-property-when). |

A slot name is 1 to 64 letters, digits or underscores. A transient property fills nothing: it is never stored.

`require` says which slots must be filled:

- absent, every slot;
- a slot name, that slot;
- `{ "anyOf": [...] }` or `{ "allOf": [...] }` over two or more terms, each a slot name or the other combinator over two or more slot names. Two levels say everything `anyOf` and `allOf` can say of a set of slots. Nothing is named twice: no term, no slot within a group, no group with its slots in another order.

Every slot is named by `require` when there is one: a slot it never names could not matter.

```json
"documentTypeRequirements": {
  "properties": {
    "title":     { "name": "$any", "type": "string", "maxBytes": 512 },
    "body":      { "name": { "property": "bodyProperty" }, "type": "string", "distinctFrom": "title" },
    "bodyBytes": { "name": "$any", "type": "array", "byteArray": true }
  },
  "require": { "allOf": ["title", { "anyOf": ["body", "bodyBytes"] }] }
}
```

A short text property, and either the string property the document names in its `bodyProperty`, other than the one taken as the title, or a byte array.

**When the requirements are met.** They are met when some choice of properties fills a set of slots `require` holds for, with slots that are `distinctFrom` each other filled by different properties. Consensus only asks whether such a choice exists. A client that needs to know which property is the title and which the body can settle it the same way every time, in two steps:

1. **Which slots.** Not every slot need be filled: `require` may hold for several sets of slots (every slot is the only set when there is no `require`). Take the sets it holds for with the most slots first and, between sets of the same size, the one whose slot names, listed in order, come first. Use the first set that step 2 can fill.
2. **Which property for each.** Take the set's slots in the order of their names and give each the first property, in the type's `position` order, that matches it and that no slot it must differ from has taken, backing up to the previous slot's next property when a slot is left with nothing.

Take the first example of this page, `body` or `bodyBytes`: a type holding a string and a byte array fills both slots. A type holding only a string cannot fill that set, so the next ones are tried: `body` alone, which the string fills (`bodyBytes` alone would come after it). Naming the property outright, or through the referring document, leaves nothing to settle.

## Either kind

`type` may list both kinds:

```json
"refersTo": {
  "type": ["permanentDocument", "moderatedDocument"],
  "contractIdProperty": "descriptionContractId",
  "documentTypeProperty": "descriptionDocumentType"
}
```

The named type is then a permanent one or a moderated one, and the reference is checked as a reference of the kind the type is: into a moderated type it resolves to the document or to its removal record, as a `moderatedDocument` reference does, and into a permanent type the document is always there. A type whose documents their owner can delete is neither, and is refused as it is for a `permanentDocument` reference (40122). The list holds the two names, once each, in either order. Only a reference naming its target through the document lists them: one naming its type in the schema knows the type's kind.

## A condition on the property (`when`)

A slot's `name`, `type` and `maxBytes` describe the property. `when` adds a condition on what the referenced type says about it, written like a [`propertyConstraints`](property-constraints.md) condition and read against the referenced type's schema instead of a document:

```json
"documentTypeRequirements": {
  "properties": {
    "body": {
      "name": "$any",
      "type": "string",
      "when": {
        "ifThen": [
          { "equal": ["$kind", { "const": "moderatedDocument" }] },
          { "allOf": [
            { "not": { "contains": ["$moderatorAbilities.deleteKeepsFields", "$self"] } },
            { "not": { "contains": ["$moderatorAbilities.changeFields", "$self"] } }
          ] }
        ]
      }
    }
  }
}
```

When the type is a moderated one, the string filling `body` is one a moderator's removal takes away (the removal record does not keep it) and one the document's author writes (it is not the moderators' field). A permanent type is asked nothing. With `"type": ["permanentDocument", "moderatedDocument"]` this accepts a text that is never removed, or a text moderators can take down, and not a text that would stay public after they did.

| Operand | Reads |
|---|---|
| `"$kind"` | The kind of the referenced type, by the `refersTo` `type` it is referred to with: `permanentDocument` or `moderatedDocument`. |
| `"$self"` | The name of the property tried for the slot. |
| `"$moderatorAbilities.deleteKeepsFields"` | The referenced type's [`deleteKeepsFields`](moderator-abilities.md): the property paths a removal record keeps. Empty when the type lists none. |
| `"$moderatorAbilities.changeFields"` | The referenced type's [`changeFields`](moderator-abilities.md): the properties only moderators write. Empty when the type lists none. |
| `{ "const": "<string>" }` | The string, 1 to 256 characters. |

| Condition | Holds when |
|---|---|
| `{ "equal": [a, b] }`, `{ "notEqual": [a, b] }` | `$kind` or `$self` is, or is not, the constant on the other side. Compared with `$kind`, the constant is `permanentDocument` or `moderatedDocument`. |
| `{ "in": [a, [...]] }`, `{ "notIn": [a, [...]] }` | `$kind` or `$self` is, or is not, one of two or more distinct strings. |
| `{ "contains": [list, a] }` | One of the two lists holds `$self` or a constant, as a whole entry. |
| `{ "not": c }`, `{ "anyOf": [...] }`, `{ "allOf": [...] }` | As in `propertyConstraints`: `anyOf` and `allOf` take two or more conditions, none listed twice and none the same combinator; `not` holds no `not` and no `notIn`. |
| `{ "ifThen": [c, then] }`, `{ "ifThenElse": [c, then, else] }` | `then` holds whenever `c` does; with an `else`, it holds when `c` does not. |

One `when` is made of at most 16 conditions (`max_document_type_requirement_condition_nodes`), combinators counted. It reads the type, never a document: no property path, no arithmetic, and no value of the referenced document. `distinctFrom`, `require` and the search for an assignment work as before, over the properties each slot's `when` leaves it.

## How it works

Nothing about the target is known when the contract is registered, so registration checks the declaration alone. Everything about the target is checked when a document is written, in this order, the first failure refusing the write:

1. The document names a contract and a document type. A document that holds the reference but leaves out `contractIdProperty`'s or `documentTypeProperty`'s property points nowhere: `ReferencedEntityNotFoundError` (40120).
2. The contract exists and has a document type of that name: `ReferencedDocumentTypeNotFoundError` (40121) otherwise. Another contract costs a billed fetch, once per write.
3. The type is not [private](../contract-keywords.md#document-type), `ReferencedDocumentTypePrivateError` (40150), and not `indexOnly`, `ReferencedDocumentTypeIndexOnlyError` (40146): the documents of such a type are index entries, and none can be found by its id.
4. The type meets `documentTypeRequirements`, each slot filled by a property its `when` holds for: `ReferencedDocumentTypeRequirementNotMetError` (40153).
5. The type is of the declared kind, as a reference naming it in the schema is held to at registration: for `permanentDocument` its documents never disappear (40122 or 40144 otherwise), for `moderatedDocument` they disappear only through a moderator's recorded removal (40143). A declaration listing both kinds is a moderated reference into a moderated type and a permanent one into any other, so it refuses only a type whose documents can be deleted (40122).
6. The document exists: `ReferencedEntityNotFoundError` (40120).

**On a replace** the reference is checked again, whole, when its value changed or when a property it reads its target from changed: the contract id, the document type name, or a property a slot takes its name from. A changed one points the same id somewhere else. Otherwise it is kept as a permanent or moderated reference is: a moderated target a moderator removed since resolves to its removal record, and an edit of the referring document's other properties passes.

**What stays true.** A document type's deletion and moderation settings never change, its `moderatorAbilities` among them, and neither do its existing properties' types, so a target that qualified keeps its kind and what a `when` read of it. The requirements are judged when the reference is checked, not afterwards.

## Rules at registration

- Both `contractIdProperty` and `documentTypeProperty` are present, name two different properties, and `type` is `permanentDocument`, `moderatedDocument` or the list of both. A `deletableDocument` reference always names its type in the schema: a dynamic target is trusted for what its type promises about its documents staying, and a deletable type promises nothing.
- The declaration carries nothing else: no `contractId`, `documentType`, `findBy`, `where`, `inList`, `consume` or `constraints`. Each needs the referenced type when the contract is registered.
- It sits on a single identifier property: not on the elements of a typed array, in a reference expression, or in `ownerRefersTo` and `creatorRefersTo`.
- `contractIdProperty` names a top-level identifier property, and `documentTypeProperty` and every slot's `{ "property": ... }` a top-level string property. None is transient or the reference property itself, and each is required wherever the reference property is, so a document that must hold the reference always says where it points. None is listed under [`moderatorAbilities.changeFields`](moderator-abilities.md), like the reference property itself: a moderator's change checks no reference again, so it could point a stored one somewhere nobody checked.
- `documentTypeRequirements` holds 1 to 4 slots (`max_document_type_requirement_slots`), each well formed; `distinctFrom` names other slots of the same `properties`; `require` names slots, and all of them.
- A slot's `when` is a condition of at most 16 conditions (`max_document_type_requirement_condition_nodes`) over the operands above, each comparison pairing what it may pair.
- No [`propertyConstraints` rule](property-constraints.md#values-of-a-referenced-document) reads a value through the reference, no [index](derived-index-properties.md) holds one, and no chained query joins through it: each resolves the referenced type from the schema.

Before protocol version 15 the keys are not part of `refersTo`: registration refuses them.

## Who vouches for the target

The requirements say what the target's type looks like. They do not say who runs its contract. A `moderatedDocument` target is removed by the moderators of the contract the document names, and the author of the document chose that contract. Where that matters, hold the contract to something as well: give the contract id property its own reference,

```json
"descriptionContractId": {
  "type": "identifier",
  "refersTo": { "type": "contract", "contractRequirements": { "moderation": "elected" } },
  "position": 1
}
```

or compare it with the contracts the application trusts in a [`propertyConstraints`](property-constraints.md) rule (`{ "in": ["descriptionContractId", ["<base58>", "<base58>"]] }`).

## Errors

| Error | Code | When |
|---|---|---|
| `ReferencedDocumentTypeRequirementNotMetError` | 40153 | Write: the named document type does not meet the reference's `documentTypeRequirements`. |
| `ReferencedDocumentTypeNotFoundError` | 40121 | Write: the named contract does not exist, or has no document type of that name. |
| `ReferencedDocumentTypePrivateError` | 40150 | Write: the named document type is private. |
| `ReferencedDocumentTypeIndexOnlyError` | 40146 | Write: the named document type is `indexOnly`. |
| `ReferencedDocumentTypeDeletableError`, `ReferencedDocumentTypeModeratedError`, `ReferencedDocumentTypeNotModeratedError` | 40122, 40144, 40143 | Write: the named document type is not of the declared kind. |
| `ReferencedEntityNotFoundError` | 40120 | Write: the document does not say where the reference points, or no document with that id exists in the named type. |

## See also

- [References (refersTo)](refers-to.md) for the targets and the replace rules.
- [Contract Moderation](../data-model/contract-moderation.md) for what makes a document type a moderated one.
- [distinctFrom](distinct-from.md), the document keyword the slot key is named after.
