use crate::errors::consensus::basic::{
    IncompatibleProtocolVersionErrorWasm, InvalidIdentifierErrorWasm, JsonSchemaErrorWasm,
    UnsupportedProtocolVersionErrorWasm,
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
    InvalidIdentityAssetLockProofChainLockValidationErrorWasm,
    InvalidIdentityAssetLockTransactionErrorWasm,
    InvalidIdentityAssetLockTransactionOutputErrorWasm,
    InvalidIdentityCreditWithdrawalTransitionCoreFeeErrorWasm,
    InvalidIdentityCreditWithdrawalTransitionOutputScriptErrorWasm,
    InvalidIdentityKeySignatureErrorWasm, InvalidIdentityPublicKeyDataErrorWasm,
    InvalidIdentityPublicKeySecurityLevelErrorWasm, InvalidInstantAssetLockProofErrorWasm,
    InvalidInstantAssetLockProofSignatureErrorWasm, MissingMasterPublicKeyErrorWasm,
    NotImplementedIdentityCreditWithdrawalTransitionPoolingErrorWasm,
};
use crate::errors::consensus::state::identity::{
    DuplicatedIdentityPublicKeyIdStateErrorWasm, DuplicatedIdentityPublicKeyStateErrorWasm,
    MissingIdentityPublicKeyIdsErrorWasm,
};
use dpp::codes::ErrorWithCode;
use dpp::consensus::basic::BasicError;
use dpp::consensus::signature::SignatureError;
use dpp::{DataTriggerActionError, StateError};
use wasm_bindgen::JsValue;

use crate::errors::consensus::basic::data_contract::{
    DataContractHaveNewUniqueIndexErrorWasm, DataContractImmutablePropertiesUpdateErrorWasm,
    DataContractInvalidIndexDefinitionUpdateErrorWasm, DataContractUniqueIndicesChangedErrorWasm,
    IncompatibleDataContractSchemaErrorWasm, InvalidDataContractIdErrorWasm,
};
use crate::errors::consensus::basic::document::{
    DuplicateDocumentTransitionsWithIdsErrorWasm, DuplicateDocumentTransitionsWithIndicesErrorWasm,
    InvalidDocumentTransitionActionErrorWasm, InvalidDocumentTransitionIdErrorWasm,
    MissingDataContractIdErrorWasm, MissingDocumentTypeErrorWasm,
};
use crate::errors::consensus::basic::state_transition::{
    InvalidStateTransitionTypeErrorWasm, MissingStateTransitionTypeErrorWasm,
    StateTransitionMaxSizeExceededErrorWasm,
};
use crate::errors::consensus::signature::IdentityNotFoundErrorWasm;
use crate::errors::consensus::state::data_contract::data_trigger::{
    DataTriggerActionConditionErrorWasm, DataTriggerActionExecutionErrorWasm,
    DataTriggerActionInvalidResultErrorWasm, DataTriggerConditionErrorWasm,
    DataTriggerExecutionErrorWasm,
};
use crate::errors::consensus::state::data_contract::DataContractAlreadyPresentErrorWasm;
use crate::errors::consensus::state::document::{
    DocumentAlreadyPresentErrorWasm, DocumentNotFoundErrorWasm, DocumentOwnerIdMismatchErrorWasm,
    DocumentTimestampWindowViolationErrorWasm, DocumentTimestampsMismatchErrorWasm,
    DuplicateUniqueIndexErrorWasm, InvalidDocumentRevisionErrorWasm,
};
use crate::errors::consensus::state::identity::{
    IdentityAlreadyExistsErrorWasm, IdentityPublicKeyDisabledAtWindowViolationErrorWasm,
    IdentityPublicKeyIsDisabledErrorWasm, IdentityPublicKeyIsReadOnlyErrorWasm,
    InvalidIdentityPublicKeyIdErrorWasm, InvalidIdentityRevisionErrorWasm,
    MaxIdentityPublicKeyLimitReachedErrorWasm,
};
use crate::errors::value_error::PlatformValueErrorWasm;
use dpp::errors::DataTriggerError;

use super::consensus::basic::data_contract::{
    DataContractMaxDepthExceedErrorWasm, DuplicateIndexErrorWasm, DuplicateIndexNameErrorWasm,
    IncompatibleRe2PatternErrorWasm, InvalidCompoundIndexErrorWasm,
    InvalidDataContractVersionErrorWasm, InvalidIndexPropertyTypeErrorWasm,
    InvalidIndexedPropertyConstraintErrorWasm, InvalidJsonSchemaRefErrorWasm,
    SystemPropertyIndexAlreadyPresentErrorWasm, UndefinedIndexPropertyErrorWasm,
    UniqueIndicesLimitReachedErrorWasm,
};
use super::consensus::basic::decode::{
    ProtocolVersionParsingErrorWasm, SerializedObjectParsingErrorWasm,
};
use super::consensus::basic::document::{
    DataContractNotPresentErrorWasm, InconsistentCompoundIndexDataErrorWasm,
    InvalidDocumentTypeErrorWasm, MissingDocumentTransitionActionErrorWasm,
    MissingDocumentTransitionTypeErrorWasm,
};
use super::consensus::basic::identity::{
    InvalidIdentityPublicKeyTypeErrorWasm, MissingPublicKeyErrorWasm,
};
use super::consensus::basic::{
    InvalidSignaturePublicKeySecurityLevelErrorWasm, InvalidStateTransitionSignatureErrorWasm,
    JsonSchemaCompilationErrorWasm, PublicKeyIsDisabledErrorWasm,
    PublicKeySecurityLevelNotMetErrorWasm, WrongPublicKeyPurposeErrorWasm,
};
use super::consensus::fee::BalanceIsNotEnoughErrorWasm;
use super::consensus::state::data_contract::data_trigger::DataTriggerInvalidResultErrorWasm;

pub fn from_consensus_error_ref(e: &DPPConsensusError) -> JsValue {
    let code = e.code();

    match e {
        DPPConsensusError::JsonSchemaError(e) => JsonSchemaErrorWasm::new(e, code).into(),
        DPPConsensusError::UnsupportedProtocolVersionError(e) => {
            UnsupportedProtocolVersionErrorWasm::from(e).into()
        }
        DPPConsensusError::IncompatibleProtocolVersionError(e) => {
            IncompatibleProtocolVersionErrorWasm::from(e).into()
        }
        DPPConsensusError::DuplicatedIdentityPublicKeyBasicIdError(e) => {
            DuplicatedIdentityPublicKeyIdErrorWasm::from(e).into()
        }
        DPPConsensusError::InvalidIdentityPublicKeyDataError(e) => {
            InvalidIdentityPublicKeyDataErrorWasm::from(e).into()
        }
        DPPConsensusError::InvalidIdentityPublicKeySecurityLevelError(e) => {
            InvalidIdentityPublicKeySecurityLevelErrorWasm::from(e).into()
        }
        DPPConsensusError::DuplicatedIdentityPublicKeyBasicError(e) => {
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
        DPPConsensusError::NotImplementedIdentityCreditWithdrawalTransitionPoolingError(e) => {
            NotImplementedIdentityCreditWithdrawalTransitionPoolingErrorWasm::from(e).into()
        }
        DPPConsensusError::IdentityInsufficientBalanceError(e) => {
            IdentityInsufficientBalanceErrorWasm::from(e).into()
        }
        DPPConsensusError::IdentityAlreadyExistsError(e) => {
            IdentityAlreadyExistsErrorWasm::from(e).into()
        }
        DPPConsensusError::SerializedObjectParsingError { parsing_error } => {
            SerializedObjectParsingErrorWasm::new(
                wasm_bindgen::JsError::new(parsing_error.to_string().as_ref()),
                code,
            )
            .into()
        }
        DPPConsensusError::ProtocolVersionParsingError(err) => {
            ProtocolVersionParsingErrorWasm::new(
                wasm_bindgen::JsError::new(err.parsing_error.to_string().as_ref()),
                code,
            )
            .into()
        }
        DPPConsensusError::IncompatibleRe2PatternError(err) => {
            IncompatibleRe2PatternErrorWasm::new(err.pattern(), err.path(), err.message(), code)
                .into()
        }
        DPPConsensusError::FeeError(e) => match e {
            dpp::consensus::fee::FeeError::BalanceIsNotEnoughError { balance, fee } => {
                BalanceIsNotEnoughErrorWasm::new(*balance, *fee, code).into()
            }
        },
        DPPConsensusError::SignatureError(e) => from_signature_error(e),
        DPPConsensusError::StateError(state_error) => from_state_error(state_error),
        DPPConsensusError::BasicError(basic_error) => from_basic_error(basic_error),
        DPPConsensusError::ValueError(value_error) => {
            PlatformValueErrorWasm::new(value_error.clone()).into()
        }
        DPPConsensusError::InvalidIdentityAssetLockProofChainLockValidationError(_) => todo!(),
        #[cfg(test)]
        DPPConsensusError::TestConsensusError(_) => todo!(),
        DPPConsensusError::DefaultError => panic!(), //not possible
        DPPConsensusError::InvalidIdentityAssetLockProofChainLockValidationError(e) => {
            InvalidIdentityAssetLockProofChainLockValidationErrorWasm::from(e).into()
        }
    }
}

pub fn from_state_error(state_error: &StateError) -> JsValue {
    let code = state_error.get_code();

    match state_error.deref() {
        StateError::DuplicatedIdentityPublicKeyIdError { duplicated_ids } => {
            DuplicatedIdentityPublicKeyIdStateErrorWasm::new(duplicated_ids.clone(), code).into()
        }
        StateError::DuplicatedIdentityPublicKeyError {
            duplicated_public_key_ids,
        } => {
            DuplicatedIdentityPublicKeyStateErrorWasm::new(duplicated_public_key_ids.clone(), code)
                .into()
        }
        StateError::DocumentAlreadyPresentError { document_id } => {
            DocumentAlreadyPresentErrorWasm::new(*document_id, code).into()
        }
        StateError::DataContractAlreadyPresentError { data_contract_id } => {
            DataContractAlreadyPresentErrorWasm::new(*data_contract_id, code).into()
        }
        StateError::DocumentNotFoundError { document_id } => {
            DocumentNotFoundErrorWasm::new(*document_id, code).into()
        }
        StateError::DocumentOwnerIdMismatchError {
            document_id,
            document_owner_id,
            existing_document_owner_id,
        } => DocumentOwnerIdMismatchErrorWasm::new(
            *document_id,
            *document_owner_id,
            *existing_document_owner_id,
            code,
        )
        .into(),
        StateError::DocumentTimestampsMismatchError { document_id } => {
            DocumentTimestampsMismatchErrorWasm::new(*document_id, code).into()
        }
        StateError::DocumentTimestampWindowViolationError {
            timestamp_name,
            document_id,
            timestamp,
            time_window_start,
            time_window_end,
        } => DocumentTimestampWindowViolationErrorWasm::new(
            timestamp_name.clone(),
            *document_id,
            *timestamp,
            *time_window_start,
            *time_window_end,
            code,
        )
        .into(),
        StateError::DuplicateUniqueIndexError {
            document_id,
            duplicating_properties,
        } => DuplicateUniqueIndexErrorWasm::new(*document_id, duplicating_properties.clone(), code)
            .into(),
        StateError::InvalidDocumentRevisionError {
            document_id,
            current_revision,
        } => InvalidDocumentRevisionErrorWasm::new(*document_id, *current_revision, code).into(),
        StateError::InvalidIdentityRevisionError {
            identity_id,
            current_revision,
        } => InvalidIdentityRevisionErrorWasm::new(*identity_id, *current_revision, code).into(),
        StateError::IdentityPublicKeyDisabledAtWindowViolationError {
            disabled_at,
            time_window_start,
            time_window_end,
        } => IdentityPublicKeyDisabledAtWindowViolationErrorWasm::new(
            *disabled_at,
            *time_window_start,
            *time_window_end,
            code,
        )
        .into(),
        StateError::IdentityPublicKeyIsReadOnlyError { public_key_index } => {
            IdentityPublicKeyIsReadOnlyErrorWasm::new(*public_key_index, code).into()
        }
        StateError::InvalidIdentityPublicKeyIdError { id } => {
            InvalidIdentityPublicKeyIdErrorWasm::new(*id, code).into()
        }
        StateError::MaxIdentityPublicKeyLimitReachedError { max_items } => {
            MaxIdentityPublicKeyLimitReachedErrorWasm::new(*max_items, code).into()
        }
        StateError::IdentityPublicKeyIsDisabledError { public_key_index } => {
            IdentityPublicKeyIsDisabledErrorWasm::new(*public_key_index, code).into()
        }
        StateError::DataTriggerError(data_trigger_error) => match data_trigger_error.deref() {
            DataTriggerError::DataTriggerConditionError {
                data_contract_id,
                document_transition_id,
                message,
                document_transition,
                owner_id,
            } => DataTriggerConditionErrorWasm::new(
                *data_contract_id,
                *document_transition_id,
                message.clone(),
                document_transition.clone(),
                *owner_id,
                code,
            )
            .into(),
            DataTriggerError::DataTriggerExecutionError {
                data_contract_id,
                document_transition_id,
                message,
                execution_error,
                document_transition,
                owner_id,
            } => DataTriggerExecutionErrorWasm::new(
                *data_contract_id,
                *document_transition_id,
                message.clone(),
                wasm_bindgen::JsError::new(execution_error.to_string().as_ref()),
                document_transition.clone(),
                *owner_id,
                code,
            )
            .into(),
            DataTriggerError::DataTriggerInvalidResultError {
                data_contract_id,
                document_transition_id,
                document_transition,
                owner_id,
            } => DataTriggerInvalidResultErrorWasm::new(
                *data_contract_id,
                *document_transition_id,
                document_transition.clone(),
                *owner_id,
                code,
            )
            .into(),
        },
        StateError::DataTriggerActionError(data_trigger_error) => {
            match data_trigger_error.deref() {
                DataTriggerActionError::DataTriggerConditionError {
                    data_contract_id,
                    document_transition_id,
                    message,
                    owner_id,
                    ..
                } => DataTriggerActionConditionErrorWasm::new(
                    *data_contract_id,
                    *document_transition_id,
                    message.clone(),
                    *owner_id,
                    code,
                )
                .into(),
                DataTriggerActionError::DataTriggerExecutionError {
                    data_contract_id,
                    document_transition_id,
                    message,
                    execution_error,
                    owner_id,
                    ..
                } => DataTriggerActionExecutionErrorWasm::new(
                    *data_contract_id,
                    *document_transition_id,
                    message.clone(),
                    wasm_bindgen::JsError::new(execution_error.to_string().as_ref()),
                    *owner_id,
                    code,
                )
                .into(),
                DataTriggerActionError::DataTriggerInvalidResultError {
                    data_contract_id,
                    document_transition_id,
                    owner_id,
                    ..
                } => DataTriggerActionInvalidResultErrorWasm::new(
                    *data_contract_id,
                    *document_transition_id,
                    *owner_id,
                    code,
                )
                .into(),
                DataTriggerActionError::ValueError(value_error) => {
                    PlatformValueErrorWasm::new(value_error.clone()).into()
                }
            }
        }
        StateError::MissingIdentityPublicKeyIdsError { ids } => {
            MissingIdentityPublicKeyIdsErrorWasm::new(ids.clone()).into()
        }
    }
}

fn from_basic_error(basic_error: &BasicError) -> JsValue {
    let code = basic_error.get_code();

    match basic_error.deref() {
        BasicError::DataContractNotPresent { data_contract_id } => {
            DataContractNotPresentErrorWasm::new(*data_contract_id, code).into()
        }
        BasicError::InvalidDataContractVersionError(err) => {
            InvalidDataContractVersionErrorWasm::new(err.expected_version(), err.version(), code)
                .into()
        }
        BasicError::DataContractMaxDepthExceedError(depth) => {
            DataContractMaxDepthExceedErrorWasm::new(*depth, code).into()
        }
        BasicError::InvalidDocumentTypeError(err) => {
            InvalidDocumentTypeErrorWasm::new(err.document_type(), err.data_contract_id(), code)
                .into()
        }
        BasicError::DuplicateIndexNameError(err) => {
            DuplicateIndexNameErrorWasm::new(err.document_type(), err.duplicate_index_name(), code)
                .into()
        }
        BasicError::InvalidJsonSchemaRefError(err) => {
            InvalidJsonSchemaRefErrorWasm::new(err.ref_error(), code).into()
        }
        BasicError::IndexError(index_error) => match index_error {
            dpp::consensus::basic::IndexError::UniqueIndicesLimitReachedError(err) => {
                UniqueIndicesLimitReachedErrorWasm::new(
                    err.document_type(),
                    err.index_limit(),
                    code,
                )
                .into()
            }
            dpp::consensus::basic::IndexError::SystemPropertyIndexAlreadyPresentError(err) => {
                SystemPropertyIndexAlreadyPresentErrorWasm::new(
                    err.document_type(),
                    err.index_definition(),
                    err.property_name(),
                    code,
                )
                .into()
            }
            dpp::consensus::basic::IndexError::UndefinedIndexPropertyError(err) => {
                UndefinedIndexPropertyErrorWasm::new(
                    err.document_type(),
                    err.index_definition(),
                    err.property_name(),
                    code,
                )
                .into()
            }
            dpp::consensus::basic::IndexError::InvalidIndexPropertyTypeError(err) => {
                InvalidIndexPropertyTypeErrorWasm::new(
                    err.document_type(),
                    err.index_definition(),
                    err.property_name(),
                    err.property_type(),
                    code,
                )
                .into()
            }
            dpp::consensus::basic::IndexError::InvalidIndexedPropertyConstraintError(err) => {
                InvalidIndexedPropertyConstraintErrorWasm::new(
                    err.document_type(),
                    err.index_definition(),
                    err.property_name(),
                    err.constraint_name(),
                    err.reason(),
                    code,
                )
                .into()
            }
            dpp::consensus::basic::IndexError::InvalidCompoundIndexError(err) => {
                InvalidCompoundIndexErrorWasm::new(
                    err.document_type(),
                    err.index_definition(),
                    code,
                )
                .into()
            }
            dpp::consensus::basic::IndexError::DuplicateIndexError(err) => {
                DuplicateIndexErrorWasm::new(err.document_type(), err.index_definition(), code)
                    .into()
            }
        },
        BasicError::JsonSchemaCompilationError(error) => {
            JsonSchemaCompilationErrorWasm::new(error.clone(), code).into()
        }
        BasicError::InconsistentCompoundIndexDataError(err) => {
            InconsistentCompoundIndexDataErrorWasm::new(
                err.index_properties(),
                err.document_type(),
                code,
            )
            .into()
        }
        BasicError::MissingDocumentTransitionTypeError => {
            MissingDocumentTransitionTypeErrorWasm::new(code).into()
        }
        BasicError::MissingDocumentTypeError => MissingDocumentTypeErrorWasm::new(code).into(),
        BasicError::MissingDocumentTransitionActionError => {
            MissingDocumentTransitionActionErrorWasm::new(code).into()
        }

        BasicError::InvalidDocumentTransitionActionError(err) => {
            InvalidDocumentTransitionActionErrorWasm::new(err.action(), code).into()
        }
        BasicError::InvalidDocumentTransitionIdError(err) => {
            InvalidDocumentTransitionIdErrorWasm::new(err.expected_id(), err.invalid_id(), code)
                .into()
        }
        BasicError::DuplicateDocumentTransitionsWithIndicesError(err) => {
            DuplicateDocumentTransitionsWithIndicesErrorWasm::new(err.references(), code).into()
        }
        BasicError::DuplicateDocumentTransitionsWithIdsError(err) => {
            DuplicateDocumentTransitionsWithIdsErrorWasm::new(err.references(), code).into()
        }
        BasicError::MissingDataContractIdError(err) => {
            MissingDataContractIdErrorWasm::new(err.raw_document_transition(), code).into()
        }
        BasicError::InvalidIdentifierError(err) => {
            InvalidIdentifierErrorWasm::new(err.identifier_name(), err.error(), code).into()
        }
        BasicError::DataContractUniqueIndicesChangedError(err) => {
            DataContractUniqueIndicesChangedErrorWasm::new(
                err.document_type(),
                err.index_name(),
                code,
            )
            .into()
        }
        BasicError::DataContractInvalidIndexDefinitionUpdateError(err) => {
            DataContractInvalidIndexDefinitionUpdateErrorWasm::new(
                err.document_type(),
                err.index_name(),
                code,
            )
            .into()
        }
        BasicError::DataContractHaveNewUniqueIndexError(err) => {
            DataContractHaveNewUniqueIndexErrorWasm::new(
                err.document_type(),
                err.index_name(),
                code,
            )
            .into()
        }
        BasicError::MissingStateTransitionTypeError => {
            MissingStateTransitionTypeErrorWasm::new(code).into()
        }
        BasicError::InvalidStateTransitionTypeError(err) => {
            InvalidStateTransitionTypeErrorWasm::new(err.transition_type(), code).into()
        }
        BasicError::StateTransitionMaxSizeExceededError(err) => {
            StateTransitionMaxSizeExceededErrorWasm::new(
                err.actual_size_kbytes(),
                err.max_size_kbytes(),
                code,
            )
            .into()
        }
        BasicError::DataContractImmutablePropertiesUpdateError(err) => {
            DataContractImmutablePropertiesUpdateErrorWasm::new(
                err.operation(),
                err.field_path(),
                code,
            )
            .into()
        }
        BasicError::IncompatibleDataContractSchemaError(err) => {
            IncompatibleDataContractSchemaErrorWasm::new(
                err.data_contract_id(),
                err.operation(),
                err.field_path(),
                err.old_schema(),
                err.new_schema(),
                code,
            )
            .into()
        }
        BasicError::InvalidIdentityKeySignatureError(err) => {
            InvalidIdentityKeySignatureErrorWasm::new(err.public_key_id(), code).into()
        }
        BasicError::InvalidDataContractIdError(err) => {
            InvalidDataContractIdErrorWasm::new(err.expected_id(), err.invalid_id(), code).into()
        }
    }
}

fn from_signature_error(signature_error: &SignatureError) -> JsValue {
    let code = signature_error.get_code();

    match signature_error.deref() {
        SignatureError::MissingPublicKeyError(err) => {
            MissingPublicKeyErrorWasm::new(err.public_key_id(), code).into()
        }
        SignatureError::InvalidIdentityPublicKeyTypeError(err) => {
            InvalidIdentityPublicKeyTypeErrorWasm::new(err.public_key_type(), code).into()
        }
        SignatureError::InvalidStateTransitionSignatureError => {
            InvalidStateTransitionSignatureErrorWasm::new(code).into()
        }
        SignatureError::IdentityNotFoundError(err) => {
            IdentityNotFoundErrorWasm::new(err.identity_id(), code).into()
        }
        SignatureError::InvalidSignaturePublicKeySecurityLevelError(err) => {
            InvalidSignaturePublicKeySecurityLevelErrorWasm::new(
                err.public_key_security_level(),
                err.allowed_key_security_levels(),
                code,
            )
            .into()
        }
        SignatureError::PublicKeyIsDisabledError(err) => {
            PublicKeyIsDisabledErrorWasm::new(err.public_key_id(), code).into()
        }
        SignatureError::PublicKeySecurityLevelNotMetError(err) => {
            PublicKeySecurityLevelNotMetErrorWasm::new(
                err.public_key_security_level(),
                err.required_security_level(),
                code,
            )
            .into()
        }
        SignatureError::WrongPublicKeyPurposeError(err) => WrongPublicKeyPurposeErrorWasm::new(
            err.public_key_purpose(),
            err.key_purpose_requirement(),
            code,
        )
        .into(),
        SignatureError::SignatureShouldNotBePresent(_) => {
            todo!()
        }
        SignatureError::BasicECDSAError(_) => {
            todo!()
        }
        SignatureError::BasicBLSError(_) => {
            todo!()
        }
    }
}

pub fn from_consensus_error(consensus_error: DPPConsensusError) -> JsValue {
    from_consensus_error_ref(&consensus_error)
}
