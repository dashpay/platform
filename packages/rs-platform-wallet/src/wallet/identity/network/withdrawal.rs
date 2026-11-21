//! Withdraw credits from an identity.

use async_trait::async_trait;
use dashcore::Address as DashAddress;
use dpp::address_funds::AddressWitness;
use dpp::identity::Identity;
use dpp::identity::IdentityPublicKey;
use dpp::platform_value::BinaryData;
use dpp::prelude::Identifier;
use dpp::ProtocolError;

use dpp::identity::signer::Signer;

use dash_sdk::platform::transition::put_settings::PutSettings;
use dash_sdk::platform::transition::withdraw_from_identity::WithdrawFromIdentityWithMetadata;

use crate::error::PlatformWalletError;
use crate::{BlockTime, WithdrawalOutcome};

use super::signing_key::credit_signing_key;
use super::*;

// Borrowed-signer adapter — see `dpns.rs`/`transfer.rs` for the same
// pattern. Lets a `&S: Signer<IdentityPublicKey>` satisfy APIs that
// take an owned signer by generic bound.
pub(crate) struct SignerRef<'a, S: ?Sized>(pub(crate) &'a S);

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
// Withdrawal
// ---------------------------------------------------------------------------

impl IdentityWallet {
    /// Withdraw credits using an externally-supplied signer.
    ///
    /// Signing is routed through the supplied `&S: Signer<IdentityPublicKey>`.
    /// Required for external-signable wallets (no seed Rust-side) and the
    /// architecturally correct path per `swift-sdk/CLAUDE.md`.
    ///
    /// The identity is still looked up from the in-process
    /// `IdentityManager` so the local balance bookkeeping in
    /// `ManagedIdentity` stays consistent.
    pub async fn withdraw_credits_with_external_signer<S>(
        &self,
        identity_id: &Identifier,
        amount: u64,
        to_address: &DashAddress,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<(), PlatformWalletError>
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

        let (new_balance, metadata) = identity
            .withdraw_with_metadata(
                &self.sdk,
                Some(to_address.clone()),
                amount,
                None, // core_fee_per_byte
                Some(credit_signing_key(&identity, None, signer, true)?),
                SignerRef(signer),
                settings,
            )
            .await
            .map_err(|e| {
                // A balance refusal stays typed (FFI code 39) so the host can
                // word it; a structured key-unavailable signer failure is
                // preserved so the FFI boundary can still restore code 31;
                // only other failures get stringified into `InvalidIdentityData`.
                crate::error::promote_identity_insufficient_balance_or(e, |e| {
                    PlatformWalletError::InvalidIdentityData(format!(
                        "Failed to withdraw credits: {}",
                        e
                    ))
                })
            })?;

        {
            let mut wm = self.wallet_manager.write().await;
            let info_guard = wm.get_wallet_info_mut(&self.wallet_id).ok_or_else(|| {
                crate::error::PlatformWalletError::WalletNotFound(
                    "Wallet info not found in wallet manager".to_string(),
                )
            })?;
            if let Some(managed) = info_guard.identity_manager.identity_mut(identity_id) {
                managed.persist_confirmed_balance(
                    new_balance,
                    BlockTime::from(metadata),
                    &self.persister,
                );
            }
        }

        Ok(())
    }

    /// Withdraw credits using an externally-provided identity and signer.
    ///
    /// Unlike [`Self::withdraw_credits_with_external_signer`], this method does
    /// **not** look up the identity in the internal `IdentityManager`. Instead,
    /// the caller supplies the `Identity` object and a `Signer` implementation
    /// directly. This is useful when the caller manages identities outside of
    /// the platform-wallet `IdentityManager` (e.g. evo-tool's
    /// `QualifiedIdentity`).
    ///
    /// Returns the remaining credit balance after the withdrawal, with the
    /// block that proved it.
    #[allow(clippy::too_many_arguments)]
    pub async fn withdraw_credits_with_signer<S: Signer<IdentityPublicKey> + Send>(
        &self,
        identity: &Identity,
        to_address: Option<DashAddress>,
        amount: u64,
        signing_withdrawal_key_to_use: Option<&IdentityPublicKey>,
        signer: S,
        settings: Option<PutSettings>,
    ) -> Result<WithdrawalOutcome, dash_sdk::Error> {
        let signing_key =
            credit_signing_key(identity, signing_withdrawal_key_to_use, &signer, true)?;
        let (remaining_credits, metadata) = identity
            .withdraw_with_metadata(
                &self.sdk,
                to_address,
                amount,
                Some(1), // core_fee_per_byte
                Some(signing_key),
                signer,
                settings,
            )
            .await?;
        Ok(WithdrawalOutcome::proved(remaining_credits, metadata))
    }
}
