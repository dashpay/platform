# Data Contracts

If you have ever built a traditional web application, you know the pattern: define a database schema, then write code that reads and writes data conforming to that schema. Dash Platform follows the same idea, but on a decentralized network. A **data contract** is the on-chain schema that defines what application data looks like, how it is indexed, and who can modify it.

Every application on Dash Platform -- whether it is DPNS (the naming service), DashPay (social payments), or your own custom dApp -- starts by registering a data contract. Once registered, users can create, update, and query *documents* that conform to that contract's schema. Think of the data contract as the `CREATE TABLE` statement and documents as the rows.

## The DataContract Enum

Like almost every core type in Platform, `DataContract` is a versioned enum. You will find its definition in `packages/rs-dpp/src/data_contract/mod.rs`:

```rust
#[derive(Debug, Clone, PartialEq, From, PlatformVersioned)]
pub enum DataContract {
    V0(DataContractV0),
    V1(DataContractV1),
}
```

This is a pattern you will see over and over: the top-level type is an enum whose variants are concrete struct versions. The `PlatformVersioned` derive macro wires it into the protocol versioning system, and the `From` derive gives you free `.into()` conversions from either variant.

The enum also provides direct-access helpers for when you know exactly which version you are dealing with:

```rust
impl DataContract {
    pub fn as_v0(&self) -> Option<&DataContractV0> { ... }
    pub fn as_v1(&self) -> Option<&DataContractV1> { ... }
    pub fn into_v0(self) -> Option<DataContractV0> { ... }
    pub fn into_v1(self) -> Option<DataContractV1> { ... }
}
```

In tests, there are convenience methods `as_latest()` and `into_latest()` that always return the most recent variant. These are gated behind `#[cfg(test)]` because production code should never assume which version is "latest" -- the protocol version determines that.

## What Lives Inside a Data Contract

The V0 struct contains the essentials. From `packages/rs-dpp/src/data_contract/v0/data_contract.rs`:

```rust
pub struct DataContractV0 {
    pub(crate) id: Identifier,
    pub(crate) version: u32,
    pub(crate) owner_id: Identifier,
    pub document_types: BTreeMap<DocumentName, DocumentType>,
    pub(crate) metadata: Option<Metadata>,
    pub(crate) config: DataContractConfig,
    pub(crate) schema_defs: Option<BTreeMap<DefinitionName, Value>>,
}
```

The key fields are:

- **`id`**: A 32-byte identifier derived from the contract creation transaction. Globally unique.
- **`version`**: A monotonically increasing counter. Every time the contract owner updates the contract, this increments.
- **`owner_id`**: The identity that created (and can update) this contract.
- **`document_types`**: A map from document type names (like `"contactRequest"` or `"domain"`) to their `DocumentType` definitions, which include the JSON Schema, indexes, and mutability rules.
- **`config`**: Contract-level configuration such as whether documents can be deleted, encryption key requirements, and so on.
- **`schema_defs`**: Shared JSON Schema `$defs` that document types can reference.

## What V1 Added

`DataContractV1` extends V0 with several important capabilities. From `packages/rs-dpp/src/data_contract/v1/data_contract.rs`:

```rust
pub struct DataContractV1 {
    // All V0 fields...
    pub id: Identifier,
    pub version: u32,
    pub owner_id: Identifier,
    pub document_types: BTreeMap<DocumentName, DocumentType>,
    pub config: DataContractConfig,
    pub schema_defs: Option<BTreeMap<DefinitionName, Value>>,

    // New in V1:
    pub created_at: Option<TimestampMillis>,
    pub updated_at: Option<TimestampMillis>,
    pub created_at_block_height: Option<BlockHeight>,
    pub updated_at_block_height: Option<BlockHeight>,
    pub created_at_epoch: Option<EpochIndex>,
    pub updated_at_epoch: Option<EpochIndex>,
    pub groups: BTreeMap<GroupContractPosition, Group>,
    pub tokens: BTreeMap<TokenContractPosition, TokenConfiguration>,
    pub keywords: Vec<String>,
    pub description: Option<String>,
}
```

The additions fall into four categories:

1. **Timestamps and block tracking** -- `created_at`, `updated_at`, `created_at_block_height`, `updated_at_block_height`, `created_at_epoch`, `updated_at_epoch`. These provide an immutable audit trail of when the contract was created and last modified.

2. **Groups** -- `BTreeMap<GroupContractPosition, Group>`. Groups enable multiparty governance. Each group has a set of member identities with associated voting power and a required power threshold for actions.

3. **Tokens** -- `BTreeMap<TokenContractPosition, TokenConfiguration>`. Contracts can now define and manage tokens with configurable supply limits, minting/burning rules, and governance controls.

4. **Searchability** -- `keywords` and `description` make contracts discoverable through the platform's `search` system contract.

## The Versioned Accessors Pattern

Here is where it gets interesting. You do not access data contract fields directly in most code. Instead, you go through *accessor traits*. These live in `packages/rs-dpp/src/data_contract/accessors/`.

The V0 getter trait, defined in `accessors/v0/mod.rs`:

```rust
pub trait DataContractV0Getters {
    fn id(&self) -> Identifier;
    fn id_ref(&self) -> &Identifier;
    fn version(&self) -> u32;
    fn owner_id(&self) -> Identifier;
    fn document_type_for_name(&self, name: &str)
        -> Result<DocumentTypeRef<'_>, DataContractError>;
    fn document_types(&self) -> &BTreeMap<DocumentName, DocumentType>;
    fn config(&self) -> &DataContractConfig;
    // ... and more
}

pub trait DataContractV0Setters {
    fn set_id(&mut self, id: Identifier);
    fn set_version(&mut self, version: u32);
    fn increment_version(&mut self);
    fn set_owner_id(&mut self, owner_id: Identifier);
    fn set_config(&mut self, config: DataContractConfig);
}
```

And the V1 getter trait extends V0, defined in `accessors/v1/mod.rs`:

```rust
pub trait DataContractV1Getters: DataContractV0Getters {
    fn groups(&self) -> &BTreeMap<GroupContractPosition, Group>;
    fn tokens(&self) -> &BTreeMap<TokenContractPosition, TokenConfiguration>;
    fn created_at(&self) -> Option<TimestampMillis>;
    fn updated_at(&self) -> Option<TimestampMillis>;
    fn keywords(&self) -> &Vec<String>;
    fn description(&self) -> Option<&String>;
    // ... and more
}
```

Notice that `DataContractV1Getters` has a supertrait bound on `DataContractV0Getters`. This means anything that implements V1 getters automatically has V0 getters too. You can use both interchangeably.

## How the Enum Dispatches

The magic happens in `packages/rs-dpp/src/data_contract/accessors/mod.rs`, where the top-level `DataContract` enum implements both traits by dispatching to the inner variant:

```rust
impl DataContractV0Getters for DataContract {
    fn id(&self) -> Identifier {
        match self {
            DataContract::V0(v0) => v0.id(),
            DataContract::V1(v1) => v1.id(),
        }
    }

    fn version(&self) -> u32 {
        match self {
            DataContract::V0(v0) => v0.version(),
            DataContract::V1(v1) => v1.version(),
        }
    }
    // ... every method follows this pattern
}
```

For V1-only fields, the implementation gracefully handles V0 contracts:

```rust
impl DataContractV1Getters for DataContract {
    fn groups(&self) -> &BTreeMap<GroupContractPosition, Group> {
        match self {
            DataContract::V0(_) => &EMPTY_GROUPS,  // static empty map
            DataContract::V1(v1) => &v1.groups,
        }
    }

    fn tokens(&self) -> &BTreeMap<TokenContractPosition, TokenConfiguration> {
        match self {
            DataContract::V0(_) => &EMPTY_TOKENS,  // static empty map
            DataContract::V1(v1) => &v1.tokens,
        }
    }

    fn created_at(&self) -> Option<TimestampMillis> {
        match self {
            DataContract::V0(_) => None,
            DataContract::V1(v1) => v1.created_at,
        }
    }
}
```

This is a deliberate design choice. When code asks a V0 contract for its groups, it gets an empty map rather than an error. When it asks for a timestamp, it gets `None`. The calling code does not need to know or care which version it is working with -- it just checks whether the `Option` has a value or whether the map is empty.

## Serialization Strategy

Data contracts use a two-step serialization approach. They are first converted to a `DataContractInSerializationFormat` (a common intermediate representation), then serialized to bytes using bincode with big-endian encoding:

```rust
impl PlatformSerializableWithPlatformVersion for DataContract {
    fn serialize_to_bytes_with_platform_version(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, ProtocolError> {
        let serialization_format: DataContractInSerializationFormat =
            self.try_into_platform_versioned(platform_version)?;
        let config = bincode::config::standard()
            .with_big_endian()
            .with_no_limit();
        bincode::encode_to_vec(serialization_format, config)
            .map_err(|e| PlatformSerializationError(
                format!("unable to serialize DataContract: {}", e)
            ))
    }
}
```

This intermediate format is important because serialization versions and code structure versions are independent. A contract serialized as V1 ten years ago must still be deserializable, even if the code structures have evolved to V5 by then. The serialization format acts as the bridge.

There is also `versioned_limit_deserialize`, which imposes a size limit and always performs full validation -- this is used for data coming from untrusted sources (anything not from Drive's own storage).

## Evolving a Contract: Adding Required Fields

Contract updates are deliberately conservative: existing documents must stay valid and their stored bytes must stay readable, so most schema changes that would break either are rejected. Historically that froze the `required` set of a document type in both directions — requiredness is baked into the document wire format (required properties serialize raw, optional ones carry a presence flag), so changing it would desynchronize every stored document's bytes from the schema used to read them.

From protocol v14, an update **may add a brand-new required property** by annotating it with `requiredSince` equal to the contract version the update creates:

```json
"properties": {
  "newField": { "type": "string", "maxLength": 63, "position": 4, "requiredSince": 3 }
},
"required": ["existingField", "newField"]
```

The rules, enforced by consensus (`DataContractInvalidRequiredFieldsUpdateError`, code 10276, on violation):

- The annotation must name **exactly the new contract version** — requiredness can be neither pre-scheduled for a future version nor backdated.
- Only **brand-new properties** can become required. Promoting an existing (optional) property is still rejected, as is removing anything from `required` or touching an existing `requiredSince` annotation.
- On contract **creation**, `requiredSince` may only be `1`.
- Annotations sit on **top-level properties** listed in `required`; nested properties cannot carry them.

What happens to data:

- **Existing documents are grandfathered.** Each document carries a *contract version stamp* recording the contract version its bytes conform to (see [Document Serialization](../serialization/document-serialization.md)); a document stamped below a property's `requiredSince` may omit that property and still reads, transfers, and deletes normally.
- **New writes are held to the new schema.** Creates must supply the property; replaces re-supply full content, so replacing a grandfathered document requires the new property and re-stamps the document at the current version — lazy migration, one document at a time.
- **Indexes are unaffected** because index additions on update remain banned — a newly added required field cannot be indexed retroactively (there is no backfill).

## Rules and Guidelines

**Do:**
- Always access fields through the accessor traits (`DataContractV0Getters`, `DataContractV1Getters`), not by pattern-matching on the enum variant.
- Handle V1-only fields being `None` or empty when the contract might be V0.
- Use `document_type_for_name()` to retrieve document types -- it returns a proper error if the name does not exist.

**Do not:**
- Use `as_v0()` / `as_v1()` in production code unless you genuinely need version-specific behavior. The trait accessors are the right abstraction.
- Assume `into_latest()` exists outside of tests -- it is `#[cfg(test)]` only.
- Forget that serialization versions persist forever. If you add a new field, old serialized contracts will not have it, and your deserialization must handle that gracefully.
- Mutate a contract's `id` after creation -- it is derived from the creation transaction and must remain stable.
