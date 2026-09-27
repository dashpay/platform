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

A contract update that breaks an update rule is refused with one of two errors, depending on which check catches it. `DocumentTypeUpdateError` (40212) comes from the comparison of the parsed document types, which judges flags such as `documentsMutable` by their meaning. `IncompatibleDocumentTypeSchemaError` (10246) comes from the comparison of the two JSON schemas, which judges property keywords such as `refersTo` or `maxLength` by their text. Top-level `required` and `indices` have errors of their own (10276 and 10217).

## Protocol versions

The document meta-schema has changed three times:

| Meta-schema | Protocol versions | What it added |
|---|---|---|
| v0 | 1 to 11 | The original keywords. A document type key the meta-schema did not know was ignored. |
| v1 | 12 | Unknown document type keys are refused. The count, sum and average keywords. |
| v2 | 13 | `keepsTransferHistory`, `keepsPurchaseHistory`, `keepsPricingHistory`. |
| v3 | 14 | References, typed arrays, `requiredSince`, `immutable`, `ttl`, `propertyConstraints`, `actionFees`, moderation deletion, ranked and time-range indexes, index-only types, and the rest marked 14 in these chapters. |

Most keywords of v0 took effect at protocol version 1. The exceptions are `tokenCost` (9) and the index keyword `countable` (12).

## Every keyword

### Document type keywords

| Keyword | Chapter |
|---|---|
| `$comment`, `$defs`, `$schema`, `additionalProperties`, `dependentRequired`, `description`, `maxProperties`, `minProperties`, `properties`, `required`, `type` | [Document Shape](contract-keywords/document-shape.md) |
| `actionFees` | [Action Fees](contract-keywords/action-fees.md) |
| `canBeDeleted`, `canBeDeletedByModerators`, `canBeDeletedByModeratorsFor` | [Deletion](contract-keywords/deletion.md) |
| `creationRestrictionMode`, `tradeMode`, `transferable` | [Creation, Transfers and Trading](contract-keywords/ownership-and-trading.md) |
| `creatorRefersTo`, `ownerRefersTo` | [Writer and Creator References](contract-keywords/owner-refers-to.md) |
| `documentsAverageable`, `documentsCountable`, `documentsSummable`, `rangeAverageable`, `rangeCountable`, `rangeSummable` | [Counts, Sums and Averages](contract-keywords/aggregates.md) |
| `documentsKeepHistory`, `keepsPricingHistory`, `keepsPurchaseHistory`, `keepsTransferHistory` | [History](contract-keywords/history.md) |
| `documentsMutable`, `immutable`, `immutableAllowSetting` | [Mutability](contract-keywords/mutability.md) |
| `entryPayload`, `indexOnly` | [Index-Only Types](contract-keywords/index-only.md) |
| `indices` | [Indexes](contract-keywords/indexes.md) |
| `propertyConstraints` | [propertyConstraints](contract-keywords/property-constraints.md) |
| `requiresIdentityDecryptionBoundedKey`, `requiresIdentityEncryptionBoundedKey`, `signatureSecurityLevelRequirement` | [Signing and Keys](contract-keywords/signing-keys.md) |
| `tokenCost` | [Token Costs](contract-keywords/token-cost.md) |
| `transient` | [transient](contract-keywords/transient.md) |
| `ttl` | [Time To Live](contract-keywords/ttl.md) |

### Property keywords

| Keyword | Chapter |
|---|---|
| `$comment`, `$id`, `$ref`, `additionalProperties`, `byteArray`, `const`, `contains`, `contentMediaType`, `dependentRequired`, `description`, `enum`, `examples`, `exclusiveMaximum`, `exclusiveMinimum`, `format`, `maxItems`, `maxLength`, `maxProperties`, `maximum`, `minItems`, `minLength`, `minProperties`, `minimum`, `multipleOf`, `pattern`, `position`, `properties`, `required`, `type`, `uniqueItems` | [Property Schemas](contract-keywords/property-schemas.md) |
| `distinctFrom` | [distinctFrom](contract-keywords/distinct-from.md) |
| `encryptedFor` | [encryptedFor](contract-keywords/encrypted-for.md) |
| `items` | [Typed Arrays](contract-keywords/typed-arrays.md) |
| `maxBytes` | [maxBytes](contract-keywords/max-bytes.md) |
| `refersTo` | [References](contract-keywords/refers-to.md) |
| `requiredSince` | [requiredSince](contract-keywords/required-since.md) |

### Inside `refersTo`

| Keyword | Chapter |
|---|---|
| `contractId`, `contractRequirements`, `documentType`, `identityProperty`, `keyIdProperty`, `keyRequirements`, `propertyAgreement`, `type` | [References](contract-keywords/refers-to.md) |
| `lookup` | [Lookups](contract-keywords/refers-to-lookup.md) |
| `anyOf`, `allOf` | [Expressions](contract-keywords/refers-to-expressions.md) |
| `inList`, `listElement` | [List Elements](contract-keywords/refers-to-list-element.md) |

### Index keywords

| Keyword | Chapter |
|---|---|
| `name`, `nullSearchable`, `properties`, `unique` | [Indexes](contract-keywords/indexes.md) |
| `contested` | [Contested Indexes](contract-keywords/contested.md) |
| `averageable`, `countable`, `rangeAverageable`, `rangeCountable`, `rangeSummable`, `summable` | [Counts, Sums and Averages](contract-keywords/aggregates.md) |
| `rankedAverageable`, `rankedCountable`, `rankedSummable` | [Ranked Indexes](contract-keywords/ranked.md) |
| `timeRange` | [Time-Range Indexes](contract-keywords/time-range.md) |
| `preallocated`, `skipIfAbsent`, `terminal` | [Index-Only Types](contract-keywords/index-only.md) |

### System properties

`$id`, `$ownerId`, `$revision`, `$creatorId`, and the creation, update and transfer times and block heights: see [System Properties](contract-keywords/system-properties.md).

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
