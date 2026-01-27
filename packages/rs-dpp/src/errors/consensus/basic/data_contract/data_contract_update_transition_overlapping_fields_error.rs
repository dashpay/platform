use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_value::Identifier;
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Data contract update transition {data_contract_id} has overlapping {field_type}: '{overlapping_key}' appears in both updated and new fields.")]
#[platform_serialize(unversioned)]
pub struct DataContractUpdateTransitionOverlappingFieldsError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    data_contract_id: Identifier,
    field_type: String,
    overlapping_key: String,
}

impl DataContractUpdateTransitionOverlappingFieldsError {
    pub fn new(data_contract_id: Identifier, field_type: String, overlapping_key: String) -> Self {
        Self {
            data_contract_id,
            field_type,
            overlapping_key,
        }
    }

    pub fn data_contract_id(&self) -> &Identifier {
        &self.data_contract_id
    }

    pub fn field_type(&self) -> &str {
        &self.field_type
    }

    pub fn overlapping_key(&self) -> &str {
        &self.overlapping_key
    }
}

impl From<DataContractUpdateTransitionOverlappingFieldsError> for ConsensusError {
    fn from(err: DataContractUpdateTransitionOverlappingFieldsError) -> Self {
        Self::BasicError(BasicError::DataContractUpdateTransitionOverlappingFieldsError(err))
    }
}
