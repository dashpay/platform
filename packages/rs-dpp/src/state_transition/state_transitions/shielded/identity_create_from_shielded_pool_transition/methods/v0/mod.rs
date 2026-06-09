#[cfg(feature = "state-transition-signing")]
use crate::address_funds::PlatformAddress;
#[cfg(feature = "state-transition-signing")]
use crate::shielded::SerializedAction;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use crate::state_transition::StateTransitionType;
#[cfg(feature = "state-transition-signing")]
use crate::{state_transition::StateTransition, ProtocolError};
#[cfg(feature = "state-transition-signing")]
use platform_version::version::PlatformVersion;

pub trait IdentityCreateFromShieldedPoolTransitionMethodsV0 {
    /// Builds the (unsigned-by-identity) transition from a pre-built Orchard bundle and the new
    /// identity's public keys. The identity id is derived from the spend nullifiers. The per-key
    /// proof-of-possession signatures are filled separately (mirroring `IdentityCreate`).
    #[cfg(feature = "state-transition-signing")]
    #[allow(clippy::too_many_arguments)]
    fn try_from_bundle(
        public_keys: Vec<IdentityPublicKeyInCreation>,
        denomination: u64,
        send_to_address_on_creation_failure: PlatformAddress,
        actions: Vec<SerializedAction>,
        anchor: [u8; 32],
        proof: Vec<u8>,
        binding_signature: [u8; 64],
        platform_version: &PlatformVersion,
    ) -> Result<StateTransition, ProtocolError>;

    /// Get State Transition Type
    fn get_type() -> StateTransitionType {
        StateTransitionType::IdentityCreateFromShieldedPool
    }
}
