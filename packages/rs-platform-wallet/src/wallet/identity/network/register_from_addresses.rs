//! Register a new identity funded by platform addresses.

use std::collections::BTreeMap;

use dpp::identity::accessors::{IdentityGettersV0, IdentitySettersV0};
use dpp::identity::signer::Signer;
use dpp::identity::Identity;
use dpp::identity::IdentityPublicKey;

use dash_sdk::platform::transition::put_identity::PutIdentity;
use dash_sdk::platform::transition::put_settings::PutSettings;

use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;

use crate::error::PlatformWalletError;

use super::*;

// ---------------------------------------------------------------------------
// Register identity from platform addresses
// ---------------------------------------------------------------------------

impl IdentityWallet {
    /// Register a new identity funded by Platform-address balances.
    ///
    /// Address-funded identity creation uses the
    /// `IdentityCreateFromAddressesTransition` on the SDK side
    /// (`put_with_address_funding`). Unlike the asset-lock path, no
    /// Core-chain transaction is built — the inputs are existing
    /// credits already committed to Platform under the caller's
    /// `PlatformPayment` (DIP-17) addresses.
    ///
    /// This method is a thin wrapper over `put_with_address_funding`
    /// plus `IdentityManager::add_identity`. The caller is
    /// responsible for constructing the placeholder `Identity` (its
    /// DIP-9 auth pubkeys) and both signers; the wallet struct
    /// carries no key material.
    ///
    /// Returns the registered identity alongside the proof-attested
    /// post-spend `AddressInfos` for the funding addresses. Prefer the
    /// composite [`PlatformWallet::register_from_addresses`], which feeds
    /// the returned `AddressInfos` through
    /// [`PlatformAddressWallet::reconcile_address_infos`] so the spent
    /// balances and advanced nonces are reflected locally without waiting
    /// for the next BLAST sync round.
    ///
    /// [`PlatformWallet::register_from_addresses`]:
    /// crate::wallet::PlatformWallet::register_from_addresses
    /// [`PlatformAddressWallet::reconcile_address_infos`]:
    /// crate::wallet::PlatformAddressWallet::reconcile_address_infos
    ///
    /// # Arguments
    ///
    /// * `identity` — fully-constructed placeholder identity (with
    ///   every auth pubkey already populated at its DIP-9 path, and
    ///   `id = Identifier::default()` — the SDK recomputes it from
    ///   the input-address map).
    /// * `inputs` — contributing addresses, each with the
    ///   `Credits` amount to spend. The on-chain nonces are fetched
    ///   automatically by the SDK right before submit (see
    ///   [`PutIdentity::put_with_address_funding_fetching_nonces`]),
    ///   so the caller doesn't need to maintain a nonce cache.
    /// * `output` — optional refund-style output pinning change to an
    ///   explicit platform address. `None` means no refund (any
    ///   residual goes into the new identity's balance).
    /// * `identity_index` — BIP-9 identity index (hardened) in the
    ///   key tree. Recorded on the `ManagedIdentity` stored in the
    ///   local `IdentityManager`; the caller's signer is expected to
    ///   have already bound it when deriving auth keys.
    /// * `identity_signer` — produces ECDSA signatures for keys that
    ///   appear on `identity.public_keys()`. Construction is the
    ///   caller's concern — seed-backed, hardware, remote signer,
    ///   whatever — the wallet struct carries no key material.
    /// * `input_address_signer` — produces ECDSA signatures for the
    ///   input [`PlatformAddress`]es. Same rationale as above.
    #[allow(clippy::too_many_arguments)]
    pub async fn register_from_addresses<
        IS: Signer<IdentityPublicKey>,
        AS: Signer<PlatformAddress> + Send + Sync,
    >(
        &self,
        identity: &Identity,
        inputs: BTreeMap<PlatformAddress, Credits>,
        output: Option<(PlatformAddress, Credits)>,
        identity_index: u32,
        identity_signer: &IS,
        input_address_signer: &AS,
        settings: Option<PutSettings>,
    ) -> Result<(Identity, dash_sdk::query_types::AddressInfos, u64), PlatformWalletError> {
        if inputs.is_empty() {
            return Err(PlatformWalletError::InvalidIdentityData(
                "At least one input address is required".to_string(),
            ));
        }

        // Route through the auto-fetching SDK variant so the caller
        // doesn't need to maintain its own nonce cache — Platform is
        // always the source of truth at submit time.
        let (mut registered_identity, address_infos, proof_height) = identity
            .put_with_address_funding_fetching_nonces(
                &self.sdk,
                inputs,
                output,
                identity_signer,
                input_address_signer,
                settings,
            )
            .await
            .map_err(|e| {
                crate::error::promote_address_nonce_error(&e).unwrap_or_else(|| {
                    PlatformWalletError::InvalidIdentityData(format!(
                        "Failed to register identity from addresses: {}",
                        e
                    ))
                })
            })?;

        // The SDK return path for `put_with_address_funding_fetching_nonces`
        // can hand back an `Identity` whose `public_keys` map is empty
        // (the pre-broadcast stub doesn't echo the registered keys
        // back). The caller-built placeholder we just submitted IS the
        // canonical record of what the registration transition put on
        // chain — its `public_keys` was serialized into the state
        // transition that Platform just accepted. Copy it onto the
        // returned identity so the in-memory `ManagedIdentity` (and
        // every downstream auth-key check, e.g. the DPNS
        // HIGH/CRITICAL gate) sees the same set of keys without
        // waiting for the next identity-fetch round to repopulate
        // them. The state transition itself signed those exact keys,
        // so id (`Identifier`) reproducibility is preserved.
        if registered_identity.public_keys().is_empty() {
            registered_identity.set_public_keys(identity.public_keys().clone());
        }
        let identity = registered_identity;

        // Step 3 (best-effort): add the identity to the local manager
        // (with its HD index) so subsequent operations route through it.
        //
        // Platform has ALREADY accepted the registration, so a local
        // persistence failure must NOT propagate as `Err` — that would
        // suppress the `(identity, address_infos)` return the caller
        // needs to reconcile the spent funding-address balances, leaving
        // them stale even though the identity exists on chain. A missed
        // local add self-heals on the next identity re-sync. This mirrors
        // the sibling `transfer_credits_to_addresses` / top-up flows,
        // which likewise treat post-acceptance local bookkeeping as
        // best-effort.
        {
            let mut wm = self.wallet_manager.write().await;
            match wm.get_wallet_info_mut(&self.wallet_id) {
                Some(info) => {
                    if let Err(e) = info.identity_manager.add_identity(
                        identity.clone(),
                        identity_index,
                        self.wallet_id,
                        &self.persister,
                    ) {
                        tracing::warn!(
                            error = %e,
                            identity_id = %identity.id(),
                            "register_from_addresses: identity registered on Platform but \
                             local add_identity failed; returning the registered identity \
                             anyway so address balances can reconcile"
                        );
                    }
                }
                None => {
                    tracing::warn!(
                        identity_id = %identity.id(),
                        "register_from_addresses: identity registered on Platform but wallet \
                         info was not found locally; skipping local persistence"
                    );
                }
            }
        }

        // The spent platform-address balances are reconciled by the
        // composite `PlatformWallet::register_from_addresses`, which routes
        // the returned `AddressInfos` (pinned at `proof_height`) through
        // the platform-address wallet's shared reconciliation seam.
        Ok((identity, address_infos, proof_height))
    }
}
