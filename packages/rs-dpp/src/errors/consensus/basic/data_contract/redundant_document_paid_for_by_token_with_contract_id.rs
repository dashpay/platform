use crate::errors::consensus::basic::BasicError;
use crate::errors::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_value::Identifier;
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Document paid for by a token has a contractId {contract_id} set, which is redundant because it is targeting the current contract")]
#[platform_serialize(unversioned)]
#[cfg_attr(feature = "apple", ferment_macro::export)]
pub struct RedundantDocumentPaidForByTokenWithContractId {
    pub contract_id: Identifier,
}

impl RedundantDocumentPaidForByTokenWithContractId {
    pub fn new(contract_id: Identifier) -> Self {
        Self { contract_id }
    }

    pub fn contract_id(&self) -> Identifier {
        self.contract_id
    }
}

impl From<RedundantDocumentPaidForByTokenWithContractId> for ConsensusError {
    fn from(err: RedundantDocumentPaidForByTokenWithContractId) -> Self {
        Self::BasicError(BasicError::RedundantDocumentPaidForByTokenWithContractId(
            err,
        ))
    }
}
