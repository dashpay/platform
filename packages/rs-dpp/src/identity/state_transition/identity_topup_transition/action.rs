use crate::consensus::basic::identity::IdentityAssetLockTransactionOutputNotFoundError;
use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::identifier::Identifier;
use crate::identity::state_transition::identity_topup_transition::IdentityTopUpTransition;
use platform_value::Bytes36;
use serde::{Deserialize, Serialize};

pub const IDENTITY_TOP_UP_TRANSITION_ACTION_VERSION: u32 = 0;

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IdentityTopUpTransitionAction {
    pub version: u32,
    pub top_up_balance_amount: u64,
    pub identity_id: Identifier,
    pub asset_lock_outpoint: Bytes36,
}

impl IdentityTopUpTransitionAction {
    pub fn from(
        value: IdentityTopUpTransition,
        top_up_balance_amount: u64,
    ) -> Result<Self, ConsensusError> {
        let IdentityTopUpTransition {
            identity_id,
            asset_lock_proof,
            ..
        } = value;
        let asset_lock_outpoint = asset_lock_proof
            .out_point()
            .ok_or(ConsensusError::BasicError(
                BasicError::IdentityAssetLockTransactionOutputNotFoundError(
                    IdentityAssetLockTransactionOutputNotFoundError::new(
                        asset_lock_proof.instant_lock_output_index().unwrap(),
                    ),
                ),
            ))?
            .into();
        Ok(IdentityTopUpTransitionAction {
            version: IDENTITY_TOP_UP_TRANSITION_ACTION_VERSION,
            top_up_balance_amount,
            identity_id,
            asset_lock_outpoint,
        })
    }

    pub fn from_borrowed(
        value: &IdentityTopUpTransition,
        top_up_balance_amount: u64,
    ) -> Result<Self, ConsensusError> {
        let IdentityTopUpTransition {
            identity_id,
            asset_lock_proof,
            ..
        } = value;
        let asset_lock_outpoint = asset_lock_proof
            .out_point()
            .ok_or(ConsensusError::BasicError(
                BasicError::IdentityAssetLockTransactionOutputNotFoundError(
                    IdentityAssetLockTransactionOutputNotFoundError::new(
                        asset_lock_proof.instant_lock_output_index().unwrap(),
                    ),
                ),
            ))?
            .into();

        Ok(IdentityTopUpTransitionAction {
            version: IDENTITY_TOP_UP_TRANSITION_ACTION_VERSION,
            top_up_balance_amount,
            identity_id: *identity_id,
            asset_lock_outpoint,
        })
    }
}
