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
use dpp::consensus::basic::data_contract::{ContestedUniqueIndexOnMutableDocumentTypeError, ContestedUniqueIndexWithUniqueIndexError, InvalidDocumentTypeRequiredSecurityLevelError, UnknownDocumentCreationRestrictionModeError, UnknownSecurityLevelError, UnknownStorageKeyRequirementsError, UnknownTradeModeError, UnknownTransferableTypeError};
use dpp::consensus::basic::document::{ContestedDocumentsTemporarilyNotAllowedError, DocumentCreationNotAllowedError, DocumentFieldMaxSizeExceededError, MaxDocumentsTransitionsExceededError, MissingPositionsInDocumentTypePropertiesError};
use dpp::consensus::basic::identity::{DataContractBoundsNotPresentError, DisablingKeyIdAlsoBeingAddedInSameTransitionError, InvalidIdentityCreditWithdrawalTransitionAmountError, InvalidIdentityUpdateTransitionDisableKeysError, InvalidIdentityUpdateTransitionEmptyError, TooManyMasterPublicKeyError, WithdrawalOutputScriptNotAllowedWhenSigningWithOwnerKeyError};
use dpp::consensus::basic::overflow_error::OverflowError;
use dpp::consensus::state::data_contract::document_type_update_error::DocumentTypeUpdateError;
use dpp::consensus::state::document::document_contest_currently_locked_error::DocumentContestCurrentlyLockedError;
use dpp::consensus::state::document::document_contest_document_with_same_id_already_present_error::DocumentContestDocumentWithSameIdAlreadyPresentError;
use dpp::consensus::state::document::document_contest_identity_already_contestant::DocumentContestIdentityAlreadyContestantError;
use dpp::consensus::state::document::document_contest_not_joinable_error::DocumentContestNotJoinableError;
use dpp::consensus::state::document::document_contest_not_paid_for_error::DocumentContestNotPaidForError;
use dpp::consensus::state::document::document_incorrect_purchase_price_error::DocumentIncorrectPurchasePriceError;
use dpp::consensus::state::document::document_not_for_sale_error::DocumentNotForSaleError;
use dpp::consensus::state::identity::identity_public_key_already_exists_for_unique_contract_bounds_error::IdentityPublicKeyAlreadyExistsForUniqueContractBoundsError;
use dpp::consensus::state::identity::master_public_key_update_error::MasterPublicKeyUpdateError;
use dpp::consensus::state::identity::missing_transfer_key_error::MissingTransferKeyError;
use dpp::consensus::state::identity::no_transfer_key_for_core_withdrawal_available_error::NoTransferKeyForCoreWithdrawalAvailableError;
use dpp::consensus::state::prefunded_specialized_balances::prefunded_specialized_balance_insufficient_error::PrefundedSpecializedBalanceInsufficientError;
use dpp::consensus::state::prefunded_specialized_balances::prefunded_specialized_balance_not_found_error::PrefundedSpecializedBalanceNotFoundError;
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
