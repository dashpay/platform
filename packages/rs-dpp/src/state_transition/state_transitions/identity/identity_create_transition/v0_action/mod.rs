use crate::consensus::basic::identity::IdentityAssetLockTransactionOutputNotFoundError;
use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::identifier::Identifier;
use crate::identity::{IdentityPublicKey, PartialIdentity};

use platform_value::Bytes36;
use serde::{Deserialize, Serialize};
use crate::state_transition::identity_create_transition::IdentityCreateTransition;
use crate::state_transition::identity_create_transition::v0::IdentityCreateTransitionV0;
use crate::state_transition::identity_public_key_transitions::IdentityPublicKeyInCreation;

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IdentityCreateTransitionActionV0 {
    pub public_keys: Vec<IdentityPublicKey>,
    pub initial_balance_amount: u64,
    pub identity_id: Identifier,
    pub asset_lock_outpoint: Bytes36,
}

impl From<IdentityCreateTransitionActionV0> for PartialIdentity {
    fn from(value: IdentityCreateTransitionActionV0) -> Self {
        let IdentityCreateTransitionActionV0 {
            initial_balance_amount,
            identity_id,
            ..
        } = value;
        PartialIdentity {
            id: identity_id,
            loaded_public_keys: Default::default(), //no need to load public keys
            balance: Some(initial_balance_amount),
            revision: None,
            not_found_public_keys: Default::default(),
        }
    }
}

impl From<&IdentityCreateTransitionActionV0> for PartialIdentity {
    fn from(value: &IdentityCreateTransitionActionV0) -> Self {
        let IdentityCreateTransitionActionV0 {
            initial_balance_amount,
            identity_id,
            ..
        } = value;
        PartialIdentity {
            id: *identity_id,
            loaded_public_keys: Default::default(), //no need to load public keys
            balance: Some(*initial_balance_amount),
            revision: None,
            not_found_public_keys: Default::default(),
        }
    }
}

impl IdentityCreateTransitionActionV0 {
    pub fn from(
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

    pub fn from_borrowed(
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
