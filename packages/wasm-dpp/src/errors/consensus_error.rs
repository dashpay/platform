use crate::errors::consensus::basic::{
    IncompatibleProtocolVersionErrorWasm, UnsupportedProtocolVersionErrorWasm,
};
use dpp::consensus::ConsensusError as DPPConsensusError;

use crate::errors::consensus::basic::identity::{DuplicatedIdentityPublicKeyErrorWasm, DuplicatedIdentityPublicKeyIdErrorWasm, IdentityAssetLockProofLockedTransactionMismatchErrorWasm, IdentityAssetLockTransactionIsNotFoundErrorWasm, IdentityAssetLockTransactionOutPointAlreadyExistsErrorWasm, IdentityAssetLockTransactionOutputNotFoundErrorWasm, InvalidAssetLockProofCoreChainHeightErrorWasm, InvalidAssetLockProofTransactionHeightErrorWasm, InvalidAssetLockTransactionOutputReturnSizeErrorWasm, InvalidIdentityAssetLockTransactionErrorWasm, InvalidIdentityAssetLockTransactionOutputErrorWasm, InvalidIdentityCreditWithdrawalTransitionCoreFeeErrorWasm, InvalidIdentityPublicKeyDataErrorWasm, InvalidIdentityPublicKeySecurityLevelErrorWasm, InvalidInstantAssetLockProofErrorWasm, InvalidInstantAssetLockProofSignatureErrorWasm, MissingMasterPublicKeyErrorWasm};
use dpp::consensus::basic::identity::InvalidInstantAssetLockProofSignatureError;
use wasm_bindgen::JsValue;

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
        // DPPConsensusError::InvalidIdentityCreditWithdrawalTransitionOutputScriptError(_) => {}
        // DPPConsensusError::StateError(_) => {}
        // DPPConsensusError::BasicError(_) => {}
        // DPPConsensusError::SerializedObjectParsingError { .. } => {}
        // DPPConsensusError::ProtocolVersionParsingError { .. } => {}
        // DPPConsensusError::IncompatibleRe2PatternError { .. } => {}
        // DPPConsensusError::IdentityInsufficientBalanceError(_) => {}
        // DPPConsensusError::IdentityAlreadyExistsError(_) => {}
        // DPPConsensusError::SignatureError(_) => {}
        // DPPConsensusError::FeeError(_) => {}
        // DPPConsensusError::TestConsensusError(_) => {}
        // TODO: remove
        _ => e.to_string().into(),
    }
}
