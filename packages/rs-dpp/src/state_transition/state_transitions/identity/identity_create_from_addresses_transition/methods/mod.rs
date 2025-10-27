mod v0;

pub use v0::*;

#[cfg(feature = "state-transition-signing")]
use crate::fee::Credits;
#[cfg(feature = "state-transition-signing")]
use crate::identity::signer::Signer;
#[cfg(feature = "state-transition-signing")]
use crate::identity::Identity;
#[cfg(feature = "state-transition-signing")]
use crate::identity::KeyOfType;
#[cfg(feature = "state-transition-signing")]
use crate::prelude::UserFeeIncrease;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::identity_create_from_addresses_transition::v0::IdentityCreateFromAddressesTransitionV0;
use crate::state_transition::identity_create_from_addresses_transition::IdentityCreateFromAddressesTransition;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::StateTransition;
use crate::state_transition::StateTransitionType;
#[cfg(feature = "state-transition-signing")]
use crate::version::PlatformVersion;
#[cfg(feature = "state-transition-signing")]
use crate::{BlsModule, ProtocolError};
#[cfg(feature = "state-transition-signing")]
use std::collections::BTreeMap;

impl IdentityCreateFromAddressesTransitionMethodsV0 for IdentityCreateFromAddressesTransition {
    #[cfg(feature = "state-transition-signing")]
    fn try_from_inputs_with_signer<S: Signer>(
        identity: &Identity,
        inputs: Vec<KeyOfType>,
        outputs: BTreeMap<KeyOfType, Credits>,
        input_private_keys: Vec<&[u8]>,
        signer: &S,
        bls: &impl BlsModule,
        user_fee_increase: UserFeeIncrease,
        platform_version: &PlatformVersion,
    ) -> Result<StateTransition, ProtocolError> {
        match platform_version
            .dpp
            .state_transition_conversion_versions
            .inputs_to_identity_create_from_addresses_transition_with_signer
        {
            0 => Ok(IdentityCreateFromAddressesTransitionV0::try_from_inputs_with_signer(
                identity,
                inputs,
                outputs,
                input_private_keys,
                signer,
                bls,
                user_fee_increase,
                platform_version,
            )?),
            v => Err(ProtocolError::UnknownVersionError(format!(
                "Unknown IdentityCreateFromAddressesTransition version for try_from_inputs_with_signer {v}"
            ))),
        }
    }

    fn get_type() -> StateTransitionType {
        StateTransitionType::IdentityCreateFromAddresses
    }
}
