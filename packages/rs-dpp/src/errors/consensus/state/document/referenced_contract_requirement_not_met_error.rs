use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
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
    "referenced contract {contract_id} for path {path} does not declare {field} {required}, which the reference requires"
)]
#[platform_serialize(unversioned)]
pub struct ReferencedContractRequirementNotMetError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    contract_id: Identifier,
    field: String,
    required: String,
    path: String,
}

impl ReferencedContractRequirementNotMetError {
    pub fn new(contract_id: Identifier, field: String, required: String, path: String) -> Self {
        Self {
            contract_id,
            field,
            required,
            path,
        }
    }

    /// The referenced contract, which exists but does not meet the requirement
    pub fn contract_id(&self) -> &Identifier {
        &self.contract_id
    }

    /// The `contractFields` key of the requirement, `moderation` for one
    pub fn field(&self) -> &str {
        &self.field
    }

    /// The value the reference requires, `elected` for one
    pub fn required(&self) -> &str {
        &self.required
    }

    /// The referring property
    pub fn path(&self) -> &str {
        &self.path
    }
}

impl From<ReferencedContractRequirementNotMetError> for ConsensusError {
    fn from(err: ReferencedContractRequirementNotMetError) -> Self {
        Self::StateError(StateError::ReferencedContractRequirementNotMetError(err))
    }
}
