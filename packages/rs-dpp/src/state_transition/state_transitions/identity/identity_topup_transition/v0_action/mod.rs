use crate::consensus::basic::identity::IdentityAssetLockTransactionOutputNotFoundError;
use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::identifier::Identifier;
use crate::state_transition::identity_topup_transition::v0::IdentityTopUpTransitionV0;
use platform_value::Bytes36;
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IdentityTopUpTransitionActionV0 {
    pub top_up_balance_amount: u64,
    pub identity_id: Identifier,
    pub asset_lock_outpoint: Bytes36,
}

impl IdentityTopUpTransitionActionV0 {
    pub fn from(
        value: IdentityTopUpTransitionV0,
        top_up_balance_amount: u64,
    ) -> Result<Self, ConsensusError> {
        let IdentityTopUpTransitionV0 {
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
        Ok(IdentityTopUpTransitionActionV0 {
            top_up_balance_amount,
            identity_id,
            asset_lock_outpoint,
        })
    }

    pub fn from_borrowed(
        value: &IdentityTopUpTransitionV0,
        top_up_balance_amount: u64,
    ) -> Result<Self, ConsensusError> {
        let IdentityTopUpTransitionV0 {
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

        Ok(IdentityTopUpTransitionActionV0 {
            top_up_balance_amount,
            identity_id: *identity_id,
            asset_lock_outpoint,
        })
    }
}
