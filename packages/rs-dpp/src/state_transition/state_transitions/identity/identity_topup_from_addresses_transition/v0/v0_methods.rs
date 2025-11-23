#[cfg(feature = "state-transition-signing")]
use crate::identity::accessors::IdentityGettersV0;
#[cfg(feature = "state-transition-signing")]
use crate::identity::signer::Signer;
#[cfg(feature = "state-transition-signing")]
use crate::identity::Identity;
#[cfg(feature = "state-transition-signing")]
use crate::prelude::UserFeeIncrease;
use crate::prelude::{Identifier, KeyOfTypeNonce};
#[cfg(feature = "state-transition-signing")]
use crate::ProtocolError;
use std::collections::BTreeMap;

use crate::state_transition::identity_topup_from_addresses_transition::accessors::IdentityTopUpFromAddressesTransitionAccessorsV0;
use crate::state_transition::identity_topup_from_addresses_transition::methods::IdentityTopUpFromAddressesTransitionMethodsV0;

use crate::address_funds::AddressWitness;
use crate::fee::Credits;
use crate::identity::KeyOfType;
#[cfg(feature = "state-transition-signing")]
use crate::serialization::Signable;
use crate::state_transition::identity_topup_from_addresses_transition::v0::IdentityTopUpFromAddressesTransitionV0;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::StateTransition;
use crate::version::FeatureVersion;
#[cfg(feature = "state-transition-signing")]
use platform_version::version::PlatformVersion;

impl IdentityTopUpFromAddressesTransitionMethodsV0 for IdentityTopUpFromAddressesTransitionV0 {
    #[cfg(feature = "state-transition-signing")]
    fn try_from_inputs_with_signer<S: Signer<KeyOfType>>(
        identity: &Identity,
        inputs: BTreeMap<KeyOfType, (KeyOfTypeNonce, Credits)>,
        signer: &S,
        user_fee_increase: UserFeeIncrease,
        _platform_version: &PlatformVersion,
        _version: Option<FeatureVersion>,
    ) -> Result<StateTransition, ProtocolError> {
        let mut identity_top_up_from_addresses_transition =
            IdentityTopUpFromAddressesTransitionV0 {
                inputs: inputs.clone(),
                identity_id: identity.id(),
                user_fee_increase,
                input_witnesses: vec![],
            };

        let state_transition: StateTransition =
            identity_top_up_from_addresses_transition.clone().into();

        let signable_bytes = state_transition.signable_bytes()?;

        identity_top_up_from_addresses_transition.input_witnesses = inputs
            .iter()
            .map(|(key_of_type, _)| signer.sign_create_witness(key_of_type, &signable_bytes))
            .collect::<Result<Vec<AddressWitness>, ProtocolError>>()?;

        Ok(identity_top_up_from_addresses_transition.into())
    }
}

impl IdentityTopUpFromAddressesTransitionAccessorsV0 for IdentityTopUpFromAddressesTransitionV0 {
    /// Set identity id
    fn set_identity_id(&mut self, identity_id: Identifier) {
        self.identity_id = identity_id;
    }

    /// Returns identity id
    fn identity_id(&self) -> &Identifier {
        &self.identity_id
    }
}
