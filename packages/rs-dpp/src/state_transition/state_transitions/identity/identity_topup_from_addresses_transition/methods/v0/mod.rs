// =====================================
// Ungated Imports
// =====================================
use crate::state_transition::StateTransitionType;

// =====================================
// Feature-Gated Imports
// =====================================
#[cfg(feature = "state-transition-signing")]
use {
    crate::{
        address_funds::PlatformAddress,
        fee::Credits,
        identity::{signer::Signer, Identity},
        prelude::{AddressNonce, UserFeeIncrease},
        state_transition::StateTransition,
        version::FeatureVersion,
        ProtocolError,
    },
    platform_version::version::PlatformVersion,
    std::collections::BTreeMap,
};

pub trait IdentityTopUpFromAddressesTransitionMethodsV0 {
    #[cfg(feature = "state-transition-signing")]
    fn try_from_inputs_with_signer<S: Signer<PlatformAddress>>(
        identity: &Identity,
        inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        signer: &S,
        user_fee_increase: UserFeeIncrease,
        platform_version: &PlatformVersion,
        version: Option<FeatureVersion>,
    ) -> Result<StateTransition, ProtocolError>;

    /// Get State Transition type
    fn get_type() -> StateTransitionType {
        StateTransitionType::IdentityTopUpFromAddresses
    }
}
