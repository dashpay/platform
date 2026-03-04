mod v0;

#[cfg(feature = "state-transition-signing")]
use std::collections::BTreeMap;
pub use v0::*;

#[cfg(feature = "state-transition-signing")]
use crate::address_funds::{AddressFundsFeeStrategy, PlatformAddress};
#[cfg(feature = "state-transition-signing")]
use crate::fee::Credits;
#[cfg(feature = "state-transition-signing")]
use crate::identity::signer::Signer;
#[cfg(feature = "state-transition-signing")]
use crate::shielded::SerializedAction;
use crate::state_transition::shield_transition::ShieldTransition;
#[cfg(feature = "state-transition-signing")]
use crate::{
    prelude::{AddressNonce, UserFeeIncrease},
    state_transition::{shield_transition::v0::ShieldTransitionV0, StateTransition},
    ProtocolError,
};
#[cfg(feature = "state-transition-signing")]
use platform_version::version::PlatformVersion;

impl ShieldTransitionMethodsV0 for ShieldTransition {
    #[cfg(feature = "state-transition-signing")]
    fn try_from_bundle_with_signer<S: Signer<PlatformAddress>>(
        inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        actions: Vec<SerializedAction>,
        flags: u8,
        value_balance: i64,
        anchor: [u8; 32],
        proof: Vec<u8>,
        binding_signature: [u8; 64],
        fee_strategy: AddressFundsFeeStrategy,
        signer: &S,
        user_fee_increase: UserFeeIncrease,
        platform_version: &PlatformVersion,
    ) -> Result<StateTransition, ProtocolError> {
        match platform_version
            .dpp
            .state_transition_serialization_versions
            .shield_state_transition
            .default_current_version
        {
            0 => ShieldTransitionV0::try_from_bundle_with_signer(
                inputs,
                actions,
                flags,
                value_balance,
                anchor,
                proof,
                binding_signature,
                fee_strategy,
                signer,
                user_fee_increase,
                platform_version,
            ),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "ShieldTransition::try_from_bundle_with_signer".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
