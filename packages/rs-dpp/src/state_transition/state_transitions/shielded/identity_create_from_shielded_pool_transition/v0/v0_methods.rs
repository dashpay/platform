#[cfg(feature = "state-transition-signing")]
use crate::address_funds::PlatformAddress;
#[cfg(feature = "state-transition-signing")]
use crate::shielded::SerializedAction;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::identity_create_from_shielded_pool_transition::derive_identity_id_from_actions;
use crate::state_transition::identity_create_from_shielded_pool_transition::methods::IdentityCreateFromShieldedPoolTransitionMethodsV0;
use crate::state_transition::identity_create_from_shielded_pool_transition::v0::IdentityCreateFromShieldedPoolTransitionV0;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
#[cfg(feature = "state-transition-signing")]
use crate::{state_transition::StateTransition, ProtocolError};
#[cfg(feature = "state-transition-signing")]
use platform_version::version::PlatformVersion;

impl IdentityCreateFromShieldedPoolTransitionMethodsV0
    for IdentityCreateFromShieldedPoolTransitionV0
{
    #[cfg(feature = "state-transition-signing")]
    fn try_from_bundle(
        public_keys: Vec<IdentityPublicKeyInCreation>,
        denomination: u64,
        send_to_address_on_creation_failure: PlatformAddress,
        actions: Vec<SerializedAction>,
        anchor: [u8; 32],
        proof: Vec<u8>,
        binding_signature: [u8; 64],
        _platform_version: &PlatformVersion,
    ) -> Result<StateTransition, ProtocolError> {
        // The identity id is deterministically derived from the (sorted) spend nullifiers, so it is
        // unique by construction and is committed into the Orchard sighash via `extra_sighash_data`.
        let identity_id = derive_identity_id_from_actions(&actions);
        let transition = IdentityCreateFromShieldedPoolTransitionV0 {
            public_keys,
            denomination,
            send_to_address_on_creation_failure,
            actions,
            anchor,
            proof,
            binding_signature,
            identity_id,
        };
        Ok(transition.into())
    }
}
