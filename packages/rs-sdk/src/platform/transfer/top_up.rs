use super::types::{AddressSigner, TransferInput, TransferOutput};
use crate::platform::transition::put_settings::PutSettings;
use crate::{Error, Sdk};
use dpp::address_funds::{AddressFundsFeeStrategy, PlatformAddress};
use dpp::fee::Credits;
use dpp::prelude::AssetLockProof;
use dpp::state_transition::address_funding_from_asset_lock_transition::methods::AddressFundingFromAssetLockTransitionMethodsV0;
use dpp::state_transition::address_funding_from_asset_lock_transition::AddressFundingFromAssetLockTransition;
use dpp::state_transition::StateTransition;
use std::collections::BTreeMap;
use zeroize::Zeroize;

/// Plan describing an address top up funded via an asset lock.
pub(crate) struct AddressTopUpPlan {
    signer: AddressSigner,
    fee_strategy: AddressFundsFeeStrategy,
    asset_lock_proof: AssetLockProof,
    asset_lock_private_key: Vec<u8>,
    outputs: BTreeMap<PlatformAddress, Option<Credits>>,
}

impl std::fmt::Debug for AddressTopUpPlan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AddressTopUpPlan")
            .field("output_count", &self.outputs.len())
            .finish()
    }
}

impl AddressTopUpPlan {
    fn new(
        signer: AddressSigner,
        fee_strategy: AddressFundsFeeStrategy,
        asset_lock_proof: AssetLockProof,
        asset_lock_private_key: Vec<u8>,
        outputs: BTreeMap<PlatformAddress, Option<Credits>>,
    ) -> Self {
        Self {
            signer,
            fee_strategy,
            asset_lock_proof,
            asset_lock_private_key,
            outputs,
        }
    }

    pub(crate) async fn build_state_transition(
        &self,
        sdk: &Sdk,
        settings: Option<PutSettings>,
    ) -> Result<StateTransition, Error> {
        let user_fee_increase = settings
            .as_ref()
            .and_then(|settings| settings.user_fee_increase)
            .unwrap_or_default();

        AddressFundingFromAssetLockTransition::try_from_asset_lock_with_signer(
            self.asset_lock_proof.clone(),
            self.asset_lock_private_key.as_slice(),
            BTreeMap::new(),
            self.outputs.clone(),
            self.fee_strategy.clone(),
            &self.signer,
            user_fee_increase,
            sdk.version(),
        )
        .map_err(Error::from)
    }
}

impl Drop for AddressTopUpPlan {
    fn drop(&mut self) {
        self.asset_lock_private_key.zeroize();
    }
}

/// Classify address top up inputs/outputs into a reusable plan.
pub(crate) fn classify_address_top_up(
    inputs: &[TransferInput],
    outputs: &BTreeMap<TransferOutput, Credits>,
    signer: AddressSigner,
    fee_strategy: AddressFundsFeeStrategy,
) -> Result<AddressTopUpPlan, Error> {
    let (proof, private_key) = extract_asset_lock_funding(inputs)?;
    let address_outputs = collect_top_up_outputs(outputs)?;
    Ok(AddressTopUpPlan::new(
        signer,
        fee_strategy,
        proof,
        private_key,
        address_outputs,
    ))
}

fn extract_asset_lock_funding(
    inputs: &[TransferInput],
) -> Result<(AssetLockProof, Vec<u8>), Error> {
    let mut funding: Option<(AssetLockProof, Vec<u8>)> = None;

    for source in inputs {
        match source {
            TransferInput::AssetLock {
                asset_lock_proof,
                asset_lock_private_key,
            } => {
                if funding.is_some() {
                    return Err(Error::InvalidCreditTransfer(
                        "address top up supports exactly one asset lock funding input".to_string(),
                    ));
                }
                funding = Some((
                    asset_lock_proof.clone(),
                    asset_lock_private_key.inner.as_ref().to_vec(),
                ));
            }
            TransferInput::Addresses { .. } | TransferInput::AddressesWithNonce { .. } => {
                return Err(Error::InvalidCreditTransfer(
                    "address top up does not accept Platform address inputs".to_string(),
                ));
            }
            TransferInput::Identity(_) => {
                return Err(Error::InvalidCreditTransfer(
                    "address top up does not accept identity funding inputs".to_string(),
                ));
            }
        }
    }

    funding.ok_or_else(|| {
        Error::InvalidCreditTransfer(
            "address top up requires an asset lock funding input".to_string(),
        )
    })
}

fn collect_top_up_outputs(
    outputs: &BTreeMap<TransferOutput, Credits>,
) -> Result<BTreeMap<PlatformAddress, Option<Credits>>, Error> {
    if outputs.is_empty() {
        return Err(Error::InvalidCreditTransfer(
            "address top up requires at least one output".to_string(),
        ));
    }

    let mut address_outputs = BTreeMap::new();
    for (output, amount) in outputs {
        match output {
            TransferOutput::PlatformAddress(address) => {
                address_outputs.insert(*address, Some(*amount));
            }
            _ => {
                return Err(Error::InvalidCreditTransfer(
                    "address top up outputs must be Platform addresses".to_string(),
                ))
            }
        }
    }

    Ok(address_outputs)
}
