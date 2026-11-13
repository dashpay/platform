//! Top-up an identity from platform addresses.

use std::collections::BTreeMap;

use dpp::identity::signer::Signer;
use dpp::identity::Identity;
use dpp::prelude::{AddressNonce, Identifier};

use dash_sdk::dapi_grpc::platform::v0::ResponseMetadata;
use dash_sdk::platform::transition::fetch_inputs_with_nonce;
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
    /// [`Self::top_up_from_addresses`].
    ///
    /// Crate-private: it does not check the wallet generation or reconcile
    /// the spent addresses. Callers outside the crate use
    /// [`PlatformWallet::top_up_external_identity_from_addresses`], which
    /// holds the lifecycle gate and reconciles.
    ///
    /// Returns [`PlatformWalletError::IdentityNotFound`] before anything is
    /// signed or broadcast when Platform has no identity with this id, and
    /// [`PlatformWalletError::IdentityLookupFailed`], likewise before
    /// anything is signed or broadcast, when the lookup itself fails.
    /// `inputs` are funding rows as for [`Self::top_up_from_addresses`].
    ///
    /// [`PlatformWallet::top_up_external_identity_from_addresses`]:
    /// crate::wallet::PlatformWallet::top_up_external_identity_from_addresses
    pub(crate) async fn top_up_external_from_addresses<S, I>(
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
            .await
            .map_err(|source| PlatformWalletError::IdentityLookupFailed {
                identity_id: *identity_id,
                source,
            })?
            .ok_or(PlatformWalletError::IdentityNotFound(*identity_id))?;

        let (address_infos, new_balance, metadata) = self
            .submit_top_up_from_addresses(&identity, inputs, address_signer, settings)
            .await?;
        Ok((address_infos, new_balance, metadata.height))
    }

    /// Build, sign, broadcast and wait for the top-up of `identity` from
    /// `inputs`, mapping the SDK error the way every address top-up reports it.
    ///
    /// The result proof of this transition authenticates only the state it
    /// affects (the identity's balance and the input addresses as of the
    /// proof's block), not that this transition executed. The result stands
    /// only once the proof shows every input address at or past the nonce
    /// this transition was signed with ([`bind_to_submitted_nonces`]);
    /// otherwise the top-up is reported unconfirmed and nothing is
    /// reconciled from it.
    async fn submit_top_up_from_addresses<S: Signer<PlatformAddress> + Send + Sync>(
        &self,
        identity: &Identity,
        inputs: BTreeMap<PlatformAddress, Credits>,
        address_signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<(AddressInfos, Credits, ResponseMetadata), PlatformWalletError> {
        let inputs = fetch_inputs_with_nonce(&self.sdk, &inputs)
            .await
            .map_err(top_up_error)?
            .into_iter()
            .map(|(address, (nonce, credits))| {
                let next = nonce.checked_add(1).ok_or_else(|| {
                    PlatformWalletError::InvalidParameter(format!(
                        "{address:?} has used every nonce and can fund nothing more"
                    ))
                })?;
                Ok((address, (next, credits)))
            })
            .collect::<Result<BTreeMap<_, _>, PlatformWalletError>>()?;
        let submitted = inputs
            .iter()
            .map(|(address, (nonce, _))| (*address, *nonce))
            .collect();
        let confirmed = identity
            .top_up_from_addresses_with_nonce_with_metadata(
                &self.sdk,
                inputs,
                address_signer,
                settings,
            )
            .await
            .map_err(top_up_error)?;
        bind_to_submitted_nonces(&submitted, confirmed)
    }
}

/// An SDK error of an address top-up as the wallet reports it: a nonce
/// race and an empty input map promoted to their own errors, every other
/// error kept typed under [`PlatformWalletError::Sdk`].
fn top_up_error(e: dash_sdk::Error) -> PlatformWalletError {
    preserve_signer_key_unavailable_or(e, |e| {
        promote_address_nonce_error(&e)
            .or_else(|| no_inputs_as_invalid_parameter(&e))
            .unwrap_or(PlatformWalletError::Sdk(e))
    })
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

/// The submitted top-up's result once its proof is bound to this
/// transition's address nonces (`submitted`).
///
/// An address nonce is used exactly once and only in order (Platform takes
/// stored + 1), and only this wallet can sign for its input addresses. A
/// proof showing every input address at or past the submitted nonce shows
/// those nonces consumed: this top-up executed, or a transition this wallet
/// signed with the same nonces did and this one never can. Either way it can
/// no longer execute later, so the result stands. A proof showing an input
/// address before its submitted nonce (or not at all) is a snapshot that does
/// not reflect this top-up, which may still execute: it is reported as the
/// SDK's unconfirmed outcome, never as done.
fn bind_to_submitted_nonces(
    submitted: &BTreeMap<PlatformAddress, AddressNonce>,
    confirmed: (AddressInfos, Credits, ResponseMetadata),
) -> Result<(AddressInfos, Credits, ResponseMetadata), PlatformWalletError> {
    for (address, nonce) in submitted {
        let proved = confirmed.0.get(address).and_then(|info| info.as_ref());
        if proved.is_none_or(|info| info.nonce < *nonce) {
            return Err(PlatformWalletError::Sdk(
                dash_sdk::Error::StateTransitionResultUnconfirmed(Box::new(
                    dash_sdk::Error::ExecutionNotProved(format!(
                        "the identity top-up proof shows {address:?} at nonce {:?}, before \
                         the submitted nonce {nonce}",
                        proved.map(|info| info.nonce)
                    )),
                )),
            ));
        }
    }
    Ok(confirmed)
}

#[cfg(test)]
mod nonce_binding_tests {
    use super::*;
    use dash_sdk::query_types::AddressInfo;

    const FIRST: PlatformAddress = PlatformAddress::P2pkh([1; 20]);
    const SECOND: PlatformAddress = PlatformAddress::P2pkh([2; 20]);

    fn proved(nonces: &[(PlatformAddress, Option<AddressNonce>)]) -> AddressInfos {
        AddressInfos::from_iter(nonces.iter().map(|(address, nonce)| {
            (
                *address,
                nonce.map(|nonce| AddressInfo {
                    address: *address,
                    nonce,
                    balance: 5,
                }),
            )
        }))
    }

    fn bind(
        proved_nonces: &[(PlatformAddress, Option<AddressNonce>)],
    ) -> Result<(AddressInfos, Credits, ResponseMetadata), PlatformWalletError> {
        let submitted = BTreeMap::from([(FIRST, 4), (SECOND, 9)]);
        bind_to_submitted_nonces(
            &submitted,
            (proved(proved_nonces), 777, ResponseMetadata::default()),
        )
    }

    /// A proof that shows an input address before the nonce this top-up
    /// was signed with is a snapshot from before it executed: the top-up
    /// may still execute, so it is reported unconfirmed, not done.
    #[test]
    fn should_report_a_snapshot_before_the_submitted_nonces_as_unconfirmed() {
        for proved_nonces in [
            [(FIRST, Some(3)), (SECOND, Some(8))],
            [(FIRST, Some(4)), (SECOND, Some(8))],
            [(FIRST, Some(4)), (SECOND, None)],
        ] {
            let result = bind(&proved_nonces);
            assert!(
                matches!(
                    &result,
                    Err(PlatformWalletError::Sdk(e)) if e.is_state_transition_result_unconfirmed()
                ),
                "{proved_nonces:?}: {result:?}"
            );
        }
    }

    /// Every input address at (or past) the nonce this top-up was signed
    /// with: the nonces it used are consumed, so the result stands.
    #[test]
    fn should_keep_a_result_whose_proof_shows_the_submitted_nonces_used() {
        for proved_nonces in [
            [(FIRST, Some(4)), (SECOND, Some(9))],
            [(FIRST, Some(5)), (SECOND, Some(9))],
        ] {
            let (_, balance, _) = bind(&proved_nonces).expect("bound to the submitted nonces");
            assert_eq!(balance, 777);
        }
    }
}
