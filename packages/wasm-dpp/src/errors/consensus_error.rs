use crate::errors::consensus::basic::{
    IncompatibleProtocolVersionErrorWasm, UnsupportedProtocolVersionErrorWasm,
};
use dpp::consensus::ConsensusError as DPPConsensusError;
use std::ops::Deref;

use crate::errors::consensus::basic::identity::{
    DuplicatedIdentityPublicKeyErrorWasm, DuplicatedIdentityPublicKeyIdErrorWasm,
    IdentityAssetLockProofLockedTransactionMismatchErrorWasm,
    IdentityAssetLockTransactionIsNotFoundErrorWasm,
    IdentityAssetLockTransactionOutPointAlreadyExistsErrorWasm,
    IdentityAssetLockTransactionOutputNotFoundErrorWasm, IdentityInsufficientBalanceErrorWasm,
    InvalidAssetLockProofCoreChainHeightErrorWasm, InvalidAssetLockProofTransactionHeightErrorWasm,
    InvalidAssetLockTransactionOutputReturnSizeErrorWasm,
    InvalidIdentityAssetLockTransactionErrorWasm,
    InvalidIdentityAssetLockTransactionOutputErrorWasm,
    InvalidIdentityCreditWithdrawalTransitionCoreFeeErrorWasm,
    InvalidIdentityCreditWithdrawalTransitionOutputScriptErrorWasm,
    InvalidIdentityPublicKeyDataErrorWasm, InvalidIdentityPublicKeySecurityLevelErrorWasm,
    InvalidInstantAssetLockProofErrorWasm, InvalidInstantAssetLockProofSignatureErrorWasm,
    MissingMasterPublicKeyErrorWasm,
};
use dpp::codes::ErrorWithCode;
use dpp::consensus::basic::identity::{
    DuplicatedIdentityPublicKeyError, DuplicatedIdentityPublicKeyIdError,
};
use dpp::consensus::basic::BasicError;
use dpp::consensus::signature::SignatureError;
use dpp::StateError;
use wasm_bindgen::JsValue;

use crate::errors::consensus::state::data_contract::DataContractAlreadyPresentErrorWasm;
use crate::errors::consensus::state::document::{
    DocumentAlreadyPresentErrorWasm, DocumentNotFoundErrorWasm, DocumentOwnerIdMismatchErrorWasm,
    DocumentTimestampWindowViolationErrorWasm, DocumentTimestampsMismatchErrorWasm,
};
use crate::errors::consensus::state::identity::IdentityAlreadyExistsErrorWasm;

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
        DPPConsensusError::IdentityAlreadyExistsError(e) => {
            IdentityAlreadyExistsErrorWasm::from(e).into()
        }
        // DPPConsensusError::TestConsensusError(_) => {}
        // DPPConsensusError::SerializedObjectParsingError { .. } => {}
        // DPPConsensusError::ProtocolVersionParsingError { .. } => {}
        // DPPConsensusError::IncompatibleRe2PatternError { .. } => {}
        // DPPConsensusError::FeeError(e) => {
        //
        // }
        DPPConsensusError::SignatureError(e) => {
            from_signature_error(e);
            "Not implemented".into()
        }
        DPPConsensusError::StateError(state_error) => {
            from_state_error(state_error)
        }
        DPPConsensusError::BasicError(basic_error) => {
            from_basic_error(basic_error);
            "Not implemented".into()
        }
        // TODO: remove
        _ => e.to_string().into(),
    }
}

fn from_state_error(state_error: &Box<StateError>) -> JsValue {
    let code = state_error.get_code();

    match state_error.deref() {
        StateError::DuplicatedIdentityPublicKeyIdError { duplicated_ids } => {
            let e = DuplicatedIdentityPublicKeyIdError::new(duplicated_ids.clone());
            DuplicatedIdentityPublicKeyIdErrorWasm::from(&e).into()
        }
        StateError::DuplicatedIdentityPublicKeyError {
            duplicated_public_key_ids,
        } => {
            let e = DuplicatedIdentityPublicKeyError::new(duplicated_public_key_ids.clone());
            DuplicatedIdentityPublicKeyErrorWasm::from(&e).into()
        }
        StateError::DocumentAlreadyPresentError { document_id } => {
            DocumentAlreadyPresentErrorWasm::new(document_id.clone(), code).into()
        }
        StateError::DataContractAlreadyPresentError { data_contract_id } => {
            DataContractAlreadyPresentErrorWasm::new(data_contract_id.clone(), code).into()
        }
        StateError::DocumentNotFoundError { document_id } => {
            DocumentNotFoundErrorWasm::new(document_id.clone(), code).into()
        }
        StateError::DocumentOwnerIdMismatchError {
            document_id,
            document_owner_id,
            existing_document_owner_id,
        } => DocumentOwnerIdMismatchErrorWasm::new(
            document_id.clone(),
            document_owner_id.clone(),
            existing_document_owner_id.clone(),
            code,
        )
        .into(),
        StateError::DocumentTimestampsMismatchError { document_id } => {
            DocumentTimestampsMismatchErrorWasm::new(document_id.clone(), code).into()
        }
        StateError::DocumentTimestampWindowViolationError {
            timestamp_name,
            document_id,
            timestamp,
            time_window_start,
            time_window_end,
        } => DocumentTimestampWindowViolationErrorWasm::new(
            timestamp_name.clone(),
            document_id.clone(),
            *timestamp,
            *time_window_start,
            *time_window_end,
            code,
        )
        .into(),
        // StateError::DuplicateUniqueIndexError { .. } => {}
        // StateError::InvalidDocumentRevisionError { .. } => {}
        // StateError::InvalidIdentityRevisionError { .. } => {}
        // StateError::IdentityPublicKeyDisabledAtWindowViolationError { .. } => {}
        // StateError::IdentityPublicKeyIsReadOnlyError { .. } => {}
        // StateError::InvalidIdentityPublicKeyIdError { .. } => {}
        // StateError::MaxIdentityPublicKeyLimitReached { .. } => {}
        // StateError::IdentityPublicKeyDisabledError { .. } => {}
        // StateError::DataTriggerError(data_trigger_error) => {
        //     match data_trigger_error.deref() {
        //         DataTriggerError::DataTriggerConditionError { .. } => {}
        //         DataTriggerError::DataTriggerExecutionError { .. } => {}
        //         DataTriggerError::DataTriggerInvalidResultError { .. } => {}
        //     }
        // },
        _ => "Not implemented".into(),
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

fn from_signature_error(signature_error: &SignatureError) {
    match signature_error {
        SignatureError::MissingPublicKeyError { .. } => {}
        SignatureError::InvalidIdentityPublicKeyTypeError { .. } => {}
        SignatureError::InvalidStateTransitionSignatureError => {}
        SignatureError::IdentityNotFoundError { .. } => {}
        SignatureError::InvalidSignaturePublicKeySecurityLevelError { .. } => {}
        SignatureError::PublicKeyIsDisabledError { .. } => {}
        SignatureError::PublicKeySecurityLevelNotMetError { .. } => {}
        SignatureError::WrongPublicKeyPurposeError { .. } => {}
    }
}
