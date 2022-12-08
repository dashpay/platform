use crate::errors::consensus::basic::{
    IncompatibleProtocolVersionErrorWasm, InvalidIdentifierErrorWasm,
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
};
use dpp::codes::ErrorWithCode;
use dpp::consensus::basic::identity::{
    DuplicatedIdentityPublicKeyError, DuplicatedIdentityPublicKeyIdError,
};
use dpp::consensus::basic::BasicError;
use dpp::consensus::signature::SignatureError;
use dpp::StateError;
use wasm_bindgen::JsValue;

use crate::errors::consensus::basic::data_contract::{
    DataContractHaveNewUniqueIndexErrorWasm, DataContractImmutablePropertiesUpdateErrorWasm,
    DataContractInvalidIndexDefinitionUpdateErrorWasm, DataContractUniqueIndicesChangedErrorWasm,
    IncompatibleDataContractSchemaErrorWasm, InvalidDataContractIdErrorWasm,
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
    IdentityPublicKeyIsReadOnlyErrorWasm, InvalidIdentityPublicKeyIdErrorWasm,
    InvalidIdentityRevisionErrorWasm, MaxIdentityPublicKeyLimitReachedErrorWasm,
};
use dpp::errors::DataTriggerError;

use super::consensus::basic::data_contract::{
    DataContractMaxDepthErrorWasm, DuplicateIndexErrorWasm, DuplicateIndexNameErrorWasm,
    InvalidCompoundIndexErrorWasm, InvalidDataContractVersionErrorWasm,
    InvalidIndexPropertyTypeErrorWasm, InvalidIndexedPropertyConstraintErrorWasm,
    InvalidJsonSchemaRefErrorWasm, SystemPropertyIndexAlreadyPresentErrorWasm,
    UndefinedIndexPropertyErrorWasm, UniqueIndicesLimitReachedErrorWasm,
};
use super::consensus::basic::document::{
    DataContractNotPresentErrorWasm, InconsistentCompoundIndexDataErrorWasm,
    InvalidDocumentTypeErrorWasm, MissingDocumentTransitionActionErrorWasm,
    MissingDocumentTypeErrorWasm,
};
use super::consensus::basic::identity::{
    InvalidIdentityPublicKeyTypeErrorWasm, MissingPublicKeyErrorWasm,
};
use super::consensus::basic::{
    InvalidSignaturePublicKeySecurityLevelErrorWasm, InvalidStateTransitionSignatureErrorWasm,
    JsonSchemaCompilationErrorWasm, PublicKeyIsDisabledErrorWasm,
    PublicKeySecurityLevelNotMetErrorWasm, WrongPublicKeyPurposeErrorWasm,
};

pub fn from_consensus_error_ref(e: &DPPConsensusError) -> JsValue {
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
        DPPConsensusError::StateError(state_error) => from_state_error(state_error),
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
        StateError::DuplicateUniqueIndexError {
            document_id,
            duplicating_properties,
        } => DuplicateUniqueIndexErrorWasm::new(
            document_id.clone(),
            duplicating_properties.clone(),
            code,
        )
        .into(),
        StateError::InvalidDocumentRevisionError {
            document_id,
            current_revision,
        } => InvalidDocumentRevisionErrorWasm::new(document_id.clone(), *current_revision, code)
            .into(),
        StateError::InvalidIdentityRevisionError {
            identity_id,
            current_revision,
        } => InvalidIdentityRevisionErrorWasm::new(identity_id.clone(), *current_revision, code)
            .into(),
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
        // TODO: Not sure, seems like this error has been removed from the js-dpp
        // StateError::IdentityPublicKeyDisabledError { public_key_index } => {}
        StateError::DataTriggerError(data_trigger_error) => {
            match data_trigger_error.deref() {
                DataTriggerError::DataTriggerConditionError {
                    data_contract_id,
                    document_transition_id,
                    message,
                    document_transition,
                    owner_id,
                } => DataTriggerConditionErrorWasm::new(
                    data_contract_id.clone(),
                    document_transition_id.clone(),
                    message.clone(),
                    document_transition.clone(),
                    owner_id.clone(),
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
                    data_contract_id.clone(),
                    document_transition_id.clone(),
                    message.clone(),
                    wasm_bindgen::JsError::new(execution_error.to_string().as_ref()),
                    document_transition.clone(),
                    owner_id.clone(),
                    code,
                )
                .into(),
                // DataTriggerError::DataTriggerInvalidResultError { .. } => {}
                _ => "Not implemented".into(),
            }
        }
        _ => "Not implemented".into(),
    }
}

fn from_basic_error(basic_error: &Box<BasicError>) -> JsValue {
    let code = basic_error.get_code();

    match basic_error.deref() {
        BasicError::DataContractNotPresent { data_contract_id } => {
            DataContractNotPresentErrorWasm::new(data_contract_id.clone(), code).into()
        }
        BasicError::InvalidDataContractVersionError {
            expected_version,
            version,
        } => InvalidDataContractVersionErrorWasm::new(*expected_version, *version, code).into(),
        BasicError::DataContractMaxDepthExceedError(depth) => {
            DataContractMaxDepthErrorWasm::new(*depth, code).into()
        }
        BasicError::InvalidDocumentTypeError {
            document_type,
            data_contract_id,
        } => {
            InvalidDocumentTypeErrorWasm::new(document_type.clone(), data_contract_id.clone(), code)
                .into()
        }
        BasicError::DuplicateIndexNameError {
            document_type,
            duplicate_index_name,
        } => DuplicateIndexNameErrorWasm::new(
            document_type.clone(),
            duplicate_index_name.clone(),
            code,
        )
        .into(),
        BasicError::InvalidJsonSchemaRefError { ref_error } => {
            InvalidJsonSchemaRefErrorWasm::new(ref_error.clone(), code).into()
        }
        BasicError::IndexError(index_error) => match index_error {
            dpp::consensus::basic::IndexError::UniqueIndicesLimitReachedError {
                document_type,
                index_limit,
            } => UniqueIndicesLimitReachedErrorWasm::new(document_type.clone(), *index_limit, code)
                .into(),
            dpp::consensus::basic::IndexError::SystemPropertyIndexAlreadyPresentError {
                document_type,
                index_definition,
                property_name,
            } => SystemPropertyIndexAlreadyPresentErrorWasm::new(
                document_type.clone(),
                index_definition.clone(),
                property_name.clone(),
                code,
            )
            .into(),
            dpp::consensus::basic::IndexError::UndefinedIndexPropertyError {
                document_type,
                index_definition,
                property_name,
            } => UndefinedIndexPropertyErrorWasm::new(
                document_type.clone(),
                index_definition.clone(),
                property_name.clone(),
                code,
            )
            .into(),
            dpp::consensus::basic::IndexError::InvalidIndexPropertyTypeError {
                document_type,
                index_definition,
                property_name,
                property_type,
            } => InvalidIndexPropertyTypeErrorWasm::new(
                document_type.clone(),
                index_definition.clone(),
                property_name.clone(),
                property_type.clone(),
                code,
            )
            .into(),
            dpp::consensus::basic::IndexError::InvalidIndexedPropertyConstraintError {
                document_type,
                index_definition,
                property_name,
                constraint_name,
                reason,
            } => InvalidIndexedPropertyConstraintErrorWasm::new(
                document_type.clone(),
                index_definition.clone(),
                property_name.clone(),
                constraint_name.clone(),
                reason.clone(),
                code,
            )
            .into(),
            dpp::consensus::basic::IndexError::InvalidCompoundIndexError {
                document_type,
                index_definition,
            } => InvalidCompoundIndexErrorWasm::new(
                document_type.clone(),
                index_definition.clone(),
                code,
            )
            .into(),
            dpp::consensus::basic::IndexError::DuplicateIndexError {
                document_type,
                index_definition,
            } => {
                DuplicateIndexErrorWasm::new(document_type.clone(), index_definition.clone(), code)
                    .into()
            }
        },
        BasicError::JsonSchemaCompilationError(error) => {
            JsonSchemaCompilationErrorWasm::new(error.clone(), code).into()
        }
        BasicError::InconsistentCompoundIndexDataError {
            index_properties,
            document_type,
        } => InconsistentCompoundIndexDataErrorWasm::new(
            index_properties.clone(),
            document_type.clone(),
            code,
        )
        .into(),
        BasicError::MissingDocumentTypeError => MissingDocumentTypeErrorWasm::new(code).into(),
        BasicError::MissingDocumentTransitionActionError => {
            MissingDocumentTransitionActionErrorWasm::new(code).into()
        }

        BasicError::InvalidDocumentTransitionActionError { .. } => "Not implemented".into(),
        BasicError::InvalidDocumentTransitionIdError { .. } => "Not implemented".into(),
        BasicError::DuplicateDocumentTransitionsWithIdsError { .. } => "Not implemented".into(),
        BasicError::MissingDataContractIdError => "Not implemented".into(),
        BasicError::InvalidIdentifierError {
            identifier_name,
            error,
        } => InvalidIdentifierErrorWasm::new(identifier_name.clone(), error.clone(), code).into(),
        BasicError::DataContractUniqueIndicesChangedError {
            document_type,
            index_name,
        } => DataContractUniqueIndicesChangedErrorWasm::new(
            document_type.clone(),
            index_name.clone(),
            code,
        )
        .into(),
        BasicError::DataContractInvalidIndexDefinitionUpdateError {
            document_type,
            index_name,
        } => DataContractInvalidIndexDefinitionUpdateErrorWasm::new(
            document_type.clone(),
            index_name.clone(),
            code,
        )
        .into(),
        BasicError::DataContractHaveNewUniqueIndexError {
            document_type,
            index_name,
        } => DataContractHaveNewUniqueIndexErrorWasm::new(
            document_type.clone(),
            index_name.clone(),
            code,
        )
        .into(),
        BasicError::IdentityNotFoundError { identity_id } => {
            IdentityNotFoundErrorWasm::new(identity_id.clone(), code).into()
        }
        BasicError::MissingStateTransitionTypeError => {
            MissingStateTransitionTypeErrorWasm::new(code).into()
        }
        BasicError::InvalidStateTransitionTypeError { transition_type } => {
            InvalidStateTransitionTypeErrorWasm::new(*transition_type, code).into()
        }
        BasicError::StateTransitionMaxSizeExceededError {
            actual_size_kbytes,
            max_size_kbytes,
        } => StateTransitionMaxSizeExceededErrorWasm::new(
            *actual_size_kbytes,
            *max_size_kbytes,
            code,
        )
        .into(),
        BasicError::DataContractImmutablePropertiesUpdateError {
            operation,
            field_path,
        } => DataContractImmutablePropertiesUpdateErrorWasm::new(
            operation.clone(),
            field_path.clone(),
            code,
        )
        .into(),
        BasicError::IncompatibleDataContractSchemaError {
            data_contract_id,
            operation,
            field_path,
            old_schema,
            new_schema,
        } => IncompatibleDataContractSchemaErrorWasm::new(
            data_contract_id.clone(),
            operation.clone(),
            field_path.clone(),
            old_schema.clone(),
            new_schema.clone(),
            code,
        )
        .into(),
        BasicError::InvalidIdentityKeySignatureError { public_key_id } => {
            InvalidIdentityKeySignatureErrorWasm::new(*public_key_id as u32, code).into()
        }
        BasicError::InvalidDataContractIdError {
            expected_id,
            invalid_id,
        } => InvalidDataContractIdErrorWasm::new(expected_id.clone(), invalid_id.clone(), code)
            .into(),
    }
}

fn from_signature_error(signature_error: &SignatureError) -> JsValue {
    let code = signature_error.get_code();

    match signature_error.deref() {
        SignatureError::MissingPublicKeyError { public_key_id } => {
            MissingPublicKeyErrorWasm::new(*public_key_id, code).into()
        }
        SignatureError::InvalidIdentityPublicKeyTypeError { public_key_type } => {
            InvalidIdentityPublicKeyTypeErrorWasm::new(*public_key_type, code).into()
        }
        SignatureError::InvalidStateTransitionSignatureError => {
            InvalidStateTransitionSignatureErrorWasm::new(code).into()
        }
        SignatureError::IdentityNotFoundError { identity_id } => {
            IdentityNotFoundErrorWasm::new(identity_id.clone(), code).into()
        }
        SignatureError::InvalidSignaturePublicKeySecurityLevelError {
            public_key_security_level,
            required_key_security_level,
        } => InvalidSignaturePublicKeySecurityLevelErrorWasm::new(
            *public_key_security_level,
            *required_key_security_level,
            code,
        )
        .into(),
        SignatureError::PublicKeyIsDisabledError { public_key_id } => {
            PublicKeyIsDisabledErrorWasm::new(*public_key_id, code).into()
        }
        SignatureError::PublicKeySecurityLevelNotMetError {
            public_key_security_level,
            required_security_level,
        } => PublicKeySecurityLevelNotMetErrorWasm::new(
            *public_key_security_level,
            *required_security_level,
            code,
        )
        .into(),
        SignatureError::WrongPublicKeyPurposeError {
            public_key_purpose,
            key_purpose_requirement,
        } => {
            WrongPublicKeyPurposeErrorWasm::new(*public_key_purpose, *key_purpose_requirement, code)
                .into()
        }
    }
}

pub fn from_consensus_error(consensus_error: DPPConsensusError) -> JsValue {
    from_consensus_error_ref(&consensus_error)
}
