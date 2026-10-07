# Contract Keywords

A data contract describes its documents with a JSON schema per document type, and Platform reads a set of keywords in those schemas: some from JSON Schema, most of its own. The chapters of this part take each keyword, or a small group that works together, and say what it does, how to write it, what is checked when a contract is registered and when a document is written, what a later contract update may do with it, and which errors it produces. The chapters of the Data Model and Drive parts explain the internals behind them and are linked from each chapter.

Everything here follows the document meta-schema of protocol version 14, `packages/rs-dpp/schema/meta_schemas/document/v3/document-meta.json`. Every document type schema is validated against it when a contract is registered or updated, and the parser (`try_from_schema`) checks the rules a JSON schema cannot express. When a chapter and the meta-schema disagree, the meta-schema is right and the chapter is out of date.

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

The contract around the document types has keys of its own, and a `config` object: see [Contract-Level Keys and config](contract-keywords/contract-config.md).

## Reading the chapters

Each chapter opens with a short table for each keyword:

- **Since** is the first protocol version at which the keyword can be used.
- **On update** is what a contract update may do with the keyword on a document type that already exists. *Fixed* means adding, removing and changing it are all refused. A document type the update adds may use any keyword, as a new contract may.
- **Errors** are consensus errors, written `ErrorName` (code). See [Error Codes](error-handling/error-codes.md) for the code ranges.

A contract update that breaks an update rule is refused with one of two errors, depending on which check catches it. `DocumentTypeUpdateError` (40212) comes from the comparison of the parsed document types, which judges flags such as `documentsMutable` by their meaning. `IncompatibleDocumentTypeSchemaError` (10246) comes from the comparison of the two JSON schemas, which judges property keywords such as `refersTo` or `maxLength` by their text. Top-level `required` and `indices` have errors of their own (10276 and 10217). Because the schema comparison reads text, an edit that changes how a keyword is written but not what it means, such as writing out a default or switching to the `documentsAverageable` shorthand, is refused with 10246.

## Protocol versions

The document meta-schema has changed three times:

| Meta-schema | Protocol versions | What it added |
|---|---|---|
| v0 | 1 to 11 | The original keywords. A document type key the meta-schema did not know was ignored. |
| v1 | 12 | Unknown document type keys are refused. The count, sum and average keywords. |
| v2 | 13 | `keepsTransferHistory`, `keepsPurchaseHistory`, `keepsPricingHistory`. |
| v3 | 14 | References, typed arrays, `requiredSince`, `immutable`, `ttl`, `propertyConstraints`, `actionFees`, moderation deletion, ranked, time-range and integer-range indexes, index-only types, and the rest marked 14 in these chapters. |

Most keywords of v0 took effect at protocol version 1. The exceptions are `tokenCost` (9) and the index keyword `countable` (12).

## The complete language

Every key a contract can write, grouped by where it goes. **Since** is the protocol version from which the key works. **Read more** links to the key's section in this part and, where there is one, to the chapter with the internals. A dotted name such as `tokenCost.<action>.amount` is a key written inside the ones before it.

### The contract

| Key | Takes | What it does | Since | Read more |
|---|---|---|---|---|
| `$formatVersion` | `"0"` or `"1"` | The contract's serialization format. `"1"`, the default from 9, carries `groups`, `tokens`, `keywords`, `description` and the timestamps. | 1 | [Contract keys](contract-keywords/contract-config.md#contract-keys) |
| `id` | identifier | The contract's id: a hash of `ownerId` and the identity nonce of the create transition. | 1 | [Contract keys](contract-keywords/contract-config.md#contract-keys) |
| `ownerId` | identifier | The identity that registers the contract, and the only one that may update it. | 1 | [Contract keys](contract-keywords/contract-config.md#contract-keys) |
| `version` | integer | 1 at creation; every update raises it by exactly one. | 1 | [Contract keys](contract-keywords/contract-config.md#contract-keys) |
| `config` | object | Contract-wide settings, [below](#config). | 1 | [config](contract-keywords/contract-config.md#config) |
| `documentSchemas` | object of document types | The document types by name, each written with the [document type keys](#document-type). | 1 | [documentSchemas](contract-keywords/contract-config.md#documentschemas) |
| `schemaDefs` | object | Definitions any property may point at with `$ref`. | 1 | [Document Shape](contract-keywords/document-shape.md#schema-and-defs) |
| `groups` | object | Sets of identities, each member with a voting power, whose approval some token actions need. | 9 | [Data Contracts](data-model/data-contracts.md#what-v1-added) |
| `tokens` | object | The contract's tokens, by position. Their configuration is not covered in this part. | 9 | [Data Contracts](data-model/data-contracts.md#what-v1-added) · [Creating a Basic Token](evo-sdk/tutorials/basic-token.md) |
| `keywords` | up to 50 strings of 3 to 50 bytes | Search keywords, for the keyword search contract. | 9 | [keywords and description](contract-keywords/contract-config.md#keywords-and-description) |
| `description` | string of 3 to 100 bytes | A short description, for the keyword search contract. | 9 | [keywords and description](contract-keywords/contract-config.md#keywords-and-description) |
| `createdAt`, `updatedAt`, `createdAtBlockHeight`, `updatedAtBlockHeight`, `createdAtEpoch`, `updatedAtEpoch` | numbers | When the contract was created and last updated. Set by the platform, never written. | 9 | [Contract keys](contract-keywords/contract-config.md#contract-keys) |
| `contractGroup` | `{ "admins", "name", "description" }` | On the create transition, beside the contract: registers a contract group, a set of contracts the signer owns, with up to 16 `admins`, a `name` of 1 to 64 characters and a `description` of 1 to 256. | 14 | [Contract Groups](data-model/contract-groups.md) |
| `contractGroupMemberships` | up to 16 `{ "contractGroupId", "member" }` | On the create transition: enrols the new contract (`"contract"`), one of its document types (`{ "documentType": ... }`) or one of its tokens (`{ "token": ... }`) in contract groups. | 14 | [Contract Groups](data-model/contract-groups.md) |

### config

| Key | Takes | What it does | Since | Read more |
|---|---|---|---|---|
| `canBeDeleted` | boolean, default `false` | Whether the contract may ever be deleted. No transition deletes a contract today. | 1 | [canBeDeleted](contract-keywords/contract-config.md#canbedeleted) |
| `readonly` | boolean, default `false` | `true`: the contract can never be updated. | 1 | [readonly](contract-keywords/contract-config.md#readonly) |
| `keepsHistory` | boolean, default `false` | Drive keeps every version of the contract. | 1 | [keepsHistory](contract-keywords/contract-config.md#keepshistory) |
| `documentsKeepHistoryContractDefault` | boolean, default `false` | `documentsKeepHistory` for a document type that does not say. | 1 | [Document type defaults](contract-keywords/contract-config.md#document-type-defaults) |
| `documentsMutableContractDefault` | boolean, default `true` | `documentsMutable` for a document type that does not say. | 1 | [Document type defaults](contract-keywords/contract-config.md#document-type-defaults) |
| `documentsCanBeDeletedContractDefault` | boolean, default `true` | `canBeDeleted` for a document type that does not say. | 1 | [Document type defaults](contract-keywords/contract-config.md#document-type-defaults) |
| `requiresIdentityEncryptionBoundedKey`, `requiresIdentityDecryptionBoundedKey` | `0` unique, `1` multiple, `2` multiple with a pointer to the latest | Lets identities bind encryption or decryption keys to the whole contract, and says how they are kept. | 1 | [Bounded key requirements](contract-keywords/contract-config.md#bounded-key-requirements) · [Contract Bounds](sdk/identity-keys.md#contract-bounds) |
| `sizedIntegerTypes` | boolean, default `true` | Stores each integer in the smallest width its bounds allow, instead of 8 bytes. | 9 | [sizedIntegerTypes](contract-keywords/contract-config.md#sizedintegertypes) |
| `moderation` | object | Makes the contract moderated: which lists it keeps and who moderates. | 14 | [moderation](contract-keywords/contract-config.md#moderation) · [Contract Moderation](data-model/contract-moderation.md) |
| `moderation.banlist`, `.suspensions`, `.warnings` | boolean, default `false` | Keeps a banlist, a suspension list, a warning list. A banned or suspended identity cannot act on the contract's documents; a warning bars nothing. | 14 | [The Model](data-model/contract-moderation.md#the-model) |
| `moderation.moderators` | `{ "$type": ... }` | Who moderates: `"contractOwner"`, `"appointedModerators"` with `identities` (1 to 16), or `"elected"` with the keys below. | 14 | [moderation](contract-keywords/contract-config.md#moderation) |
| `moderators.seatContestable` | boolean, required when elected | Whether a seated team may later be challenged. | 14 | [Elected Moderation](data-model/contract-moderation.md#elected-moderation) |
| `moderators.challengeCoolDown` | seconds, two weeks to three years | How long a seated team is safe from a challenge after a seat change. Required when the seat is contestable, refused when it is not. | 14 | [Elected Moderation](data-model/contract-moderation.md#elected-moderation) |
| `moderators.moderatedDocumentTypes` | object: document type → abilities | The document types the team moderates, each with its abilities: `ban`, `suspend`, `warn`, `deleteDocuments`. | 14 | [Elected Moderation](data-model/contract-moderation.md#elected-moderation) |
| `moderators.interim` | `{ "$type": ... }` | Who moderates until a team is seated: `"contractOwner"`, `"appointedModerators"`, `"notYetUsable"` (the moderated types cannot be used yet) or `"noModeration"`. | 14 | [Elected Moderation](data-model/contract-moderation.md#elected-moderation) |
| `moderators.joinWindow`, `.voteWindow` | seconds | How long applicants may join an election, and how long masternodes then vote. | 14 | [Elected Moderation](data-model/contract-moderation.md#elected-moderation) |
| `moderators.electionDelay` | seconds | How long after the contract's creation the first election may be called. | 14 | [Elected Moderation](data-model/contract-moderation.md#elected-moderation) |
| `moderators.maxAddedModerators` | 0 to 15, default 0 | How many members the seated leader may add after the election. | 14 | [Elected Moderation](data-model/contract-moderation.md#elected-moderation) |
| `moderators.ownerProtected` | boolean, default `false` | Protects the contract owner from the seated team. | 14 | [Elected Moderation](data-model/contract-moderation.md#elected-moderation) |

### Document type

| Key | Takes | What it does | Since | Read more |
|---|---|---|---|---|
| `type` | `"object"` | Required. A document is an object. | 1 | [type](contract-keywords/document-shape.md#type) |
| `properties` | object of 1 to 100 properties | The document's properties, each written with the [property keys](#property). | 1 | [properties](contract-keywords/document-shape.md#properties) |
| `required` | array of names | The properties every document holds. A system time or height listed here is recorded. | 1 | [required](contract-keywords/document-shape.md#required) |
| `additionalProperties` | `false` | Required: a document holds only the declared properties. | 1 | [additionalProperties](contract-keywords/document-shape.md#additionalproperties) |
| `minProperties`, `maxProperties` | integer | How many properties a document holds. | 1 | [minProperties and maxProperties](contract-keywords/document-shape.md#minproperties-and-maxproperties) |
| `dependentRequired` | object | A property that requires others when present. | 1 | [dependentRequired](contract-keywords/document-shape.md#dependentrequired) |
| `$comment`, `description` | string | Notes; consensus ignores them. | 1 | [$comment and description](contract-keywords/document-shape.md#comment-and-description) |
| `$schema`, `$defs` | added by the platform | The meta-schema URL and the contract's `schemaDefs`. A document type writing either is refused. | 1 | [$schema and $defs](contract-keywords/document-shape.md#schema-and-defs) |
| `transient` | array of top-level names | Properties validated on the transition but never stored. | 1 | [transient](contract-keywords/transient.md) · [internals](data-model/documents.md#transient-properties) |
| `documentsMutable` | boolean, default `true` | `false`: documents cannot be replaced. | 1 | [documentsMutable](contract-keywords/mutability.md#documentsmutable) |
| `immutable` | array of top-level names and `{ property, when }` | Properties frozen at creation, or while a condition holds, on a mutable type. | 14 | [immutable](contract-keywords/mutability.md#immutable) · [internals](data-model/documents.md#immutable-properties-on-mutable-document-types) |
| `canBeDeleted` | boolean or `"onlyWhenConsumed"`, default `true` | `false`: a document's owner cannot delete it. `"onlyWhenConsumed"` (14): only a create that consumes it deletes it. | 1 | [canBeDeleted](contract-keywords/deletion.md#canbedeleted) |
| `retractedWhen` | one condition, as an `immutable` entry's `when` | The replace a banned or suspended owner may still make on a moderated contract: one whose written document meets the condition. | 14 | [retractedWhen](contract-keywords/deletion.md#retractedwhen) · [internals](data-model/contract-moderation.md#the-document-gate) |
| `moderatorAbilities` | object: `delete`, `deleteWithin` (seconds), `deleteKeepsRecord`, `deleteRefundsOwner`, `deleteSettled` (`leader`, `approvals`, `approversPredateDocument`), `deleteKeepsFields` (array of property paths), `changeFields` (array of top-level names) | What the contract's moderators may do to documents of the type: delete them, within a window after their last change, past it only when so many members of a seated team agree (the leader among them when the rule says so, members the leader added only for documents written after their addition unless the rule says otherwise), with or without a removal record keeping the fields that stay public and a refund to the owner, and write the fields only they write. | 14 | [Moderator Abilities](contract-keywords/moderator-abilities.md) · [delete](contract-keywords/deletion.md#moderatorabilitiesdelete) · [internals](data-model/contract-moderation.md#changing-document-fields) |
| `ttl` | seconds, 3600 to 31536000 | The platform deletes each document this long after its creation. | 14 | [Time To Live](contract-keywords/ttl.md) · [internals](data-model/document-ttl.md) |
| `creationRestrictionMode` | `0` anyone, `1` contract owner, `2` nobody | Who may create documents. | 1 | [creationRestrictionMode](contract-keywords/ownership-and-trading.md#creationrestrictionmode) |
| `transferable` | `0` never, `1` always | Whether an owner may give a document to another identity. | 1 | [transferable](contract-keywords/ownership-and-trading.md#transferable) |
| `tradeMode` | `0` none, `1` direct purchase | Whether an owner may set a price and anyone buy at it. | 1 | [tradeMode](contract-keywords/ownership-and-trading.md#trademode) |
| `documentsKeepHistory` | boolean, default `false` | Drive keeps every revision of every document. | 1 | [documentsKeepHistory](contract-keywords/history.md#documentskeephistory) |
| `keepsTransferHistory`, `keepsPurchaseHistory`, `keepsPricingHistory` | boolean, default `false` | Records every transfer, purchase or price update in the document history contract. | 13 | [History](contract-keywords/history.md#keepstransferhistory) |
| `signatureSecurityLevelRequirement` | `1` critical, `2` high (default), `3` medium | The weakest key level that may sign a transition on the type. | 1 | [signatureSecurityLevelRequirement](contract-keywords/signing-keys.md#signaturesecuritylevelrequirement) · [Security Level](sdk/identity-keys.md#security-level) |
| `requiresIdentityEncryptionBoundedKey`, `requiresIdentityDecryptionBoundedKey` | `0` unique, `1` multiple, `2` multiple with a pointer to the latest | Lets identities bind encryption or decryption keys to the type, and says how they are kept. | 1 | [Signing and Keys](contract-keywords/signing-keys.md#requiresidentityencryptionboundedkey) · [Contract Bounds](sdk/identity-keys.md#contract-bounds) |
| `ownerRefersTo` | a [`refersTo`](#refersto) declaration | A reference the writer must meet, on types whose documents are never transferred or traded. | 14 | [ownerRefersTo](contract-keywords/owner-refers-to.md#ownerrefersto) · [internals](data-model/documents.md#on-the-writer-or-the-creator-ownerrefersto-creatorrefersto) |
| `creatorRefersTo` | a [`refersTo`](#refersto) declaration | A reference the creator must meet, on types whose documents can be transferred or traded. | 14 | [creatorRefersTo](contract-keywords/owner-refers-to.md#creatorrefersto) |
| `propertyConstraints` | object of named rules, at most 16 | Rules over several properties every created or replaced document meets. See the [operators](#propertyconstraints). | 14 | [propertyConstraints](contract-keywords/property-constraints.md) · [internals](data-model/documents.md#property-constraints-propertyconstraints) |
| `tokenCost` | object keyed by action | Token payments for actions on documents. See the [keys](#tokencost-and-actionfees). | 9 | [Token Costs](contract-keywords/token-cost.md) · [Fees](fees/overview.md#gas-paid-by-the-contract-owner) |
| `actionFees` | object keyed by action | Credit fees for actions on documents, paid to the owner's and moderators' pots. See the [keys](#tokencost-and-actionfees). | 14 | [Action Fees](contract-keywords/action-fees.md) · [Fees](fees/overview.md#document-action-fees) |
| `indices` | array of 1 to 10 indexes | The indexes documents are queried by, each written with the [index keys](#index). | 1 | [Indexes](contract-keywords/indexes.md#indices) · [internals](drive/indexes.md) |
| `documentsCountable` | boolean | Keeps a count of the type's documents. | 12 | [documentsCountable](contract-keywords/aggregates.md#documentscountable) · [internals](drive/document-count-trees.md#primary-key-tree-flags) |
| `documentsSummable` | property name | Keeps the sum of one integer property over the type's documents. | 12 | [documentsSummable](contract-keywords/aggregates.md#documentssummable) · [internals](drive/document-sum-trees.md#primary-key-tree-flags) |
| `documentsAverageable` | property name | Shorthand for `documentsCountable` plus `documentsSummable`. | 12 | [documentsAverageable](contract-keywords/aggregates.md#documentsaverageable) |
| `rangeCountable`, `rangeSummable`, `rangeAverageable` | boolean | Provable counts, sums or averages over ranges of document ids. | 12 | [Document type range keys](contract-keywords/aggregates.md#document-type-rangecountable-rangesummable-rangeaverageable) |
| `indexOnly` | boolean | Documents are never stored whole: the index entries are the rows. | 14 | [indexOnly](contract-keywords/index-only.md#indexonly) · [internals](drive/index-only-document-types.md) |
| `entryPayload` | array of 1 to 16 names | On an index-only type, properties carried in each entry's value instead of a key. | 14 | [entryPayload](contract-keywords/index-only.md#entrypayload) |

### Property

| Key | Takes | What it does | Since | Read more |
|---|---|---|---|---|
| `type` | `string`, `integer`, `number`, `boolean`, `object`, `array` | The kind of value. An array is a byte array or a typed array. | 1 | [type](contract-keywords/property-schemas.md#type) |
| `position` | integer | The property's place in the stored document. Required; top-level positions run 0, 1, 2 with no gap. | 1 | [position](contract-keywords/property-schemas.md#position) · [Document Serialization](serialization/document-serialization.md) |
| `minLength`, `maxLength` | integer | A string's length in characters. | 1 | [Strings](contract-keywords/property-schemas.md#strings) |
| `pattern` | regular expression | A string must match it. Needs `maxLength` of at most 50000. | 1 | [Strings](contract-keywords/property-schemas.md#strings) |
| `format` | `date-time`, `date`, `time`, `email`, `idn-email`, `hostname`, `ipv4`, `ipv6`, `uri`, `regex` | A string must have this format. Needs `maxLength` of at most 50000. | 1 | [Strings](contract-keywords/property-schemas.md#strings) |
| `maxBytes` | 1 to 65535 | The most UTF-8 bytes a string may take. | 14 | [maxBytes](contract-keywords/max-bytes.md) · [internals](data-model/documents.md#byte-caps-on-strings-maxbytes) |
| `minimum`, `maximum`, `exclusiveMinimum`, `exclusiveMaximum`, `multipleOf` | number | Numeric bounds. `minimum` and `maximum` also decide an integer's stored width. | 1 | [Numbers](contract-keywords/property-schemas.md#numbers) |
| `enum`, `const` | values | The values allowed, or the one value allowed. | 1 | [enum and const](contract-keywords/property-schemas.md#enum-and-const) |
| `byteArray` | `true` | Makes an array a string of bytes, stored raw. | 1 | [Byte arrays and identifiers](contract-keywords/property-schemas.md#byte-arrays-and-identifiers) |
| `contentMediaType` | `"application/x.dash.dpp.identifier"` | Makes a 32-byte array an identifier. | 1 | [Byte arrays and identifiers](contract-keywords/property-schemas.md#byte-arrays-and-identifiers) |
| `minItems`, `maxItems`, `uniqueItems`, `contains` | integer, boolean, schema | A byte array's length in bytes, or a typed array's number of elements (at most 1024); no repeats; an element that matches. | 1 | [Arrays](contract-keywords/property-schemas.md#arrays) |
| `items` | an element schema | Makes an array a typed array whose elements all follow this schema. | 14 | [Typed Arrays](contract-keywords/typed-arrays.md) · [internals](data-model/documents.md#typed-arrays) |
| `properties`, `required`, `additionalProperties`, `minProperties`, `maxProperties`, `dependentRequired` | as on a document type | A nested object's members and its bounds. | 1 | [Objects](contract-keywords/property-schemas.md#objects) |
| `$ref` | `"#/$defs/<name>"` | Uses a definition from the contract's `schemaDefs`. | 1 | [$ref](contract-keywords/property-schemas.md#ref) |
| `$id`, `$comment`, `description`, `examples` | annotations | Notes; consensus ignores them. | 1 | [Annotations](contract-keywords/property-schemas.md#annotations) |
| `requiredSince` | contract version | Lets an update add a required property that older documents may leave out. | 14 | [requiredSince](contract-keywords/required-since.md) · [internals](data-model/data-contracts.md#evolving-a-contract-adding-required-fields) |
| `distinctFrom` | a property path or `"$ownerId"` | An identifier must differ from another identifier of the document, or from the owner. | 14 | [distinctFrom](contract-keywords/distinct-from.md) · [internals](data-model/documents.md#distinct-identifier-properties) |
| `encryptedFor` | `{ "recipient", "recipientKey", "senderKey", "scheme" }` | Declares how an encrypted byte array was made: whose keys, which scheme. | 14 | [encryptedFor](contract-keywords/encrypted-for.md) · [internals](data-model/documents.md#encrypted-properties-encryptedfor) |
| `encryptedFor.recipient` | identifier property path or `"$ownerId"` | The identity the value is encrypted to. | 14 | [encryptedFor](contract-keywords/encrypted-for.md#example) |
| `encryptedFor.recipientKey`, `.senderKey` | integer property paths | The properties holding the recipient's and the sender's key ids. | 14 | [encryptedFor](contract-keywords/encrypted-for.md#example) |
| `encryptedFor.scheme` | `"ecdh-secp256k1-aes256-cbc"` | How the ciphertext is made. | 14 | [The scheme](contract-keywords/encrypted-for.md#the-scheme) |
| `generatedFrom` | `{ "function", "params" }` | The platform generates the string from other properties of the document; on arrival when a document leaves it out. | 14 | [generatedFrom](contract-keywords/generated-from.md) · [internals](data-model/documents.md#generated-properties-generatedfrom) |
| `generatedFrom.function` | `"sys.stringTransformations.homographSafeASCII"` | The system function that generates the value: `sys.stringTransformations.` `lowercase`, `uppercase`, `capitalize`, `camelCase`, `snakeCase` or `homographSafeASCII`. | 14 | [Functions](contract-keywords/generated-from.md#functions) |
| `generatedFrom.params` | property paths | The properties the function reads, in order. | 14 | [Params](contract-keywords/generated-from.md#params) |
| `refersTo` | a declaration | What an identifier points at, checked when a document is written. See the [keys](#refersto). | 14 | [References](contract-keywords/refers-to.md) · [internals](data-model/documents.md#document-references-refersto) |

A typed array's element (`items`) takes `type`, `enum`, `minimum`, `maximum`, `exclusiveMinimum`, `exclusiveMaximum`, `multipleOf`, `minLength`, `maxLength`, `pattern`, `format`, `minItems` and `maxItems` (bytes of a byte array element), `byteArray`, `contentMediaType`, `maxBytes`, `distinctFrom`, `refersTo`, `$comment` and `description`. It takes no `position`, `const`, `uniqueItems` or `examples`.

### refersTo

| Key | Takes | What it does | Since | Read more |
|---|---|---|---|---|
| `type` | a target below | What the value points at. | 14 | [Targets](contract-keywords/refers-to.md#targets) |
| `type: "identity"` | | The value is the id of an existing identity. | 14 | [identity](contract-keywords/refers-to.md#identity) |
| `type: "contract"` | | The value is the id of an existing data contract. | 14 | [contract](contract-keywords/refers-to.md#contract) |
| `type: "token"` | | The value is the id of an existing token. | 14 | [token](contract-keywords/refers-to.md#token) |
| `type: "permanentDocument"` | | The value is the id of a document whose type can never lose its documents; with `findBy`, part of the key that finds it; with `inList`, an element of its list. | 14 | [permanentDocument](contract-keywords/refers-to.md#permanentdocument) |
| `type: "deletableDocument"` | | The value is the id of a document that can be deleted, or with `findBy` part of the key that finds it; checked again on every replace. | 14 | [deletableDocument](contract-keywords/refers-to.md#deletabledocument) |
| `type: "identityPublicKey"` | | The value names an identity key that exists and is not disabled. | 14 | [identityPublicKey](contract-keywords/refers-to.md#identitypublickey) |
| `documentType` | document type name | The referenced document type. | 14 | [documentType](contract-keywords/refers-to.md#documenttype) |
| `contractId` | identifier | The contract holding `documentType`, when it is not this one. | 14 | [contractId](contract-keywords/refers-to.md#contractid) |
| `findBy` | 1 to 10 entries: referenced property → `"."`, `"$ownerId"`, a path or a function | Finds the document through the unique index of `documentType` over exactly these properties, in any order; `"."` is the value itself. Without it the value is the document's `$id`. | 14 | [findBy](contract-keywords/refers-to-lookup.md) · [internals](data-model/documents.md#found-by-a-unique-index-findby) |
| `findBy.<property>` | `{ "function": "sys.hash.sha256d", "params" }` | A function entry: the referenced property holds the hash of `params` (paths, `{ "const": text }`, `"."` for a value without a path), which fills that part of the key, finding a commitment made earlier. At most one. | 14 | [Commit and reveal](contract-keywords/refers-to-lookup.md#commit-and-reveal) · [internals](data-model/documents.md#commit-and-reveal-a-findby-function) |
| `where` | 1 to 10 entries `{ "<referenced property>": "<referring value>" }` | Checked on the document found: each referenced property (or `$ownerId`, `$creatorId`, `$id`) must equal the referring property. A value of `"$ownerId"`, the writer, makes a write gate. Never finds the document. | 14 | [where](contract-keywords/refers-to.md#where) |
| `minimumAgeBlocks` | 1 to 4294967295 | Beside a `findBy` function: the commitment was created at least this many blocks before the create. | 14 | [Commit and reveal](contract-keywords/refers-to-lookup.md#commit-and-reveal) |
| `consume` | `true` | Beside a `findBy` function, on a `deletableDocument` with the `where` entry `"$ownerId": "$ownerId"`, into a type with `canBeDeleted` `true` or `"onlyWhenConsumed"`: the create deletes the writer's commitment. | 14 | [Commit and reveal](contract-keywords/refers-to-lookup.md#commit-and-reveal) |
| `inList` | typed array path | On a `permanentDocument` whose `findBy` is `{ "$id": <property> }`: the list on that document the value must be in. | 14 | [List Elements](contract-keywords/refers-to-list-element.md) · [internals](data-model/documents.md#an-element-of-a-list-inlist) |
| `keyIdProperty` | integer property path | On an identity property: the property holding the key id. | 14 | [keyIdProperty and identityProperty](contract-keywords/refers-to.md#keyidproperty-and-identityproperty) |
| `identityProperty` | `"$ownerId"`, `"$creatorId"` or a path | On a key id property: whose key it is. | 14 | [keyIdProperty and identityProperty](contract-keywords/refers-to.md#keyidproperty-and-identityproperty) |
| `keyRequirements.purpose` | `authentication`, `encryption`, `decryption`, `transfer`, `voting`, `owner` | The key's purpose. | 14 | [keyRequirements](contract-keywords/refers-to.md#keyrequirements) |
| `keyRequirements.boundTo` | document type name | The key must be bound to that document type of this contract. | 14 | [keyRequirements](contract-keywords/refers-to.md#keyrequirements) |
| `contractRequirements.moderation` | `"elected"`, `"electionOpen"` | The contract declares an elected team, or one whose election may be called. | 14 | [contractRequirements](contract-keywords/refers-to.md#contractrequirements) · [Elected Moderation](data-model/contract-moderation.md#elected-moderation) |
| `contractRequirements.minimumAgeSeconds`, `.minimumSecondsSinceUpdate` | seconds | The contract was created, or last changed, at least this long ago. | 14 | [contractRequirements](contract-keywords/refers-to.md#contractrequirements) |
| `contractRequirements.owner` | `"self"`, `"other"` | The contract is owned by the writer, or by someone else. | 14 | [contractRequirements](contract-keywords/refers-to.md#contractrequirements) |
| `contractRequirements.readonly`, `.keepsHistory` | `true` | The contract can never be updated, or keeps history. | 14 | [contractRequirements](contract-keywords/refers-to.md#contractrequirements) |
| `contractRequirements.ownerProtected` | boolean | The contract's elected team does, or does not, protect its owner. | 14 | [contractRequirements](contract-keywords/refers-to.md#contractrequirements) |
| `anyOf`, `allOf` | 2 to 4 operands | In place of `type`: at least one, or every, operand holds. Nest at most 4 deep. | 14 | [Expressions](contract-keywords/refers-to-expressions.md) · [internals](data-model/documents.md#reference-expressions-anyof-allof) |

### tokenCost and actionFees

`<action>` is one of `create`, `replace`, `delete`, `transfer`, `update_price` and `purchase`.

| Key | Takes | What it does | Since | Read more |
|---|---|---|---|---|
| `tokenCost.<action>.tokenPosition` | 0 to 65535, required | Which token is charged. | 9 | [Token Costs](contract-keywords/token-cost.md) |
| `tokenCost.<action>.amount` | at least 1, required | How many tokens the action costs. | 9 | [Token Costs](contract-keywords/token-cost.md) |
| `tokenCost.<action>.contractId` | identifier | The contract whose token is charged, when it is not this one. | 9 | [contractId](contract-keywords/token-cost.md#tokens-of-another-contract-contractid) |
| `tokenCost.<action>.effect` | `0` to the contract owner (default), `1` burn | What happens to the tokens paid. | 9 | [effect](contract-keywords/token-cost.md#effect-transfer-or-burn) |
| `tokenCost.<action>.gasFeesPaidBy` | `0` document owner (default), `1` contract owner, `2` prefer contract owner | Who the contract owner offers to have pay the gas. Accepted from 9, acted on from 14. | 14 | [gasFeesPaidBy](contract-keywords/token-cost.md#who-pays-the-gas-gasfeespaidby) · [Fees](fees/overview.md#gas-paid-by-the-contract-owner) |
| `tokenCost.<action>.optional` | boolean, default `false` | A transition may skip the token and pay in credits. | 14 | [Optional costs](contract-keywords/token-cost.md#optional-costs) · [Fees](fees/overview.md#optional-token-costs) |
| `actionFees.pricing` | `"feeMultiplier"` (default), `"fixed"` | Whether the amounts scale with the epoch's fee multiplier. | 14 | [Action Fees](contract-keywords/action-fees.md#how-it-works) |
| `actionFees.<action>.owner` | credits | Paid into the contract owner's pot. | 14 | [The pots and the claim](contract-keywords/action-fees.md#the-pots-and-the-claim) |
| `actionFees.<action>.moderators` | credits | Paid into the moderators' pot. Needs `moderation`. | 14 | [The pots and the claim](contract-keywords/action-fees.md#the-pots-and-the-claim) · [Fee Pots](data-model/contract-moderation.md#fee-pots-and-the-claim) |

### propertyConstraints

A rule is one condition. Conditions:

| Key | Takes | Holds when | Since | Read more |
|---|---|---|---|---|
| `equal`, `notEqual` | `[a, b]` | The two sides are equal, or differ: integer expressions, strings or identifiers. | 14 | [Conditions](contract-keywords/property-constraints.md#conditions) |
| `lessThan`, `lessThanOrEqual`, `greaterThan`, `greaterThanOrEqual` | `[a, b]` | The integer comparison holds. | 14 | [Conditions](contract-keywords/property-constraints.md#conditions) |
| `in` | `[a, [values]]` | `a` takes one of two or more listed integers, strings or identifiers. | 14 | [Conditions](contract-keywords/property-constraints.md#conditions) |
| `present`, `absent` | a path | The document holds the property, or leaves it out (or null, or an object with no member present). | 14 | [Conditions](contract-keywords/property-constraints.md#conditions) |
| `anyOf`, `allOf` | two or more conditions | At least one, or every, condition holds, checked in order. | 14 | [Evaluation order](contract-keywords/property-constraints.md#evaluation-order-and-short-circuiting) |
| `not` | a condition | The condition does not hold. | 14 | [Conditions](contract-keywords/property-constraints.md#conditions) |
| `ifThen`, `ifThenElse` | `[if, then]`, `[if, then, else]` | The second condition holds when the first does (and, for `ifThenElse`, the third when it does not); only the branch taken is evaluated. | 14 | [Conditions](contract-keywords/property-constraints.md#conditions) |
| `notIn` | `[a, [values]]` | `a` takes none of the listed values. | 14 | [Conditions](contract-keywords/property-constraints.md#conditions) |
| `startsWith`, `endsWith` | `[text, affix]` | A string starts or ends with another, byte for byte. | 14 | [Conditions](contract-keywords/property-constraints.md#conditions) |
| `contains` | `[array, value]` | A typed array holds an element equal to the value. | 14 | [Conditions](contract-keywords/property-constraints.md#conditions) |

Expressions:

| Key | Takes | Value | Since | Read more |
|---|---|---|---|---|
| an integer | `100` | Itself. | 14 | [Expressions](contract-keywords/property-constraints.md#expressions) |
| a path | `"price"`, `"meta.total"` | An integer or boolean property's value; 0 when left out. | 14 | [Expressions](contract-keywords/property-constraints.md#expressions) |
| `add`, `multiply` | two or more operands | The sum or product. | 14 | [Arithmetic](contract-keywords/property-constraints.md#arithmetic) |
| `subtract`, `divide`, `modulo`, `power` | `[a, b]` | The difference, Euclidean quotient or remainder, or power. | 14 | [Arithmetic](contract-keywords/property-constraints.md#arithmetic) |
| `min`, `max`, `abs` | two or more operands, or one for `abs` | The least, the greatest, or the absolute value. | 14 | [Expressions](contract-keywords/property-constraints.md#expressions) |
| `countOf`, `sumOf` | `[type, filter?]`, `[type, property, filter?]` | How many documents of a type of the contract match the filter, or the total of an integer property over them, from its count or sum trees. | 14 | [Totals of other documents](contract-keywords/property-constraints.md#totals-of-other-documents) |
| `ifAbsent` | `[path, default]` | The property's value, or the default when left out (an integer, or a string for a string property). | 14 | [Expressions](contract-keywords/property-constraints.md#expressions) |
| `length`, `byteLength` | a string path | A string's length in characters, or in UTF-8 bytes. | 14 | [Expressions](contract-keywords/property-constraints.md#expressions) |
| `count` | an array path | The elements of a typed array, or the bytes of a byte array. | 14 | [Expressions](contract-keywords/property-constraints.md#expressions) |
| `$createdAt`, `$updatedAt`, `$transferredAt`, `$createdAtBlockHeight`, `$updatedAtBlockHeight`, `$transferredAtBlockHeight`, `$createdAtCoreBlockHeight`, `$updatedAtCoreBlockHeight`, `$transferredAtCoreBlockHeight` | a path | A time or height the document records, when listed in `required`. | 14 | [Times and heights](contract-keywords/property-constraints.md#times-and-heights) |
| `const` | a string | A string constant, or a base58 identifier, as one side of `equal` or `notEqual`. | 14 | [Strings](contract-keywords/property-constraints.md#strings) |
| `$ownerId` | | The document's owner, as an identifier side. | 14 | [Identifiers and $ownerId](contract-keywords/property-constraints.md#identifiers-and-ownerid) |

### Index

| Key | Takes | What it does | Since | Read more |
|---|---|---|---|---|
| `name` | 1 to 32 characters, required | The index's name, unique in the type. | 1 | [name](contract-keywords/indexes.md#name) |
| `properties` | 1 to 10 `{ "<path>": "asc" }` | The indexed properties, in order. A flat index of an index-only type leaves it out. From protocol version 14 a path may read through a reference, `"<reference property>.<field>"`, a value of the referenced document the document does not store. | 1 | [properties](contract-keywords/indexes.md#properties) · [Values of Referenced Documents](contract-keywords/derived-index-properties.md) · [internals](drive/indexes.md) |
| `unique` | boolean | No two documents share the indexed values. | 1 | [unique](contract-keywords/indexes.md#unique) |
| `nullSearchable` | boolean, default `true` | `false` leaves out documents whose indexed values are all null. | 1 | [nullSearchable](contract-keywords/indexes.md#nullsearchable) |
| `contested` | object | Matching values are decided by a masternode vote, not first come. | 1 | [Contested Indexes](contract-keywords/contested.md) · [internals](data-model/contested-documents.md) |
| `contested.resolution` | `0` vote with lock, `1` vote without lock | How the contest is decided. `1` from 14. | 1 | [The keys](contract-keywords/contested.md#the-keys) |
| `contested.fieldMatches` | `[{ "field", "regexPattern" }]` | Which values are contested. | 1 | [The keys](contract-keywords/contested.md#the-keys) |
| `contested.description` | string | A note; consensus ignores it. | 1 | [The keys](contract-keywords/contested.md#the-keys) |
| `countable` | `"notCountable"`, `"countable"`, `"countableAllowingOffset"` or boolean | Keeps a document count per indexed value. | 12 | [countable](contract-keywords/aggregates.md#countable) · [internals](drive/document-count-trees.md#per-index-countable-flag) |
| `summable` | property name | Keeps the sum of an integer property per indexed value. | 12 | [summable](contract-keywords/aggregates.md#summable) · [internals](drive/document-sum-trees.md#per-index-summable-flag) |
| `averageable` | property name | Shorthand for `countable` plus `summable`. | 12 | [averageable](contract-keywords/aggregates.md#averageable) |
| `rangeCountable`, `rangeSummable`, `rangeAverageable` | boolean | Provable counts, sums or averages over ranges of the indexed value. | 12 | [Index range keys](contract-keywords/aggregates.md#index-rangecountable-rangesummable-rangeaverageable) |
| `rankedCountable` | boolean or `{ "at": ... }` | Orders the indexed values by document count, for "top K" queries; `at` names the levels ranked. | 14 | [rankedCountable](contract-keywords/ranked.md#rankedcountable) · [internals](drive/document-ranked-trees.md#contract-grammar) |
| `rankedSummable`, `rankedAverageable` | boolean, or `{ "at": ... }` on a `summableOffCountIndex` index | Orders them by sum, or by average. | 14 | [Ranked Indexes](contract-keywords/ranked.md#rankedsummable) |
| `timeRange` | `{ "on", "range", "step", "phase", "ttl" }` | Buckets a system timestamp into time windows, for trending queries. | 14 | [Time-Range Indexes](contract-keywords/time-range.md) · [internals](drive/time-range-ttl.md) |
| `timeRange.on` | `"$createdAt"`, `"$updatedAt"`, `"$transferredAt"` | The timestamp to bucket: the index's first property. | 14 | [The keys](contract-keywords/time-range.md#the-keys) |
| `timeRange.range`, `.step` | seconds | Each window's length, and the time between window starts. | 14 | [The keys](contract-keywords/time-range.md#the-keys) |
| `timeRange.phase` | seconds, default 0 | Shifts the window boundaries. | 14 | [The keys](contract-keywords/time-range.md#the-keys) |
| `timeRange.ttl` | seconds, at most one week | Expires the index's entries after their window; on an index-only type, the rows leave this index. | 14 | [The keys](contract-keywords/time-range.md#the-keys) · [internals](drive/time-range-ttl.md#cleanup) |
| `integerRange` | `{ "on", "range", "step", "phase" }` | Buckets an integer property into value windows, for counts and rankings per band. | 14 | [Integer-Range Indexes](contract-keywords/integer-range.md) |
| `integerRange.on` | property name | The integer property to bucket: the index's first property, required. | 14 | [The keys](contract-keywords/integer-range.md#the-keys) |
| `integerRange.range`, `.step`, `.phase` | integers, `phase` default 0 | Each window's length, the distance between window starts, and the shift of the window boundaries. | 14 | [The keys](contract-keywords/integer-range.md#the-keys) |
| `terminal` | property name or list | On an index-only type, what keys each entry in place of the document id. | 14 | [terminal](contract-keywords/index-only.md#terminal) |
| `preallocated` | boolean | On an index-only type, creates the index's trees with the referenced document. | 14 | [preallocated](contract-keywords/index-only.md#preallocated) · [internals](drive/index-only-document-types.md#preallocated-index-paths) |
| `summableOffCountIndex` | index name | On an index-only type, keeps one counter per group of how many entries the named index keeps for it, in place of an entry per document: `count(*)` and the sum read the named index's entries, and the average divides them by the groups. | 14 | [summableOffCountIndex](contract-keywords/index-only.md#summableoffcountindex) |
| `outlivesDelete` | boolean | On an index-only type's time window with a `ttl`, a delete leaves the index's entries to expire, and a create writes over one already there. | 14 | [outlivesDelete](contract-keywords/index-only.md#outlivesdelete) · [internals](drive/index-only-document-types.md#entries-that-outlive-a-delete-outlivesdelete) |
| `skipIfAbsent` | `true` or property names | A document missing a property of the skip set writes no entry into the index. | 14 | [skipIfAbsent](contract-keywords/indexes.md#skipifabsent) · [internals](drive/index-only-document-types.md#conditional-participation-skipifabsent) |

### System properties

| Property | Holds | Recorded | Since | Read more |
|---|---|---|---|---|
| `$id` | The document's id. | always | 1 | [$id](contract-keywords/system-properties.md#id) |
| `$ownerId` | The identity that owns the document. | always | 1 | [$ownerId](contract-keywords/system-properties.md#ownerid) |
| `$revision` | 1 at creation, raised by every replace, transfer, price update and purchase. | on types whose documents can change hands or content | 1 | [$revision](contract-keywords/system-properties.md#revision) |
| `$createdAt`, `$updatedAt`, `$transferredAt` | Block times, in milliseconds, of the creation, the last replace or price update, and the last transfer or purchase. | when listed in `required` | 1 | [Timestamps](contract-keywords/system-properties.md#timestamps) |
| `$createdAtBlockHeight`, `$updatedAtBlockHeight`, `$transferredAtBlockHeight` | Platform block heights of the same events. | when listed in `required` | 1 | [Block heights](contract-keywords/system-properties.md#block-heights) |
| `$createdAtCoreBlockHeight`, `$updatedAtCoreBlockHeight`, `$transferredAtCoreBlockHeight` | Core chain block heights of the same events. | when listed in `required` | 1 | [Block heights](contract-keywords/system-properties.md#block-heights) |
| `$creatorId` | The identity that created the document. | on transferable or tradeable types of format-1 contracts | 10 | [$creatorId](contract-keywords/system-properties.md#creatorid) |
| `$moderatedAt`, `$moderatedBy` | Block time and moderator of the last write of the fields only moderators write. | on types listing `moderatorAbilities.changeFields`, once a moderator writes them | 14 | [$moderatedAt and $moderatedBy](contract-keywords/system-properties.md#moderatedat-and-moderatedby) |

## Limits

The first three limits come from the meta-schema, the rest from protocol version 14's `SystemLimits`. A contract over a limit is refused at registration.

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
| `max_contested_summed_value_magnitude` | 134217728 (2^27) | the `minimum` and `maximum` of a summed property on a type with a contested index |
| `max_expiring_signed_summed_value_magnitude` | 134217728 (2^27) | the `minimum` and `maximum` of a summed property that admits negative values, on a type with a `ttl` |
