use crate::platform_wallet_info::PlatformWalletInfo;
use async_trait::async_trait;
use dashcore::Transaction;
use key_wallet::transaction_checking::{
    TransactionCheckResult, TransactionContext, WalletTransactionChecker,
};
use key_wallet::Wallet;

/// Implement WalletTransactionChecker for PlatformWalletInfo
#[async_trait]
impl WalletTransactionChecker for PlatformWalletInfo {
    async fn check_core_transaction(
        &mut self,
        tx: &Transaction,
        context: TransactionContext,
        wallet: &mut Wallet,
        update_state: bool,
    ) -> TransactionCheckResult {
        // Check transaction with underlying wallet info
        let result = self
            .wallet_info
            .check_core_transaction(tx, context, wallet, update_state)
            .await;

        // If the transaction is relevant, and it's an asset lock, automatically fetch identities
        if result.is_relevant {
            use dashcore::transaction::special_transaction::TransactionPayload;

            if matches!(
                &tx.special_transaction_payload,
                Some(TransactionPayload::AssetLockPayloadType(_))
            ) {
                // Check if we have an SDK configured
                if self.identity_manager().sdk.is_some() {
                    // Call the identity fetching logic
                    if let Err(e) = self
                        .fetch_identity_and_contacts_for_asset_lock(wallet, tx)
                        .await
                    {
                        eprintln!("Failed to fetch identity for asset lock: {}", e);
                    }
                }
            }
        }

        result
    }
}
