use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};

use platform_value::{BinaryData, Bytes32, IntegerReplacementType, ReplacementType, Value};
use serde::{Deserialize, Serialize};

use crate::{
    identity::KeyID,
    prelude::Identifier,
    state_transition::{StateTransitionFieldTypes, StateTransitionLike, StateTransitionType},
    BlsModule, Convertible, NonConsensusError, ProtocolError,
};

use crate::identity::accessors::IdentityGettersV0;
use crate::identity::Identity;
use crate::identity::KeyType::ECDSA_HASH160;
use crate::prelude::AssetLockProof;
use crate::serialization::{PlatformDeserializable, Signable};
use crate::state_transition::identity_topup_transition::accessors::IdentityTopUpTransitionAccessorsV0;
use crate::state_transition::identity_topup_transition::methods::IdentityTopUpTransitionMethodsV0;
use bincode::{config, Decode, Encode};

use crate::state_transition::identity_topup_transition::v0::IdentityTopUpTransitionV0;
use crate::version::FeatureVersion;

impl IdentityTopUpTransitionMethodsV0 for IdentityTopUpTransitionV0 {
    #[cfg(feature = "state-transition-signing")]
    fn try_from_identity(
        identity: Identity,
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_private_key: &[u8],
        bls: &impl BlsModule,
        version: FeatureVersion,
    ) -> Result<Self, ProtocolError> {
        let mut identity_top_up_transition = IdentityTopUpTransitionV0 {
            asset_lock_proof,
            identity_id: identity.id(),
            signature: Default::default(),
        };

        identity_top_up_transition.sign_by_private_key(
            asset_lock_proof_private_key,
            ECDSA_HASH160,
            bls,
        )?;

        Ok(identity_top_up_transition)
    }
}

impl IdentityTopUpTransitionAccessorsV0 for IdentityTopUpTransitionV0 {
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
