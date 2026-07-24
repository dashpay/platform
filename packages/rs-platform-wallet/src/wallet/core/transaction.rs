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
use key_wallet::{Account, DerivationPath, ReservationToken, Utxo};

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
    /// The key-wallet [`ReservationToken`] stamped onto the selected inputs when
    /// `build_unsigned_reserved` reserved them, or `None` when the build took no
    /// reservation (no reservation set attached — not reached on the funded
    /// finalize path). Held so an abandoned or definitively-rejected send
    /// releases the reservation *owner-guarded*: after this build's inputs may
    /// have been swept by key-wallet's TTL and re-reserved by a concurrent build
    /// under a new token, releasing by outpoint alone would free that other
    /// build's inputs (the `dashpay/platform#4185` double-spend window).
    /// [`ManagedCoreFundsAccount::release_reservation_if_owner`] releases only
    /// inputs still owned by this token, closing that window.
    reservation_token: Option<ReservationToken>,
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

    /// The key-wallet [`ReservationToken`] the funding inputs were reserved
    /// under (`None` if the build reserved nothing). The broadcast/abandon
    /// release paths present it to
    /// [`ManagedCoreFundsAccount::release_reservation_if_owner`] so a rejected
    /// or abandoned send frees only reservations this build still owns.
    pub fn reservation_token(&self) -> Option<ReservationToken> {
        self.reservation_token
    }

    /// Consume this finalized transaction into the owned parts the deferred
    /// [`SignedPaymentRegistry`](crate::SignedPaymentRegistry) stores.
    ///
    /// Consuming (rather than cloning) is what enforces unique reservation
    /// ownership: `SignedCoreTransaction` is deliberately not `Clone`, so a
    /// finalize yields exactly one ownership object and the registry can be
    /// handed it exactly once — a caller cannot mint two live tokens that name
    /// the same held reservation (`dashpay/platform#4185`). The transaction,
    /// funding account, and reservation height are derived here, not supplied
    /// independently by the caller.
    pub(crate) fn into_registered_parts(self) -> RegisteredPaymentParts {
        RegisteredPaymentParts {
            transaction: self.transaction,
            funding_account_type: self.funding_account_type,
            funding_account_index: self.funding_account_index,
            reservation_height: self.reservation_height,
            reservation_token: self.reservation_token,
        }
    }
}

/// The owned facts the deferred-payment registry takes over when it registers a
/// finalized transaction. Produced only by
/// [`SignedCoreTransaction::into_registered_parts`], which consumes the
/// non-`Clone` ownership object exactly once.
pub(crate) struct RegisteredPaymentParts {
    pub(crate) transaction: Transaction,
    pub(crate) funding_account_type: AccountTypePreference,
    pub(crate) funding_account_index: u32,
    pub(crate) reservation_height: u32,
    pub(crate) reservation_token: Option<ReservationToken>,
}

#[cfg(any(test, feature = "test-utils"))]
impl SignedCoreTransaction {
    /// Build a `SignedCoreTransaction` directly, for tests that need a finalized
    /// ownership object without running the full funding + signing pipeline
    /// (e.g. the registry and FFI destroy/lifecycle tests).
    pub fn new_for_test(
        transaction: Transaction,
        fee: u64,
        funding_account_type: AccountTypePreference,
        funding_account_index: u32,
        reservation_height: u32,
        reservation_token: Option<ReservationToken>,
    ) -> Self {
        Self {
            transaction,
            fee,
            funding_account_type,
            funding_account_index,
            reservation_height,
            reservation_token,
        }
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
        let (unsigned, fee, selected, paths, height, reservation_token) = {
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

            // `set_funding` observes ReservationSet and `build_unsigned_reserved`
            // records its selection AND returns the token stamped onto the
            // reserved inputs. There is no await between them and the manager
            // write guard prevents another finalizer interleaving. The token
            // rides in `SignedCoreTransaction` so a later abandon or rejected
            // broadcast releases *only* the inputs this build still owns, even
            // if a TTL sweep re-reserved them under a new token meanwhile
            // (`dashpay/platform#4185`).
            let (unsigned, fee, reservation_token) = builder
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

            (unsigned, fee, selected, paths, height, reservation_token)
        };

        let signed = match signer
            .sign_tx(unsigned.clone(), selected, move |address| {
                paths.get(&address).cloned()
            })
            .await
        {
            Ok(signed) => signed,
            Err(error) => {
                // Signing awaited an (external) signer with the manager lock
                // dropped, so key-wallet's TTL sweep could have reclaimed this
                // build's reservation and a concurrent build re-taken the same
                // inputs under a new token. Release owner-guarded so we free
                // only what this build still owns.
                self.release_transaction_reservation(
                    account_type,
                    account_index,
                    &unsigned,
                    reservation_token,
                )
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
            reservation_token,
        })
    }

    /// Release a finalized transaction that the caller has chosen not to send.
    pub async fn abandon_transaction(&self, transaction: &SignedCoreTransaction) {
        self.release_transaction_reservation(
            transaction.funding_account_type,
            transaction.funding_account_index,
            &transaction.transaction,
            transaction.reservation_token,
        )
        .await;
    }

    /// Release the funding reservation `transaction` holds, bound to this
    /// handle's own wallet *generation*.
    ///
    /// `token` is the [`ReservationToken`] the build stamped onto the inputs
    /// (`SignedCoreTransaction::reservation_token`). When present the release is
    /// *owner-guarded* — it frees only inputs still owned by that token, so a
    /// reservation key-wallet's TTL swept and a concurrent build re-took is left
    /// untouched (`dashpay/platform#4185`). When `None` (the build reserved
    /// nothing) it falls back to the unconditional by-outpoint release; that
    /// path is never reached for a funded finalize, which always reserves.
    pub(crate) async fn release_transaction_reservation(
        &self,
        account_type: AccountTypePreference,
        account_index: u32,
        transaction: &Transaction,
        token: Option<ReservationToken>,
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
            // Owner-guarded when the build stamped a token: even within this
            // generation, a TTL sweep between build and release could have
            // re-reserved the same outpoints under a new token, and an
            // unconditional release would free that newer reservation. With the
            // token key-wallet frees only inputs this build still owns. `None`
            // (no reservation taken) falls back to the unconditional release.
            Some(managed) => match token {
                Some(token) => managed.release_reservation_if_owner(transaction, token),
                None => managed.release_reservation(transaction),
            },
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
