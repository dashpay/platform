use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};

use platform_value::{BinaryData, Bytes32, IntegerReplacementType, ReplacementType, Value};
use serde::{Deserialize, Serialize};

use crate::{
    identity::KeyID,
    prelude::Identifier,
    state_transition::{StateTransitionFieldTypes, StateTransitionLike, StateTransitionType},
    BlsModule, Convertible, NonConsensusError, ProtocolError,
};

use crate::identity::Identity;
use crate::identity::KeyType::ECDSA_HASH160;
use crate::prelude::AssetLockProof;
use crate::serialization::{PlatformDeserializable, Signable};
use bincode::{config, Decode, Encode};

use crate::state_transition::identity_topup_transition::v0::IdentityTopUpTransitionV0;
use crate::version::FeatureVersion;

pub trait IdentityTopUpTransitionV0Methods {
    fn try_from_identity(
        identity: Identity,
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_private_key: &[u8],
        bls: &impl BlsModule,
        version: FeatureVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized;

    /// Get State Transition type
    fn get_type() -> StateTransitionType {
        StateTransitionType::IdentityTopUp
    }
    /// Set asset lock
    fn set_asset_lock_proof(&mut self, asset_lock_proof: AssetLockProof);
    /// Get asset lock proof
    fn asset_lock_proof(&self) -> &AssetLockProof;
    /// Set identity id
    fn set_identity_id(&mut self, identity_id: Identifier);
    /// Returns identity id
    fn identity_id(&self) -> &Identifier;
}

impl IdentityTopUpTransitionV0Methods for IdentityTopUpTransitionV0 {
    fn try_from_identity(
        identity: Identity,
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_private_key: &[u8],
        bls: &impl BlsModule,
        version: FeatureVersion,
    ) -> Result<Self, ProtocolError> {
        let mut identity_top_up_transition = IdentityTopUpTransitionV0 {
            asset_lock_proof,
            identity_id: identity.id,
            signature: Default::default(),
        };

        identity_top_up_transition.sign_by_private_key(
            asset_lock_proof_private_key,
            ECDSA_HASH160,
            bls,
        )?;

        Ok(identity_top_up_transition)
    }

    /// Set asset lock
    fn set_asset_lock_proof(&mut self, asset_lock_proof: AssetLockProof) {
        self.asset_lock_proof = asset_lock_proof;
    }

    /// Get asset lock proof
    fn asset_lock_proof(&self) -> &AssetLockProof {
        &self.asset_lock_proof
    }

    /// Set identity id
    fn set_identity_id(&mut self, identity_id: Identifier) {
        self.identity_id = identity_id;
    }

    /// Returns identity id
    fn identity_id(&self) -> &Identifier {
        &self.identity_id
    }
}
