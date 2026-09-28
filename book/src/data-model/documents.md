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

From protocol version 14 a property of a document type can declare what it points at, and consensus refuses a create or replace whose target does not exist when the document is written (the reference is a write-time constraint only; nothing resolves it for a reader). The keyword is `refersTo` on the property, its `type` one of `identity`, `contract` (optionally with `contractRequirements`, see [Contract Moderation](contract-moderation.md)), `token`, `permanentDocument`, `deletableDocument`, `identityPublicKey` and `listElement` (see [An element of a list](#an-element-of-a-list-listelement)), or a reference expression combining several with `anyOf` and `allOf` (see [Reference expressions](#reference-expressions-anyof-allof)). Every form sits on an identifier property, with one exception below. The parsed shape is `DocumentPropertyType::IdentifierWithReference(target)`, and any change to a declaration on contract update is an incompatible schema change.

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

Both forms share the state check (`validate_referenced_identity_key_v0` in the document reference validation): the key must exist and not be disabled, else the write is refused, paid, with `ReferencedIdentityKeyNotFoundError` (40123) or `ReferencedIdentityKeyDisabledError` (40124). Identity keys can be disabled but never removed, so a validated reference never dangles. The owner form's identity is the transition's signer, which the transition already proved exists, so the key fetch is its only read; a key id of some other identity's key is meaningless by construction, there is no property to name another identity. On replace the identity form is re-validated when either the identity property or its key id property changed. For the key id form it depends on where the identity comes from. `$ownerId` is the writer, transition metadata that never appears among the changed fields, and the document may have changed hands since the key id was written, so the reference is re-validated on every replace, touched or not, as the `$ownerId` writer gate is: after a transfer the new owner has to repoint the key id at one of its own keys. `$creatorId` never changes, so it is re-validated when the key id changed. A document written before its type recorded creator ids (no type did before protocol version 10, nor one of a contract whose config was still version 0) records none, and a contract update may add a `$creatorId` key id property to its type: setting that key id on such a document is refused (`ReferencedKeyIdPropertyInvalidError`, 40125), while a replace leaving it unset is not checked. A property path is re-validated when the key id or that property changed, and a key id set while the property is not is refused (`ReferencedKeyIdPropertyInvalidError`, 40125). A transfer itself is not checked in any form, so the reference governs writing, not holding. The declaring property must carry exactly the key id range in its schema, whatever the contract's integer sizing setting, and a `keyIdProperty` of the identity form may not name a property that carries this form, nor may a path name an identifier carrying an `identityPublicKey` reference: one pair is declared once (40125 at registration). The charter contract's `joinRequest.senderKeyId`, the owner's encryption key a shared secret is derived from, is the first user.

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

A `deletableDocument` reference may carry a `lookup` too, and then promises less than its id form. Once the document a key found is deleted, a new document with the same key makes the reference resolve again, to different content, where an id is produced at most once and a dead id reference stays dead. So a deletable lookup means "a document with this key exists now", which is what a membership gate needs: the moderation charters' `resignationRequest` requires its writer to have an `addedModerator` for the charter now, and the leader takes an added member off by deleting that document. Every replace re-validates it, as it does a `deletableDocument` reference by id; an immutable property may not hold one, since the clearing a dead id reference allows reads an id, which a key is not; and it may be an operand of a reference expression and the target of an `ownerRefersTo` (see below).

`index` names an index of the referenced document type. `keys` maps every property of that index, by its name on the referenced side (system ones such as `$ownerId` included), in any order, to where its value comes from on the referring side:

- a property path of the referring document type (`"submittedCharterId"`, `"meta.charterId"`);
- `"$ownerId"`, the referring document's owner, the writer;
- `"."`, the value of the property that carries the reference, or the element. It appears exactly once: without it every value would resolve to the same document.

What is checked when the contract enters the chain, on registration and on update:

- `lookup` is only allowed on `permanentDocument` and `deletableDocument` references (meta-schema v3 and the parser, `apply_property_reference` 0), on the property or on the `items` of a typed array.
- Each property a key reads must exist on the referring type, be required (and so must every object around it), not be transient nor sit inside a transient object, and hold a single value, so a lookup never runs with a missing key part and a reader can assemble the same key from the stored document. A key that reads `"$ownerId"` needs a referring type whose documents can be neither transferred nor traded: the reference is judged when the document is written, and a transfer or purchase would move the writer part of its key without a write. These are properties of the referring type alone and are checked on every parse (generation 3).
- The index must exist and be unique, so the key finds at most one document; it may not bucket a timestamp with `timeRange`, and the referenced type may not be `indexOnly`. `keys` must cover each property of the index exactly once and nothing else, and each source must hold the same kind of value as the index property it fills (the rule of `propertyAgreement`, `DocumentPropertyType::value_kind`).
- The key must stay with the document it found, or the reference could dangle without the document being deleted: every schema property of the index must be fixed once written (the referenced type is immutable, or the property, or the top-level object holding it, is listed under `immutable`), `$ownerId` is only a key part on a type whose documents can be neither transferred nor traded, and the update and transfer times are refused where a replace, transfer or purchase moves them. `$id`, `$creatorId` and the creation times are always fixed.
- A changed, added or removed `lookup` is an incompatible schema change on update, like the rest of a `refersTo`.

The checks on the referenced type run where that type is in hand. For a document type of the same contract the contract parse runs them under full validation (`create_document_types_from_document_schemas` 1, next to the `keyRequirements.boundTo` check), once every document type is parsed; a target of the wrong kind is left to registration, which refuses a deletable type for a `permanentDocument` lookup (40122) and one that forbids deletion for a `deletableDocument` lookup (40131). For a type of another contract (`contractId`) the registration state validation runs them against that contract, where the other `refersTo` checks into another contract run, and refuses a declaration that cannot resolve with `ReferencedDocumentLookupInvalidError` (state code 40137). Index definitions cannot change on a contract update from protocol version 14 (`validate_update` 1 compares them by name), and neither can the flags the permanence rule reads, so the answer holds.

When the referring document is created or replaced, the document reference validation assembles the key for each value and queries the index for at most one document, billed as a document fetch of the same kind as the id lookup (`fetch_document_through_lookup`). No document, or a key it cannot assemble, refuses the write, paid, with `ReferencedEntityNotFoundError` (40120) naming the property, or the element by its list path (`members[1]`); its target reads "found through unique index `<index>`". A `propertyAgreement` beside the `lookup` is checked against the document the index found, exactly as for an id reference. A replace re-validates a `permanentDocument` lookup when the property itself changed (for a list, the elements the stored list did not hold), and every value, every element included, when a property a key reads changed. Nothing else can move a key part: the writer is fixed on a type allowed to read it, and the referenced side's key is fixed by the rule above, so a validated permanent lookup never dangles. A `deletableDocument` lookup is re-validated on every replace, since the document it found may be gone.

Joins cannot go through a lookup reference: a chained query or a composite by-id join needs the join property's values to be the outer documents' ids, so both refuse such a property, and a `preallocated` index cannot be bound through one. In Rust the declaration is its own variant, `DocumentPropertyReferenceTarget::PermanentDocumentLookup` (and `DeletableDocumentLookup` for the deletable form, appended after it), rather than a field of `PermanentDocument`: the enum is embedded in the reference errors, so an id reference keeps its encoding, and code matching `PermanentDocument` as "the value is a document id" cannot mistake a lookup for one. The rules are on `DocumentReferenceLookup`. `as_document_reference` returns only references whose value is a document id, the accessor for joins; the validators use `as_any_document_reference`, whose declaration carries the lookup.

### Reference expressions (`anyOf`, `allOf`)

A `refersTo` may combine targets in place of naming one. `{ "anyOf": [...] }` holds if at least one operand holds, `{ "allOf": [...] }` if every operand holds for the same value. An operand is a leaf, an ordinary target with its own keys, or an expression of the other combinator, so the two nest:

```json
"memberId": {
  "type": "array", "byteArray": true, "minItems": 32, "maxItems": 32,
  "contentMediaType": "application/x.dash.dpp.identifier",
  "refersTo": {
    "anyOf": [
      {
        "type": "permanentDocument", "documentType": "addedModerator",
        "lookup": { "index": "byModerator", "keys": { "submittedCharterId": "submittedCharterId", "moderatorId": "." } }
      },
      {
        "allOf": [
          { "type": "identity" },
          {
            "type": "permanentDocument", "documentType": "joinRequest",
            "lookup": { "index": "bySubmittedCharter", "keys": { "submittedCharterId": "submittedCharterId", "$ownerId": "." } }
          }
        ]
      }
    ]
  },
  "position": 2
}
```

reads: the member was added to the charter, or it is an identity that asked to join it. The same form sits on the `items` of a typed array, where each element meets the expression on its own.

What is checked when the contract enters the chain:

- On every parse (meta-schema v3 and the parser, `apply_property_reference` 0): a combinator is the declaration's one key (a `propertyAgreement` or a `lookup` belongs to a leaf, inside it), a list names at least two operands (a single one is declared on its own), and an `anyOf` directly inside an `anyOf` (or an `allOf` inside an `allOf`) is refused, since it says what one flat list says.
- Every leaf is an `identity`, a `permanentDocument` (by id or with a `lookup`), a `listElement` or a `deletableDocument` with a `lookup`. The first three are existence checks against entities that are never deleted, so an expression of only them holds for good once it holds, as a single one of them does, and a replace re-validates it only when its value or a property one of its leaves binds changed. A deletable lookup leaf may find nothing later, so an expression holding one is re-validated on every replace, and an immutable property may not hold it. The other types do not compose with other operands and are refused, as is the key id form (`identityProperty`): `deletableDocument` by id is re-validated on every replace and may be cleared once its document is deleted (the immutable-property exception), which assumes the property refers to that one target; `identityPublicKey` pairs the value with a key id property no other operand reads; a `contract` target's requirements are gates judged against the block time and the writer rather than an existence check, and a contract or token id is never also an identity or document id. Admitting one later takes a new `apply_property_reference` generation.
- Under full validation (registration): at most `SystemLimits::max_reference_operands` operands in one list and at most `max_reference_expression_depth` combinators on any path from the declaration to a leaf (4 and 4 at protocol version 14; the example above is 2 deep), no two alike operands in one list (a leaf naming the declaring contract explicitly is the same as one omitting it), and every leaf counted against `max_references_per_document`: an `anyOf` of two on a typed array of `maxItems` 15 counts 30, since each leaf may be read for each element.
- Every leaf is checked exactly as the same target declared alone: the referenced document type, its permanence, the `propertyAgreement` sides and the `lookup` rules, at the same places (the contract parse for a type of the same contract, the registration state validation for another contract's). Every leaf must pass, since each has to be a declaration that could hold. An error names the failing leaf by where it sits, `refersTo anyOf[1].allOf[1] lookup: ...` from the parse and `resignation.memberId.anyOf[1].allOf[1]` from registration.
- A changed expression (an operand added, removed, changed or moved, `anyOf` swapped for `allOf`, a single target turned into an expression or back) is an incompatible schema change on update, like the rest of a `refersTo`. Inside `refersTo`, `anyOf` and `allOf` are the declaration's data; the schema compatibility rules never read them as JSON Schema keywords.

When the referring document is created or replaced, the document reference validation evaluates each value (each element) operand by operand in declared order, a nested expression the same way. An `anyOf` stops at the first operand that holds; when none does, the write is refused, paid, with the last operand's result. An `allOf` stops at the first operand that fails and refuses the write with its result. A refusal is therefore always the error a leaf declared alone would give (for the example, `ReferencedEntityNotFoundError` (40120) for a lookup that found nothing, naming the property or the element), and the author's order decides which one a writer sees: put the most general operand of an `anyOf` last, and the cheapest or most telling one of an `allOf` first. There is no error of its own for "no operand held": each leaf's failure already has a precise error, and a combined one would have to nest one per leaf or lose their reasons. Every read is billed as it is made, so a value the second operand of an `anyOf` holds for pays for the first operand's query too, while an `allOf` whose first operand fails reads nothing more. A `propertyAgreement` is checked only against its own leaf's document: a value whose first leaf fails its agreement is still accepted through a second leaf without one.

Joins and preallocated indexes need one target: a chained query or a composite by-id join refuses an expression join property, and a `preallocated` index is never bound through one. In Rust the combinators are `DocumentPropertyReferenceTarget::AnyOf(ReferenceOperands)` and `AllOf(ReferenceOperands)`, appended to the enum so every single target keeps its encoding. An expression is no document reference as a whole (`as_any_document_reference` is `None`); code that checks every declaration walks `DocumentPropertyReferenceTarget::leaves` (or `leaves_with_paths`), the leaves of an expression or the declaration itself. Since the enum is embedded in consensus errors, which clients decode from bytes a node sends, decoding refuses a nesting deeper than `MAX_REFERENCE_EXPRESSION_DECODE_DEPTH` (16, above every protocol version's registration limit, which a test holds it to), so no bytes can drive the decoder into unbounded recursion. A reference error never carries a combinator: a refusal is a leaf's error.

### An element of a list (`listElement`)

A `listElement` reference says the value must be one of the identifiers a list of another document holds, the document an agreement pair names by its `$id`:

```json
"resignation": {
  "type": "object",
  "properties": {
    "electedCharterId": {
      "type": "array", "byteArray": true, "minItems": 32, "maxItems": 32,
      "contentMediaType": "application/x.dash.dpp.identifier",
      "refersTo": { "type": "permanentDocument", "documentType": "electedCharter" },
      "position": 0
    },
    "memberId": {
      "type": "array", "byteArray": true, "minItems": 32, "maxItems": 32,
      "contentMediaType": "application/x.dash.dpp.identifier",
      "refersTo": {
        "type": "listElement",
        "documentType": "electedCharter",
        "propertyAgreement": { "electedCharterId": "$id" },
        "inList": "members"
      },
      "position": 1
    }
  },
  "required": ["electedCharterId", "memberId"],
  "additionalProperties": false
}
```

reads: `memberId` must be one of the `members` of the `electedCharter` document whose `$id` this document's `electedCharterId` holds. The moderation charters, whose elected charter holds its `members`, are the first users. The declaration sits on an identifier property, on the `items` of a typed array of identifiers, where every element must be listed (see [References on the Elements](#references-on-the-elements)), as a leaf of a reference expression (see [Reference expressions](#reference-expressions-anyof-allof)), or on the writer or the creator (see [On the writer or the creator](#on-the-writer-or-the-creator-ownerrefersto-creatorrefersto)), where the value is that identity: `"ownerRefersTo": { "anyOf": [<listElement>, <addedModerator lookup>] }` is the charters' rule that the writer is a seated or an added moderator.

A list element is a document reference in every respect but one. It takes the `contractId`, `documentType` and `propertyAgreement` of a `permanentDocument` reference, with the same checks (the referenced type must forbid deletion, every pair must exist and share one value kind), and `$id` joins `$ownerId` and `$creatorId` as a system name the referenced side of any agreement pair may carry. What differs is what the value is: not the referenced document's id but an element of its list. The document is the one the pair with `$id` on the referenced side names, so a `listElement` holds exactly one such pair, read from an identifier property of the referring type (a schema property, never `$ownerId`: no document has the writer's id), and `inList` names the typed array of identifiers on the referenced type. What is checked when the contract enters the chain:

- The `$id` pair reads a stored identifier property of the referring type (it and every object around it not transient), so a reader can tell from the stored document which list the value was checked against. It may be optional, and it needs no `refersTo` of its own. Checked by the parse under full validation (generation 3), a leaf of an expression as it would be alone.
- The list's document type forbids deletion, and `inList` is a stored typed array of identifiers on it that never changes once a document is written: the type is immutable (`documentsMutable: false`), or the list's top-level property is listed under `immutable`, the rule a lookup's key parts are judged by. An `immutableAllowSetting` entry can only be set on a document that has no value for it, against which no value was ever accepted, so it does not weaken the rule. For a type of the same contract the contract parse checks this (`create_document_types_from_document_schemas` 1); for a type of another contract, registration checks it against that contract in state and refuses a list that does not qualify with `ReferencedDocumentListInvalidError` (state code 40138). A missing document type is refused as for any document reference (40121), and a deletable one by the parse for the same contract (with the list reason) or by registration for another (40122).
- Each value counts against `SystemLimits::max_references_per_document`, one for a property, `maxItems` for a typed array, like every other reference.
- A changed, added or removed `listElement` is an incompatible schema change on update.

When the referring document is created or replaced, the document reference validation fetches the document whose id the `$id` pair's property holds, by id, checks the other pairs against it exactly as for a `permanentDocument`, and requires the value to be in its list. Every by-id document fetch of one write is shared: a charter that `electedCharterId`'s own reference already fetched, or that the elements of one typed array all name, is fetched and billed once, and the list is collected once into a set, so each value is a set lookup. A value the list does not hold, a document the id names that does not exist, or a value set while the `$id` property is not, refuses the write, paid, with `ReferencedEntityNotFoundError` (40120) naming the property, or the element by its list path (`witnesses[1]`); its target reads "list element (`<inList>` of the `<documentType>` document `<property>` names)". A failing extra pair is `ReferencedDocumentPropertyMismatchError` (40127), as always. A replace checks a list element again when its value changed or when the referring side of any of its pairs changed (the `$id` property among them, since it may name another charter), the rule every agreement follows, and then every value, every element included; only a list that changed on its own leaves out the elements the stored list already held. Nothing else can make a validated value unlisted: the list's document can never be deleted and its list never changes.

For example:

```text
electedCharter 7kX...: members [Alice, Bob]
resignation { electedCharterId: 7kX..., memberId: Alice }  -> accepted
resignation { electedCharterId: 7kX..., memberId: Carol }  -> refused, 40120:
  referenced list element (members of the electedCharter document electedCharterId names)
  <Carol> not found for path memberId
```

In Rust the declaration is the appended variant `DocumentPropertyReferenceTarget::ListElement(ListElementReference)`, so every earlier variant keeps its encoding in the reference errors; the rules are on `ListElementReference` (`document_id_property`, `referring_side_error`, `referenced_side_error`, `listed_values`). `as_any_document_reference` carries it with `in_list` set, so the registration validator checks its contract, type and pairs through the same code as the other document references, while `as_document_reference` leaves it out, as it does a lookup: joins and `preallocated` indexes never go through it.

### On the writer or the creator (`ownerRefersTo`, `creatorRefersTo`)

A property's reference constrains a value the writer chose. Some rules constrain the writer instead: in the moderation charters, a `resignationRequest` may only come from a moderator of the team it resigns from. A document type states that with the doctype-level `ownerRefersTo` keyword, one `refersTo` declaration whose value is the document's `$ownerId`, the writer, rather than a property's value:

```json
"resignationRequest": {
  "type": "object",
  "ownerRefersTo": {
    "type": "deletableDocument",
    "documentType": "addedModerator",
    "lookup": {
      "index": "byElectedCharterMember",
      "keys": { "electedCharterId": "electedCharterId", "memberId": "." }
    }
  },
  "properties": { "electedCharterId": { "...": "..." } }
}
```

reads: the writer must be the `memberId` of an `addedModerator` for this document's `electedCharterId`, one that exists when the request is written (the leader takes an added member off by deleting it).

- The declaration is the one an identifier property carries, read by the same code (`apply_property_reference` 0), but only these targets can hold a writer: `identity`, a `permanentDocument` or a `deletableDocument` found through a `lookup`, and a `listElement` (the writer an element of the list, see [An element of a list](#an-element-of-a-list-listelement)), alone or as the leaves of a reference expression (above). The rest are refused, as a leaf of an expression too. `contract`, `token` and a document by id would need the writer's identity id to be a contract, token or document id, which it never is, so a document type declaring one could never be written; `identityPublicKey` pairs the value with a key id the writer does not carry. Meta-schema v3 reuses the property declaration by `$ref` and admits only those forms; the parser (generation 3, `parse_owner_reference`, which reads the stored schema once the core parse has run the meta-schema) refuses the others on the stored path too. The parsed declaration is `DocumentTypeV2::owner_reference`, read through `DocumentTypeV2Getters::owner_reference`, and the property types are unchanged.
- Only a document type whose documents can be neither transferred nor traded may declare it, checked on every parse. A transfer or a purchase is not a write, so it would hand the document to an owner the declaration never checked; with neither possible, the owner of every document is the writer that was checked.
- In a `lookup`, `"."` is the writer, and a `"$ownerId"` key part is the writer as well. Every referring-side rule of a property's lookup applies unchanged (its `"$ownerId"` rule holds by the point above), and so does every referenced-side rule, for a type of the same contract at contract level and for one of another contract at registration.
- A `propertyAgreement` works as on a property reference; its referring side may name `$ownerId`, which is then the same writer as the reference's value.
- It counts one against `SystemLimits::max_references_per_document`.
- When a document is created, the document reference validation checks the writer against the target exactly as a property's value is checked, before the properties' references. A replace re-validates it under the rules of its target, as a property's: when a property its lookup or a `propertyAgreement` reads changed, and on every replace for a pair keyed by `$ownerId`, a writer gate. Nothing else can change the outcome: the writer is the owner, the target can never be deleted and its key is fixed. A failure is the error the target reports for a property (`ReferencedEntityNotFoundError`, 40120, for a lookup that finds no document; `ReferencedDocumentPropertyMismatchError`, 40127, for an agreement; and the rest), with `$ownerId` as its path. An `identity` target reads nothing: the transition has already proved that the writer exists. At registration the contract reference validation checks the declaration as a property's, naming it `<documentType>.$ownerId`.
- Adding, removing or changing it is an incompatible schema change on update (`validate_schema_compatibility` 1 freezes the keyword as the shared rule set freezes `refersTo`).
- Every validator, the reference bound and the client bindings enumerate a type's references through `DocumentTypeRef::reference_declarations`, which yields the owner or creator reference first, as `ReferenceHolder::Owner` or `ReferenceHolder::Creator`, then each property's, so none can skip it.

A document type whose documents can be transferred or traded declares `creatorRefersTo` instead: the same declaration, whose value is the document's `$creatorId`, its creator, which a transfer or a purchase never changes. A marketplace item that only an elected moderator may mint, and anyone may then own, reads:

```json
"moderatorBadge": {
  "type": "object",
  "transferable": 1,
  "creatorRefersTo": {
    "type": "listElement",
    "documentType": "electedCharter",
    "propertyAgreement": { "electedCharterId": "$id" },
    "inList": "members"
  },
  "properties": { "electedCharterId": { "...": "..." } }
}
```

- It takes the same targets but a `deletableDocument` (the creator never changes, and a document a transfer handed on could not be replaced once the one a lookup found is deleted), except one found through a computed key, which is judged on the create alone (see [Commit and reveal](#commit-and-reveal-a-computed-lookup-key)), with `"."` the creator in a lookup, and is refused where `ownerRefersTo` is admitted: only a document type that records creator ids may declare it, a transferable or tradeable type of a format-1 contract (`should_use_creator_id`), checked on every parse. A type therefore declares at most one of the two. A `"$ownerId"` key part in its lookup is refused, as in a property's lookup on such a type, since the owner moves.
- When a document is created its creator is the writer; on a replace, the value is the stored creator, whoever writes, and the replace rules are those of its target, as for the owner reference. A transfer or a purchase needs no check. A failure is the target's error at the path `$creatorId`, and registration names the declaration `<documentType>.$creatorId`. An `identity` target reads nothing: the creator existed when it wrote the document, and an identity is never removed. It counts one against `max_references_per_document`, and a change to it is an incompatible schema change on update.

### Commit and reveal (a computed lookup key)

A lookup key may be computed instead of read: `{ "sha256d": [part, ...] }` in place of a source fills the index property with the SHA-256 of the SHA-256 of a preimage the document being created reveals. The document the key finds is a commitment made earlier, so the lookup is a commit and reveal: the document may be created only while a commitment to values it carries exists. DPNS registers names this way, with a `preorder` holding `saltedDomainHash` under a unique index, and a `domain` whose create trigger hashes what the domain reveals. The same rule, written as a declaration, reads:

```json
"domain": {
  "type": "object",
  "transferable": 1,
  "creatorRefersTo": {
    "type": "deletableDocument",
    "documentType": "preorder",
    "lookup": {
      "index": "saltedHash",
      "keys": { "saltedDomainHash": { "sha256d": [
        { "property": "preorderSalt" },
        { "ifEmpty": "parentDomainName",
          "then": [{ "property": "label" }],
          "else": [
            { "property": "normalizedLabel" },
            { "text": "." },
            { "property": "parentDomainName" }
          ] }
      ] } },
      "minimumAgeSeconds": 60,
      "consume": true
    },
    "propertyAgreement": { "$ownerId": "$ownerId" }
  }
}
```

reads: the creator must own a `preorder`, at least a minute old, whose `saltedDomainHash` is the sha256d of the salt followed by the label for a top-level name, or by the normalized label, a dot and the parent otherwise, and creating the domain deletes that preorder. The hash is byte for byte the one `create_domain_data_trigger_v1` computes.

The preimage is the bytes of its parts concatenated with nothing between them:

- `{ "property": path }`: a string property's UTF-8, a byte array property's bytes, an identifier property's 32 bytes;
- `{ "text": text }`: fixed UTF-8 text, 1 to 64 bytes;
- `{ "ifEmpty": path, "then": [...], "else": [...] }`, at most one: its `then` parts when the named string or byte array property is empty, its `else` parts otherwise. Neither branch holds another `ifEmpty`.

At most 16 parts in all. Plain concatenation is ambiguous when two variable-length parts meet (`"ab" + "c"` and `"a" + "bc"` are the same bytes), so every variable-length part (a string, or a byte array whose size is not fixed) that another one follows must be followed directly by a one-byte text part, its separator, and a value holding that byte is refused. Each preimage then splits into its parts one way only. Integers and other kinds are not parts.

The document such a key finds is judged once, when the document is created:

- A part may be transient (the DPNS salt is), since it is read from the create transition, and optional, since a create missing one is refused. Every stored value the lookup reads, parts and other key parts alike, must be fixed once written (the type is immutable or lists the property under `immutable`), and a property carrying the reference must be set when the document is created (not listed under `immutableAllowSetting`). A replace leaves the reference alone: nothing it reads can have changed, and the commitment may be gone.
- `"."` must still fill a key part on a property; on `ownerRefersTo` and `creatorRefersTo` it may be left out beside a computed key, since the declaration applies to every create. `creatorRefersTo` takes a `deletableDocument` target only through a computed key.
- A lookup holds at most one computed key. The index property it fills must be a byte array of exactly 32 bytes. Beside it the lookup may declare:
  - `minimumAgeSeconds`: the found document's `$createdAt` must be at least that many seconds before the block time of the create, so a commitment and its reveal cannot share a block. The referenced type must list `$createdAt` in `required`; a document recording no creation time never meets it.
  - `consume: true`: the create deletes the found document in the same state transition, its storage refunded to its owner as a delete by that owner would be. Only on a `deletableDocument` reference whose `propertyAgreement` pairs `"$ownerId"` with `"$ownerId"`, into a document type of the declaring contract whose owner may delete its documents (`canBeDeleted: true`).
- Whose commitment it is, is the `propertyAgreement` pair `{ "$ownerId": "$ownerId" }`: the writer must own the document found. Without it anyone who learns a preimage may reveal it.

What is refused, and where:

- The shape, the parts and the rules above: the contract parse on registration and update (meta-schema v3 `lookupHashKey`, `apply_property_reference` 0, `DocumentReferenceLookup::referring_side_error` and `referenced_side_error`), and for a lookup into another contract the registration state validation (`ReferencedDocumentLookupInvalidError`, 40137), which also refuses `consume` there.
- A create missing a part, or whose variable-length value holds its separator: document create structure validation 1, before any read (`DocumentReferencePreimageInvalidError`, 10423). A computed key that is a leaf of a reference expression is left to the write-time check, where a key it cannot assemble finds no document, so that operand fails and the others still decide.
- No document for the key: `ReferencedEntityNotFoundError` (40120), the lookup billed as the document fetch it is. Another identity's commitment where the owner pair is declared: `ReferencedDocumentPropertyMismatchError` (40127). A commitment younger than `minimumAgeSeconds`: `ReferencedDocumentRequirementNotMetError` (40142). All paid, from document create state validation 2.
- A batch in which a create consumes a document that another create of the batch consumes too, or that another transition of the batch deletes, replaces, transfers, reprices or buys, refuses that create with 40120 (`ConsumedDocuments` in the batch state validation). Today a batch holds one transition, so this only guards the day that cap is raised.

In Rust the key is `LookupKeySource::Hash(LookupHashKey)`, appended to the key sources, with its parts `LookupPreimagePart`; `DocumentReferenceLookup::is_checked_on_create_only` tells a lookup holding one, and `minimum_age_seconds` and `consume` sit on the lookup. `first_unrevealable_lookup_key` is the structure check. A create that consumes carries its commitments in `DocumentCreateTransitionAction::consumed_documents`, turned into delete operations with the create's own.

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
- An `immutableAllowSetting` entry may not be a `deletableDocument` reference by id. A replace may clear such a reference on an immutable property once its target is deleted (see [References on the Elements](#references-on-the-elements)), and the property, absent again, would then accept a first-time set to another document: the frozen reference would be repointed. The clear stays, because every replace re-validates the reference and without the clear the document could never be replaced again, so the pair is refused at registration instead, and on contract update, which parses the whole new contract the same way. Every other `deletableDocument` form is already refused on any immutable property.
- On contract update `immutable` may gain entries but never lose one, and `immutableAllowSetting` may lose entries but only gain one for a property that becomes immutable in the same update (`DocumentTypeUpdateError` otherwise). Each rule keeps the promise documents were created under: nothing frozen becomes editable, and nothing already frozen starts accepting a late set. The schema compatibility differ strips both keys, like `indices` and `required`, so `validate_update` v1 is the single judge.

Enforcement lives in the replace action's state validation (generation 1). The action already records which top-level properties differ from the stored document in `changed_data_fields` (the same set that scopes `refersTo` re-validation), and alongside it which of those the stored document had no value for (`added_data_fields`). A changed property in the type's `immutable_fields()` fails the replace with `DocumentImmutablePropertyChangedError` (state code 40128) unless it is in `immutable_fields_allow_setting()` and was absent before, or it is a `deletableDocument` reference by id that the replace removed after its target was deleted. The registration rule above keeps the two from ever applying to one property. "Differ" covers a changed value, a property the stored document lacked, and a property the replace dropped. Transfers, price updates and purchases carry no property data and are unaffected.

In Rust the lists are `DocumentTypeV2Getters::immutable_fields()` and `immutable_fields_allow_setting()`. Earlier document type generations return empty sets.

## Transient Properties

The doctype-level `transient` keyword lists top-level properties whose values are validated on the transition but never stored. DPNS uses it for the `domain`'s `preorderSalt`: the write proves the salted preorder, and the salt is then dropped.

```json
"transient": ["preorderSalt"]
```

A create drops the listed values before its document is built. Before protocol version 14 a replace stored whatever it carried, so a replaced document kept values its create had dropped; from protocol version 14 a replace drops them the same way (`document_from_replace_transition_action` 1). Values are dropped by top-level name, so a leaf of a transient object goes with the object.

Because no stored document carries a transient value, a rule that reads a stored value refuses a transient one, judged by the property's path and every object around it (`is_transient`). From protocol version 14, at registration:

- Every `transient` entry names a top-level property. A nested path, a system property or an undeclared name would mark a property transient in the parsed type while its value was still stored.
- No index reads a transient property. Every document would sit in the index's null branch, so a query by the value would find nothing and a unique index would enforce nothing.
- A `refersTo` lookup reads no transient property to assemble its key, and its index keys documents by none. A `propertyAgreement` names none on its referenced side. The referring side may be transient: it is judged on the transition, a write gate like the writer's `$ownerId`.
- A key reference does not store its key id with a transient identity, whichever side declares it (`identityProperty` on the key id, `keyIdProperty` on the identity): the key id alone names no key.
- `encryptedFor` names no transient recipient or key id.

The list cannot change on contract update: it decides which values stored documents carry and how every property is encoded (a transient property takes a presence byte even when required). From protocol version 14 any change to the names it lists is an incompatible schema change (the list is compared as a set, so reordering or repeating a name is no change); before it, the schema compatibility check failed on the keyword as unsupported, an internal error.

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
- A replace follows the rules of a single reference, element by element. A changed list re-validates the elements the stored list did not hold (the replace action carries the stored value of each changed property, `stored_changed_values`); the ones it held are unchanged references and are left alone, as an unchanged single reference is. Every element is re-validated when a property bound by a `propertyAgreement` changed, and on every replace when an agreement is keyed by `$ownerId`, the elements are `deletableDocument` references, or they are `contract` references with an `owner` requirement on a document type whose documents can be transferred or traded. An element repeating an earlier one of the same list is not fetched again.
- Every read is billed as a single reference's is. A foreign contract holding the referenced document type is fetched once per list, not once per element. Contract registration caps the references one document of a type can carry at `SystemLimits::max_references_per_document` (256), counting one per property with `refersTo` (an identifier, or a key id carrying a key reference), one for the type's `ownerRefersTo` and `maxItems` per typed array of referencing elements: `maxItems` alone would let a type declare many lists of up to 1024 references each, and each one is a read when a document is written.
- An `immutable` property may not hold a `deletableDocument` reference a replace could not clear: a typed array of them, at the top level or inside an immutable object, or a single one inside an immutable object. Every replace re-validates them, so once a target is deleted the property would have to change, which an immutable property cannot. A single `deletableDocument` reference that is itself the immutable top-level property has a way out, a replace may remove it once its target is gone, and that exception reads the one identifier the removed top-level property held. That property may not also be listed under `immutableAllowSetting`: once it is cleared, the allowance would let the next replace set it to another document.
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

The parser (generation 3, meta-schema v3) admits the keyword on byte array properties only, never on an identifier (`contentMediaType` set) or any other type, and checks at contract registration that the three named properties exist with those types, that none of them is `transient` or sits inside a transient object (a transient value is stripped before storage, which would leave the stored ciphertext without its recipe), and that the byte array's own `maxItems` can hold the scheme's shortest ciphertext. A contract update that adds, removes or changes an `encryptedFor` declaration is an incompatible schema change (`IncompatibleDocumentTypeSchemaError`, 10246): documents already written under the old recipe could not be read under the new one. Contracts parsed before protocol version 14 ignore the keyword entirely.

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

Clients encrypt and decrypt through the declaration rather than a per-contract recipe. The Rust SDK's `dash_sdk::platform::encrypted_for` module has `encrypt_property`, which writes the ciphertext and both key id properties, and `decrypt_property`. `EncryptedPropertyEnvelope::read` names the identities and key ids a reader needs. `select_encryption_keys` picks the keys the document type's `identityPublicKey` references demand through their `keyRequirements`. In JavaScript the same helpers are `sdk.encryptedFor.encrypt`, `decrypt` and `envelope` (`WasmSdk.encryptDocumentProperty`, `decryptDocumentProperty` and `encryptedPropertyEnvelope`). The layout has no authentication tag, so a wrong key fails the padding check except about once in 256 attempts, when it yields garbage.

## Byte Caps on Strings (`maxBytes`)

Protocol version 14 adds the property keyword `maxBytes`, a bound plain JSON Schema cannot count: the most bytes a string may take in UTF-8. `maxLength` counts characters, and a character is up to four bytes, so `maxLength: 4096` alone admits values up to the 5120-byte cap every field has (`SystemLimits::max_field_value_size`), not 4096 bytes.

```json
"description": {
  "type": "string", "minLength": 1, "maxLength": 4096,
  "maxBytes": 4096,
  "position": 1
}
```

The keyword goes on a string property, or on the `items` of a typed array of strings, where it bounds every element; it is refused on the array itself and on elements of any other type. It is an integer from 1 to 65535 and no lower than `minLength`, since a string of `minLength` characters is at least that many bytes. On contract update it moves like `maxLength`: raising or removing it is compatible, adding or lowering it is not.

The parser (generation 3, meta-schema v3, `apply_max_bytes`) folds the bound into the string's `StringPropertySizes::max_bytes`, next to `max_length`, so the sizes the type reports take it into account: `max_byte_size` is the smaller of `maxBytes` and four bytes a character, and random documents stay within it.

The check runs where the JSON schema validation of a document's properties runs, `DataContract::validate_document_properties`, right after it: on every document create and replace, and in every client that validates a document before sending it. A longer string is refused with `DocumentPropertyMaxBytesExceededError` (basic code 10421), which names the property (`tags[2]` for an element) and both lengths. The document validation (version 0, extended in place) is inert before protocol version 14, where no string carries a byte cap and the `validate_max_bytes` method slot is `None`. In Rust the check is `DocumentTypeBasicMethods::validate_max_bytes_properties()`; in JavaScript the error reaches an app as `DocumentMaxBytesErrorCode.MaxBytesExceeded`.

## Generated Properties (`generatedFrom`)

Protocol version 14 adds the property keyword `generatedFrom`: the platform generates a string property's value with a system function of other properties of the same document, its params. The first system functions change the case of a string (`lowercase`, `uppercase`, `capitalize`, `camelCase`, `snakeCase`) or apply the DPNS `domain` rule (`normalizedLabel` is `label` lowercased, with `o`, `i` and `l` replaced by `0`, `1` and `1`), so any contract can build a unique index that treats look-alike names as one.

```json
"normalizedLabel": {
  "type": "string", "maxLength": 63,
  "generatedFrom": {
    "function": "sys.stringTransformations.homographSafeASCII",
    "params": ["label"]
  },
  "position": 1
}
```

The functions are a closed list of system functions (`SystemFunction`), named under `sys.` so that functions a contract may bring later can be told apart by name; each declares how many params it takes. `SystemFunction` is an enum of namespaces, each an enum of its own in a module of its own: `SystemFunction::StringTransformation(StringTransformation)` for `sys.stringTransformations`, in `system_function::string_transformations`, whose six functions each take one string. `lowercase`, `uppercase` and `capitalize` change the case of ASCII letters; `camelCase` and `snakeCase` split the string into words (at every ASCII character that is neither a letter nor a digit, and before an ASCII uppercase letter that starts a word) and join them as `helloWorld` or `hello_world`; `homographSafeASCII` maps `A` to `Z` to lowercase, then `o` to `0` and `i` and `l` to `1`. Every one changes ASCII characters only and keeps every other character, with no Unicode table, because Unicode case mappings differ between releases of the standard library, and two nodes must never generate different values; every one gives back its own output unchanged, and a test pins that the meta-schema lists exactly the functions the parser knows. On ASCII `homographSafeASCII` equals `convert_to_homograph_safe_chars`, the function the DPNS trigger uses; a test pins that over every ASCII character and over strings from the DPNS alphabets. It refuses nothing: the characters a value may hold are each param's `pattern`'s to decide, and the generated property needs no pattern of its own, since every value it holds is generated from params that passed theirs. A param is a property path for now (`GenerationParam::Property`); literals, system values and nested calls can be added later without changing what parses today.

The parser (generation 3, meta-schema v3, `apply_generated_from`) reads the declaration onto `DocumentProperty::generated_from` (`Option<GeneratedFrom>`, absent on every property parsed before protocol version 14) and checks that `params` lists as many params as the function takes. `validate_generated_from_declarations` checks every param against the other properties on every parse: another string property, not generated itself, not transient nor inside a transient object (and neither is the declaring property), and inside every object that holds the declaring property. The meta-schema refuses the keyword beside `$ref`, whose definition replaces every keyword written next to it. On contract update a changed, added or removed `generatedFrom` is an incompatible schema change, and a property the update adds may declare it only when one of its params is new too (`validate_update` 1 refuses one whose params all existed with `DocumentTypeUpdateError`, 40212: documents stored before the update were never generated). The declaring paths and their declarations are cached on the document type (`DocumentTypeV2Getters::generated_from_fields`), so a write to a type without declarations pays nothing.

Three methods do the work at write time:

- `DocumentTypeBasicMethods::fill_generated_properties` writes every declared property the document leaves out while supplying every param. The action transformers of document create, replace and index-only delete call it first, before the contest resolution and every check read the data, so the stored document, its indexes and its contest all hold the generated value. `Document::try_from_create_transition` and `try_from_replace_transition` call it too, and so does `index_only_transition_entry_path_query`, the one builder the prover and the verifier share for index-only entries, so proofs are built and checked against the document the platform stored.
- `DocumentTypeBasicMethods::regenerate_generated_properties` is the client-side twin: it sets every declared property to what its params generate, replacing a value the document holds and removing it when a param is absent. The transition builders (`from_document` of create, replace and index-only delete), the SDK's contest fund lookup and the property-constraint pre-checks of the JavaScript and FFI SDKs call it, so a document fetched, edited and sent back carries the value of its new params rather than the stale one the platform would refuse, and its contest is detected from it. Random documents call it too, after drawing the params of every generated property they drew.
- `DocumentTypeBasicMethods::validate_generated_from_properties` runs in `DataContract::validate_document_properties`, after the JSON schema and `maxBytes`: a declared property must equal what its function generates from its params, and be absent when a param is. A document that repeats a key in an object on the way to the property or to a param is refused too: the schema validation and the stored document keep the last of repeated keys, where the path reads the platform generates from find the first. A property that does not pass refuses the write with `DocumentPropertyNotGeneratedError` (basic code 10424). A document that arrived has been generated, so on the platform the check only refuses a value the client sent, a value sent without its params, or a repeated key.

Every call site was extended in place and is inert before protocol version 14: the `apply_generated_from`, `fill_generated_properties` and `validate_generated_from` slots are `None` there, and the meta-schemas refuse the keyword. In JavaScript the error reaches an app as `DocumentGeneratedFromErrorCode.DocumentPropertyNotGenerated`.

## Property Constraints (`propertyConstraints`)

Protocol version 14 adds the doctype-level `propertyConstraints` keyword: named rules the properties of every created or replaced document must meet, where JSON Schema can only bound one property at a time. Each rule is a condition: a comparison of two integer expressions, a test of whether an integer expression takes one of listed values, a comparison of a string property with string constants, a test of whether the document holds a property, or `anyOf`, `allOf` or `not` over conditions.

```json
"propertyConstraints": {
  "depositCoversOrder": {
    "lessThanOrEqual": [
      { "multiply": [{ "add": ["price", "fee"] }, "quantity"] },
      "deposit"
    ]
  },
  "wholeLots": { "equal": [{ "modulo": ["quantity", 10] }, 0] },
  "minimumOrder": {
    "greaterThanOrEqual": [{ "multiply": ["price", { "ifAbsent": ["quantity", 1] }] }, 100]
  },
  "feeWaivedOrAtLeastTen": {
    "anyOf": [{ "equal": ["fee", 0] }, { "greaterThanOrEqual": ["fee", 10] }]
  },
  "discountGivenAboveZero": {
    "anyOf": [{ "absent": "discount" }, { "greaterThan": ["discount", 0] }]
  },
  "tieredFee": { "in": ["fee", [0, 10, 25, 50]] },
  "closedNeedsClosedAt": {
    "anyOf": [{ "notEqual": ["status", { "const": "closed" }] }, { "present": "closedAt" }]
  }
}
```

The first rule reads `((price + fee) * quantity) <= deposit`, `feeWaivedOrAtLeastTen` reads `fee == 0 || fee >= 10`, `discountGivenAboveZero` lets an offer leave its discount out but not give a discount of 0, `tieredFee` holds the fee to four tiers, and `closedNeedsClosedAt` says a closed offer carries a `closedAt`. A rule's name is 1 to 64 letters, digits or underscores, and the rule is a condition, an object with one key:

- a comparison, `equal`, `notEqual`, `lessThan`, `lessThanOrEqual`, `greaterThan` or `greaterThanOrEqual`, listing the left and the right expression;
- `{ "in": [expression, [values]] }`, holding if the integer expression takes one of two or more distinct integer values. It says what an `anyOf` of `equal`s says, in one node per value instead of three, so a set of up to 30 values fits the node limit where the `anyOf` fits 10. A value is a literal, never a path or an expression;
- a string comparison: `{ "equal": [path, { "const": "closed" }] }` or `notEqual`, with the constant on either side; `{ "notEqual": ["fromCurrency", "toCurrency"] }`, two bare paths that both name string properties, which compares their strings; or `{ "in": [path, ["open", "pending"]] }`, whose values are two or more distinct strings. The path names a string property, typically one with an `enum`. A string on its own is a path, so a constant is written as `{ "const": ... }`, while the values an `in` lists are literals and need no wrapper. Strings are only compared for equality, never ordered or used in arithmetic. A string property the document leaves out equals no constant and no other string property, not even one also left out, so `notEqual` holds for it and `equal` and `in` do not, unless `{ "ifAbsent": [path, "open"] }` gives it a string default, which it then reads as (it may stand wherever the bare path does, and makes the comparison one of strings); `present` and `absent` test it directly. When the property declares an `enum`, every constant compared with it must be one of the enum's values, so a misspelling is refused at registration rather than making the rule quietly never hold;
- an identifier comparison, the same three forms for identifier properties, those declaring `refersTo` included: `{ "equal": ["paymentToken", { "const": "<base58>" }] }` or `notEqual`, `{ "notEqual": ["buyerId", "sellerId"] }`, or `{ "in": ["paymentToken", ["<base58>", "<base58>"]] }`. Constants are base58 identifiers of 32 bytes, checked at registration, and compared by their bytes, whatever form the document gives the identifier in. An identifier property the document leaves out equals no identifier, not even another one left out; identifiers take no `ifAbsent` default and are never ordered. `$ownerId`, the document's owner, is an identifier operand too: `{ "equal": ["authorId", "$ownerId"] }` holds the author to the owner, and `{ "in": ["$ownerId", ["<base58>", ...]] }` lets only the identities listed own a document of the type. It is no property, so `present` or an integer operand refuses it, and comparing it with itself is refused. Since a transfer and a purchase give the document a new owner, each is judged against the rules reading `$ownerId`, with the new owner, and refused when it would break one; an indexOnly type refuses such a rule, since its deletes carry no owner;
- `{ "startsWith": [text, affix] }` and `{ "endsWith": [text, affix] }`, holding if the first string starts or ends with the second, byte for byte with no case folding: each side a `{ "const": ... }`, a string property or an `ifAbsent` string default, at least one a property, never the same one twice. `{ "startsWith": ["url", { "const": "https://" }] }` holds a link to https, and `{ "startsWith": ["path", "parentPath"] }` a path under its parent's. A string property left out without a default takes no string, and the condition does not hold for it; a constant tested against a property that declares an `enum` must start or end one of its values;
- `{ "contains": [path, value] }`, holding if the typed array property at the path holds an element equal to the value, looked for as the array's elements are: an integer expression among integers, a string constant, string property or `ifAbsent` string default among strings, an identifier constant, identifier property or `$ownerId` among identifiers. `{ "not": { "contains": ["labels", { "const": "used" }] } }` refuses a `"used"` label, and `{ "contains": ["participants", "$ownerId"] }` holds the owner to the participants (so a transfer or purchase to a non-participant is refused). An array the document leaves out holds nothing, a string or identifier property it leaves out is among no elements, and a string constant must be one of the elements' `enum` values when they declare one;
- `{ "present": path }`, holding if the document holds the property, and `{ "absent": path }`, holding if it leaves it out (a property set to null counts as left out, and so does an object none of whose members is present, such as `{}`, which a stored document does not keep). An operand reads a property the document leaves out as 0, so only these tell "not given" from "given as 0". They may name a property of any type, an object or a member of one included, since they read no value;
- `{ "anyOf": [...] }`, holding if at least one of two or more conditions holds;
- `{ "allOf": [...] }`, holding if every one of two or more conditions holds;
- `{ "not": condition }`, holding if its one condition does not;
- `{ "ifThen": [a, b] }`, holding if `b` holds whenever `a` does: `b` is evaluated only when `a` holds, so `{ "ifThen": [{ "greaterThan": ["discount", 0] }, { "greaterThanOrEqual": [{ "divide": ["price", "discount"] }, 10] }] }` never divides by zero, and a fault in either breaks the rule. It says what `{ "anyOf": [{ "not": a }, b] }` says, in one node fewer. The two may not be alike;
- `{ "ifThenElse": [a, b, c] }`, holding if `b` holds when `a` does and `c` holds when it does not; only the branch `a` selects is evaluated. `{ "ifThenElse": [{ "greaterThanOrEqual": ["price", 1000] }, { "lessThanOrEqual": ["fee", 50] }, { "lessThanOrEqual": ["fee", 10] }] }` allows a higher fee on an expensive offer. No two of the three may be alike;
- `{ "notIn": [expression, [values]] }`, an `in` negated, listed the same way and in as many nodes: `{ "notIn": ["fee", [7, 13]] }` refuses two fees.

Conditions nest: `{ "not": { "allOf": [{ "equal": ["price", 0] }, { "greaterThan": ["quantity", 10] }] } }` refuses a free order of more than 10. An `anyOf` or `allOf` may not list two alike conditions, nor hold one of its own kind directly (it says what one flat list says), and a `not` may not hold a `not` or a `notIn` directly. An expression is one of:

- an integer value (`100`; a float with no fractional part, `100.0`, reads as that integer, as the meta-schema's `integer` type admits it);
- a string, the dotted path of an integer or boolean property of the document type (`"price"`, `"meta.total"`, `"waiveFee"`), whose value it takes, 0 when the document leaves the property out. A boolean reads as 1 for true and 0 for false, so `{ "equal": [{ "multiply": ["waiveFee", "fee"] }, 0] }` says a waived fee is 0;
- `{ "ifAbsent": [path, value] }`, the property's value, or `value` when the document leaves it out (an integer value here; a string value gives a string property a default in a string comparison instead);
- `{ "add": [...] }` or `{ "multiply": [...] }` over two or more operands;
- `{ "min": [...] }` or `{ "max": [...] }`, the least or greatest of two or more operands, every one evaluated (a fault in any breaks the rule), and `{ "abs": a }`, the absolute value of one: `{ "lessThanOrEqual": ["fee", { "max": [10, { "divide": ["price", 10] }] }] }` caps a fee at 10 or a tenth of the price, whichever is more, and `{ "lessThanOrEqual": [{ "abs": { "subtract": ["a", "b"] } }, 5] }` keeps two values within 5;
- `{ "subtract": [a, b] }`, `{ "divide": [a, b] }`, `{ "modulo": [a, b] }` or `{ "power": [a, b] }`;
- a size: `{ "length": path }`, the characters of a string property (counted as `maxLength` counts them), `{ "byteLength": path }`, its UTF-8 bytes (as `maxBytes` counts them), or `{ "count": path }`, the items of an array property or the bytes of a byte array property. Where `maxLength`, `maxBytes` and `maxItems` bound one property by a fixed number, a size can be compared with another property or bounded only under a condition: `{ "lessThanOrEqual": [{ "count": "tags" }, "maxTags"] }` holds a list to its own limit, and `{ "anyOf": [{ "greaterThan": ["fee", 0] }, { "lessThanOrEqual": [{ "length": "title" }, 20] }] }` keeps a free listing's title short. A property the document leaves out or sets to null has size 0, and a size never breaks a rule by itself (a value of another type would read as 0 too, but the schema validation refuses it first);
- a system time or height: `"$createdAt"`, `"$updatedAt"` and `"$transferredAt"`, block times in milliseconds, and each with `BlockHeight` or `CoreBlockHeight` appended, the Platform and Core block heights: those of the document's creation, of its last create, replace or price update, and of its last create, transfer or purchase. A rule may read one only on a type that records it by listing it in `required`, so every stored document holds it; it takes no `ifAbsent`, and an indexOnly type, whose deletes carry none, reads none. `{ "lessThanOrEqual": [{ "subtract": ["endsAt", "$createdAt"] }, 604800000] }` keeps a listing to a week from its creation, and `{ "lessThanOrEqual": ["$updatedAt", "endsAt"] }` refuses a replace or a price update after it ends;
- a total read from state: `{ "countOf": [type] }` or `{ "countOf": [type, filter] }`, how many documents of a type of the same contract there are, or how many match the filter, and `{ "sumOf": [type, property] }` or `{ "sumOf": [type, property, filter] }`, the total of an integer property over them. A filter maps each key, a property of the counted type or `$ownerId`, to the value it must take, read from the document being written: one of its properties, `$ownerId`, an integer or a `{ "const": ... }`. The total is the one a count or sum tree keeps, as it will be once the write is done (the document itself counted when the type is its own, at its new values, and moved to its new owner by a transfer or purchase), so `{ "lessThanOrEqual": [{ "countOf": ["listing", { "$ownerId": "$ownerId" }] }, 10] }` on `listing` keeps every owner at ten listings or fewer. A whole-type total needs `documentsCountable` or `documentsSummable`, and a filtered one an index of the counted type whose properties are exactly the filter's keys, countable or summing the property. A rule is judged only when its own type is written, so a fact about another type can go stale after the write.

A JSON number is always a value and a string always a path, so a property named `100` is not confused with the number, and the rule is a tree the meta-schema can check rather than a string with precedence rules to parse. Consensus holds nothing but this tree; an SDK may offer an infix spelling that compiles to it.

The arithmetic is exact over `i128`. Operands are evaluated left to right, and every intermediate result must fit: an overflow, a divisor of 0, a negative exponent or a property value that is not an integer (a float with no fractional part passes the schema's `integer` type) breaks the rule instead of wrapping or truncating. `divide` and `modulo` are Euclidean, so the remainder is never negative and the quotient is the one that goes with it (`-7` by `2` is `-4` remainder `1`); for operands that are not negative this is ordinary integer division. `0` to the power `0` is `1`. There are no floats: a `number` property cannot be read, which keeps every node's result bit-identical.

Conditions are checked in declared order and no further than the outcome needs: a comparison evaluates its left side, then its right; `anyOf` stops at the first condition that holds and `allOf` at the first that fails. A fault in a condition that is checked breaks the rule whatever the others would say, and `not` does not turn it into a pass. So an earlier condition guards a later one: `{ "anyOf": [{ "equal": ["b", 0] }, { "equal": [{ "divide": ["a", "b"] }, 2] }] }` holds for a `b` of 0 without dividing by it, while the same two conditions the other way round divide by zero and break the rule.

The parser (generation 3, meta-schema v3) checks the keyword on every parse, stored contracts included: the shape, that every path an operand reads names an integer or boolean property of the type, every path `length` or `byteLength` measures a string property, every path `count` counts an array or byte array property, every system time or height a rule reads is one the type lists in `required` (and an indexOnly type reads none), every path compared with a string names a string property (and every constant compared with one that declares an `enum` is one of its values) (a nested one by its dotted path) and every path `present` or `absent` tests names a property of the type, and that neither is `transient` nor inside a transient object (a transient value is never stored, so a stored document could not be held to the rule), that every comparison and `in` reads at least one property (a constant one would make its rule, or an `anyOf` around it, hold for every document or for none), that no `in` lists a value twice, that no `anyOf` or `allOf` holds one of its own kind directly and no `not` a `not`, that no literal divisor is 0 and no literal exponent negative, and that no condition or operand nests deeper than `MAX_PROPERTY_CONSTRAINT_PARSE_DEPTH` (64), a constant that keeps a parse without full validation from recursing without bound and that no registrable rule comes near. Under full validation, when a contract registers or updates, it also holds the limits: at most `SystemLimits::max_property_constraints` rules per type (16) and `max_property_constraint_nodes` nodes per rule (32), counting every comparison and logical operator, every `in` and each value it lists, every `const`, every `present` or `absent`, every arithmetic operator and every operand (a size is one), that no `anyOf` or `allOf` lists the same condition twice (conditions that parse alike, so `1` and `1.0` are the same value), and that the rules of one type read at most `max_property_constraint_aggregates` (4) distinct totals. Once every document type of the contract is parsed, a registration also checks every `countOf` and `sumOf`: the type it counts is one of the contract's and not indexOnly, a tree of it keeps the total, the filter's keys and values are integers, strings or identifiers of the same kind, and every property a value reads is required. The rules are fixed when the document type is created: adding, removing or changing one is an incompatible schema change (`IncompatibleDocumentTypeSchemaError`, 10246), since stored documents were judged against the rules as they were.

Enforcement lives in `DataContract::validate_document_properties` (generation 0, extended in place: the call is inert before protocol version 14, where `validate_property_constraints` is `None`), after the schema validation. Document create and replace structure validation call it, so consensus applies the rules, and so does every client that validates a document before sending it. The rules are checked in name order against the document's properties, which for a replace is the whole document, and the first one broken fails with `DocumentPropertyConstraintViolatedError` (basic code 10422), naming the document type, the rule and why: the rule does not hold, or evaluating it overflowed, divided by zero, raised to a negative power or read a value that is not an integer. The check changes nothing stored. It reads state only for the `countOf` and `sumOf` totals, which the document batch transformer reads when it builds the action (`Drive::fetch_property_constraint_aggregate`, billed with the write) and hands the check in `DocumentSystemValues::aggregates`; the limits bound its cost. `$ownerId` and the system times and heights read the values of the document version being written (`validate_document_properties` takes them as `DocumentSystemValues`): on a create the writer and the block's time and heights, on a replace the writer, the stored creation and transfer values and the block's as the update. A client passes what it knows: an owner it does not know equals no identifier, and a rule reading a time, a height or a total it is not given is not judged. The SDK pre-checks use the device clock for the times a write records, and leave a rule reading a block height unjudged, since the height is unknown until the block. Transfers and purchases change no property, only the owner and the transfer's time and heights, so only the rules reading those are judged again, with the new values (`DocumentTypeV0Methods::validate_property_constraints_for_system_change`, next to the `distinctFrom` check). Price updates change only the update's time and heights, and are judged against the rules reading those the same way.

In Rust the rules are `DocumentTypeV2Getters::property_constraints` (a map from name to `PropertyConstraint`: a comparison, an `in`, a string comparison (`TextCompare`, `TextIn`), a `present` or `absent`, or an `anyOf`, `allOf` or `not` of them, empty on types that predate the keyword; `property_reads` lists what a rule reads and how: by value, by presence, by size (`Length`, `Count`) or in a comparison of strings or identifiers, `system_reads` the system times and heights it reads, and `aggregate_reads` the totals, each an `AggregateRead`), each rule's `holds` and `violation` evaluate it against a document's data and `DocumentSystemValues`, and the document check is `DocumentTypeV0Methods::validate_property_constraints`.

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
