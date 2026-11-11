use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use crate::identity::TimestampMillis;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use platform_value::Identifier;
use thiserror::Error;

/// The proposal or approval of a settled document's deletion by a member the leader added to
/// the seated team after the document was created, on a type whose `deleteSettled` rule admits
/// only members from before the document (`approversPredateDocument`, on by default). The
/// leader names whom it adds, so such members could otherwise be added to approve whatever the
/// leader proposes. The leader and the elected members always count.
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
    "Member {} of the moderation team of contract {} was added at {}, not before document {} was created at {}, so it can not approve the document's deletion",
    member_id,
    contract_id,
    added_at,
    document_id,
    document_created_at
)]
#[platform_serialize(unversioned)]
pub struct ContractTeamMemberAddedAfterDocumentError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    contract_id: Identifier,
    member_id: Identifier,
    added_at: TimestampMillis,
    document_id: Identifier,
    document_created_at: TimestampMillis,
}

impl ContractTeamMemberAddedAfterDocumentError {
    pub fn new(
        contract_id: Identifier,
        member_id: Identifier,
        added_at: TimestampMillis,
        document_id: Identifier,
        document_created_at: TimestampMillis,
    ) -> Self {
        Self {
            contract_id,
            member_id,
            added_at,
            document_id,
            document_created_at,
        }
    }

    /// The moderated contract
    pub fn contract_id(&self) -> Identifier {
        self.contract_id
    }

    /// The member the leader added, who signed
    pub fn member_id(&self) -> Identifier {
        self.member_id
    }

    /// When the member was added: its `addedModerator`'s `$createdAt`
    pub fn added_at(&self) -> TimestampMillis {
        self.added_at
    }

    /// The document whose deletion it proposed or approved
    pub fn document_id(&self) -> Identifier {
        self.document_id
    }

    /// When the document was created: its `$createdAt`
    pub fn document_created_at(&self) -> TimestampMillis {
        self.document_created_at
    }
}

impl From<ContractTeamMemberAddedAfterDocumentError> for ConsensusError {
    fn from(err: ContractTeamMemberAddedAfterDocumentError) -> Self {
        Self::StateError(StateError::ContractTeamMemberAddedAfterDocumentError(err))
    }
}
