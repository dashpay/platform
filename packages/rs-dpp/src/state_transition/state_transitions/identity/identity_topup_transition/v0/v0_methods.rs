#[cfg(feature = "state-transition-signing")]
use crate::identity::accessors::IdentityGettersV0;
#[cfg(feature = "state-transition-signing")]
use crate::identity::Identity;
use crate::prelude::Identifier;
#[cfg(feature = "state-transition-signing")]
use crate::prelude::{AssetLockProof, UserFeeIncrease};
#[cfg(feature = "state-transition-signing")]
use crate::ProtocolError;
#[cfg(feature = "state-transition-signing")]
use dashcore::signer;

use crate::state_transition::identity_topup_transition::accessors::IdentityTopUpTransitionAccessorsV0;
use crate::state_transition::identity_topup_transition::methods::IdentityTopUpTransitionMethodsV0;

#[cfg(feature = "state-transition-signing")]
use crate::serialization::Signable;
#[cfg(feature = "state-transition-signing")]
use platform_version::version::PlatformVersion;

use crate::state_transition::identity_topup_transition::v0::IdentityTopUpTransitionV0;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::StateTransition;
#[cfg(feature = "state-transition-signing")]
use crate::version::FeatureVersion;

impl IdentityTopUpTransitionMethodsV0 for IdentityTopUpTransitionV0 {
    #[cfg(feature = "state-transition-signing")]
    fn try_from_identity(
        identity: &Identity,
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_private_key: &[u8],
        user_fee_increase: UserFeeIncrease,
        _platform_version: &PlatformVersion,
        _version: Option<FeatureVersion>,
    ) -> Result<StateTransition, ProtocolError> {
        let identity_top_up_transition = IdentityTopUpTransitionV0 {
            asset_lock_proof,
            identity_id: identity.id(),
            user_fee_increase,
            signature: Default::default(),
        };

        let mut state_transition: StateTransition = identity_top_up_transition.into();

        let data = state_transition.signable_bytes()?;

        let signature = signer::sign(&data, asset_lock_proof_private_key)?;
        state_transition.set_signature(signature.to_vec().into());

        Ok(state_transition)
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
