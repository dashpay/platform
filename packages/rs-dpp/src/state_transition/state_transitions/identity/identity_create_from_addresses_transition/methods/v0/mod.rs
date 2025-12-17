#[cfg(feature = "state-transition-signing")]
use std::collections::BTreeMap;

#[cfg(feature = "state-transition-signing")]
use crate::address_funds::AddressFundsFeeStrategy;
#[cfg(feature = "state-transition-signing")]
use crate::address_funds::PlatformAddress;
#[cfg(feature = "state-transition-signing")]
use crate::fee::Credits;
#[cfg(feature = "state-transition-signing")]
use crate::identity::signer::Signer;
#[cfg(feature = "state-transition-signing")]
use crate::identity::Identity;
#[cfg(feature = "state-transition-signing")]
use crate::identity::IdentityPublicKey;
#[cfg(feature = "state-transition-signing")]
use crate::prelude::AddressNonce;
#[cfg(feature = "state-transition-signing")]
use crate::prelude::UserFeeIncrease;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::StateTransition;
use crate::state_transition::StateTransitionType;
#[cfg(feature = "state-transition-signing")]
use crate::ProtocolError;
#[cfg(feature = "state-transition-signing")]
use platform_version::version::PlatformVersion;

pub trait IdentityCreateFromAddressesTransitionMethodsV0 {
    #[cfg(feature = "state-transition-signing")]
    #[allow(clippy::too_many_arguments)]
    fn try_from_inputs_with_signer<S: Signer<IdentityPublicKey>, WS: Signer<PlatformAddress>>(
        identity: &Identity,
        inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        output: Option<(PlatformAddress, Credits)>,
        fee_strategy: AddressFundsFeeStrategy,
        identity_public_key_signer: &S,
        address_signer: &WS,
        user_fee_increase: UserFeeIncrease,
        platform_version: &PlatformVersion,
    ) -> Result<StateTransition, ProtocolError>;
    /// Get State Transition type
    fn get_type() -> StateTransitionType;
}
