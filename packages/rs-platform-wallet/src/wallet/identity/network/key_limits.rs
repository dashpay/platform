//! Raising the limits of one of an identity's authentication keys (protocol version 14).
//!
//! A key registered with a budget or an expiry is topped up, or has its expiry moved later,
//! with an `IdentityKeyLimitsUpdate` transition rather than replaced. The wallet computes the
//! absolute values from the key it holds, signs with the identity's MASTER key (or a CRITICAL
//! authentication key without limits and without contract bounds) through the supplied
//! signer, and lays the key as stored after the update over its cache so the client's key
//! row follows through the persister.

use dash_sdk::platform::transition::put_settings::PutSettings;
use dash_sdk::platform::transition::update_identity_key_limits::{
    raised_key_limits, UpdateIdentityKeyLimits,
};
use dpp::fee::Credits;
use dpp::identity::signer::Signer;
use dpp::identity::{IdentityPublicKey, KeyID, TimestampMillis};
use dpp::prelude::Identifier;

use super::update::SignerRef;
use super::*;
use crate::error::PlatformWalletError;

impl IdentityWallet {
    /// Raise the limits of the key `key_id` of `identity_id`: add `add_budget` credits to its
    /// total budget (and to what is left of it), and move its expiry to `expires_at`. At least
    /// one must be given. What consensus would refuse and charge for is refused here first: a
    /// limit the key does not have, a zero top-up, an expiry that is not later.
    ///
    /// Signing is routed through the supplied `&S: Signer<IdentityPublicKey>`, which must hold
    /// the identity's MASTER key or a CRITICAL authentication key without limits and without
    /// contract bounds. No identity revision is claimed or bumped, so the cached identity only
    /// needs to hold the signing key.
    ///
    /// Resolves to the key as stored after the update, which is also laid over the cached
    /// identity and upserted through the persister, as
    /// [`Self::update_identity_with_external_signer`] does for an added key.
    pub async fn update_identity_key_limits_with_external_signer<S>(
        &self,
        identity_id: &Identifier,
        key_id: KeyID,
        add_budget: Option<Credits>,
        expires_at: Option<TimestampMillis>,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<IdentityPublicKey, PlatformWalletError>
    where
        S: Signer<IdentityPublicKey> + Send + Sync,
    {
        let identity = {
            let wm = self.wallet_manager.read().await;
            let info = wm.get_wallet_info(&self.wallet_id).ok_or_else(|| {
                PlatformWalletError::WalletNotFound(
                    "Wallet info not found in wallet manager".to_string(),
                )
            })?;
            info.identity_manager
                .identity(identity_id)
                .map(|m| m.identity.clone())
                .ok_or(PlatformWalletError::IdentityNotFound(*identity_id))?
        };

        let (total_budget, expires_at) =
            raised_key_limits(&identity, key_id, add_budget, expires_at)
                .map_err(|e| PlatformWalletError::InvalidIdentityData(e.to_string()))?;

        let updated_key = identity
            .update_key_limits(
                &self.sdk,
                key_id,
                total_budget,
                expires_at,
                None,
                SignerRef(signer),
                settings,
            )
            .await?;

        // Post-broadcast local apply: the cached key carries the raised limits and the
        // client's key row follows through the persister.
        {
            let mut wm = self.wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&self.wallet_id).ok_or_else(|| {
                PlatformWalletError::WalletNotFound(
                    "Wallet info not found in wallet manager".to_string(),
                )
            })?;
            if let Some(managed) = info.identity_manager.managed_identity_mut(identity_id) {
                managed
                    .replace_key(updated_key.clone(), &self.persister)
                    .map_err(|e| {
                        PlatformWalletError::Persistence(format!(
                            "identity key not persisted after the key limits update: {e}"
                        ))
                    })?;
            }
        }

        Ok(updated_key)
    }
}
