//! Identity top-up convenience wrapper.
//!
//! The substantive top-up logic lives next to identity registration in
//! [`super::registration`] — both flows share the same funding
//! resolution, IS→CL fallback, and asset-lock cleanup machinery, so
//! keeping them in one module avoids duplication.
//!
//! This file just hosts the convenience wrapper `top_up_identity`
//! (defaults to wallet-balance funding) that delegates to the L2
//! orchestrator `top_up_identity_with_funding`.

use dpp::prelude::Identifier;

use dash_sdk::platform::transition::put_settings::PutSettings;

use crate::error::PlatformWalletError;
use crate::wallet::asset_lock::AssetLockFunding;

use super::*;

impl IdentityWallet {
    /// Top up an existing identity's credit balance from this wallet's
    /// UTXOs.
    ///
    /// Convenience wrapper around
    /// [`top_up_identity_with_funding`](Self::top_up_identity_with_funding)
    /// for the common case (`AssetLockFunding::FromWalletBalance`).
    ///
    /// # Arguments
    ///
    /// * `identity_id` - The identifier of the identity to top up.
    /// * `amount_duffs` - Amount of Dash (in duffs) to add.
    /// * `account_index` - Index addressing the standard (BIP44/BIP32)
    ///   families of the pooled funding set. The top-up draws from the
    ///   union of those two accounts and every DashPay contact-receiving
    ///   account (which span their own indices); CoinJoin stays out of
    ///   the pool, since mixing it with transparent coins in one
    ///   transaction would undo the mixing.
    /// * `asset_lock_signer` - External ECDSA signer that produces both
    ///   the funding-input P2PKH signatures during asset-lock build and
    ///   the consume-phase outer signature on the IdentityTopUp
    ///   transition. In Swift this is a `MnemonicResolverCoreSigner`
    ///   wrapping the Keychain resolver vtable.
    pub async fn top_up_identity<AS>(
        &self,
        identity_id: &Identifier,
        amount_duffs: u64,
        account_index: u32,
        asset_lock_signer: &AS,
        settings: Option<PutSettings>,
    ) -> Result<(), PlatformWalletError>
    where
        AS: ::key_wallet::signer::ExtendedPubKeySigner + Send + Sync,
    {
        self.top_up_identity_with_funding(
            identity_id,
            AssetLockFunding::FromWalletBalance {
                amount_duffs,
                account_index,
            },
            asset_lock_signer,
            settings,
        )
        .await?;
        Ok(())
    }
}
