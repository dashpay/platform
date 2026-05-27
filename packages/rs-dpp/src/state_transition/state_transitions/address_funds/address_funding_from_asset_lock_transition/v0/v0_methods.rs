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
use crate::{prelude::UserFeeIncrease, state_transition::StateTransition, ProtocolError};
#[cfg(feature = "state-transition-signing")]
use dashcore::signer;
#[cfg(feature = "state-transition-signing")]
use platform_version::version::PlatformVersion;

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
        _platform_version: &PlatformVersion,
    ) -> Result<StateTransition, ProtocolError> {
        // Create the unsigned transition
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

        // Sign the asset lock proof
        let signature = signer::sign(&signable_bytes, asset_lock_proof_private_key)?;
        address_funding_transition.signature = signature.to_vec().into();

        // Sign with input witnesses
        let mut input_witnesses: Vec<AddressWitness> = Vec::with_capacity(inputs.len());
        for address in inputs.keys() {
            input_witnesses.push(signer.sign_create_witness(address, &signable_bytes).await?);
        }
        address_funding_transition.input_witnesses = input_witnesses;

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
