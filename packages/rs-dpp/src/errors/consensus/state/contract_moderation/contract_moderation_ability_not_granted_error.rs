use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::data_contract::config::moderation::ModerationAbility;
use crate::errors::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use platform_value::Identifier;
use thiserror::Error;

/// A moderation action by the seated team of an elected contract that the contract's
/// declaration does not give the team: a ban, a suspension or a warning (or lifting one) when
/// no moderated document type carries the ability, or a document deletion or restore on a type
/// that does not carry `deleteDocuments`. The team acts with the declaration's abilities and no
/// others.
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
    "The elected moderation declaration of contract {} does not give its seated team the {} ability{}",
    contract_id,
    ability,
    document_type_name.as_ref().map(|name| format!(" on document type {name}")).unwrap_or_default()
)]
#[platform_serialize(unversioned)]
pub struct ContractModerationAbilityNotGrantedError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    contract_id: Identifier,
    ability: ModerationAbility,
    document_type_name: Option<String>,
}

impl ContractModerationAbilityNotGrantedError {
    /// `document_type_name` is the type a deletion or a restore names, `None` for an action on
    /// a list, which is contract-wide.
    pub fn new(
        contract_id: Identifier,
        ability: ModerationAbility,
        document_type_name: Option<String>,
    ) -> Self {
        Self {
            contract_id,
            ability,
            document_type_name,
        }
    }

    pub fn contract_id(&self) -> Identifier {
        self.contract_id
    }

    pub fn ability(&self) -> ModerationAbility {
        self.ability
    }

    pub fn document_type_name(&self) -> Option<&str> {
        self.document_type_name.as_deref()
    }
}

impl From<ContractModerationAbilityNotGrantedError> for ConsensusError {
    fn from(err: ContractModerationAbilityNotGrantedError) -> Self {
        Self::StateError(StateError::ContractModerationAbilityNotGrantedError(err))
    }
}
