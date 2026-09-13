// ============================
// Standard Library
// ============================
#[cfg(feature = "state-transition-signing")]
use std::collections::BTreeMap;

// ============================
// Crate: Ungated Imports
// ============================
use crate::address_funds::PlatformAddress;
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
use crate::state_transition::{
    address_funds_constructor_dispatch_error, consensus_errors_as_protocol_error,
    verify_address_witnesses,
};
#[cfg(feature = "state-transition-signing")]
use crate::{
    serialization::PlatformMessageSignable,
    state_transition::public_key_in_creation::accessors::IdentityPublicKeyInCreationV0Getters,
};

#[cfg(feature = "state-transition-signing")]
use crate::{
    address_funds::AddressFundsFeeStrategy,
    identity::{
        accessors::IdentityGettersV0,
        identity_public_key::accessors::v0::IdentityPublicKeyGettersV0, signer::Signer, Identity,
        IdentityPublicKey,
    },
    prelude::{AddressNonce, UserFeeIncrease},
    serialization::Signable,
    state_transition::{
        public_key_in_creation::accessors::IdentityPublicKeyInCreationV0Setters, StateTransition,
    },
    version::PlatformVersion,
    ProtocolError,
};

impl IdentityCreateFromAddressesTransitionMethodsV0 for IdentityCreateFromAddressesTransitionV0 {
    #[cfg(feature = "state-transition-signing")]
    async fn try_from_inputs_with_signer<
        S: Signer<IdentityPublicKey>,
        WS: Signer<PlatformAddress>,
    >(
        identity: &Identity,
        inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        output: Option<(PlatformAddress, Credits)>,
        fee_strategy: AddressFundsFeeStrategy,
        identity_public_key_signer: &S,
        address_signer: &WS,
        user_fee_increase: UserFeeIncrease,
        platform_version: &PlatformVersion,
    ) -> Result<StateTransition, ProtocolError> {
        // Create the unsigned transition
        let mut identity_create_from_addresses_transition =
            IdentityCreateFromAddressesTransitionV0 {
                inputs,
                output,
                fee_strategy,
                user_fee_increase,
                input_witnesses: Vec::new(),
                ..Default::default()
            };

        if let Some(error) = address_funds_constructor_dispatch_error(
            StateTransitionType::IdentityCreateFromAddresses,
            platform_version,
        ) {
            return Err(error);
        }

        let public_keys: Vec<IdentityPublicKeyInCreation> = identity
            .public_keys()
            .values()
            .map(|public_key| public_key.clone().into())
            .collect();

        // Validate public key structure (purpose/security level compatibility)
        // before broadcasting, so invalid combinations are caught client-side
        // rather than being rejected by the network.
        //
        // LOCKSTEP: both this call and the
        // `validate_structure_without_input_witnesses` call below are hard-coded
        // to the v0 basic-structure checks. If a future v1 basic-structure is
        // introduced for this transition, both the drive-abci server dispatcher
        // AND this SDK constructor must be updated together (e.g. by routing
        // through a versioned `validate_basic_structure` wrapper as
        // IdentityUpdate does).
        let validation_result =
            IdentityPublicKeyInCreation::validate_identity_public_keys_structure(
                &public_keys,
                true, // in create_identity context
                platform_version,
            )?;
        if let Some(error) = consensus_errors_as_protocol_error(validation_result) {
            return Err(error);
        }

        identity_create_from_addresses_transition.set_public_keys(public_keys);

        // Pre-signing structure check: validate everything except the witness
        // count, so structural errors fail fast before performing any async
        // signer work. See the LOCKSTEP note above.
        let pre_validation_result = identity_create_from_addresses_transition
            .validate_structure_without_input_witnesses(platform_version);
        if let Some(error) = consensus_errors_as_protocol_error(pre_validation_result) {
            return Err(error);
        }

        // Get signable bytes for the state transition
        let state_transition: StateTransition =
            identity_create_from_addresses_transition.clone().into();
        let signable_bytes = state_transition.signable_bytes()?;

        // Sign public keys with the identity public key signer (proof of possession)
        for (public_key_with_witness, (_, public_key)) in identity_create_from_addresses_transition
            .public_keys
            .iter_mut()
            .zip(identity.public_keys().iter())
        {
            if public_key.key_type().is_unique_key_type() {
                let signature = identity_public_key_signer
                    .sign(public_key, &signable_bytes)
                    .await?;
                public_key_with_witness.set_signature(signature);
            }
        }

        // Verify proof-of-possession signatures we just produced before
        // returning, matching the server-side
        // `IdentityCreateFromAddressesStateTransitionSignaturesValidationV0`
        // check. Only keys with unique types were signed above, so verify
        // those exact keys here.
        for public_key_with_witness in identity_create_from_addresses_transition.public_keys.iter()
        {
            if !public_key_with_witness.key_type().is_unique_key_type() {
                continue;
            }
            let pop_result = signable_bytes.as_slice().verify_signature(
                public_key_with_witness.key_type(),
                public_key_with_witness.data().as_slice(),
                public_key_with_witness.signature().as_slice(),
            );
            if let Some(error) = consensus_errors_as_protocol_error(pop_result) {
                return Err(error);
            }
        }

        // Create witnesses for each input address
        let mut input_witnesses =
            Vec::with_capacity(identity_create_from_addresses_transition.inputs.len());
        for address in identity_create_from_addresses_transition.inputs.keys() {
            input_witnesses.push(
                address_signer
                    .sign_create_witness(address, &signable_bytes)
                    .await?,
            );
        }
        verify_address_witnesses(
            identity_create_from_addresses_transition.inputs.keys(),
            &input_witnesses,
            &signable_bytes,
        )?;
        identity_create_from_addresses_transition.input_witnesses = input_witnesses;

        // After signing, only the witness count needs (re-)validation; the rest
        // of the structure was already verified above.
        let validation_result =
            identity_create_from_addresses_transition.validate_input_witnesses_count();
        if let Some(error) = consensus_errors_as_protocol_error(validation_result) {
            return Err(error);
        }

        Ok(identity_create_from_addresses_transition.into())
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
}

impl StateTransitionIdentityIdFromInputs for IdentityCreateFromAddressesTransitionV0 {}
