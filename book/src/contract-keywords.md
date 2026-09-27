# Contract Keywords Reference

This page lists every keyword a data contract's document type schema accepts. For each one it gives what the keyword does, where it may go, the protocol version it arrived in, what a contract update may do with it and, where one applies, the error a document that breaks it is refused with. The chapters linked from each entry explain the mechanics.

The list follows the document meta-schema of protocol version 14, `packages/rs-dpp/schema/meta_schemas/document/v3/document-meta.json`. Every document type schema is validated against it when a contract is registered or updated, and the parser (`try_from_schema`) checks the rules a JSON schema cannot express. When this page and the meta-schema disagree, the meta-schema is right and this page is out of date.

## Where keywords go

A contract's `documentSchemas` maps each document type name to its schema. Keywords sit at three levels:

```json
"post": {
  "type": "object",
  "documentsMutable": true,
  "canBeDeleted": true,
  "indices": [
    { "name": "byOwner", "properties": [{ "$ownerId": "asc" }, { "$createdAt": "asc" }] }
  ],
  "properties": {
    "text": { "type": "string", "maxLength": 280, "maxBytes": 560, "position": 0 },
    "replyTo": {
      "type": "array", "byteArray": true, "minItems": 32, "maxItems": 32,
      "contentMediaType": "application/x.dash.dpp.identifier",
      "refersTo": { "type": "deletableDocument", "documentType": "post" },
      "position": 1
    }
  },
  "required": ["$createdAt", "text"],
  "additionalProperties": false
}
```

- **Document type keywords** sit at the top of the schema (`documentsMutable`, `canBeDeleted`, `indices`, `required`). They say what may happen to a document of the type and who may do it.
- **Index keywords** sit inside an entry of `indices` (`name`, `properties`).
- **Property keywords** sit inside a property's schema (`type`, `maxLength`, `maxBytes`, `position`, `refersTo`). Most are ordinary JSON Schema; the rest are Platform's own.

## How to read the tables

- **From** is the first protocol version at which the keyword can be used. The document meta-schema changed at three versions: v1 at protocol version 12, v2 at 13 and v3 at 14. From 12 the meta-schema refuses a key it does not know; before 12 an unknown document type key was ignored.
- **Update** is what a contract update may do with the keyword on a document type that already exists. *Fixed* means adding, removing and changing it are all refused. A document type the update adds may use any keyword, as a new contract may.
- Codes in parentheses are consensus error codes (see [Error Codes](error-handling/error-codes.md)). A contract update that breaks an update rule is refused with `IncompatibleDocumentTypeSchemaError` (10246) when the schema comparison catches it, or `DocumentTypeUpdateError` (40212) when the document type comparison does. The tables say which.

## Document type keywords

### Shape

| Keyword | Value | What it does | From | Update |
|---|---|---|---|---|
| `type` | `"object"` | Required. A document is always an object. | 1 | Fixed |
| `$schema` | the meta-schema URL | The platform adds it when it reads the contract; a contract need not write it. | 1 | Fixed |
| `properties` | object | Required. The document's properties, 1 to 100 of them, each named with 1 to 64 letters, digits or underscores (from 14; earlier versions also admitted `-`). The system properties below may be listed here too. See [Property keywords](#property-keywords). | 1 | Properties may be added, optional or (with `requiredSince`) required; none may be removed (10246) |
| `additionalProperties` | `false` | Required, and only `false`: a document holds only the properties its type declares. | 1 | Fixed |
| `required` | array of names | The properties every document must hold. Listing a system timestamp or block height (`$createdAt`, `$updatedAtBlockHeight`) makes the platform record it on every document of the type; one that is not listed is never recorded. See [System properties](#system-properties). | 1 | May gain only a property the update adds, annotated with `requiredSince`; loses nothing (`DataContractInvalidRequiredFieldsUpdateError`, 10276) |
| `transient` | array of names | Properties validated on the transition but never stored. From 14 a replace drops them as a create does, each entry must name a top-level property, and no index, lookup key or `encryptedFor` may read one. See [Transient Properties](data-model/documents.md#transient-properties). | 1 | Fixed (10246); order and repeats are no change |
| `$comment`, `description` | string | Notes for readers; consensus ignores them. | 1 | Free |
| `minProperties`, `maxProperties` | integer | JSON Schema bounds on how many properties a document holds. | 12 | Fixed |
| `dependentRequired` | object | JSON Schema: a property that requires others when present. | 12 | May lose entries, not gain them (10246) |
| `$defs` | object | Definitions local to the document type, which `$ref` may point at. Contract-wide definitions live in the contract's `schemaDefs`. | 1 | Definitions may be added, not removed (10246) |

### What may happen to a document

| Keyword | Value | What it does | From | Update |
|---|---|---|---|---|
| `documentsMutable` | boolean, default `true` | `false` makes documents unchangeable after creation: a replace is refused (`InvalidDocumentTransitionActionError`, 10404). | 1 | Fixed (40212) |
| `canBeDeleted` | boolean, default `true` | `false` stops a document's owner from deleting it (10404). Says nothing about moderators or a `ttl`. From 14 a type that keeps history may not allow deletion. | 1 | Fixed (40212), except that a type keeping history may turn it off |
| `immutable` | array of names | Top-level properties frozen at creation on an otherwise mutable type: a replace that changes, adds or removes one is refused (`DocumentImmutablePropertyChangedError`, 40128). Only with `documentsMutable: true`; no system, nested or transient names. See [Immutable Properties](data-model/documents.md#immutable-properties-on-mutable-document-types). | 14 | May gain entries, never lose one (40212) |
| `immutableAllowSetting` | array of names | The `immutable` properties a replace may still set once, while the stored document has no value for them. Every entry must also be in `immutable`. | 14 | May lose entries; may gain one only for a property made immutable in the same update (40212) |
| `transferable` | `0` never, `1` always | `1` lets an owner give a document to another identity with a transfer transition. A transfer of a type set to `0` is refused (10404). | 1 | Fixed (40212) |
| `tradeMode` | `0` none, `1` direct purchase | `1` lets an owner set a price and anyone buy the document at that price, with no approval. Refused price updates are 10404; a purchase of a document with no price is `DocumentNotForSaleError` (40108), one at the wrong price `DocumentIncorrectPurchasePriceError` (40109). | 1 | Fixed (40212) |
| `creationRestrictionMode` | `0` anyone, `1` contract owner only, `2` nobody | Who may create documents. `2` is for system contracts whose documents only the platform writes. A refused create is `DocumentCreationNotAllowedError` (10416). | 1 | Fixed (40212) |
| `ttl` | seconds, 3600 to 31536000 | The platform deletes each document this long after its `$createdAt`, whoever owns it. Storage is priced for the time the document lives and refunds nothing. Needs `$createdAt` in `required`; refused with `documentsKeepHistory`, `indexOnly` and a contested index. After expiry a document can only be deleted (`DocumentExpiredError`, 40140). See [Document Time To Live](data-model/document-ttl.md). | 14 | Fixed (40212) |
| `documentsKeepHistory` | boolean, default `false` | Drive keeps every revision of every document, not only the latest. | 1 | Fixed (40212) |
| `keepsTransferHistory` | boolean, default `false` | Records every transfer of a document of the type in the document history system contract. | 13 | Fixed (40212) |
| `keepsPurchaseHistory` | boolean, default `false` | Records every purchase in the document history system contract. | 13 | Fixed (40212) |
| `keepsPricingHistory` | boolean, default `false` | Records every price update in the document history system contract. | 13 | Fixed (40212) |

### Who may write

| Keyword | Value | What it does | From | Update |
|---|---|---|---|---|
| `signatureSecurityLevelRequirement` | `1` critical, `2` high, `3` medium | The weakest identity key security level that may sign a transition on documents of the type. Default `2` (high). A key that is too weak is refused (`InvalidSignaturePublicKeySecurityLevelError`, 20004). See [Security Level](sdk/identity-keys.md#security-level). | 1 | Fixed (40212) |
| `requiresIdentityEncryptionBoundedKey` | `0` unique, `1` multiple, `2` multiple with a pointer to the latest | Lets identities add encryption keys bound to this document type, and says how they are kept: one key that cannot be replaced, several, or several with a pointer to the latest. A key may only be bound to a type that declares it. See [Contract Bounds](sdk/identity-keys.md#contract-bounds). | 1 | Fixed (40212) |
| `requiresIdentityDecryptionBoundedKey` | same as above | The same for decryption keys. | 1 | Fixed (40212) |
| `ownerRefersTo` | a `refersTo` declaration | A reference whose value is the writer (`$ownerId`) instead of a property: for example, the writer must own a document a lookup finds, or be an element of a list. Only on types whose documents can be neither transferred nor traded. Checked on create, and on a replace that changes what it reads. See [On the writer or the creator](data-model/documents.md#on-the-writer-or-the-creator-ownerrefersto-creatorrefersto). | 14 | Fixed (10246) |
| `creatorRefersTo` | a `refersTo` declaration | The same for the document's creator (`$creatorId`), for types whose documents can be transferred or traded. A type declares at most one of the two. | 14 | Fixed (10246) |
| `canBeDeletedByModerators` | boolean | Lets the contract's moderators delete documents of the type with a moderation transition, leaving a removal record. Needs `moderation` in the contract config; refused on types that keep history, are `indexOnly` or restrict creation. Makes the type count as deletable for references. See [Deleting Documents](data-model/contract-moderation.md#deleting-documents). | 14 | Fixed (40212) |
| `canBeDeletedByModeratorsFor` | seconds, 1 to 4294967295 | Limits the moderators' deletion to this long after the document's last change (`$updatedAt`). Later deletions are refused (`DocumentModerationWindowElapsedError`, 41116). Needs `canBeDeletedByModerators: true` and `$updatedAt` in `required`. | 14 | Fixed (40212) |

### Rules over several properties

| Keyword | Value | What it does | From | Update |
|---|---|---|---|---|
| `propertyConstraints` | object of named rules | Rules every created or replaced document must meet, where JSON Schema bounds one property at a time: comparisons and arithmetic over integer properties and the sizes of strings and arrays, string and identifier comparisons, value sets, presence tests, combined with `anyOf`, `allOf` and `not`. At most 16 rules of at most 32 nodes each. A broken rule refuses the document (`DocumentPropertyConstraintViolatedError`, 10422). See [the operators](#propertyconstraints-operators) and [Property Constraints](data-model/documents.md#property-constraints-propertyconstraints). | 14 | Fixed (10246) |

### Costs

| Keyword | Value | What it does | From | Update |
|---|---|---|---|---|
| `tokenCost` | object keyed by action | A token payment for an action on a document: `create`, `replace`, `delete`, `transfer`, `update_price` or `purchase`. Each takes the keys below. A transition that leaves out the payment a required cost asks for is refused (`RequiredTokenPaymentInfoNotSetError`, 40115). See [Fee System Overview](fees/overview.md#gas-paid-by-the-contract-owner). | 9 | Fixed |
| `tokenCost.<action>.tokenPosition` | integer | Required. Which token of the contract (`contractId` absent) or of the named contract is charged. | 9 | |
| `tokenCost.<action>.amount` | integer, at least 1 | Required. How many tokens the action costs. | 9 | |
| `tokenCost.<action>.contractId` | identifier | The contract whose token is charged, when it is not this one. | 9 | |
| `tokenCost.<action>.effect` | `0` transfer to the contract owner (default), `1` burn | What happens to the tokens paid. Burning is only allowed on the contract's own token (10261). | 9 | |
| `tokenCost.<action>.gasFeesPaidBy` | `0` document owner (default), `1` contract owner, `2` prefer contract owner | Who the contract owner offers to have pay the gas of a token-paid action. A transition that insists on a payer the type does not offer is refused (`GasFeesPaidByNotAllowedError`, 40129). Acted on from 14. | 14 | |
| `tokenCost.<action>.optional` | boolean, default `false` | `true` lets a transition skip the token and pay the gas in credits instead. See [Optional token costs](fees/overview.md#optional-token-costs). | 14 | |
| `actionFees` | object keyed by action | A fixed fee in credits, on top of the gas, for `create`, `replace`, `delete`, `transfer`, `update_price` or `purchase`, split between the contract owner's pot and the moderators' pot. The transition must state the fee it agrees to (`DocumentActionFeeAgreementNotSetError` 40132, `DocumentActionFeeAgreementMismatchError` 40133, `DocumentActionFeeMultiplierNotToleratedError` 40134). See [Document action fees](fees/overview.md#document-action-fees). | 14 | Fixed (40212) |
| `actionFees.pricing` | `"feeMultiplier"` (default) or `"fixed"` | Whether the amounts scale with the epoch's fee multiplier or are charged as written. | 14 | |
| `actionFees.<action>.owner` | credits | Added to the contract owner's pot, which the owner claims. | 14 | |
| `actionFees.<action>.moderators` | credits | Added to the moderators' pot, shared by the moderation team. Needs `moderation` in the contract config (10902). | 14 | |

### Storage layout and aggregates

| Keyword | Value | What it does | From | Update |
|---|---|---|---|---|
| `indices` | array of 1 to 10 indexes | The indexes documents are queried by, and the uniqueness rules they enforce. See [Index keywords](#index-keywords). | 1 | Fixed from 14: no index added, removed or changed (10217); reordering is no change |
| `documentsCountable` | boolean | Keeps a count of the type's documents in the primary key tree, so the total is read in one step. See [Document Count Trees](drive/document-count-trees.md#primary-key-tree-flags). | 12 | Fixed (40212) |
| `rangeCountable` | boolean | A provable count tree on the primary key, for counts over id ranges. Implies `documentsCountable`. | 12 | Fixed (40212) |
| `documentsSummable` | property name | Keeps the sum of one required integer property over all the type's documents. See [Document Sum Trees](drive/document-sum-trees.md#primary-key-tree-flags). | 12 | Fixed (40212) |
| `rangeSummable` | boolean | A provable sum tree on the primary key. Needs `documentsSummable` or `documentsAverageable`. Rarely useful; the index flag of the same name is what most contracts want. | 12 | Fixed (40212) |
| `documentsAverageable` | property name | Shorthand for `documentsCountable: true` plus `documentsSummable` on the named property. | 12 | Fixed (40212) |
| `rangeAverageable` | boolean | Shorthand for `rangeCountable` plus `rangeSummable`. Needs `documentsAverageable`. | 12 | Fixed (40212) |
| `indexOnly` | boolean | Documents are never written to primary storage: the index entries are the rows. Needs every property required and indexed, `$ownerId` in an index, `documentsMutable: false`, no transfers, trading, history or transient properties. See [Index-Only Document Types](drive/index-only-document-types.md). | 14 | Fixed (40212) |
| `entryPayload` | array of 1 to 16 names | On an `indexOnly` type, the properties stored in each entry's value instead of in a key. | 14 | Fixed (40212) |

## Property keywords

### JSON Schema keywords

A property's schema is JSON Schema (draft 2020-12), limited to the keywords below. Every document is validated against it on create and replace, and a value it refuses is a `JsonSchemaError`. Unless a row says otherwise, a contract update that breaks a property's update rule is refused with `IncompatibleDocumentTypeSchemaError` (10246).

| Keyword | Where | What it does | From | Update |
|---|---|---|---|---|
| `type` | every property | `string`, `integer`, `number`, `boolean`, `object` or `array`. An array is a byte array (`byteArray: true`) or, from 14, a typed array (`items`). | 1 | Fixed |
| `minLength`, `maxLength` | strings | Length in characters. A string with `pattern` or `format` must declare `maxLength` of at most 50000. | 1 | May be loosened: `maxLength` raised, `minLength` lowered, either removed |
| `pattern` | strings | A regular expression the value must match, in the syntax of Rust's `regex` crate (`IncompatibleRe2PatternError`, 10202). | 1 | May be removed, not added or changed |
| `format` | strings | A JSON Schema format such as `date-time` or `uri`. | 1 | May be removed, not added or changed |
| `minimum`, `maximum`, `exclusiveMinimum`, `exclusiveMaximum`, `multipleOf` | numbers | Numeric bounds. On an integer they also decide how many bytes it is stored in, when the contract sets `sizedIntegerTypes`. | 1 | May be loosened or removed, unless that changes an integer's stored width or sign (40212); `multipleOf` is fixed |
| `enum` | any | The values allowed, at least one, none repeated. | 1 | Values may be added, not removed; the keyword may be removed, not added. A value that widens an integer's stored width is refused (40212) |
| `const` | any | The one value allowed. | 1 | May be removed, not added or changed |
| `minItems`, `maxItems` | arrays | On a byte array, the length in bytes; on a typed array, the number of elements (`maxItems` required there, at most 1024). | 1 | May be loosened; a byte array may not switch between fixed and variable length (40212) |
| `uniqueItems` | arrays | On a typed array, no element may repeat. On a plain byte array, no byte may repeat; refused on an identifier. | 1 | May be removed, not added |
| `contains` | arrays | JSON Schema `contains`. | 1 | Fixed |
| `properties`, `required`, `additionalProperties` | objects | A nested object's own properties (each needs a `position`), which of them are required, and `additionalProperties: false`, which is required. | 1 | Nested properties may be added, not removed; a nested `required` and `additionalProperties` are fixed |
| `minProperties`, `maxProperties`, `dependentRequired` | objects | JSON Schema bounds on a nested object. | 1 | `dependentRequired` may lose entries, not gain them; the others are fixed |
| `$ref` | any | Points at a definition in the contract's `schemaDefs` (`#/$defs/...`). Only local references starting with `#`. | 1 | Fixed |
| `$id`, `$comment`, `description`, `examples` | any | Annotations; consensus ignores them. | 1 | Free, except that `$id` may only be added |

### Platform keywords

| Keyword | Where | What it does | From | Update |
|---|---|---|---|---|
| `position` | every property | The property's place in the stored document, which is encoded by position, not by name. Top-level positions, and those inside each object, run 0, 1, 2 with no gap (`MissingPositionsInDocumentTypePropertiesError`, 10411). See [Document Serialization](serialization/document-serialization.md). | 1 | Fixed |
| `byteArray` | arrays | `true` makes the array a string of bytes, stored raw. | 1 | Fixed |
| `contentMediaType` | byte arrays | `"application/x.dash.dpp.identifier"` makes a 32-byte array an identifier: shown in base58, converted from strings, and the only kind of property `refersTo` and `distinctFrom` accept. It must come with `byteArray: true`, `minItems: 32` and `maxItems: 32`. | 1 | Fixed |
| `items` | arrays | Makes the array a typed array: a list whose elements are all one scalar type (integer, number, string, boolean, byte array or identifier), stored inline. Elements take `enum`, bounds, `maxBytes`, `distinctFrom` and `refersTo`, but no `position` or `const`. See [Typed Arrays](data-model/documents.md#typed-arrays). | 14 | `items` may be neither added nor removed; the keywords inside it follow the rules above, and a change to how elements are stored is refused (40212) |
| `requiredSince` | top-level required properties | The contract version from which a property is required. It lets an update add a new required property: documents written under an earlier version may leave it out. On an update it must equal the version the update creates; on a new contract it may only be 1 (`DataContractInvalidRequiredFieldsUpdateError`, 10276). See [Adding Required Fields](data-model/data-contracts.md#evolving-a-contract-adding-required-fields). | 14 | Only on a property the update adds; an existing annotation is fixed |
| `maxBytes` | strings, and string elements | The most bytes the value may take in UTF-8, 1 to 65535, where `maxLength` counts characters of up to four bytes each. A longer value is refused (`DocumentPropertyMaxBytesExceededError`, 10421). See [Byte Caps on Strings](data-model/documents.md#byte-caps-on-strings-maxbytes). | 14 | May be raised or removed; not added or lowered (10246) |
| `distinctFrom` | identifiers, and identifier elements | The value must differ from another identifier property of the document, or from `$ownerId`. An equal pair is refused (`DocumentPropertyNotDistinctError`, 10419), and so is a transfer or purchase to the identity a `$ownerId` rule names. See [Distinct Identifier Properties](data-model/documents.md#distinct-identifier-properties). | 14 | Fixed (10246) |
| `encryptedFor` | byte arrays that are not identifiers | Declares how the property's ciphertext was made: `recipient` (an identifier property or `$ownerId`), `recipientKey` and `senderKey` (key id properties) and `scheme` (`"ecdh-secp256k1-aes256-cbc"`). All four are required. Consensus checks only the length shape (`InvalidEncryptedPropertyShapeError`, 10420). See [Encrypted Properties](data-model/documents.md#encrypted-properties-encryptedfor). | 14 | Fixed (10246) |
| `refersTo` | identifiers, identifier elements, and key id integers | What the value points at, checked when the document is written: the target must exist, and may have to meet further requirements. See [refersTo](#refersto) below. | 14 | Fixed (10246) |

## `refersTo`

A reference is checked when a document is created or replaced. Nothing checks it again when the target changes later: a `permanentDocument` target can never be deleted, and a `deletableDocument` reference is checked again on the referring document's next replace. A document carries at most 256 references, counting one per property, one for `ownerRefersTo` or `creatorRefersTo`, and `maxItems` per typed array of references. See [Document References](data-model/documents.md#document-references-refersto).

### Targets

| `type` | The value is | Keys it takes |
|---|---|---|
| `identity` | the id of an existing identity | none |
| `contract` | the id of an existing data contract | `contractRequirements` |
| `token` | the id of an existing token | none |
| `permanentDocument` | the id of a document of a type that can never be deleted, or with `lookup` a part of a unique index key that finds one | `documentType`, `contractId`, `propertyAgreement`, `lookup` |
| `deletableDocument` | the same for a type that can be deleted. Re-checked on every replace, so a reference to a deleted document must be repointed or cleared. | `documentType`, `contractId`, `propertyAgreement`, `lookup` |
| `identityPublicKey` | an identity key that exists and is not disabled. On an identifier, the identity, with `keyIdProperty` naming the key id property; on an integer from 0 to 4294967295, the key id, with `identityProperty` naming whose key it is. | `keyIdProperty` or `identityProperty`, `keyRequirements` |
| `listElement` | one of the identifiers a list of another document holds | `documentType`, `contractId`, `propertyAgreement` (with one `$id` pair), `inList` |

Instead of a `type`, a declaration may hold only `anyOf` (at least one operand holds) or only `allOf` (every operand holds). An operand is an `identity`, a `permanentDocument`, a `listElement`, a `deletableDocument` with a `lookup`, or an expression of the other combinator. A list holds 2 to 4 operands and expressions nest at most 4 deep. See [Reference expressions](data-model/documents.md#reference-expressions-anyof-allof).

### Keys

| Key | For | What it does |
|---|---|---|
| `documentType` | document and list references | The referenced document type. For `permanentDocument` and `listElement` it must forbid deletion; for `deletableDocument` it must allow it. |
| `contractId` | document and list references | The contract holding `documentType`, as base58 or 32 bytes. Absent means this contract. |
| `propertyAgreement` | document and list references | Up to 10 pairs `{ "referring property": "referenced property" }` that must be equal when the document is written. The referring side may be `$ownerId`, which makes the pair a write gate; the referenced side may be `$ownerId`, `$creatorId` or `$id`. |
| `lookup` | `permanentDocument`, `deletableDocument` | `{ "index": ..., "keys": {...} }`: the value is part of a key, and the referenced document is the one a unique index of `documentType` finds. Each key maps an index property to a referring property path, `"$ownerId"` or `"."` (the value itself, exactly once). See [Resolved through a unique index](data-model/documents.md#resolved-through-a-unique-index-lookup). |
| `inList` | `listElement` | The typed array of identifiers on the referenced document the value must be in. The list must never change once written. See [An element of a list](data-model/documents.md#an-element-of-a-list-listelement). |
| `keyIdProperty` | `identityPublicKey` on an identifier | The sibling integer property holding the key id. |
| `identityProperty` | `identityPublicKey` on a key id | Whose key it is: `"$ownerId"`, `"$creatorId"` or an identifier property path. |
| `keyRequirements` | `identityPublicKey` | What the key must be: `purpose` (`authentication`, `encryption`, `decryption`, `transfer`, `voting` or `owner`) and `boundTo` (a document type of this contract the key must be bound to). |
| `contractRequirements` | `contract` | What the contract must be: `moderation` (`"elected"`, or `"electionOpen"` once its election delay has passed), `minimumAgeSeconds`, `minimumSecondsSinceUpdate`, `owner` (`"self"`, the writer, or `"other"`), `readonly: true`, `keepsHistory: true`, `ownerProtected` (boolean). See [Elected Moderation](data-model/contract-moderation.md#elected-moderation). |

### Errors

| Error | Code | When |
|---|---|---|
| `ReferencedEntityNotFoundError` | 40120 | The target does not exist, a lookup finds nothing, or a value is not in the list. |
| `ReferencedDocumentTypeNotFoundError` | 40121 | At registration: `documentType` does not exist. |
| `ReferencedDocumentTypeDeletableError` | 40122 | At registration: a `permanentDocument` or `listElement` reference names a type that can be deleted. |
| `ReferencedIdentityKeyNotFoundError` | 40123 | The key does not exist. |
| `ReferencedIdentityKeyDisabledError` | 40124 | The key is disabled. |
| `ReferencedKeyIdPropertyInvalidError` | 40125 | A key id is set without the identity it belongs to, or a key reference is declared twice. |
| `ReferencedDocumentPropertyAgreementInvalidError` | 40126 | At registration: a `propertyAgreement` pair names a missing property or mismatched kinds. |
| `ReferencedDocumentPropertyMismatchError` | 40127 | A `propertyAgreement` pair does not hold. |
| `ReferencedDocumentTypeNotDeletableError` | 40131 | At registration: a `deletableDocument` reference names a type that forbids deletion. |
| `ReferencedContractRequirementNotMetError` | 40135 | A `contractRequirements` entry is not met. |
| `ReferencedIdentityKeyRequirementNotMetError` | 40136 | A `keyRequirements` entry is not met. |
| `ReferencedDocumentLookupInvalidError` | 40137 | At registration: a lookup into another contract cannot resolve. |
| `ReferencedDocumentListInvalidError` | 40138 | At registration: an `inList` list of another contract does not qualify. |

## Index keywords

An index entry sits in `indices`. A document type has at most 10 indexes of at most 10 properties each. See [Indexes](drive/indexes.md).

| Keyword | Value | What it does | From |
|---|---|---|---|
| `name` | string, 1 to 32 characters | Required. The index's name, unique within the type. Queries and errors name it. | 1 |
| `properties` | array of `{ "<property>": "asc" }` | The indexed properties, in order; a query uses the index through a prefix of them. A nested property is named by its dotted path (`records.identity`), and system properties such as `$ownerId` and `$createdAt` may be indexed. An indexed string needs `maxLength` of at most 63 and a byte array `maxItems` of at most 255 (`InvalidIndexedPropertyConstraintError`, 10205); a typed array cannot be indexed (`InvalidIndexPropertyTypeError`, 10206). | 1 |
| `unique` | boolean | No two documents may hold the same values for all the properties (`DuplicateUniqueIndexError`, 40105). A document with a null among them is not held to it. | 1 |
| `nullSearchable` | boolean, default `true` | `false` leaves out of the index a document whose indexed properties are all null. | 1 |
| `contested` | object | Makes a unique index a contested resource: a document whose values match opens or joins a contest that masternodes vote on, instead of being refused as a duplicate. Takes `resolution` (required: `0` masternode vote, `1` masternode vote without a lock choice, from 14), `fieldMatches` (a list of `{ "field", "regexPattern" }`, the values that are contested) and `description`. Needs `unique: true` on a type whose documents cannot be replaced (`ContestedUniqueIndexOnMutableDocumentTypeError`, 10248). See [Contested Documents](data-model/contested-documents.md). | 1 |
| `countable` | `"notCountable"`, `"countable"`, `"countableAllowingOffset"`, or a boolean | Keeps a document count per indexed value, so counts are read without walking the documents. See [Document Count Trees](drive/document-count-trees.md#per-index-countable-flag). | 12 |
| `rangeCountable` | boolean | Counts over ranges of the indexed value in logarithmic time, with proofs. Implies `countable`. | 12 |
| `summable` | property name | Keeps the sum of a required integer property per indexed value. See [Document Sum Trees](drive/document-sum-trees.md#per-index-summable-flag). | 12 |
| `rangeSummable` | boolean | Sums over ranges of the indexed value. Needs `summable` or `averageable`. | 12 |
| `averageable` | property name | Shorthand for `countable: "countable"` plus `summable` on the named property. | 12 |
| `rangeAverageable` | boolean | Shorthand for `rangeCountable` plus `rangeSummable`. Needs `averageable`. | 12 |
| `rankedCountable` | boolean, or `{ "at": ... }` | Orders the indexed values by how many documents each has, for "top K" queries with proofs. `at` names the index level, or levels, that carry the ranking. Needs `rangeCountable`. See [Document Ranked Trees](drive/document-ranked-trees.md#contract-grammar). | 14 |
| `rankedSummable` | boolean | Orders the indexed values by the sum of the `summable` property. Needs `rangeSummable`. | 14 |
| `rankedAverageable` | boolean | Orders the indexed values by the average of the `averageable` property. Needs `rangeAverageable`, or `rangeCountable` and `rangeSummable`. | 14 |
| `timeRange` | `{ "on", "range", "step", "phase", "ttl" }` | Buckets the index's first property, a system timestamp, into time windows of `range` seconds starting every `step` seconds, offset by `phase`, for trending queries. `ttl` (at most one week) expires entries past their window. See [Time-Range Index TTL](drive/time-range-ttl.md#grammar-and-validation). | 14 |
| `terminal` | property name or list of names | On an `indexOnly` type, the property or properties whose values key each entry, in place of the document id. Default `$ownerId`. | 14 |
| `preallocated` | boolean | On an `indexOnly` type bound to a same-contract `permanentDocument` reference, creates the index's trees when the referenced document is created, so every entry costs the same. See [Preallocated index paths](drive/index-only-document-types.md#preallocated-index-paths). | 14 |
| `skipIfAbsent` | boolean | On an `indexOnly` type, a document without the index's first property writes no entry. See [Conditional participation](drive/index-only-document-types.md#conditional-participation-skipifabsent). | 14 |

From protocol version 14 a contract update may not add, remove or change an index of an existing document type (`DataContractInvalidIndexDefinitionUpdateError`, 10217). Indexes are compared by name, so reordering `indices` is no change and renaming one is a removal plus an addition. A document type the update adds may declare any index.

## `propertyConstraints` operators

A rule is an object with one key. Paths are dotted property paths (`"meta.total"`); a bare string is always a path and a bare number always a value.

| Operator | Form | Holds when |
|---|---|---|
| `equal`, `notEqual` | `[a, b]` | The two integer expressions are equal (or not). Also compares a string property with `{ "const": "..." }` or another string property, and an identifier property with a base58 `const`, another identifier property or `$ownerId`. |
| `lessThan`, `lessThanOrEqual`, `greaterThan`, `greaterThanOrEqual` | `[a, b]` | The comparison of the two integer expressions holds. |
| `in` | `[a, [v1, v2, ...]]` | `a` takes one of two or more distinct values: integers, or strings for a string or identifier property. |
| `present` | `"path"` | The document holds the property, with a value other than null. |
| `absent` | `"path"` | The document leaves the property out or sets it to null. |
| `anyOf` | `[c1, c2, ...]` | At least one condition holds, checked in order. |
| `allOf` | `[c1, c2, ...]` | Every condition holds, checked in order. |
| `not` | `c` | The condition does not hold. |

An integer expression is a number, the path of an integer or boolean property (a property left out reads as 0, a boolean as 1 or 0), or an object with one key:

| Operator | Form | Value |
|---|---|---|
| `add`, `multiply` | `[a, b, ...]` | The sum or product of two or more operands. |
| `subtract`, `divide`, `modulo`, `power` | `[a, b]` | The result of the operation. `divide` and `modulo` are Euclidean. |
| `ifAbsent` | `["path", default]` | The property's value, or `default` when the document leaves it out. |
| `length`, `byteLength` | `"path"` | The characters (as `maxLength` counts them) or UTF-8 bytes (as `maxBytes` counts them) of a string property, 0 when the document leaves it out. |
| `count` | `"path"` | The items of an array property, or the bytes of a byte array property, 0 when the document leaves it out. |
| `const` | `"string"` | A string or identifier constant, only as a side of `equal` or `notEqual`. |

Arithmetic is exact over 128-bit integers: an overflow, a division by zero or a negative exponent breaks the rule.

## System properties

The platform manages these. A document type lists them to use them: in `required` to have the platform record them, in `indices` to query by them, and in `properties` where it wants to say more about them.

| Property | What it holds | Recorded |
|---|---|---|
| `$id` | The document's id. | Always |
| `$ownerId` | The identity that owns the document: its creator, then whoever it was transferred or sold to. | Always |
| `$revision` | The document's revision: 1 at creation, raised by every replace, transfer, price update and purchase. | On types whose documents can be replaced, transferred or traded |
| `$createdAt`, `$updatedAt`, `$transferredAt` | Block times, in milliseconds, of the creation, the last replace or price update, and the last transfer. | When listed in `required` |
| `$createdAtBlockHeight`, `$updatedAtBlockHeight`, `$transferredAtBlockHeight` | Platform block heights of the same events. | When listed in `required` |
| `$createdAtCoreBlockHeight`, `$updatedAtCoreBlockHeight`, `$transferredAtCoreBlockHeight` | Core chain block heights of the same events. | When listed in `required` |
| `$creatorId` | The identity that created the document, which a transfer or purchase never changes. Not declared in `properties`: references, `creatorRefersTo` and indexes read it. | On transferable or tradeable types of format-1 contracts, from protocol version 10 |

## Contract-level keys

The document types sit in a contract, which has keys of its own. See [Data Contracts](data-model/data-contracts.md#what-v1-added).

| Key | What it is | From |
|---|---|---|
| `$formatVersion` | The contract's serialization format, `"0"` or `"1"`. Format 1 is the default from protocol version 9 and carries every key below. | 1 |
| `id`, `ownerId`, `version` | The contract's id, the identity that owns it, and its version, which each update raises by one. | 1 |
| `config` | Contract-wide settings, below. | 1 |
| `documentSchemas` | The document types, by name. | 1 |
| `schemaDefs` | Definitions every document type may `$ref`. An update may add definitions, not remove them (`IncompatibleDataContractSchemaError`, 10213). | 1 |
| `groups` | Groups of identities that act together, each member with a voting power, where a token action needs a group's approval. | 9 |
| `tokens` | The contract's tokens, by position. | 9 |
| `keywords` | Up to 50 search keywords, 3 to 50 characters each, no repeats, for the keyword search contract. | 9 |
| `description` | 3 to 100 characters, for the keyword search contract. | 9 |
| `createdAt`, `updatedAt`, and their block heights and epochs | Set by the platform. | 9 |

The `config` keys:

| Key | Default | What it does | From | Update |
|---|---|---|---|---|
| `canBeDeleted` | `false` | Whether the contract itself may ever be deleted. No transition deletes a contract today. | 1 | Fixed (40002) |
| `readonly` | `false` | `true` means the contract can never be updated (`DataContractIsReadonlyError`, 40001). | 1 | Cannot be set by an update (40002) |
| `keepsHistory` | `false` | Drive keeps every version of the contract. | 1 | Fixed (40002) |
| `documentsKeepHistoryContractDefault` | `false` | The `documentsKeepHistory` of a document type that does not say. | 1 | Fixed (40002) |
| `documentsMutableContractDefault` | `true` | The `documentsMutable` of a document type that does not say. | 1 | Fixed (40002) |
| `documentsCanBeDeletedContractDefault` | `true` | The `canBeDeleted` of a document type that does not say. | 1 | Fixed (40002) |
| `requiresIdentityEncryptionBoundedKey`, `requiresIdentityDecryptionBoundedKey` | absent | The document type keywords of the same names, for keys bound to the whole contract. | 1 | Fixed (40002) |
| `sizedIntegerTypes` | `true` | Stores each integer in the smallest width its `minimum` and `maximum` allow, instead of 8 bytes. | 9 | May be turned on, not off (40002) |
| `moderation` | absent | Declares which of a banlist, a suspension list and a warning list the contract keeps, and who moderates: the owner, appointed identities or an elected team. Needed by `canBeDeletedByModerators` and the moderators' share of `actionFees`. See [Contract Moderation](data-model/contract-moderation.md). | 14 | The lists kept and an elected team are fixed; appointed moderators may change (40002) |

## Limits

The first three come from the meta-schema, the rest from protocol version 14's `SystemLimits`. A contract over a limit is refused at registration.

| Limit | Value | Applies to |
|---|---|---|
| Properties per object | 100 | `properties`, at the top and in each nested object |
| Indexes per document type | 10 | `indices` |
| Properties per index | 10 | an index's `properties` |
| `max_field_value_size` | 5120 bytes | any one value a document stores (`DocumentFieldMaxSizeExceededError`, 10417) |
| `max_typed_array_items` | 1024 | a typed array's `maxItems` |
| `max_references_per_document` | 256 | references one document carries |
| `max_reference_operands` | 4 | operands in one `anyOf` or `allOf` of a reference |
| `max_reference_expression_depth` | 4 | nesting of reference expressions |
| `max_property_constraints` | 16 | rules in one `propertyConstraints` |
| `max_property_constraint_nodes` | 32 | nodes in one rule |
| `min_document_ttl_seconds`, `max_document_ttl_seconds` | 3600, 31536000 | `ttl` |
| `max_time_range_ttl_seconds` | 604800 | a `timeRange` index's `ttl` |
