//! Top-up an identity from platform addresses.

use std::collections::BTreeMap;

use dpp::identity::signer::Signer;
use dpp::identity::Identity;
use dpp::prelude::Identifier;

use dash_sdk::dapi_grpc::platform::v0::ResponseMetadata;
use dash_sdk::platform::transition::put_settings::PutSettings;
use dash_sdk::platform::transition::top_up_identity_from_addresses::TopUpIdentityFromAddressesWithMetadata;
use dash_sdk::platform::Fetch;

use dpp::address_funds::PlatformAddress;
use dpp::consensus::basic::BasicError;
use dpp::consensus::ConsensusError;
use dpp::fee::Credits;

use dash_sdk::query_types::AddressInfos;

use crate::error::{
    preserve_signer_key_unavailable_or, promote_address_nonce_error, PlatformWalletError,
};
use crate::BlockTime;

use super::*;

#[cfg(test)]
mod external_identity_tests;

// ---------------------------------------------------------------------------
// Top-up from platform addresses
// ---------------------------------------------------------------------------

impl IdentityWallet {
    /// Top up an identity by spending platform address balances.
    ///
    /// Uses the `TopUpIdentityFromAddresses` SDK trait. Address nonces are
    /// looked up automatically.
    ///
    /// This method owns only the identity-side balance update and returns
    /// the proof-attested post-spend `AddressInfos` alongside the new
    /// identity balance. Prefer the composite
    /// [`PlatformWallet::top_up_from_addresses`], which feeds the returned
    /// `AddressInfos` through
    /// [`PlatformAddressWallet::reconcile_address_infos`] — the
    /// platform-address wallet holds the address provider needed to map a
    /// spent address back to its derivation index (including addresses
    /// restored from disk that are no longer in a live derived pool).
    ///
    /// [`PlatformWallet::top_up_from_addresses`]:
    /// crate::wallet::PlatformWallet::top_up_from_addresses
    /// [`PlatformAddressWallet::reconcile_address_infos`]:
    /// crate::wallet::PlatformAddressWallet::reconcile_address_infos
    ///
    /// # Arguments
    ///
    /// * `identity_id` - The identity to top up.
    /// * `inputs` - Funding rows: platform addresses and the credits to
    ///   spend from each. Rows naming the same address are summed; a sum
    ///   that overflows is refused as an invalid parameter.
    /// * `address_signer` - Produces ECDSA signatures for the input
    ///   [`PlatformAddress`]es. Construction is the caller's concern —
    ///   seed-backed, hardware, FFI trampoline, whatever — the wallet
    ///   struct carries no key material itself.
    pub async fn top_up_from_addresses<S, I>(
        &self,
        identity_id: &Identifier,
        inputs: I,
        address_signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<(AddressInfos, Credits, u64), PlatformWalletError>
    where
        S: Signer<PlatformAddress> + Send + Sync,
        I: IntoIterator<Item = (PlatformAddress, Credits)> + Send,
    {
        let inputs = funding_inputs(inputs)?;
        let identity = {
            let wm = self.wallet_manager.read().await;
            let info = wm.get_wallet_info(&self.wallet_id).ok_or_else(|| {
                crate::error::PlatformWalletError::WalletNotFound(
                    "Wallet info not found in wallet manager".to_string(),
                )
            })?;
            let manager = &info.identity_manager;
            manager
                .identity(identity_id)
                .map(|m| m.identity.clone())
                .ok_or(PlatformWalletError::IdentityNotFound(*identity_id))?
        };

        let (address_infos, mut new_balance, metadata) = self
            .submit_top_up_from_addresses(&identity, inputs, address_signer, settings)
            .await?;

        let proof_height = metadata.height;

        // Reconcile the verified identity snapshot; failed cache writes remain pending.
        {
            let mut wm = self.wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&self.wallet_id).ok_or_else(|| {
                crate::error::PlatformWalletError::WalletNotFound(
                    "Wallet info not found in wallet manager".to_string(),
                )
            })?;
            if let Some(managed) = info.identity_manager.managed_identity_mut(identity_id) {
                new_balance = managed.persist_confirmed_balance(
                    new_balance,
                    BlockTime::from(metadata),
                    &self.persister,
                );
            }
        }

        // The spent platform-address balances are reconciled by the
        // composite `PlatformWallet::top_up_from_addresses`, which routes
        // the returned `AddressInfos` through the platform-address wallet's
        // shared reconciliation seam.
        Ok((address_infos, new_balance, proof_height))
    }

    /// Top up an identity this wallet does not manage by spending platform
    /// address balances.
    ///
    /// The identity is fetched from Platform (proved) rather than read from
    /// the identity manager, and the identity manager is neither read nor
    /// written: someone else's identity is never recorded as the wallet's
    /// own. Returns the same proof-attested post-spend `AddressInfos`, new
    /// identity balance and proof height as
    /// [`Self::top_up_from_addresses`]; prefer the composite
    /// [`PlatformWallet::top_up_external_identity_from_addresses`], which
    /// also reconciles the spent addresses.
    ///
    /// Returns [`PlatformWalletError::IdentityNotFound`] before anything is
    /// signed or broadcast when Platform has no identity with this id.
    /// `inputs` are funding rows as for [`Self::top_up_from_addresses`].
    ///
    /// [`PlatformWallet::top_up_external_identity_from_addresses`]:
    /// crate::wallet::PlatformWallet::top_up_external_identity_from_addresses
    pub async fn top_up_external_from_addresses<S, I>(
        &self,
        identity_id: &Identifier,
        inputs: I,
        address_signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<(AddressInfos, Credits, u64), PlatformWalletError>
    where
        S: Signer<PlatformAddress> + Send + Sync,
        I: IntoIterator<Item = (PlatformAddress, Credits)> + Send,
    {
        let inputs = funding_inputs(inputs)?;
        let identity = Identity::fetch(&self.sdk, *identity_id)
            .await?
            .ok_or(PlatformWalletError::IdentityNotFound(*identity_id))?;

        let (address_infos, new_balance, metadata) = self
            .submit_top_up_from_addresses(&identity, inputs, address_signer, settings)
            .await?;
        Ok((address_infos, new_balance, metadata.height))
    }

    /// Build, sign, broadcast and wait for the top-up of `identity` from
    /// `inputs`, mapping the SDK error the way every address top-up reports it.
    async fn submit_top_up_from_addresses<S: Signer<PlatformAddress> + Send + Sync>(
        &self,
        identity: &Identity,
        inputs: BTreeMap<PlatformAddress, Credits>,
        address_signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<(AddressInfos, Credits, ResponseMetadata), PlatformWalletError> {
        identity
            .top_up_from_addresses_with_metadata(&self.sdk, inputs, address_signer, settings)
            .await
            .map_err(|e| {
                preserve_signer_key_unavailable_or(e, |e| {
                    promote_address_nonce_error(&e)
                        .or_else(|| no_inputs_as_invalid_parameter(&e))
                        .unwrap_or_else(|| {
                            PlatformWalletError::InvalidIdentityData(format!(
                                "Failed to top up identity from addresses: {}",
                                e
                            ))
                        })
                })
            })
    }
}

/// The input map of an address-funded top-up from its funding rows: rows
/// naming the same address are one input of their sum. A sum that overflows
/// is refused with [`PlatformWalletError::InvalidParameter`]. No rows make an
/// empty map, which the SDK refuses as a transition without inputs.
pub(crate) fn funding_inputs(
    rows: impl IntoIterator<Item = (PlatformAddress, Credits)>,
) -> Result<BTreeMap<PlatformAddress, Credits>, PlatformWalletError> {
    let mut inputs = BTreeMap::new();
    for (address, credits) in rows {
        let sum: &mut Credits = inputs.entry(address).or_default();
        *sum = sum.checked_add(credits).ok_or_else(|| {
            PlatformWalletError::InvalidParameter(format!(
                "the funding amounts for {address:?} overflow"
            ))
        })?;
    }
    Ok(inputs)
}

/// The SDK's refusal of a transition without inputs, reported as the
/// caller's mistake it is.
fn no_inputs_as_invalid_parameter(error: &dash_sdk::Error) -> Option<PlatformWalletError> {
    matches!(
        error.consensus_error(),
        Some(ConsensusError::BasicError(
            BasicError::TransitionNoInputsError(_)
        ))
    )
    .then(|| PlatformWalletError::InvalidParameter(error.to_string()))
}
