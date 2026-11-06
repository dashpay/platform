//! On-demand private-key derivation for a single core (Layer-1) address.
//!
//! The wallet already knows the full BIP-32 derivation path of every
//! address it tracks (it is stored on the address's
//! [`AddressInfo`](key_wallet::managed_account::address_pool::AddressInfo)
//! inside the managed account collection). This module joins that path
//! back to key material so a caller can reveal the private key for one
//! address without knowing anything about derivation-path shapes — the
//! whole lookup-and-derive happens here, on the Rust side, in a single
//! call.
//!
//! # Key source
//!
//! The wallets the iOS app holds are external-signable (watch-only from
//! Rust's point of view): the BIP-39 seed lives in the iOS Keychain, not
//! in the `WalletManager`. Each reveal takes a `master_key` callback and
//! calls it only for such a wallet, after the address is found and with
//! the wallet-manager lock released, because the host may block on a
//! Keychain prompt. A wallet with resident private keys derives straight
//! from the in-process key-wallet and never calls it. The master's scalar
//! is wiped after use, and the returned scalar is wrapped in [`Zeroizing`].

use std::str::FromStr;

use dashcore::crypto::ecdsa::EcdsaSecretKey;
use dashcore::{Address, Network, PrivateKey as DashPrivateKey, PublicKey};
use key_wallet::bip32::{ChildNumber, DerivationPath, ExtendedPrivKey};
use key_wallet::Wallet;
use zeroize::Zeroizing;

use super::mnemonic::key_derivation;
use super::platform_wallet::PlatformWallet;
use crate::error::PlatformWalletError;

/// How many indexes of each chain of BIP-44 account 0
/// [`PlatformWallet::find_bip44_address_path`] searches. It is the bound of
/// the scan dashwallet-ios ran on `KeyWallet` to find a CrowdNode account
/// address, which can sit past the gap limit the account pools keep.
const BIP44_ADDRESS_SCAN_LIMIT: u32 = 300;

/// The private key for one of a wallet's core addresses, in the two
/// forms a caller wants to display: raw 32-byte scalar (sensitive, held
/// in a [`Zeroizing`] buffer) and its network-aware compressed WIF.
///
/// The raw scalar backs the hex rendering the FFI produces; both it and
/// the WIF are wiped by the FFI free function once marshalled to the
/// host.
pub struct CoreAddressPrivateKey {
    /// The address's full BIP-32 derivation path (informational — the
    /// same value already stored on the address row).
    pub derivation_path: DerivationPath,
    /// Raw 32-byte secp256k1 private-key scalar. Zeroized on drop.
    pub private_key: Zeroizing<[u8; 32]>,
    /// Private key in WIF (Wallet Import Format) — network-aware
    /// (mainnet vs testnet/devnet/regtest version byte) and compressed.
    pub wif: String,
}

/// Why a key reveal failed.
#[derive(Debug, thiserror::Error)]
pub enum CoreAddressKeyError<E> {
    /// The wallet refused the reveal.
    #[error(transparent)]
    Wallet(#[from] PlatformWalletError),
    /// The wallet holds no private keys, and `master_key` failed to supply
    /// the master key.
    #[error("the key source failed: {0}")]
    KeySource(#[source] E),
}

impl PlatformWallet {
    /// Derive the private key for `address_str`, one of this wallet's
    /// tracked core addresses.
    ///
    /// Looks the address up across *every* managed account pool (standard
    /// BIP44/BIP32, CoinJoin, identity registration/top-up funding,
    /// provider keys, DashPay) to find its full derivation path, then
    /// derives the secp256k1 private key at that path.
    ///
    /// `master_key` supplies the master key of a wallet without resident
    /// private keys (see the module docs).
    ///
    /// # Errors
    /// - [`CoreAddressKeyError::KeySource`] with the error of `master_key`.
    /// - [`PlatformWalletError::AddressOperation`] if `address_str` is not
    ///   a valid address for this wallet's network.
    /// - [`PlatformWalletError::AddressNotFound`] if the address is valid
    ///   but not tracked by any of this wallet's accounts.
    /// - [`PlatformWalletError::WalletNotFound`] if the wallet was removed
    ///   from its manager, such as while `master_key` ran.
    /// - [`PlatformWalletError::KeyDerivation`] if key derivation fails or
    ///   the derived key does not control the address, such as for a master
    ///   key of another wallet.
    pub fn derive_core_address_private_key<E>(
        &self,
        address_str: &str,
        master_key: impl FnOnce() -> Result<ExtendedPrivKey, E>,
    ) -> Result<CoreAddressPrivateKey, CoreAddressKeyError<E>> {
        let address = parse_address(address_str, self.network())?;
        let path = {
            let manager = self.wallet_manager().blocking_read();
            let info = manager
                .get_wallet_info(&self.wallet_id())
                .ok_or_else(|| self.wallet_not_found())?;
            info.core_wallet
                .accounts
                .all_accounts()
                .into_iter()
                .find_map(|account| account.get_address_info(&address).map(|info| info.path))
                .ok_or_else(|| {
                    PlatformWalletError::AddressNotFound(format!(
                        "address '{address_str}' is not tracked by any account in this wallet"
                    ))
                })?
        };
        self.private_key_from_source(path, &address, master_key)
    }

    /// Returns the derivation path of `address_str` among the first
    /// [`BIP44_ADDRESS_SCAN_LIMIT`] indexes of BIP-44 account 0, external
    /// chain before internal, or `None` when the scan does not reach it.
    ///
    /// The account pools are not consulted, so an address on any other
    /// account is not found. The scan derives public keys from the account
    /// xpub, so it needs no private key.
    ///
    /// Takes a short `blocking_read` on the wallet manager, so call it off
    /// the async runtime's worker.
    ///
    /// # Errors
    /// - [`PlatformWalletError::AddressOperation`] if `address_str` is not
    ///   a valid address for this wallet's network.
    /// - [`PlatformWalletError::WalletNotFound`] if the wallet was removed
    ///   from its manager.
    /// - [`PlatformWalletError::KeyDerivation`] if deriving a scanned
    ///   address fails.
    pub fn find_bip44_address_path(
        &self,
        address_str: &str,
    ) -> Result<Option<DerivationPath>, PlatformWalletError> {
        let address = parse_address(address_str, self.network())?;
        self.scan_bip44_account(&address)
    }

    /// The scan of [`Self::find_bip44_address_path`] for a parsed address.
    fn scan_bip44_account(
        &self,
        address: &Address,
    ) -> Result<Option<DerivationPath>, PlatformWalletError> {
        let network = self.network();
        let account = {
            let manager = self.wallet_manager().blocking_read();
            let wallet = manager
                .get_wallet(&self.wallet_id())
                .ok_or_else(|| self.wallet_not_found())?;
            wallet.accounts.standard_bip44_accounts.get(&0).cloned()
        };
        let Some(account) = account else {
            return Ok(None);
        };

        let account_path = account.derivation_path().map_err(key_derivation)?;
        for chain in [0, 1] {
            let chain = ChildNumber::from_normal_idx(chain).map_err(key_derivation)?;
            let chain_xpub = account
                .account_xpub
                .ckd_pub(chain)
                .map_err(key_derivation)?;
            for index in 0..BIP44_ADDRESS_SCAN_LIMIT {
                let index = ChildNumber::from_normal_idx(index).map_err(key_derivation)?;
                let public_key = chain_xpub.ckd_pub(index).map_err(key_derivation)?.to_pub();
                if Address::p2pkh(&public_key, network) == *address {
                    return Ok(Some(account_path.extend([chain, index])));
                }
            }
        }
        Ok(None)
    }

    /// Derives the private key of `address_str` at the path
    /// [`Self::find_bip44_address_path`] finds for it.
    ///
    /// `master_key` supplies the master key, as for
    /// [`Self::derive_core_address_private_key`].
    ///
    /// # Errors
    /// - [`PlatformWalletError::AddressNotFound`] if the scan does not
    ///   reach the address.
    /// - Any error of [`Self::find_bip44_address_path`], and the
    ///   `master_key` and derivation errors of
    ///   [`Self::derive_core_address_private_key`].
    pub fn bip44_address_private_key<E>(
        &self,
        address_str: &str,
        master_key: impl FnOnce() -> Result<ExtendedPrivKey, E>,
    ) -> Result<CoreAddressPrivateKey, CoreAddressKeyError<E>> {
        let address = parse_address(address_str, self.network())?;
        let path = self.scan_bip44_account(&address)?.ok_or_else(|| {
            PlatformWalletError::AddressNotFound(format!(
                "address '{address_str}' is not within the BIP-44 account 0 scan"
            ))
        })?;
        self.private_key_from_source(path, &address, master_key)
    }

    /// Derives the key at `path` from the wallet's resident keys, or from
    /// the master key `master_key` supplies when the wallet has none.
    ///
    /// `master_key` runs without the wallet-manager lock, so the wallet may
    /// be removed meanwhile.
    fn private_key_from_source<E>(
        &self,
        path: DerivationPath,
        address: &Address,
        master_key: impl FnOnce() -> Result<ExtendedPrivKey, E>,
    ) -> Result<CoreAddressPrivateKey, CoreAddressKeyError<E>> {
        let has_resident_keys = {
            let manager = self.wallet_manager().blocking_read();
            let wallet = manager
                .get_wallet(&self.wallet_id())
                .ok_or_else(|| self.wallet_not_found())?;
            !wallet.is_external_signable() && !wallet.is_watch_only()
        };
        let master = if has_resident_keys {
            None
        } else {
            Some(master_key().map_err(CoreAddressKeyError::KeySource)?)
        };

        let result = {
            let manager = self.wallet_manager().blocking_read();
            manager
                .get_wallet(&self.wallet_id())
                .ok_or_else(|| self.wallet_not_found())
                .and_then(|wallet| {
                    private_key_at(path, address, wallet, master.as_ref(), self.network())
                })
        };
        Ok(result?)
    }

    fn wallet_not_found(&self) -> PlatformWalletError {
        PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id()))
    }
}

/// Parses `address_str` and checks it against `network`, so a string of
/// another network can't match an address of this one (base58 prefixes
/// overlap across the test networks).
fn parse_address(address_str: &str, network: Network) -> Result<Address, PlatformWalletError> {
    Address::from_str(address_str)
        .map_err(|e| {
            PlatformWalletError::AddressOperation(format!("invalid address '{address_str}': {e}"))
        })?
        .require_network(network)
        .map_err(|e| {
            PlatformWalletError::AddressOperation(format!(
                "address '{address_str}' is not valid for {network:?}: {e}"
            ))
        })
}

/// Derives the private key at `path` from `master`, or from `wallet`'s
/// resident keys when it is `None` (see the module docs).
///
/// The key must control `address`. A resolved master of another wallet
/// derives a valid key at the same path, so the result is checked rather
/// than trusted.
fn private_key_at(
    path: DerivationPath,
    address: &Address,
    wallet: &Wallet,
    master: Option<&ExtendedPrivKey>,
    network: Network,
) -> Result<CoreAddressPrivateKey, PlatformWalletError> {
    // Derive the raw scalar from the selected key source.
    let secret_bytes: Zeroizing<[u8; 32]> = match master {
        Some(master) => {
            let derived = master.derive_priv(&path).map_err(|e| {
                PlatformWalletError::KeyDerivation(format!(
                    "failed to derive private key at {path}: {e}"
                ))
            })?;
            derived.private_key.to_bytes()
        }
        None => {
            // Resident key-bearing wallet — derive from its own root.
            let secret_key = wallet.derive_private_key(&path).map_err(|e| {
                PlatformWalletError::KeyDerivation(format!(
                    "failed to derive private key at {path} from resident wallet: {e}"
                ))
            })?;
            secret_key.to_bytes()
        }
    };

    // Build the network-aware compressed WIF. `SecretKey::from_secret_bytes`
    // over the just-derived bytes is infallible, but map its error
    // rather than unwrap to keep the boundary panic-free.
    let secret_key = EcdsaSecretKey::from_bytes(&secret_bytes).map_err(|e| {
        PlatformWalletError::KeyDerivation(format!(
            "derived private key bytes were not a valid secp256k1 scalar: {e}"
        ))
    })?;
    if Address::p2pkh(&PublicKey::new(secret_key.public_key()), network) != *address {
        return Err(PlatformWalletError::KeyDerivation(format!(
            "the key derived at {path} does not control address '{address}'"
        )));
    }
    let wif = DashPrivateKey {
        compressed: true,
        network,
        inner: secret_key,
    }
    .to_wif();

    Ok(CoreAddressPrivateKey {
        derivation_path: path,
        private_key: secret_bytes,
        wif,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{
        mnemonic_wallet_manager, platform_wallet_in, test_platform_wallet_manager,
        TEST_FRENCH_MNEMONIC as FRENCH, TEST_PLATFORM_WALLET_MNEMONIC,
    };
    use crate::wallet::mnemonic::first_receive_address_for_mnemonic;
    use key_wallet::account::StandardAccountType;
    use key_wallet::bip32::ExtendedPubKey;
    use key_wallet::{AccountType, Mnemonic};
    use std::cell::Cell;
    use std::convert::Infallible;
    use tokio::runtime::Runtime;

    /// A `master_key` callback that supplies `master` and counts its calls.
    fn supply<'a>(
        master: &'a ExtendedPrivKey,
        calls: &'a Cell<u32>,
    ) -> impl FnOnce() -> Result<ExtendedPrivKey, Infallible> + 'a {
        move || {
            calls.set(calls.get() + 1);
            Ok(master.clone())
        }
    }

    #[test]
    fn should_find_scanned_bip44_addresses_while_key_reveal_stays_pool_only() {
        let runtime = Runtime::new().unwrap();
        let (manager, wallet_id) = runtime.block_on(test_platform_wallet_manager());
        let wallet = runtime.block_on(manager.get_wallet(&wallet_id)).unwrap();
        let seed = Mnemonic::from_phrase(TEST_PLATFORM_WALLET_MNEMONIC)
            .unwrap()
            .to_seed("");
        let master = ExtendedPrivKey::new_master(Network::Testnet, &*seed).unwrap();
        let account_path =
            |account_type: AccountType| account_type.derivation_path(Network::Testnet).unwrap();
        let bip44 = account_path(AccountType::Standard {
            index: 0,
            standard_account_type: StandardAccountType::BIP44Account,
        });
        let at = |base: &DerivationPath, children: &[u32]| {
            let children: Vec<ChildNumber> = children
                .iter()
                .map(|i| ChildNumber::from_normal_idx(*i).unwrap())
                .collect();
            base.extend(children)
        };
        let address_at = |path: &DerivationPath| {
            let key = master.derive_priv(path).unwrap();
            let public_key = PublicKey::new(ExtendedPubKey::from_priv(&key).public_key);
            Address::p2pkh(&public_key, Network::Testnet).to_string()
        };
        let calls = Cell::new(0);

        // Key reveal stays pool-only: a tracked address resolves, one past
        // the pool does not.
        let tracked = at(&bip44, &[0, 0]);
        let key = wallet
            .derive_core_address_private_key(&address_at(&tracked), supply(&master, &calls))
            .unwrap();
        assert_eq!(key.derivation_path, tracked);
        assert!(matches!(
            wallet.derive_core_address_private_key(
                &address_at(&at(&bip44, &[0, 299])),
                supply(&master, &calls)
            ),
            Err(CoreAddressKeyError::Wallet(
                PlatformWalletError::AddressNotFound(_)
            ))
        ));
        // The test wallet holds no private keys, so the found address asks
        // for the master key and the missing one does not.
        assert_eq!(calls.get(), 1);

        // The scan finds both chains up to its last index, and the key it
        // derives is the master's key at that path.
        for path in [
            at(&bip44, &[0, 0]),
            at(&bip44, &[0, 299]),
            at(&bip44, &[1, 250]),
        ] {
            let address = address_at(&path);
            assert_eq!(
                wallet.find_bip44_address_path(&address).unwrap(),
                Some(path.clone())
            );
            let key = wallet
                .bip44_address_private_key(&address, supply(&master, &calls))
                .unwrap();
            assert_eq!(key.derivation_path, path);
            assert_eq!(
                *key.private_key,
                *master.derive_priv(&path).unwrap().private_key.to_bytes()
            );
        }

        // A master key of another wallet derives a key that does not
        // control the address, and both routes refuse it. A failing key
        // source reaches the caller unchanged.
        let foreign_seed = Mnemonic::from_phrase(FRENCH).unwrap().to_seed("");
        let foreign_master = ExtendedPrivKey::new_master(Network::Testnet, &*foreign_seed).unwrap();
        let tracked_address = address_at(&tracked);
        for result in [
            wallet
                .derive_core_address_private_key(&tracked_address, supply(&foreign_master, &calls)),
            wallet.bip44_address_private_key(&tracked_address, supply(&foreign_master, &calls)),
        ] {
            assert!(matches!(
                result,
                Err(CoreAddressKeyError::Wallet(
                    PlatformWalletError::KeyDerivation(_)
                ))
            ));
        }
        for result in [
            wallet.derive_core_address_private_key(&tracked_address, || Err("locked")),
            wallet.bip44_address_private_key(&tracked_address, || Err("locked")),
        ] {
            assert!(matches!(
                result,
                Err(CoreAddressKeyError::KeySource("locked"))
            ));
        }
        calls.set(0);

        // Past the bound, other accounts and another wallet are not found.
        let bip44_account_1 = account_path(AccountType::Standard {
            index: 1,
            standard_account_type: StandardAccountType::BIP44Account,
        });
        let bip32 = account_path(AccountType::Standard {
            index: 0,
            standard_account_type: StandardAccountType::BIP32Account,
        });
        let voting = account_path(AccountType::ProviderVotingKeys);
        let foreign = first_receive_address_for_mnemonic(FRENCH, Network::Testnet).unwrap();
        for address in [
            address_at(&at(&bip44, &[0, 300])),
            address_at(&at(&bip44_account_1, &[0, 0])),
            address_at(&at(&bip32, &[0, 0])),
            address_at(&at(&voting, &[0])),
            foreign.to_string(),
        ] {
            assert_eq!(
                wallet.find_bip44_address_path(&address).unwrap(),
                None,
                "{address}"
            );
            assert!(matches!(
                wallet.bip44_address_private_key(&address, supply(&master, &calls)),
                Err(CoreAddressKeyError::Wallet(
                    PlatformWalletError::AddressNotFound(_)
                ))
            ));
        }
        assert_eq!(calls.get(), 0);

        // A mainnet address on this testnet wallet, and no address at all.
        for address in ["XoJA8qE3N2Y3jMLEtZ3vcN42qseZ8LvFf5", "not an address"] {
            assert!(matches!(
                wallet.find_bip44_address_path(address),
                Err(PlatformWalletError::AddressOperation(_))
            ));
        }
    }

    /// A `master_key` callback for wallets that must never need one.
    fn no_key_source() -> Result<ExtendedPrivKey, Infallible> {
        panic!("a wallet with resident keys called its key source")
    }

    #[test]
    fn should_reveal_resident_keys_without_the_key_source() {
        let runtime = Runtime::new().unwrap();
        let (wallet_manager, wallet_id, _, address) =
            runtime.block_on(mnemonic_wallet_manager(TEST_PLATFORM_WALLET_MNEMONIC));
        let wallet = runtime.block_on(platform_wallet_in(wallet_manager, wallet_id));
        let seed = Mnemonic::from_phrase(TEST_PLATFORM_WALLET_MNEMONIC)
            .unwrap()
            .to_seed("");
        let master = ExtendedPrivKey::new_master(Network::Testnet, seed.as_ref()).unwrap();
        let path = AccountType::Standard {
            index: 0,
            standard_account_type: StandardAccountType::BIP44Account,
        }
        .derivation_path(Network::Testnet)
        .unwrap()
        .extend([ChildNumber::from_normal_idx(0).unwrap(); 2]);
        let expected = master.derive_priv(&path).unwrap().private_key.to_bytes();

        let address = address.to_string();
        for key in [
            wallet.derive_core_address_private_key(&address, no_key_source),
            wallet.bip44_address_private_key(&address, no_key_source),
        ] {
            let key = key.unwrap();
            assert_eq!(key.derivation_path, path);
            assert_eq!(*key.private_key, *expected);
            let wif = DashPrivateKey::from_wif(&key.wif).unwrap();
            assert_eq!(wif.network, Network::Testnet);
            assert_eq!(*wif.inner.to_bytes(), *expected);
        }
    }

    #[test]
    fn should_report_a_removed_wallet_as_missing() {
        let runtime = Runtime::new().unwrap();
        let (manager, wallet_id) = runtime.block_on(test_platform_wallet_manager());
        let wallet = runtime.block_on(manager.get_wallet(&wallet_id)).unwrap();
        let address =
            first_receive_address_for_mnemonic(TEST_PLATFORM_WALLET_MNEMONIC, Network::Testnet)
                .unwrap()
                .to_string();
        assert!(wallet.find_bip44_address_path(&address).unwrap().is_some());
        runtime.block_on(manager.remove_wallet(&wallet_id)).unwrap();

        assert!(matches!(
            wallet.find_bip44_address_path(&address),
            Err(PlatformWalletError::WalletNotFound(_))
        ));
        let unused = || -> Result<ExtendedPrivKey, Infallible> { unreachable!() };
        assert!(matches!(
            wallet.bip44_address_private_key(&address, unused),
            Err(CoreAddressKeyError::Wallet(
                PlatformWalletError::WalletNotFound(_)
            ))
        ));
        assert!(matches!(
            wallet.derive_core_address_private_key(&address, unused),
            Err(CoreAddressKeyError::Wallet(
                PlatformWalletError::WalletNotFound(_)
            ))
        ));

        // The wallet can also go while `master_key` runs. Removing it there
        // would deadlock if the callback ran under the wallet-manager lock.
        let seed = Mnemonic::from_phrase(TEST_PLATFORM_WALLET_MNEMONIC)
            .unwrap()
            .to_seed("");
        let master = ExtendedPrivKey::new_master(Network::Testnet, seed.as_ref()).unwrap();
        for bip44 in [false, true] {
            let (manager, wallet_id) = runtime.block_on(test_platform_wallet_manager());
            let wallet = runtime.block_on(manager.get_wallet(&wallet_id)).unwrap();
            let remove_then_supply = || {
                runtime.block_on(manager.remove_wallet(&wallet_id)).unwrap();
                Ok::<_, Infallible>(master.clone())
            };
            let result = if bip44 {
                wallet.bip44_address_private_key(&address, remove_then_supply)
            } else {
                wallet.derive_core_address_private_key(&address, remove_then_supply)
            };
            assert!(
                matches!(
                    result,
                    Err(CoreAddressKeyError::Wallet(
                        PlatformWalletError::WalletNotFound(_)
                    ))
                ),
                "bip44: {bip44}"
            );
        }
    }
}
