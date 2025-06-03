use crate::errors::consensus::basic::{
    IncompatibleProtocolVersionErrorWasm, InvalidIdentifierErrorWasm,
    InvalidSignaturePublicKeyPurposeErrorWasm, JsonSchemaErrorWasm,
    UnsupportedProtocolVersionErrorWasm, UnsupportedVersionErrorWasm,
};
use dpp::consensus::ConsensusError as DPPConsensusError;

use crate::errors::consensus::basic::identity::{
    DuplicatedIdentityPublicKeyErrorWasm, DuplicatedIdentityPublicKeyIdErrorWasm,
    IdentityAssetLockProofLockedTransactionMismatchErrorWasm,
    IdentityAssetLockStateTransitionReplayErrorWasm,
    IdentityAssetLockTransactionIsNotFoundErrorWasm,
    IdentityAssetLockTransactionOutPointAlreadyExistsErrorWasm,
    IdentityAssetLockTransactionOutPointNotEnoughBalanceErrorWasm,
    IdentityAssetLockTransactionOutputNotFoundErrorWasm, IdentityCreditTransferToSelfErrorWasm,
    IdentityInsufficientBalanceErrorWasm, InvalidAssetLockProofCoreChainHeightErrorWasm,
    InvalidAssetLockProofTransactionHeightErrorWasm,
    InvalidAssetLockTransactionOutputReturnSizeErrorWasm,
    InvalidIdentityAssetLockProofChainLockValidationErrorWasm,
    InvalidIdentityAssetLockTransactionErrorWasm,
    InvalidIdentityAssetLockTransactionOutputErrorWasm,
    InvalidIdentityCreditTransferAmountErrorWasm,
    InvalidIdentityCreditWithdrawalTransitionCoreFeeErrorWasm,
    InvalidIdentityCreditWithdrawalTransitionOutputScriptErrorWasm,
    InvalidIdentityKeySignatureErrorWasm, InvalidIdentityPublicKeyDataErrorWasm,
    InvalidIdentityPublicKeySecurityLevelErrorWasm, InvalidInstantAssetLockProofErrorWasm,
    InvalidInstantAssetLockProofSignatureErrorWasm, MissingMasterPublicKeyErrorWasm,
    NotImplementedIdentityCreditWithdrawalTransitionPoolingErrorWasm,
};

use crate::errors::consensus::state::identity::{
    DuplicatedIdentityPublicKeyIdStateErrorWasm, DuplicatedIdentityPublicKeyStateErrorWasm,
    InvalidIdentityNonceErrorWasm, MissingIdentityPublicKeyIdsErrorWasm,
};
use dpp::consensus::basic::decode::VersionError;
use dpp::consensus::basic::BasicError::{
    DuplicatedIdentityPublicKeyBasicError, DuplicatedIdentityPublicKeyIdBasicError,
    IdentityAssetLockProofLockedTransactionMismatchError,
    IdentityAssetLockStateTransitionReplayError, IdentityAssetLockTransactionIsNotFoundError,
    IdentityAssetLockTransactionOutPointAlreadyConsumedError,
    IdentityAssetLockTransactionOutPointNotEnoughBalanceError,
    IdentityAssetLockTransactionOutputNotFoundError, IncompatibleProtocolVersionError,
    IncompatibleRe2PatternError, InvalidAssetLockProofCoreChainHeightError,
    InvalidAssetLockProofTransactionHeightError, InvalidAssetLockTransactionOutputReturnSizeError,
    InvalidIdentityAssetLockProofChainLockValidationError,
    InvalidIdentityAssetLockTransactionError, InvalidIdentityAssetLockTransactionOutputError,
    InvalidIdentityCreditTransferAmountError,
    InvalidIdentityCreditWithdrawalTransitionCoreFeeError,
    InvalidIdentityCreditWithdrawalTransitionOutputScriptError, InvalidIdentityPublicKeyDataError,
    InvalidIdentityPublicKeySecurityLevelError, InvalidInstantAssetLockProofError,
    InvalidInstantAssetLockProofSignatureError, MissingMasterPublicKeyError,
    NotImplementedIdentityCreditWithdrawalTransitionPoolingError, ProtocolVersionParsingError,
    UnsupportedProtocolVersionError, UnsupportedVersionError,
};
use dpp::consensus::basic::{BasicError, UnsupportedFeatureError};
use dpp::consensus::fee::fee_error::FeeError;
use dpp::consensus::signature::SignatureError;
use dpp::consensus::state::state_error::StateError;

use dpp::consensus::state::data_trigger::DataTriggerError::{
  DataTriggerConditionError, DataTriggerExecutionError, DataTriggerInvalidResultError,
};
use wasm_bindgen::{JsError, JsValue};
use dpp::consensus::basic::data_contract::{ContestedUniqueIndexOnMutableDocumentTypeError, ContestedUniqueIndexWithUniqueIndexError, DataContractTokenConfigurationUpdateError, DecimalsOverLimitError, DuplicateKeywordsError, GroupExceedsMaxMembersError, GroupHasTooFewMembersError, GroupMemberHasPowerOfZeroError, GroupMemberHasPowerOverLimitError, GroupNonUnilateralMemberPowerHasLessThanRequiredPowerError, GroupPositionDoesNotExistError, GroupRequiredPowerIsInvalidError, GroupTotalPowerLessThanRequiredError, InvalidDescriptionLengthError, InvalidDocumentTypeRequiredSecurityLevelError, InvalidKeywordCharacterError, InvalidKeywordLengthError, InvalidTokenBaseSupplyError, InvalidTokenDistributionFunctionDivideByZeroError, InvalidTokenDistributionFunctionIncoherenceError, InvalidTokenDistributionFunctionInvalidParameterError, InvalidTokenDistributionFunctionInvalidParameterTupleError, InvalidTokenLanguageCodeError, InvalidTokenNameCharacterError, InvalidTokenNameLengthError, MainGroupIsNotDefinedError, NewTokensDestinationIdentityOptionRequiredError, NonContiguousContractGroupPositionsError, NonContiguousContractTokenPositionsError, RedundantDocumentPaidForByTokenWithContractId, TokenPaymentByBurningOnlyAllowedOnInternalTokenError, TooManyKeywordsError, UnknownDocumentActionTokenEffectError, UnknownDocumentCreationRestrictionModeError, UnknownGasFeesPaidByError, UnknownSecurityLevelError, UnknownStorageKeyRequirementsError, UnknownTradeModeError, UnknownTransferableTypeError};
use dpp::consensus::basic::document::{ContestedDocumentsTemporarilyNotAllowedError, DocumentCreationNotAllowedError, DocumentFieldMaxSizeExceededError, MaxDocumentsTransitionsExceededError, MissingPositionsInDocumentTypePropertiesError};
use dpp::consensus::basic::group::GroupActionNotAllowedOnTransitionError;
use dpp::consensus::basic::identity::{DataContractBoundsNotPresentError, DisablingKeyIdAlsoBeingAddedInSameTransitionError, InvalidIdentityCreditWithdrawalTransitionAmountError, InvalidIdentityUpdateTransitionDisableKeysError, InvalidIdentityUpdateTransitionEmptyError, TooManyMasterPublicKeyError, WithdrawalOutputScriptNotAllowedWhenSigningWithOwnerKeyError};
use dpp::consensus::basic::overflow_error::OverflowError;
use dpp::consensus::basic::token::{ChoosingTokenMintRecipientNotAllowedError, ContractHasNoTokensError, DestinationIdentityForTokenMintingNotSetError, InvalidActionIdError, InvalidTokenAmountError, InvalidTokenConfigUpdateNoChangeError, InvalidTokenIdError, InvalidTokenNoteTooBigError, InvalidTokenPositionError, MissingDefaultLocalizationError, TokenNoteOnlyAllowedWhenProposerError, TokenTransferToOurselfError, InvalidTokenDistributionTimeIntervalNotMinuteAlignedError, InvalidTokenDistributionTimeIntervalTooShortError, InvalidTokenDistributionBlockIntervalTooShortError};
use dpp::consensus::state::data_contract::data_contract_not_found_error::DataContractNotFoundError;
use dpp::consensus::state::data_contract::data_contract_update_action_not_allowed_error::DataContractUpdateActionNotAllowedError;
use dpp::consensus::state::data_contract::document_type_update_error::DocumentTypeUpdateError;
use dpp::consensus::state::document::document_contest_currently_locked_error::DocumentContestCurrentlyLockedError;
use dpp::consensus::state::document::document_contest_document_with_same_id_already_present_error::DocumentContestDocumentWithSameIdAlreadyPresentError;
use dpp::consensus::state::document::document_contest_identity_already_contestant::DocumentContestIdentityAlreadyContestantError;
use dpp::consensus::state::document::document_contest_not_joinable_error::DocumentContestNotJoinableError;
use dpp::consensus::state::document::document_contest_not_paid_for_error::DocumentContestNotPaidForError;
use dpp::consensus::state::document::document_incorrect_purchase_price_error::DocumentIncorrectPurchasePriceError;
use dpp::consensus::state::document::document_not_for_sale_error::DocumentNotForSaleError;
use dpp::consensus::state::group::{GroupActionAlreadyCompletedError, GroupActionAlreadySignedByIdentityError, GroupActionDoesNotExistError, IdentityMemberOfGroupNotFoundError, IdentityNotMemberOfGroupError, ModificationOfGroupActionMainParametersNotPermittedError};
use dpp::consensus::state::identity::identity_for_token_configuration_not_found_error::IdentityInTokenConfigurationNotFoundError;
use dpp::consensus::state::identity::identity_public_key_already_exists_for_unique_contract_bounds_error::IdentityPublicKeyAlreadyExistsForUniqueContractBoundsError;
use dpp::consensus::state::identity::identity_to_freeze_does_not_exist_error::IdentityToFreezeDoesNotExistError;
use dpp::consensus::state::identity::master_public_key_update_error::MasterPublicKeyUpdateError;
use dpp::consensus::state::identity::missing_transfer_key_error::MissingTransferKeyError;
use dpp::consensus::state::identity::no_transfer_key_for_core_withdrawal_available_error::NoTransferKeyForCoreWithdrawalAvailableError;
use dpp::consensus::state::identity::RecipientIdentityDoesNotExistError;
use dpp::consensus::state::prefunded_specialized_balances::prefunded_specialized_balance_insufficient_error::PrefundedSpecializedBalanceInsufficientError;
use dpp::consensus::state::prefunded_specialized_balances::prefunded_specialized_balance_not_found_error::PrefundedSpecializedBalanceNotFoundError;
use dpp::consensus::state::token::{IdentityDoesNotHaveEnoughTokenBalanceError, IdentityTokenAccountNotFrozenError, IdentityTokenAccountFrozenError, TokenIsPausedError, IdentityTokenAccountAlreadyFrozenError, UnauthorizedTokenActionError, TokenSettingMaxSupplyToLessThanCurrentSupplyError, TokenMintPastMaxSupplyError, NewTokensDestinationIdentityDoesNotExistError, NewAuthorizedActionTakerIdentityDoesNotExistError, NewAuthorizedActionTakerGroupDoesNotExistError, NewAuthorizedActionTakerMainGroupNotSetError, InvalidGroupPositionError, TokenAlreadyPausedError, TokenNotPausedError, InvalidTokenClaimPropertyMismatch, InvalidTokenClaimNoCurrentRewards, InvalidTokenClaimWrongClaimant, TokenTransferRecipientIdentityNotExistError, PreProgrammedDistributionTimestampInPastError, IdentityHasNotAgreedToPayRequiredTokenAmountError, RequiredTokenPaymentInfoNotSetError, IdentityTryingToPayWithWrongTokenError, TokenDirectPurchaseUserPriceTooLow, TokenAmountUnderMinimumSaleAmount, TokenNotForDirectSale, InvalidTokenPositionStateError};
use dpp::consensus::state::voting::masternode_incorrect_voter_identity_id_error::MasternodeIncorrectVoterIdentityIdError;
use dpp::consensus::state::voting::masternode_incorrect_voting_address_error::MasternodeIncorrectVotingAddressError;
use dpp::consensus::state::voting::masternode_not_found_error::MasternodeNotFoundError;
use dpp::consensus::state::voting::masternode_vote_already_present_error::MasternodeVoteAlreadyPresentError;
use dpp::consensus::state::voting::masternode_voted_too_many_times::MasternodeVotedTooManyTimesError;
use dpp::consensus::state::voting::vote_poll_not_available_for_voting_error::VotePollNotAvailableForVotingError;
use dpp::consensus::state::voting::vote_poll_not_found_error::VotePollNotFoundError;

use crate::errors::consensus::basic::data_contract::{
    DataContractErrorWasm, DataContractHaveNewUniqueIndexErrorWasm,
    DataContractImmutablePropertiesUpdateErrorWasm,
    DataContractInvalidIndexDefinitionUpdateErrorWasm, DataContractUniqueIndicesChangedErrorWasm,
    IncompatibleDataContractSchemaErrorWasm, IncompatibleDocumentTypeSchemaErrorWasm,
    InvalidDataContractIdErrorWasm, InvalidDocumentTypeNameErrorWasm,
};
use crate::errors::consensus::basic::document::{
    DocumentTransitionsAreAbsentErrorWasm, DuplicateDocumentTransitionsWithIdsErrorWasm,
    DuplicateDocumentTransitionsWithIndicesErrorWasm, IdentityContractNonceOutOfBoundsErrorWasm,
    InvalidDocumentTransitionActionErrorWasm, InvalidDocumentTransitionIdErrorWasm,
    MissingDataContractIdErrorWasm, MissingDocumentTypeErrorWasm,
};
use crate::errors::consensus::basic::state_transition::{
    InvalidStateTransitionTypeErrorWasm, MissingStateTransitionTypeErrorWasm,
    StateTransitionMaxSizeExceededErrorWasm,
};
use crate::errors::consensus::signature::{
    BasicBLSErrorWasm, BasicECDSAErrorWasm, IdentityNotFoundErrorWasm,
    SignatureShouldNotBePresentErrorWasm,
};
use crate::errors::consensus::state::data_contract::data_trigger::{
    DataTriggerConditionErrorWasm, DataTriggerExecutionErrorWasm, DataTriggerInvalidResultErrorWasm,
};
use crate::errors::consensus::state::data_contract::{
    DataContractAlreadyPresentErrorWasm, DataContractConfigUpdateErrorWasm,
    DataContractIsReadonlyErrorWasm, DataContractUpdatePermissionErrorWasm,
};
use crate::errors::consensus::state::document::{
    DocumentAlreadyPresentErrorWasm, DocumentNotFoundErrorWasm, DocumentOwnerIdMismatchErrorWasm,
    DocumentTimestampWindowViolationErrorWasm, DocumentTimestampsMismatchErrorWasm,
    DuplicateUniqueIndexErrorWasm, InvalidDocumentRevisionErrorWasm,
};
use crate::errors::consensus::state::identity::{
    IdentityAlreadyExistsErrorWasm, IdentityPublicKeyIsDisabledErrorWasm,
    IdentityPublicKeyIsReadOnlyErrorWasm, InvalidIdentityPublicKeyIdErrorWasm,
    InvalidIdentityRevisionErrorWasm, MaxIdentityPublicKeyLimitReachedErrorWasm,
};

use crate::errors::consensus::basic::data_contract::{
    DataContractMaxDepthExceedErrorWasm, DuplicateIndexErrorWasm, DuplicateIndexNameErrorWasm,
    IncompatibleRe2PatternErrorWasm, InvalidCompoundIndexErrorWasm,
    InvalidDataContractVersionErrorWasm, InvalidIndexPropertyTypeErrorWasm,
    InvalidIndexedPropertyConstraintErrorWasm, InvalidJsonSchemaRefErrorWasm,
    SystemPropertyIndexAlreadyPresentErrorWasm, UndefinedIndexPropertyErrorWasm,
    UniqueIndicesLimitReachedErrorWasm,
};
use crate::errors::consensus::basic::decode::{
    ProtocolVersionParsingErrorWasm, SerializedObjectParsingErrorWasm,
};
use crate::errors::consensus::basic::document::{
    DataContractNotPresentErrorWasm, InconsistentCompoundIndexDataErrorWasm,
    InvalidDocumentTypeErrorWasm, MissingDocumentTransitionActionErrorWasm,
    MissingDocumentTransitionTypeErrorWasm,
};
use crate::errors::consensus::basic::identity::{
    InvalidIdentityPublicKeyTypeErrorWasm, MissingPublicKeyErrorWasm,
};
use crate::errors::consensus::basic::{
    InvalidSignaturePublicKeySecurityLevelErrorWasm, InvalidStateTransitionSignatureErrorWasm,
    JsonSchemaCompilationErrorWasm, PublicKeyIsDisabledErrorWasm,
    PublicKeySecurityLevelNotMetErrorWasm, WrongPublicKeyPurposeErrorWasm,
};
use crate::errors::consensus::fee::BalanceIsNotEnoughErrorWasm;

use crate::errors::consensus::value_error::ValueErrorWasm;
use crate::generic_consensus_error;

use super::state::document::DocumentTimestampsAreEqualErrorWasm;

pub fn from_consensus_error_ref(e: &DPPConsensusError) -> JsValue {
    match e {
        DPPConsensusError::FeeError(e) => match e {
            FeeError::BalanceIsNotEnoughError(e) => BalanceIsNotEnoughErrorWasm::from(e).into(),
        },
        DPPConsensusError::SignatureError(e) => from_signature_error(e),
        DPPConsensusError::StateError(state_error) => from_state_error(state_error),
        DPPConsensusError::BasicError(basic_error) => from_basic_error(basic_error),
        DPPConsensusError::DefaultError => JsError::new("DefaultError").into(),
        #[cfg(test)]
        #[allow(unreachable_patterns)]
        e => JsError::new(&format!("unsupported error: {:?}", e)).into(),
    }
}

pub fn from_state_error(state_error: &StateError) -> JsValue {
    match state_error {
        StateError::DuplicatedIdentityPublicKeyIdStateError(e) => {
            DuplicatedIdentityPublicKeyIdStateErrorWasm::from(e).into()
        }
        StateError::DuplicatedIdentityPublicKeyStateError(e) => {
            DuplicatedIdentityPublicKeyStateErrorWasm::from(e).into()
        }
        StateError::DocumentAlreadyPresentError(e) => {
            DocumentAlreadyPresentErrorWasm::from(e).into()
        }
        StateError::DataContractAlreadyPresentError(e) => {
            DataContractAlreadyPresentErrorWasm::from(e).into()
        }
        StateError::DocumentNotFoundError(e) => DocumentNotFoundErrorWasm::from(e).into(),
        StateError::DocumentOwnerIdMismatchError(e) => {
            DocumentOwnerIdMismatchErrorWasm::from(e).into()
        }
        StateError::DocumentTimestampsMismatchError(e) => {
            DocumentTimestampsMismatchErrorWasm::from(e).into()
        }
        StateError::DocumentTimestampWindowViolationError(e) => {
            DocumentTimestampWindowViolationErrorWasm::from(e).into()
        }
        StateError::DuplicateUniqueIndexError(e) => DuplicateUniqueIndexErrorWasm::from(e).into(),
        StateError::InvalidDocumentRevisionError(e) => {
            InvalidDocumentRevisionErrorWasm::from(e).into()
        }
        StateError::InvalidIdentityRevisionError(e) => {
            InvalidIdentityRevisionErrorWasm::from(e).into()
        }
        StateError::IdentityPublicKeyIsReadOnlyError(e) => {
            IdentityPublicKeyIsReadOnlyErrorWasm::from(e).into()
        }
        StateError::InvalidIdentityPublicKeyIdError(e) => {
            InvalidIdentityPublicKeyIdErrorWasm::from(e).into()
        }
        StateError::MaxIdentityPublicKeyLimitReachedError(e) => {
            MaxIdentityPublicKeyLimitReachedErrorWasm::from(e).into()
        }
        StateError::IdentityPublicKeyIsDisabledError(e) => {
            IdentityPublicKeyIsDisabledErrorWasm::from(e).into()
        }
        StateError::MissingIdentityPublicKeyIdsError(e) => {
            MissingIdentityPublicKeyIdsErrorWasm::from(e).into()
        }
        StateError::DataTriggerError(data_trigger_error) => match data_trigger_error {
            DataTriggerConditionError(e) => DataTriggerConditionErrorWasm::from(e).into(),
            DataTriggerExecutionError(e) => DataTriggerExecutionErrorWasm::from(e).into(),
            DataTriggerInvalidResultError(e) => DataTriggerInvalidResultErrorWasm::from(e).into(),
        },
        StateError::IdentityAlreadyExistsError(e) => {
            let wasm_error: IdentityAlreadyExistsErrorWasm = e.into();
            wasm_error.into()
        }
        StateError::IdentityInsufficientBalanceError(e) => {
            let wasm_error: IdentityInsufficientBalanceErrorWasm = e.into();
            wasm_error.into()
        }
        StateError::DocumentTimestampsAreEqualError(e) => {
            DocumentTimestampsAreEqualErrorWasm::from(e).into()
        }
        StateError::DataContractIsReadonlyError(e) => {
            DataContractIsReadonlyErrorWasm::from(e).into()
        }
        StateError::DataContractConfigUpdateError(e) => {
            DataContractConfigUpdateErrorWasm::from(e).into()
        }
        StateError::InvalidIdentityNonceError(e) => InvalidIdentityNonceErrorWasm::from(e).into(),
        StateError::IdentityPublicKeyAlreadyExistsForUniqueContractBoundsError(e) => {
            generic_consensus_error!(
                IdentityPublicKeyAlreadyExistsForUniqueContractBoundsError,
                e
            )
            .into()
        }
        StateError::DocumentTypeUpdateError(e) => {
            generic_consensus_error!(DocumentTypeUpdateError, e).into()
        }
        StateError::DocumentNotForSaleError(e) => {
            generic_consensus_error!(DocumentNotForSaleError, e).into()
        }
        StateError::DocumentIncorrectPurchasePriceError(e) => {
            generic_consensus_error!(DocumentIncorrectPurchasePriceError, e).into()
        }
        StateError::PrefundedSpecializedBalanceInsufficientError(e) => {
            generic_consensus_error!(PrefundedSpecializedBalanceInsufficientError, e).into()
        }
        StateError::PrefundedSpecializedBalanceNotFoundError(e) => {
            generic_consensus_error!(PrefundedSpecializedBalanceNotFoundError, e).into()
        }
        StateError::DataContractUpdatePermissionError(e) => {
            DataContractUpdatePermissionErrorWasm::from(e).into()
        }
        StateError::MasternodeNotFoundError(e) => {
            generic_consensus_error!(MasternodeNotFoundError, e).into()
        }
        StateError::DocumentContestCurrentlyLockedError(e) => {
            generic_consensus_error!(DocumentContestCurrentlyLockedError, e).into()
        }
        StateError::DocumentContestNotJoinableError(e) => {
            generic_consensus_error!(DocumentContestNotJoinableError, e).into()
        }
        StateError::DocumentContestIdentityAlreadyContestantError(e) => {
            generic_consensus_error!(DocumentContestIdentityAlreadyContestantError, e).into()
        }
        StateError::VotePollNotFoundError(e) => {
            generic_consensus_error!(VotePollNotFoundError, e).into()
        }
        StateError::VotePollNotAvailableForVotingError(e) => {
            generic_consensus_error!(VotePollNotAvailableForVotingError, e).into()
        }
        StateError::MasternodeVotedTooManyTimesError(e) => {
            generic_consensus_error!(MasternodeVotedTooManyTimesError, e).into()
        }
        StateError::MasternodeVoteAlreadyPresentError(e) => {
            generic_consensus_error!(MasternodeVoteAlreadyPresentError, e).into()
        }
        StateError::MasternodeIncorrectVotingAddressError(e) => {
            generic_consensus_error!(MasternodeIncorrectVotingAddressError, e).into()
        }
        StateError::MasternodeIncorrectVoterIdentityIdError(e) => {
            generic_consensus_error!(MasternodeIncorrectVoterIdentityIdError, e).into()
        }
        StateError::DocumentContestDocumentWithSameIdAlreadyPresentError(e) => {
            generic_consensus_error!(DocumentContestDocumentWithSameIdAlreadyPresentError, e).into()
        }
        StateError::MissingTransferKeyError(e) => {
            generic_consensus_error!(MissingTransferKeyError, e).into()
        }
        StateError::NoTransferKeyForCoreWithdrawalAvailableError(e) => {
            generic_consensus_error!(NoTransferKeyForCoreWithdrawalAvailableError, e).into()
        }
        StateError::DocumentContestNotPaidForError(e) => {
            generic_consensus_error!(DocumentContestNotPaidForError, e).into()
        }
        StateError::RecipientIdentityDoesNotExistError(e) => {
            generic_consensus_error!(RecipientIdentityDoesNotExistError, e).into()
        }
        StateError::IdentityDoesNotHaveEnoughTokenBalanceError(e) => {
            generic_consensus_error!(IdentityDoesNotHaveEnoughTokenBalanceError, e).into()
        }
        StateError::UnauthorizedTokenActionError(e) => {
            generic_consensus_error!(UnauthorizedTokenActionError, e).into()
        }
        StateError::IdentityTokenAccountFrozenError(e) => {
            generic_consensus_error!(IdentityTokenAccountFrozenError, e).into()
        }
        StateError::TokenIsPausedError(e) => generic_consensus_error!(TokenIsPausedError, e).into(),
        StateError::IdentityTokenAccountAlreadyFrozenError(e) => {
            generic_consensus_error!(IdentityTokenAccountAlreadyFrozenError, e).into()
        }
        StateError::TokenAlreadyPausedError(e) => {
            generic_consensus_error!(TokenAlreadyPausedError, e).into()
        }
        StateError::TokenNotPausedError(e) => {
            generic_consensus_error!(TokenNotPausedError, e).into()
        }
        StateError::IdentityNotMemberOfGroupError(e) => {
            generic_consensus_error!(IdentityNotMemberOfGroupError, e).into()
        }
        StateError::GroupActionDoesNotExistError(e) => {
            generic_consensus_error!(GroupActionDoesNotExistError, e).into()
        }
        StateError::GroupActionAlreadyCompletedError(e) => {
            generic_consensus_error!(GroupActionAlreadyCompletedError, e).into()
        }
        StateError::GroupActionAlreadySignedByIdentityError(e) => {
            generic_consensus_error!(GroupActionAlreadySignedByIdentityError, e).into()
        }
        StateError::IdentityTokenAccountNotFrozenError(e) => {
            generic_consensus_error!(IdentityTokenAccountNotFrozenError, e).into()
        }
        StateError::DataContractUpdateActionNotAllowedError(e) => {
            generic_consensus_error!(DataContractUpdateActionNotAllowedError, e).into()
        }
        StateError::TokenSettingMaxSupplyToLessThanCurrentSupplyError(e) => {
            generic_consensus_error!(TokenSettingMaxSupplyToLessThanCurrentSupplyError, e).into()
        }
        StateError::TokenMintPastMaxSupplyError(e) => {
            generic_consensus_error!(TokenMintPastMaxSupplyError, e).into()
        }
        StateError::NewTokensDestinationIdentityDoesNotExistError(e) => {
            generic_consensus_error!(NewTokensDestinationIdentityDoesNotExistError, e).into()
        }
        StateError::NewAuthorizedActionTakerIdentityDoesNotExistError(e) => {
            generic_consensus_error!(NewAuthorizedActionTakerIdentityDoesNotExistError, e).into()
        }
        StateError::NewAuthorizedActionTakerGroupDoesNotExistError(e) => {
            generic_consensus_error!(NewAuthorizedActionTakerGroupDoesNotExistError, e).into()
        }
        StateError::NewAuthorizedActionTakerMainGroupNotSetError(e) => {
            generic_consensus_error!(NewAuthorizedActionTakerMainGroupNotSetError, e).into()
        }
        StateError::InvalidGroupPositionError(e) => {
            generic_consensus_error!(InvalidGroupPositionError, e).into()
        }
        StateError::InvalidTokenClaimPropertyMismatch(e) => {
            generic_consensus_error!(InvalidTokenClaimPropertyMismatch, e).into()
        }
        StateError::InvalidTokenClaimNoCurrentRewards(e) => {
            generic_consensus_error!(InvalidTokenClaimNoCurrentRewards, e).into()
        }
        StateError::InvalidTokenClaimWrongClaimant(e) => {
            generic_consensus_error!(InvalidTokenClaimWrongClaimant, e).into()
        }
        StateError::TokenTransferRecipientIdentityNotExistError(e) => {
            generic_consensus_error!(TokenTransferRecipientIdentityNotExistError, e).into()
        }
        StateError::PreProgrammedDistributionTimestampInPastError(e) => {
            generic_consensus_error!(PreProgrammedDistributionTimestampInPastError, e).into()
        }
        StateError::IdentityHasNotAgreedToPayRequiredTokenAmountError(e) => {
            generic_consensus_error!(IdentityHasNotAgreedToPayRequiredTokenAmountError, e).into()
        }
        StateError::RequiredTokenPaymentInfoNotSetError(e) => {
            generic_consensus_error!(RequiredTokenPaymentInfoNotSetError, e).into()
        }
        StateError::IdentityTryingToPayWithWrongTokenError(e) => {
            generic_consensus_error!(IdentityTryingToPayWithWrongTokenError, e).into()
        }
        StateError::TokenDirectPurchaseUserPriceTooLow(e) => {
            generic_consensus_error!(TokenDirectPurchaseUserPriceTooLow, e).into()
        }
        StateError::TokenAmountUnderMinimumSaleAmount(e) => {
            generic_consensus_error!(TokenAmountUnderMinimumSaleAmount, e).into()
        }
        StateError::TokenNotForDirectSale(e) => {
            generic_consensus_error!(TokenNotForDirectSale, e).into()
        }
        StateError::IdentityInTokenConfigurationNotFoundError(e) => {
            generic_consensus_error!(IdentityInTokenConfigurationNotFoundError, e).into()
        }
        StateError::IdentityMemberOfGroupNotFoundError(e) => {
            generic_consensus_error!(IdentityMemberOfGroupNotFoundError, e).into()
        }
        StateError::ModificationOfGroupActionMainParametersNotPermittedError(e) => {
            generic_consensus_error!(ModificationOfGroupActionMainParametersNotPermittedError, e)
                .into()
        }
        StateError::IdentityToFreezeDoesNotExistError(e) => {
            generic_consensus_error!(IdentityToFreezeDoesNotExistError, e).into()
        }
        StateError::DataContractNotFoundError(e) => {
            generic_consensus_error!(DataContractNotFoundError, e).into()
        }
        StateError::InvalidTokenPositionStateError(e) => {
            generic_consensus_error!(InvalidTokenPositionStateError, e).into()
        }
    }
}

// TODO: Move as From/TryInto trait implementation to wasm error modules
fn from_basic_error(basic_error: &BasicError) -> JsValue {
    match basic_error {
        BasicError::ValueError(value_error) => ValueErrorWasm::from(value_error).into(),
        BasicError::DataContractNotPresentError(err) => {
            DataContractNotPresentErrorWasm::from(err).into()
        }
        BasicError::InvalidDataContractVersionError(err) => {
            InvalidDataContractVersionErrorWasm::from(err).into()
        }
        BasicError::DataContractMaxDepthExceedError(err) => {
            DataContractMaxDepthExceedErrorWasm::from(err).into()
        }
        BasicError::InvalidDocumentTypeError(err) => InvalidDocumentTypeErrorWasm::from(err).into(),
        BasicError::DuplicateIndexNameError(err) => DuplicateIndexNameErrorWasm::from(err).into(),
        BasicError::InvalidJsonSchemaRefError(err) => {
            InvalidJsonSchemaRefErrorWasm::from(err).into()
        }
        BasicError::UniqueIndicesLimitReachedError(err) => {
            UniqueIndicesLimitReachedErrorWasm::from(err).into()
        }
        BasicError::SystemPropertyIndexAlreadyPresentError(err) => {
            SystemPropertyIndexAlreadyPresentErrorWasm::from(err).into()
        }
        BasicError::UndefinedIndexPropertyError(err) => {
            UndefinedIndexPropertyErrorWasm::from(err).into()
        }
        BasicError::InvalidIndexPropertyTypeError(err) => {
            InvalidIndexPropertyTypeErrorWasm::from(err).into()
        }
        BasicError::InvalidIndexedPropertyConstraintError(err) => {
            InvalidIndexedPropertyConstraintErrorWasm::from(err).into()
        }
        BasicError::InvalidCompoundIndexError(err) => {
            InvalidCompoundIndexErrorWasm::from(err).into()
        }
        BasicError::DuplicateIndexError(err) => DuplicateIndexErrorWasm::from(err).into(),
        BasicError::JsonSchemaCompilationError(error) => {
            JsonSchemaCompilationErrorWasm::from(error).into()
        }
        BasicError::InconsistentCompoundIndexDataError(err) => {
            InconsistentCompoundIndexDataErrorWasm::from(err).into()
        }
        BasicError::MissingDocumentTransitionTypeError(err) => {
            MissingDocumentTransitionTypeErrorWasm::from(err).into()
        }
        BasicError::MissingDocumentTypeError(err) => MissingDocumentTypeErrorWasm::from(err).into(),
        BasicError::MissingDocumentTransitionActionError(err) => {
            MissingDocumentTransitionActionErrorWasm::from(err).into()
        }
        BasicError::InvalidDocumentTransitionActionError(err) => {
            InvalidDocumentTransitionActionErrorWasm::from(err).into()
        }
        BasicError::InvalidDocumentTransitionIdError(err) => {
            InvalidDocumentTransitionIdErrorWasm::from(err).into()
        }
        BasicError::DuplicateDocumentTransitionsWithIndicesError(err) => {
            DuplicateDocumentTransitionsWithIndicesErrorWasm::from(err).into()
        }
        BasicError::DuplicateDocumentTransitionsWithIdsError(err) => {
            DuplicateDocumentTransitionsWithIdsErrorWasm::from(err).into()
        }
        BasicError::MissingDataContractIdBasicError(err) => {
            MissingDataContractIdErrorWasm::from(err).into()
        }
        BasicError::InvalidIdentifierError(err) => InvalidIdentifierErrorWasm::from(err).into(),
        BasicError::DataContractUniqueIndicesChangedError(err) => {
            DataContractUniqueIndicesChangedErrorWasm::from(err).into()
        }
        BasicError::DataContractInvalidIndexDefinitionUpdateError(err) => {
            DataContractInvalidIndexDefinitionUpdateErrorWasm::from(err).into()
        }
        BasicError::DataContractHaveNewUniqueIndexError(err) => {
            DataContractHaveNewUniqueIndexErrorWasm::from(err).into()
        }
        BasicError::MissingStateTransitionTypeError(err) => {
            MissingStateTransitionTypeErrorWasm::from(err).into()
        }
        BasicError::InvalidStateTransitionTypeError(err) => {
            InvalidStateTransitionTypeErrorWasm::from(err).into()
        }
        BasicError::StateTransitionMaxSizeExceededError(err) => {
            StateTransitionMaxSizeExceededErrorWasm::from(err).into()
        }
        BasicError::DataContractImmutablePropertiesUpdateError(err) => {
            DataContractImmutablePropertiesUpdateErrorWasm::from(err).into()
        }
        BasicError::IncompatibleDataContractSchemaError(err) => {
            IncompatibleDataContractSchemaErrorWasm::from(err).into()
        }
        BasicError::IncompatibleDocumentTypeSchemaError(err) => {
            IncompatibleDocumentTypeSchemaErrorWasm::from(err).into()
        }
        BasicError::InvalidIdentityKeySignatureError(err) => {
            InvalidIdentityKeySignatureErrorWasm::from(err).into()
        }
        BasicError::InvalidDataContractIdError(err) => {
            InvalidDataContractIdErrorWasm::from(err).into()
        }
        BasicError::IdentityCreditTransferToSelfError(err) => {
            IdentityCreditTransferToSelfErrorWasm::from(err).into()
        }
        BasicError::NonceOutOfBoundsError(err) => {
            IdentityContractNonceOutOfBoundsErrorWasm::from(err).into()
        }
        BasicError::InvalidDocumentTypeNameError(err) => {
            InvalidDocumentTypeNameErrorWasm::from(err).into()
        }
        ProtocolVersionParsingError(e) => ProtocolVersionParsingErrorWasm::from(e).into(),
        BasicError::SerializedObjectParsingError(e) => {
            SerializedObjectParsingErrorWasm::from(e).into()
        }
        BasicError::JsonSchemaError(e) => JsonSchemaErrorWasm::from(e).into(),
        UnsupportedProtocolVersionError(e) => UnsupportedProtocolVersionErrorWasm::from(e).into(),
        UnsupportedVersionError(e) => UnsupportedVersionErrorWasm::from(e).into(),
        IncompatibleProtocolVersionError(e) => IncompatibleProtocolVersionErrorWasm::from(e).into(),
        DuplicatedIdentityPublicKeyIdBasicError(e) => {
            DuplicatedIdentityPublicKeyIdErrorWasm::from(e).into()
        }
        InvalidIdentityPublicKeyDataError(e) => {
            InvalidIdentityPublicKeyDataErrorWasm::from(e).into()
        }
        InvalidIdentityPublicKeySecurityLevelError(e) => {
            InvalidIdentityPublicKeySecurityLevelErrorWasm::from(e).into()
        }
        DuplicatedIdentityPublicKeyBasicError(e) => {
            DuplicatedIdentityPublicKeyErrorWasm::from(e).into()
        }
        MissingMasterPublicKeyError(e) => MissingMasterPublicKeyErrorWasm::from(e).into(),
        IdentityAssetLockTransactionOutPointAlreadyConsumedError(e) => {
            IdentityAssetLockTransactionOutPointAlreadyExistsErrorWasm::from(e).into()
        }
        IdentityAssetLockTransactionOutPointNotEnoughBalanceError(e) => {
            IdentityAssetLockTransactionOutPointNotEnoughBalanceErrorWasm::from(e).into()
        }
        InvalidIdentityAssetLockTransactionOutputError(e) => {
            InvalidIdentityAssetLockTransactionOutputErrorWasm::from(e).into()
        }
        IdentityAssetLockStateTransitionReplayError(e) => {
            IdentityAssetLockStateTransitionReplayErrorWasm::from(e).into()
        }
        InvalidAssetLockTransactionOutputReturnSizeError(e) => {
            InvalidAssetLockTransactionOutputReturnSizeErrorWasm::from(e).into()
        }
        IdentityAssetLockTransactionOutputNotFoundError(e) => {
            IdentityAssetLockTransactionOutputNotFoundErrorWasm::from(e).into()
        }
        InvalidIdentityAssetLockTransactionError(e) => {
            InvalidIdentityAssetLockTransactionErrorWasm::from(e).into()
        }
        InvalidInstantAssetLockProofError(e) => {
            InvalidInstantAssetLockProofErrorWasm::from(e).into()
        }
        InvalidInstantAssetLockProofSignatureError(e) => {
            InvalidInstantAssetLockProofSignatureErrorWasm::from(e).into()
        }
        InvalidIdentityAssetLockProofChainLockValidationError(e) => {
            InvalidIdentityAssetLockProofChainLockValidationErrorWasm::from(e).into()
        }
        IdentityAssetLockProofLockedTransactionMismatchError(e) => {
            IdentityAssetLockProofLockedTransactionMismatchErrorWasm::from(e).into()
        }
        IdentityAssetLockTransactionIsNotFoundError(e) => {
            IdentityAssetLockTransactionIsNotFoundErrorWasm::from(e).into()
        }
        InvalidAssetLockProofCoreChainHeightError(e) => {
            InvalidAssetLockProofCoreChainHeightErrorWasm::from(e).into()
        }
        InvalidAssetLockProofTransactionHeightError(e) => {
            InvalidAssetLockProofTransactionHeightErrorWasm::from(e).into()
        }
        InvalidIdentityCreditTransferAmountError(e) => {
            InvalidIdentityCreditTransferAmountErrorWasm::from(e).into()
        }
        InvalidIdentityCreditWithdrawalTransitionCoreFeeError(e) => {
            InvalidIdentityCreditWithdrawalTransitionCoreFeeErrorWasm::from(e).into()
        }
        InvalidIdentityCreditWithdrawalTransitionOutputScriptError(e) => {
            InvalidIdentityCreditWithdrawalTransitionOutputScriptErrorWasm::from(e).into()
        }
        NotImplementedIdentityCreditWithdrawalTransitionPoolingError(e) => {
            NotImplementedIdentityCreditWithdrawalTransitionPoolingErrorWasm::from(e).into()
        }
        IncompatibleRe2PatternError(err) => IncompatibleRe2PatternErrorWasm::from(err).into(),
        BasicError::VersionError(err) => generic_consensus_error!(VersionError, err).into(),
        BasicError::ContractError(e) => DataContractErrorWasm::from(e).into(),
        BasicError::UnknownSecurityLevelError(e) => {
            generic_consensus_error!(UnknownSecurityLevelError, e).into()
        }
        BasicError::UnknownStorageKeyRequirementsError(e) => {
            generic_consensus_error!(UnknownStorageKeyRequirementsError, e).into()
        }
        BasicError::DataContractBoundsNotPresentError(e) => {
            generic_consensus_error!(DataContractBoundsNotPresentError, e).into()
        }
        BasicError::MissingPositionsInDocumentTypePropertiesError(e) => {
            generic_consensus_error!(MissingPositionsInDocumentTypePropertiesError, e).into()
        }
        BasicError::MaxDocumentsTransitionsExceededError(e) => {
            generic_consensus_error!(MaxDocumentsTransitionsExceededError, e).into()
        }
        BasicError::DisablingKeyIdAlsoBeingAddedInSameTransitionError(e) => {
            generic_consensus_error!(DisablingKeyIdAlsoBeingAddedInSameTransitionError, e).into()
        }
        BasicError::TooManyMasterPublicKeyError(e) => {
            generic_consensus_error!(TooManyMasterPublicKeyError, e).into()
        }
        BasicError::MasterPublicKeyUpdateError(e) => {
            generic_consensus_error!(MasterPublicKeyUpdateError, e).into()
        }
        BasicError::InvalidDocumentTypeRequiredSecurityLevelError(e) => {
            generic_consensus_error!(InvalidDocumentTypeRequiredSecurityLevelError, e).into()
        }
        BasicError::InvalidIdentityCreditWithdrawalTransitionAmountError(e) => {
            generic_consensus_error!(InvalidIdentityCreditWithdrawalTransitionAmountError, e).into()
        }
        BasicError::InvalidIdentityUpdateTransitionEmptyError(e) => {
            generic_consensus_error!(InvalidIdentityUpdateTransitionEmptyError, e).into()
        }
        BasicError::InvalidIdentityUpdateTransitionDisableKeysError(e) => {
            generic_consensus_error!(InvalidIdentityUpdateTransitionDisableKeysError, e).into()
        }
        BasicError::DocumentTransitionsAreAbsentError(e) => {
            DocumentTransitionsAreAbsentErrorWasm::from(e).into()
        }
        BasicError::UnknownTransferableTypeError(e) => {
            generic_consensus_error!(UnknownTransferableTypeError, e).into()
        }
        BasicError::UnknownTradeModeError(e) => {
            generic_consensus_error!(UnknownTradeModeError, e).into()
        }
        BasicError::UnknownDocumentCreationRestrictionModeError(e) => {
            generic_consensus_error!(UnknownDocumentCreationRestrictionModeError, e).into()
        }
        BasicError::DocumentCreationNotAllowedError(e) => {
            generic_consensus_error!(DocumentCreationNotAllowedError, e).into()
        }
        BasicError::OverflowError(e) => generic_consensus_error!(OverflowError, e).into(),
        BasicError::ContestedUniqueIndexOnMutableDocumentTypeError(e) => {
            generic_consensus_error!(ContestedUniqueIndexOnMutableDocumentTypeError, e).into()
        }
        BasicError::UnsupportedFeatureError(e) => {
            generic_consensus_error!(UnsupportedFeatureError, e).into()
        }
        BasicError::DocumentFieldMaxSizeExceededError(e) => {
            generic_consensus_error!(DocumentFieldMaxSizeExceededError, e).into()
        }
        BasicError::ContestedUniqueIndexWithUniqueIndexError(e) => {
            generic_consensus_error!(ContestedUniqueIndexWithUniqueIndexError, e).into()
        }
        BasicError::ContestedDocumentsTemporarilyNotAllowedError(e) => {
            generic_consensus_error!(ContestedDocumentsTemporarilyNotAllowedError, e).into()
        }
        BasicError::WithdrawalOutputScriptNotAllowedWhenSigningWithOwnerKeyError(e) => {
            generic_consensus_error!(
                WithdrawalOutputScriptNotAllowedWhenSigningWithOwnerKeyError,
                e
            )
            .into()
        }
        BasicError::DataContractTokenConfigurationUpdateError(e) => {
            generic_consensus_error!(DataContractTokenConfigurationUpdateError, e).into()
        }
        BasicError::InvalidTokenIdError(e) => {
            generic_consensus_error!(InvalidTokenIdError, e).into()
        }

        BasicError::InvalidTokenPositionError(e) => {
            generic_consensus_error!(InvalidTokenPositionError, e).into()
        }

        BasicError::ContractHasNoTokensError(e) => {
            generic_consensus_error!(ContractHasNoTokensError, e).into()
        }

        BasicError::InvalidActionIdError(e) => {
            generic_consensus_error!(InvalidActionIdError, e).into()
        }
        BasicError::NonContiguousContractTokenPositionsError(e) => {
            generic_consensus_error!(NonContiguousContractTokenPositionsError, e).into()
        }
        BasicError::NonContiguousContractGroupPositionsError(e) => {
            generic_consensus_error!(NonContiguousContractGroupPositionsError, e).into()
        }
        BasicError::InvalidTokenBaseSupplyError(e) => {
            generic_consensus_error!(InvalidTokenBaseSupplyError, e).into()
        }
        BasicError::TokenTransferToOurselfError(e) => {
            generic_consensus_error!(TokenTransferToOurselfError, e).into()
        }
        BasicError::DestinationIdentityForTokenMintingNotSetError(e) => {
            generic_consensus_error!(DestinationIdentityForTokenMintingNotSetError, e).into()
        }
        BasicError::ChoosingTokenMintRecipientNotAllowedError(e) => {
            generic_consensus_error!(ChoosingTokenMintRecipientNotAllowedError, e).into()
        }
        BasicError::GroupActionNotAllowedOnTransitionError(e) => {
            generic_consensus_error!(GroupActionNotAllowedOnTransitionError, e).into()
        }
        BasicError::GroupPositionDoesNotExistError(e) => {
            generic_consensus_error!(GroupPositionDoesNotExistError, e).into()
        }
        BasicError::GroupExceedsMaxMembersError(e) => {
            generic_consensus_error!(GroupExceedsMaxMembersError, e).into()
        }
        BasicError::GroupMemberHasPowerOfZeroError(e) => {
            generic_consensus_error!(GroupMemberHasPowerOfZeroError, e).into()
        }
        BasicError::GroupMemberHasPowerOverLimitError(e) => {
            generic_consensus_error!(GroupMemberHasPowerOverLimitError, e).into()
        }
        BasicError::GroupTotalPowerLessThanRequiredError(e) => {
            generic_consensus_error!(GroupTotalPowerLessThanRequiredError, e).into()
        }
        BasicError::GroupNonUnilateralMemberPowerHasLessThanRequiredPowerError(e) => {
            generic_consensus_error!(
                GroupNonUnilateralMemberPowerHasLessThanRequiredPowerError,
                e
            )
            .into()
        }
        BasicError::InvalidTokenAmountError(e) => {
            generic_consensus_error!(InvalidTokenAmountError, e).into()
        }
        BasicError::InvalidTokenConfigUpdateNoChangeError(e) => {
            generic_consensus_error!(InvalidTokenConfigUpdateNoChangeError, e).into()
        }
        BasicError::InvalidTokenDistributionFunctionDivideByZeroError(e) => {
            generic_consensus_error!(InvalidTokenDistributionFunctionDivideByZeroError, e).into()
        }
        BasicError::InvalidTokenDistributionFunctionInvalidParameterError(e) => {
            generic_consensus_error!(InvalidTokenDistributionFunctionInvalidParameterError, e)
                .into()
        }
        BasicError::InvalidTokenDistributionFunctionInvalidParameterTupleError(e) => {
            generic_consensus_error!(
                InvalidTokenDistributionFunctionInvalidParameterTupleError,
                e
            )
            .into()
        }
        BasicError::InvalidTokenDistributionFunctionIncoherenceError(e) => {
            generic_consensus_error!(InvalidTokenDistributionFunctionIncoherenceError, e).into()
        }
        BasicError::InvalidTokenNoteTooBigError(e) => {
            generic_consensus_error!(InvalidTokenNoteTooBigError, e).into()
        }
        BasicError::MissingDefaultLocalizationError(e) => {
            generic_consensus_error!(MissingDefaultLocalizationError, e).into()
        }
        BasicError::UnknownGasFeesPaidByError(e) => {
            generic_consensus_error!(UnknownGasFeesPaidByError, e).into()
        }
        BasicError::UnknownDocumentActionTokenEffectError(e) => {
            generic_consensus_error!(UnknownDocumentActionTokenEffectError, e).into()
        }
        BasicError::TokenPaymentByBurningOnlyAllowedOnInternalTokenError(e) => {
            generic_consensus_error!(TokenPaymentByBurningOnlyAllowedOnInternalTokenError, e).into()
        }
        BasicError::TooManyKeywordsError(e) => {
            generic_consensus_error!(TooManyKeywordsError, e).into()
        }
        BasicError::DuplicateKeywordsError(e) => {
            generic_consensus_error!(DuplicateKeywordsError, e).into()
        }
        BasicError::InvalidKeywordLengthError(e) => {
            generic_consensus_error!(InvalidKeywordLengthError, e).into()
        }
        BasicError::InvalidDescriptionLengthError(e) => {
            generic_consensus_error!(InvalidDescriptionLengthError, e).into()
        }
        BasicError::NewTokensDestinationIdentityOptionRequiredError(e) => {
            generic_consensus_error!(NewTokensDestinationIdentityOptionRequiredError, e).into()
        }
        BasicError::InvalidKeywordCharacterError(e) => {
            generic_consensus_error!(InvalidKeywordCharacterError, e).into()
        }
        BasicError::InvalidTokenNameCharacterError(e) => {
            generic_consensus_error!(InvalidTokenNameCharacterError, e).into()
        }
        BasicError::DecimalsOverLimitError(e) => {
            generic_consensus_error!(DecimalsOverLimitError, e).into()
        }
        BasicError::InvalidTokenNameLengthError(e) => {
            generic_consensus_error!(InvalidTokenNameLengthError, e).into()
        }
        BasicError::InvalidTokenLanguageCodeError(e) => {
            generic_consensus_error!(InvalidTokenLanguageCodeError, e).into()
        }
        BasicError::MainGroupIsNotDefinedError(e) => {
            generic_consensus_error!(MainGroupIsNotDefinedError, e).into()
        }
        BasicError::GroupRequiredPowerIsInvalidError(e) => {
            generic_consensus_error!(GroupRequiredPowerIsInvalidError, e).into()
        }
        BasicError::TokenNoteOnlyAllowedWhenProposerError(e) => {
            generic_consensus_error!(TokenNoteOnlyAllowedWhenProposerError, e).into()
        }
        BasicError::InvalidTokenDistributionBlockIntervalTooShortError(e) => {
            generic_consensus_error!(InvalidTokenDistributionBlockIntervalTooShortError, e).into()
        }
        BasicError::InvalidTokenDistributionTimeIntervalTooShortError(e) => {
            generic_consensus_error!(InvalidTokenDistributionTimeIntervalTooShortError, e).into()
        }
        BasicError::InvalidTokenDistributionTimeIntervalNotMinuteAlignedError(e) => {
            generic_consensus_error!(InvalidTokenDistributionTimeIntervalNotMinuteAlignedError, e)
                .into()
        }
        BasicError::RedundantDocumentPaidForByTokenWithContractId(e) => {
            generic_consensus_error!(RedundantDocumentPaidForByTokenWithContractId, e).into()
        }
        BasicError::GroupHasTooFewMembersError(e) => {
            generic_consensus_error!(GroupHasTooFewMembersError, e).into()
        }
    }
}

fn from_signature_error(signature_error: &SignatureError) -> JsValue {
    match signature_error {
        SignatureError::MissingPublicKeyError(err) => MissingPublicKeyErrorWasm::from(err).into(),
        SignatureError::InvalidIdentityPublicKeyTypeError(err) => {
            InvalidIdentityPublicKeyTypeErrorWasm::from(err).into()
        }
        SignatureError::InvalidStateTransitionSignatureError(err) => {
            InvalidStateTransitionSignatureErrorWasm::from(err).into()
        }
        SignatureError::IdentityNotFoundError(err) => IdentityNotFoundErrorWasm::from(err).into(),
        SignatureError::InvalidSignaturePublicKeySecurityLevelError(err) => {
            InvalidSignaturePublicKeySecurityLevelErrorWasm::from(err).into()
        }
        SignatureError::PublicKeyIsDisabledError(err) => {
            PublicKeyIsDisabledErrorWasm::from(err).into()
        }
        SignatureError::PublicKeySecurityLevelNotMetError(err) => {
            PublicKeySecurityLevelNotMetErrorWasm::from(err).into()
        }
        SignatureError::WrongPublicKeyPurposeError(err) => {
            WrongPublicKeyPurposeErrorWasm::from(err).into()
        }
        SignatureError::SignatureShouldNotBePresentError(err) => {
            SignatureShouldNotBePresentErrorWasm::from(err).into()
        }
        SignatureError::BasicECDSAError(err) => BasicECDSAErrorWasm::from(err).into(),
        SignatureError::BasicBLSError(err) => BasicBLSErrorWasm::from(err).into(),
        SignatureError::InvalidSignaturePublicKeyPurposeError(err) => {
            InvalidSignaturePublicKeyPurposeErrorWasm::from(err).into()
        }
    }
}

pub fn from_consensus_error(consensus_error: DPPConsensusError) -> JsValue {
    from_consensus_error_ref(&consensus_error)
}
