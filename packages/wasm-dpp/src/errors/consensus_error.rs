use std::ops::Deref;
use crate::errors::consensus::basic::{
    IncompatibleProtocolVersionErrorWasm, UnsupportedProtocolVersionErrorWasm,
};
use dpp::consensus::ConsensusError as DPPConsensusError;

use crate::errors::consensus::basic::identity::{DuplicatedIdentityPublicKeyErrorWasm, DuplicatedIdentityPublicKeyIdErrorWasm, IdentityAssetLockProofLockedTransactionMismatchErrorWasm, IdentityAssetLockTransactionIsNotFoundErrorWasm, IdentityAssetLockTransactionOutPointAlreadyExistsErrorWasm, IdentityAssetLockTransactionOutputNotFoundErrorWasm, IdentityInsufficientBalanceErrorWasm, InvalidAssetLockProofCoreChainHeightErrorWasm, InvalidAssetLockProofTransactionHeightErrorWasm, InvalidAssetLockTransactionOutputReturnSizeErrorWasm, InvalidIdentityAssetLockTransactionErrorWasm, InvalidIdentityAssetLockTransactionOutputErrorWasm, InvalidIdentityCreditWithdrawalTransitionCoreFeeErrorWasm, InvalidIdentityCreditWithdrawalTransitionOutputScriptErrorWasm, InvalidIdentityPublicKeyDataErrorWasm, InvalidIdentityPublicKeySecurityLevelErrorWasm, InvalidInstantAssetLockProofErrorWasm, InvalidInstantAssetLockProofSignatureErrorWasm, MissingMasterPublicKeyErrorWasm};
use dpp::consensus::basic::identity::InvalidInstantAssetLockProofSignatureError;
use wasm_bindgen::JsValue;
use dpp::{DataTriggerError, StateError};
use dpp::consensus::basic::BasicError;

pub fn from_consensus_error(e: &DPPConsensusError) -> JsValue {
    match e {
        DPPConsensusError::JsonSchemaError(e) => {
            // TODO: rework JSONSchema error
            e.to_string().into()
        }
        DPPConsensusError::UnsupportedProtocolVersionError(e) => {
            UnsupportedProtocolVersionErrorWasm::from(e).into()
        }
        DPPConsensusError::IncompatibleProtocolVersionError(e) => {
            IncompatibleProtocolVersionErrorWasm::from(e).into()
        }
        DPPConsensusError::DuplicatedIdentityPublicKeyIdError(e) => {
            DuplicatedIdentityPublicKeyIdErrorWasm::from(e).into()
        }
        DPPConsensusError::InvalidIdentityPublicKeyDataError(e) => {
            InvalidIdentityPublicKeyDataErrorWasm::from(e).into()
        }
        DPPConsensusError::InvalidIdentityPublicKeySecurityLevelError(e) => {
            InvalidIdentityPublicKeySecurityLevelErrorWasm::from(e).into()
        }
        DPPConsensusError::DuplicatedIdentityPublicKeyError(e) => {
            DuplicatedIdentityPublicKeyErrorWasm::from(e).into()
        }
        DPPConsensusError::MissingMasterPublicKeyError(e) => {
            MissingMasterPublicKeyErrorWasm::from(e).into()
        }
        DPPConsensusError::IdentityAssetLockTransactionOutPointAlreadyExistsError(e) => {
            IdentityAssetLockTransactionOutPointAlreadyExistsErrorWasm::from(e).into()
        }
        DPPConsensusError::InvalidIdentityAssetLockTransactionOutputError(e) => {
            InvalidIdentityAssetLockTransactionOutputErrorWasm::from(e).into()
        }
        DPPConsensusError::InvalidAssetLockTransactionOutputReturnSize(e) => {
            InvalidAssetLockTransactionOutputReturnSizeErrorWasm::from(e).into()
        }
        DPPConsensusError::IdentityAssetLockTransactionOutputNotFoundError(e) => {
            IdentityAssetLockTransactionOutputNotFoundErrorWasm::from(e).into()
        }
        DPPConsensusError::InvalidIdentityAssetLockTransactionError(e) => {
            InvalidIdentityAssetLockTransactionErrorWasm::from(e).into()
        }
        DPPConsensusError::InvalidInstantAssetLockProofError(e) => {
            InvalidInstantAssetLockProofErrorWasm::from(e).into()
        }
        DPPConsensusError::InvalidInstantAssetLockProofSignatureError(e) => {
            InvalidInstantAssetLockProofSignatureErrorWasm::from(e).into()
        }
        DPPConsensusError::IdentityAssetLockProofLockedTransactionMismatchError(e) => {
            IdentityAssetLockProofLockedTransactionMismatchErrorWasm::from(e).into()
        }
        DPPConsensusError::IdentityAssetLockTransactionIsNotFoundError(e) => {
            IdentityAssetLockTransactionIsNotFoundErrorWasm::from(e).into()
        }
        DPPConsensusError::InvalidAssetLockProofCoreChainHeightError(e) => {
            InvalidAssetLockProofCoreChainHeightErrorWasm::from(e).into()
        }
        DPPConsensusError::InvalidAssetLockProofTransactionHeightError(e) => {
            InvalidAssetLockProofTransactionHeightErrorWasm::from(e).into()
        }
        DPPConsensusError::InvalidIdentityCreditWithdrawalTransitionCoreFeeError(e) => {
            InvalidIdentityCreditWithdrawalTransitionCoreFeeErrorWasm::from(e).into()
        }
        DPPConsensusError::InvalidIdentityCreditWithdrawalTransitionOutputScriptError(e) => {
            InvalidIdentityCreditWithdrawalTransitionOutputScriptErrorWasm::from(e).into()
        }
        DPPConsensusError::IdentityInsufficientBalanceError(e) => {
            IdentityInsufficientBalanceErrorWasm::from(e).into()
        }
        // DPPConsensusError::IdentityAlreadyExistsError(e) => {
        //
        // }
        // DPPConsensusError::SignatureError(_) => {}
        // DPPConsensusError::FeeError(_) => {}
        // DPPConsensusError::TestConsensusError(_) => {}
        // DPPConsensusError::SerializedObjectParsingError { .. } => {}
        // DPPConsensusError::ProtocolVersionParsingError { .. } => {}
        // DPPConsensusError::IncompatibleRe2PatternError { .. } => {}
        DPPConsensusError::StateError(state_error) => {
            from_state_error(state_error);
            "Not implemented".into()
        }
        DPPConsensusError::BasicError(basic_error) => {
            from_basic_error(basic_error);
            "Not implemented".into()
        }
        // TODO: remove
        _ => e.to_string().into(),
    }
}

fn from_state_error(state_error: &Box<StateError>) {
    match state_error.deref() {
        StateError::DataTriggerError(data_trigger_error) => {
            match data_trigger_error.deref() {
                DataTriggerError::DataTriggerConditionError { .. } => {}
                DataTriggerError::DataTriggerExecutionError { .. } => {}
                DataTriggerError::DataTriggerInvalidResultError { .. } => {}
            }
        },
        StateError::DuplicatedIdentityPublicKeyIdError { .. } => {}
        StateError::DuplicatedIdentityPublicKeyError { .. } => {}
        StateError::DocumentAlreadyPresentError { .. } => {}
        StateError::DataContractAlreadyPresentError { .. } => {}
        StateError::DocumentNotFoundError { .. } => {}
        StateError::DocumentOwnerMismatchError { .. } => {}
        StateError::DocumentTimestampMismatchError { .. } => {}
        StateError::DocumentTimestampWindowViolationError { .. } => {}
        StateError::DuplicateUniqueIndexError { .. } => {}
        StateError::InvalidDocumentRevisionError { .. } => {}
        StateError::InvalidIdentityRevisionError { .. } => {}
        StateError::IdentityPublicKeyDisabledAtWindowViolationError { .. } => {}
        StateError::IdentityPublicKeyIsReadOnlyError { .. } => {}
        StateError::InvalidIdentityPublicKeyIdError { .. } => {}
        StateError::MaxIdentityPublicKeyLimitReached { .. } => {}
        StateError::IdentityPublicKeyDisabledError { .. } => {}
    }
}

fn from_basic_error(basic_error: &Box<BasicError>) {
    match basic_error.deref() {
        BasicError::DataContractNotPresent { .. } => {}
        BasicError::InvalidDataContractVersionError { .. } => {}
        BasicError::DataContractMaxDepthExceedError(_) => {}
        BasicError::InvalidDocumentTypeError { .. } => {}
        BasicError::DuplicateIndexNameError { .. } => {}
        BasicError::InvalidJsonSchemaRefError { .. } => {}
        BasicError::IndexError(_) => {}
        BasicError::JsonSchemaCompilationError(_) => {}
        BasicError::InconsistentCompoundIndexDataError { .. } => {}
        BasicError::MissingDocumentTypeError => {}
        BasicError::MissingDocumentTransitionActionError => {}
        BasicError::InvalidDocumentTransitionActionError { .. } => {}
        BasicError::InvalidDocumentTransitionIdError { .. } => {}
        BasicError::DuplicateDocumentTransitionsWithIdsError { .. } => {}
        BasicError::MissingDataContractIdError => {}
        BasicError::InvalidIdentifierError { .. } => {}
        BasicError::DataContractUniqueIndicesChangedError { .. } => {}
        BasicError::DataContractInvalidIndexDefinitionUpdateError { .. } => {}
        BasicError::DataContractHaveNewUniqueIndexError { .. } => {}
        BasicError::IdentityNotFoundError { .. } => {}
        BasicError::MissingStateTransitionTypeError => {}
        BasicError::InvalidStateTransitionTypeError { .. } => {}
        BasicError::StateTransitionMaxSizeExceededError { .. } => {}
        BasicError::DataContractImmutablePropertiesUpdateError { .. } => {}
        BasicError::IncompatibleDataContractSchemaError { .. } => {}
        BasicError::InvalidIdentityPublicKeySignatureError { .. } => {}
        BasicError::InvalidDataContractId { .. } => {}
    }
}