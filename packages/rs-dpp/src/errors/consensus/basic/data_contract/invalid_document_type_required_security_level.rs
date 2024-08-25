use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

use crate::identity::SecurityLevel;

use bincode::{Decode, Encode};
use platform_value::Identifier;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Invalid document type security level error security level: got {security_level:?} for {contract_id}::{document_type_name}")]
#[platform_serialize(unversioned)]
pub struct InvalidDocumentTypeRequiredSecurityLevelError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    security_level: SecurityLevel,
    contract_id: Identifier,
    document_type_name: String,
}

impl InvalidDocumentTypeRequiredSecurityLevelError {
    pub fn new(
        security_level: SecurityLevel,
        contract_id: Identifier,
        document_type_name: String,
    ) -> Self {
        Self {
            security_level,
            contract_id,
            document_type_name,
        }
    }

    pub fn security_level(&self) -> SecurityLevel {
        self.security_level
    }

    pub fn contract_id(&self) -> Identifier {
        self.contract_id
    }

    pub fn document_type_name(&self) -> &String {
        &self.document_type_name
    }
}

impl From<InvalidDocumentTypeRequiredSecurityLevelError> for ConsensusError {
    fn from(err: InvalidDocumentTypeRequiredSecurityLevelError) -> Self {
        Self::BasicError(BasicError::InvalidDocumentTypeRequiredSecurityLevelError(
            err,
        ))
    }
}
