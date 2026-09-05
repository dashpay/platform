#[cfg(feature = "state-transition-signing")]
use std::collections::BTreeMap;

#[cfg(feature = "state-transition-signing")]
use crate::address_funds::{AddressFundsFeeStrategy, AddressWitness, PlatformAddress};
#[cfg(feature = "state-transition-signing")]
use crate::fee::Credits;
#[cfg(feature = "state-transition-signing")]
use crate::identity::signer::Signer;
#[cfg(feature = "state-transition-signing")]
use crate::prelude::{AddressNonce, AssetLockProof};
#[cfg(feature = "state-transition-signing")]
use crate::serialization::Signable;
use crate::state_transition::address_funding_from_asset_lock_transition::methods::AddressFundingFromAssetLockTransitionMethodsV0;
use crate::state_transition::address_funding_from_asset_lock_transition::v0::AddressFundingFromAssetLockTransitionV0;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::{
    address_funds_constructor_dispatch_error, consensus_errors_as_protocol_error,
    verify_address_witnesses, StateTransitionType,
};
#[cfg(feature = "state-transition-signing")]
use crate::util::hash::ripemd160_sha256;
#[cfg(feature = "state-transition-signing")]
use crate::{prelude::UserFeeIncrease, state_transition::StateTransition, ProtocolError};
#[cfg(feature = "state-transition-signing")]
use dashcore::secp256k1::{Secp256k1, SecretKey};
#[cfg(feature = "state-transition-signing")]
use dashcore::signer;
#[cfg(feature = "state-transition-signing")]
use dashcore::ScriptBuf;
#[cfg(feature = "state-transition-signing")]
use platform_version::version::PlatformVersion;

#[cfg(feature = "state-transition-signing")]
fn p2pkh_pubkey_hash(script: &ScriptBuf) -> Option<[u8; 20]> {
    script
        .is_p2pkh()
        .then(|| script.as_bytes().get(3..23)?.try_into().ok())
        .flatten()
}

impl AddressFundingFromAssetLockTransitionMethodsV0 for AddressFundingFromAssetLockTransitionV0 {
    #[cfg(feature = "state-transition-signing")]
    async fn try_from_asset_lock_with_signer_and_private_key<S: Signer<PlatformAddress>>(
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_private_key: &[u8],
        inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        outputs: BTreeMap<PlatformAddress, Option<Credits>>,
        fee_strategy: AddressFundsFeeStrategy,
        signer: &S,
        user_fee_increase: UserFeeIncrease,
        platform_version: &PlatformVersion,
    ) -> Result<StateTransition, ProtocolError> {
        // Create the unsigned transition
        let mut address_funding_transition = AddressFundingFromAssetLockTransitionV0 {
            asset_lock_proof,
            inputs,
            outputs,
            fee_strategy,
            user_fee_increase,
            signature: Default::default(),
            input_witnesses: Vec::new(),
        };

        if let Some(error) = address_funds_constructor_dispatch_error(
            StateTransitionType::AddressFundingFromAssetLock,
            platform_version,
        ) {
            return Err(error);
        }

        // Pre-signing structure check: validate everything except the witness
        // count, so structural errors fail fast before performing any async
        // signer work.
        //
        // LOCKSTEP: this call is hard-coded to the v0 basic-structure check.
        // If a future v1 basic-structure is introduced for this transition,
        // both the drive-abci server dispatcher AND this SDK constructor must
        // be updated together (e.g. by routing through a versioned
        // `validate_basic_structure` wrapper as IdentityUpdate does).
        let pre_validation_result =
            address_funding_transition.validate_structure_without_input_witnesses(platform_version);
        if let Some(error) = consensus_errors_as_protocol_error(pre_validation_result) {
            return Err(error);
        }

        // Validate the asset lock proof structure client-side before signing
        // so malformed proofs are caught locally rather than being rejected by
        // the network during basic-structure validation. Mirrors the symmetric
        // check in IdentityCreateTransitionV0::try_from_identity_with_signer.
        let asset_lock_validation_result = address_funding_transition
            .asset_lock_proof
            .validate_structure(platform_version)?;
        if let Some(error) = consensus_errors_as_protocol_error(asset_lock_validation_result) {
            return Err(error);
        }

        let state_transition: StateTransition = address_funding_transition.clone().into();

        let signable_bytes = state_transition.signable_bytes()?;

        // Sign the asset lock proof
        let signature = signer::sign(&signable_bytes, asset_lock_proof_private_key)?;
        if let Some(transaction) = address_funding_transition.asset_lock_proof.transaction() {
            let output_index = address_funding_transition.asset_lock_proof.output_index() as usize;
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
        } else {
            // Only Instant asset-lock proofs carry the full transaction needed for
            // local script/private-key matching. Chain proofs are still validated
            // structurally above, but this constructor cannot verify their locked
            // output locally from just the outpoint.
            tracing::debug!(
                "skipping local asset-lock private-key verification because the proof does not carry a transaction"
            );
        }
        address_funding_transition.signature = signature.to_vec().into();

        // Sign with input witnesses
        let mut input_witnesses: Vec<AddressWitness> =
            Vec::with_capacity(address_funding_transition.inputs.len());
        for address in address_funding_transition.inputs.keys() {
            input_witnesses.push(signer.sign_create_witness(address, &signable_bytes).await?);
        }
        verify_address_witnesses(
            address_funding_transition.inputs.keys(),
            &input_witnesses,
            &signable_bytes,
        )?;
        address_funding_transition.input_witnesses = input_witnesses;

        // After signing, only the witness count needs (re-)validation; the rest
        // of the structure was already verified above.
        let validation_result = address_funding_transition.validate_input_witnesses_count();
        if let Some(error) = consensus_errors_as_protocol_error(validation_result) {
            return Err(error);
        }

        tracing::debug!("try_from_asset_lock_with_signer: Successfully created transition");
        Ok(address_funding_transition.into())
    }

    #[cfg(all(feature = "state-transition-signing", feature = "core_key_wallet"))]
    async fn try_from_asset_lock_with_signers<S, AS>(
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_path: &::key_wallet::bip32::DerivationPath,
        inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        outputs: BTreeMap<PlatformAddress, Option<Credits>>,
        fee_strategy: AddressFundsFeeStrategy,
        signer: &S,
        asset_lock_signer: &AS,
        user_fee_increase: UserFeeIncrease,
        _platform_version: &PlatformVersion,
    ) -> Result<StateTransition, ProtocolError>
    where
        S: Signer<PlatformAddress>,
        AS: ::key_wallet::signer::Signer,
    {
        // Build the unsigned inner transition. The outer wrapper
        // signature and the per-input witnesses are both
        // `#[platform_signable(exclude_from_sig_hash)]`, so they
        // don't affect the signable bytes the per-input signer
        // produces — we can compute signable bytes once with both
        // empty.
        let mut address_funding_transition = AddressFundingFromAssetLockTransitionV0 {
            asset_lock_proof,
            inputs: inputs.clone(),
            outputs,
            fee_strategy,
            user_fee_increase,
            signature: Default::default(),
            input_witnesses: Vec::new(),
        };

        let state_transition: StateTransition = address_funding_transition.clone().into();
        let signable_bytes = state_transition.signable_bytes()?;

        // Sign per-input witnesses up front so the input_witnesses
        // field is populated before we hand the inner over to the
        // outer ST for the asset-lock signature.
        let mut input_witnesses: Vec<AddressWitness> = Vec::with_capacity(inputs.len());
        for address in inputs.keys() {
            input_witnesses.push(signer.sign_create_witness(address, &signable_bytes).await?);
        }
        address_funding_transition.input_witnesses = input_witnesses;

        // Build the outer ST and route the asset-lock-proof signature
        // through the external `Signer`. The derive + sign + zeroise
        // sequence happens inside the signer — the host never sees a
        // raw private key, only a 32-byte digest goes in and a
        // serialised signature comes out.
        let mut state_transition: StateTransition = address_funding_transition.into();
        state_transition
            .sign_with_core_signer(asset_lock_proof_path, asset_lock_signer)
            .await?;

        Ok(state_transition)
    }
}
