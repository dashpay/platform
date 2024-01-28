use crate::errors::consensus::basic::{
    IncompatibleProtocolVersionErrorWasm, InvalidIdentifierErrorWasm, JsonSchemaErrorWasm,
    UnsupportedProtocolVersionErrorWasm, UnsupportedVersionErrorWasm,
};
use dpp::consensus::ConsensusError as DPPConsensusError;

use crate::errors::consensus::basic::identity::{
    DuplicatedIdentityPublicKeyErrorWasm, DuplicatedIdentityPublicKeyIdErrorWasm,
    IdentityAssetLockProofLockedTransactionMismatchErrorWasm,
    IdentityAssetLockTransactionIsNotFoundErrorWasm,
    IdentityAssetLockTransactionOutPointAlreadyExistsErrorWasm,
    IdentityAssetLockTransactionOutputNotFoundErrorWasm, IdentityCreditTransferToSelfErrorWasm,
    IdentityInsufficientBalanceErrorWasm, InvalidAssetLockProofCoreChainHeightErrorWasm,
    InvalidAssetLockProofTransactionHeightErrorWasm, InvalidAssetLockProofValueErrorWasm,
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
use dpp::consensus::basic::BasicError;
use dpp::consensus::basic::BasicError::{
    DuplicatedIdentityPublicKeyBasicError, DuplicatedIdentityPublicKeyIdBasicError,
    IdentityAssetLockProofLockedTransactionMismatchError,
    IdentityAssetLockTransactionIsNotFoundError,
    IdentityAssetLockTransactionOutPointAlreadyExistsError,
    IdentityAssetLockTransactionOutputNotFoundError, IncompatibleProtocolVersionError,
    IncompatibleRe2PatternError, InvalidAssetLockProofCoreChainHeightError,
    InvalidAssetLockProofTransactionHeightError, InvalidAssetLockTransactionOutputReturnSizeError,
    InvalidIdentityAssetLockProofChainLockValidationError,
    InvalidIdentityAssetLockTransactionError, InvalidIdentityAssetLockTransactionOutputError,
    InvalidIdentityCreditWithdrawalTransitionCoreFeeError,
    InvalidIdentityCreditWithdrawalTransitionOutputScriptError, InvalidIdentityPublicKeyDataError,
    InvalidIdentityPublicKeySecurityLevelError, InvalidInstantAssetLockProofError,
    InvalidInstantAssetLockProofSignatureError, JsonSchemaError, MissingMasterPublicKeyError,
    NotImplementedIdentityCreditWithdrawalTransitionPoolingError, ProtocolVersionParsingError,
    SerializedObjectParsingError, UnsupportedProtocolVersionError, UnsupportedVersionError,
};
use dpp::consensus::fee::fee_error::FeeError;
use dpp::consensus::signature::SignatureError;

// TODO(versioning): remove
// use dpp::consensus::state::data_trigger::data_trigger_error::DataTriggerError;
use dpp::consensus::state::state_error::StateError;

use wasm_bindgen::{JsError, JsValue};

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
use crate::errors::consensus::signature::{
    BasicBLSErrorWasm, BasicECDSAErrorWasm, IdentityNotFoundErrorWasm,
    SignatureShouldNotBePresentErrorWasm,
};
// use crate::errors::consensus::state::data_contract::data_trigger::{
//     DataTriggerConditionErrorWasm, DataTriggerExecutionErrorWasm,
// };
use crate::errors::consensus::state::data_contract::{
    DataContractAlreadyPresentErrorWasm, DataContractConfigUpdateErrorWasm,
    DataContractIsReadonlyErrorWasm,
};
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
        StateError::IdentityPublicKeyDisabledAtWindowViolationError(e) => {
            IdentityPublicKeyDisabledAtWindowViolationErrorWasm::from(e).into()
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
        // TODO(versioning): restore
        // StateError::DataTriggerError(data_trigger_error) => match data_trigger_error.deref() {
        //     DataTriggerError::DataTriggerConditionError(e) => {
        //         DataTriggerConditionErrorWasm::from(e).into()
        //     }
        //     DataTriggerError::DataTriggerExecutionError(e) => {
        //         DataTriggerExecutionErrorWasm::from(e).into()
        //     }
        //     DataTriggerError::DataTriggerInvalidResultError(e) => {
        //         DataTriggerInvalidResultErrorWasm::from(e).into()
        //     }
        // },
        // TODO(versioning): restore
        // StateError::DataTriggerActionError(_) => JsError::new("Data Trigger action error").into(),
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
        StateError::InvalidAssetLockProofValueError(e) => {
            InvalidAssetLockProofValueErrorWasm::from(e).into()
        }
        // TODO(versioning): restore
        _ => todo!(),
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
        BasicError::InvalidIdentityKeySignatureError(err) => {
            InvalidIdentityKeySignatureErrorWasm::from(err).into()
        }
        BasicError::InvalidDataContractIdError(err) => {
            InvalidDataContractIdErrorWasm::from(err).into()
        }
        BasicError::IdentityCreditTransferToSelfError(err) => {
            IdentityCreditTransferToSelfErrorWasm::from(err).into()
        }
        ProtocolVersionParsingError(e) => ProtocolVersionParsingErrorWasm::from(e).into(),
        SerializedObjectParsingError(e) => SerializedObjectParsingErrorWasm::from(e).into(),
        JsonSchemaError(e) => JsonSchemaErrorWasm::from(e).into(),
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
        // TODO(versioning): cover other errors
        _ => todo!(),
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
    }
}

pub fn from_consensus_error(consensus_error: DPPConsensusError) -> JsValue {
    from_consensus_error_ref(&consensus_error)
}
