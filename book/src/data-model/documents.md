# Documents

If data contracts are the tables, then **documents** are the rows. A document is an instance of a document type defined within a data contract. When a user creates a profile on DashPay, submits a domain name on DPNS, or stores any application data on the platform, they are creating a document.

Documents are the most fundamental unit of user data on Dash Platform. They are stored in GroveDB (through Drive), indexed for efficient querying, and cryptographically provable. Understanding how documents work at the Rust level is essential for working with the platform codebase.

## The Document Enum

Like `DataContract` and `Identity`, `Document` is a versioned enum. From `packages/rs-dpp/src/document/mod.rs`:

```rust
#[derive(Clone, Debug, PartialEq, From)]
pub enum Document {
    V0(DocumentV0),
}
```

Currently there is only one variant, `V0`. But the enum wrapper is already in place so that future protocol versions can introduce a `V1` variant without breaking existing code. All code that works with documents goes through the accessor traits, so adding a new variant is purely additive.

## What Lives Inside a Document

The `DocumentV0` struct is defined in `packages/rs-dpp/src/document/v0/mod.rs`:

```rust
pub struct DocumentV0 {
    pub id: Identifier,
    pub owner_id: Identifier,
    pub properties: BTreeMap<String, Value>,
    pub revision: Option<Revision>,
    pub created_at: Option<TimestampMillis>,
    pub updated_at: Option<TimestampMillis>,
    pub transferred_at: Option<TimestampMillis>,
    pub created_at_block_height: Option<BlockHeight>,
    pub updated_at_block_height: Option<BlockHeight>,
    pub transferred_at_block_height: Option<BlockHeight>,
    pub created_at_core_block_height: Option<CoreBlockHeight>,
    pub updated_at_core_block_height: Option<CoreBlockHeight>,
    pub transferred_at_core_block_height: Option<CoreBlockHeight>,
    pub creator_id: Option<Identifier>,
    pub contract_version: Option<u32>,
}
```

Let us walk through the key fields:

- **`id`**: A 32-byte unique identifier. Unlike contract IDs, document IDs are *derived* from a combination of the contract ID, owner ID, document type name, entropy and (protocol v14+) the identity contract nonce of the create transition. This makes them deterministic, unique, and impossible to produce twice.

- **`owner_id`**: The identity that currently owns this document. Ownership can change if the document type supports transfers.

- **`properties`**: The actual application data, stored as a `BTreeMap<String, Value>`. The `Value` type comes from `platform-value` and can represent strings, integers, byte arrays, nested maps, and arrays. The keys correspond to the property names defined in the document type's JSON Schema.

- **`revision`**: An `Option<Revision>` (which is a `u64`). Mutable documents track revisions -- each update increments the revision. Immutable document types will have `None` here.

- **Timestamps**: Six pairs of timestamp fields covering three events (creation, update, transfer) across three time references (milliseconds, block height, core block height). Whether these are populated depends on the document type schema -- if the schema requires `$createdAt`, the platform fills it in when the document is created.

- **`creator_id`**: The original creator of the document. This differs from `owner_id` when a document has been transferred to a new owner.

- **`contract_version`**: The data contract version this document's bytes conform to — the *contract version stamp* (protocol v14+, document serialization format 3). Drive assigns it whenever document content is supplied (create and replace) and preserves it through transfers and purchases. `None` means the document was serialized before format 3, which predates every `requiredSince` annotation. The stamp resolves per-property byte layouts when a document type gains required properties through contract updates — see the [Document Serialization](../serialization/document-serialization.md) chapter.

## Document ID Generation

Document IDs are not random -- they are derived deterministically. From `packages/rs-dpp/src/document/generate_document_id.rs`:

```rust
impl Document {
    pub fn generate_document_id_v1(
        contract_id: &Identifier,
        owner_id: &Identifier,
        document_type_name: &str,
        entropy: &[u8],
        identity_contract_nonce: IdentityNonce,
    ) -> Identifier {
        let mut buf: Vec<u8> = Vec::with_capacity(/* ... */);

        buf.extend_from_slice(DOCUMENT_ID_V1_DOMAIN_TAG); // b"dash:document-id:v1"
        buf.extend_from_slice(contract_id.as_slice());
        buf.extend_from_slice(owner_id.as_slice());
        buf.extend_from_slice(document_type_name.as_bytes());
        buf.extend_from_slice(entropy);
        buf.extend_from_slice(&identity_contract_nonce.to_be_bytes());

        Identifier::from(hash_double(&buf))
    }
}
```

The ID is a double SHA-256 hash of a domain tag, the contract ID, owner ID, document type name, client-provided entropy and the identity contract nonce of the create transition. `Document::generate_document_id` picks the derivation from the platform version; consensus recomputes it for every create and rejects a mismatch with `InvalidDocumentTransitionIdError`. This means:
- The ID commits to the owner, so nobody else can take it, and to the contract and document type, preventing cross-contract collisions.
- The ID is a deterministic function of its inputs: whoever knows the entropy and the nonce, which is the client building the create transition, can compute it before the document exists (see below). The entropy is the one input other parties can not guess, so as long as the client generates it unpredictably, nobody else can compute the ID of a document that has not been sent yet and point other documents at it in advance.
- The nonce makes the ID single use. An identity contract nonce is consumed at most once, so an ID can be produced at most once.

### Why the nonce is part of the ID

Up to protocol version 13 the ID was `generate_document_id_v0`: the same hash without the domain tag and the nonce. The create check only asks whether a document exists under the ID *right now*, so the owner of a deleted document could create a new document with the same entropy and get the same ID back, with different content. Everything that referenced the ID (likes, replies, a `refersTo` property, a moderation removal record) then pointed at the new content. For a document type with `documentsMutable: false` and `canBeDeleted: true` that is content substitution, the very thing immutability is supposed to rule out.

From protocol version 14 a reference to a document ID means that one document or nothing. This also holds for documents created before the upgrade: their entropy-only IDs can not be produced by the new derivation, and the old derivation is no longer accepted.

### What this means for clients

The ID of a new document only exists once the nonce of its create transition is assigned, and it changes if the transition is rebuilt with another nonce:
- The ID a `Document` carries before its create transition is built (for example the one `create_document_from_data` gives it) is a **placeholder**. `DocumentCreateTransitionV0::from_document` replaces it with the derived ID, so every transition built through dpp carries the right one.
- On the Rust path (rs-sdk, or `from_document` directly) the `Document` you passed in keeps its placeholder: read the ID from the transition, or from the confirmed document `put_to_platform_and_wait_for_response` returns.
- To know IDs up front (a chain of documents that reference each other), assign the nonces first: nonces may be used out of order within a window of 24.
- JavaScript gets the same through `wasm-dpp2`, with one difference: `new DocumentCreateTransition({ document, identityContractNonce })` derives the ID for the network's protocol version (`platformVersion` option, latest by default) and writes it both onto the transition and back onto `document`, so after construction `document.id` is the final ID and may be read from there. `Document.generateId(type, owner, contract, entropy, identityContractNonce)` and `document.setIdForCreation(identityContractNonce)` give the ID before the transition exists, and `new Document({ ..., identityContractNonce })` derives it at construction (an explicit `id` passed alongside the nonce must equal the derived one). A `Document` built without a nonce carries the entropy-only placeholder until it is passed to `DocumentCreateTransition`. No app needs to reimplement the hash.

## The Accessor Traits

Documents follow the same accessor-trait pattern as data contracts. The getter trait is defined in `packages/rs-dpp/src/document/accessors/v0/mod.rs`:

```rust
pub trait DocumentV0Getters {
    fn id(&self) -> Identifier;
    fn owner_id(&self) -> Identifier;
    fn properties(&self) -> &BTreeMap<String, Value>;
    fn properties_mut(&mut self) -> &mut BTreeMap<String, Value>;
    fn revision(&self) -> Option<Revision>;
    fn created_at(&self) -> Option<TimestampMillis>;
    fn updated_at(&self) -> Option<TimestampMillis>;
    fn transferred_at(&self) -> Option<TimestampMillis>;
    fn created_at_block_height(&self) -> Option<u64>;
    fn updated_at_block_height(&self) -> Option<u64>;
    fn creator_id(&self) -> Option<Identifier>;
    fn contract_version(&self) -> Option<u32>;
    // ... and more
}
```

The setter trait extends it with mutation methods and also provides convenient typed setters:

```rust
pub trait DocumentV0Setters: DocumentV0Getters {
    fn set_id(&mut self, id: Identifier);
    fn set_owner_id(&mut self, owner_id: Identifier);
    fn set_properties(&mut self, properties: BTreeMap<String, Value>);
    fn set_revision(&mut self, revision: Option<Revision>);
    fn set_created_at(&mut self, created_at: Option<TimestampMillis>);
    fn set_updated_at(&mut self, updated_at: Option<TimestampMillis>);

    // Generic property access via path syntax
    fn set(&mut self, path: &str, value: Value) { ... }
    fn remove(&mut self, path: &str) -> Option<Value> { ... }

    // Typed setters for common types
    fn set_u8(&mut self, property_name: &str, value: u8);
    fn set_u64(&mut self, property_name: &str, value: u64);
    fn set_bytes(&mut self, property_name: &str, value: Vec<u8>);
    // ... and more
}
```

Notice the `set()` method provides lodash-style path syntax: `"root.people[0].name"`. Parents are created automatically if they do not exist.

## The DocumentMethodsV0 Trait

Beyond simple field access, documents have behavior defined by the `DocumentMethodsV0` trait in `packages/rs-dpp/src/document/document_methods/mod.rs`:

```rust
pub trait DocumentMethodsV0 {
    fn get_raw_for_contract(
        &self,
        key: &str,
        document_type_name: &str,
        contract: &DataContract,
        owner_id: Option<[u8; 32]>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Vec<u8>>, ProtocolError>;

    fn get_raw_for_document_type(
        &self,
        key_path: &str,
        document_type: DocumentTypeRef,
        owner_id: Option<[u8; 32]>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Vec<u8>>, ProtocolError>;

    fn hash(
        &self,
        contract: &DataContract,
        document_type: DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, ProtocolError>;

    fn increment_revision(&mut self) -> Result<(), ProtocolError>;

    fn is_equal_ignoring_time_based_fields(
        &self,
        rhs: &Self,
        also_ignore_fields: Option<Vec<&str>>,
        platform_version: &PlatformVersion,
    ) -> Result<bool, ProtocolError>;
}
```

The `get_raw_for_contract` and `get_raw_for_document_type` methods retrieve a document property as raw bytes, using the document type schema to determine how to serialize the value. This is critical for building index keys and storage operations.

The `is_equal_ignoring_time_based_fields` method is particularly useful in validation. Since timestamps and block heights are set by the network (not the client), you often want to compare two documents while ignoring those fields -- for example, to verify that a client's update only changed the fields it was supposed to change.

## Version Dispatching in Methods

Every method in the `Document` implementation dispatches through the platform version, following the standard pattern:

```rust
impl DocumentMethodsV0 for Document {
    fn get_raw_for_contract(
        &self,
        key: &str,
        document_type_name: &str,
        contract: &DataContract,
        owner_id: Option<[u8; 32]>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Vec<u8>>, ProtocolError> {
        match self {
            Document::V0(document_v0) => {
                match platform_version
                    .dpp
                    .document_versions
                    .document_method_versions
                    .get_raw_for_contract
                {
                    0 => document_v0.get_raw_for_contract_v0(
                        key, document_type_name, contract,
                        owner_id, platform_version,
                    ),
                    version => Err(ProtocolError::UnknownVersionMismatch {
                        method: "DocumentMethodV0::get_raw_for_contract".to_string(),
                        known_versions: vec![0],
                        received: version,
                    }),
                }
            }
        }
    }
}
```

This is a double dispatch: first on the document variant (V0), then on the method version from the platform version configuration. This allows the platform to evolve both the document structure and the behavior of document methods independently.

## How Documents Reference Their Contract

Documents do not carry a reference to their contract inside the struct itself. Instead, the relationship is established through context -- the document type name and contract are passed alongside the document whenever they are needed (for serialization, validation, hashing, and storage).

When serializing, a document is always serialized *relative to its document type*:

```rust
let serialized = <Document as DocumentPlatformConversionMethodsV0>::serialize(
    &document,
    document_type,  // the schema determines field order and encoding
    &contract,
    platform_version,
)?;

let deserialized = Document::from_bytes(
    &serialized,
    document_type,  // same schema needed for decoding
    platform_version,
)?;
```

This means a document's binary representation is *not self-describing*. You need the document type definition to interpret the bytes. This is a deliberate design choice for storage efficiency -- field names are not repeated in every serialized document.

## The INITIAL_REVISION Constant

When a new document is created, it starts at revision 1:

```rust
pub const INITIAL_REVISION: u64 = 1;
```

Revision 0 is never used for active documents. This allows `0` to serve as a sentinel value meaning "no revision" in some contexts.

## Document References (`refersTo`)

From protocol version 14 a property of a document type can declare what it points at, and consensus refuses a create or replace whose target does not exist when the document is written (the reference is a write-time constraint only; nothing resolves it for a reader). The keyword is `refersTo` on the property, its `type` one of `identity`, `contract` (optionally with `contractRequirements`, see [Contract Moderation](contract-moderation.md)), `token`, `permanentDocument`, `deletableDocument` and `identityPublicKey`. Every form sits on an identifier property, with one exception below. The parsed shape is `DocumentPropertyType::IdentifierWithReference(target)`, and any change to a declaration on contract update is an incompatible schema change.

An `identityPublicKey` reference names one key of one identity, and comes in two forms that differ in which property carries what:

- **On the identity property.** The identifier property carries the identity id and `keyIdProperty` names the sibling integer property carrying the key id: `"refersTo": { "type": "identityPublicKey", "keyIdProperty": "toKeyIndex" }`. Consensus fetches the named key of that identity.
- **On the key id property.** The integer property carries the key id and `identityProperty` names whose key it is: `"refersTo": { "type": "identityPublicKey", "identityProperty": "$ownerId" }`. The property must declare exactly the range of a `KeyID` (`"type": "integer", "minimum": 0, "maximum": 4294967295`) and the declaration takes no `keyIdProperty`. `identityProperty` is `$ownerId` (the writer), `$creatorId` (the document's creator, only on a document type that records creator ids: a transferable or tradeable type of a format-1 contract) or the path of an identifier property of the same document type (which must exist, be an identifier and not carry an `identityPublicKey` reference of its own); the last two are checked at contract registration. `keyRequirements` sit on this form exactly as on the identifier form. The parsed shape is `DocumentPropertyType::KeyIdWithReference(KeyIdReference)`, the identity source plus the requirements, sized, encoded and queried exactly as a plain `u32`.

```json
"senderKeyId": {
  "type": "integer", "minimum": 0, "maximum": 4294967295,
  "refersTo": { "type": "identityPublicKey", "identityProperty": "$ownerId" },
  "position": 3
}
```

Both forms share the state check (`validate_referenced_identity_key_v0` in the document reference validation): the key must exist and not be disabled, else the write is refused, paid, with `ReferencedIdentityKeyNotFoundError` (40123) or `ReferencedIdentityKeyDisabledError` (40124). Identity keys can be disabled but never removed, so a validated reference never dangles. The owner form's identity is the transition's signer, which the transition already proved exists, so the key fetch is its only read; a key id of some other identity's key is meaningless by construction, there is no property to name another identity. On replace the identity form is re-validated when either the identity property or its key id property changed. For the key id form it depends on where the identity comes from. `$ownerId` is the writer, transition metadata that never appears among the changed fields, and the document may have changed hands since the key id was written, so the reference is re-validated on every replace, touched or not, as the `$ownerId` writer gate is: after a transfer the new owner has to repoint the key id at one of its own keys. `$creatorId` never changes, so it is re-validated when the key id changed. A property path is re-validated when the key id or that property changed, and a key id set while the property is not is refused (`ReferencedKeyIdPropertyInvalidError`, 40125). A transfer itself is not checked in any form, so the reference governs writing, not holding. The declaring property must carry exactly the key id range in its schema, whatever the contract's integer sizing setting, and a `keyIdProperty` of the identity form may not name a property that carries this form, nor may a path name an identifier carrying an `identityPublicKey` reference: one pair is declared once (40125 at registration). The charter contract's `joinRequest.senderKeyId`, the owner's encryption key a shared secret is derived from, is the first user.

### Resolved through a unique index (`lookup`)

A `permanentDocument` reference normally holds the referenced document's id. It may instead carry a `lookup`: the property's value, or on the elements of a typed array each element (see [References on the Elements](#references-on-the-elements)), is then one part of a key, and the referenced document is the one a unique index of the referenced document type finds for that key. The reference holds if that document exists. The moderation charter's `members` is the first user:

```json
"members": {
  "type": "array", "minItems": 0, "maxItems": 15, "uniqueItems": true,
  "items": {
    "type": "array", "byteArray": true, "minItems": 32, "maxItems": 32,
    "contentMediaType": "application/x.dash.dpp.identifier",
    "distinctFrom": "$ownerId",
    "refersTo": {
      "type": "permanentDocument",
      "documentType": "joinRequest",
      "lookup": {
        "index": "bySubmittedCharter",
        "keys": { "submittedCharterId": "submittedCharterId", "$ownerId": "." }
      }
    }
  },
  "position": 2
}
```

reads: every member must be the owner of a `joinRequest` whose `submittedCharterId` equals this document's `submittedCharterId`. Without `lookup` the list would have to hold the join requests' ids, which the writer would have to find first, and which say nothing about who asked to join. The same form works on a scalar identifier property, where `"."` is the property's own value.

A `deletableDocument` reference takes no `lookup`. Once the document a key found is deleted, a new document with the same key would make the reference resolve again, to different content, where an id is produced at most once and a dead id reference stays dead.

`index` names an index of the referenced document type. `keys` maps every property of that index, by its name on the referenced side (system ones such as `$ownerId` included), in any order, to where its value comes from on the referring side:

- a property path of the referring document type (`"submittedCharterId"`, `"meta.charterId"`);
- `"$ownerId"`, the referring document's owner, the writer;
- `"."`, the value of the property that carries the reference, or the element. It appears exactly once: without it every value would resolve to the same document.

What is checked when the contract enters the chain, on registration and on update:

- `lookup` is only allowed on `permanentDocument` references (meta-schema v3 and the parser, `apply_property_reference` 0), on the property or on the `items` of a typed array.
- Each property a key reads must exist on the referring type, be required (and so must every object around it), not be transient, and hold a single value, so a lookup never runs with a missing key part and a reader can assemble the same key from the stored document. A key that reads `"$ownerId"` needs a referring type whose documents can be neither transferred nor traded: the reference is judged when the document is written, and a transfer or purchase would move the writer part of its key without a write. These are properties of the referring type alone and are checked on every parse (generation 3).
- The index must exist and be unique, so the key finds at most one document; it may not bucket a timestamp with `timeRange`, and the referenced type may not be `indexOnly`. `keys` must cover each property of the index exactly once and nothing else, and each source must hold the same kind of value as the index property it fills (the rule of `propertyAgreement`, `DocumentPropertyType::value_kind`).
- The key must stay with the document it found, or the reference could dangle without the document being deleted: every schema property of the index must be fixed once written (the referenced type is immutable, or the property, or the top-level object holding it, is listed under `immutable`), `$ownerId` is only a key part on a type whose documents can be neither transferred nor traded, and the update and transfer times are refused where a replace, transfer or purchase moves them. `$id`, `$creatorId` and the creation times are always fixed.
- A changed, added or removed `lookup` is an incompatible schema change on update, like the rest of a `refersTo`.

The checks on the referenced type run where that type is in hand. For a document type of the same contract the contract parse runs them under full validation (`create_document_types_from_document_schemas` 1, next to the `keyRequirements.boundTo` check), once every document type is parsed; a deletable target is left to registration, which refuses it for the `permanentDocument` reference (40122). For a type of another contract (`contractId`) the registration state validation runs them against that contract, where the other `refersTo` checks into another contract run, and refuses a declaration that cannot resolve with `ReferencedDocumentLookupInvalidError` (state code 40137). Index definitions cannot change on a contract update from protocol version 14 (`validate_update` 1 compares them by name), and neither can the flags the permanence rule reads, so the answer holds.

When the referring document is created or replaced, the document reference validation assembles the key for each value and queries the index for at most one document, billed as a document fetch of the same kind as the id lookup (`fetch_document_through_lookup`). No document, or a key it cannot assemble, refuses the write, paid, with `ReferencedEntityNotFoundError` (40120) naming the property, or the element by its list path (`members[1]`); its target reads "found through unique index `<index>`". A `propertyAgreement` beside the `lookup` is checked against the document the index found, exactly as for an id reference. A replace re-validates the reference when the property itself changed (for a list, the elements the stored list did not hold), and every value, every element included, when a property a key reads changed. Nothing else can move a key part: the writer is fixed on a type allowed to read it, and the referenced side's key is fixed by the rule above, so a validated lookup reference never dangles.

Joins cannot go through a lookup reference: a chained query or a composite by-id join needs the join property's values to be the outer documents' ids, so both refuse such a property, and a `preallocated` index cannot be bound through one. In Rust the declaration is its own variant, `DocumentPropertyReferenceTarget::PermanentDocumentLookup`, appended to the enum rather than a field of `PermanentDocument`: the enum is embedded in the reference errors, so an id reference keeps its encoding, and code matching `PermanentDocument` as "the value is a document id" cannot mistake a lookup for one. The rules are on `DocumentReferenceLookup`. `as_document_reference` returns only references whose value is a document id, the accessor for joins; the validators use `as_any_document_reference`, whose declaration carries the lookup.

## Immutable Properties on Mutable Document Types

A document type either allows replaces (`documentsMutable: true`, the default) or freezes its documents entirely. Protocol version 14 adds a middle ground: the doctype-level `immutable` keyword lists top-level properties that are frozen at creation while the rest of the document stays replaceable.

```json
"post": {
  "type": "object",
  "documentsMutable": true,
  "properties": {
    "author": { "type": "string", "maxLength": 63, "position": 0 },
    "body": { "type": "string", "maxLength": 500, "position": 1 }
  },
  "required": ["author", "body"],
  "immutable": ["author"],
  "additionalProperties": false
}
```

A second list, `immutableAllowSetting`, relaxes the first for optional properties that are not known at creation: a property listed there may still be set by a replace while the stored document has no value for it, and is frozen from then on (it can neither change nor be removed). Every entry must also be in `immutable`.

```json
"immutable": ["author", "mood"],
"immutableAllowSetting": ["mood"]
```

The parser (generation 3, meta-schema v3) checks both lists when a contract enters the chain:

- Every `immutable` entry names a declared top-level property. System properties (`$`-prefixed) are refused because the platform manages them, and nested paths are refused: list the containing object to freeze it whole, nested values included. A `transient` property is refused too: it is never stored, so once frozen it could never be written, and combined with `required` no replace could pass at all.
- The replace compares stored and supplied values by underlying data, recursing into objects regardless of member order and into arrays position by position, with integer widths ignored. Storage reorders object members by schema position and narrows integers, so a byte-for-byte comparison would flag an untouched object as changed.
- The lists are only allowed when `documentsMutable` is true. On an immutable document type every property is already frozen.
- Every `immutableAllowSetting` entry is also in `immutable`; on its own the allowance means nothing.
- On contract update `immutable` may gain entries but never lose one, and `immutableAllowSetting` may lose entries but only gain one for a property that becomes immutable in the same update (`DocumentTypeUpdateError` otherwise). Each rule keeps the promise documents were created under: nothing frozen becomes editable, and nothing already frozen starts accepting a late set. The schema compatibility differ strips both keys, like `indices` and `required`, so `validate_update` v1 is the single judge.

Enforcement lives in the replace action's state validation (generation 1). The action already records which top-level properties differ from the stored document in `changed_data_fields` (the same set that scopes `refersTo` re-validation), and alongside it which of those the stored document had no value for (`added_data_fields`). A changed property in the type's `immutable_fields()` fails the replace with `DocumentImmutablePropertyChangedError` (state code 40128) unless it is in `immutable_fields_allow_setting()` and was absent before. "Differ" covers a changed value, a property the stored document lacked, and a property the replace dropped. Transfers, price updates and purchases carry no property data and are unaffected.

In Rust the lists are `DocumentTypeV2Getters::immutable_fields()` and `immutable_fields_allow_setting()`. Earlier document type generations return empty sets.

## Typed Arrays

Up to protocol version 13 a `type: "array"` property had to be a byte array (`byteArray: true`). Protocol version 14 adds typed arrays: a list whose `items` schema says what every element is.

```json
"reasons": {
  "type": "array",
  "minItems": 0,
  "maxItems": 64,
  "uniqueItems": true,
  "items": {
    "type": "array", "byteArray": true, "minItems": 32, "maxItems": 32,
    "contentMediaType": "application/x.dash.dpp.identifier"
  },
  "position": 2
}
```

- An element is a scalar: an integer, a number, a string (with `minLength` / `maxLength`), a boolean, a byte array (`byteArray: true`, whose `minItems` / `maxItems` count bytes) or an identifier. Objects and arrays of arrays are refused. An identifier element may carry `refersTo` (see [References on the Elements](#references-on-the-elements)). An element may be limited to allowed values with `enum`; `const` is refused on elements, since a one-value `enum` does the same and a contract update can still widen it. The parser reads an element's `enum`, `minimum` and `maximum` onto the typed array (`ArrayItemConstraints`), refusing on both parse paths an `enum` with no member or a member of another type, an `enum` on a byte array or identifier element, and a `minimum` above the `maximum`, so random document generation stays inside them; the JSON schema validator enforces them on every document.
- On the array itself `minItems` and `maxItems` count elements, not bytes. `maxItems` is required, `minItems` may not exceed it and `contentMediaType` belongs on the items; these hold on every parse. Contract registration also caps `maxItems` at `SystemLimits::max_typed_array_items` (1024), so a typed array's worst-case size, which fee estimation charges by, stays small. `uniqueItems: true` refuses a document that repeats an element.
- A byte array keeps its form and takes no `items`. On a plain byte array `uniqueItems` keeps its old meaning, no repeated byte, but an identifier (a byte array with the identifier `contentMediaType`) refuses it: an identifier is one value, and "no repeated byte" would refuse most of them.
- The document is validated against the JSON schema as always, so a list that is too long, too short, repeats an element under `uniqueItems` or holds a wrong-typed element fails with the usual `JsonSchemaError`.

The array is stored inline in the document, like any other property: a varint element count followed by the elements, each encoded exactly as a required property of the element's type (see [Document Serialization](../serialization/document-serialization.md)). The `reasons` list above is therefore one count byte and 32 raw bytes per identifier, and an integer element bounded `0`..`100` takes one byte. Since the stored bytes depend on the element's type, a contract update may not change how an element encodes: raising an integer element's `maximum` (or adding an `enum` value) past its width, or unpinning a fixed-size byte array element, is refused with `DocumentTypeUpdateError`. A longer `maxLength`, a larger `maxItems` or a raised `maximum` that keeps the width are accepted. Identifier and byte array elements are conversion paths (`reasons[]`, `find_identifier_and_binary_paths` 1), so a document built from JSON or a value map converts every element, as it converts a scalar identifier. `ExtendedDocument::set_untrusted` converts every member of a list set at such a path. From protocol version 14 a property name and a document type name are word characters only (`^[a-zA-Z0-9_]{1,64}$`): every earlier generation also admitted `-`, which the path syntax (`a.b`, `list[]`) was never written for, and a census of every contract on mainnet and testnet found none using it. Nothing is written per element, so a typed array cannot be an index property (`InvalidIndexPropertyTypeError`), an indexOnly terminal or entry payload property, or one side of a `propertyAgreement`.

In Rust a typed array parses to `DocumentPropertyType::TypedArray(TypedArrayProperty)`, whose `item_type` is the `DocumentPropertyType` the `items` schema parses to as a property schema (`try_from_value_map` with the contract's parsing options). The parse is the versioned `parse_typed_array` (`None` before protocol version 14, where an array that is not a byte array is refused as it always was). The older `DocumentPropertyType::Array` variant, whose elements are an `ArrayItemType` in their own length-prefixed encoding, is never produced by the parser.

### References on the Elements

An identifier element may carry a `refersTo` declaration, which every element of the list then declares. The moderation charters' `reasons` refers to `reason` documents this way:

```json
"items": {
  "type": "array", "byteArray": true, "minItems": 32, "maxItems": 32,
  "contentMediaType": "application/x.dash.dpp.identifier",
  "refersTo": { "type": "permanentDocument", "documentType": "reason" }
}
```

- The declaration takes every target and key a single identifier property's `refersTo` takes (`identity`, `contract` with `contractRequirements`, `token`, and `permanentDocument` and `deletableDocument` with `contractId`, `documentType` and `propertyAgreement`), with the same checks at contract registration, except `identityPublicKey` in either form: its `keyIdProperty` names one sibling key id, and the `identityProperty` form sits on the key id itself, neither of which can pair with many elements, so an element may not declare it. The declaration belongs on the `items`; on the array itself it is refused.
- When a document is created or replaced each element is checked as a single reference is, in list order: the target must exist, a referenced contract must meet the `contractRequirements`, a referenced document's type must be deletable or not as declared, and each `propertyAgreement` pair must hold. The referring side of a pair is still a property of the referring document or its `$ownerId`, the same for every element; the referenced side is a property of that element's referenced document. The first element that fails refuses the write with the error a single reference gives (`ReferencedEntityNotFoundError` 40120, `ReferencedDocumentPropertyMismatchError` 40127, `ReferencedContractRequirementNotMetError` 40135 and so on), whose `path` names the element by its list path: `reasons[2]` for the third. An empty or absent list checks nothing. Registration errors name the declaration `submittedCharter.reasons[]`.
- A replace follows the rules of a single reference, element by element. A changed list re-validates the elements the stored list did not hold (the replace action carries the stored value of each changed property, `stored_changed_values`); the ones it held are unchanged references and are left alone, as an unchanged single reference is. Every element is re-validated when a property bound by a `propertyAgreement` changed, and on every replace when an agreement is keyed by `$ownerId` or the elements are `deletableDocument` references. An element repeating an earlier one of the same list is not fetched again.
- Every read is billed as a single reference's is. A foreign contract holding the referenced document type is fetched once per list, not once per element. Contract registration caps the references one document of a type can carry at `SystemLimits::max_references_per_document` (256), counting one per property with `refersTo` (an identifier, or a key id carrying a key reference) and `maxItems` per typed array of referencing elements: `maxItems` alone would let a type declare many lists of up to 1024 references each, and each one is a read when a document is written.
- An `immutable` property may not hold a `deletableDocument` reference a replace could not clear: a typed array of them, at the top level or inside an immutable object, or a single one inside an immutable object. Every replace re-validates them, so once a target is deleted the property would have to change, which an immutable property cannot. A single `deletableDocument` reference that is itself the immutable top-level property has a way out, a replace may remove it once its target is gone, and that exception reads the one identifier the removed top-level property held.
- A changed element `refersTo` is an incompatible schema change on contract update, as a changed `refersTo` on a scalar is.

In Rust the element parses to `DocumentPropertyType::IdentifierWithReference(target)` inside `item_type`, through the same versioned `apply_property_reference` a scalar identifier goes through. `DocumentPropertyType::reference()` reports a property's declaration as `PropertyReference::Value(target)` for a scalar and `PropertyReference::Elements(target)` for a typed array; the registration check (`validate_data_contract_references`) and the write-time check (`validate_document_references`) both enumerate references through it.

## Distinct Identifier Properties

Protocol version 14 adds the property-level `distinctFrom` keyword, a pure structure rule on identifier properties: the property's value must differ from the value of a named property of the same document, or from the document's `$ownerId`. It sits next to the reference keywords (`refersTo` and its `propertyAgreement`, which bind a property to another document's values) but reads nothing beyond the transition being written.

```json
"delegateId": {
  "type": "array", "byteArray": true, "minItems": 32, "maxItems": 32,
  "contentMediaType": "application/x.dash.dpp.identifier",
  "distinctFrom": "$ownerId",
  "position": 0
},
"backupId": {
  "type": "array", "byteArray": true, "minItems": 32, "maxItems": 32,
  "contentMediaType": "application/x.dash.dpp.identifier",
  "distinctFrom": "delegateId",
  "position": 1
}
```

The value is `"$ownerId"` or the dotted path of another property of the document type (`"meta.reviewerId"` for a nested one). The parser (generation 3, meta-schema v3) checks the declaration when a contract enters the chain, on registration and on update:

- The keyword is only allowed on identifier properties, enforced by the same dependent schema shape that restricts `refersTo`.
- A named property must exist on the document type, must itself be an identifier (the only kind the value can be compared with), and must not be the declaring property. `$ownerId` needs no check; no other system property is accepted.
- On contract update a changed, added or removed `distinctFrom` is an incompatible schema change, like a changed `refersTo`.
- A typed array of identifiers declares it on its `items`, and every element must then differ from the named value; the declaration is refused on the array itself and on elements of any other type.

Enforcement lives in the structure validation of the document create and replace actions (create structure generation 1, introduced at protocol version 14, and replace structure generation 0, extended in place: the call is inert before 14, where no property can carry the keyword), after the schema validation of the document's properties, so every value compared is already a 32-byte identifier. The check reads the transition's data and the owner id it carries and never touches Drive; the declaring properties come from a list the parser built (`distinct_from_fields`), so a type without declarations costs nothing. An equal pair fails the write with `DocumentPropertyNotDistinctError` (basic code 10419), which names the document type, the property and what it collided with. When the named property is absent from the document there is nothing to differ from, so the rule passes.

A transfer or purchase changes `$ownerId` without touching the data, so the transfer and purchase structure validations (generation 0, extended in place: the call is inert before protocol version 14, where no property can carry the keyword) judge the stored document against its new owner: a transfer to, or a purchase by, the identity a `$ownerId`-distinct property names is refused with the same error. Price updates change neither owner nor data and are not judged.

In Rust the declaration is `DocumentProperty::distinct_from` (`Option<DistinctFrom>`, absent on every property parsed before protocol version 14), the document check is `DocumentTypeV0Methods::validate_distinct_from_properties`, and `DistinctFrom::violation` judges one value on its own, which is how the elements of a typed array are judged one by one.

## Encrypted Properties (`encryptedFor`)

A byte array property may hold ciphertext that only one identity can read. Before protocol version 14 the contract said nothing about it, so every wallet had to learn the recipe (whose keys, which scheme, where the IV sits) from documentation or a side channel. From protocol version 14 the property declares it with the `encryptedFor` keyword, and wallets and SDKs read the recipe from the contract.

```json
"encryptedMessage": {
  "type": "array", "byteArray": true, "minItems": 32, "maxItems": 1040,
  "encryptedFor": {
    "recipient": "recipientId",
    "recipientKey": "recipientKeyId",
    "senderKey": "senderKeyId",
    "scheme": "ecdh-secp256k1-aes256-cbc"
  },
  "position": 4
}
```

All four keys are required. `recipient` is the dotted path of an identifier property of the same document type whose value is the recipient identity's id, or `$ownerId` for a message the writer encrypts to themself. `recipientKey` and `senderKey` are dotted paths of integer properties of the same document type carrying the recipient's and the sender's identity key ids; each must declare `minimum` at least 0 and `maximum` at most 4294967295, read from the schema itself, so the rule holds whatever `sizedIntegerTypes` the contract sets. `scheme` is a closed set with one member today.

The parser (generation 3, meta-schema v3) admits the keyword on byte array properties only, never on an identifier (`contentMediaType` set) or any other type, and checks at contract registration that the three named properties exist with those types, that none of them is `transient` (a transient property is stripped before storage, which would leave the stored ciphertext without its recipe), and that the byte array's own `maxItems` can hold the scheme's shortest ciphertext. A contract update that adds, removes or changes an `encryptedFor` declaration is an incompatible schema change (`IncompatibleDocumentTypeSchemaError`, 10246): documents already written under the old recipe could not be read under the new one. Contracts parsed before protocol version 14 ignore the keyword entirely.

### The `ecdh-secp256k1-aes256-cbc` layout

This is the scheme the dashpay contact request already uses for `encryptedPublicKey` and `encryptedAccountLabel` (DIP-15), implemented in `packages/rs-platform-encryption`:

1. The shared key is the libsecp256k1 ECDH of the sender's private key and the recipient's public key: `SHA256(parity || x)` of the product point, where `parity` is `0x02` for an even `y` and `0x03` for an odd one. The sender's key is the identity key with the id the `senderKey` property carries, on the document's `$ownerId` identity; the recipient's key is the one with the id the `recipientKey` property carries, on the identity the `recipient` property names (the owner itself for `$ownerId`). Either side derives the same 32 bytes from its own private key and the other's public key.
2. The writer draws a random 16-byte IV.
3. The value is the IV followed by the plaintext encrypted with AES-256-CBC under the shared key and that IV, with PKCS7 padding.

So a ciphertext is `16 + 16 * ceil((len(plaintext) + 1) / 16)` bytes: at least 32, always a multiple of 16. The reader splits the first 16 bytes off as the IV, derives the shared key from its own private key and the sender's public key, and decrypts the rest.

### What consensus checks, and what it cannot

Consensus sees bytes, not keys. On every document create and replace, after the JSON schema validation of the document's properties, the structure validation (create structure generation 1, introduced at protocol version 14, and replace structure generation 0, extended in place: the call is inert before 14, where no property can carry the keyword) walks the document type's declared properties and, for each one the transition supplies, checks that its length is at least the scheme's IV plus one block and a multiple of the block length. A value that is not refuses the transition with `InvalidEncryptedPropertyShapeError` (basic code 10420), which names the property path, the scheme and the lengths involved. No state is read; the check runs in the mempool as well as in the block. The JSON schema's own `minItems` and `maxItems` are checked first, so a value outside them (a lone 16-byte IV against `minItems: 32`, say) is refused with the schema's error rather than 10420; the shape check only sees values the bounds already admit.

Nothing else is verifiable on chain: not that the bytes decrypt, not that they decrypt under the keys the document names, not that the named key ids exist on the identities or have an encryption purpose, and not that the plaintext is what the document type means it to be. A writer can store any 32 bytes. Whether the keys exist and are of the right kind is what the reference keywords are for: a `refersTo` of type `identityPublicKey` with `keyIdProperty` on the recipient property makes consensus check that the recipient's key exists, and `encryptedFor` neither duplicates nor requires it. Readers must treat a value that fails to decrypt as a bad message, not as a protocol violation.

In Rust the declaration is `DocumentProperty::encrypted_for` (`Option<EncryptedFor>`), listed per document type by `DocumentTypeV0Getters::encrypted_properties()`, and the shape check is `DocumentTypeBasicMethods::validate_encrypted_property_shapes()`, versioned on the `validate_encrypted_property_shapes` method slot (`None` before protocol version 14, which is what keeps the in-place replace call inert). In JavaScript, `contract.documentTypeEncryptedProperties(name)` and `contract.documentEncryptedProperties` expose the same declarations, and the shape error reaches an app as `DocumentEncryptionErrorCode.InvalidEncryptedPropertyShape`.

## Rules and Guidelines

**Do:**
- Always serialize and deserialize documents using their document type definition. The type determines field layout.
- Use `is_equal_ignoring_time_based_fields()` when comparing documents for validation purposes.
- Use `increment_revision()` rather than manually manipulating the revision field -- it handles overflow checking.
- Access properties through the accessor traits, not by reaching into the inner `DocumentV0` struct.

**Do not:**
- Assume a document carries its contract reference. The contract and document type are always passed as separate arguments.
- Manually construct document IDs. Use `Document::generate_document_id()` with proper entropy, the nonce of the create transition and the network's platform version.
- Rely on the ID of a document that has not been sent yet. It is a placeholder until the create transition is built.
- Treat serialized document bytes as self-describing. Without the document type schema, the bytes are meaningless.
- Set time-based fields from client code. The platform sets `created_at`, `updated_at`, block heights, and similar fields during state transition processing.
