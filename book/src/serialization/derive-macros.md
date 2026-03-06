# Derive Macros

In the previous chapter we looked at the `PlatformVersionEncode` and `PlatformVersionedDecode` traits and the `platform_encode_to_vec` / `platform_versioned_decode_from_slice` functions. But you rarely implement those traits by hand. Instead, you use three derive macros from the `rs-platform-serialization-derive` crate:

- `PlatformSerialize` -- generates a high-level `serialize_to_bytes()` method
- `PlatformDeserialize` -- generates a high-level `deserialize_from_bytes()` method
- `PlatformSignable` -- generates a `signable_bytes()` method that excludes signature fields

These macros live in `packages/rs-platform-serialization-derive/src/lib.rs` and are the glue that connects Rust struct definitions to the platform serialization system.

## `PlatformSerialize` and `PlatformDeserialize`

These two macros work as a pair. Let us start with how they are used on the `ConsensusError` type:

```rust
#[derive(
    thiserror::Error,
    Debug,
    Encode,
    Decode,
    PlatformSerialize,
    PlatformDeserialize,
    Clone,
    PartialEq,
)]
#[platform_serialize(limit = 2000)]
#[error(transparent)]
pub enum ConsensusError {
    // ...
}
```

And on a leaf error struct:

```rust
#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Document {document_id} is already present")]
#[platform_serialize(unversioned)]
pub struct DocumentAlreadyPresentError {
    document_id: Identifier,
}
```

And on the `StateTransition` enum:

```rust
#[derive(
    Debug, Clone, Encode, Decode,
    PlatformSerialize, PlatformDeserialize, PlatformSignable,
    From, PartialEq,
)]
#[platform_serialize(unversioned)]
#[platform_serialize(limit = 100000)]
pub enum StateTransition {
    DataContractCreate(DataContractCreateTransition),
    DataContractUpdate(DataContractUpdateTransition),
    Batch(BatchTransition),
    // ...
}
```

Notice that `StateTransition` uses **two** `#[platform_serialize]` attributes -- one for `unversioned` and one for `limit`. These are combined internally.

## What the derives generate

`PlatformSerialize` generates an implementation of one of two traits, depending on whether `unversioned` is set:

**With `unversioned`** -- implements `PlatformSerializable`:

```rust
// Generated code (simplified):
impl PlatformSerializable for DocumentAlreadyPresentError {
    type Error = ProtocolError;

    fn serialize_to_bytes(&self) -> Result<Vec<u8>, Self::Error> {
        let config = bincode::config::standard()
            .with_big_endian()
            .with_no_limit();
        bincode::encode_to_vec(self, config)
            .map_err(|e| {
                ProtocolError::PlatformSerializationError(
                    format!("unable to serialize DocumentAlreadyPresentError: {}", e)
                )
            })
    }

    fn serialize_consume_to_bytes(self) -> Result<Vec<u8>, Self::Error> {
        // same as above, taking self by value
    }
}
```

**Without `unversioned`** -- implements `PlatformSerializableWithPlatformVersion`:

```rust
// Generated code (simplified):
impl PlatformSerializableWithPlatformVersion for ConsensusError {
    type Error = ProtocolError;

    fn serialize_to_bytes_with_platform_version(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, ProtocolError> {
        let config = bincode::config::standard()
            .with_big_endian()
            .with_limit::<{ 2000 }>();
        platform_serialization::platform_encode_to_vec(self, config, platform_version)
            .map_err(|e| match e {
                bincode::error::EncodeError::Io { inner, index } =>
                    ProtocolError::MaxEncodedBytesReachedError {
                        max_size_kbytes: 2000,
                        size_hit: index,
                    },
                _ => ProtocolError::PlatformSerializationError(
                    format!("unable to serialize ConsensusError: {}", e)
                ),
            })
    }
}
```

The key differences: the `unversioned` variant uses plain `bincode::encode_to_vec`, while the versioned variant uses `platform_serialization::platform_encode_to_vec` which threads the `PlatformVersion` through the encode chain. When a `limit` is set, the config uses `with_limit::<{ N }>()` and the error mapping converts bincode IO errors to `MaxEncodedBytesReachedError`.

In both cases, the derive also generates bincode `Encode` and `Decode` implementations by internally calling into `derive_bincode` -- this is why you still need `Encode` and `Decode` in the derive list alongside `PlatformSerialize` and `PlatformDeserialize`.

## The `#[platform_serialize]` attributes

The `#[platform_serialize(...)]` attribute accepts several parameters. Here is the full list from the derive macro source in `packages/rs-platform-serialization-derive/src/lib.rs`:

### `limit = N`

Sets the maximum serialized size in bytes:

```rust
#[platform_serialize(limit = 2000)]
```

When encoding exceeds this limit, the error is `MaxEncodedBytesReachedError`. This is critical for types received from the network.

### `unversioned`

Generates `PlatformSerializable` instead of `PlatformSerializableWithPlatformVersion`:

```rust
#[platform_serialize(unversioned)]
```

Use this when the type's serialization format does not change between protocol versions. Most leaf types (individual error structs, simple data holders) use this.

### `passthrough`

For enums, serializes by delegating directly to the inner variant's serialization method:

```rust
#[platform_serialize(passthrough)]
```

When `MyEnum::Variant(inner)` is serialized with passthrough, it calls `inner.serialize()` directly rather than encoding the enum tag + inner data. This means the enum variant information is lost in serialization -- deserialization must use `platform_version_path` to know which variant to decode into.

Cannot be combined with `limit`, `untagged`, or `into`.

### `untagged`

For enums, serializes without the variant tag number:

```rust
#[platform_serialize(untagged)]
```

Similar to passthrough but still uses the enum's own encoding logic rather than delegating to the inner type. The variant index is omitted from the output. Like passthrough, deserialization requires knowing which variant to expect.

Cannot be combined with `passthrough`.

### `into = "TypePath"`

For structs, converts the value to another type before serialization:

```rust
#[platform_serialize(into = "DataContractInSerializationFormat")]
```

The generated code calls `.into()` to convert to the target type, then serializes that type. This is useful for types that have a different in-memory representation than their serialization format.

Cannot be used on enums.

### `platform_version_path = "..."`

Used with `passthrough` or `untagged` to specify how to determine the correct variant during deserialization:

```rust
#[platform_serialize(
    passthrough,
    platform_version_path = "dpp.contract_versions.contract_serialization_version.default_current_version"
)]
```

The deserialization code reads this field path from the `PlatformVersion` to determine which variant index to decode.

### `crate_name = "..."`

Overrides the default crate path (defaults to `crate`):

```rust
#[platform_serialize(crate_name = "dpp")]
```

### `allow_prepend_version` and `force_prepend_version`

These flags are defined in the code but currently not actively used in the main serialization path. They were designed for prepending version bytes to serialized output. Only one can be used at a time.

## The `#[platform_error_type]` attribute

By default, the derives use `ProtocolError` as the error type. You can override this:

```rust
#[derive(PlatformSerialize, PlatformDeserialize)]
#[platform_error_type(MyCustomError)]
pub struct MyType { ... }
```

The error type must have `PlatformSerializationError(String)` and `MaxEncodedBytesReachedError { max_size_kbytes, size_hit }` variants (or equivalent conversion paths).

## `PlatformSignable` -- the signature hash derive

The `PlatformSignable` derive solves a specific problem: when you sign a state transition, you need to hash all the fields **except** the signature itself. You cannot include the signature in the data that was signed -- that would be circular.

Here is how it is used on `DataContractCreateTransitionV0` in `packages/rs-dpp/src/state_transition/state_transitions/contract/data_contract_create_transition/v0/mod.rs`:

```rust
#[derive(Debug, Clone, Encode, Decode, PartialEq, PlatformSignable)]
pub struct DataContractCreateTransitionV0 {
    pub data_contract: DataContractInSerializationFormat,
    pub identity_nonce: IdentityNonce,
    pub user_fee_increase: UserFeeIncrease,
    #[platform_signable(exclude_from_sig_hash)]
    pub signature_public_key_id: KeyID,
    #[platform_signable(exclude_from_sig_hash)]
    pub signature: BinaryData,
}
```

The `#[platform_signable(exclude_from_sig_hash)]` attribute marks fields that should be excluded from the signature hash. The derive generates:

1. A new struct `DataContractCreateTransitionV0Signable<'a>` containing only the non-excluded fields as `Cow` references
2. A `From<&DataContractCreateTransitionV0>` implementation for the signable struct
3. An implementation of the `Signable` trait on the original struct

The generated code looks approximately like this:

```rust
// Generated by PlatformSignable derive:

#[derive(Debug, Clone, bincode::Encode)]
pub struct DataContractCreateTransitionV0Signable<'a> {
    data_contract: std::borrow::Cow<'a, DataContractInSerializationFormat>,
    identity_nonce: std::borrow::Cow<'a, IdentityNonce>,
    user_fee_increase: std::borrow::Cow<'a, UserFeeIncrease>,
    // signature_public_key_id -- excluded
    // signature -- excluded
}

impl<'a> From<&'a DataContractCreateTransitionV0>
    for DataContractCreateTransitionV0Signable<'a>
{
    fn from(original: &'a DataContractCreateTransitionV0) -> Self {
        DataContractCreateTransitionV0Signable {
            data_contract: std::borrow::Cow::Borrowed(&original.data_contract),
            identity_nonce: std::borrow::Cow::Borrowed(&original.identity_nonce),
            user_fee_increase: std::borrow::Cow::Borrowed(&original.user_fee_increase),
        }
    }
}

impl crate::serialization::Signable for DataContractCreateTransitionV0 {
    fn signable_bytes(&self) -> Result<Vec<u8>, ProtocolError> {
        let config = bincode::config::standard().with_big_endian();
        let intermediate: DataContractCreateTransitionV0Signable = self.into();
        bincode::encode_to_vec(intermediate, config).map_err(|e| {
            ProtocolError::PlatformSerializationError(
                format!("unable to serialize to produce sig hash \
                    DataContractCreateTransitionV0: {}", e)
            )
        })
    }
}
```

The `Cow` references avoid cloning the data just to produce a hash. The intermediate struct borrows from the original and only serializes the fields that matter for the signature.

### `PlatformSignable` on enums

When used on an enum (like `StateTransition`), the derive generates a corresponding `Signable` enum where each variant wraps the signable version of its inner type:

```rust
// On StateTransition:
#[derive(PlatformSignable)]
pub enum StateTransition {
    DataContractCreate(DataContractCreateTransition),
    DataContractUpdate(DataContractUpdateTransition),
    // ...
}

// Generates:
#[derive(Debug, Clone, bincode::Encode, derive_more::From)]
pub enum StateTransitionSignable<'a> {
    DataContractCreate(DataContractCreateTransitionSignable<'a>),
    DataContractUpdate(DataContractUpdateTransitionSignable<'a>),
    // ...
}
```

The `signable_bytes` implementation on the enum encodes a variant index (as `u16`) followed by the inner type's signable bytes:

```rust
impl Signable for StateTransition {
    fn signable_bytes(&self) -> Result<Vec<u8>, ProtocolError> {
        let config = bincode::config::standard().with_big_endian();
        let signable_bytes = match self {
            StateTransition::DataContractCreate(ref inner) => {
                let mut buf = bincode::encode_to_vec(&(0u16), config).unwrap();
                let inner_signable_bytes = inner.signable_bytes()?;
                buf.extend(inner_signable_bytes);
                buf
            }
            StateTransition::DataContractUpdate(ref inner) => {
                let mut buf = bincode::encode_to_vec(&(1u16), config).unwrap();
                let inner_signable_bytes = inner.signable_bytes()?;
                buf.extend(inner_signable_bytes);
                buf
            }
            // ...
        };
        Ok(signable_bytes)
    }
}
```

### `PlatformSignable` attributes

- `exclude_from_sig_hash` -- marks a field to be excluded from the signable struct
- `into = "TypePath"` -- converts a field to a different type in the signable struct (used for fields that need a different representation for hashing)
- `derive_into` -- on enums, generates `From` conversions from the original enum to the signable enum
- `derive_bincode_with_borrowed_vec` -- manually implements `bincode::Encode` for the signable struct instead of deriving it (needed when fields contain borrowed `Vec` types)

## The relationship between Encode/Decode and PlatformSerialize/PlatformDeserialize

This is a common source of confusion. Here is how the pieces fit together:

- **`Encode` / `Decode`** (from bincode) -- field-level encoding. These know how to write each field to bytes and read it back. They are the low-level building blocks.

- **`PlatformVersionEncode` / `PlatformVersionedDecode`** (from rs-platform-serialization) -- version-aware field-level encoding. These wrap `Encode`/`Decode` with a `PlatformVersion` parameter.

- **`PlatformSerialize` / `PlatformDeserialize`** (derive macros) -- high-level serialization. These generate the `serialize_to_bytes()` and `deserialize_from_bytes()` methods that configure bincode, enforce size limits, and convert errors.

When you derive all of them on a type, the call chain is:

```
your_type.serialize_to_bytes()           // PlatformSerialize-generated method
  -> platform_encode_to_vec(...)         // from rs-platform-serialization
    -> your_type.platform_encode(...)    // PlatformVersionEncode (auto-generated)
      -> field.encode(encoder)           // bincode Encode for each field
```

You need `Encode` and `Decode` in the derive list alongside `PlatformSerialize` and `PlatformDeserialize` because the platform derives internally delegate to bincode encoding.

## Rules

**Do:**
- Always derive `Encode, Decode, PlatformSerialize, PlatformDeserialize` together
- Use `#[platform_serialize(unversioned)]` for types with a stable serialization format
- Use `#[platform_serialize(limit = N)]` on types received from untrusted sources
- Use `#[platform_signable(exclude_from_sig_hash)]` on signature and signature key ID fields
- Keep the `#[platform_error_type]` attribute consistent with the crate's error type

**Do not:**
- Use `passthrough` on structs (it is enum-only)
- Use `into` on enums (it is struct-only)
- Combine `passthrough` with `limit`, `untagged`, or `into`
- Combine `force_prepend_version` with `allow_prepend_version`
- Forget that `PlatformSignable` on enums requires each inner type to also implement `Signable`
- Change the order of fields in a `PlatformSignable` struct -- this changes the signature hash, which invalidates existing signatures
