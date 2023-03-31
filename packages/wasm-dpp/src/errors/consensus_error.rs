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
};
use dpp::consensus::basic::BasicError;
use dpp::consensus::basic::BasicError::{
    DuplicatedIdentityPublicKeyBasicError, DuplicatedIdentityPublicKeyIdBasicError,
    IdentityAssetLockProofLockedTransactionMismatchError,
    IdentityAssetLockTransactionIsNotFoundError,
    IdentityAssetLockTransactionOutPointAlreadyExistsError,
    IdentityAssetLockTransactionOutputNotFoundError, IncompatibleProtocolVersionError,
    IncompatibleRe2PatternError, InvalidAssetLockProofCoreChainHeightError,
    InvalidAssetLockProofTransactionHeightError, InvalidAssetLockTransactionOutputReturnSizeError,
    InvalidIdentityAssetLockTransactionError, InvalidIdentityAssetLockTransactionOutputError,
    InvalidIdentityCreditWithdrawalTransitionCoreFeeError,
    InvalidIdentityCreditWithdrawalTransitionOutputScriptError, InvalidIdentityPublicKeyDataError,
    InvalidIdentityPublicKeySecurityLevelError, InvalidInstantAssetLockProofError,
    InvalidInstantAssetLockProofSignatureError, JsonSchemaError, MissingMasterPublicKeyError,
    NotImplementedIdentityCreditWithdrawalTransitionPoolingError, ProtocolVersionParsingError,
    SerializedObjectParsingError, UnsupportedProtocolVersionError,
};
use dpp::consensus::fee::fee_error::FeeError;
use dpp::consensus::signature::signature_error::SignatureError;
use dpp::consensus::state::data_trigger::data_trigger_error::DataTriggerError;
use dpp::consensus::state::state_error::StateError;
use dpp::errors::consensus::codes::ErrorWithCode;
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
    DataTriggerConditionErrorWasm, DataTriggerExecutionErrorWasm,
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
        DPPConsensusError::FeeError(e) => match e {
            FeeError::BalanceIsNotEnoughError(e) => {
                BalanceIsNotEnoughErrorWasm::new(e.balance(), e.fee(), code).into()
            }
        },
        DPPConsensusError::SignatureError(e) => from_signature_error(e),
        DPPConsensusError::StateError(state_error) => from_state_error(state_error),
        DPPConsensusError::BasicError(basic_error) => from_basic_error(basic_error),
    }
}

pub fn from_state_error(state_error: &StateError) -> JsValue {
    let code = state_error.code();

    match state_error.deref() {
        StateError::DuplicatedIdentityPublicKeyIdStateError(e) => {
            DuplicatedIdentityPublicKeyIdStateErrorWasm::new(e.duplicated_ids().clone(), code)
                .into()
        }
        StateError::DuplicatedIdentityPublicKeyStateError(e) => {
            DuplicatedIdentityPublicKeyStateErrorWasm::new(
                e.duplicated_public_key_ids().clone(),
                code,
            )
            .into()
        }
        StateError::DocumentAlreadyPresentError(e) => {
            DocumentAlreadyPresentErrorWasm::new(*e.document_id(), code).into()
        }
        StateError::DataContractAlreadyPresentError(e) => {
            DataContractAlreadyPresentErrorWasm::new(*e.data_contract_id(), code).into()
        }
        StateError::DocumentNotFoundError(e) => {
            DocumentNotFoundErrorWasm::new(*e.document_id(), code).into()
        }
        StateError::DocumentOwnerIdMismatchError(e) => DocumentOwnerIdMismatchErrorWasm::new(
            *e.document_id(),
            *e.document_owner_id(),
            *e.existing_document_owner_id(),
            code,
        )
        .into(),
        StateError::DocumentTimestampsMismatchError(e) => {
            DocumentTimestampsMismatchErrorWasm::new(*e.document_id(), code).into()
        }
        StateError::DocumentTimestampWindowViolationError(e) => {
            DocumentTimestampWindowViolationErrorWasm::new(
                e.timestamp_name().clone(),
                *e.document_id(),
                *e.timestamp(),
                *e.time_window_start(),
                *e.time_window_end(),
                code,
            )
            .into()
        }
        StateError::DuplicateUniqueIndexError(e) => DuplicateUniqueIndexErrorWasm::new(
            *e.document_id(),
            e.duplicating_properties().clone(),
            code,
        )
        .into(),
        StateError::InvalidDocumentRevisionError(e) => {
            InvalidDocumentRevisionErrorWasm::new(*e.document_id(), *e.current_revision(), code)
                .into()
        }
        StateError::InvalidIdentityRevisionError(e) => {
            InvalidIdentityRevisionErrorWasm::new(*e.identity_id(), *e.current_revision(), code)
                .into()
        }
        StateError::IdentityPublicKeyDisabledAtWindowViolationError(e) => {
            IdentityPublicKeyDisabledAtWindowViolationErrorWasm::new(
                e.disabled_at(),
                e.time_window_start(),
                e.time_window_end(),
                code,
            )
            .into()
        }
        StateError::IdentityPublicKeyIsReadOnlyError(e) => {
            IdentityPublicKeyIsReadOnlyErrorWasm::new(e.public_key_index(), code).into()
        }
        StateError::InvalidIdentityPublicKeyIdError(e) => {
            InvalidIdentityPublicKeyIdErrorWasm::new(e.id(), code).into()
        }
        StateError::MaxIdentityPublicKeyLimitReachedError(e) => {
            MaxIdentityPublicKeyLimitReachedErrorWasm::new(e.max_items(), code).into()
        }
        StateError::IdentityPublicKeyIsDisabledError(e) => {
            IdentityPublicKeyIsDisabledErrorWasm::new(e.public_key_index(), code).into()
        }
        StateError::DataTriggerError(data_trigger_error) => match data_trigger_error.deref() {
            DataTriggerError::DataTriggerConditionError(e) => DataTriggerConditionErrorWasm::new(
                *e.data_contract_id(),
                *e.document_id(),
                e.message().to_string(),
                code,
            )
            .into(),
            DataTriggerError::DataTriggerExecutionError(e) => DataTriggerExecutionErrorWasm::new(
                *e.data_contract_id(),
                *e.document_id(),
                e.message().to_string(),
                code,
            )
            .into(),
            DataTriggerError::DataTriggerInvalidResultError(e) => {
                DataTriggerInvalidResultErrorWasm::new(
                    *e.data_contract_id(),
                    *e.document_transition_id(),
                    code,
                )
                .into()
            }
        },
        StateError::IdentityAlreadyExistsError(e) => {
            let wasm_error: IdentityAlreadyExistsErrorWasm = e.into();
            wasm_error.into()
        }
        StateError::IdentityInsufficientBalanceError(e) => {
            let wasm_error: IdentityInsufficientBalanceErrorWasm = e.into();
            wasm_error.into()
        }
    }
}

// TODO: Move as From/TryInto trait implementation to wasm error modules
fn from_basic_error(basic_error: &BasicError) -> JsValue {
    let code = basic_error.code();

    match basic_error.deref() {
        BasicError::ValueError(value_error) => {
            PlatformValueErrorWasm::new(value_error.clone()).into()
        }
        BasicError::DataContractNotPresentError(err) => {
            DataContractNotPresentErrorWasm::new(err.data_contract_id(), code).into()
        }
        BasicError::InvalidDataContractVersionError(err) => {
            InvalidDataContractVersionErrorWasm::new(err.expected_version(), err.version(), code)
                .into()
        }
        BasicError::DataContractMaxDepthExceedError(err) => {
            DataContractMaxDepthExceedErrorWasm::new(err.max_depth(), code).into()
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
            InvalidJsonSchemaRefErrorWasm::new(err.error_message(), code).into()
        }
        BasicError::UniqueIndicesLimitReachedError(err) => UniqueIndicesLimitReachedErrorWasm::new(
            err.document_type().to_string(),
            err.index_limit(),
            code,
        )
        .into(),
        BasicError::SystemPropertyIndexAlreadyPresentError(err) => {
            SystemPropertyIndexAlreadyPresentErrorWasm::new(
                err.document_type().to_string(),
                err.index_name().to_string(),
                err.property_name().to_string(),
                code,
            )
            .into()
        }
        BasicError::UndefinedIndexPropertyError(err) => UndefinedIndexPropertyErrorWasm::new(
            err.document_type().to_string(),
            err.index_definition().to_string(),
            err.property_name().to_string(),
            code,
        )
        .into(),
        BasicError::InvalidIndexPropertyTypeError(err) => InvalidIndexPropertyTypeErrorWasm::new(
            err.document_type().to_string(),
            err.index_name().to_string(),
            err.property_name().to_string(),
            err.property_type().to_string(),
            code,
        )
        .into(),
        BasicError::InvalidIndexedPropertyConstraintError(err) => {
            InvalidIndexedPropertyConstraintErrorWasm::new(
                err.document_type().to_string(),
                err.index_name().to_string(),
                err.property_name().to_string(),
                err.constraint_name().to_string(),
                err.reason().to_string(),
                code,
            )
            .into()
        }
        BasicError::InvalidCompoundIndexError(err) => InvalidCompoundIndexErrorWasm::new(
            err.document_type().to_string(),
            err.index_name().to_string(),
            code,
        )
        .into(),
        BasicError::DuplicateIndexError(err) => DuplicateIndexErrorWasm::new(
            err.document_type().to_string(),
            err.index_name().to_string(),
            code,
        )
        .into(),
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
            InvalidDocumentTransitionActionErrorWasm::new(err.action().to_string(), code).into()
        }
        BasicError::InvalidDocumentTransitionIdError(err) => {
            InvalidDocumentTransitionIdErrorWasm::new(err.expected_id(), err.invalid_id(), code)
                .into()
        }
        BasicError::DuplicateDocumentTransitionsWithIndicesError(err) => {
            DuplicateDocumentTransitionsWithIndicesErrorWasm::new(err.references().to_owned(), code)
                .into()
        }
        BasicError::DuplicateDocumentTransitionsWithIdsError(err) => {
            DuplicateDocumentTransitionsWithIdsErrorWasm::new(err.references().to_owned(), code)
                .into()
        }
        BasicError::MissingDataContractIdBasicError => {
            MissingDataContractIdErrorWasm::new(code).into()
        }
        BasicError::InvalidIdentifierError(err) => InvalidIdentifierErrorWasm::new(
            err.identifier_name().to_string(),
            err.error_message().to_string(),
            code,
        )
        .into(),
        BasicError::DataContractUniqueIndicesChangedError(err) => {
            DataContractUniqueIndicesChangedErrorWasm::new(
                err.document_type().to_string(),
                err.index_name().to_string(),
                code,
            )
            .into()
        }
        BasicError::DataContractInvalidIndexDefinitionUpdateError(err) => {
            DataContractInvalidIndexDefinitionUpdateErrorWasm::new(
                err.document_type().to_string(),
                err.index_name().to_string(),
                code,
            )
            .into()
        }
        BasicError::DataContractHaveNewUniqueIndexError(err) => {
            DataContractHaveNewUniqueIndexErrorWasm::new(
                err.document_type().to_string(),
                err.index_name().to_string(),
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
        ProtocolVersionParsingError(e) => ProtocolVersionParsingErrorWasm::new(
            wasm_bindgen::JsError::new(e.parsing_error.to_string().as_ref()),
            code,
        )
        .into(),
        SerializedObjectParsingError(e) => SerializedObjectParsingErrorWasm::new(
            wasm_bindgen::JsError::new(e.parsing_error().to_string().as_ref()),
            code,
        )
        .into(),
        JsonSchemaError(e) => JsonSchemaErrorWasm::new(e, code).into(),
        UnsupportedProtocolVersionError(e) => UnsupportedProtocolVersionErrorWasm::from(e).into(),
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
        IdentityAssetLockTransactionOutPointAlreadyExistsError(e) => {
            IdentityAssetLockTransactionOutPointAlreadyExistsErrorWasm::from(e).into()
        }
        InvalidIdentityAssetLockTransactionOutputError(e) => {
            InvalidIdentityAssetLockTransactionOutputErrorWasm::from(e).into()
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
        InvalidIdentityCreditWithdrawalTransitionCoreFeeError(e) => {
            InvalidIdentityCreditWithdrawalTransitionCoreFeeErrorWasm::from(e).into()
        }
        InvalidIdentityCreditWithdrawalTransitionOutputScriptError(e) => {
            InvalidIdentityCreditWithdrawalTransitionOutputScriptErrorWasm::from(e).into()
        }
        NotImplementedIdentityCreditWithdrawalTransitionPoolingError(e) => {
            NotImplementedIdentityCreditWithdrawalTransitionPoolingErrorWasm::from(e).into()
        }
        IncompatibleRe2PatternError(err) => {
            IncompatibleRe2PatternErrorWasm::new(err.pattern(), err.path(), err.message(), code)
                .into()
        }
    }
}

fn from_signature_error(signature_error: &SignatureError) -> JsValue {
    let code = signature_error.code();

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
                err.required_key_security_level(),
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
    }
}

pub fn from_consensus_error(consensus_error: DPPConsensusError) -> JsValue {
    from_consensus_error_ref(&consensus_error)
}
