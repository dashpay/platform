use crate::state_transition::StateTransitionType;
#[cfg(feature = "state-transition-signing")]
use crate::{
    identity::{signer::Signer, Identity, IdentityPublicKey},
    prelude::{IdentityNonce, UserFeeIncrease},
    shielded::SerializedAction,
    state_transition::StateTransition,
    ProtocolError,
};
#[cfg(feature = "state-transition-signing")]
use platform_version::version::PlatformVersion;

pub trait ShieldFromIdentityTransitionMethodsV0 {
    /// Build and identity-sign a shield-from-identity transition from an already
    /// proven outputs-only Orchard bundle.
    ///
    /// `signing_transfer_key_to_use` selects the TRANSFER key; when `None` the first
    /// TRANSFER key the signer can use is picked, as for credit transfers.
    #[cfg(feature = "state-transition-signing")]
    #[allow(clippy::too_many_arguments)]
    async fn try_from_bundle_with_identity_signer<S: Signer<IdentityPublicKey>>(
        identity: &Identity,
        amount: u64,
        actions: Vec<SerializedAction>,
        anchor: [u8; 32],
        proof: Vec<u8>,
        binding_signature: [u8; 64],
        user_fee_increase: UserFeeIncrease,
        signer: &S,
        signing_transfer_key_to_use: Option<&IdentityPublicKey>,
        nonce: IdentityNonce,
        platform_version: &PlatformVersion,
    ) -> Result<StateTransition, ProtocolError>;

    /// Get State Transition Type
    fn get_type() -> StateTransitionType {
        StateTransitionType::ShieldFromIdentity
    }
}
