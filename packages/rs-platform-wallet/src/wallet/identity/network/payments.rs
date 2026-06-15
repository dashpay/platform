//! DashPay payment recording and send-to-contact flows.

use dpp::prelude::Identifier;
use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;

use super::*;
use crate::broadcaster::TransactionBroadcaster;
use crate::error::PlatformWalletError;
use crate::wallet::identity::types::dashpay::payment::DashpayAddressMatch;

// ---------------------------------------------------------------------------
// Incoming payment recording
// ---------------------------------------------------------------------------

impl<B: TransactionBroadcaster + ?Sized> IdentityWallet<B> {
    /// Match a Core transaction output address against DashPay contact
    /// receiving accounts AND record the payment if matched.
    ///
    /// Combines address matching with payment recording in one call so
    /// callers don't need to manually construct `PaymentEntry` or access
    /// `ManagedIdentity`. Returns the match info for logging.
    ///
    /// Non-blocking: returns `Err(())` if the wallet-manager lock is
    /// contended. Safe to call from any thread.
    #[allow(clippy::result_unit_err)]
    pub fn try_record_incoming_payment(
        &self,
        address: &dashcore::Address,
        txid: String,
        value: u64,
    ) -> Result<Option<DashpayAddressMatch>, ()> {
        let wm = self.wallet_manager.try_write().map_err(|_| ())?;
        let Some(info) = wm.get_wallet_info(&self.wallet_id) else {
            return Ok(None);
        };
        let m = Self::match_in_collection(info, address);
        drop(wm);

        if let Some(ref m) = m {
            let this = self.clone();
            let owner_id = m.user_identity_id;
            let contact_id = m.friend_identity_id;
            tokio::spawn(async move {
                let mut wm = this.wallet_manager.write().await;
                if let Some(info) = wm.get_wallet_info_mut(&this.wallet_id) {
                    if let Some(managed) = info.identity_manager.managed_identity_mut(&owner_id) {
                        managed.record_dashpay_payment(
                            txid,
                            crate::wallet::identity::types::dashpay::payment::PaymentEntry::new_received(
                                contact_id, value, None,
                            ),
                            &this.persister,
                        );
                    }
                }
            });
        }

        Ok(m)
    }
}

// ---------------------------------------------------------------------------
// Send payment to contact
// ---------------------------------------------------------------------------

impl<B: TransactionBroadcaster + ?Sized> IdentityWallet<B> {
    /// Send a Core payment to a DashPay contact.
    ///
    /// Derives the next payment address from the contact's `DashpayExternalAccount`
    /// address pool, builds and broadcasts the transaction via the injected
    /// broadcaster, and records the [`PaymentEntry`] on the sender's
    /// [`ManagedIdentity`].
    ///
    /// # Prerequisite
    ///
    /// `register_external_contact_account` must have been called first so the
    /// watch-only account (and hence its address pool) is available in the
    /// wallet manager. Returns [`PlatformWalletError::InvalidIdentityData`] if
    /// no external account exists for this contact pair.
    ///
    /// # Arguments
    ///
    /// * `from_identity_id` - Our identity that is sending the payment.
    /// * `to_contact_id`    - The contact's identity.
    /// * `amount_duffs`     - Amount to send in duffs (1 DASH = 1e8 duffs).
    /// * `memo`             - Optional free-text memo to attach to the entry.
    ///
    /// # Returns
    ///
    /// The `Txid` of the broadcast transaction and the newly created
    /// [`PaymentEntry`] recording the outgoing payment.
    pub async fn send_payment(
        &self,
        from_identity_id: &Identifier,
        to_contact_id: &Identifier,
        amount_duffs: u64,
        memo: Option<String>,
    ) -> Result<
        (
            dashcore::Txid,
            crate::wallet::identity::types::dashpay::payment::PaymentEntry,
        ),
        PlatformWalletError,
    > {
        use std::collections::BTreeSet;

        use dashcore::OutPoint;
        use key_wallet::account::account_collection::DashpayAccountKey;
        use key_wallet::wallet::managed_wallet_info::coin_selection::SelectionStrategy;
        use key_wallet::wallet::managed_wallet_info::transaction_builder::TransactionBuilder;
        use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;

        let account_index: u32 = 0;

        // Build, sign, reserve — all under one write-lock acquisition. Mirrors
        // `CoreWallet::send_to_addresses` so concurrent calls between this and
        // core `send_to_addresses` cannot select the same UTXO (#3585).
        let (payment_address, tx, _reservation) = {
            let mut wm = self.wallet_manager.write().await;

            // Resolve the external account's xpub so we can derive addresses.
            let contact_xpub = {
                let wallet = wm.get_wallet(&self.wallet_id).ok_or_else(|| {
                    PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id))
                })?;
                wallet
                    .accounts
                    .dashpay_external_accounts
                    .get(&DashpayAccountKey {
                        index: account_index,
                        user_identity_id: from_identity_id.to_buffer(),
                        friend_identity_id: to_contact_id.to_buffer(),
                    })
                    .map(|a| a.account_xpub)
                    .ok_or_else(|| {
                        PlatformWalletError::InvalidIdentityData(format!(
                            "No DashpayExternalAccount found for contact {} — call \
                             register_external_contact_account first",
                            to_contact_id
                        ))
                    })?
            };

            let (wallet, info) = wm
                .get_wallet_and_info_mut(&self.wallet_id)
                .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;

            // Resolve the funding-account xpub up front so we can advance the
            // change-address derivation under the same lock.
            let funding_xpub = wallet
                .accounts
                .standard_bip44_accounts
                .get(&0)
                .map(|a| a.account_xpub)
                .ok_or_else(|| {
                    PlatformWalletError::TransactionBuild(
                        "BIP-44 account 0 not found in wallet".to_string(),
                    )
                })?;

            let current_height = info.core_wallet.synced_height();

            let managed_account = info
                .core_wallet
                .accounts
                .standard_bip44_accounts
                .get_mut(&0)
                .ok_or_else(|| {
                    PlatformWalletError::TransactionBuild(
                        "BIP-44 managed account 0 not found".to_string(),
                    )
                })?;

            // Snapshot spendable UTXOs minus any in-flight reservations from
            // a concurrent `send_to_addresses`/`send_payment` on this wallet.
            let reserved = self.reservations.snapshot();
            let spendable: Vec<_> = managed_account
                .spendable_utxos(current_height)
                .into_iter()
                .filter(|utxo| !reserved.contains(&utxo.outpoint))
                .cloned()
                .collect();
            if spendable.is_empty() {
                return Err(PlatformWalletError::NoSpendableInputs {
                    account_index,
                    account_type:
                        key_wallet::account::account_type::StandardAccountType::BIP44Account,
                    context: "all UTXOs used or reserved by in-flight transactions".to_string(),
                });
            }

            // Pick a change address no concurrent send has peeked,
            // committing the index advance under this write lock.
            let change_addr = crate::wallet::core::change_address::pick_and_reserve_change_address(
                &self.reservations,
                managed_account,
                &funding_xpub,
            )?;

            // Derive the recipient's payment address from the external pool.
            // Done *after* the change-address pick so a derivation failure
            // doesn't leave a committed funding-side change advance dangling
            // without a matching outpoint reservation.
            let key = DashpayAccountKey {
                index: account_index,
                user_identity_id: from_identity_id.to_buffer(),
                friend_identity_id: to_contact_id.to_buffer(),
            };
            let external_account = info
                .core_wallet
                .accounts
                .dashpay_external_accounts
                .get_mut(&key)
                .ok_or_else(|| {
                    PlatformWalletError::InvalidIdentityData(format!(
                        "No managed DashpayExternalAccount found for contact {}",
                        to_contact_id
                    ))
                })?;
            let payment_address = external_account
                .next_address(Some(&contact_xpub), true)
                .map_err(|e| PlatformWalletError::TransactionBuild(e.to_string()))?;

            // Re-borrow the managed funding account for the builder (the
            // external_account borrow above ended at `payment_address`).
            let managed_account = info
                .core_wallet
                .accounts
                .standard_bip44_accounts
                .get_mut(&0)
                .ok_or_else(|| {
                    PlatformWalletError::TransactionBuild(
                        "BIP-44 managed account 0 not found".to_string(),
                    )
                })?;

            let builder = TransactionBuilder::new()
                .set_current_height(current_height)
                .set_selection_strategy(SelectionStrategy::LargestFirst)
                .set_change_address(change_addr.clone())
                .add_inputs(spendable.iter().cloned())
                .add_output(&payment_address, amount_duffs);

            let (tx, _fee) = builder
                .build_signed(wallet, |addr| {
                    managed_account.address_derivation_path(&addr)
                })
                .await
                .map_err(|e| {
                    crate::wallet::core::broadcast::classify_build_error(
                        e,
                        key_wallet::account::account_type::StandardAccountType::BIP44Account,
                        account_index,
                    )
                })?;

            // Defense-in-depth: confirm the builder picked only outpoints
            // from our pre-filtered spendable snapshot.
            let selected: BTreeSet<OutPoint> =
                tx.input.iter().map(|txin| txin.previous_output).collect();
            let spendable_outpoints: BTreeSet<OutPoint> =
                spendable.iter().map(|utxo| utxo.outpoint).collect();
            if !selected.is_subset(&spendable_outpoints) {
                return Err(PlatformWalletError::ConcurrentSpendConflict {
                    selected: selected.into_iter().collect(),
                });
            }

            // Second defense-in-depth net, in parity with
            // `CoreWallet::send_to_addresses`: re-snapshot the funding
            // account's spendable UTXOs after `build_signed` and confirm
            // every selected outpoint is still present. See
            // [`crate::wallet::core::broadcast::assert_selected_still_spendable`].
            let fresh_spendable_outpoints: BTreeSet<OutPoint> = managed_account
                .spendable_utxos(current_height)
                .into_iter()
                .map(|utxo| utxo.outpoint)
                .collect();
            crate::wallet::core::broadcast::assert_selected_still_spendable(
                &selected,
                &fresh_spendable_outpoints,
            )?;

            // Atomic reject-overlap: `reserve` returns `None` if a concurrent
            // caller grabbed any selected outpoint or the change address between
            // our prefilter and here. Unreachable today (both filters run under
            // this same write lock), but handled defensively as the retryable
            // concurrent-spend conflict the prefilter would otherwise surface.
            let reservation = self
                .reservations
                .reserve(selected.iter().copied().collect(), Some(change_addr))
                .ok_or_else(|| PlatformWalletError::ConcurrentSpendConflict {
                    selected: selected.iter().copied().collect(),
                })?;

            (payment_address, tx, reservation)
        };

        // Broadcast + reconcile via the shared post-broadcast helper.
        // The hook records the outgoing `PaymentEntry` on the sender's
        // `ManagedIdentity` inside the same write lock the reconcile uses
        // — keeping the entry recording in the same critical section
        // ensures it cannot race a concurrent state mutation between
        // `check_core_transaction` and `record_dashpay_payment`.
        let entry = crate::wallet::identity::types::dashpay::payment::PaymentEntry::new_sent(
            *to_contact_id,
            amount_duffs,
            memo,
        );
        let entry_for_hook = entry.clone();
        let persister = self.persister.clone();
        let from_identity = *from_identity_id;

        let txid = crate::wallet::core::broadcast::broadcast_and_reconcile(
            &self.wallet_manager,
            self.wallet_id,
            &self.broadcaster,
            &tx,
            _reservation,
            move |info, txid, _reconciled| {
                if let Some(managed) = info.identity_manager.managed_identity_mut(&from_identity) {
                    managed.record_dashpay_payment(txid.to_string(), entry_for_hook, &persister);
                }
            },
        )
        .await?;

        tracing::info!(
            from_identity = %from_identity_id,
            to_contact = %to_contact_id,
            amount_duffs,
            %txid,
            payment_address = %payment_address,
            "DashPay payment broadcast"
        );

        Ok((txid, entry))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    use async_trait::async_trait;
    use dashcore::hashes::Hash;
    use dashcore::{OutPoint, Transaction, Txid};
    use dpp::identity::v0::IdentityV0;
    use dpp::identity::Identity;
    use dpp::prelude::Identifier;
    use key_wallet::account::account_type::AccountType;
    use key_wallet::wallet::initialization::WalletAccountCreationOptions;
    use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface as _;
    use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
    use key_wallet::wallet::Wallet;
    use key_wallet::{Network, Utxo};
    use key_wallet_manager::WalletManager;
    use tokio::sync::{Notify, RwLock};

    use super::*;
    use crate::broadcaster::TransactionBroadcaster;
    use crate::error::PlatformWalletError;
    use crate::wallet::core::balance::WalletBalance;
    use crate::wallet::core::reservations::OutpointReservations;
    use crate::wallet::identity::types::dashpay::payment::PaymentDirection;
    use crate::wallet::identity::IdentityManager;
    use crate::wallet::persister::{NoPlatformPersistence, WalletPersister};
    use crate::wallet::platform_wallet::{PlatformWalletInfo, WalletId};

    /// The single seeded UTXO's outpoint — fixed so assertions can name it.
    fn funding_outpoint() -> OutPoint {
        OutPoint::new(Txid::from_byte_array([7u8; 32]), 0)
    }

    /// In-memory signer backed by the test wallet's mnemonic root, so
    /// `build_signed` can sign the seeded UTXO.
    struct InMemorySigner {
        root: key_wallet::wallet::root_extended_keys::RootExtendedPrivKey,
        network: Network,
    }

    const IN_MEMORY_METHODS: &[key_wallet::signer::SignerMethod] =
        &[key_wallet::signer::SignerMethod::Digest];

    #[async_trait]
    impl key_wallet::signer::Signer for InMemorySigner {
        type Error = String;

        fn supported_methods(&self) -> &[key_wallet::signer::SignerMethod] {
            IN_MEMORY_METHODS
        }

        async fn sign_ecdsa(
            &self,
            path: &key_wallet::bip32::DerivationPath,
            sighash: [u8; 32],
        ) -> Result<
            (
                dashcore::secp256k1::ecdsa::Signature,
                dashcore::secp256k1::PublicKey,
            ),
            Self::Error,
        > {
            let secp = dashcore::secp256k1::Secp256k1::new();
            let xpriv = self
                .root
                .to_extended_priv_key(self.network)
                .derive_priv(&secp, path)
                .map_err(|e| e.to_string())?;
            let msg = dashcore::secp256k1::Message::from_digest(sighash);
            let sig = secp.sign_ecdsa(&msg, &xpriv.private_key);
            let pk = dashcore::secp256k1::PublicKey::from_secret_key(&secp, &xpriv.private_key);
            Ok((sig, pk))
        }

        async fn public_key(
            &self,
            path: &key_wallet::bip32::DerivationPath,
        ) -> Result<dashcore::secp256k1::PublicKey, Self::Error> {
            let secp = dashcore::secp256k1::Secp256k1::new();
            let xpriv = self
                .root
                .to_extended_priv_key(self.network)
                .derive_priv(&secp, path)
                .map_err(|e| e.to_string())?;
            Ok(dashcore::secp256k1::PublicKey::from_secret_key(
                &secp,
                &xpriv.private_key,
            ))
        }
    }

    fn root_from(wallet: &Wallet) -> key_wallet::wallet::root_extended_keys::RootExtendedPrivKey {
        match &wallet.wallet_type {
            key_wallet::wallet::WalletType::Mnemonic {
                root_extended_private_key,
                ..
            } => root_extended_private_key.clone(),
            _ => unreachable!("test wallets are mnemonic"),
        }
    }

    /// Mock broadcaster that gates `broadcast()` on an external `Notify`.
    /// `entered` fires the instant `broadcast()` is awaited — by then the
    /// caller has reserved its outpoint and dropped the wallet write lock.
    struct GatedBroadcaster {
        gate: Arc<Notify>,
        entered: Arc<Notify>,
        calls: AtomicUsize,
    }

    #[async_trait]
    impl TransactionBroadcaster for GatedBroadcaster {
        async fn broadcast(&self, transaction: &Transaction) -> Result<Txid, PlatformWalletError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            self.entered.notify_one();
            self.gate.notified().await;
            Ok(transaction.txid())
        }
    }

    /// Build a single-wallet manager with one funded BIP-44 account 0,
    /// a registered watch-only `DashpayExternalAccount` for the
    /// `(from_identity, to_contact)` pair, and a sending `ManagedIdentity`.
    /// Returns the manager handle, wallet id, the two identity ids, and a
    /// signer that owns the seeded UTXO.
    fn build_dashpay_wallet(
        utxo_value: u64,
    ) -> (
        Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
        WalletId,
        Identifier,
        Identifier,
        InMemorySigner,
    ) {
        let from_identity = Identifier::from([0xA1; 32]);
        let to_contact = Identifier::from([0xB2; 32]);

        let wallet = Wallet::new_random(Network::Testnet, WalletAccountCreationOptions::Default)
            .expect("test wallet");
        let signer = InMemorySigner {
            root: root_from(&wallet),
            network: Network::Testnet,
        };

        // Reuse the funding account's xpub as the contact's watch-only
        // xpub: `send_payment` only derives a payment address from it, and
        // the test asserts plumbing, not contact-key provenance.
        let contact_xpub = wallet
            .accounts
            .standard_bip44_accounts
            .get(&0)
            .expect("bip44 account 0")
            .account_xpub;

        let mut wallet = wallet;
        let dashpay_account_type = AccountType::DashpayExternalAccount {
            index: 0,
            user_identity_id: from_identity.to_buffer(),
            friend_identity_id: to_contact.to_buffer(),
        };
        wallet
            .add_account(dashpay_account_type, Some(contact_xpub))
            .expect("register watch-only dashpay external account");

        let xpub = wallet
            .accounts
            .standard_bip44_accounts
            .get(&0)
            .expect("bip44 account 0")
            .account_xpub;

        let mut wallet_info = ManagedWalletInfo::from_wallet(&wallet, 0);
        wallet_info.update_synced_height(100);

        // Mirror the watch-only managed account so the external pool is
        // available for address derivation, exactly as
        // `register_external_contact_account` does.
        let dashpay_account = key_wallet::Account {
            parent_wallet_id: None,
            account_type: dashpay_account_type,
            network: Network::Testnet,
            account_xpub: contact_xpub,
            is_watch_only: true,
        };
        wallet_info
            .accounts
            .insert_funds_bearing_account(
                key_wallet::managed_account::ManagedCoreFundsAccount::from_account(
                    &dashpay_account,
                ),
            )
            .expect("insert watch-only managed dashpay account");

        let funding_address = wallet_info
            .accounts
            .standard_bip44_accounts
            .get_mut(&0)
            .expect("managed bip44 account 0")
            .next_receive_address(Some(&xpub), true)
            .expect("derive funding address");

        let outpoint = funding_outpoint();
        let mut utxo = Utxo::new(
            outpoint,
            dashcore::TxOut {
                value: utxo_value,
                script_pubkey: funding_address.script_pubkey(),
            },
            funding_address,
            1,
            false,
        );
        utxo.is_confirmed = true;
        wallet_info
            .accounts
            .standard_bip44_accounts
            .get_mut(&0)
            .expect("managed bip44 account 0")
            .utxos
            .insert(outpoint, utxo);

        let mut identity_manager = IdentityManager::new();

        let info = PlatformWalletInfo {
            core_wallet: wallet_info,
            balance: Arc::new(WalletBalance::new()),
            identity_manager: IdentityManager::new(),
            tracked_asset_locks: BTreeMap::new(),
        };

        let mut wm: WalletManager<PlatformWalletInfo> = WalletManager::new(Network::Testnet);
        let wallet_id = wm.insert_wallet(wallet, info).expect("insert wallet");

        // Add the sending identity through the same persister the wallet
        // uses, so `record_dashpay_payment` finds it post-broadcast.
        let persister = WalletPersister::new(wallet_id, Arc::new(NoPlatformPersistence));
        identity_manager
            .add_identity(make_test_identity(from_identity), 0, wallet_id, &persister)
            .expect("add sending identity");
        wm.get_wallet_info_mut(&wallet_id)
            .expect("info")
            .identity_manager = identity_manager;

        (
            Arc::new(RwLock::new(wm)),
            wallet_id,
            from_identity,
            to_contact,
            signer,
        )
    }

    fn make_test_identity(id: Identifier) -> Identity {
        Identity::V0(IdentityV0 {
            id,
            public_keys: BTreeMap::new(),
            balance: 0,
            revision: 0,
        })
    }

    /// Construct an `IdentityWallet` wired to a mock broadcaster and a
    /// shared reservation set, mirroring how `PlatformWallet::new` wires
    /// the identity handle to its sibling `CoreWallet`'s reservations.
    fn make_identity_wallet<B: TransactionBroadcaster + ?Sized>(
        wm: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
        wallet_id: WalletId,
        broadcaster: Arc<B>,
        reservations: OutpointReservations,
    ) -> IdentityWallet<B> {
        let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
        let persister = WalletPersister::new(wallet_id, Arc::new(NoPlatformPersistence));
        let event_manager = Arc::new(crate::events::PlatformEventManager::new(vec![]));
        let spv = Arc::new(crate::spv::SpvRuntime::new(Arc::clone(&wm), event_manager));
        // `asset_locks` is pinned to `SpvBroadcaster` but is never exercised
        // by `send_payment` — the never-started runtime only satisfies the
        // type.
        let asset_locks = Arc::new(crate::wallet::asset_lock::manager::AssetLockManager::new(
            Arc::clone(&sdk),
            Arc::clone(&wm),
            wallet_id,
            Arc::new(Notify::new()),
            Arc::new(crate::broadcaster::SpvBroadcaster::new(spv)),
            persister.clone(),
        ));

        IdentityWallet {
            sdk,
            wallet_manager: wm,
            wallet_id,
            asset_locks,
            persister,
            broadcaster,
            reservations,
        }
    }

    // The post-build fresh-spendable re-snapshot check
    // (`assert_selected_still_spendable`, called from `send_payment`) is
    // covered as a pure set check by unit tests in `core::broadcast`. A
    // path-level test that trips it through `send_payment` is not added
    // here: `build_signed` and the re-snapshot run under one held write
    // lock, so no in-process task can drop a selected UTXO between them
    // without a production-side mutation hook. That's the very condition
    // the check is a forward-compat net for.
    // TODO: add a send_payment-path trip test once a UTXO mutator can run
    // outside the send-flow write lock (the change that makes the check
    // reachable).

    /// Happy path: `send_payment` records a Sent `PaymentEntry` on the
    /// sending identity, and the outpoint reserved during the build is
    /// released once the broadcast reconciles successfully.
    #[tokio::test]
    async fn send_payment_records_entry_and_releases_reservation() {
        let (wm, wallet_id, from_identity, to_contact, _signer) = build_dashpay_wallet(2_000_000);

        let gate = Arc::new(Notify::new());
        let entered = Arc::new(Notify::new());
        let broadcaster = Arc::new(GatedBroadcaster {
            gate: Arc::clone(&gate),
            entered: Arc::clone(&entered),
            calls: AtomicUsize::new(0),
        });
        let reservations = OutpointReservations::new();
        let identity = make_identity_wallet(
            Arc::clone(&wm),
            wallet_id,
            Arc::clone(&broadcaster) as Arc<dyn TransactionBroadcaster>,
            reservations.clone(),
        );

        let send = identity.clone();
        let handle = tokio::spawn(async move {
            send.send_payment(&from_identity, &to_contact, 100_000, Some("lunch".into()))
                .await
        });

        // Once the broadcast is entered, the outpoint is reserved and the
        // wallet write lock is dropped — observe the reservation held.
        entered.notified().await;
        assert!(
            reservations.contains(&funding_outpoint()),
            "the selected outpoint must be reserved while the broadcast is in flight"
        );

        // Let the broadcast complete; the post-broadcast reconcile then
        // releases the reservation and records the entry.
        gate.notify_one();
        let (_txid, entry) = handle
            .await
            .expect("task panicked")
            .expect("send_payment ok");

        assert_eq!(entry.direction, PaymentDirection::Sent);
        assert_eq!(entry.counterparty_id, to_contact);
        assert_eq!(entry.amount_duffs, 100_000);
        assert_eq!(entry.memo.as_deref(), Some("lunch"));

        // (a) Entry recorded on the sending identity.
        {
            let wm_guard = wm.read().await;
            let managed = wm_guard
                .get_wallet_info(&wallet_id)
                .expect("info")
                .identity_manager
                .managed_identity(&from_identity)
                .expect("sending identity present");
            assert_eq!(
                managed.dashpay_payments.len(),
                1,
                "exactly one outgoing payment must be recorded on the sender"
            );
            let recorded = managed.dashpay_payments.values().next().expect("entry");
            assert_eq!(recorded.direction, PaymentDirection::Sent);
            assert_eq!(recorded.counterparty_id, to_contact);
        }

        // (b) Reservation released after a successful reconcile.
        assert!(
            !reservations.contains(&funding_outpoint()),
            "the outpoint reservation must be released after a successful reconcile"
        );
        assert_eq!(
            broadcaster.calls.load(Ordering::SeqCst),
            1,
            "the broadcaster must be called exactly once"
        );
    }

    /// (c) When the funding outpoint is already reserved by a concurrent
    /// in-flight send (core `send_to_addresses` or another `send_payment`),
    /// `send_payment` short-circuits with `NoSpendableInputs` — the
    /// race-loser never reaches the broadcaster.
    #[tokio::test]
    async fn send_payment_loses_utxo_race_with_no_spendable_inputs() {
        let (wm, wallet_id, from_identity, to_contact, _signer) = build_dashpay_wallet(2_000_000);

        let broadcaster = Arc::new(GatedBroadcaster {
            gate: Arc::new(Notify::new()),
            entered: Arc::new(Notify::new()),
            calls: AtomicUsize::new(0),
        });
        let reservations = OutpointReservations::new();
        let identity = make_identity_wallet(
            Arc::clone(&wm),
            wallet_id,
            Arc::clone(&broadcaster) as Arc<dyn TransactionBroadcaster>,
            reservations.clone(),
        );

        // Simulate a concurrent send holding the wallet's only UTXO.
        let _guard = reservations
            .reserve(vec![funding_outpoint()], None)
            .expect("reserve");

        let result = identity
            .send_payment(&from_identity, &to_contact, 100_000, None)
            .await;

        assert!(
            matches!(result, Err(PlatformWalletError::NoSpendableInputs { .. })),
            "the race-loser must short-circuit with NoSpendableInputs; got: {result:?}"
        );
        assert_eq!(
            broadcaster.calls.load(Ordering::SeqCst),
            0,
            "the race-loser must never reach the broadcaster"
        );
    }
}
