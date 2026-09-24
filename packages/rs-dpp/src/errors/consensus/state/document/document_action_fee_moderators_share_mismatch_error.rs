use crate::balances::credits::Credits;
use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use thiserror::Error;

/// The transition's action fee agreement names less than the moderators part a document type of
/// an elected contract declares, and that is not the discount the contract's seated moderation
/// charter gives: its `moderatorsShare` of the declared part, rounded down. With no seated
/// charter there is no discount (`moderators_share` is `None`). The signer agrees to the declared
/// part, or to the share the seated charter takes.
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
    "Document {} of type {} declares a moderators fee of {} credits; the transition agreed to {}, which is not {}",
    action,
    document_type_name,
    declared_moderators,
    agreed_moderators,
    moderators_share.map(|share| format!("the seated moderation charter's {share}% share of it")).unwrap_or_else(|| "discounted: the contract has no seated moderation charter".to_string())
)]
#[platform_serialize(unversioned)]
pub struct DocumentActionFeeModeratorsShareMismatchError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    document_type_name: String,
    action: String,
    declared_moderators: Credits,
    agreed_moderators: Credits,
    moderators_share: Option<u8>,
}

impl DocumentActionFeeModeratorsShareMismatchError {
    /// `moderators_share` is the percentage the contract's seated charter takes, `None` when no
    /// charter is seated.
    pub fn new(
        document_type_name: String,
        action: String,
        declared_moderators: Credits,
        agreed_moderators: Credits,
        moderators_share: Option<u8>,
    ) -> Self {
        Self {
            document_type_name,
            action,
            declared_moderators,
            agreed_moderators,
            moderators_share,
        }
    }

    pub fn document_type_name(&self) -> &str {
        &self.document_type_name
    }

    pub fn action(&self) -> &str {
        &self.action
    }

    pub fn declared_moderators(&self) -> Credits {
        self.declared_moderators
    }

    pub fn agreed_moderators(&self) -> Credits {
        self.agreed_moderators
    }

    pub fn moderators_share(&self) -> Option<u8> {
        self.moderators_share
    }
}

impl From<DocumentActionFeeModeratorsShareMismatchError> for ConsensusError {
    fn from(err: DocumentActionFeeModeratorsShareMismatchError) -> Self {
        Self::StateError(StateError::DocumentActionFeeModeratorsShareMismatchError(
            err,
        ))
    }
}
