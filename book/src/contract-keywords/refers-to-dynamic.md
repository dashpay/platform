# A Target the Document Names

A document reference normally names its target in the schema: a `documentType`, and a `contractId` when the type is in another contract. A reference whose `contractId` and `documentType` are written `"$self.<property>"` leaves both to the document: two of its own properties say which contract and which document type the value points into. `$self` is the document being written, as [`$old`](property-constraints.md) is the stored one in a rule. `documentTypeRequirements` then says what that document type must look like. Reach for it when a contract should accept a document of any contract that qualifies: a proposal described by a moderated text someone else hosts, a listing attached to a record of whichever registry issued it.

| | |
|---|---|
| **Where** | `refersTo` on an identifier property, at the top level or inside an object. Not on the `items` of a typed array, not an operand of an [expression](refers-to-expressions.md), not [`ownerRefersTo` or `creatorRefersTo`](owner-refers-to.md). |
| **Value** | `"contractId": "$self.<property>"` and `"documentType": "$self.<property>"`, both required, and optionally `documentTypeRequirements`, in a `"type": "permanentDocument"` or `"type": "moderatedDocument"` declaration that carries nothing else. |
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
        "contractId": "$self.descriptionContractId",
        "documentType": "$self.descriptionDocumentType",
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
| `contractId` | `"$self."` followed by the name of a top-level identifier property of the document type, 1 to 64 letters, digits or underscores | The property holding the id of the contract the referenced document type lives in, where a reference naming its target in the schema writes the id itself. |
| `documentType` | `"$self."` followed by the name of a top-level string property | The property holding the name of the referenced document type, where a reference naming its target in the schema writes the name itself. |
| `documentTypeRequirements` | object: `properties` (required), `require` | What the named document type must look like. Absent, the type is held to its kind alone. |

The two come together: a declaration names its target in the schema or through the document, never half of each, so a `$self.` `contractId` beside a literal `documentType` is refused, as is one beside none. The declaring contract is named like any other: a document pointing into its own contract writes that contract's id. Nothing a schema could write literally starts with `$self.`: a contract id is base58 and a document type name holds no `$`.

## documentTypeRequirements

`properties` holds 1 to 4 named **slots**. Each describes a top-level property the referenced document type must have, and is **filled** by one property of that type matching the description.

| Slot key | Value | Meaning |
|---|---|---|
| `name` | `"$any"`, a property name, or `"$self.<a string property of the referring document>"` | Which property may fill the slot: any property matching the rest; the one called this; or the one the referring document names in that property of its own. |
| `type` | `"string"`, `"identifier"`, `"integer"`, `"boolean"`, or `"array"` with `"byteArray": true` | What the property holds. A byte array slot is not filled by an identifier, nor an identifier slot by a byte array. |
| `maxBytes` | integer, 1 to 4294967295 | On a string or byte array slot: the property can never hold more bytes than this. A string's bound is its `maxBytes` or four times its `maxLength`, the smaller of the two when it declares both; a byte array's is its `maxItems`. A property declaring no bound meets none. |
| `distinctFrom` | a slot name, or a list of slot names | This slot is filled by a different property than each of them. Saying it on one side is enough. |

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
    "body":      { "name": "$self.bodyProperty", "type": "string", "distinctFrom": "title" },
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

## How it works

Nothing about the target is known when the contract is registered, so registration checks the declaration alone. Everything about the target is checked when a document is written, in this order, the first failure refusing the write:

1. The document names a contract and a document type. A document that holds the reference but leaves out the property its `contractId` or its `documentType` is read from points nowhere: `ReferencedEntityNotFoundError` (40120).
2. The contract exists and has a document type of that name: `ReferencedDocumentTypeNotFoundError` (40121) otherwise. Another contract costs a billed fetch, once per write.
3. The type is not [private](../contract-keywords.md#document-type), `ReferencedDocumentTypePrivateError` (40150), and not `indexOnly`, `ReferencedDocumentTypeIndexOnlyError` (40146): the documents of such a type are index entries, and none can be found by its id.
4. The type meets `documentTypeRequirements`: `ReferencedDocumentTypeRequirementNotMetError` (40153).
5. The type is of the declared kind, as a reference naming it in the schema is held to at registration: for `permanentDocument` its documents never disappear (40122 or 40144 otherwise), for `moderatedDocument` they disappear only through a moderator's recorded removal (40143).
6. The document exists: `ReferencedEntityNotFoundError` (40120).

**On a replace** the reference is checked again, whole, when its value changed or when a property it reads its target from changed: the contract id, the document type name, or a property a slot takes its name from. A changed one points the same id somewhere else. Otherwise it is kept as a permanent or moderated reference is: a moderated target a moderator removed since resolves to its removal record, and an edit of the referring document's other properties passes.

**What stays true.** A document type's deletion and moderation settings never change, and neither do its existing properties' types, so a target that qualified keeps its kind. The requirements are judged when the reference is checked, not afterwards.

## Rules at registration

- Both `contractId` and `documentType` are present and read from the document (`"$self.<property>"`), from two different properties, and `type` is `permanentDocument` or `moderatedDocument`. A `deletableDocument` reference always names its type in the schema: a dynamic target is trusted for what its type promises about its documents staying, and a deletable type promises nothing.
- The declaration carries nothing else: no `findBy`, `where`, `inList`, `consume` or `constraints`. Each needs the referenced type when the contract is registered. `documentTypeRequirements` comes only with a target read from the document: a type the schema names needs no description.
- It sits on a single identifier property: not on the elements of a typed array, in a reference expression, or in `ownerRefersTo` and `creatorRefersTo`.
- `contractId` reads a top-level identifier property, and `documentType` and every slot's `"$self.<property>"` name a top-level string property. None is transient or the reference property itself, and each is required wherever the reference property is, so a document that must hold the reference always says where it points. None is listed under [`moderatorAbilities.changeFields`](moderator-abilities.md), like the reference property itself: a moderator's change checks no reference again, so it could point a stored one somewhere nobody checked.
- `documentTypeRequirements` holds 1 to 4 slots (`max_document_type_requirement_slots`), each well formed; `distinctFrom` names other slots of the same `properties`; `require` names slots, and all of them.
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
