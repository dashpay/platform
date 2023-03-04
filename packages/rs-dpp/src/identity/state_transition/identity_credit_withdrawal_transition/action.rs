use crate::identifier::Identifier;
use crate::identity::core_script::CoreScript;
use crate::identity::state_transition::identity_credit_withdrawal_transition::{IdentityCreditWithdrawalTransition, Pooling};
use crate::prelude::Revision;
use serde::{Deserialize, Serialize};

pub const IDENTITY_CREDIT_WITHDRAWAL_TRANSITION_VERSION: u32 = 0;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IdentityCreditWithdrawalTransitionAction {
    pub version: u32,
    pub identity_id: Identifier,
    pub amount: u64,
    pub core_fee_per_byte: u32,
    pub pooling: Pooling,
    pub output_script: CoreScript,
    pub revision: Revision,
}

impl From<IdentityCreditWithdrawalTransition> for IdentityCreditWithdrawalTransitionAction {
    fn from(value: IdentityCreditWithdrawalTransition) -> Self {
        let IdentityCreditWithdrawalTransition { identity_id, amount, core_fee_per_byte, pooling, output_script, revision, .. } = value;
        IdentityCreditWithdrawalTransitionAction {
            version: IDENTITY_CREDIT_WITHDRAWAL_TRANSITION_VERSION,
            identity_id,
            amount,
            core_fee_per_byte,
            pooling,
            output_script,
            revision,
        }
    }
}

impl From<&IdentityCreditWithdrawalTransition> for IdentityCreditWithdrawalTransitionAction {
    fn from(value: &IdentityCreditWithdrawalTransition) -> Self {
        let IdentityCreditWithdrawalTransition { identity_id, amount, core_fee_per_byte, pooling, output_script, revision, .. } = value;
        IdentityCreditWithdrawalTransitionAction {
            version: IDENTITY_CREDIT_WITHDRAWAL_TRANSITION_VERSION,
            identity_id: *identity_id,
            amount: *amount,
            core_fee_per_byte: *core_fee_per_byte,
            pooling: *pooling,
            output_script: output_script.clone(),
            revision: *revision,
        }
    }
}