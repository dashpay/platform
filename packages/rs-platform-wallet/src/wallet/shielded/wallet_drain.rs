//! Read-only quote for pooled Core → Shielded drains.

use std::collections::HashSet;
use std::sync::Arc;

use dashcore::blockdata::transaction::special_transaction::asset_lock::AssetLockPayload;
use dashcore::blockdata::transaction::special_transaction::TransactionPayload;
use dashcore::hashes::Hash;
use dashcore::{PubkeyHash, ScriptBuf, TxOut};
use dpp::balances::credits::CREDITS_PER_DUFF;
use key_wallet::wallet::managed_wallet_info::coin_selection::{SelectionError, SelectionStrategy};
use key_wallet::wallet::managed_wallet_info::fee::FeeRate;
use key_wallet::wallet::managed_wallet_info::transaction_builder::{
    BuilderError, TransactionBuilder,
};
use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;

use crate::error::PlatformWalletError;
use crate::wallet::asset_lock::manager::DEFAULT_FEE_PER_KB;
use crate::wallet::asset_lock::{
    AssetLockOperationContext, AssetLockOperationRegistration, AssetLockOperationSnapshot,
};
use crate::wallet::core::resolved_funding_accounts;
use crate::wallet::PlatformWallet;
use crate::ASSET_LOCK_FUNDING_SOURCES;

use super::fund_from_asset_lock::shield_from_asset_lock_num_actions;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WalletDrainPreviewStatus {
    Ready,
    InsufficientFunds,
    InputLimitMayBeExceeded,
}

/// Optimistic preview: the account reservation sets are private to key-wallet,
/// so other operations' reserved inputs may be counted. It never reserves,
/// signs, consumes a change/funding address, or promises a minimum amount.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WalletDrainPreview {
    pub status: WalletDrainPreviewStatus,
    pub candidate_input_count: usize,
    pub candidate_balance_duffs: u64,
    pub pool_fee_credits: u64,
    pub estimated_l1_fee_duffs: Option<u64>,
    pub estimated_recipient_credits: Option<u64>,
}

impl PlatformWallet {
    /// Make a queued drain visible to status readers before spawning its worker.
    pub fn register_wallet_drain_operation(
        &self,
        operation: Arc<AssetLockOperationContext>,
    ) -> AssetLockOperationRegistration {
        self.asset_locks.register_wallet_drain_operation(operation)
    }

    /// Durable row, live row and active context only. Never touches the
    /// network and never waits on the shield guard.
    pub async fn shielded_wallet_drain_operation_status(
        &self,
        operation_id: [u8; 16],
    ) -> Result<AssetLockOperationSnapshot, PlatformWalletError> {
        self.asset_locks
            .wallet_drain_operation_status(operation_id)
            .await
    }

    pub async fn shielded_wallet_drain_preview(
        &self,
        account_index: u32,
        dummy_outputs: usize,
    ) -> Result<WalletDrainPreview, PlatformWalletError> {
        let pool_fee_credits = self
            .shield_from_asset_lock_pool_fee(shield_from_asset_lock_num_actions(dummy_outputs))?;
        let (candidates, height) = {
            let manager = self.wallet_manager.read().await;
            let (wallet, info) =
                manager
                    .get_wallet_and_info(&self.wallet_id())
                    .ok_or_else(|| {
                        PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id()))
                    })?;
            let height = info.core_wallet.last_processed_height();
            let accounts = &info.core_wallet.accounts;
            let funding = resolved_funding_accounts(
                accounts,
                wallet,
                &ASSET_LOCK_FUNDING_SOURCES,
                account_index,
                false,
            )?;
            let mut seen = HashSet::new();
            let candidates = funding
                .iter()
                .filter_map(|account| accounts.funds_account(account))
                .flat_map(|account| account.utxos.values())
                .filter(|utxo| {
                    (utxo.is_confirmed || utxo.is_instantlocked) && utxo.is_spendable(height)
                })
                .filter(|utxo| seen.insert(utxo.outpoint))
                .cloned()
                .collect::<Vec<_>>();
            (candidates, height)
        };
        let candidate_balance_duffs = candidates.iter().try_fold(0u64, |total, input| {
            total.checked_add(input.value()).ok_or_else(|| {
                PlatformWalletError::AssetLockTransaction(
                    "wallet drain candidate balance overflows duffs".into(),
                )
            })
        })?;
        let mut preview = WalletDrainPreview {
            status: WalletDrainPreviewStatus::InsufficientFunds,
            candidate_input_count: candidates.len(),
            candidate_balance_duffs,
            pool_fee_credits,
            estimated_l1_fee_duffs: None,
            estimated_recipient_credits: None,
        };
        // Only a P2PKH placeholder is needed for the payload size. No wallet
        // key or address is touched. With no add_funding calls this local
        // builder carries no live reservation sets.
        let built = TransactionBuilder::new()
            .set_fee_rate(FeeRate::new(DEFAULT_FEE_PER_KB))
            .set_current_height(height)
            .set_selection_strategy(SelectionStrategy::All)
            .require_final_inputs()
            .set_special_payload(TransactionPayload::AssetLockPayloadType(
                AssetLockPayload::new(vec![TxOut {
                    value: 0,
                    script_pubkey: ScriptBuf::new_p2pkh(&PubkeyHash::all_zeros()),
                }]),
            ))
            .add_inputs(candidates)
            .build_unsigned_reserved();
        match built {
            Ok((_, fee, _)) => {
                preview.estimated_l1_fee_duffs = Some(fee);
                let recipient = candidate_balance_duffs
                    .checked_sub(fee)
                    .and_then(|duffs| duffs.checked_mul(CREDITS_PER_DUFF))
                    .and_then(|credits| credits.checked_sub(pool_fee_credits))
                    .filter(|credits| *credits > 0);
                preview.estimated_recipient_credits = recipient;
                if recipient.is_some() {
                    preview.status = WalletDrainPreviewStatus::Ready;
                }
            }
            Err(BuilderError::TooManyInputs { .. }) => {
                preview.status = WalletDrainPreviewStatus::InputLimitMayBeExceeded;
            }
            Err(
                BuilderError::InsufficientFunds { .. }
                | BuilderError::NoInputs
                | BuilderError::CoinSelection(
                    SelectionError::NoUtxosAvailable | SelectionError::InsufficientFunds { .. },
                ),
            ) => {}
            Err(error) => {
                return Err(PlatformWalletError::AssetLockTransaction(format!(
                    "wallet drain preview failed: {error}"
                )))
            }
        }
        Ok(preview)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::broadcaster::SpvBroadcaster;
    use crate::events::PlatformEventManager;
    use crate::spv::SpvRuntime;
    use crate::test_support::{
        funded_wallet_manager_with_contact, funded_wallet_manager_with_outputs,
    };
    use crate::wallet::persister::NoPlatformPersistence;
    use key_wallet::account::StandardAccountType;
    use std::sync::Arc;
    use tokio::sync::Notify;

    async fn wallet_with_outputs(outputs: &[u64]) -> PlatformWallet {
        let (manager, id, generation, _) =
            funded_wallet_manager_with_outputs(StandardAccountType::BIP44Account, outputs).await;
        let spv = Arc::new(SpvRuntime::new(
            manager.clone(),
            Arc::new(PlatformEventManager::new(Vec::new())),
        ));
        PlatformWallet::new(
            Arc::new(dash_sdk::SdkBuilder::new_mock().build().unwrap()),
            id,
            manager,
            generation,
            Arc::new(Notify::new()),
            Arc::new(NoPlatformPersistence),
            Arc::new(SpvBroadcaster::new(spv)),
        )
    }

    #[tokio::test]
    async fn should_wallet_drain_preview_report_fees_and_leave_wallet_unchanged() {
        let wallet = wallet_with_outputs(&[100_000, 200_000]).await;
        let before = {
            let manager = wallet.wallet_manager.read().await;
            format!(
                "{:?}",
                manager
                    .get_wallet_info(&wallet.wallet_id())
                    .unwrap()
                    .core_wallet
            )
        };
        let quote = wallet.shielded_wallet_drain_preview(0, 0).await.unwrap();
        assert_eq!(quote.status, WalletDrainPreviewStatus::Ready);
        assert_eq!(quote.candidate_balance_duffs, 300_000);
        assert_eq!(quote.candidate_input_count, 2);
        assert_eq!(
            quote.estimated_recipient_credits,
            Some(
                (300_000 - quote.estimated_l1_fee_duffs.unwrap()) * CREDITS_PER_DUFF
                    - quote.pool_fee_credits
            )
        );
        assert_eq!(
            wallet.shielded_wallet_drain_preview(0, 0).await.unwrap(),
            quote
        );
        let after = {
            let manager = wallet.wallet_manager.read().await;
            format!(
                "{:?}",
                manager
                    .get_wallet_info(&wallet.wallet_id())
                    .unwrap()
                    .core_wallet
            )
        };
        assert_eq!(
            before, after,
            "preview must not consume addresses, reserve coins or mutate state"
        );
    }

    #[tokio::test]
    async fn should_wallet_drain_preview_distinguish_limit_and_insufficient_funds() {
        let tiny = wallet_with_outputs(&[1]).await;
        assert_eq!(
            tiny.shielded_wallet_drain_preview(0, 0)
                .await
                .unwrap()
                .status,
            WalletDrainPreviewStatus::InsufficientFunds
        );
        let large = wallet_with_outputs(&vec![10_000; 501]).await;
        let quote = large.shielded_wallet_drain_preview(0, 0).await.unwrap();
        assert_eq!(
            quote.status,
            WalletDrainPreviewStatus::InputLimitMayBeExceeded
        );
        assert_eq!(quote.candidate_input_count, 501);
        assert_eq!(quote.estimated_l1_fee_duffs, None);
        assert_eq!(quote.estimated_recipient_credits, None);
    }

    #[tokio::test]
    async fn should_wallet_drain_preview_exclude_nonfinal_and_locked_inputs() {
        let wallet = wallet_with_outputs(&[100_000, 200_000, 300_000]).await;
        {
            let mut manager = wallet.wallet_manager.write().await;
            let info = manager.get_wallet_info_mut(&wallet.wallet_id()).unwrap();
            let account = info
                .core_wallet
                .accounts
                .standard_bip44_accounts
                .get_mut(&0)
                .unwrap();
            for utxo in account.utxos.values_mut() {
                if utxo.value() == 200_000 {
                    utxo.is_confirmed = false;
                    utxo.is_instantlocked = false;
                }
                if utxo.value() == 300_000 {
                    utxo.is_locked = true;
                }
            }
        }
        let quote = wallet.shielded_wallet_drain_preview(0, 0).await.unwrap();
        assert_eq!(quote.candidate_input_count, 1);
        assert_eq!(quote.candidate_balance_duffs, 100_000);
    }

    #[tokio::test]
    async fn should_wallet_drain_preview_include_dashpay_and_newly_final_funds() {
        let (manager, id, generation, _, _) =
            funded_wallet_manager_with_contact(&[100_000], &[200_000]).await;
        let spv = Arc::new(SpvRuntime::new(
            manager.clone(),
            Arc::new(PlatformEventManager::new(Vec::new())),
        ));
        let wallet = PlatformWallet::new(
            Arc::new(dash_sdk::SdkBuilder::new_mock().build().unwrap()),
            id,
            manager.clone(),
            generation,
            Arc::new(Notify::new()),
            Arc::new(NoPlatformPersistence),
            Arc::new(SpvBroadcaster::new(spv)),
        );
        assert_eq!(
            wallet
                .shielded_wallet_drain_preview(0, 0)
                .await
                .unwrap()
                .candidate_balance_duffs,
            300_000
        );
        {
            let mut manager = manager.write().await;
            let info = manager.get_wallet_info_mut(&id).unwrap();
            for account in info
                .core_wallet
                .accounts
                .dashpay_receival_accounts
                .values_mut()
            {
                for utxo in account.utxos.values_mut() {
                    utxo.is_confirmed = false;
                    utxo.is_instantlocked = false;
                }
            }
        }
        assert_eq!(
            wallet
                .shielded_wallet_drain_preview(0, 0)
                .await
                .unwrap()
                .candidate_balance_duffs,
            100_000
        );
        {
            let mut manager = manager.write().await;
            let info = manager.get_wallet_info_mut(&id).unwrap();
            for account in info
                .core_wallet
                .accounts
                .dashpay_receival_accounts
                .values_mut()
            {
                for utxo in account.utxos.values_mut() {
                    utxo.is_instantlocked = true;
                }
            }
        }
        assert_eq!(
            wallet
                .shielded_wallet_drain_preview(0, 0)
                .await
                .unwrap()
                .candidate_balance_duffs,
            300_000
        );
    }
}
