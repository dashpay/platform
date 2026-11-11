use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use platform_value::Identifier;
use thiserror::Error;

/// A document create or replace sets, changes or removes a field its type lists under
/// `moderatorAbilities.changeFields`, and its signer does not moderate the contract: only
/// the moderators write those fields.
#[derive(
    Error,
    Debug,
    Clone,
    PartialEq,
    Eq,
    Encode,
    Decode,
    PlatformSerialize,
    PlatformDeserializeTrusted,
    PlatformDeserializeUntrusted,
    DecodeUntrusted,
)]
#[error(
    "Only the moderators of contract {} write field {} of documents of type {}, and {} does not moderate it (document {})",
    contract_id,
    field,
    document_type_name,
    identity_id,
    document_id
)]
#[platform_serialize(unversioned)]
pub struct DocumentModeratorFieldNotWritableError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    contract_id: Identifier,
    document_type_name: String,
    document_id: Identifier,
    field: String,
    identity_id: Identifier,
}

impl DocumentModeratorFieldNotWritableError {
    pub fn new(
        contract_id: Identifier,
        document_type_name: String,
        document_id: Identifier,
        field: String,
        identity_id: Identifier,
    ) -> Self {
        Self {
            contract_id,
            document_type_name,
            document_id,
            field,
            identity_id,
        }
    }

    pub fn contract_id(&self) -> Identifier {
        self.contract_id
    }

    pub fn document_type_name(&self) -> &str {
        &self.document_type_name
    }

    pub fn document_id(&self) -> Identifier {
        self.document_id
    }

    pub fn field(&self) -> &str {
        &self.field
    }

    pub fn identity_id(&self) -> Identifier {
        self.identity_id
    }
}

impl From<DocumentModeratorFieldNotWritableError> for ConsensusError {
    fn from(err: DocumentModeratorFieldNotWritableError) -> Self {
        Self::StateError(StateError::DocumentModeratorFieldNotWritableError(err))
    }
}
