use key_wallet::transaction_checking::{TransactionCheckResult, TransactionContext, WalletTransactionChecker};
use dashcore::{Network, Transaction};
use key_wallet::Wallet;
use crate::platform_wallet_info::PlatformWalletInfo;

/// Implement WalletTransactionChecker by delegating to ManagedWalletInfo
impl WalletTransactionChecker for PlatformWalletInfo {
    fn check_transaction(
        &mut self,
        tx: &Transaction,
        network: Network,
        context: TransactionContext,
        update_state_with_wallet_if_found: Option<&Wallet>,
    ) -> TransactionCheckResult {
        // Delegate to the underlying wallet info
        self.wallet_info
            .check_transaction(tx, network, context, update_state_with_wallet_if_found)
    }
}