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
#[error("Data contract update transition {data_contract_id} has conflicting keyword '{keyword}': cannot add and remove the same keyword.")]
#[platform_serialize(unversioned)]
pub struct DataContractUpdateTransitionConflictingKeywordError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    data_contract_id: Identifier,
    keyword: String,
}

impl DataContractUpdateTransitionConflictingKeywordError {
    pub fn new(data_contract_id: Identifier, keyword: String) -> Self {
        Self {
            data_contract_id,
            keyword,
        }
    }

    pub fn data_contract_id(&self) -> &Identifier {
        &self.data_contract_id
    }

    pub fn keyword(&self) -> &str {
        &self.keyword
    }
}

impl From<DataContractUpdateTransitionConflictingKeywordError> for ConsensusError {
    fn from(err: DataContractUpdateTransitionConflictingKeywordError) -> Self {
        Self::BasicError(BasicError::DataContractUpdateTransitionConflictingKeywordError(err))
    }
}
