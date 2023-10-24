mod fields;
pub mod v0;

use crate::data_contract::created_data_contract::v0::{
    CreatedDataContractInSerializationFormatV0, CreatedDataContractV0,
};
use crate::prelude::DataContract;
use crate::version::PlatformVersion;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use derive_more::From;

use crate::data_contract::serialized_version::DataContractInSerializationFormat;
use crate::serialization::{
    PlatformDeserializableWithPotentialValidationFromVersionedStructure,
    PlatformSerializableWithPlatformVersion,
};
use crate::ProtocolError::{PlatformDeserializationError, PlatformSerializationError};
use platform_value::{Bytes32, Value};
use platform_version::TryIntoPlatformVersioned;

/// The created data contract is a intermediate structure that can be consumed by a
/// contract create state transition.
///
///

#[derive(Clone, Debug, PartialEq, From)]
pub enum CreatedDataContract {
    V0(CreatedDataContractV0),
}

#[derive(Clone, Debug, Encode, Decode, From)]
pub enum CreatedDataContractInSerializationFormat {
    V0(CreatedDataContractInSerializationFormatV0),
}

impl PlatformSerializableWithPlatformVersion for CreatedDataContract {
    type Error = ProtocolError;

    fn serialize_to_bytes_with_platform_version(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, ProtocolError> {
        self.clone()
            .serialize_consume_to_bytes_with_platform_version(platform_version)
    }

    fn serialize_consume_to_bytes_with_platform_version(
        self,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, ProtocolError> {
        let (data_contract, entropy) = self.data_contract_and_entropy_owned();
        let data_contract_serialization_format: DataContractInSerializationFormat =
            data_contract.try_into_platform_versioned(platform_version)?;
        let created_data_contract_in_serialization_format = match platform_version
            .dpp
            .contract_versions
            .created_data_contract_structure
        {
            0 => Ok(CreatedDataContractInSerializationFormat::V0(
                CreatedDataContractInSerializationFormatV0 {
                    data_contract: data_contract_serialization_format,
                    entropy_used: entropy,
                },
            )),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "CreatedDataContract::serialize_to_bytes_with_platform_version".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }?;
        let config = bincode::config::standard()
            .with_big_endian()
            .with_no_limit();
        bincode::encode_to_vec(created_data_contract_in_serialization_format, config).map_err(|e| {
            PlatformSerializationError(format!("unable to serialize CreatedDataContract: {}", e))
        })
    }
}

impl PlatformDeserializableWithPotentialValidationFromVersionedStructure for CreatedDataContract {
    fn versioned_deserialize(
        data: &[u8],
        validate: bool,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized,
    {
        let config = bincode::config::standard()
            .with_big_endian()
            .with_no_limit();
        let created_data_contract_in_serialization_format: CreatedDataContractInSerializationFormat =
            bincode::borrow_decode_from_slice(data, config)
                .map_err(|e| {
                    PlatformDeserializationError(format!(
                        "unable to deserialize DataContract: {}",
                        e
                    ))
                })?
                .0;
        let (data_contract_in_serialization_format, entropy) =
            created_data_contract_in_serialization_format.data_contract_and_entropy_owned();
        let data_contract = DataContract::try_from_platform_versioned(
            data_contract_in_serialization_format,
            validate,
            platform_version,
        )?;
        match platform_version
            .dpp
            .contract_versions
            .created_data_contract_structure
        {
            0 => Ok(CreatedDataContract::V0(CreatedDataContractV0 {
                data_contract,
                entropy_used: entropy,
            })),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "CreatedDataContract::versioned_deserialize".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}

impl From<CreatedDataContract> for DataContract {
    fn from(value: CreatedDataContract) -> Self {
        match value {
            CreatedDataContract::V0(created_data_contract) => created_data_contract.data_contract,
        }
    }
}

impl CreatedDataContract {
    pub fn data_contract_owned(self) -> DataContract {
        match self {
            CreatedDataContract::V0(v0) => v0.data_contract,
        }
    }

    pub fn data_contract_and_entropy_owned(self) -> (DataContract, Bytes32) {
        match self {
            CreatedDataContract::V0(v0) => (v0.data_contract, v0.entropy_used),
        }
    }

    pub fn data_contract(&self) -> &DataContract {
        match self {
            CreatedDataContract::V0(v0) => &v0.data_contract,
        }
    }

    pub fn data_contract_mut(&mut self) -> &mut DataContract {
        match self {
            CreatedDataContract::V0(v0) => &mut v0.data_contract,
        }
    }

    pub fn entropy_used_owned(self) -> Bytes32 {
        match self {
            CreatedDataContract::V0(v0) => v0.entropy_used,
        }
    }

    pub fn entropy_used(&self) -> &Bytes32 {
        match self {
            CreatedDataContract::V0(v0) => &v0.entropy_used,
        }
    }

    #[cfg(test)]
    pub fn set_entropy_used(&mut self, entropy_used: Bytes32) {
        match self {
            CreatedDataContract::V0(v0) => v0.entropy_used = entropy_used,
        }
    }

    pub fn from_contract_and_entropy(
        data_contract: DataContract,
        entropy: Bytes32,
        platform_version: &PlatformVersion,
    ) -> Result<CreatedDataContract, ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .created_data_contract_structure
        {
            0 => Ok(CreatedDataContractV0 {
                data_contract,
                entropy_used: entropy,
            }
            .into()),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "CreatedDataContract::from_contract_and_entropy".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    #[cfg(feature = "data-contract-value-conversion")]
    pub fn from_object(
        raw_object: Value,
        validate: bool,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .created_data_contract_structure
        {
            0 => Ok(
                CreatedDataContractV0::from_object(raw_object, validate, platform_version)?.into(),
            ),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "CreatedDataContract::from_object".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}

impl CreatedDataContractInSerializationFormat {
    pub fn data_contract_and_entropy_owned(self) -> (DataContractInSerializationFormat, Bytes32) {
        match self {
            CreatedDataContractInSerializationFormat::V0(v0) => (v0.data_contract, v0.entropy_used),
        }
    }
}
