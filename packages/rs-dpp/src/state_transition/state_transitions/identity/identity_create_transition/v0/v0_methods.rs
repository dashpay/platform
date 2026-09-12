use crate::{prelude::Identifier, state_transition::StateTransitionType};
#[cfg(feature = "state-transition-signing")]
use crate::{BlsModule, ProtocolError};

#[cfg(feature = "state-transition-signing")]
use crate::identity::accessors::IdentityGettersV0;
#[cfg(feature = "state-transition-signing")]
use crate::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
#[cfg(feature = "state-transition-signing")]
use crate::identity::signer::Signer;
#[cfg(feature = "state-transition-signing")]
use crate::identity::state_transition::AssetLockProved;
#[cfg(feature = "state-transition-signing")]
use crate::identity::Identity;
#[cfg(feature = "state-transition-signing")]
use crate::identity::KeyType::ECDSA_HASH160;
#[cfg(feature = "state-transition-signing")]
use crate::prelude::AssetLockProof;
#[cfg(feature = "state-transition-signing")]
use crate::prelude::UserFeeIncrease;
#[cfg(feature = "state-transition-signing")]
use crate::serialization::PlatformMessageSignable;
#[cfg(feature = "state-transition-signing")]
use crate::serialization::Signable;
use crate::state_transition::identity_create_transition::accessors::IdentityCreateTransitionAccessorsV0;
use crate::state_transition::identity_create_transition::methods::IdentityCreateTransitionMethodsV0;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::public_key_in_creation::accessors::IdentityPublicKeyInCreationV0Getters;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::public_key_in_creation::accessors::IdentityPublicKeyInCreationV0Setters;
#[cfg(feature = "state-transition-signing")]
use crate::util::hash::ripemd160_sha256;

#[cfg(feature = "state-transition-signing")]
use crate::identity::IdentityPublicKey;
use crate::state_transition::identity_create_transition::v0::IdentityCreateTransitionV0;
use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::{consensus_errors_as_protocol_error, StateTransition};
#[cfg(feature = "state-transition-signing")]
use crate::version::PlatformVersion;
#[cfg(feature = "state-transition-signing")]
use dashcore::secp256k1::{Secp256k1, SecretKey};
#[cfg(feature = "state-transition-signing")]
use dashcore::ScriptBuf;

#[cfg(feature = "state-transition-signing")]
fn p2pkh_pubkey_hash(script: &ScriptBuf) -> Option<[u8; 20]> {
    script
        .is_p2pkh()
        .then(|| script.as_bytes().get(3..23)?.try_into().ok())
        .flatten()
}

impl IdentityCreateTransitionMethodsV0 for IdentityCreateTransitionV0 {
    #[cfg(feature = "state-transition-signing")]
    async fn try_from_identity_with_signer_and_private_key<S: Signer<IdentityPublicKey>>(
        identity: &Identity,
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_private_key: &[u8],
        signer: &S,
        bls: &impl BlsModule,
        user_fee_increase: UserFeeIncrease,
        platform_version: &PlatformVersion,
    ) -> Result<StateTransition, ProtocolError> {
        let public_keys: Vec<IdentityPublicKeyInCreation> = identity
            .public_keys()
            .values()
            .map(|public_key| public_key.clone().into())
            .collect();
        let identity_id = asset_lock_proof.create_identifier()?;

        // Validate public key structure (purpose/security level compatibility)
        // before broadcasting, so invalid combinations are caught client-side
        // rather than being rejected by the network.
        //
        // LOCKSTEP: this call is hard-coded to the v0 public-keys-structure
        // check. If a future v1 basic-structure is introduced for this
        // transition, both the drive-abci server dispatcher AND this SDK
        // constructor must be updated together (e.g. by routing through a
        // versioned `validate_basic_structure` wrapper as IdentityUpdate does).
        let validation_result =
            IdentityPublicKeyInCreation::validate_identity_public_keys_structure(
                &public_keys,
                true, // in create_identity context
                platform_version,
            )?;
        if let Some(error) = consensus_errors_as_protocol_error(validation_result) {
            return Err(error);
        }

        let mut identity_create_transition = IdentityCreateTransitionV0 {
            public_keys,
            asset_lock_proof,
            user_fee_increase,
            identity_id,
            ..Default::default()
        };

        // Validate the asset lock proof structure client-side before signing
        // so malformed proofs are caught locally rather than being rejected by
        // the network during basic-structure validation.
        let asset_lock_validation_result = identity_create_transition
            .asset_lock_proof()
            .validate_structure(platform_version)?;
        if let Some(error) = consensus_errors_as_protocol_error(asset_lock_validation_result) {
            return Err(error);
        }

        //todo: remove clone
        let state_transition: StateTransition = identity_create_transition.clone().into();

        let key_signable_bytes = state_transition.signable_bytes()?;

        for (public_key_with_witness, (_, public_key)) in identity_create_transition
            .public_keys
            .iter_mut()
            .zip(identity.public_keys().iter())
        {
            if public_key.key_type().is_unique_key_type() {
                let signature = signer.sign(public_key, &key_signable_bytes).await?;
                public_key_with_witness.set_signature(signature);
            }
        }

        // Verify the proof-of-possession signatures we just produced before
        // returning, mirroring the server-side identity_create signatures
        // validator. Only keys with unique types were signed above, so verify
        // those exact keys here.
        for public_key_with_witness in identity_create_transition.public_keys.iter() {
            if !public_key_with_witness.key_type().is_unique_key_type() {
                continue;
            }
            let pop_result = key_signable_bytes.as_slice().verify_signature(
                public_key_with_witness.key_type(),
                public_key_with_witness.data().as_slice(),
                public_key_with_witness.signature().as_slice(),
            );
            if let Some(error) = consensus_errors_as_protocol_error(pop_result) {
                return Err(error);
            }
        }

        if let Some(transaction) = identity_create_transition.asset_lock_proof().transaction() {
            let output_index =
                identity_create_transition.asset_lock_proof().output_index() as usize;
            let output = transaction
                .special_transaction_payload
                .as_ref()
                .and_then(|payload| match payload {
                    dashcore::transaction::special_transaction::TransactionPayload::AssetLockPayloadType(payload) => {
                        payload.credit_outputs.get(output_index)
                    }
                    _ => None,
                })
                .ok_or_else(|| {
                    ProtocolError::Generic(format!(
                        "asset lock proof output {output_index} is not available for local signature verification"
                    ))
                })?;

            if let Some(locked_pubkey_hash) = p2pkh_pubkey_hash(&output.script_pubkey) {
                let secret_key =
                    SecretKey::from_slice(asset_lock_proof_private_key).map_err(|e| {
                        ProtocolError::Generic(format!("invalid asset lock proof private key: {e}"))
                    })?;
                let secp = Secp256k1::new();
                let public_key =
                    dashcore::secp256k1::PublicKey::from_secret_key(&secp, &secret_key);
                let compressed_pubkey_hash = ripemd160_sha256(&public_key.serialize());
                let uncompressed_pubkey_hash =
                    ripemd160_sha256(&public_key.serialize_uncompressed());

                if locked_pubkey_hash != compressed_pubkey_hash
                    && locked_pubkey_hash != uncompressed_pubkey_hash
                {
                    return Err(ProtocolError::Generic(
                        "asset lock proof private key does not match the locked output".to_string(),
                    ));
                }
            }
        }

        let mut state_transition: StateTransition = identity_create_transition.into();
        state_transition.sign_by_private_key(asset_lock_proof_private_key, ECDSA_HASH160, bls)?;

        Ok(state_transition)
    }

    /// Signer-driven counterpart to
    /// [`Self::try_from_identity_with_signer_and_private_key`]: the
    /// asset-lock-proof signature is produced by an external
    /// [`key_wallet::signer::Signer`] rather than from a raw
    /// `&[u8]` private key. See trait docs for the architectural rationale.
    #[cfg(all(feature = "state-transition-signing", feature = "core_key_wallet"))]
    #[allow(clippy::too_many_arguments)]
    async fn try_from_identity_with_signers<IS, AS>(
        identity: &Identity,
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_path: &::key_wallet::bip32::DerivationPath,
        identity_signer: &IS,
        asset_lock_signer: &AS,
        _bls: &impl BlsModule,
        user_fee_increase: UserFeeIncrease,
        _platform_version: &PlatformVersion,
    ) -> Result<StateTransition, ProtocolError>
    where
        IS: Signer<IdentityPublicKey>,
        AS: ::key_wallet::signer::Signer,
    {
        let public_keys: Vec<IdentityPublicKeyInCreation> = identity
            .public_keys()
            .values()
            .map(|public_key| public_key.clone().into())
            .collect();

        // Validate public key structure (purpose/security level compatibility)
        // before broadcasting, so invalid combinations are caught client-side
        // rather than being rejected by the network.
        let validation_result =
            IdentityPublicKeyInCreation::validate_identity_public_keys_structure(
                &public_keys,
                true, // in create_identity context
                _platform_version,
            )?;
        if let Some(error) = consensus_errors_as_protocol_error(validation_result) {
            return Err(error);
        }

        let identity_id = asset_lock_proof.create_identifier()?;

        let mut identity_create_transition = IdentityCreateTransitionV0 {
            public_keys,
            asset_lock_proof,
            user_fee_increase,
            identity_id,
            ..Default::default()
        };

        //todo: remove clone
        let state_transition: StateTransition = identity_create_transition.clone().into();

        let key_signable_bytes = state_transition.signable_bytes()?;

        for (public_key_with_witness, (_, public_key)) in identity_create_transition
            .public_keys
            .iter_mut()
            .zip(identity.public_keys().iter())
        {
            if public_key.key_type().is_unique_key_type() {
                let signature = identity_signer
                    .sign(public_key, &key_signable_bytes)
                    .await?;
                public_key_with_witness.set_signature(signature);
            }
        }

        let mut state_transition: StateTransition = identity_create_transition.into();

        // Atomic derive + sign + zeroise happens inside the signer. The host
        // never sees a raw private key — only a 32-byte digest goes in and a
        // serialised signature comes out.
        state_transition
            .sign_with_core_signer(asset_lock_proof_path, asset_lock_signer)
            .await?;

        Ok(state_transition)
    }

    /// Get State Transition type
    fn get_type() -> StateTransitionType {
        StateTransitionType::IdentityCreate
    }
}

impl IdentityCreateTransitionAccessorsV0 for IdentityCreateTransitionV0 {
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

    /// Returns identity id
    fn identity_id(&self) -> Identifier {
        self.identity_id
    }
}
