use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::identity::KeyID;
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use platform_value::Identifier;
use thiserror::Error;

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
    "referenced public key {key_id} of identity {identity_id} for {document_type_name}.{path} has {field} {actual}, the reference requires {required}"
)]
#[platform_serialize(unversioned)]
pub struct ReferencedIdentityKeyRequirementNotMetError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    document_type_name: String,
    path: String,
    identity_id: Identifier,
    key_id: KeyID,
    field: String,
    required: String,
    actual: String,
}

impl ReferencedIdentityKeyRequirementNotMetError {
    pub fn new(
        document_type_name: String,
        path: String,
        identity_id: Identifier,
        key_id: KeyID,
        field: String,
        required: String,
        actual: String,
    ) -> Self {
        Self {
            document_type_name,
            path,
            identity_id,
            key_id,
            field,
            required,
            actual,
        }
    }

    /// The document type declaring the reference
    pub fn document_type_name(&self) -> &str {
        &self.document_type_name
    }

    /// The referring property
    pub fn path(&self) -> &str {
        &self.path
    }

    /// The identity whose key was referenced
    pub fn identity_id(&self) -> &Identifier {
        &self.identity_id
    }

    /// The referenced key, which exists and is enabled but does not meet the requirement
    pub fn key_id(&self) -> KeyID {
        self.key_id
    }

    /// The `keyRequirements` key of the requirement, `purpose` or `boundTo`
    pub fn field(&self) -> &str {
        &self.field
    }

    /// The value the reference requires, `decryption` for one
    pub fn required(&self) -> &str {
        &self.required
    }

    /// What the key has instead, `encryption` for one
    pub fn actual(&self) -> &str {
        &self.actual
    }
}

impl From<ReferencedIdentityKeyRequirementNotMetError> for ConsensusError {
    fn from(err: ReferencedIdentityKeyRequirementNotMetError) -> Self {
        Self::StateError(StateError::ReferencedIdentityKeyRequirementNotMetError(err))
    }
}
