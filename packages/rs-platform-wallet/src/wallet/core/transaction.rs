//! Atomic Core transaction finalization.
//!
//! Funding and reservation deliberately happen in one synchronous critical
//! section under the wallet-manager write lock. Signing happens only after the
//! lock is dropped, so an external signer may call back into a host mnemonic
//! resolver without pinning wallet state.

use std::collections::HashMap;
use std::sync::Arc;

use dashcore::{Address, Transaction};
use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
use key_wallet::managed_account::ManagedCoreFundsAccount;
use key_wallet::wallet::managed_wallet_info::coin_selection::SelectionError;
use key_wallet::wallet::managed_wallet_info::transaction_builder::{
    BuilderError, TransactionBuilder, TransactionSigner,
};
use key_wallet::wallet::managed_wallet_info::transaction_building::AccountTypePreference;
use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;
use key_wallet::{Account, DerivationPath, Utxo};

use super::CoreWallet;
use crate::broadcaster::TransactionBroadcaster;
use crate::PlatformWalletError;

fn map_builder_error(
    error: BuilderError,
    account_type: AccountTypePreference,
    account_index: u32,
) -> PlatformWalletError {
    let funds = match error {
        BuilderError::InsufficientFunds {
            available,
            required,
        }
        | BuilderError::CoinSelection(SelectionError::InsufficientFunds {
            available,
            required,
        }) => Some((Some(available), Some(required))),
        BuilderError::CoinSelection(SelectionError::NoUtxosAvailable) => Some((Some(0), None)),
        _ => None,
    };
    if let Some((available, required)) = funds {
        return PlatformWalletError::CoreInsufficientFunds {
            account_type,
            account_index,
            available,
            required,
        };
    }
    PlatformWalletError::TransactionBuild(error.to_string())
}

/// A signed Core transaction whose selected inputs remain reserved until it is
/// broadcast, explicitly abandoned, observed by sync, or reclaimed by the
/// reservation TTL.
#[derive(Debug)]
pub struct SignedCoreTransaction {
    transaction: Transaction,
    fee: u64,
    funding_account_type: AccountTypePreference,
    funding_account_index: u32,
    /// The wallet's `last_processed_height` captured **inside** the funding
    /// critical section — the exact clock `set_current_height` stamped the
    /// selected inputs' reservation with, sampled *before* the (potentially
    /// slow, external) signer ran. The deferred-payment registry's age guard
    /// must baseline off this, not off a fresh `last_processed_height` sampled
    /// after signing: a slow external signer could otherwise let the wallet
    /// advance far enough that the token looks fresh while the reservation it
    /// covers has already aged toward key-wallet's TTL sweep.
    reservation_height: u32,
}

impl SignedCoreTransaction {
    pub fn transaction(&self) -> &Transaction {
        &self.transaction
    }

    pub fn fee(&self) -> u64 {
        self.fee
    }

    pub fn funding_account_type(&self) -> AccountTypePreference {
        self.funding_account_type
    }

    pub fn funding_account_index(&self) -> u32 {
        self.funding_account_index
    }

    /// The `last_processed_height` the funding reservation was stamped with,
    /// captured in the funding critical section before signing. The deferred
    /// registry registers the token with this height so its age guard measures
    /// the reservation's true age rather than a post-signing sample.
    pub fn reservation_height(&self) -> u32 {
        self.reservation_height
    }
}

fn account(
    wallet: &key_wallet::Wallet,
    account_type: AccountTypePreference,
    account_index: u32,
) -> Option<&Account> {
    match account_type {
        AccountTypePreference::BIP44 => wallet.get_bip44_account(account_index),
        AccountTypePreference::BIP32 => wallet.get_bip32_account(account_index),
        AccountTypePreference::CoinJoin => wallet.get_coinjoin_account(account_index),
    }
}

fn managed_account(
    accounts: &key_wallet::account::ManagedAccountCollection,
    account_type: AccountTypePreference,
    account_index: u32,
) -> Option<&ManagedCoreFundsAccount> {
    match account_type {
        AccountTypePreference::BIP44 => accounts.standard_bip44_accounts.get(&account_index),
        AccountTypePreference::BIP32 => accounts.standard_bip32_accounts.get(&account_index),
        AccountTypePreference::CoinJoin => accounts.coinjoin_accounts.get(&account_index),
    }
}

fn managed_account_mut(
    accounts: &mut key_wallet::account::ManagedAccountCollection,
    account_type: AccountTypePreference,
    account_index: u32,
) -> Option<&mut ManagedCoreFundsAccount> {
    match account_type {
        AccountTypePreference::BIP44 => accounts.standard_bip44_accounts.get_mut(&account_index),
        AccountTypePreference::BIP32 => accounts.standard_bip32_accounts.get_mut(&account_index),
        AccountTypePreference::CoinJoin => accounts.coinjoin_accounts.get_mut(&account_index),
    }
}

impl<B: TransactionBroadcaster + ?Sized> CoreWallet<B> {
    /// Consume a configured builder, atomically fund and reserve its selected
    /// inputs, then sign without holding the wallet-manager lock.
    pub async fn finalize_transaction<S: TransactionSigner + ?Sized + Sync>(
        &self,
        builder: TransactionBuilder,
        account_type: AccountTypePreference,
        account_index: u32,
        signer: &S,
    ) -> Result<SignedCoreTransaction, PlatformWalletError> {
        let (unsigned, fee, selected, paths, height) = {
            let mut manager = self.wallet_manager.write().await;
            let (wallet, info) = manager
                .get_wallet_and_info_mut(&self.wallet_id)
                .ok_or_else(|| PlatformWalletError::WalletNotFound("wallet not found".into()))?;

            let account = account(wallet, account_type, account_index)
                .cloned()
                .ok_or_else(|| {
                    PlatformWalletError::WalletNotFound(format!(
                        "wallet account {account_type:?} #{account_index} not found"
                    ))
                })?;
            let height = info.core_wallet.last_processed_height();
            let managed =
                managed_account_mut(&mut info.core_wallet.accounts, account_type, account_index)
                    .ok_or_else(|| {
                        PlatformWalletError::WalletNotFound(format!(
                            "managed account {account_type:?} #{account_index} not found"
                        ))
                    })?;

            // `set_funding` observes ReservationSet and the build records
            // its selection. There is no await between them and the
            // manager write guard prevents another finalizer interleaving.
            //
            // The `ReservationToken` is intentionally discarded: this path
            // releases abandoned reservations via the unconditional
            // `release_reservation(&unsigned)` below. Owner-guarded release
            // via the token is threaded separately (dashpay/platform#4185).
            let (unsigned, fee, _reservation) = builder
                .set_current_height(height)
                .set_funding(managed, &account)
                .build_unsigned_reserved()
                .map_err(|error| map_builder_error(error, account_type, account_index))?;

            let selected: Vec<Utxo> = match unsigned
                .input
                .iter()
                .map(|input| {
                    managed
                        .utxos
                        .get(&input.previous_output)
                        .cloned()
                        .ok_or_else(|| {
                            PlatformWalletError::TransactionBuild(format!(
                                "selected input {} is no longer in the funding account",
                                input.previous_output
                            ))
                        })
                })
                .collect::<Result<_, _>>()
            {
                Ok(selected) => selected,
                Err(error) => {
                    managed.release_reservation(&unsigned);
                    return Err(error);
                }
            };
            let paths: HashMap<Address, DerivationPath> = match selected
                .iter()
                .map(|utxo| {
                    managed
                        .address_derivation_path(&utxo.address)
                        .map(|path| (utxo.address.clone(), path))
                        .ok_or_else(|| {
                            PlatformWalletError::TransactionBuild(format!(
                                "no derivation path for selected input address {}",
                                utxo.address
                            ))
                        })
                })
                .collect::<Result<_, _>>()
            {
                Ok(paths) => paths,
                Err(error) => {
                    managed.release_reservation(&unsigned);
                    return Err(error);
                }
            };

            (unsigned, fee, selected, paths, height)
        };

        let signed = match signer
            .sign_tx(unsigned.clone(), selected, move |address| {
                paths.get(&address).cloned()
            })
            .await
        {
            Ok(signed) => signed,
            Err(error) => {
                self.release_transaction_reservation(account_type, account_index, &unsigned)
                    .await;
                return Err(PlatformWalletError::TransactionBuild(error.to_string()));
            }
        };

        Ok(SignedCoreTransaction {
            transaction: signed,
            fee,
            funding_account_type: account_type,
            funding_account_index: account_index,
            reservation_height: height,
        })
    }

    /// Release a finalized transaction that the caller has chosen not to send.
    pub async fn abandon_transaction(&self, transaction: &SignedCoreTransaction) {
        self.release_transaction_reservation(
            transaction.funding_account_type,
            transaction.funding_account_index,
            &transaction.transaction,
        )
        .await;
    }

    pub(crate) async fn release_transaction_reservation(
        &self,
        account_type: AccountTypePreference,
        account_index: u32,
        transaction: &Transaction,
    ) {
        // Validate the generation AND mutate the `ReservationSet` under one
        // manager-lock hold. `ReservationSet::release` removes an outpoint
        // unconditionally, and it is reached via `wallet_id` — an identity that a
        // remove-then-recreate under the same id preserves. Between a token's
        // generation validation and this cleanup the wallet could therefore have
        // been re-created, and an unguarded release-by-outpoint could then free
        // the NEW generation's reservation on the same input.
        //
        // Binding the release to this handle's own generation closes that
        // window: the wallet registered under `wallet_id` is the same generation
        // as `self` iff their per-generation balance `Arc`s are pointer-equal
        // (`wallet_id` + the shared manager `Arc` are both preserved across a
        // recreation; only the balance `Arc` is fresh — the same identity
        // `is_same_generation` uses). A read lock is enough and makes this atomic
        // against recreation: a recreate needs the manager *write* lock, so it
        // cannot interleave between the pointer check and the release below.
        let manager = self.wallet_manager.read().await;
        let Some(info) = manager.get_wallet_info(&self.wallet_id) else {
            tracing::warn!(
                wallet_id = %hex::encode(self.wallet_id),
                ?account_type,
                account_index,
                "could not release finalized Core transaction reservation: wallet not found"
            );
            return;
        };
        if !Arc::ptr_eq(&info.balance, self.generation()) {
            // The wallet under this id is a different (re-created) generation:
            // releasing by outpoint could free ITS reservation. Leave it — the
            // original generation's reservation ceased to exist with it.
            tracing::warn!(
                wallet_id = %hex::encode(self.wallet_id),
                ?account_type,
                account_index,
                "skipping reservation release: wallet was re-created under the same id \
                 (different generation) since the token was minted"
            );
            return;
        }
        match managed_account(&info.core_wallet.accounts, account_type, account_index) {
            Some(managed) => managed.release_reservation(transaction),
            None => tracing::warn!(
                wallet_id = %hex::encode(self.wallet_id),
                ?account_type,
                account_index,
                "could not release finalized Core transaction reservation: account not found"
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use async_trait::async_trait;
    use dashcore::secp256k1::{ecdsa, PublicKey};
    use dashcore::{Address as DashAddress, Network};
    use key_wallet::account::account_type::StandardAccountType;
    use key_wallet::signer::{Signer, SignerMethod};
    use key_wallet::wallet::managed_wallet_info::transaction_builder::TransactionBuilder;
    use key_wallet::wallet::managed_wallet_info::transaction_building::AccountTypePreference;
    use key_wallet::DerivationPath;
    use tokio::sync::Barrier;

    use crate::broadcaster::TransactionBroadcaster;
    use crate::test_support::{
        funded_wallet_manager, AlwaysMaybeSentBroadcaster, AlwaysOkBroadcaster,
        AlwaysRejectedBroadcaster, WalletSigner,
    };
    use crate::wallet::core::CoreWallet;
    use crate::PlatformWalletError;

    fn preference(account_type: StandardAccountType) -> AccountTypePreference {
        match account_type {
            StandardAccountType::BIP44Account => AccountTypePreference::BIP44,
            StandardAccountType::BIP32Account => AccountTypePreference::BIP32,
        }
    }

    async fn core<B: TransactionBroadcaster>(
        account_type: StandardAccountType,
        broadcaster: Arc<B>,
    ) -> (CoreWallet<B>, WalletSigner) {
        let (manager, wallet_id, balance, signer) = funded_wallet_manager(account_type).await;
        let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
        (
            CoreWallet::new(sdk, manager, wallet_id, broadcaster, balance),
            signer,
        )
    }

    fn payment_builder(tag: u8) -> TransactionBuilder {
        TransactionBuilder::new().add_output(
            &DashAddress::dummy(Network::Testnet, usize::from(tag)),
            1_000_000,
        )
    }

    #[tokio::test]
    async fn concurrent_same_account_finalizers_cannot_reserve_the_same_input() {
        let account_type = StandardAccountType::BIP44Account;
        let (core, signer) = core(account_type, Arc::new(AlwaysOkBroadcaster)).await;
        let core = Arc::new(core);
        let barrier = Arc::new(Barrier::new(3));
        let mut tasks = Vec::new();
        for tag in [10, 11] {
            let core = Arc::clone(&core);
            let signer = signer.clone();
            let barrier = Arc::clone(&barrier);
            tasks.push(tokio::spawn(async move {
                barrier.wait().await;
                core.finalize_transaction(
                    payment_builder(tag),
                    preference(account_type),
                    0,
                    &signer,
                )
                .await
            }));
        }
        barrier.wait().await;
        let mut results = Vec::new();
        for task in tasks {
            results.push(task.await);
        }
        let successes = results
            .iter()
            .filter(|result| matches!(result, Ok(Ok(_))))
            .count();
        let build_failures = results
            .iter()
            .filter(|result| {
                matches!(
                    result,
                    Ok(Err(PlatformWalletError::CoreInsufficientFunds { .. }))
                )
            })
            .count();
        assert_eq!((successes, build_failures), (1, 1));
    }

    struct FailingSigner;

    #[async_trait]
    impl Signer for FailingSigner {
        type Error = &'static str;

        fn supported_methods(&self) -> &[SignerMethod] {
            &[SignerMethod::Digest]
        }

        async fn sign_ecdsa(
            &self,
            _path: &DerivationPath,
            _sighash: [u8; 32],
        ) -> Result<(ecdsa::Signature, PublicKey), Self::Error> {
            Err("intentional signer failure")
        }

        async fn public_key(&self, _path: &DerivationPath) -> Result<PublicKey, Self::Error> {
            Err("intentional signer failure")
        }
    }

    #[tokio::test]
    async fn validation_and_signing_failures_do_not_strand_reservations() {
        let account_type = StandardAccountType::BIP44Account;
        let (core, signer) = core(account_type, Arc::new(AlwaysOkBroadcaster)).await;

        let validation = core
            .finalize_transaction(
                TransactionBuilder::new(),
                preference(account_type),
                0,
                &signer,
            )
            .await;
        assert!(matches!(
            validation,
            Err(PlatformWalletError::TransactionBuild(_))
        ));

        let signing = core
            .finalize_transaction(
                payment_builder(20),
                preference(account_type),
                0,
                &FailingSigner,
            )
            .await;
        assert!(matches!(
            signing,
            Err(PlatformWalletError::TransactionBuild(_))
        ));

        assert!(core
            .finalize_transaction(payment_builder(21), preference(account_type), 0, &signer)
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn abandon_and_definitive_rejection_release_but_maybe_sent_retains() {
        let account_type = StandardAccountType::BIP44Account;
        let (rejection_core, signer) =
            core(account_type, Arc::new(AlwaysRejectedBroadcaster)).await;
        let abandoned = rejection_core
            .finalize_transaction(payment_builder(30), preference(account_type), 0, &signer)
            .await
            .expect("finalize for abandon");
        rejection_core.abandon_transaction(&abandoned).await;

        let rejected = rejection_core
            .finalize_transaction(payment_builder(31), preference(account_type), 0, &signer)
            .await
            .expect("reservation released by abandon");
        assert!(matches!(
            rejection_core
                .broadcast_finalized_transaction(&rejected)
                .await,
            Err(PlatformWalletError::TransactionBroadcast(_))
        ));
        assert!(rejection_core
            .finalize_transaction(payment_builder(32), preference(account_type), 0, &signer)
            .await
            .is_ok());

        let (ambiguous_core, ambiguous_signer) =
            core(account_type, Arc::new(AlwaysMaybeSentBroadcaster)).await;
        let ambiguous = ambiguous_core
            .finalize_transaction(
                payment_builder(33),
                preference(account_type),
                0,
                &ambiguous_signer,
            )
            .await
            .expect("finalize ambiguous send");
        assert!(matches!(
            ambiguous_core
                .broadcast_finalized_transaction(&ambiguous)
                .await,
            Err(PlatformWalletError::TransactionBroadcastUnconfirmed(_))
        ));
        assert!(matches!(
            ambiguous_core
                .finalize_transaction(
                    payment_builder(34),
                    preference(account_type),
                    0,
                    &ambiguous_signer,
                )
                .await,
            Err(PlatformWalletError::CoreInsufficientFunds { .. })
        ));
    }
}
