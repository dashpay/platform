/// transformer
pub mod transformer;
/// v0
pub mod v0;

use crate::state_transition_action::shielded::shielded_withdrawal::v0::ShieldedWithdrawalTransitionActionV0;
use derive_more::From;
use dpp::document::Document;
use dpp::fee::Credits;
use dpp::identity::core_script::CoreScript;
use dpp::prelude::UserFeeIncrease;
use dpp::withdrawal::Pooling;

/// Shielded withdrawal transition action
#[derive(Debug, Clone, From)]
pub enum ShieldedWithdrawalTransitionAction {
    /// v0
    V0(ShieldedWithdrawalTransitionActionV0),
}

impl ShieldedWithdrawalTransitionAction {
    /// Get withdrawal amount
    pub fn amount(&self) -> Credits {
        match self {
            ShieldedWithdrawalTransitionAction::V0(transition) => transition.amount,
        }
    }
    /// Get nullifiers
    pub fn nullifiers(&self) -> &Vec<[u8; 32]> {
        match self {
            ShieldedWithdrawalTransitionAction::V0(transition) => &transition.nullifiers,
        }
    }
    /// Get note commitments
    pub fn note_commitments(&self) -> &Vec<[u8; 32]> {
        match self {
            ShieldedWithdrawalTransitionAction::V0(transition) => &transition.note_commitments,
        }
    }
    /// Get encrypted notes
    pub fn encrypted_notes(&self) -> &Vec<Vec<u8>> {
        match self {
            ShieldedWithdrawalTransitionAction::V0(transition) => &transition.encrypted_notes,
        }
    }
    /// Get anchor
    pub fn anchor(&self) -> &[u8; 32] {
        match self {
            ShieldedWithdrawalTransitionAction::V0(transition) => &transition.anchor,
        }
    }
    /// Get core fee per byte
    pub fn core_fee_per_byte(&self) -> u32 {
        match self {
            ShieldedWithdrawalTransitionAction::V0(transition) => transition.core_fee_per_byte,
        }
    }
    /// Get pooling strategy
    pub fn pooling(&self) -> Pooling {
        match self {
            ShieldedWithdrawalTransitionAction::V0(transition) => transition.pooling,
        }
    }
    /// Get output script
    pub fn output_script(&self) -> &CoreScript {
        match self {
            ShieldedWithdrawalTransitionAction::V0(transition) => &transition.output_script,
        }
    }
    /// fee multiplier
    pub fn user_fee_increase(&self) -> UserFeeIncrease {
        match self {
            ShieldedWithdrawalTransitionAction::V0(transition) => transition.user_fee_increase,
        }
    }
    /// Get prepared withdrawal document
    pub fn prepared_withdrawal_document(&self) -> &Document {
        match self {
            ShieldedWithdrawalTransitionAction::V0(transition) => {
                &transition.prepared_withdrawal_document
            }
        }
    }
    /// Get prepared withdrawal document owned
    pub fn prepared_withdrawal_document_owned(self) -> Document {
        match self {
            ShieldedWithdrawalTransitionAction::V0(transition) => {
                transition.prepared_withdrawal_document
            }
        }
    }
}
