use crate::errors::consensus::basic::{IncompatibleProtocolVersionErrorWasm, UnsupportedProtocolVersionErrorWasm};
use dpp::consensus::ConsensusError as DPPConsensusError;
use std::ops::Deref;
use wasm_bindgen::prelude::*;
use wasm_bindgen::{JsCast, JsValue};
use crate::errors::consensus::basic::identity::{DuplicatedIdentityPublicKeyIdErrorWasm, InvalidIdentityPublicKeyDataErrorWasm};

pub fn from_consensus_error(e: &DPPConsensusError) -> JsValue {
    match e {
        DPPConsensusError::JsonSchemaError(e) => {
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
            InvalidIdentityPublicKeyDataErrorWasm::from(e.clone()).into()
        }
        // DPPConsensusError::InvalidIdentityPublicKeySecurityLevelError(_) => {}
        // DPPConsensusError::DuplicatedIdentityPublicKeyError(_) => {}
        // DPPConsensusError::MissingMasterPublicKeyError(_) => {}
        // DPPConsensusError::IdentityAssetLockTransactionOutPointAlreadyExistsError(_) => {}
        // DPPConsensusError::InvalidIdentityAssetLockTransactionOutputError(_) => {}
        // DPPConsensusError::InvalidAssetLockTransactionOutputReturnSize(_) => {}
        // DPPConsensusError::IdentityAssetLockTransactionOutputNotFoundError(_) => {}
        // DPPConsensusError::InvalidIdentityAssetLockTransactionError(_) => {}
        // DPPConsensusError::InvalidInstantAssetLockProofError(_) => {}
        // DPPConsensusError::InvalidInstantAssetLockProofSignatureError(_) => {}
        // DPPConsensusError::IdentityAssetLockProofLockedTransactionMismatchError(_) => {}
        // DPPConsensusError::IdentityAssetLockTransactionIsNotFoundError(_) => {}
        // DPPConsensusError::InvalidAssetLockProofCoreChainHeightError(_) => {}
        // DPPConsensusError::InvalidAssetLockProofTransactionHeightError(_) => {}
        // DPPConsensusError::InvalidIdentityCreditWithdrawalTransitionCoreFeeError(_) => {}
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
