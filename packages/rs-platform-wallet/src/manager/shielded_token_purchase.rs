//! Buying a token at its direct purchase price from a wallet's shielded notes
//! (`TokenPurchaseFromShieldedPool`, type 28), and the shielded sync that
//! looks for it, requested once it may have reached the network.

use std::sync::Arc;

use dpp::balances::credits::TokenAmount;
use dpp::data_contract::TokenContractPosition;
use dpp::fee::Credits;
use dpp::prelude::Identifier;
use dpp::shielded::builder::OrchardProver;
use tracing::debug;

use crate::changeset::PlatformWalletPersistence;
use crate::error::PlatformWalletError;
use crate::manager::shielded_sync::ShieldedSyncManager;
use crate::manager::PlatformWalletManager;
use crate::wallet::platform_wallet::{PlatformWallet, WalletId};
use crate::wallet::shielded::{NetworkShieldedCoordinator, ShieldedTokenPurchaseOutcome};

/// What a shielded token purchase from one wallet needs from the manager: the
/// wallet, the shielded coordinator whose store holds its notes, and the
/// shielded sync that looks for the purchase. Taken from the manager in one
/// call ([`PlatformWalletManager::shielded_token_purchaser`]) and owned, so a
/// host runs the purchase, which proves for up to a minute, without holding
/// the manager.
pub struct ShieldedTokenPurchaser {
    wallet: Arc<PlatformWallet>,
    coordinator: Arc<NetworkShieldedCoordinator>,
    sync: Arc<ShieldedSyncManager>,
}

impl ShieldedTokenPurchaser {
    /// Buy `token_count` tokens of the token at `token_contract_position` of
    /// `data_contract_id` from `account`'s shielded notes, paying
    /// `total_agreed_price` plus the pool-paid fee
    /// ([`PlatformWallet::shielded_token_purchase_from_pool`]), then request a
    /// shielded sync when the purchase may have reached the network without
    /// being confirmed: accepted ([`ShieldedTokenPurchaseOutcome::Submitted`])
    /// or unconfirmed ([`PlatformWalletError::ShieldedSpendUnconfirmed`],
    /// [`PlatformWalletError::ShieldedBroadcastUnconfirmed`]). Once the
    /// purchase lands, a sync confirms its activity row and marks its notes
    /// spent; a sync also runs the redrive of its saved record. It is
    /// requested, not awaited: a pass already in flight takes its place.
    #[allow(clippy::too_many_arguments)]
    pub async fn purchase<P: OrchardProver>(
        &self,
        seed: &[u8],
        account: u32,
        data_contract_id: &Identifier,
        token_contract_position: TokenContractPosition,
        token_count: TokenAmount,
        total_agreed_price: Credits,
        prover: P,
    ) -> Result<ShieldedTokenPurchaseOutcome, PlatformWalletError> {
        let result = self
            .wallet
            .shielded_token_purchase_from_pool(
                &self.coordinator,
                seed,
                account,
                data_contract_id,
                token_contract_position,
                token_count,
                total_agreed_price,
                prover,
            )
            .await;
        if purchase_needs_shielded_sync(&result) {
            request_shielded_sync(&self.sync);
        }
        result
    }
}

impl<P: PlatformWalletPersistence + 'static> PlatformWalletManager<P> {
    /// What a shielded token purchase from `wallet_id` needs
    /// ([`ShieldedTokenPurchaser`]). Refuses a wallet the manager does not
    /// hold ([`PlatformWalletError::WalletNotFound`]) and a manager whose
    /// shielded support is not configured
    /// ([`PlatformWalletError::ShieldedNotBound`]).
    pub async fn shielded_token_purchaser(
        &self,
        wallet_id: &WalletId,
    ) -> Result<ShieldedTokenPurchaser, PlatformWalletError> {
        let wallet = self
            .get_wallet(wallet_id)
            .await
            .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(wallet_id)))?;
        let coordinator = self
            .shielded_coordinator()
            .await
            .ok_or(PlatformWalletError::ShieldedNotBound)?;
        Ok(ShieldedTokenPurchaser {
            wallet,
            coordinator,
            sync: self.shielded_sync_arc(),
        })
    }

    /// Buy a token from `wallet_id`'s shielded notes and request a shielded
    /// sync to look for it ([`ShieldedTokenPurchaser::purchase`]).
    #[allow(clippy::too_many_arguments)]
    pub async fn shielded_token_purchase_from_pool<Pr: OrchardProver>(
        &self,
        wallet_id: &WalletId,
        seed: &[u8],
        account: u32,
        data_contract_id: &Identifier,
        token_contract_position: TokenContractPosition,
        token_count: TokenAmount,
        total_agreed_price: Credits,
        prover: Pr,
    ) -> Result<ShieldedTokenPurchaseOutcome, PlatformWalletError> {
        self.shielded_token_purchaser(wallet_id)
            .await?
            .purchase(
                seed,
                account,
                data_contract_id,
                token_contract_position,
                token_count,
                total_agreed_price,
                prover,
            )
            .await
    }
}

/// Whether a token purchase that ended as `result` may be on the network
/// without being confirmed, so a shielded sync should look for it: accepted
/// (its proof does not show it executed) or unconfirmed. A refusal before
/// anything was sent needs none.
fn purchase_needs_shielded_sync(
    result: &Result<ShieldedTokenPurchaseOutcome, PlatformWalletError>,
) -> bool {
    matches!(
        result,
        Ok(ShieldedTokenPurchaseOutcome::Submitted)
            | Err(PlatformWalletError::ShieldedSpendUnconfirmed { .. })
            | Err(PlatformWalletError::ShieldedBroadcastUnconfirmed { .. })
    )
}

/// Start one shielded sync pass on `sync` without waiting for it. A pass
/// already in flight takes its place, and outside a runtime nothing starts:
/// a later scheduled pass looks for the purchase.
fn request_shielded_sync(sync: &Arc<ShieldedSyncManager>) {
    let Ok(runtime) = tokio::runtime::Handle::try_current() else {
        debug!("shielded token purchase: no runtime to start a sync on; a later pass looks for it");
        return;
    };
    let sync = Arc::clone(sync);
    runtime.spawn(async move {
        let summary = sync.sync_now(true).await;
        if summary.sync_unix_seconds == 0 {
            debug!(
                "shielded token purchase: sync skipped (a pass was in flight or shielded is \
                 unconfigured); a later pass looks for it"
            );
        } else {
            debug!(
                wallets = summary.wallet_results.len(),
                "shielded token purchase: sync pass completed"
            );
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An accepted purchase and an unconfirmed one may be on the network, so
    /// each requests a sync; a refusal before anything was sent requests none.
    #[test]
    fn should_request_a_sync_only_for_a_purchase_that_may_be_on_the_network() {
        assert!(purchase_needs_shielded_sync(&Ok(
            ShieldedTokenPurchaseOutcome::Submitted
        )));
        assert!(purchase_needs_shielded_sync(&Err(
            PlatformWalletError::ShieldedSpendUnconfirmed {
                operation: "token purchase from shielded pool",
                reason: "the result wait timed out".to_string(),
            }
        )));
        for refused in [
            PlatformWalletError::ShieldedInsufficientBalance {
                available: 1,
                required: 2,
            },
            PlatformWalletError::ShieldedBroadcastFailed("refused before delivery".to_string()),
            PlatformWalletError::TokenShieldedPoolDisabled {
                token_id: Identifier::from([1; 32]),
            },
            PlatformWalletError::WalletRemovedBeforeSend("00".to_string()),
        ] {
            assert!(
                !purchase_needs_shielded_sync(&Err(refused)),
                "a refusal before sending needs no sync"
            );
        }
    }
}
