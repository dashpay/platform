use crate::state_transition_action::identity::identity_create::v0::IdentityCreateTransitionActionV0;
use dpp::consensus::basic::decode::SerializedObjectParsingError;
use dpp::consensus::basic::identity::IdentityAssetLockTransactionOutputNotFoundError;
use dpp::consensus::basic::BasicError;
use dpp::consensus::ConsensusError;
use dpp::fee::Credits;
use dpp::platform_value::Bytes36;
use std::io;

use dpp::state_transition::state_transitions::identity::identity_create_transition::v0::IdentityCreateTransitionV0;

impl IdentityCreateTransitionActionV0 {
    /// try from
    pub fn try_from(
        value: IdentityCreateTransitionV0,
        initial_balance_amount: Credits,
    ) -> Result<Self, ConsensusError> {
        let IdentityCreateTransitionV0 {
            public_keys,
            identity_id,
            asset_lock_proof,
            ..
        } = value;

        let asset_lock_outpoint =
            asset_lock_proof
                .out_point()
                .ok_or(ConsensusError::BasicError(
                    BasicError::IdentityAssetLockTransactionOutputNotFoundError(
                        IdentityAssetLockTransactionOutputNotFoundError::new(
                            asset_lock_proof.instant_lock_output_index().unwrap(),
                        ),
                    ),
                ))?;

        let outpoint_bytes = asset_lock_outpoint
            .try_into()
            .map_err(|e: io::Error| SerializedObjectParsingError::new(e.to_string()))?;

        Ok(IdentityCreateTransitionActionV0 {
            public_keys: public_keys.into_iter().map(|a| a.into()).collect(),
            initial_balance_amount,
            identity_id,
            asset_lock_outpoint: Bytes36::new(outpoint_bytes),
        })
    }

    /// try from borrowed
    pub fn try_from_borrowed(
        value: &IdentityCreateTransitionV0,
        initial_balance_amount: Credits,
    ) -> Result<Self, ConsensusError> {
        let IdentityCreateTransitionV0 {
            public_keys,
            identity_id,
            asset_lock_proof,
            ..
        } = value;

        let asset_lock_outpoint =
            asset_lock_proof
                .out_point()
                .ok_or(ConsensusError::BasicError(
                    BasicError::IdentityAssetLockTransactionOutputNotFoundError(
                        IdentityAssetLockTransactionOutputNotFoundError::new(
                            asset_lock_proof.instant_lock_output_index().unwrap(),
                        ),
                    ),
                ))?;

        let outpoint_bytes = asset_lock_outpoint
            .try_into()
            .map_err(|e: io::Error| SerializedObjectParsingError::new(e.to_string()))?;

        Ok(IdentityCreateTransitionActionV0 {
            public_keys: public_keys.iter().map(|key| key.into()).collect(),
            initial_balance_amount,
            identity_id: *identity_id,
            asset_lock_outpoint: Bytes36::new(outpoint_bytes),
        })
    }
}
