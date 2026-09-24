use crate::errors::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use thiserror::Error;

use crate::consensus::state::address_funds::{AddressDoesNotExistError, AddressInvalidNonceError, AddressNotEnoughFundsError, AddressesNotEnoughFundsError};
use crate::consensus::state::shielded::insufficient_pool_notes_error::InsufficientPoolNotesError;
use crate::consensus::state::shielded::insufficient_shielded_fee_error::InsufficientShieldedFeeError;
use crate::consensus::state::shielded::invalid_anchor_error::InvalidAnchorError;
use crate::consensus::state::shielded::invalid_shielded_proof_error::InvalidShieldedProofError;
use crate::consensus::state::shielded::nullifier_already_spent_error::NullifierAlreadySpentError;
use crate::consensus::state::contract_moderation::{
    ContractModeratedDocumentTypeNotYetUsableError, ContractModerationAbilityNotGrantedError,
    ModerationCharterAddedModeratorLimitReachedError, ModerationReasonNotListedError,
    ContractModerationNotEnabledError, ContractModerationTargetNotAllowedError,
    ContractFeeClaimNotAllowedError, ContractFeesAlreadyClaimedThisEpochError,
    ContractFeesNothingToClaimError, ContractModerationCounterpartyBarredError,
    ContractModerationTargetNotFoundError,
    ContractModeratorIdentityNotFoundError,
    ContractSuspensionNotInFutureError, ContractUserAlreadyBannedError, ContractUserBannedError,
    ContractUserNotBannedError, ContractUserNotSuspendedError, ContractUserNotWarnedError,
    ContractUserSuspendedError, ContractUserWarningLimitReachedError,
    ContractDocumentAlreadyRestoredError, ContractDocumentRemovalNotFoundError,
    DocumentModerationWindowElapsedError, DocumentRestoreHashMismatchError,
    DocumentRestoreWindowElapsedError, DocumentTypeNotDeletableByModeratorsError,
    IdentityNotContractModeratorError,
};
use crate::consensus::state::contract_group::{
    ContractGroupAlreadyExistsError, ContractGroupNotFoundError,
    ContractGroupAdminNotFoundError, IdentityNotContractGroupOwnerOrAdminError,
};
use crate::consensus::state::data_contract::data_contract_already_present_error::DataContractAlreadyPresentError;
use crate::consensus::state::data_contract::data_contract_config_update_error::DataContractConfigUpdateError;
use crate::consensus::state::data_contract::data_contract_is_readonly_error::DataContractIsReadonlyError;
use crate::consensus::state::data_trigger::DataTriggerError;
use crate::consensus::state::document::document_action_fee_agreement_mismatch_error::DocumentActionFeeAgreementMismatchError;
use crate::consensus::state::document::document_action_fee_moderators_share_mismatch_error::DocumentActionFeeModeratorsShareMismatchError;
use crate::consensus::state::document::document_action_fee_agreement_not_set_error::DocumentActionFeeAgreementNotSetError;
use crate::consensus::state::document::document_action_fee_multiplier_not_tolerated_error::DocumentActionFeeMultiplierNotToleratedError;
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
use crate::consensus::state::identity::{IdentityAlreadyExistsError, IdentityInsufficientBalanceError, RecipientIdentityDoesNotExistError};
use crate::consensus::ConsensusError;
use crate::consensus::state::data_contract::data_contract_not_found_error::DataContractNotFoundError;
use crate::consensus::state::data_contract::data_contract_update_action_not_allowed_error::DataContractUpdateActionNotAllowedError;
use crate::consensus::state::data_contract::data_contract_update_permission_error::DataContractUpdatePermissionError;
use crate::consensus::state::data_contract::document_type_update_error::DocumentTypeUpdateError;
use crate::consensus::state::document::document_contest_currently_locked_error::DocumentContestCurrentlyLockedError;
use crate::consensus::state::document::document_contest_document_with_same_id_already_present_error::DocumentContestDocumentWithSameIdAlreadyPresentError;
use crate::consensus::state::document::document_contest_identity_already_contestant::DocumentContestIdentityAlreadyContestantError;
use crate::consensus::state::document::document_contest_index_mismatch_error::DocumentContestIndexMismatchError;
use crate::consensus::state::document::document_contest_not_joinable_error::DocumentContestNotJoinableError;
use crate::consensus::state::document::document_contest_not_paid_for_error::DocumentContestNotPaidForError;
use crate::consensus::state::document::document_contest_not_required_error::DocumentContestNotRequiredError;
use crate::consensus::state::document::referenced_document_type_deletable_error::ReferencedDocumentTypeDeletableError;
use crate::consensus::state::document::referenced_document_type_not_deletable_error::ReferencedDocumentTypeNotDeletableError;
use crate::consensus::state::document::referenced_document_type_not_found_error::ReferencedDocumentTypeNotFoundError;
use crate::consensus::state::document::referenced_contract_requirement_not_met_error::ReferencedContractRequirementNotMetError;
use crate::consensus::state::document::referenced_document_lookup_invalid_error::ReferencedDocumentLookupInvalidError;
use crate::consensus::state::document::referenced_document_list_invalid_error::ReferencedDocumentListInvalidError;
use crate::consensus::state::document::referenced_entity_not_found_error::ReferencedEntityNotFoundError;
use crate::consensus::state::document::referenced_identity_key_disabled_error::ReferencedIdentityKeyDisabledError;
use crate::consensus::state::document::referenced_identity_key_not_found_error::ReferencedIdentityKeyNotFoundError;
use crate::consensus::state::document::referenced_identity_key_requirement_not_met_error::ReferencedIdentityKeyRequirementNotMetError;
use crate::consensus::state::document::referenced_document_property_agreement_invalid_error::ReferencedDocumentPropertyAgreementInvalidError;
use crate::consensus::state::document::referenced_document_property_mismatch_error::ReferencedDocumentPropertyMismatchError;
use crate::consensus::state::document::referenced_key_id_property_invalid_error::ReferencedKeyIdPropertyInvalidError;
use crate::consensus::state::document::document_incorrect_purchase_price_error::DocumentIncorrectPurchasePriceError;
use crate::consensus::state::document::document_not_for_sale_error::DocumentNotForSaleError;
use crate::consensus::state::group::{GroupActionAlreadyCompletedError, GroupActionAlreadySignedByIdentityError, GroupActionDoesNotExistError, IdentityMemberOfGroupNotFoundError, IdentityNotMemberOfGroupError, ModificationOfGroupActionMainParametersNotPermittedError};
use crate::consensus::state::identity::identity_for_token_configuration_not_found_error::IdentityInTokenConfigurationNotFoundError;
use crate::consensus::state::identity::identity_public_key_already_exists_for_unique_contract_bounds_error::IdentityPublicKeyAlreadyExistsForUniqueContractBoundsError;
use crate::consensus::state::identity::identity_public_key_already_expired_error::IdentityPublicKeyAlreadyExpiredError;
use crate::consensus::state::identity::identity_public_key_budget_exceeded_error::IdentityPublicKeyBudgetExceededError;
use crate::consensus::state::identity::identity_public_key_limit_not_raised_error::IdentityPublicKeyLimitNotRaisedError;
use crate::consensus::state::document::document_immutable_property_changed_error::DocumentImmutablePropertyChangedError;
use crate::consensus::state::identity::identity_public_key_limit_not_set_error::IdentityPublicKeyLimitNotSetError;
use crate::consensus::state::identity::gas_sponsor_insufficient_balance_error::GasSponsorInsufficientBalanceError;
use crate::consensus::state::token::{GasFeesPaidByNotAllowedError, InconsistentGasFeesPaidByInBatchError};
use crate::consensus::state::identity::identity_to_freeze_does_not_exist_error::IdentityToFreezeDoesNotExistError;
use crate::consensus::state::identity::invalid_identity_contract_nonce_error::InvalidIdentityNonceError;
use crate::consensus::state::identity::missing_transfer_key_error::MissingTransferKeyError;
use crate::consensus::state::identity::no_transfer_key_for_core_withdrawal_available_error::NoTransferKeyForCoreWithdrawalAvailableError;
use crate::consensus::state::prefunded_specialized_balances::prefunded_specialized_balance_insufficient_error::PrefundedSpecializedBalanceInsufficientError;
use crate::consensus::state::prefunded_specialized_balances::prefunded_specialized_balance_not_found_error::PrefundedSpecializedBalanceNotFoundError;
use crate::consensus::state::token::{IdentityDoesNotHaveEnoughTokenBalanceError, IdentityTokenAccountFrozenError, IdentityTokenAccountNotFrozenError, InvalidGroupPositionError, NewAuthorizedActionTakerGroupDoesNotExistError, NewAuthorizedActionTakerIdentityDoesNotExistError, NewAuthorizedActionTakerMainGroupNotSetError, NewTokensDestinationIdentityDoesNotExistError, TokenMintPastMaxSupplyError, TokenSettingMaxSupplyToLessThanCurrentSupplyError, UnauthorizedTokenActionError, IdentityTokenAccountAlreadyFrozenError, TokenAlreadyPausedError, TokenIsPausedError, TokenNotPausedError, InvalidTokenClaimPropertyMismatch, InvalidTokenClaimNoCurrentRewards, InvalidTokenClaimWrongClaimant, PreProgrammedDistributionTimestampInPastError, TokenTransferRecipientIdentityNotExistError, IdentityHasNotAgreedToPayRequiredTokenAmountError, RequiredTokenPaymentInfoNotSetError, IdentityTryingToPayWithWrongTokenError, TokenDirectPurchaseUserPriceTooLow, TokenAmountUnderMinimumSaleAmount, TokenNotForDirectSale, InvalidTokenPositionStateError, TokenOncePerIdentityDistributionAlreadyClaimedError};
use crate::consensus::state::voting::masternode_incorrect_voter_identity_id_error::MasternodeIncorrectVoterIdentityIdError;
use crate::consensus::state::voting::masternode_incorrect_voting_address_error::MasternodeIncorrectVotingAddressError;
use crate::consensus::state::voting::masternode_not_found_error::MasternodeNotFoundError;
use crate::consensus::state::voting::masternode_vote_already_present_error::MasternodeVoteAlreadyPresentError;
use crate::consensus::state::voting::masternode_voted_too_many_times::MasternodeVotedTooManyTimesError;
use crate::consensus::state::voting::vote_poll_not_available_for_voting_error::VotePollNotAvailableForVotingError;
use crate::consensus::state::voting::vote_choice_not_allowed_for_vote_poll_error::VoteChoiceNotAllowedForVotePollError;
use crate::consensus::state::voting::vote_poll_not_found_error::VotePollNotFoundError;

use super::document::document_timestamps_are_equal_error::DocumentTimestampsAreEqualError;

#[derive(
    Error,
    Debug,
    PartialEq,
    Encode,
    Decode,
    PlatformSerialize,
    PlatformDeserializeTrusted,
    PlatformDeserializeUntrusted,
    Clone,
    DecodeUntrusted,
)]
pub enum StateError {
    /*

    DO NOT CHANGE ORDER OF VARIANTS WITHOUT INTRODUCING OF NEW VERSION

    */
    #[error(transparent)]
    DataContractAlreadyPresentError(DataContractAlreadyPresentError),

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
    DocumentContestNotPaidForError(DocumentContestNotPaidForError),

    #[error(transparent)]
    DocumentContestDocumentWithSameIdAlreadyPresentError(
        DocumentContestDocumentWithSameIdAlreadyPresentError,
    ),

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
    MissingTransferKeyError(MissingTransferKeyError),

    #[error(transparent)]
    NoTransferKeyForCoreWithdrawalAvailableError(NoTransferKeyForCoreWithdrawalAvailableError),

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

    #[error(transparent)]
    RecipientIdentityDoesNotExistError(RecipientIdentityDoesNotExistError),

    #[error(transparent)]
    IdentityDoesNotHaveEnoughTokenBalanceError(IdentityDoesNotHaveEnoughTokenBalanceError),

    #[error(transparent)]
    UnauthorizedTokenActionError(UnauthorizedTokenActionError),

    #[error(transparent)]
    IdentityTokenAccountFrozenError(IdentityTokenAccountFrozenError),

    #[error(transparent)]
    IdentityTokenAccountNotFrozenError(IdentityTokenAccountNotFrozenError),

    #[error(transparent)]
    IdentityNotMemberOfGroupError(IdentityNotMemberOfGroupError),

    #[error(transparent)]
    GroupActionDoesNotExistError(GroupActionDoesNotExistError),

    #[error(transparent)]
    GroupActionAlreadyCompletedError(GroupActionAlreadyCompletedError),

    #[error(transparent)]
    GroupActionAlreadySignedByIdentityError(GroupActionAlreadySignedByIdentityError),

    #[error(transparent)]
    DataContractUpdateActionNotAllowedError(DataContractUpdateActionNotAllowedError),

    #[error(transparent)]
    TokenSettingMaxSupplyToLessThanCurrentSupplyError(
        TokenSettingMaxSupplyToLessThanCurrentSupplyError,
    ),

    #[error(transparent)]
    TokenMintPastMaxSupplyError(TokenMintPastMaxSupplyError),

    #[error(transparent)]
    InvalidTokenClaimPropertyMismatch(InvalidTokenClaimPropertyMismatch),

    #[error(transparent)]
    InvalidTokenClaimNoCurrentRewards(InvalidTokenClaimNoCurrentRewards),

    #[error(transparent)]
    InvalidTokenClaimWrongClaimant(InvalidTokenClaimWrongClaimant),

    #[error(transparent)]
    NewTokensDestinationIdentityDoesNotExistError(NewTokensDestinationIdentityDoesNotExistError),

    #[error(transparent)]
    NewAuthorizedActionTakerIdentityDoesNotExistError(
        NewAuthorizedActionTakerIdentityDoesNotExistError,
    ),

    #[error(transparent)]
    NewAuthorizedActionTakerGroupDoesNotExistError(NewAuthorizedActionTakerGroupDoesNotExistError),

    #[error(transparent)]
    NewAuthorizedActionTakerMainGroupNotSetError(NewAuthorizedActionTakerMainGroupNotSetError),

    #[error(transparent)]
    InvalidGroupPositionError(InvalidGroupPositionError),

    #[error(transparent)]
    TokenIsPausedError(TokenIsPausedError),

    #[error(transparent)]
    IdentityTokenAccountAlreadyFrozenError(IdentityTokenAccountAlreadyFrozenError),

    #[error(transparent)]
    TokenAlreadyPausedError(TokenAlreadyPausedError),

    #[error(transparent)]
    TokenNotPausedError(TokenNotPausedError),

    #[error(transparent)]
    TokenTransferRecipientIdentityNotExistError(TokenTransferRecipientIdentityNotExistError),

    #[error(transparent)]
    PreProgrammedDistributionTimestampInPastError(PreProgrammedDistributionTimestampInPastError),

    #[error(transparent)]
    IdentityHasNotAgreedToPayRequiredTokenAmountError(
        IdentityHasNotAgreedToPayRequiredTokenAmountError,
    ),

    #[error(transparent)]
    RequiredTokenPaymentInfoNotSetError(RequiredTokenPaymentInfoNotSetError),

    #[error(transparent)]
    IdentityTryingToPayWithWrongTokenError(IdentityTryingToPayWithWrongTokenError),

    #[error(transparent)]
    TokenDirectPurchaseUserPriceTooLow(TokenDirectPurchaseUserPriceTooLow),

    #[error(transparent)]
    TokenAmountUnderMinimumSaleAmount(TokenAmountUnderMinimumSaleAmount),

    #[error(transparent)]
    TokenNotForDirectSale(TokenNotForDirectSale),

    #[error(transparent)]
    IdentityInTokenConfigurationNotFoundError(IdentityInTokenConfigurationNotFoundError),

    #[error(transparent)]
    IdentityMemberOfGroupNotFoundError(IdentityMemberOfGroupNotFoundError),

    #[error(transparent)]
    ModificationOfGroupActionMainParametersNotPermittedError(
        ModificationOfGroupActionMainParametersNotPermittedError,
    ),

    #[error(transparent)]
    IdentityToFreezeDoesNotExistError(IdentityToFreezeDoesNotExistError),

    #[error(transparent)]
    DataContractNotFoundError(DataContractNotFoundError),

    #[error(transparent)]
    InvalidTokenPositionStateError(InvalidTokenPositionStateError),

    #[error(transparent)]
    AddressDoesNotExistError(AddressDoesNotExistError),

    #[error(transparent)]
    AddressNotEnoughFundsError(AddressNotEnoughFundsError),

    #[error(transparent)]
    AddressesNotEnoughFundsError(AddressesNotEnoughFundsError),

    #[error(transparent)]
    AddressInvalidNonceError(AddressInvalidNonceError),

    #[error(transparent)]
    InvalidAnchorError(InvalidAnchorError),

    #[error(transparent)]
    NullifierAlreadySpentError(NullifierAlreadySpentError),

    #[error(transparent)]
    InvalidShieldedProofError(InvalidShieldedProofError),

    #[error(transparent)]
    InsufficientPoolNotesError(InsufficientPoolNotesError),

    #[error(transparent)]
    InsufficientShieldedFeeError(InsufficientShieldedFeeError),

    #[error(transparent)]
    DocumentContestIndexMismatchError(DocumentContestIndexMismatchError),

    #[error(transparent)]
    DocumentContestNotRequiredError(DocumentContestNotRequiredError),

    #[error(transparent)]
    ReferencedEntityNotFoundError(ReferencedEntityNotFoundError),

    #[error(transparent)]
    ReferencedDocumentTypeNotFoundError(ReferencedDocumentTypeNotFoundError),

    #[error(transparent)]
    ReferencedDocumentTypeDeletableError(ReferencedDocumentTypeDeletableError),

    #[error(transparent)]
    ReferencedIdentityKeyNotFoundError(ReferencedIdentityKeyNotFoundError),

    #[error(transparent)]
    ReferencedIdentityKeyDisabledError(ReferencedIdentityKeyDisabledError),

    #[error(transparent)]
    ReferencedKeyIdPropertyInvalidError(ReferencedKeyIdPropertyInvalidError),

    #[error(transparent)]
    ReferencedDocumentPropertyAgreementInvalidError(
        ReferencedDocumentPropertyAgreementInvalidError,
    ),

    #[error(transparent)]
    ReferencedDocumentPropertyMismatchError(ReferencedDocumentPropertyMismatchError),

    // Contract groups (protocol version 14).
    #[error(transparent)]
    ContractGroupAlreadyExistsError(ContractGroupAlreadyExistsError),

    #[error(transparent)]
    ContractGroupNotFoundError(ContractGroupNotFoundError),

    #[error(transparent)]
    IdentityNotContractGroupOwnerOrAdminError(IdentityNotContractGroupOwnerOrAdminError),

    #[error(transparent)]
    ContractGroupAdminNotFoundError(ContractGroupAdminNotFoundError),

    // Authentication key limits (protocol version 14).
    #[error(transparent)]
    IdentityPublicKeyBudgetExceededError(IdentityPublicKeyBudgetExceededError),

    #[error(transparent)]
    IdentityPublicKeyAlreadyExpiredError(IdentityPublicKeyAlreadyExpiredError),

    // Identity key limits update (protocol version 14).
    #[error(transparent)]
    IdentityPublicKeyLimitNotSetError(IdentityPublicKeyLimitNotSetError),

    #[error(transparent)]
    IdentityPublicKeyLimitNotRaisedError(IdentityPublicKeyLimitNotRaisedError),

    // Immutable document properties (protocol version 14).
    #[error(transparent)]
    DocumentImmutablePropertyChangedError(DocumentImmutablePropertyChangedError),

    // Once-per-identity token distribution (protocol version 14).
    #[error(transparent)]
    TokenOncePerIdentityDistributionAlreadyClaimedError(
        TokenOncePerIdentityDistributionAlreadyClaimedError,
    ),

    // Gas paid by the contract owner (protocol version 14).
    #[error(transparent)]
    GasFeesPaidByNotAllowedError(GasFeesPaidByNotAllowedError),

    #[error(transparent)]
    InconsistentGasFeesPaidByInBatchError(InconsistentGasFeesPaidByInBatchError),

    #[error(transparent)]
    GasSponsorInsufficientBalanceError(GasSponsorInsufficientBalanceError),

    // Contract moderation (protocol version 14).
    #[error(transparent)]
    ContractModerationNotEnabledError(ContractModerationNotEnabledError),

    #[error(transparent)]
    IdentityNotContractModeratorError(IdentityNotContractModeratorError),

    #[error(transparent)]
    ContractModerationTargetNotAllowedError(ContractModerationTargetNotAllowedError),

    #[error(transparent)]
    ContractUserAlreadyBannedError(ContractUserAlreadyBannedError),

    #[error(transparent)]
    ContractUserNotBannedError(ContractUserNotBannedError),

    #[error(transparent)]
    ContractUserNotSuspendedError(ContractUserNotSuspendedError),

    #[error(transparent)]
    ContractSuspensionNotInFutureError(ContractSuspensionNotInFutureError),

    #[error(transparent)]
    ContractUserBannedError(ContractUserBannedError),

    #[error(transparent)]
    ContractUserSuspendedError(ContractUserSuspendedError),

    #[error(transparent)]
    ContractModerationTargetNotFoundError(ContractModerationTargetNotFoundError),

    #[error(transparent)]
    ContractModeratorIdentityNotFoundError(ContractModeratorIdentityNotFoundError),

    #[error(transparent)]
    ContractModerationCounterpartyBarredError(ContractModerationCounterpartyBarredError),

    // Contract fee claims (protocol version 14).
    #[error(transparent)]
    ContractFeesAlreadyClaimedThisEpochError(ContractFeesAlreadyClaimedThisEpochError),

    #[error(transparent)]
    ContractFeesNothingToClaimError(ContractFeesNothingToClaimError),

    #[error(transparent)]
    ContractFeeClaimNotAllowedError(ContractFeeClaimNotAllowedError),

    // `refersTo: deletableDocument` (protocol version 14). Appended here,
    // away from the other reference errors, because the enum is append-only.
    #[error(transparent)]
    ReferencedDocumentTypeNotDeletableError(ReferencedDocumentTypeNotDeletableError),

    // Document deletion by moderators (protocol version 14).
    #[error(transparent)]
    DocumentTypeNotDeletableByModeratorsError(DocumentTypeNotDeletableByModeratorsError),

    // Document action fee agreements (protocol version 14).
    #[error(transparent)]
    DocumentActionFeeAgreementNotSetError(DocumentActionFeeAgreementNotSetError),

    #[error(transparent)]
    DocumentActionFeeAgreementMismatchError(DocumentActionFeeAgreementMismatchError),

    #[error(transparent)]
    DocumentActionFeeMultiplierNotToleratedError(DocumentActionFeeMultiplierNotToleratedError),

    // The moderators' deletion window (protocol version 14).
    #[error(transparent)]
    DocumentModerationWindowElapsedError(DocumentModerationWindowElapsedError),

    // The warning list (protocol version 14).
    #[error(transparent)]
    ContractUserNotWarnedError(ContractUserNotWarnedError),

    #[error(transparent)]
    ContractUserWarningLimitReachedError(ContractUserWarningLimitReachedError),
    // The moderators' restore of a deleted document (protocol version 14).
    #[error(transparent)]
    ContractDocumentRemovalNotFoundError(ContractDocumentRemovalNotFoundError),

    #[error(transparent)]
    DocumentRestoreWindowElapsedError(DocumentRestoreWindowElapsedError),

    #[error(transparent)]
    DocumentRestoreHashMismatchError(DocumentRestoreHashMismatchError),

    #[error(transparent)]
    ContractDocumentAlreadyRestoredError(ContractDocumentAlreadyRestoredError),

    // Elected moderation teams (protocol version 14).
    #[error(transparent)]
    ContractModeratedDocumentTypeNotYetUsableError(ContractModeratedDocumentTypeNotYetUsableError),

    // Contested indexes resolved without a Lock choice (protocol version 14).
    #[error(transparent)]
    VoteChoiceNotAllowedForVotePollError(VoteChoiceNotAllowedForVotePollError),

    // Requirements on a referenced contract (protocol version 14).
    #[error(transparent)]
    ReferencedContractRequirementNotMetError(ReferencedContractRequirementNotMetError),

    // Requirements on a referenced identity key (protocol version 14).
    #[error(transparent)]
    ReferencedIdentityKeyRequirementNotMetError(ReferencedIdentityKeyRequirementNotMetError),

    // Document references resolved through a unique index (protocol version 14).
    #[error(transparent)]
    ReferencedDocumentLookupInvalidError(ReferencedDocumentLookupInvalidError),

    // References to an element of a list of a referenced document (protocol version 14).
    #[error(transparent)]
    ReferencedDocumentListInvalidError(ReferencedDocumentListInvalidError),

    // Elected moderation teams moderating from their seated charter (protocol version 14).
    #[error(transparent)]
    ContractModerationAbilityNotGrantedError(ContractModerationAbilityNotGrantedError),

    #[error(transparent)]
    ModerationCharterAddedModeratorLimitReachedError(
        ModerationCharterAddedModeratorLimitReachedError,
    ),

    #[error(transparent)]
    DocumentActionFeeModeratorsShareMismatchError(DocumentActionFeeModeratorsShareMismatchError),

    // A seated moderation team's action names a reason its proposal lists (protocol version
    // 14).
    #[error(transparent)]
    ModerationReasonNotListedError(ModerationReasonNotListedError),
}

impl From<StateError> for ConsensusError {
    fn from(error: StateError) -> Self {
        Self::StateError(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consensus::state::contract_moderation::ContractModerationCounterpartyRole;
    use crate::consensus::state::identity::identity_public_key_limit_not_set_error::KeyLimit;
    use crate::data_contract::config::moderation::{ContractModerationList, ModerationAbility};
    use crate::data_contract::document_type::action_fees::agreement::{
        AgreedFeeMultiplier, DocumentActionFeeAgreement,
    };
    use crate::data_contract::document_type::action_fees::{
        ActionFeePricing, ContractFeePot, DocumentActionFee,
    };
    use crate::data_contract::document_type::{
        DocumentPropertyReferenceTarget, DocumentReferenceLookup, ListElementReference,
        LookupKeySource,
    };
    use crate::tokens::gas_fees_paid_by::GasFeesPaidBy;
    use crate::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
    use crate::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
    use crate::voting::vote_polls::VotePoll;
    use platform_value::Identifier;
    use std::collections::BTreeMap;

    /// `StateError` is encoded by variant position, so inserting a variant
    /// anywhere but the end silently reassigns the discriminant of every
    /// variant after it — consensus errors travel to WASM and JavaScript
    /// clients, which would then decode an existing error as a different one.
    /// These are the frozen discriminants of the first variant, of the variant
    /// that follows the document contest block (the one an insertion there
    /// would shift first), and of the variants appended since, down to the
    /// last one, which the test's final assertion pins.
    fn discriminant_of(error: StateError) -> u8 {
        let bytes = bincode::encode_to_vec(error, bincode::config::standard())
            .expect("expected to encode the state error");
        // Discriminants below 251 are a single byte under bincode's varint.
        bytes[0]
    }

    /// A reference error for an id reference encodes exactly as it did before
    /// lookup references existed: the lookup form is an appended variant of
    /// `DocumentPropertyReferenceTarget` (`PermanentDocumentLookup`), not a
    /// field of `PermanentDocument`, so a client decoding with an earlier dpp
    /// still reads every error an id reference produces. The bytes are pinned;
    /// `PermanentDocument` keeps variant 3 of the target, the lookup form
    /// takes 6.
    #[test]
    fn should_keep_the_encoding_of_a_reference_error_for_an_id_reference() {
        let id_reference = DocumentPropertyReferenceTarget::PermanentDocument {
            contract_id: None,
            document_type_name: "note".to_string(),
            property_agreement: BTreeMap::new(),
        };
        let error = StateError::ReferencedEntityNotFoundError(ReferencedEntityNotFoundError::new(
            Identifier::from([1; 32]),
            id_reference.clone(),
            "noteId".to_string(),
        ));
        let bytes = bincode::encode_to_vec(error, bincode::config::standard())
            .expect("expected to encode the state error");
        // StateError variant 93, the referenced id, target variant 3
        // (PermanentDocument: no contract id, "note", no agreement), the path.
        assert_eq!(
            hex::encode(bytes),
            concat!(
                "5d",
                "0101010101010101010101010101010101010101010101010101010101010101",
                "03",
                "00",
                "046e6f7465",
                "00",
                "066e6f74654964",
            )
        );

        let target_variant = |target: &DocumentPropertyReferenceTarget| {
            bincode::encode_to_vec(target, bincode::config::standard())
                .expect("expected to encode the target")[0]
        };
        assert_eq!(target_variant(&id_reference), 3);
        assert_eq!(
            target_variant(&DocumentPropertyReferenceTarget::PermanentDocumentLookup {
                contract_id: None,
                document_type_name: "note".to_string(),
                property_agreement: BTreeMap::new(),
                lookup: DocumentReferenceLookup {
                    index: "byOwner".to_string(),
                    keys: [("$ownerId".to_string(), LookupKeySource::ReferenceValue)].into(),
                },
            }),
            6
        );
        // A list element reference is appended after the expressions (anyOf
        // 7, allOf 8, pinned in `reference_expression.rs`)
        assert_eq!(
            target_variant(&DocumentPropertyReferenceTarget::ListElement(
                ListElementReference {
                    contract_id: None,
                    document_type_name: "electedCharter".to_string(),
                    property_agreement: [("electedCharterId".to_string(), "$id".to_string())]
                        .into(),
                    in_list: "members".to_string(),
                }
            )),
            9
        );
        // A deletable document found through a lookup is appended after it
        assert_eq!(
            target_variant(&DocumentPropertyReferenceTarget::DeletableDocumentLookup {
                contract_id: None,
                document_type_name: "note".to_string(),
                property_agreement: BTreeMap::new(),
                lookup: DocumentReferenceLookup {
                    index: "byOwner".to_string(),
                    keys: [("$ownerId".to_string(), LookupKeySource::ReferenceValue)].into(),
                },
            }),
            10
        );
    }

    #[test]
    fn state_error_discriminants_are_frozen() {
        assert_eq!(
            discriminant_of(StateError::DataContractAlreadyPresentError(
                DataContractAlreadyPresentError::new(Identifier::from([1; 32]))
            )),
            0
        );
        assert_eq!(
            discriminant_of(StateError::DocumentNotFoundError(
                DocumentNotFoundError::new(Identifier::from([1; 32]))
            )),
            8
        );
        assert_eq!(
            discriminant_of(StateError::InsufficientShieldedFeeError(
                InsufficientShieldedFeeError::new("fee".to_string())
            )),
            90
        );
        assert_eq!(
            discriminant_of(StateError::DocumentContestIndexMismatchError(
                DocumentContestIndexMismatchError::new(
                    Identifier::from([1; 32]),
                    "expected".to_string(),
                    "provided".to_string(),
                )
            )),
            91
        );
        assert_eq!(
            discriminant_of(StateError::DocumentContestNotRequiredError(
                DocumentContestNotRequiredError::new(
                    Identifier::from([1; 32]),
                    "provided".to_string(),
                )
            )),
            92
        );
        assert_eq!(
            discriminant_of(StateError::ReferencedEntityNotFoundError(
                ReferencedEntityNotFoundError::new(
                    Identifier::from([1; 32]),
                    crate::data_contract::document_type::DocumentPropertyReferenceTarget::Identity,
                    "toUserId".to_string(),
                )
            )),
            93
        );
        assert_eq!(
            discriminant_of(StateError::ReferencedDocumentTypeNotFoundError(
                ReferencedDocumentTypeNotFoundError::new(
                    Identifier::from([1; 32]),
                    "note".to_string(),
                    "parentNoteId".to_string(),
                )
            )),
            94
        );
        assert_eq!(
            discriminant_of(StateError::ReferencedDocumentTypeDeletableError(
                ReferencedDocumentTypeDeletableError::new(
                    Identifier::from([1; 32]),
                    "note".to_string(),
                    "parentNoteId".to_string(),
                )
            )),
            95
        );
        assert_eq!(
            discriminant_of(StateError::ReferencedIdentityKeyNotFoundError(
                ReferencedIdentityKeyNotFoundError::new(
                    Identifier::from([1; 32]),
                    2,
                    "toUserId".to_string(),
                )
            )),
            96
        );
        assert_eq!(
            discriminant_of(StateError::ReferencedIdentityKeyDisabledError(
                ReferencedIdentityKeyDisabledError::new(
                    Identifier::from([1; 32]),
                    2,
                    "toUserId".to_string(),
                )
            )),
            97
        );
        assert_eq!(
            discriminant_of(StateError::ReferencedKeyIdPropertyInvalidError(
                ReferencedKeyIdPropertyInvalidError::new(
                    "recipientKeyIndex".to_string(),
                    "toUserId".to_string(),
                    "missing".to_string(),
                )
            )),
            98
        );
        assert_eq!(
            discriminant_of(StateError::ReferencedDocumentPropertyAgreementInvalidError(
                ReferencedDocumentPropertyAgreementInvalidError::new(
                    "like.postId".to_string(),
                    "hashtag".to_string(),
                    "hashtag".to_string(),
                    "missing".to_string(),
                )
            )),
            99
        );
        assert_eq!(
            discriminant_of(StateError::ReferencedDocumentPropertyMismatchError(
                ReferencedDocumentPropertyMismatchError::new(
                    "postId".to_string(),
                    "hashtag".to_string(),
                    "hashtag".to_string(),
                )
            )),
            100
        );
        // Contract groups (protocol version 14).
        let group_id = Identifier::from([1u8; 32]);
        let identity_id = Identifier::from([2u8; 32]);
        assert_eq!(
            discriminant_of(StateError::ContractGroupAlreadyExistsError(
                ContractGroupAlreadyExistsError::new(group_id)
            )),
            101
        );
        assert_eq!(
            discriminant_of(StateError::ContractGroupNotFoundError(
                ContractGroupNotFoundError::new(group_id)
            )),
            102
        );
        assert_eq!(
            discriminant_of(StateError::IdentityNotContractGroupOwnerOrAdminError(
                IdentityNotContractGroupOwnerOrAdminError::new(identity_id, group_id)
            )),
            103
        );
        assert_eq!(
            discriminant_of(StateError::ContractGroupAdminNotFoundError(
                ContractGroupAdminNotFoundError::new(group_id, identity_id)
            )),
            104
        );
        // Authentication key limits (protocol version 14): the tail of the enum.
        assert_eq!(
            discriminant_of(StateError::IdentityPublicKeyBudgetExceededError(
                IdentityPublicKeyBudgetExceededError::new(identity_id, 1, 2, 3)
            )),
            105
        );
        assert_eq!(
            discriminant_of(StateError::IdentityPublicKeyAlreadyExpiredError(
                IdentityPublicKeyAlreadyExpiredError::new(1, 2, 3)
            )),
            106
        );
        // Identity key limits update (protocol version 14): the tail of the enum.
        assert_eq!(
            discriminant_of(StateError::IdentityPublicKeyLimitNotSetError(
                IdentityPublicKeyLimitNotSetError::new(1, KeyLimit::Budget)
            )),
            107
        );
        assert_eq!(
            discriminant_of(StateError::IdentityPublicKeyLimitNotRaisedError(
                IdentityPublicKeyLimitNotRaisedError::new(1, KeyLimit::Expiry, 2, 3)
            )),
            108
        );
        // Immutable document properties (protocol version 14).
        assert_eq!(
            discriminant_of(StateError::DocumentImmutablePropertyChangedError(
                DocumentImmutablePropertyChangedError::new(
                    identity_id,
                    "post".to_string(),
                    "author".to_string()
                )
            )),
            109
        );
        // Once-per-identity token distribution (protocol version 14).
        assert_eq!(
            discriminant_of(
                StateError::TokenOncePerIdentityDistributionAlreadyClaimedError(
                    TokenOncePerIdentityDistributionAlreadyClaimedError::new(
                        group_id,
                        identity_id,
                        1
                    )
                )
            ),
            110
        );
        // Gas paid by the contract owner (protocol version 14): the tail of the enum.
        assert_eq!(
            discriminant_of(StateError::GasFeesPaidByNotAllowedError(
                GasFeesPaidByNotAllowedError::new(
                    "post".to_string(),
                    "create".to_string(),
                    GasFeesPaidBy::ContractOwner,
                    GasFeesPaidBy::DocumentOwner,
                )
            )),
            111
        );
        assert_eq!(
            discriminant_of(StateError::InconsistentGasFeesPaidByInBatchError(
                InconsistentGasFeesPaidByInBatchError::new(Some(identity_id), None)
            )),
            112
        );
        assert_eq!(
            discriminant_of(StateError::GasSponsorInsufficientBalanceError(
                GasSponsorInsufficientBalanceError::new(identity_id, 1, 2)
            )),
            113
        );
        // Contract moderation (protocol version 14): every variant, the tail of the enum.
        assert_eq!(
            discriminant_of(StateError::ContractModerationNotEnabledError(
                ContractModerationNotEnabledError::new(group_id, ContractModerationList::Banlist,)
            )),
            114
        );
        assert_eq!(
            discriminant_of(StateError::IdentityNotContractModeratorError(
                IdentityNotContractModeratorError::new(group_id, identity_id)
            )),
            115
        );
        assert_eq!(
            discriminant_of(StateError::ContractModerationTargetNotAllowedError(
                ContractModerationTargetNotAllowedError::new(group_id, identity_id)
            )),
            116
        );
        assert_eq!(
            discriminant_of(StateError::ContractUserAlreadyBannedError(
                ContractUserAlreadyBannedError::new(group_id, identity_id)
            )),
            117
        );
        assert_eq!(
            discriminant_of(StateError::ContractUserNotBannedError(
                ContractUserNotBannedError::new(group_id, identity_id)
            )),
            118
        );
        assert_eq!(
            discriminant_of(StateError::ContractUserNotSuspendedError(
                ContractUserNotSuspendedError::new(group_id, identity_id)
            )),
            119
        );
        assert_eq!(
            discriminant_of(StateError::ContractSuspensionNotInFutureError(
                ContractSuspensionNotInFutureError::new(group_id, identity_id, 1, 2)
            )),
            120
        );
        assert_eq!(
            discriminant_of(StateError::ContractUserBannedError(
                ContractUserBannedError::new(group_id, identity_id)
            )),
            121
        );
        assert_eq!(
            discriminant_of(StateError::ContractUserSuspendedError(
                ContractUserSuspendedError::new(group_id, identity_id, 1)
            )),
            122
        );
        assert_eq!(
            discriminant_of(StateError::ContractModerationTargetNotFoundError(
                ContractModerationTargetNotFoundError::new(group_id, identity_id)
            )),
            123
        );
        assert_eq!(
            discriminant_of(StateError::ContractModeratorIdentityNotFoundError(
                ContractModeratorIdentityNotFoundError::new(group_id, identity_id)
            )),
            124
        );
        assert_eq!(
            discriminant_of(StateError::ContractModerationCounterpartyBarredError(
                ContractModerationCounterpartyBarredError::new(
                    group_id,
                    identity_id,
                    ContractModerationCounterpartyRole::Recipient,
                )
            )),
            125
        );
        // Contract fee claims (protocol version 14).
        assert_eq!(
            discriminant_of(StateError::ContractFeesAlreadyClaimedThisEpochError(
                ContractFeesAlreadyClaimedThisEpochError::new(
                    group_id,
                    ContractFeePot::Moderators,
                    7
                )
            )),
            126
        );
        assert_eq!(
            discriminant_of(StateError::ContractFeesNothingToClaimError(
                ContractFeesNothingToClaimError::new(group_id, ContractFeePot::Owner)
            )),
            127
        );
        assert_eq!(
            discriminant_of(StateError::ContractFeeClaimNotAllowedError(
                ContractFeeClaimNotAllowedError::new(group_id, ContractFeePot::Owner, identity_id)
            )),
            128
        );
        // `refersTo: deletableDocument` (protocol version 14).
        assert_eq!(
            discriminant_of(StateError::ReferencedDocumentTypeNotDeletableError(
                ReferencedDocumentTypeNotDeletableError::new(
                    group_id,
                    "note".to_string(),
                    "noteId".to_string()
                )
            )),
            129
        );
        // Document deletion by moderators (protocol version 14).
        assert_eq!(
            discriminant_of(StateError::DocumentTypeNotDeletableByModeratorsError(
                DocumentTypeNotDeletableByModeratorsError::new(group_id, "post".to_string())
            )),
            130
        );
        // Document action fee agreements (protocol version 14).
        let declared_fee = DocumentActionFee {
            owner: 1,
            moderators: 2,
        };
        let agreed_fee_multiplier = AgreedFeeMultiplier {
            known_permille: 1000,
            increase_tolerance_percent: 20,
        };
        assert_eq!(
            discriminant_of(StateError::DocumentActionFeeAgreementNotSetError(
                DocumentActionFeeAgreementNotSetError::new(
                    "post".to_string(),
                    "create".to_string(),
                    ActionFeePricing::Fixed,
                    declared_fee,
                )
            )),
            131
        );
        assert_eq!(
            discriminant_of(StateError::DocumentActionFeeAgreementMismatchError(
                DocumentActionFeeAgreementMismatchError::new(
                    "post".to_string(),
                    "create".to_string(),
                    ActionFeePricing::Fixed,
                    declared_fee,
                    &DocumentActionFeeAgreement::for_declared_fee(
                        ActionFeePricing::FeeMultiplier,
                        declared_fee,
                        agreed_fee_multiplier,
                    ),
                )
            )),
            132
        );
        assert_eq!(
            discriminant_of(StateError::DocumentActionFeeMultiplierNotToleratedError(
                DocumentActionFeeMultiplierNotToleratedError::new(
                    "post".to_string(),
                    "create".to_string(),
                    agreed_fee_multiplier,
                    1500,
                )
            )),
            133
        );
        // The moderators' deletion window (protocol version 14).
        assert_eq!(
            discriminant_of(StateError::DocumentModerationWindowElapsedError(
                DocumentModerationWindowElapsedError::new(group_id, identity_id, 1, 2, 3)
            )),
            134
        );
        // The warning list (protocol version 14).
        assert_eq!(
            discriminant_of(StateError::ContractUserNotWarnedError(
                ContractUserNotWarnedError::new(group_id, identity_id)
            )),
            135
        );
        assert_eq!(
            discriminant_of(StateError::ContractUserWarningLimitReachedError(
                ContractUserWarningLimitReachedError::new(group_id, identity_id, 16)
            )),
            136
        );
        // The moderators' restore of a deleted document (protocol version 14).
        assert_eq!(
            discriminant_of(StateError::ContractDocumentRemovalNotFoundError(
                ContractDocumentRemovalNotFoundError::new(
                    group_id,
                    "post".to_string(),
                    identity_id
                )
            )),
            137
        );
        assert_eq!(
            discriminant_of(StateError::DocumentRestoreWindowElapsedError(
                DocumentRestoreWindowElapsedError::new(group_id, identity_id, 1, 2, 3)
            )),
            138
        );
        assert_eq!(
            discriminant_of(StateError::DocumentRestoreHashMismatchError(
                DocumentRestoreHashMismatchError::new(group_id, identity_id, [1; 32], [2; 32])
            )),
            139
        );
        assert_eq!(
            discriminant_of(StateError::ContractDocumentAlreadyRestoredError(
                ContractDocumentAlreadyRestoredError::new(group_id, identity_id, identity_id, 4)
            )),
            140
        );
        // Elected moderation teams (protocol version 14): the tail of the enum.
        assert_eq!(
            discriminant_of(StateError::ContractModeratedDocumentTypeNotYetUsableError(
                ContractModeratedDocumentTypeNotYetUsableError::new(group_id, "post".to_string())
            )),
            141
        );
        // Contested indexes without a Lock choice (protocol version 14).
        assert_eq!(
            discriminant_of(StateError::VoteChoiceNotAllowedForVotePollError(
                VoteChoiceNotAllowedForVotePollError::new(
                    VotePoll::ContestedDocumentResourceVotePoll(
                        ContestedDocumentResourceVotePoll {
                            contract_id: Identifier::new([7; 32]),
                            document_type_name: "domain".to_string(),
                            index_name: "parentNameAndLabel".to_string(),
                            index_values: vec![],
                        }
                    ),
                    ResourceVoteChoice::Lock,
                )
            )),
            142
        );
        // Requirements on a referenced contract (protocol version 14).
        assert_eq!(
            discriminant_of(StateError::ReferencedContractRequirementNotMetError(
                ReferencedContractRequirementNotMetError::new(
                    group_id,
                    "moderation".to_string(),
                    "elected".to_string(),
                    "targetContractId".to_string(),
                )
            )),
            143
        );
        // Requirements on a referenced identity key (protocol version 14).
        assert_eq!(
            discriminant_of(StateError::ReferencedIdentityKeyRequirementNotMetError(
                ReferencedIdentityKeyRequirementNotMetError::new(
                    "joinRequest".to_string(),
                    "recipientId".to_string(),
                    group_id,
                    2,
                    "purpose".to_string(),
                    "decryption".to_string(),
                    "encryption".to_string(),
                )
            )),
            144
        );
        // Document references resolved through a unique index (protocol version 14).
        assert_eq!(
            discriminant_of(StateError::ReferencedDocumentLookupInvalidError(
                ReferencedDocumentLookupInvalidError::new(
                    "electedCharter.members".to_string(),
                    "bySubmittedCharter".to_string(),
                    "is not unique".to_string(),
                )
            )),
            145
        );
        // References to an element of a list of a referenced document (protocol version 14).
        assert_eq!(
            discriminant_of(StateError::ReferencedDocumentListInvalidError(
                ReferencedDocumentListInvalidError::new(
                    "resignation.memberId".to_string(),
                    "members".to_string(),
                    "is not a typed array of identifiers".to_string(),
                )
            )),
            146
        );
        // Elected moderation teams moderating from their seated charter (protocol version
        // 14): the tail of the enum.
        assert_eq!(
            discriminant_of(StateError::ContractModerationAbilityNotGrantedError(
                ContractModerationAbilityNotGrantedError::new(
                    group_id,
                    ModerationAbility::DeleteDocuments,
                    Some("post".to_string()),
                )
            )),
            147
        );
        assert_eq!(
            discriminant_of(
                StateError::ModerationCharterAddedModeratorLimitReachedError(
                    ModerationCharterAddedModeratorLimitReachedError::new(group_id, identity_id, 2)
                )
            ),
            148
        );
        assert_eq!(
            discriminant_of(StateError::DocumentActionFeeModeratorsShareMismatchError(
                DocumentActionFeeModeratorsShareMismatchError::new(
                    "post".to_string(),
                    "create".to_string(),
                    100,
                    50,
                    Some(60),
                )
            )),
            149
        );
        // A seated moderation team's action names a reason its proposal lists (protocol
        // version 14): the tail of the enum.
        assert_eq!(
            discriminant_of(StateError::ModerationReasonNotListedError(
                ModerationReasonNotListedError::new(group_id, identity_id, None)
            )),
            150
        );
    }
}
