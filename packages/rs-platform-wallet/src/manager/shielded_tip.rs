//! Sending a shielded tip from one of the manager's wallets, and the
//! shielded sync requested once the tip may have reached the network.

use std::sync::Arc;

use dpp::prelude::Identifier;
use dpp::shielded::builder::OrchardProver;
use tracing::debug;

use crate::changeset::PlatformWalletPersistence;
use crate::error::PlatformWalletError;
use crate::manager::shielded_sync::ShieldedSyncManager;
use crate::manager::PlatformWalletManager;
use crate::wallet::platform_wallet::{PlatformWallet, WalletId};
use crate::wallet::shielded::{ShieldedTipAdmission, ShieldedTipRecipient, ShieldedTipSource};

/// What a shielded tip from one wallet needs from the manager: the wallet,
/// the shielded coordinator whose store holds its notes, and the shielded
/// sync that resolves a tip whose outcome is not known. Taken from the
/// manager in one call ([`PlatformWalletManager::shielded_tip_sender`]) and
/// owned, so a host runs the send, which proves for up to a minute, without
/// holding the manager.
pub struct ShieldedTipSender {
    wallet: Arc<PlatformWallet>,
    sync: Arc<ShieldedSyncManager>,
}

impl ShieldedTipSender {
    /// Admit one tip from the wallet
    /// ([`PlatformWallet::admit_shielded_tip`]): taken before the seed is
    /// resolved, and passed to [`Self::send`].
    pub fn admit(&self) -> Result<ShieldedTipAdmission, PlatformWalletError> {
        self.wallet.admit_shielded_tip()
    }

    /// Send the tip ([`PlatformWallet::send_shielded_tip`]), then request a
    /// shielded sync when it may have left the wallet without being
    /// confirmed ([`PlatformWalletError::ShieldedSpendUnconfirmed`]): a sync
    /// sees it executed, sends its saved transition again, or proves it can
    /// no longer execute, which is what lets the recipient be tipped again.
    /// The sync is requested, not awaited: a pass already in flight takes
    /// its place.
    #[allow(clippy::too_many_arguments)]
    pub async fn send<P: OrchardProver>(
        &self,
        admission: ShieldedTipAdmission,
        seed: &[u8],
        source: ShieldedTipSource,
        username: &str,
        expected_recipient: &ShieldedTipRecipient,
        expected_identity: Option<&Identifier>,
        amount: u64,
        memo: [u8; 36],
        prover: P,
    ) -> Result<(), PlatformWalletError> {
        let result = self
            .wallet
            .send_shielded_tip(
                admission,
                seed,
                source,
                username,
                expected_recipient,
                expected_identity,
                amount,
                memo,
                prover,
            )
            .await;
        if tip_needs_shielded_sync(&result) {
            request_shielded_sync(&self.sync);
        }
        result
    }
}

impl<P: PlatformWalletPersistence + 'static> PlatformWalletManager<P> {
    /// What a shielded tip from `wallet_id` needs ([`ShieldedTipSender`]).
    /// Fails when the wallet is not registered
    /// ([`PlatformWalletError::WalletNotFound`]) or shielded support is not
    /// configured ([`PlatformWalletError::ShieldedNotBound`]).
    pub async fn shielded_tip_sender(
        &self,
        wallet_id: &WalletId,
    ) -> Result<ShieldedTipSender, PlatformWalletError> {
        let wallet = self
            .get_wallet(wallet_id)
            .await
            .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(wallet_id)))?;
        if self.shielded_coordinator().await.is_none() {
            return Err(PlatformWalletError::ShieldedNotBound);
        }
        Ok(ShieldedTipSender {
            wallet,
            sync: self.shielded_sync_arc(),
        })
    }
}

/// Whether a tip that ended as `result` may be on the network without being
/// confirmed, so a shielded sync should look for it. A refusal before
/// anything was sent needs none.
fn tip_needs_shielded_sync(result: &Result<(), PlatformWalletError>) -> bool {
    matches!(
        result,
        Err(PlatformWalletError::ShieldedSpendUnconfirmed { .. })
    )
}

/// Start one shielded sync pass on `sync` without waiting for it. A pass
/// already in flight takes its place, and outside a runtime nothing starts:
/// a later scheduled pass looks for the tip.
fn request_shielded_sync(sync: &Arc<ShieldedSyncManager>) {
    let Ok(runtime) = tokio::runtime::Handle::try_current() else {
        debug!("shielded tip: no runtime to start a sync on; a later pass looks for it");
        return;
    };
    let sync = Arc::clone(sync);
    runtime.spawn(async move {
        let summary = sync.sync_now(true).await;
        debug!(
            ran = summary.sync_unix_seconds != 0,
            "shielded tip: sync requested after an unconfirmed send"
        );
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_request_a_sync_only_for_a_tip_that_may_have_left_the_wallet() {
        assert!(tip_needs_shielded_sync(&Err(
            PlatformWalletError::ShieldedSpendUnconfirmed {
                operation: "transfer",
                reason: "result wait timed out".to_string(),
            }
        )));
        for settled in [
            Ok(()),
            Err(PlatformWalletError::ShieldedTipPending),
            Err(PlatformWalletError::ShieldedBroadcastFailed(
                "no address".to_string(),
            )),
        ] {
            assert!(!tip_needs_shielded_sync(&settled), "{settled:?}");
        }
    }
}
