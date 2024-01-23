use crate::{prelude::Identifier, BlsModule, ProtocolError};

use crate::identity::accessors::IdentityGettersV0;
use crate::identity::Identity;
use crate::identity::KeyType::ECDSA_HASH160;
use crate::prelude::AssetLockProof;

use crate::state_transition::identity_topup_transition::accessors::IdentityTopUpTransitionAccessorsV0;
use crate::state_transition::identity_topup_transition::methods::IdentityTopUpTransitionMethodsV0;

use crate::state_transition::identity_topup_transition::MinimalAssetLockValue;
use platform_version::version::PlatformVersion;

use crate::state_transition::identity_topup_transition::v0::IdentityTopUpTransitionV0;
use crate::state_transition::StateTransition;
use crate::version::FeatureVersion;

impl IdentityTopUpTransitionMethodsV0 for IdentityTopUpTransitionV0 {
    #[cfg(feature = "state-transition-signing")]
    fn try_from_identity(
        identity: Identity,
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_private_key: &[u8],
        bls: &impl BlsModule,
        _platform_version: &PlatformVersion,
        _version: Option<FeatureVersion>,
    ) -> Result<StateTransition, ProtocolError> {
        let identity_top_up_transition = IdentityTopUpTransitionV0 {
            asset_lock_proof,
            identity_id: identity.id(),
            signature: Default::default(),
        };

        let mut state_transition: StateTransition = identity_top_up_transition.into();

        state_transition.sign_by_private_key(asset_lock_proof_private_key, ECDSA_HASH160, bls)?;

        Ok(state_transition)
    }

    fn get_minimal_asset_lock_value(
        _platform_version: &PlatformVersion,
    ) -> Result<u64, ProtocolError> {
        Ok(MinimalAssetLockValue::V0 as u64)
    }
}

impl IdentityTopUpTransitionAccessorsV0 for IdentityTopUpTransitionV0 {
    /// Set identity id
    fn set_identity_id(&mut self, identity_id: Identifier) {
        self.identity_id = identity_id;
    }

    /// Returns identity id
    fn identity_id(&self) -> &Identifier {
        &self.identity_id
    }
}
