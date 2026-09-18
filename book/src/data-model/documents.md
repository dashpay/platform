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

- **`id`**: A 32-byte unique identifier. Unlike contract IDs, document IDs are *derived* from a combination of the contract ID, owner ID, document type name, and entropy. This makes them deterministic yet unique.

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
    pub fn generate_document_id_v0(
        contract_id: &Identifier,
        owner_id: &Identifier,
        document_type_name: &str,
        entropy: &[u8],
    ) -> Identifier {
        let mut buf: Vec<u8> = vec![];
        buf.extend_from_slice(&contract_id.to_buffer());
        buf.extend_from_slice(&owner_id.to_buffer());
        buf.extend_from_slice(document_type_name.as_bytes());
        buf.extend_from_slice(entropy);

        Identifier::from_bytes(&hash_double_to_vec(&buf)).unwrap()
    }
}
```

The ID is a double SHA-256 hash of the concatenation of the contract ID, owner ID, document type name, and client-provided entropy. This means:
- The same entropy in the same context always produces the same ID (deterministic).
- Different entropy always produces a different ID (unique in practice).
- The ID commits to both the contract and document type, preventing cross-contract collisions.

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

## Rules and Guidelines

**Do:**
- Always serialize and deserialize documents using their document type definition. The type determines field layout.
- Use `is_equal_ignoring_time_based_fields()` when comparing documents for validation purposes.
- Use `increment_revision()` rather than manually manipulating the revision field -- it handles overflow checking.
- Access properties through the accessor traits, not by reaching into the inner `DocumentV0` struct.

**Do not:**
- Assume a document carries its contract reference. The contract and document type are always passed as separate arguments.
- Manually construct document IDs. Use `generate_document_id_v0()` with proper entropy.
- Treat serialized document bytes as self-describing. Without the document type schema, the bytes are meaningless.
- Set time-based fields from client code. The platform sets `created_at`, `updated_at`, block heights, and similar fields during state transition processing.
