//! A wallet's tokens in token shielded pools: which pools it watches and
//! what the local scan found there ([`crate::wallet::shielded::token_pool`]).

use dpp::data_contract::TokenContractPosition;
use dpp::prelude::Identifier;

use crate::changeset::PlatformWalletPersistence;
use crate::error::PlatformWalletError;
use crate::manager::PlatformWalletManager;
use crate::wallet::platform_wallet::WalletId;
use crate::wallet::shielded::{ShieldedTokenPoolBalance, ShieldedTokenPoolWatch};

impl<P: PlatformWalletPersistence + 'static> PlatformWalletManager<P> {
    /// Watch the shielded pool of the token at `token_contract_position` of
    /// `data_contract_id` for `wallet_id`, so every later shielded sync scans
    /// it for the wallet's notes
    /// ([`NetworkShieldedCoordinator::watch_token_pool`]). Refuses a manager
    /// whose shielded support is not configured, a wallet not bound to it, and
    /// a token without a shielded pool or whose configuration cannot be read.
    ///
    /// [`NetworkShieldedCoordinator::watch_token_pool`]:
    ///     crate::wallet::shielded::NetworkShieldedCoordinator::watch_token_pool
    pub async fn watch_shielded_token_pool(
        &self,
        wallet_id: &WalletId,
        data_contract_id: Identifier,
        token_contract_position: TokenContractPosition,
    ) -> Result<ShieldedTokenPoolWatch, PlatformWalletError> {
        self.shielded_coordinator()
            .await
            .ok_or(PlatformWalletError::ShieldedNotBound)?
            .watch_token_pool(*wallet_id, data_contract_id, token_contract_position)
            .await
    }

    /// Stop watching `token_id`'s shielded pool for `wallet_id`, forgetting
    /// the wallet's notes there.
    pub async fn unwatch_shielded_token_pool(
        &self,
        wallet_id: &WalletId,
        token_id: Identifier,
    ) -> Result<(), PlatformWalletError> {
        self.shielded_coordinator()
            .await
            .ok_or(PlatformWalletError::ShieldedNotBound)?
            .unwatch_token_pool(*wallet_id, token_id)
            .await
    }

    /// `wallet_id`'s tokens in the shielded pools it watches, one entry per
    /// pool and account, from the local scan alone.
    pub async fn shielded_token_pool_balances(
        &self,
        wallet_id: &WalletId,
    ) -> Result<Vec<ShieldedTokenPoolBalance>, PlatformWalletError> {
        self.shielded_coordinator()
            .await
            .ok_or(PlatformWalletError::ShieldedNotBound)?
            .token_pool_balances(*wallet_id)
            .await
    }
}
