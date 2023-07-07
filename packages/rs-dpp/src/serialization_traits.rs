use crate::identity::KeyType;

#[cfg(feature = "validation")]
use crate::validation::SimpleConsensusValidationResult;
use crate::version::FeatureVersion;
use crate::{BlsModule, ProtocolError};
use platform_value::Value;

pub trait Signable {
    fn signable_bytes(&self) -> Result<Vec<u8>, ProtocolError>;
}

pub trait PlatformSerializable {
    fn serialize(&self) -> Result<Vec<u8>, ProtocolError>;

    /// If the trait is not used just do a simple serialize
    fn serialize_consume(self) -> Result<Vec<u8>, ProtocolError>
    where
        Self: Sized,
    {
        self.serialize()
    }
}

pub trait PlatformSerializableWithPrefixVersion {
    fn serialize_with_prefix_version(&self, feature_version: FeatureVersion) -> Result<Vec<u8>, ProtocolError>;

    /// If the trait is not used just do a simple serialize
    fn serialize_consume_with_prefix_version(self, feature_version: FeatureVersion) -> Result<Vec<u8>, ProtocolError>
        where
            Self: Sized,
    {
        self.serialize_with_prefix_version(feature_version)
    }
}


pub trait PlatformDeserializable {
    fn deserialize(data: &[u8]) -> Result<Self, ProtocolError>
    where
        Self: Sized;
}

pub trait PlatformSerializableIntoStructureVersion {
    /// Version based serialization is done based on the desired structure version.
    /// For example we have DataContractV0 and DataContractV1 for code based Contracts
    /// This means objects that will execute code
    /// And we would have DataContractSerializationFormatV0 and DataContractSerializationFormatV1
    /// which are the different ways to serialize the concept of a data contract.
    /// The data contract would call versioned_serialize. There should be a converted for each
    /// Data contract Version towards each DataContractSerializationFormat
    fn versioned_serialize(
        &self,
        structure_version: FeatureVersion,
    ) -> Result<Vec<u8>, ProtocolError>;

    /// If the trait is not used just do a simple serialize
    fn versioned_serialize_consume(
        self,
        structure_version: FeatureVersion,
    ) -> Result<Vec<u8>, ProtocolError>
    where
        Self: Sized,
    {
        self.versioned_serialize(structure_version)
    }
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
        system_version: FeatureVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized;
}

pub trait ValueConvertible {
    fn to_object(&self) -> Result<Value, ProtocolError>;

    fn into_object(self) -> Result<Value, ProtocolError>;

    fn from_object(value: Value) -> Result<Self, ProtocolError>
    where
        Self: Sized;

    fn from_object_ref(value: &Value) -> Result<Self, ProtocolError>
    where
        Self: Sized;
}

#[cfg(feature = "validation")]
pub trait PlatformMessageSignable {
    fn verify_signature(
        &self,
        public_key_type: KeyType,
        public_key_data: &[u8],
        signature: &[u8],
    ) -> Result<SimpleConsensusValidationResult, ProtocolError>;
    fn sign_by_private_key(
        &self,
        private_key: &[u8],
        key_type: KeyType,
        bls: &impl BlsModule,
    ) -> Result<Vec<u8>, ProtocolError>;
}
