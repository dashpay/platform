use crate::platform_wallet_info::PlatformWalletInfo;
use dashcore::Network;
use key_wallet::wallet::managed_wallet_info::ManagedAccountOperations;
use key_wallet::{AccountType, ExtendedPubKey, Wallet};

/// Implement ManagedAccountOperations for PlatformWalletInfo
impl ManagedAccountOperations for PlatformWalletInfo {
    fn add_managed_account(
        &mut self,
        wallet: &Wallet,
        account_type: AccountType,
        network: Network,
    ) -> key_wallet::Result<()> {
        self.wallet_info
            .add_managed_account(wallet, account_type, network)
    }

    fn add_managed_account_with_passphrase(
        &mut self,
        wallet: &Wallet,
        account_type: AccountType,
        network: Network,
        passphrase: &str,
    ) -> key_wallet::Result<()> {
        self.wallet_info.add_managed_account_with_passphrase(
            wallet,
            account_type,
            network,
            passphrase,
        )
    }

    fn add_managed_account_from_xpub(
        &mut self,
        account_type: AccountType,
        network: Network,
        account_xpub: ExtendedPubKey,
    ) -> key_wallet::Result<()> {
        self.wallet_info
            .add_managed_account_from_xpub(account_type, network, account_xpub)
    }

    #[cfg(feature = "bls")]
    fn add_managed_bls_account(
        &mut self,
        wallet: &Wallet,
        account_type: AccountType,
        network: Network,
    ) -> key_wallet::Result<()> {
        self.wallet_info
            .add_managed_bls_account(wallet, account_type, network)
    }

    #[cfg(feature = "bls")]
    fn add_managed_bls_account_with_passphrase(
        &mut self,
        wallet: &Wallet,
        account_type: AccountType,
        network: Network,
        passphrase: &str,
    ) -> key_wallet::Result<()> {
        self.wallet_info.add_managed_bls_account_with_passphrase(
            wallet,
            account_type,
            network,
            passphrase,
        )
    }

    #[cfg(feature = "bls")]
    fn add_managed_bls_account_from_public_key(
        &mut self,
        account_type: AccountType,
        network: Network,
        bls_public_key: [u8; 48],
    ) -> key_wallet::Result<()> {
        self.wallet_info.add_managed_bls_account_from_public_key(
            account_type,
            network,
            bls_public_key,
        )
    }

    #[cfg(feature = "eddsa")]
    fn add_managed_eddsa_account(
        &mut self,
        wallet: &Wallet,
        account_type: AccountType,
        network: Network,
    ) -> key_wallet::Result<()> {
        self.wallet_info
            .add_managed_eddsa_account(wallet, account_type, network)
    }

    #[cfg(feature = "eddsa")]
    fn add_managed_eddsa_account_with_passphrase(
        &mut self,
        wallet: &Wallet,
        account_type: AccountType,
        network: Network,
        passphrase: &str,
    ) -> key_wallet::Result<()> {
        self.wallet_info.add_managed_eddsa_account_with_passphrase(
            wallet,
            account_type,
            network,
            passphrase,
        )
    }

    #[cfg(feature = "eddsa")]
    fn add_managed_eddsa_account_from_public_key(
        &mut self,
        account_type: AccountType,
        network: Network,
        ed25519_public_key: [u8; 32],
    ) -> key_wallet::Result<()> {
        self.wallet_info.add_managed_eddsa_account_from_public_key(
            account_type,
            network,
            ed25519_public_key,
        )
    }
}
