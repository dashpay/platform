#[cfg(any(
    feature = "message-signature-verification",
    feature = "message-signing"
))]
use crate::identity::KeyType;

use serde::de::DeserializeOwned;
use serde::Serialize;
#[cfg(feature = "json-conversion")]
use serde_json::Value as JsonValue;

#[cfg(feature = "message-signature-verification")]
use crate::validation::SimpleConsensusValidationResult;
use crate::version::PlatformVersion;
#[cfg(feature = "message-signing")]
use crate::BlsModule;
use crate::ProtocolError;
use platform_value::Value;

pub trait Signable {
    fn signable_bytes(&self) -> Result<Vec<u8>, ProtocolError>;
}

pub trait PlatformSerializable {
    type Error;
    fn serialize_to_bytes(&self) -> Result<Vec<u8>, Self::Error>;

    /// If the trait is not used just do a simple serialize
    fn serialize_consume_to_bytes(self) -> Result<Vec<u8>, Self::Error>
    where
        Self: Sized,
    {
        self.serialize_to_bytes()
    }
}

pub trait PlatformSerializableWithPlatformVersion {
    type Error;
    /// Version based serialization is done based on the desired structure version.
    /// For example we have DataContractV0 and DataContractV1 for code based Contracts
    /// This means objects that will execute code
    /// And we would have DataContractSerializationFormatV0 and DataContractSerializationFormatV1
    /// which are the different ways to serialize the concept of a data contract.
    /// The data contract would call versioned_serialize. There should be a converted for each
    /// Data contract Version towards each DataContractSerializationFormat
    fn serialize_to_bytes_with_platform_version(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Self::Error>;

    /// If the trait is not used just do a simple serialize
    fn serialize_consume_to_bytes_with_platform_version(
        self,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Self::Error>
    where
        Self: Sized,
    {
        self.serialize_to_bytes_with_platform_version(platform_version)
    }
}

pub trait PlatformDeserializable {
    fn deserialize_from_bytes(data: &[u8]) -> Result<Self, ProtocolError>
    where
        Self: Sized,
    {
        Self::deserialize_from_bytes_no_limit(data)
    }

    fn deserialize_from_bytes_no_limit(data: &[u8]) -> Result<Self, ProtocolError>
    where
        Self: Sized;
}

pub trait PlatformDeserializableFromVersionedStructure {
    /// We will deserialize a versioned structure into a code structure
    /// For example we have DataContractV0 and DataContractV1
    /// The system version will tell which version to deserialize into
    /// This happens by first deserializing the data into a potentially versioned structure
    /// For example we could have DataContractSerializationFormatV0 and DataContractSerializationFormatV1
    /// Both of the structures will be valid in perpetuity as they are saved into the state.
    /// So from the bytes we could get DataContractSerializationFormatV0.
    /// Then the system_version given will tell to transform DataContractSerializationFormatV0 into
    /// DataContractV1 (if system version is 1)
    fn versioned_deserialize(
        data: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized;
}

pub trait PlatformDeserializableWithPotentialValidationFromVersionedStructure {
    /// We will deserialize a versioned structure into a code structure
    /// For example we have DataContractV0 and DataContractV1
    /// The system version will tell which version to deserialize into
    /// This happens by first deserializing the data into a potentially versioned structure
    /// For example we could have DataContractSerializationFormatV0 and DataContractSerializationFormatV1
    /// Both of the structures will be valid in perpetuity as they are saved into the state.
    /// So from the bytes we could get DataContractSerializationFormatV0.
    /// Then the system_version given will tell to transform DataContractSerializationFormatV0 into
    /// DataContractV1 (if system version is 1)
    fn versioned_deserialize(
        data: &[u8],
        full_validation: bool,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized;
}

pub trait PlatformDeserializableWithBytesLenFromVersionedStructure {
    /// We will deserialize a versioned structure into a code structure
    /// For example we have DataContractV0 and DataContractV1
    /// The system version will tell which version to deserialize into
    /// This happens by first deserializing the data into a potentially versioned structure
    /// For example we could have DataContractSerializationFormatV0 and DataContractSerializationFormatV1
    /// Both of the structures will be valid in perpetuity as they are saved into the state.
    /// So from the bytes we could get DataContractSerializationFormatV0.
    /// Then the system_version given will tell to transform DataContractSerializationFormatV0 into
    /// DataContractV1 (if system version is 1)
    fn versioned_deserialize_with_bytes_len(
        data: &[u8],
        full_validation: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(Self, usize), ProtocolError>
    where
        Self: Sized;
}

pub trait PlatformLimitDeserializableFromVersionedStructure {
    fn versioned_limit_deserialize(
        data: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized;
}

/// Convert to/from `platform_value::Value` using **non-human-readable** serde
/// (`Identifier` = `Value::Identifier(bytes)`, binary = `Value::Bytes(bytes)`,
/// raw byte fields preserved without stringification).
///
/// # ⚠️ HR / non-HR divergence (Critical-1)
///
/// `ValueConvertible` calls `platform_value::to_value`, which uses a serializer
/// that reports `is_human_readable() == false`. The mirror trait
/// [`JsonConvertible`] uses `serde_json::to_value`, which reports `true`.
/// Types whose `Serialize` impl branches on `is_human_readable()` produce
/// **structurally different output** between the two paths:
///
/// | Type | `to_json()` (HR) | `to_object()` (non-HR) |
/// |---|---|---|
/// | [`platform_value::Identifier`] | `"5bV6jUfh..."` (bs58 string) | `Value::Identifier([u8; 32])` |
/// | [`platform_value::BinaryData`] | `"sg=="` (base64 string) | `Value::Bytes(Vec<u8>)` |
/// | `Bytes20` / `Bytes32` / `Bytes36` | base64 string | `Value::Bytes32([u8; N])` etc. |
/// | `CoreScript` | `"dqkU..."` (base64 string) | `Value::Bytes(Vec<u8>)` |
///
/// **Do not assume** `self.to_object()?.try_into_json()` ≡ `self.to_json()`.
/// They render the same field as a string in one and a byte array in the
/// other. Round-trip tests should exercise each path independently.
///
/// # ⚠️ `ContentDeserializer` caveat
///
/// Manual `Deserialize` impls that branch on `deserializer.is_human_readable()`
/// must also handle `serde::__private::de::ContentDeserializer`, used
/// internally by `#[serde(tag = "...")]` enums. ContentDeserializer **always
/// reports `is_human_readable: true`** regardless of the original source, so
/// a non-HR `platform_value::Value` flowing into a tagged enum gets shape-
/// inferred as if it were HR. Recipe: write a dual-shape visitor accepting
/// both shapes in the HR branch via `deserialize_any`. See
/// [`platform_value::Bytes32::deserialize`] for the canonical example, and
/// `rs-dpp/src/serialization/serde_bytes.rs` for `[u8; N]` / `Vec<u8>`.
#[cfg(feature = "value-conversion")]
pub trait ValueConvertible: Serialize + DeserializeOwned {
    fn to_object(&self) -> Result<Value, ProtocolError>
    where
        Self: Sized,
    {
        platform_value::to_value(self).map_err(ProtocolError::ValueError)
    }

    fn into_object(self) -> Result<Value, ProtocolError>
    where
        Self: Sized,
    {
        platform_value::to_value(self).map_err(ProtocolError::ValueError)
    }

    fn from_object(value: Value) -> Result<Self, ProtocolError>
    where
        Self: Sized,
    {
        platform_value::from_value(value).map_err(ProtocolError::ValueError)
    }

    fn from_object_ref(value: &Value) -> Result<Self, ProtocolError>
    where
        Self: Sized,
    {
        platform_value::from_value(value.clone()).map_err(ProtocolError::ValueError)
    }
}

/// Convert to/from JSON using **human-readable** serde (`Identifier` = base58,
/// binary = base64).
///
/// This trait produces clean `serde_json::Value` with native number types.
/// Any JS-boundary concerns (large number stringification) are handled by the
/// WASM layer.
///
/// # ⚠️ HR / non-HR divergence (Critical-1)
///
/// `JsonConvertible` calls `serde_json::to_value`, which uses a serializer
/// that reports `is_human_readable() == true`. The mirror trait
/// [`ValueConvertible`] uses `platform_value::to_value`, which reports
/// `false`. Types whose `Serialize` impl branches on `is_human_readable()`
/// produce **structurally different output** between the two paths:
///
/// | Type | `to_json()` (HR) | `to_object()` (non-HR) |
/// |---|---|---|
/// | [`platform_value::Identifier`] | `"5bV6jUfh..."` (bs58 string) | `Value::Identifier([u8; 32])` |
/// | [`platform_value::BinaryData`] | `"sg=="` (base64 string) | `Value::Bytes(Vec<u8>)` |
/// | `Bytes20` / `Bytes32` / `Bytes36` | base64 string | `Value::Bytes32([u8; N])` etc. |
/// | `CoreScript` | `"dqkU..."` (base64 string) | `Value::Bytes(Vec<u8>)` |
///
/// **Do not assume** `self.to_object()?.try_into_json()` ≡ `self.to_json()`.
/// They render the same field as a string in one and a byte array in the
/// other. Round-trip tests should exercise each path independently.
///
/// # ⚠️ `ContentDeserializer` caveat
///
/// Manual `Deserialize` impls that branch on `deserializer.is_human_readable()`
/// must also handle `serde::__private::de::ContentDeserializer`, used
/// internally by `#[serde(tag = "...")]` enums. ContentDeserializer **always
/// reports `is_human_readable: true`** regardless of the original source — so
/// a non-HR `platform_value::Value` flowing into a tagged enum gets shape-
/// inferred as if it were HR. Recipe: write a dual-shape visitor accepting
/// both shapes in the HR branch via `deserialize_any`. See
/// [`platform_value::Bytes32::deserialize`] for the canonical example, and
/// `rs-dpp/src/serialization/serde_bytes.rs` for `[u8; N]` / `Vec<u8>`.
#[cfg(feature = "json-conversion")]
pub trait JsonConvertible: Serialize + DeserializeOwned {
    fn to_json(&self) -> Result<JsonValue, ProtocolError> {
        serde_json::to_value(self).map_err(|e| ProtocolError::EncodingError(e.to_string()))
    }

    fn from_json(json: JsonValue) -> Result<Self, ProtocolError> {
        serde_json::from_value(json).map_err(|e| ProtocolError::DecodingError(e.to_string()))
    }
}

pub trait PlatformMessageSignable {
    #[cfg(feature = "message-signature-verification")]
    fn verify_signature(
        &self,
        public_key_type: KeyType,
        public_key_data: &[u8],
        signature: &[u8],
    ) -> SimpleConsensusValidationResult;

    #[cfg(feature = "message-signing")]
    fn sign_by_private_key(
        &self,
        private_key: &[u8],
        key_type: KeyType,
        bls: &impl BlsModule,
    ) -> Result<Vec<u8>, ProtocolError>;
}
