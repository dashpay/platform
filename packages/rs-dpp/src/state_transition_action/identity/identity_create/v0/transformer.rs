use crate::consensus::basic::identity::IdentityAssetLockTransactionOutputNotFoundError;
use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::state_transition::identity_create_transition::v0::IdentityCreateTransitionV0;
use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use crate::state_transition_action::identity::identity_create::v0::IdentityCreateTransitionActionV0;

impl IdentityCreateTransitionActionV0 {
    pub fn try_from(
        value: IdentityCreateTransitionV0,
        initial_balance_amount: u64,
    ) -> Result<Self, ConsensusError> {
        let IdentityCreateTransitionV0 {
            public_keys,
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
        Ok(IdentityCreateTransitionActionV0 {
            public_keys: public_keys
                .into_iter()
                .map(IdentityPublicKeyInCreation::to_identity_public_key)
                .collect(),
            initial_balance_amount,
            identity_id,
            asset_lock_outpoint,
        })
    }

    pub fn try_from_borrowed(
        value: &IdentityCreateTransitionV0,
        initial_balance_amount: u64,
    ) -> Result<Self, ConsensusError> {
        let IdentityCreateTransitionV0 {
            public_keys,
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
        Ok(IdentityCreateTransitionActionV0 {
            public_keys: public_keys
                .iter()
                .map(|key| key.clone().to_identity_public_key())
                .collect(),
            initial_balance_amount,
            identity_id: *identity_id,
            asset_lock_outpoint,
        })
    }
}
