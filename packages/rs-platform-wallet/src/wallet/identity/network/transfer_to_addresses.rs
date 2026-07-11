//! Transfer identity credits to platform addresses.

use std::collections::BTreeMap;

use async_trait::async_trait;
use dpp::address_funds::AddressWitness;
use dpp::identity::accessors::IdentitySettersV0;
use dpp::identity::signer::Signer;
use dpp::identity::IdentityPublicKey;
use dpp::platform_value::BinaryData;
use dpp::prelude::Identifier;
use dpp::ProtocolError;

use dash_sdk::platform::transition::put_settings::PutSettings;
use dash_sdk::platform::transition::transfer_to_addresses::TransferToAddresses;

use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;

use crate::error::PlatformWalletError;

use super::*;

// Borrowed-signer adapter — see `dpns.rs` for the pattern.
struct SignerRef<'a, S: ?Sized>(&'a S);

impl<'a, S: ?Sized> std::fmt::Debug for SignerRef<'a, S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SignerRef")
    }
}

#[async_trait]
impl<'a, K, S> Signer<K> for SignerRef<'a, S>
where
    K: Send + Sync,
    S: Signer<K> + ?Sized + Send + Sync,
{
    async fn sign(&self, key: &K, data: &[u8]) -> Result<BinaryData, ProtocolError> {
        self.0.sign(key, data).await
    }

    async fn sign_create_witness(
        &self,
        key: &K,
        data: &[u8],
    ) -> Result<AddressWitness, ProtocolError> {
        self.0.sign_create_witness(key, data).await
    }

    fn can_sign_with(&self, key: &K) -> bool {
        self.0.can_sign_with(key)
    }
}

// ---------------------------------------------------------------------------
// Transfer credits to platform addresses
// ---------------------------------------------------------------------------

impl IdentityWallet {
    /// Transfer credits from an identity to multiple platform
    /// addresses using an externally-supplied signer.
    ///
    /// Signing is routed through the supplied `&S: Signer<IdentityPublicKey>`.
    /// Required for external-signable wallets.
    ///
    /// Returns the proof-attested post-transfer `AddressInfos` for the
    /// recipient addresses alongside the sender's new balance. Prefer the
    /// composite [`PlatformWallet::transfer_credits_to_addresses_with_external_signer`],
    /// which feeds the returned `AddressInfos` through
    /// [`PlatformAddressWallet::reconcile_address_infos`] so recipients
    /// owned by this wallet see their new balance immediately instead of
    /// waiting for the next BLAST sync round.
    ///
    /// [`PlatformWallet::transfer_credits_to_addresses_with_external_signer`]:
    /// crate::wallet::PlatformWallet::transfer_credits_to_addresses_with_external_signer
    /// [`PlatformAddressWallet::reconcile_address_infos`]:
    /// crate::wallet::PlatformAddressWallet::reconcile_address_infos
    pub async fn transfer_credits_to_addresses_with_external_signer<S>(
        &self,
        identity_id: &Identifier,
        recipient_addresses: BTreeMap<PlatformAddress, Credits>,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<(dash_sdk::query_types::AddressInfos, Credits, u64), PlatformWalletError>
    where
        S: Signer<IdentityPublicKey> + Send + Sync,
    {
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

        let (address_infos, new_balance, proof_height) = identity
            .transfer_credits_to_addresses(
                &self.sdk,
                recipient_addresses,
                None, // signing_transfer_key_to_use
                &SignerRef(signer),
                settings,
            )
            .await
            .map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to transfer credits to addresses: {}",
                    e
                ))
            })?;

        {
            let mut wm = self.wallet_manager.write().await;
            let info_guard = wm.get_wallet_info_mut(&self.wallet_id).ok_or_else(|| {
                crate::error::PlatformWalletError::WalletNotFound(
                    "Wallet info not found in wallet manager".to_string(),
                )
            })?;
            if let Some(managed) = info_guard
                .identity_manager
                .managed_identity_mut(identity_id)
            {
                managed.identity.set_balance(new_balance);
                if let Err(e) = self.persister.store(managed.snapshot_changeset().into()) {
                    tracing::error!(
                        identity = %identity_id,
                        error = %e,
                        "Failed to persist identity balance update after \
                         transfer_to_addresses (external signer)"
                    );
                }
            }
        }

        // Recipient platform-address balances are reconciled by the
        // composite `PlatformWallet::transfer_credits_to_addresses_with_external_signer`,
        // which routes the returned `AddressInfos` through the
        // platform-address wallet's shared reconciliation seam.
        Ok((address_infos, new_balance, proof_height))
    }
}
