#[cfg(any(
    feature = "message-signature-verification",
    feature = "message-signing"
))]
use crate::identity::KeyType;

use serde::{Deserialize, Serialize};

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
        validate: bool,
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
        validate: bool,
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

pub trait ValueConvertible<'a>: Serialize + Deserialize<'a> {
    fn to_object(&self) -> Result<Value, ProtocolError>
    where
        Self: Sized + Clone,
    {
        platform_value::to_value(self.clone()).map_err(ProtocolError::ValueError)
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
