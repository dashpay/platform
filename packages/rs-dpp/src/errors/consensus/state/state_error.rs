use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

use crate::errors::consensus::state::data_contract::data_contract_already_present_error::DataContractAlreadyPresentError;
use crate::errors::consensus::state::data_contract::data_contract_config_update_error::DataContractConfigUpdateError;
use crate::errors::consensus::state::data_contract::data_contract_is_readonly_error::DataContractIsReadonlyError;
#[cfg(feature = "state-transition-validation")]
use crate::errors::consensus::state::data_trigger::DataTriggerError;
use crate::errors::consensus::state::document::document_already_present_error::DocumentAlreadyPresentError;
use crate::errors::consensus::state::document::document_not_found_error::DocumentNotFoundError;
use crate::errors::consensus::state::document::document_owner_id_mismatch_error::DocumentOwnerIdMismatchError;
use crate::errors::consensus::state::document::document_timestamp_window_violation_error::DocumentTimestampWindowViolationError;
use crate::errors::consensus::state::document::document_timestamps_mismatch_error::DocumentTimestampsMismatchError;
use crate::errors::consensus::state::document::duplicate_unique_index_error::DuplicateUniqueIndexError;
use crate::errors::consensus::state::document::invalid_document_revision_error::InvalidDocumentRevisionError;
use crate::errors::consensus::state::identity::duplicated_identity_public_key_id_state_error::DuplicatedIdentityPublicKeyIdStateError;
use crate::errors::consensus::state::identity::duplicated_identity_public_key_state_error::DuplicatedIdentityPublicKeyStateError;
use crate::errors::consensus::state::identity::identity_public_key_is_disabled_error::IdentityPublicKeyIsDisabledError;
use crate::errors::consensus::state::identity::identity_public_key_is_read_only_error::IdentityPublicKeyIsReadOnlyError;
use crate::errors::consensus::state::identity::invalid_identity_public_key_id_error::InvalidIdentityPublicKeyIdError;
use crate::errors::consensus::state::identity::invalid_identity_revision_error::InvalidIdentityRevisionError;
use crate::errors::consensus::state::identity::max_identity_public_key_limit_reached_error::MaxIdentityPublicKeyLimitReachedError;
use crate::errors::consensus::state::identity::missing_identity_public_key_ids_error::MissingIdentityPublicKeyIdsError;
use crate::errors::consensus::state::identity::{IdentityAlreadyExistsError, IdentityInsufficientBalanceError, RecipientIdentityDoesNotExistError};
use crate::errors::consensus::ConsensusError;
use crate::errors::consensus::state::data_contract::data_contract_not_found_error::DataContractNotFoundError;
use crate::errors::consensus::state::data_contract::data_contract_update_action_not_allowed_error::DataContractUpdateActionNotAllowedError;
use crate::errors::consensus::state::data_contract::data_contract_update_permission_error::DataContractUpdatePermissionError;
use crate::errors::consensus::state::data_contract::document_type_update_error::DocumentTypeUpdateError;
use crate::errors::consensus::state::document::document_contest_currently_locked_error::DocumentContestCurrentlyLockedError;
use crate::errors::consensus::state::document::document_contest_document_with_same_id_already_present_error::DocumentContestDocumentWithSameIdAlreadyPresentError;
use crate::errors::consensus::state::document::document_contest_identity_already_contestant::DocumentContestIdentityAlreadyContestantError;
use crate::errors::consensus::state::document::document_contest_not_joinable_error::DocumentContestNotJoinableError;
use crate::errors::consensus::state::document::document_contest_not_paid_for_error::DocumentContestNotPaidForError;
use crate::errors::consensus::state::document::document_incorrect_purchase_price_error::DocumentIncorrectPurchasePriceError;
use crate::errors::consensus::state::document::document_not_for_sale_error::DocumentNotForSaleError;
use crate::errors::consensus::state::group::{GroupActionAlreadyCompletedError, GroupActionAlreadySignedByIdentityError, GroupActionDoesNotExistError, IdentityNotMemberOfGroupError, ModificationOfGroupActionMainParametersNotPermittedError};
use crate::errors::consensus::state::group::identity_for_group_not_found_error::IdentityMemberOfGroupNotFoundError;
use crate::errors::consensus::state::identity::identity_for_token_configuration_not_found_error::IdentityInTokenConfigurationNotFoundError;
use crate::errors::consensus::state::identity::identity_public_key_already_exists_for_unique_contract_bounds_error::IdentityPublicKeyAlreadyExistsForUniqueContractBoundsError;
use crate::errors::consensus::state::identity::identity_to_freeze_does_not_exist_error::IdentityToFreezeDoesNotExistError;
use crate::errors::consensus::state::identity::invalid_identity_contract_nonce_error::InvalidIdentityNonceError;
use crate::errors::consensus::state::identity::missing_transfer_key_error::MissingTransferKeyError;
use crate::errors::consensus::state::identity::no_transfer_key_for_core_withdrawal_available_error::NoTransferKeyForCoreWithdrawalAvailableError;
use crate::errors::consensus::state::prefunded_specialized_balances::prefunded_specialized_balance_insufficient_error::PrefundedSpecializedBalanceInsufficientError;
use crate::errors::consensus::state::prefunded_specialized_balances::prefunded_specialized_balance_not_found_error::PrefundedSpecializedBalanceNotFoundError;
use crate::errors::consensus::state::token::identity_does_not_have_enough_token_balance_error::IdentityDoesNotHaveEnoughTokenBalanceError;
use crate::errors::consensus::state::token::identity_has_not_agreed_to_pay_required_token_amount_error::IdentityHasNotAgreedToPayRequiredTokenAmountError;
use crate::errors::consensus::state::token::identity_token_account_already_frozen_error::IdentityTokenAccountAlreadyFrozenError;
use crate::errors::consensus::state::token::identity_token_account_frozen_error::IdentityTokenAccountFrozenError;
use crate::errors::consensus::state::token::identity_token_account_not_frozen_error::IdentityTokenAccountNotFrozenError;
use crate::errors::consensus::state::token::identity_trying_to_pay_with_wrong_token_error::IdentityTryingToPayWithWrongTokenError;
use crate::errors::consensus::state::token::invalid_group_position_error::InvalidGroupPositionError;
use crate::errors::consensus::state::token::invalid_token_claim_no_current_rewards::InvalidTokenClaimNoCurrentRewards;
use crate::errors::consensus::state::token::invalid_token_claim_property_mismatch::InvalidTokenClaimPropertyMismatch;
use crate::errors::consensus::state::token::invalid_token_claim_wrong_claimant::InvalidTokenClaimWrongClaimant;
use crate::errors::consensus::state::token::invalid_token_position_error::InvalidTokenPositionStateError;
use crate::errors::consensus::state::token::new_authorized_action_taker_group_does_not_exist_error::NewAuthorizedActionTakerGroupDoesNotExistError;
use crate::errors::consensus::state::token::new_authorized_action_taker_identity_does_not_exist_error::NewAuthorizedActionTakerIdentityDoesNotExistError;
use crate::errors::consensus::state::token::new_authorized_action_taker_main_group_not_set_error::NewAuthorizedActionTakerMainGroupNotSetError;
use crate::errors::consensus::state::token::new_tokens_destination_identity_does_not_exist_error::NewTokensDestinationIdentityDoesNotExistError;
use crate::errors::consensus::state::token::pre_programmed_distribution_timestamp_in_past_error::PreProgrammedDistributionTimestampInPastError;
use crate::errors::consensus::state::token::required_token_payment_info_not_set_error::RequiredTokenPaymentInfoNotSetError;
use crate::errors::consensus::state::token::token_already_paused_error::TokenAlreadyPausedError;
use crate::errors::consensus::state::token::token_amount_under_minimum_sale_amount::TokenAmountUnderMinimumSaleAmount;
use crate::errors::consensus::state::token::token_direct_purchase_user_price_too_low::TokenDirectPurchaseUserPriceTooLow;
use crate::errors::consensus::state::token::token_is_paused_error::TokenIsPausedError;
use crate::errors::consensus::state::token::token_mint_past_max_supply_error::TokenMintPastMaxSupplyError;
use crate::errors::consensus::state::token::token_not_for_direct_sale::TokenNotForDirectSale;
use crate::errors::consensus::state::token::token_not_paused_error::TokenNotPausedError;
use crate::errors::consensus::state::token::token_setting_max_supply_to_less_than_current_supply_error::TokenSettingMaxSupplyToLessThanCurrentSupplyError;
use crate::errors::consensus::state::token::token_transfer_recipient_identity_not_exist_error::TokenTransferRecipientIdentityNotExistError;
use crate::errors::consensus::state::token::unauthorized_token_action_error::UnauthorizedTokenActionError;
use crate::errors::consensus::state::voting::masternode_incorrect_voter_identity_id_error::MasternodeIncorrectVoterIdentityIdError;
use crate::errors::consensus::state::voting::masternode_incorrect_voting_address_error::MasternodeIncorrectVotingAddressError;
use crate::errors::consensus::state::voting::masternode_not_found_error::MasternodeNotFoundError;
use crate::errors::consensus::state::voting::masternode_vote_already_present_error::MasternodeVoteAlreadyPresentError;
use crate::errors::consensus::state::voting::masternode_voted_too_many_times::MasternodeVotedTooManyTimesError;
use crate::errors::consensus::state::voting::vote_poll_not_available_for_voting_error::VotePollNotAvailableForVotingError;
use crate::errors::consensus::state::voting::vote_poll_not_found_error::VotePollNotFoundError;

// use super::document::document_timestamps_are_equal_error::DocumentTimestampsAreEqualError;
use crate::errors::consensus::state::document::document_timestamps_are_equal_error::DocumentTimestampsAreEqualError;

#[derive(
    Error, Debug, PartialEq, Encode, Decode, PlatformSerialize, PlatformDeserialize, Clone,
)]
#[cfg_attr(feature = "apple", ferment_macro::export)]
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
}

impl From<StateError> for ConsensusError {
    fn from(error: StateError) -> Self {
        Self::StateError(error)
    }
}

pub enum TestStateError {
    Empty,
    #[cfg(feature = "state-transition-validation")]
    Conditional,
}

#[cfg(feature = "ferment")]
pub mod fermented {
    use ferment::{boxed, FFIConversionFrom, FFIConversionTo};

    #[repr(C)]
    #[derive(Clone)]
    pub enum TestStateError {
        Empty,
        #[cfg(feature = "state-transition-validation")]
        Conditional,
    }

    impl FFIConversionFrom<super::TestStateError> for TestStateError {
        unsafe fn ffi_from_const(ffi: *const Self) -> super::TestStateError {
            let ffi_ref = &*ffi;
            match ffi_ref {
                TestStateError::Empty => super::TestStateError::Empty,
                #[cfg(feature = "state-transition-validation")]
                TestStateError::Conditional => super::TestStateError::Conditional,
            }
        }
    }
    impl FFIConversionTo<super::TestStateError> for TestStateError {
        unsafe fn ffi_to_const(obj: super::TestStateError) -> *const Self {
            boxed(match obj {
                super::TestStateError::Empty => TestStateError::Empty,
                #[cfg(feature = "state-transition-validation")]
                super::TestStateError::Conditional => TestStateError::Conditional,
            })
        }
    }

    impl Drop for TestStateError {
        fn drop(&mut self) {}
    }
    #[no_mangle]
    pub unsafe extern "C" fn dpp_errors_consensus_state_state_error_TestStateError_Empty_ctor(
    ) -> *mut TestStateError {
        boxed(TestStateError::Empty)
    }
    #[cfg(feature = "state-transition-validation")]
    #[no_mangle]
    pub unsafe extern "C" fn dpp_errors_consensus_state_state_error_TestStateError_Conditional_ctor(
    ) -> *mut TestStateError {
        boxed(TestStateError::Conditional)
    }
}
