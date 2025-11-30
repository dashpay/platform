// ============================
// Standard Library
// ============================
#[cfg(feature = "state-transition-signing")]
use std::collections::BTreeMap;

// ============================
// Crate: Ungated Imports
// ============================
use crate::address_funds::{AddressFundsFeeStrategy, PlatformAddress};
use crate::fee::Credits;
use crate::state_transition::identity_create_from_addresses_transition::accessors::IdentityCreateFromAddressesTransitionAccessorsV0;
use crate::state_transition::identity_create_from_addresses_transition::methods::IdentityCreateFromAddressesTransitionMethodsV0;
use crate::state_transition::identity_create_from_addresses_transition::v0::IdentityCreateFromAddressesTransitionV0;
use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use crate::state_transition::StateTransitionIdentityIdFromInputs;
use crate::state_transition::StateTransitionType;

// ============================
// Crate: Feature-Gated (state-transition-signing)
// ============================
#[cfg(feature = "state-transition-signing")]
use crate::{
    identity::{
        accessors::IdentityGettersV0,
        identity_public_key::accessors::v0::IdentityPublicKeyGettersV0, signer::Signer, Identity,
        IdentityPublicKey, KeyType::ECDSA_HASH160,
    },
    prelude::{AddressNonce, UserFeeIncrease},
    serialization::Signable,
    state_transition::{
        public_key_in_creation::accessors::IdentityPublicKeyInCreationV0Setters, StateTransition,
    },
    version::PlatformVersion,
    BlsModule, ProtocolError,
};

impl IdentityCreateFromAddressesTransitionMethodsV0 for IdentityCreateFromAddressesTransitionV0 {
    #[cfg(feature = "state-transition-signing")]
    fn try_from_inputs_with_signer<S: Signer<IdentityPublicKey>>(
        identity: &Identity,
        inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        input_private_keys: Vec<&[u8]>,
        signer: &S,
        bls: &impl BlsModule,
        user_fee_increase: UserFeeIncrease,
        _platform_version: &PlatformVersion,
    ) -> Result<StateTransition, ProtocolError> {
        let mut identity_create_from_addresses_transition =
            IdentityCreateFromAddressesTransitionV0 {
                inputs,
                user_fee_increase,
                ..Default::default()
            };
        let public_keys = identity
            .public_keys()
            .values()
            .map(|public_key| public_key.clone().into())
            .collect();
        identity_create_from_addresses_transition.set_public_keys(public_keys);

        //todo: remove clone
        let state_transition: StateTransition =
            identity_create_from_addresses_transition.clone().into();

        let key_signable_bytes = state_transition.signable_bytes()?;

        // Sign with public keys
        identity_create_from_addresses_transition
            .public_keys
            .iter_mut()
            .zip(identity.public_keys().iter())
            .try_for_each(|(public_key_with_witness, (_, public_key))| {
                if public_key.key_type().is_unique_key_type() {
                    let signature = signer.sign(public_key, &key_signable_bytes)?;
                    public_key_with_witness.set_signature(signature);
                }
                Ok::<(), ProtocolError>(())
            })?;

        let mut state_transition: StateTransition =
            identity_create_from_addresses_transition.into();

        // Sign with input private keys
        for input_private_key in input_private_keys {
            state_transition.sign_by_private_key(input_private_key, ECDSA_HASH160, bls)?;
        }

        Ok(state_transition)
    }

    /// Get State Transition type
    fn get_type() -> StateTransitionType {
        StateTransitionType::IdentityCreateFromAddresses
    }
}

impl IdentityCreateFromAddressesTransitionAccessorsV0 for IdentityCreateFromAddressesTransitionV0 {
    /// Get identity public keys
    fn public_keys(&self) -> &[IdentityPublicKeyInCreation] {
        &self.public_keys
    }

    /// Get identity public keys
    fn public_keys_mut(&mut self) -> &mut Vec<IdentityPublicKeyInCreation> {
        &mut self.public_keys
    }

    /// Replaces existing set of public keys with a new one
    fn set_public_keys(&mut self, public_keys: Vec<IdentityPublicKeyInCreation>) {
        self.public_keys = public_keys;
    }

    /// Adds public keys to the existing public keys array
    fn add_public_keys(&mut self, public_keys: &mut Vec<IdentityPublicKeyInCreation>) {
        self.public_keys.append(public_keys);
    }

    /// Get the optional output
    fn output(&self) -> Option<&(PlatformAddress, Credits)> {
        self.output.as_ref()
    }

    /// Set the optional output
    fn set_output(&mut self, output: Option<(PlatformAddress, Credits)>) {
        self.output = output;
    }

    /// Get fee strategy
    fn fee_strategy(&self) -> &AddressFundsFeeStrategy {
        &self.fee_strategy
    }

    /// Set fee strategy
    fn set_fee_strategy(&mut self, fee_strategy: AddressFundsFeeStrategy) {
        self.fee_strategy = fee_strategy;
    }
}

impl StateTransitionIdentityIdFromInputs for IdentityCreateFromAddressesTransitionV0 {}
