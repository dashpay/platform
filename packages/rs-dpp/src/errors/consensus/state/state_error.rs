use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

use crate::consensus::state::data_contract::data_contract_already_present_error::DataContractAlreadyPresentError;
use crate::consensus::state::data_contract::data_contract_config_update_error::DataContractConfigUpdateError;
use crate::consensus::state::data_contract::data_contract_is_readonly_error::DataContractIsReadonlyError;
#[cfg(feature = "state-transition-validation")]
use crate::consensus::state::data_trigger::DataTriggerError;
use crate::consensus::state::document::document_already_present_error::DocumentAlreadyPresentError;
use crate::consensus::state::document::document_not_found_error::DocumentNotFoundError;
use crate::consensus::state::document::document_owner_id_mismatch_error::DocumentOwnerIdMismatchError;
use crate::consensus::state::document::document_timestamp_window_violation_error::DocumentTimestampWindowViolationError;
use crate::consensus::state::document::document_timestamps_mismatch_error::DocumentTimestampsMismatchError;
use crate::consensus::state::document::duplicate_unique_index_error::DuplicateUniqueIndexError;
use crate::consensus::state::document::invalid_document_revision_error::InvalidDocumentRevisionError;
use crate::consensus::state::identity::duplicated_identity_public_key_id_state_error::DuplicatedIdentityPublicKeyIdStateError;
use crate::consensus::state::identity::duplicated_identity_public_key_state_error::DuplicatedIdentityPublicKeyStateError;
use crate::consensus::state::identity::identity_public_key_is_disabled_error::IdentityPublicKeyIsDisabledError;
use crate::consensus::state::identity::identity_public_key_is_read_only_error::IdentityPublicKeyIsReadOnlyError;
use crate::consensus::state::identity::invalid_identity_public_key_id_error::InvalidIdentityPublicKeyIdError;
use crate::consensus::state::identity::invalid_identity_revision_error::InvalidIdentityRevisionError;
use crate::consensus::state::identity::max_identity_public_key_limit_reached_error::MaxIdentityPublicKeyLimitReachedError;
use crate::consensus::state::identity::missing_identity_public_key_ids_error::MissingIdentityPublicKeyIdsError;
use crate::consensus::state::identity::{
    IdentityAlreadyExistsError, IdentityInsufficientBalanceError,
};
use crate::consensus::ConsensusError;
use crate::consensus::state::data_contract::data_contract_update_permission_error::DataContractUpdatePermissionError;
use crate::consensus::state::data_contract::document_type_update_error::DocumentTypeUpdateError;
use crate::consensus::state::document::document_contest_currently_locked_error::DocumentContestCurrentlyLockedError;
use crate::consensus::state::document::document_contest_identity_already_contestant::DocumentContestIdentityAlreadyContestantError;
use crate::consensus::state::document::document_contest_not_joinable_error::DocumentContestNotJoinableError;
use crate::consensus::state::document::document_incorrect_purchase_price_error::DocumentIncorrectPurchasePriceError;
use crate::consensus::state::document::document_not_for_sale_error::DocumentNotForSaleError;
use crate::consensus::state::identity::identity_public_key_already_exists_for_unique_contract_bounds_error::IdentityPublicKeyAlreadyExistsForUniqueContractBoundsError;
use crate::consensus::state::identity::invalid_identity_contract_nonce_error::InvalidIdentityNonceError;
use crate::consensus::state::prefunded_specialized_balances::prefunded_specialized_balance_insufficient_error::PrefundedSpecializedBalanceInsufficientError;
use crate::consensus::state::prefunded_specialized_balances::prefunded_specialized_balance_not_found_error::PrefundedSpecializedBalanceNotFoundError;
use crate::consensus::state::voting::masternode_incorrect_voter_identity_id_error::MasternodeIncorrectVoterIdentityIdError;
use crate::consensus::state::voting::masternode_incorrect_voting_address_error::MasternodeIncorrectVotingAddressError;
use crate::consensus::state::voting::masternode_not_found_error::MasternodeNotFoundError;
use crate::consensus::state::voting::masternode_vote_already_present_error::MasternodeVoteAlreadyPresentError;
use crate::consensus::state::voting::masternode_voted_too_many_times::MasternodeVotedTooManyTimesError;
use crate::consensus::state::voting::vote_poll_not_available_for_voting_error::VotePollNotAvailableForVotingError;
use crate::consensus::state::voting::vote_poll_not_found_error::VotePollNotFoundError;

use super::document::document_timestamps_are_equal_error::DocumentTimestampsAreEqualError;

#[derive(
    Error, Debug, PartialEq, Encode, Decode, PlatformSerialize, PlatformDeserialize, Clone,
)]
pub enum StateError {
    /*

    DO NOT CHANGE ORDER OF VARIANTS WITHOUT INTRODUCING OF NEW VERSION

    */
    #[error(transparent)]
    DataContractAlreadyPresentError(DataContractAlreadyPresentError),

    // TODO: Not sure we can do it.
    //   The order of variants must be always the same otherwise serialization won't work
    #[cfg(feature = "state-transition-validation")]
    #[error(transparent)]
    DataTriggerError(DataTriggerError),

    #[error(transparent)]
    DocumentAlreadyPresentError(DocumentAlreadyPresentError),

    #[error(transparent)]
    DocumentContestCurrentlyLockedError(DocumentContestCurrentlyLockedError),

    #[error(transparent)]
    DocumentContestNotJoinableError(DocumentContestNotJoinableError),

    #[error(transparent)]
    DocumentContestIdentityAlreadyContestantError(DocumentContestIdentityAlreadyContestantError),

    #[error(transparent)]
    DocumentNotFoundError(DocumentNotFoundError),

    #[error(transparent)]
    DocumentNotForSaleError(DocumentNotForSaleError),

    #[error(transparent)]
    DocumentIncorrectPurchasePriceError(DocumentIncorrectPurchasePriceError),

    #[error(transparent)]
    DocumentOwnerIdMismatchError(DocumentOwnerIdMismatchError),

    #[error(transparent)]
    DocumentTimestampsMismatchError(DocumentTimestampsMismatchError),

    #[error(transparent)]
    DocumentTimestampWindowViolationError(DocumentTimestampWindowViolationError),

    #[error(transparent)]
    DuplicateUniqueIndexError(DuplicateUniqueIndexError),

    #[error(transparent)]
    InvalidDocumentRevisionError(InvalidDocumentRevisionError),

    #[error(transparent)]
    IdentityAlreadyExistsError(IdentityAlreadyExistsError),

    #[error(transparent)]
    IdentityPublicKeyAlreadyExistsForUniqueContractBoundsError(
        IdentityPublicKeyAlreadyExistsForUniqueContractBoundsError,
    ),

    #[error(transparent)]
    IdentityPublicKeyIsReadOnlyError(IdentityPublicKeyIsReadOnlyError),

    #[error(transparent)]
    MissingIdentityPublicKeyIdsError(MissingIdentityPublicKeyIdsError),

    #[error(transparent)]
    InvalidIdentityPublicKeyIdError(InvalidIdentityPublicKeyIdError),

    #[error(transparent)]
    InvalidIdentityRevisionError(InvalidIdentityRevisionError),

    #[error(transparent)]
    InvalidIdentityNonceError(InvalidIdentityNonceError),

    #[error(transparent)]
    MaxIdentityPublicKeyLimitReachedError(MaxIdentityPublicKeyLimitReachedError),

    #[error(transparent)]
    DuplicatedIdentityPublicKeyStateError(DuplicatedIdentityPublicKeyStateError),

    #[error(transparent)]
    DuplicatedIdentityPublicKeyIdStateError(DuplicatedIdentityPublicKeyIdStateError),

    #[error(transparent)]
    IdentityPublicKeyIsDisabledError(IdentityPublicKeyIsDisabledError),

    #[error(transparent)]
    IdentityInsufficientBalanceError(IdentityInsufficientBalanceError),

    #[error(transparent)]
    DocumentTimestampsAreEqualError(DocumentTimestampsAreEqualError),

    #[error(transparent)]
    DataContractIsReadonlyError(DataContractIsReadonlyError),

    #[error(transparent)]
    DataContractConfigUpdateError(DataContractConfigUpdateError),

    #[error(transparent)]
    DocumentTypeUpdateError(DocumentTypeUpdateError),

    #[error(transparent)]
    PrefundedSpecializedBalanceInsufficientError(PrefundedSpecializedBalanceInsufficientError),

    #[error(transparent)]
    PrefundedSpecializedBalanceNotFoundError(PrefundedSpecializedBalanceNotFoundError),

    #[error(transparent)]
    DataContractUpdatePermissionError(DataContractUpdatePermissionError),

    #[error(transparent)]
    MasternodeNotFoundError(MasternodeNotFoundError),

    #[error(transparent)]
    MasternodeIncorrectVoterIdentityIdError(MasternodeIncorrectVoterIdentityIdError),

    #[error(transparent)]
    MasternodeIncorrectVotingAddressError(MasternodeIncorrectVotingAddressError),

    #[error(transparent)]
    VotePollNotFoundError(VotePollNotFoundError),

    #[error(transparent)]
    VotePollNotAvailableForVotingError(VotePollNotAvailableForVotingError),

    #[error(transparent)]
    MasternodeVotedTooManyTimesError(MasternodeVotedTooManyTimesError),

    #[error(transparent)]
    MasternodeVoteAlreadyPresentError(MasternodeVoteAlreadyPresentError),
}

impl From<StateError> for ConsensusError {
    fn from(error: StateError) -> Self {
        Self::StateError(error)
    }
}
