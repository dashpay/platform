//! Transfer credits between identities.

use async_trait::async_trait;
use dpp::address_funds::AddressWitness;
use dpp::identity::accessors::IdentitySettersV0;
use dpp::identity::Identity;
use dpp::identity::IdentityPublicKey;
use dpp::platform_value::BinaryData;
use dpp::prelude::Identifier;
use dpp::ProtocolError;

use dpp::identity::signer::Signer;

use dash_sdk::platform::transition::put_settings::PutSettings;
use dash_sdk::platform::transition::transfer::TransferToIdentity;

use crate::error::PlatformWalletError;

use super::*;

// Local borrowed-signer adapter — mirrors the one in `dpns.rs`. Lets
// callers hand a `&S: Signer<IdentityPublicKey>` into APIs that demand
// an owned signer by generic bound. Same rationale: `Signer<K>` is
// not implemented for `&T`, and we do not want to force callers to
// clone or `Arc`-wrap their `KeychainSigner` per call.
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
// Credit transfer
// ---------------------------------------------------------------------------

impl IdentityWallet {
    /// Transfer credits using an externally-supplied signer.
    ///
    /// Signing is routed through the supplied `&S: Signer<IdentityPublicKey>`.
    /// Required for external-signable wallets (no seed Rust-side, e.g.
    /// watch-only wallets where the seed lives in iOS Keychain) and
    /// the architecturally correct path per `swift-sdk/CLAUDE.md`.
    ///
    /// The identity is still looked up from the in-process
    /// `IdentityManager` so the local balance bookkeeping in
    /// `ManagedIdentity` stays consistent with on-chain reality and
    /// the persister observes the new balance via the snapshot
    /// changeset.
    pub async fn transfer_credits_with_external_signer<S>(
        &self,
        from_id: &Identifier,
        to_id: &Identifier,
        amount: u64,
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
                .identity(from_id)
                .map(|m| m.identity.clone())
                .ok_or(PlatformWalletError::IdentityNotFound(*from_id))?
        };

        let (sender_balance, _receiver_balance) = identity
            .transfer_credits(
                &self.sdk,
                *to_id,
                amount,
                None, // signing_transfer_key_to_use
                SignerRef(signer),
                settings,
            )
            .await
            .map_err(|e| {
                // Preserve a structured key-unavailable signer failure so the
                // FFI boundary can still restore code 31; only genuine
                // operation failures get stringified into `InvalidIdentityData`.
                crate::error::preserve_signer_key_unavailable_or(e, |e| {
                    PlatformWalletError::InvalidIdentityData(format!(
                        "Failed to transfer credits: {}",
                        e
                    ))
                })
            })?;

        // Update the sender's balance and queue the snapshot so the
        // change survives relaunch + reaches Swift via the persister.
        {
            let mut wm = self.wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&self.wallet_id).ok_or_else(|| {
                crate::error::PlatformWalletError::WalletNotFound(
                    "Wallet info not found in wallet manager".to_string(),
                )
            })?;
            if let Some(managed) = info.identity_manager.managed_identity_mut(from_id) {
                managed.identity.set_balance(sender_balance);
                if let Err(e) = self.persister.store(managed.snapshot_changeset().into()) {
                    tracing::error!(
                        identity = %from_id,
                        error = %e,
                        "Failed to persist identity balance update after transfer (external signer)"
                    );
                }
            }
        }

        Ok(())
    }

    /// Transfer credits using an externally-provided identity and signer.
    ///
    /// Unlike [`Self::transfer_credits_with_external_signer`], this method does
    /// **not** look up the identity in the internal `IdentityManager`. The
    /// caller supplies the `Identity` and a `Signer` directly.
    ///
    /// Returns `(sender_balance, receiver_balance)` after the transfer.
    pub async fn transfer_credits_with_signer<S: Signer<IdentityPublicKey> + Send>(
        &self,
        identity: &Identity,
        to_id: Identifier,
        amount: u64,
        signing_transfer_key_to_use: Option<&IdentityPublicKey>,
        signer: S,
        settings: Option<PutSettings>,
    ) -> Result<(u64, u64), dash_sdk::Error> {
        identity
            .transfer_credits(
                &self.sdk,
                to_id,
                amount,
                signing_transfer_key_to_use,
                signer,
                settings,
            )
            .await
    }
}
