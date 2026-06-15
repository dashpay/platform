//! DashPay payment recording and send-to-contact flows.

use dpp::prelude::Identifier;
use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;

use std::sync::Arc;

use tokio::sync::RwLock;

use key_wallet_manager::WalletManager;

use super::*;
use crate::broadcaster::TransactionBroadcaster;
use crate::error::PlatformWalletError;
use crate::wallet::platform_wallet::{PlatformWalletInfo, WalletId};

// ---------------------------------------------------------------------------
// Incoming payment recording + reconcile
// ---------------------------------------------------------------------------

impl<B: TransactionBroadcaster + ?Sized> IdentityWallet<B> {
    /// Derive missing `Received` [`PaymentEntry`]s from the wallet's
    /// `DashpayReceivingFunds` accounts' UTXO sets.
    ///
    /// Recovery path for incoming-payment history: live detection
    /// ([`record_incoming_dashpay_payments`]) only fires while the app
    /// is running, so payments received before a relaunch (whose UTXOs
    /// are restored from persistence) or during a missed event window
    /// would otherwise never appear in the payment history. Runs as a
    /// local-only step of `dashpay_sync()` — no network round-trips.
    ///
    /// Idempotent: entries are keyed by txid and an existing entry for
    /// a txid (including the owner's own `Sent` record when both
    /// identities live in one wallet) is never overwritten.
    ///
    /// Returns the number of newly recorded entries.
    pub async fn reconcile_incoming_payments(&self) -> Result<usize, PlatformWalletError> {
        use std::collections::BTreeMap;

        let mut wm = self.wallet_manager.write().await;
        let Some(info) = wm.get_wallet_info_mut(&self.wallet_id) else {
            return Ok(0);
        };

        // Sum per (owner, contact, txid) first so the immutable borrow
        // of the account collection ends before the identity-manager
        // mutations below. Multiple outputs of one tx to the same
        // receival account collapse into a single entry.
        let mut totals: BTreeMap<(Identifier, Identifier, String), u64> = BTreeMap::new();
        for (key, account) in &info.core_wallet.accounts.dashpay_receival_accounts {
            for utxo in account.utxos.values() {
                let txid = utxo.outpoint.txid.to_string();
                *totals
                    .entry((
                        Identifier::from(key.user_identity_id),
                        Identifier::from(key.friend_identity_id),
                        txid,
                    ))
                    .or_default() += utxo.txout.value;
            }
        }

        let mut recorded = 0usize;
        for ((owner, contact, txid), amount_duffs) in totals {
            let Some(managed) = info.identity_manager.managed_identity_mut(&owner) else {
                continue;
            };
            if managed.dashpay_payments.contains_key(&txid) {
                continue;
            }
            tracing::info!(
                owner = %owner,
                contact = %contact,
                %txid,
                amount_duffs,
                "Recording reconciled incoming DashPay payment"
            );
            // Self-healing path: a failed persist is re-derived from UTXOs
            // on the next reconcile sweep, so log and continue.
            if let Err(e) = managed.record_dashpay_payment(
                txid,
                crate::wallet::identity::types::dashpay::payment::PaymentEntry::new_received(
                    contact,
                    amount_duffs,
                    None,
                ),
                &self.persister,
            ) {
                tracing::warn!(error = %e, "Failed to persist reconciled payment; will retry next sweep");
            }
            recorded += 1;
        }
        Ok(recorded)
    }
}

/// Record `Received` [`PaymentEntry`]s for a freshly detected Core
/// transaction whose outputs pay DashPay receival-account addresses.
///
/// Live-detection half of incoming-payment recording: called by the
/// wallet-event adapter
/// ([`spawn_wallet_event_adapter`](crate::changeset::core_bridge::spawn_wallet_event_adapter))
/// on every `TransactionDetected` event, so a payment from a contact
/// lands in the receiver's payment history the moment SPV sees the
/// transaction. The recurring [`IdentityWallet::reconcile_incoming_payments`]
/// sweep covers anything this misses (relaunch restore, dropped events).
///
/// Idempotent per txid — re-detections of the same transaction
/// (mempool → in-block → chain-locked) hit the existing-entry guard.
pub(crate) async fn record_incoming_dashpay_payments(
    wallet_manager: &Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    wallet_id: &WalletId,
    persister: &crate::wallet::persister::WalletPersister,
    record: &key_wallet::managed_account::transaction_record::TransactionRecord,
) {
    use key_wallet::managed_account::transaction_record::OutputRole;
    use std::collections::BTreeMap;

    // Candidate outputs: received by us, with a decodable address.
    // Change outputs can't be DashPay-incoming (they pay back to our
    // standard accounts), so only `Received` is considered.
    let candidates: Vec<(dashcore::Address, u64)> = record
        .output_details
        .iter()
        .filter(|d| matches!(d.role, OutputRole::Received))
        .filter_map(|d| Some((d.address.clone()?, d.value)))
        .collect();
    if candidates.is_empty() {
        return;
    }
    let txid = record.txid.to_string();

    let mut wm = wallet_manager.write().await;
    let Some(info) = wm.get_wallet_info_mut(wallet_id) else {
        return;
    };

    let mut totals: BTreeMap<(Identifier, Identifier), u64> = BTreeMap::new();
    for (address, value) in candidates {
        if let Some(m) = IdentityWallet::<crate::broadcaster::SpvBroadcaster>::match_in_collection(
            info, &address,
        ) {
            *totals
                .entry((m.user_identity_id, m.friend_identity_id))
                .or_default() += value;
        }
    }

    for ((owner, contact), amount_duffs) in totals {
        let Some(managed) = info.identity_manager.managed_identity_mut(&owner) else {
            continue;
        };
        if managed.dashpay_payments.contains_key(&txid) {
            continue;
        }
        tracing::info!(
            owner = %owner,
            contact = %contact,
            %txid,
            amount_duffs,
            "Recording incoming DashPay payment"
        );
        // Self-healing: a failed persist of a live-detected Received entry
        // is re-derived from UTXOs by the next reconcile sweep.
        if let Err(e) = managed.record_dashpay_payment(
            txid.clone(),
            crate::wallet::identity::types::dashpay::payment::PaymentEntry::new_received(
                contact,
                amount_duffs,
                None,
            ),
            persister,
        ) {
            tracing::warn!(error = %e, "Failed to persist live incoming payment; will retry next sweep");
        }
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
        use key_wallet::account::account_collection::DashpayAccountKey;
        use key_wallet::wallet::managed_wallet_info::coin_selection::SelectionStrategy;
        use key_wallet::wallet::managed_wallet_info::transaction_builder::TransactionBuilder;
        use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;

        let account_index: u32 = 0;

        let (payment_address, tx) = {
            let mut wm = self.wallet_manager.write().await;

            // Resolve the external account's xpub so we can derive addresses.
            let contact_xpub = {
                // Look up the external account in the *immutable* AccountCollection on
                // `Wallet`. The ManagedAccountCollection only stores the managed state;
                // the xpub lives on the immutable Account in `wallet.accounts`.
                // For a watch-only external account we stored the contact's xpub directly
                // as `account_xpub` on the Account struct — look it up via DashpayAccountKey.
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

            // Derive the next unused address from the external account's address pool.
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
            let account = wallet
                .accounts
                .standard_bip44_accounts
                .get(&0)
                .ok_or_else(|| {
                    PlatformWalletError::TransactionBuild(
                        "BIP-44 account 0 not found in wallet".to_string(),
                    )
                })?;

            let builder = TransactionBuilder::new()
                .set_current_height(current_height)
                .set_selection_strategy(SelectionStrategy::LargestFirst)
                .set_funding(managed_account, account)
                .add_output(&payment_address, amount_duffs);

            let (tx, _fee) = builder
                .build_signed(wallet, |addr| {
                    managed_account.address_derivation_path(&addr)
                })
                .await
                .map_err(|e| PlatformWalletError::TransactionBuild(e.to_string()))?;

            (payment_address, tx)
        };

        // --- 3. Broadcast the transaction. ---
        let txid = self
            .broadcaster
            .broadcast(&tx)
            .await
            .map_err(|e| PlatformWalletError::TransactionBroadcast(e.to_string()))?;

        tracing::info!(
            from_identity = %from_identity_id,
            to_contact = %to_contact_id,
            amount_duffs,
            %txid,
            payment_address = %payment_address,
            "DashPay payment broadcast"
        );

        // --- 4. Record the outgoing payment on the sender's ManagedIdentity. ---
        let entry = crate::wallet::identity::types::dashpay::payment::PaymentEntry::new_sent(
            *to_contact_id,
            amount_duffs,
            memo,
        );
        {
            let mut wm = self.wallet_manager.write().await;
            if let Some(info) = wm.get_wallet_info_mut(&self.wallet_id) {
                if let Some(managed) = info.identity_manager.managed_identity_mut(from_identity_id)
                {
                    // Propagate a persist failure: the tx is already
                    // broadcast on-chain, but the local Sent entry + memo
                    // has no on-chain recovery, so a silent drop would lose
                    // the user's payment record. Surfacing it lets the UI
                    // report the partial outcome (sent, but not recorded).
                    managed
                        .record_dashpay_payment(txid.to_string(), entry.clone(), &self.persister)
                        .map_err(|e| {
                            PlatformWalletError::Persistence(format!(
                                "payment broadcast but not recorded locally: {e}"
                            ))
                        })?;
                }
            }
        }

        Ok((txid, entry))
    }
}

#[cfg(test)]
mod tests {
    //! Receiver-side payment persistence tests.
    //!
    //! These pin the three pieces that make incoming DashPay payments
    //! survive across app relaunches (UAT 2026-06-12 found all three
    //! missing — Alice's received payments showed "Payments (0)"):
    //!
    //! 1. `register_contact_account` must PERSIST the account
    //!    registration, so the `DashpayReceivingFunds` account is
    //!    rebuilt at next load and its persisted UTXOs route instead
    //!    of being dropped (`dropped_no_account`).
    //! 2. `reconcile_incoming_payments` must derive missing
    //!    `Received` PaymentEntries from the receival accounts' UTXO
    //!    sets (recovers history after restore and any missed live
    //!    events).
    //! 3. Reconcile must be idempotent and never clobber an existing
    //!    entry for the same txid.

    use std::collections::BTreeMap;
    use std::sync::{Arc, Mutex};

    use dpp::identity::v0::IdentityV0;
    use dpp::identity::Identity;
    use dpp::prelude::Identifier;
    use key_wallet::account::account_collection::DashpayAccountKey;
    use key_wallet::mnemonic::{Language, Mnemonic};
    use key_wallet::wallet::initialization::WalletAccountCreationOptions;
    use key_wallet::Network;

    use crate::changeset::{
        ClientStartState, PersistenceError, PlatformWalletChangeSet, PlatformWalletPersistence,
    };
    use crate::error::PlatformWalletError;
    use crate::events::{EventHandler, PlatformEventHandler};
    use crate::wallet::persister::WalletPersister;
    use crate::wallet::platform_wallet::WalletId;
    use crate::PlatformWalletManager;

    const TEST_MNEMONIC: &str = "abandon abandon abandon abandon abandon abandon \
         abandon abandon abandon abandon abandon about";

    /// Persister that records every store round so tests can assert on
    /// exactly what would reach the host (SwiftData) for a given flow.
    #[derive(Default)]
    struct RecordingPersister {
        stores: Mutex<Vec<(WalletId, PlatformWalletChangeSet)>>,
    }

    impl PlatformWalletPersistence for RecordingPersister {
        fn store(
            &self,
            wallet_id: WalletId,
            changeset: PlatformWalletChangeSet,
        ) -> Result<(), PersistenceError> {
            self.stores.lock().unwrap().push((wallet_id, changeset));
            Ok(())
        }

        fn flush(&self, _wallet_id: WalletId) -> Result<(), PersistenceError> {
            Ok(())
        }

        fn load(&self) -> Result<ClientStartState, PersistenceError> {
            Ok(ClientStartState::default())
        }
    }

    struct NoopEventHandler;
    impl EventHandler for NoopEventHandler {}
    impl PlatformEventHandler for NoopEventHandler {}

    async fn make_wallet() -> (
        Arc<PlatformWalletManager<RecordingPersister>>,
        Arc<RecordingPersister>,
        WalletId,
    ) {
        let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
        let persister = Arc::new(RecordingPersister::default());
        let handler: Arc<dyn PlatformEventHandler> = Arc::new(NoopEventHandler);
        let manager = Arc::new(PlatformWalletManager::new(
            sdk,
            Arc::clone(&persister),
            handler,
        ));
        let mnemonic =
            Mnemonic::from_phrase(TEST_MNEMONIC, Language::English).expect("valid mnemonic");
        let wallet = manager
            .create_wallet_from_seed_bytes(
                Network::Testnet,
                mnemonic.to_seed(""),
                WalletAccountCreationOptions::Default,
                Some(0),
            )
            .await
            .expect("wallet creation");
        let wallet_id = wallet.wallet_id();
        (manager, persister, wallet_id)
    }

    fn bare_identity(id_bytes: [u8; 32]) -> Identity {
        Identity::V0(IdentityV0 {
            id: Identifier::from(id_bytes),
            public_keys: BTreeMap::new(),
            balance: 0,
            revision: 0,
        })
    }

    /// Insert a fake UTXO into the (owner, contact) receival account,
    /// paying `value_duffs` to the account's first pool address, and
    /// return the txid hex string used.
    async fn plant_receival_utxo(
        manager: &Arc<PlatformWalletManager<RecordingPersister>>,
        wallet_id: WalletId,
        owner: Identifier,
        contact: Identifier,
        txid_byte: u8,
        value_duffs: u64,
    ) -> String {
        use dashcore::hashes::Hash;
        let wallet = manager
            .get_wallet(&wallet_id)
            .await
            .expect("wallet registered");
        let iw = wallet.identity();
        let mut wm = iw.wallet_manager.write().await;
        let info = wm.get_wallet_info_mut(&wallet_id).expect("wallet info");
        let key = DashpayAccountKey {
            index: 0,
            user_identity_id: owner.to_buffer(),
            friend_identity_id: contact.to_buffer(),
        };
        let account = info
            .core_wallet
            .accounts
            .dashpay_receival_accounts
            .get_mut(&key)
            .expect("receival account registered");
        let address_info = {
            use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
            account
                .managed_account_type()
                .address_pools()
                .first()
                .expect("receival account has a pool")
                .addresses
                .values()
                .next()
                .expect("pool has at least one derived address")
                .clone()
        };
        let txid = dashcore::Txid::from_slice(&[txid_byte; 32]).expect("txid");
        let outpoint = dashcore::OutPoint { txid, vout: 0 };
        account.utxos.insert(
            outpoint,
            key_wallet::Utxo {
                outpoint,
                txout: dashcore::TxOut {
                    value: value_duffs,
                    script_pubkey: address_info.script_pubkey.clone(),
                },
                address: address_info.address.clone(),
                height: 100,
                is_coinbase: false,
                is_confirmed: true,
                is_instantlocked: false,
                is_locked: false,
                is_trusted: false,
            },
        );
        txid.to_string()
    }

    /// 1. Registering a contact receival account must persist an
    /// `AccountRegistrationEntry` — otherwise the account (and every
    /// UTXO routed to it) silently vanishes on the next app launch
    /// (`load: ... dropped_no_account` observed live on devnet).
    #[tokio::test]
    async fn register_contact_account_persists_account_registration() {
        let (manager, persister, wallet_id) = make_wallet().await;
        let owner = Identifier::from([0xAA; 32]);
        let contact = Identifier::from([0xBB; 32]);

        persister.stores.lock().unwrap().clear();

        {
            let wallet = manager.get_wallet(&wallet_id).await.expect("wallet");
            wallet
                .identity()
                .register_contact_account(&owner, &contact, 0)
                .await
                .expect("register_contact_account");
        }

        {
            let stores = persister.stores.lock().unwrap();
            let registered = stores.iter().any(|(_, cs)| {
                cs.account_registrations.iter().any(|entry| {
                    matches!(
                        entry.account_type,
                        key_wallet::account::AccountType::DashpayReceivingFunds {
                            user_identity_id,
                            friend_identity_id,
                            ..
                        } if user_identity_id == owner.to_buffer()
                            && friend_identity_id == contact.to_buffer()
                    )
                })
            });
            assert!(
                registered,
                "register_contact_account must emit an AccountRegistrationEntry \
                 so the DashpayReceivingFunds account survives relaunch"
            );
        }

        // Re-registering must be a no-op (no duplicate persistence round).
        persister.stores.lock().unwrap().clear();
        {
            let wallet = manager.get_wallet(&wallet_id).await.expect("wallet");
            wallet
                .identity()
                .register_contact_account(&owner, &contact, 0)
                .await
                .expect("re-register is a no-op");
        }
        let stores = persister.stores.lock().unwrap();
        assert!(
            stores
                .iter()
                .all(|(_, cs)| cs.account_registrations.is_empty()),
            "re-registering an existing contact account must not re-persist"
        );
    }

    /// 2. Reconcile derives `Received` entries from receival-account
    /// UTXOs (restores payment history after relaunch / missed events),
    /// and 3. is idempotent across passes.
    #[tokio::test]
    async fn reconcile_records_received_payments_from_receival_utxos() {
        let (manager, persister, wallet_id) = make_wallet().await;
        let owner = Identifier::from([0xAA; 32]);
        let contact = Identifier::from([0xBB; 32]);

        {
            let wallet = manager.get_wallet(&wallet_id).await.expect("wallet");
            let iw = wallet.identity();
            iw.register_contact_account(&owner, &contact, 0)
                .await
                .expect("register_contact_account");
            // The owner identity must be managed for the entry to land.
            let mut wm = iw.wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&wallet_id).expect("info");
            info.identity_manager
                .add_identity(
                    bare_identity([0xAA; 32]),
                    0,
                    wallet_id,
                    &WalletPersister::new(wallet_id, Arc::clone(&persister) as _),
                )
                .expect("add managed identity");
        }

        let txid = plant_receival_utxo(&manager, wallet_id, owner, contact, 0x07, 1_000_000).await;

        let wallet = manager.get_wallet(&wallet_id).await.expect("wallet");
        let iw = wallet.identity();

        let recorded = iw
            .reconcile_incoming_payments()
            .await
            .expect("reconcile pass");
        assert_eq!(recorded, 1, "one missing Received entry must be recorded");

        {
            let wm = iw.wallet_manager.read().await;
            let info = wm.get_wallet_info(&wallet_id).expect("info");
            let managed = info
                .identity_manager
                .managed_identity(&owner)
                .expect("managed identity");
            let entry = managed
                .dashpay_payments
                .get(&txid)
                .expect("Received entry recorded under the UTXO's txid");
            assert_eq!(entry.counterparty_id, contact);
            assert_eq!(entry.amount_duffs, 1_000_000);
            assert_eq!(
                entry.direction,
                super::super::super::types::dashpay::payment::PaymentDirection::Received
            );
        }

        // Idempotency: a second pass records nothing new.
        let recorded_again = iw
            .reconcile_incoming_payments()
            .await
            .expect("second reconcile pass");
        assert_eq!(recorded_again, 0, "reconcile must be idempotent");
    }

    /// 3b. An existing entry under the same txid (e.g. the sender's
    /// own `Sent` record when both identities share one wallet) must
    /// not be clobbered by reconcile.
    #[tokio::test]
    async fn reconcile_does_not_clobber_existing_entry_for_same_txid() {
        let (manager, persister, wallet_id) = make_wallet().await;
        let owner = Identifier::from([0xAA; 32]);
        let contact = Identifier::from([0xBB; 32]);

        {
            let wallet = manager.get_wallet(&wallet_id).await.expect("wallet");
            let iw = wallet.identity();
            iw.register_contact_account(&owner, &contact, 0)
                .await
                .expect("register_contact_account");
            let mut wm = iw.wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&wallet_id).expect("info");
            info.identity_manager
                .add_identity(
                    bare_identity([0xAA; 32]),
                    0,
                    wallet_id,
                    &WalletPersister::new(wallet_id, Arc::clone(&persister) as _),
                )
                .expect("add managed identity");
        }

        let txid = plant_receival_utxo(&manager, wallet_id, owner, contact, 0x09, 500_000).await;

        let wallet = manager.get_wallet(&wallet_id).await.expect("wallet");
        let iw = wallet.identity();

        // Pre-record an entry under the same txid.
        let preexisting = crate::wallet::identity::types::dashpay::payment::PaymentEntry::new_sent(
            contact, 123, None,
        );
        {
            let mut wm = iw.wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&wallet_id).expect("info");
            let managed = info
                .identity_manager
                .managed_identity_mut(&owner)
                .expect("managed identity");
            managed
                .record_dashpay_payment(
                    txid.clone(),
                    preexisting.clone(),
                    &WalletPersister::new(wallet_id, Arc::clone(&persister) as _),
                )
                .expect("record");
        }

        let recorded = iw
            .reconcile_incoming_payments()
            .await
            .expect("reconcile pass");
        assert_eq!(recorded, 0, "existing txid entry must be left alone");

        let wm = iw.wallet_manager.read().await;
        let info = wm.get_wallet_info(&wallet_id).expect("info");
        let managed = info
            .identity_manager
            .managed_identity(&owner)
            .expect("managed identity");
        assert_eq!(
            managed.dashpay_payments.get(&txid),
            Some(&preexisting),
            "reconcile must not overwrite the pre-existing entry"
        );
    }

    /// Persister that succeeds until `armed`, then fails every store —
    /// lets a test build state normally, then prove a later user-initiated
    /// write propagates a persist failure instead of swallowing it.
    #[derive(Default)]
    struct ToggleFailPersister {
        armed: std::sync::atomic::AtomicBool,
    }

    impl PlatformWalletPersistence for ToggleFailPersister {
        fn store(
            &self,
            _wallet_id: WalletId,
            _changeset: PlatformWalletChangeSet,
        ) -> Result<(), PersistenceError> {
            if self.armed.load(std::sync::atomic::Ordering::SeqCst) {
                Err(PersistenceError::backend("store armed to fail"))
            } else {
                Ok(())
            }
        }
        fn flush(&self, _wallet_id: WalletId) -> Result<(), PersistenceError> {
            Ok(())
        }
        fn load(&self) -> Result<ClientStartState, PersistenceError> {
            Ok(ClientStartState::default())
        }
    }

    /// **C1 (Critical) — reject must PROPAGATE a persist failure.**
    /// The reject tombstone is local-only (no on-chain rejection), so a
    /// swallowed store error would resurrect the rejected contact on the
    /// next launch with no signal. The user-initiated `reject` path must
    /// return the error instead.
    ///
    /// RED before the fix: `reject_contact_request` logged the store error
    /// and returned `Ok(())`. GREEN: it returns `Err(Persistence)`.
    #[tokio::test]
    async fn reject_propagates_persist_failure() {
        let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
        let persister = Arc::new(ToggleFailPersister::default());
        let handler: Arc<dyn PlatformEventHandler> = Arc::new(NoopEventHandler);
        let manager = Arc::new(PlatformWalletManager::new(
            sdk,
            Arc::clone(&persister),
            handler,
        ));
        let mnemonic =
            Mnemonic::from_phrase(TEST_MNEMONIC, Language::English).expect("valid mnemonic");
        let wallet = manager
            .create_wallet_from_seed_bytes(
                Network::Testnet,
                mnemonic.to_seed(""),
                WalletAccountCreationOptions::Default,
                Some(0),
            )
            .await
            .expect("wallet creation");
        let wallet_id = wallet.wallet_id();

        let owner = Identifier::from([0xAA; 32]);
        let contact = Identifier::from([0xBB; 32]);

        // Setup (persister still succeeding): managed owner + an incoming
        // request to reject.
        {
            let iw = wallet.identity();
            let mut wm = iw.wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&wallet_id).expect("info");
            let p = WalletPersister::new(wallet_id, Arc::clone(&persister) as _);
            info.identity_manager
                .add_identity(bare_identity([0xAA; 32]), 0, wallet_id, &p)
                .expect("add owner");
            let incoming =
                crate::wallet::identity::types::dashpay::contact_request::ContactRequest::new(
                    contact,
                    owner,
                    1,
                    2,
                    0,
                    vec![7u8; 96],
                    100,
                    0,
                );
            info.identity_manager
                .managed_identity_mut(&owner)
                .expect("managed")
                .add_incoming_contact_request(incoming, &p);
        }

        // Arm the persister to fail, then reject: must return Err, NOT Ok.
        persister
            .armed
            .store(true, std::sync::atomic::Ordering::SeqCst);
        let iw = wallet.identity();
        let result = iw.reject_contact_request(&owner, &contact).await;
        assert!(
            matches!(result, Err(PlatformWalletError::Persistence(_))),
            "reject must propagate a persist failure (got {result:?}), \
             else the tombstone is lost and the contact resurrects"
        );
    }
}
