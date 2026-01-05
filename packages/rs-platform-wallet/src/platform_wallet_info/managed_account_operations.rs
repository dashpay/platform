use crate::platform_wallet_info::PlatformWalletInfo;
use key_wallet::wallet::managed_wallet_info::ManagedAccountOperations;
use key_wallet::{AccountType, ExtendedPubKey, Wallet};

/// Implement ManagedAccountOperations for PlatformWalletInfo
impl ManagedAccountOperations for PlatformWalletInfo {
    fn add_managed_account(
        &mut self,
        wallet: &Wallet,
        account_type: AccountType,
    ) -> key_wallet::Result<()> {
        self.wallet_info
            .add_managed_account(wallet, account_type)
    }

    fn add_managed_account_with_passphrase(
        &mut self,
        wallet: &Wallet,
        account_type: AccountType,
        passphrase: &str,
    ) -> key_wallet::Result<()> {
        self.wallet_info.add_managed_account_with_passphrase(
            wallet,
            account_type,
            passphrase,
        )
    }

    fn add_managed_account_from_xpub(
        &mut self,
        account_type: AccountType,
        account_xpub: ExtendedPubKey,
    ) -> key_wallet::Result<()> {
        self.wallet_info
            .add_managed_account_from_xpub(account_type, account_xpub)
    }

    #[cfg(feature = "bls")]
    fn add_managed_bls_account(
        &mut self,
        wallet: &Wallet,
        account_type: AccountType,
    ) -> key_wallet::Result<()> {
        self.wallet_info
            .add_managed_bls_account(wallet, account_type)
    }

    #[cfg(feature = "bls")]
    fn add_managed_bls_account_with_passphrase(
        &mut self,
        wallet: &Wallet,
        account_type: AccountType,
        passphrase: &str,
    ) -> key_wallet::Result<()> {
        self.wallet_info.add_managed_bls_account_with_passphrase(
            wallet,
            account_type,
            passphrase,
        )
    }

    #[cfg(feature = "bls")]
    fn add_managed_bls_account_from_public_key(
        &mut self,
        account_type: AccountType,
        bls_public_key: [u8; 48],
    ) -> key_wallet::Result<()> {
        self.wallet_info.add_managed_bls_account_from_public_key(
            account_type,
            bls_public_key,
        )
    }

    #[cfg(feature = "eddsa")]
    fn add_managed_eddsa_account(
        &mut self,
        wallet: &Wallet,
        account_type: AccountType,
    ) -> key_wallet::Result<()> {
        self.wallet_info
            .add_managed_eddsa_account(wallet, account_type)
    }

    #[cfg(feature = "eddsa")]
    fn add_managed_eddsa_account_with_passphrase(
        &mut self,
        wallet: &Wallet,
        account_type: AccountType,
        passphrase: &str,
    ) -> key_wallet::Result<()> {
        self.wallet_info.add_managed_eddsa_account_with_passphrase(
            wallet,
            account_type,
            passphrase,
        )
    }

    #[cfg(feature = "eddsa")]
    fn add_managed_eddsa_account_from_public_key(
        &mut self,
        account_type: AccountType,
        ed25519_public_key: [u8; 32],
    ) -> key_wallet::Result<()> {
        self.wallet_info.add_managed_eddsa_account_from_public_key(
            account_type,
            ed25519_public_key,
        )
    }
}
