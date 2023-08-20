mod fields;
pub mod v0;

use crate::data_contract::created_data_contract::v0::CreatedDataContractV0;
use crate::prelude::DataContract;
use crate::version::PlatformVersion;
use crate::ProtocolError;
use derive_more::From;

use platform_value::{Bytes32, Value};

/// The created data contract is a intermediate structure that can be consumed by a
/// contract create state transition.
///
///

#[derive(Clone, Debug, From)]
pub enum CreatedDataContract {
    V0(CreatedDataContractV0),
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
            .created_data_contract_structure_version
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
            .created_data_contract_structure_version
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
