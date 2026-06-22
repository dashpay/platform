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

    /// Flip `Pending` `Sent` [`PaymentEntry`]s to `Confirmed` when the
    /// persisted core transaction record reports the transaction final.
    ///
    /// Recovery path for sent-payment confirmation. The live confirm path
    /// ([`confirm_sent_dashpay_payment`](super::confirm_sent_dashpay_payment))
    /// flips a sent payment the moment its block / InstantSend-lock event
    /// arrives, but that is a single live event: if it is missed — a lagged
    /// wallet-event broadcast, or a relaunch after the transaction confirmed
    /// but before the flip was captured — the entry would otherwise stay
    /// `Pending` forever (received payments self-heal from receival-account
    /// UTXOs; sent payments have no such ground truth). This sweep consults
    /// the persisted core tx record (txid + context) and flips any `Pending`
    /// `Sent` entry whose transaction is mined or InstantSend-locked.
    ///
    /// Runs as a local-only step of `dashpay_sync()` — one persister read
    /// per pending sent payment, no network round-trips. Idempotent: a
    /// `Confirmed` entry is left alone, and a transaction not yet final is
    /// retried on the next sweep.
    ///
    /// Returns the number of entries confirmed this pass.
    pub async fn reconcile_sent_payments(&self) -> Result<usize, PlatformWalletError> {
        use crate::wallet::identity::types::dashpay::payment::{PaymentDirection, PaymentStatus};
        use key_wallet::transaction_checking::TransactionContext;

        // Snapshot the pending sent (owner, txid) pairs under a read lock so
        // the persister reads below don't hold the wallet lock across I/O.
        let pending: Vec<(Identifier, String)> = {
            let wm = self.wallet_manager.read().await;
            let Some(info) = wm.get_wallet_info(&self.wallet_id) else {
                return Ok(0);
            };
            let mut out = Vec::new();
            for owner in info.identity_manager.identity_ids() {
                let Some(managed) = info.identity_manager.managed_identity(&owner) else {
                    continue;
                };
                for (txid, entry) in &managed.dashpay_payments {
                    if entry.direction == PaymentDirection::Sent
                        && entry.status == PaymentStatus::Pending
                    {
                        out.push((owner, txid.clone()));
                    }
                }
            }
            out
        };

        let mut confirmed = 0usize;
        for (_owner, txid_str) in pending {
            let Ok(txid) = txid_str.parse::<dashcore::Txid>() else {
                continue;
            };
            let record = match self.persister.get_core_tx_record(&txid) {
                Ok(Some(record)) => record,
                Ok(None) => continue,
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        txid = %txid_str,
                        "reconcile_sent_payments: tx-record read failed; will retry next sweep"
                    );
                    continue;
                }
            };
            // An InstantSend lock is final for DashPay display, same as a
            // mined block.
            let is_final = record.is_confirmed()
                || matches!(record.context, TransactionContext::InstantSend(_));
            if !is_final {
                continue;
            }
            // Flip in place via the shared confirm path (re-checks the
            // entry is still a `Pending` `Sent` under its own write lock,
            // so it stays correct if a live event raced this sweep).
            confirm_sent_payment_by_txid(
                &self.wallet_manager,
                &self.wallet_id,
                &self.persister,
                &txid_str,
            )
            .await;
            confirmed += 1;
        }
        Ok(confirmed)
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

/// Advance a sender's `Sent` [`PaymentEntry`] from `Pending` to
/// `Confirmed` once its broadcast transaction reaches finality.
///
/// [`IdentityWallet::send_payment`] records the outgoing entry as
/// `Pending` at broadcast time and nothing else advances it. The wallet
/// re-emits the sender's own transaction as it moves through mempool →
/// InstantSend → in-block → chain-locked, so when a re-detection reports
/// the transaction final the matching entry is flipped in place.
///
/// An **InstantSend lock counts as final** for DashPay display: it is
/// effectively irreversible, so the user sees `Confirmed` without waiting
/// for the surrounding block. A bare mempool re-detection (no IS lock, not
/// yet mined) leaves the entry `Pending` — which it genuinely still is.
/// Idempotent: once `Confirmed`, later re-detections find nothing to
/// change and skip the persistence round.
pub(crate) async fn confirm_sent_dashpay_payment(
    wallet_manager: &Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    wallet_id: &WalletId,
    persister: &crate::wallet::persister::WalletPersister,
    record: &key_wallet::managed_account::transaction_record::TransactionRecord,
) {
    use key_wallet::transaction_checking::TransactionContext;
    // Mined (InBlock / InChainLockedBlock) OR InstantSend-locked advances
    // the entry. A plain mempool sighting does not.
    let is_instant_send = matches!(record.context, TransactionContext::InstantSend(_));
    if !record.is_confirmed() && !is_instant_send {
        return;
    }
    confirm_sent_payment_by_txid(
        wallet_manager,
        wallet_id,
        persister,
        &record.txid.to_string(),
    )
    .await;
}

/// Confirm a sender's `Sent` [`PaymentEntry`] by txid alone, for a
/// [`WalletEvent::TransactionInstantLocked`](key_wallet_manager::WalletEvent::TransactionInstantLocked)
/// that applies an InstantSend lock to a previously-seen transaction.
/// That event carries no [`TransactionRecord`](key_wallet::managed_account::transaction_record::TransactionRecord),
/// only the txid; an IS lock is treated as final for DashPay display, so
/// this flips a matching `Pending` `Sent` entry to `Confirmed`. Idempotent
/// (the underlying flip skips entries already past `Pending`).
pub(crate) async fn confirm_sent_dashpay_payment_by_txid(
    wallet_manager: &Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    wallet_id: &WalletId,
    persister: &crate::wallet::persister::WalletPersister,
    txid: &dashcore::Txid,
) {
    confirm_sent_payment_by_txid(wallet_manager, wallet_id, persister, &txid.to_string()).await;
}

/// Flip the `Pending` `Sent` [`PaymentEntry`] under `txid` (if any) to
/// `Confirmed`, in place, preserving amount/memo/counterparty.
///
/// No-op when no entry exists for `txid`, it is not a `Sent` entry, or it
/// is already past `Pending` (so repeated confirmed re-detections are
/// idempotent and skip the persistence round). Separated from the event
/// glue above so the state transition is unit-testable without
/// constructing a full `TransactionRecord`.
async fn confirm_sent_payment_by_txid(
    wallet_manager: &Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    wallet_id: &WalletId,
    persister: &crate::wallet::persister::WalletPersister,
    txid: &str,
) {
    use crate::wallet::identity::types::dashpay::payment::{PaymentDirection, PaymentStatus};

    let mut wm = wallet_manager.write().await;
    let Some(info) = wm.get_wallet_info_mut(wallet_id) else {
        return;
    };

    // The sent transaction belongs to one managed identity; find the
    // `Pending` `Sent` entry under this txid and confirm it in place.
    for owner in info.identity_manager.identity_ids() {
        let Some(managed) = info.identity_manager.managed_identity_mut(&owner) else {
            continue;
        };
        let confirmed = match managed.dashpay_payments.get(txid) {
            Some(entry)
                if entry.direction == PaymentDirection::Sent
                    && entry.status == PaymentStatus::Pending =>
            {
                let mut updated = entry.clone();
                updated.status = PaymentStatus::Confirmed;
                updated
            }
            _ => continue,
        };
        tracing::info!(owner = %owner, %txid, "Confirming sent DashPay payment");
        if let Err(e) = managed.record_dashpay_payment(txid.to_string(), confirmed, persister) {
            tracing::warn!(
                error = %e,
                "Failed to persist sent-payment confirmation; will retry on next detection"
            );
        }
        // txid is unique — only one identity can hold this entry.
        break;
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
    /// * `signer`           - Keychain-backed [`key_wallet::signer::Signer`]
    ///   that produces each funding input's ECDSA signature on demand. The
    ///   wallet seed is never made resident — every signature is derived and
    ///   wiped inside the signer (mirrors `core_wallet::send_to_addresses`).
    ///
    /// # Returns
    ///
    /// The `Txid` of the broadcast transaction and the newly created
    /// [`PaymentEntry`] recording the outgoing payment.
    pub async fn send_payment<S: key_wallet::signer::Signer>(
        &self,
        from_identity_id: &Identifier,
        to_contact_id: &Identifier,
        amount_duffs: u64,
        memo: Option<String>,
        signer: &S,
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

            // Sign through the injected signer (blanket
            // `impl<S: Signer> TransactionSigner for S`) rather than the
            // resident `wallet`, so funding-input signatures are produced
            // from Keychain-derived keys without a resident seed.
            let (tx, _fee) = builder
                .build_signed(signer, |addr| {
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
    //! survive across app relaunches (without them, a recipient's received
    //! payments show "Payments (0)"):
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

    /// Persister that answers `get_core_tx_record` from a configurable
    /// in-memory map, so a test can stage the persisted core transaction
    /// state the sent-payment reconcile reads. `store`/`flush` are no-ops;
    /// `load` returns the default state.
    #[derive(Default)]
    struct RecordStorePersister {
        records: Mutex<
            std::collections::HashMap<
                dashcore::Txid,
                key_wallet::managed_account::transaction_record::TransactionRecord,
            >,
        >,
    }

    impl PlatformWalletPersistence for RecordStorePersister {
        fn store(
            &self,
            _wallet_id: WalletId,
            _changeset: PlatformWalletChangeSet,
        ) -> Result<(), PersistenceError> {
            Ok(())
        }
        fn flush(&self, _wallet_id: WalletId) -> Result<(), PersistenceError> {
            Ok(())
        }
        fn load(&self) -> Result<ClientStartState, PersistenceError> {
            Ok(ClientStartState::default())
        }
        fn get_core_tx_record(
            &self,
            _wallet_id: WalletId,
            txid: &dashcore::Txid,
        ) -> Result<
            Option<key_wallet::managed_account::transaction_record::TransactionRecord>,
            PersistenceError,
        > {
            Ok(self.records.lock().unwrap().get(txid).cloned())
        }
    }

    struct NoopEventHandler;
    impl EventHandler for NoopEventHandler {}
    impl PlatformEventHandler for NoopEventHandler {}

    /// Build a testnet wallet backed by an arbitrary persister `P`, for
    /// flows that need a persister beyond [`RecordingPersister`] (e.g. the
    /// sent-payment reconcile, which reads `get_core_tx_record`).
    async fn make_wallet_with<P: PlatformWalletPersistence + 'static>(
        persister: Arc<P>,
    ) -> (Arc<PlatformWalletManager<P>>, WalletId) {
        let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
        let handler: Arc<dyn PlatformEventHandler> = Arc::new(NoopEventHandler);
        let manager = Arc::new(PlatformWalletManager::new(
            sdk,
            Arc::clone(&persister),
            handler,
        ));
        let mnemonic =
            Mnemonic::from_phrase(TEST_MNEMONIC, Language::English).expect("valid mnemonic");
        let seed = mnemonic.to_seed("");
        let wallet = manager
            .create_wallet_from_seed_bytes(
                Network::Testnet,
                &seed,
                WalletAccountCreationOptions::Default,
                Some(0),
            )
            .await
            .expect("wallet creation");
        let wallet_id = wallet.wallet_id();
        // Creation downgrades the wallet to external-signable; re-attach the
        // seed so private-key paths (DashPay contact-xpub derivation) work,
        // mirroring the app's post-restore keychain unlock.
        manager
            .attach_wallet_seed(wallet_id, &seed)
            .await
            .expect("attach seed");
        (manager, wallet_id)
    }

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
        let seed = mnemonic.to_seed("");
        let wallet = manager
            .create_wallet_from_seed_bytes(
                Network::Testnet,
                &seed,
                WalletAccountCreationOptions::Default,
                Some(0),
            )
            .await
            .expect("wallet creation");
        let wallet_id = wallet.wallet_id();
        // Creation downgrades the wallet to external-signable; re-attach the
        // seed so private-key paths (DashPay contact-xpub derivation) work,
        // mirroring the app's post-restore keychain unlock.
        manager
            .attach_wallet_seed(wallet_id, &seed)
            .await
            .expect("attach seed");
        (manager, persister, wallet_id)
    }

    /// Like [`make_wallet`] but WITHOUT re-attaching the seed, so the wallet
    /// stays external-signable (`has_seed() == false`) — the watch-only /
    /// seedless state the unattended sync sweep can hit before a Keychain
    /// unlock.
    async fn make_watch_only_wallet() -> (
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
        let seed = mnemonic.to_seed("");
        let wallet = manager
            .create_wallet_from_seed_bytes(
                Network::Testnet,
                &seed,
                WalletAccountCreationOptions::Default,
                Some(0),
            )
            .await
            .expect("wallet creation");
        let wallet_id = wallet.wallet_id();
        // Intentionally NO attach_wallet_seed: creation downgrades to
        // external-signable, so the wallet has no resident key material.
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
    /// (`load: ... dropped_no_account`).
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
                .register_contact_account(&owner, &contact, 0, None)
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
                .register_contact_account(&owner, &contact, 0, None)
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
            iw.register_contact_account(&owner, &contact, 0, None)
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
            iw.register_contact_account(&owner, &contact, 0, None)
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

    /// **C1 (Critical) — ignore must PROPAGATE a persist failure.**
    /// Ignore is local-only (no on-chain artifact), so a swallowed store
    /// error would resurface the ignored sender on the next launch with no
    /// signal. The user-initiated `ignore` path must return the error
    /// instead.
    ///
    /// The hazard: if `ignore_contact_sender` merely logged the store error
    /// and returned `Ok(())`, the ignore would be lost; it must return
    /// `Err(Persistence)`.
    #[tokio::test]
    async fn ignore_propagates_persist_failure() {
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
        let seed = mnemonic.to_seed("");
        let wallet = manager
            .create_wallet_from_seed_bytes(
                Network::Testnet,
                &seed,
                WalletAccountCreationOptions::Default,
                Some(0),
            )
            .await
            .expect("wallet creation");
        let wallet_id = wallet.wallet_id();

        let owner = Identifier::from([0xAA; 32]);
        let contact = Identifier::from([0xBB; 32]);

        // Setup (persister still succeeding): managed owner + an incoming
        // request to ignore.
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

        // Arm the persister to fail, then ignore: must return Err, NOT Ok.
        persister
            .armed
            .store(true, std::sync::atomic::Ordering::SeqCst);
        let iw = wallet.identity();
        let result = iw.ignore_contact_sender(&owner, &contact).await;
        assert!(
            matches!(result, Err(PlatformWalletError::Persistence(_))),
            "ignore must propagate a persist failure (got {result:?}), \
             else the ignore is lost and the sender resurfaces"
        );
    }

    /// A `Sent` payment must advance `Pending → Confirmed` once its
    /// transaction confirms on-chain. `send_payment` records it `Pending`
    /// and nothing else moved it, so before the confirm path was wired the
    /// entry was stuck `Pending` forever (sent payments never showed
    /// confirmed). Pins the flip, idempotency on re-detection, and that
    /// amount/memo are preserved.
    #[tokio::test]
    async fn confirm_flips_sent_payment_pending_to_confirmed() {
        use crate::wallet::identity::types::dashpay::payment::{
            PaymentDirection, PaymentEntry, PaymentStatus,
        };

        let (manager, persister, wallet_id) = make_wallet().await;
        let owner = Identifier::from([0xAA; 32]);
        let contact = Identifier::from([0xBB; 32]);
        let txid = "a".repeat(64);

        let wallet = manager.get_wallet(&wallet_id).await.expect("wallet");
        let iw = wallet.identity();

        let p = WalletPersister::new(wallet_id, Arc::clone(&persister) as _);
        {
            let mut wm = iw.wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&wallet_id).expect("info");
            info.identity_manager
                .add_identity(bare_identity([0xAA; 32]), 0, wallet_id, &p)
                .expect("add owner");
            info.identity_manager
                .managed_identity_mut(&owner)
                .expect("managed")
                .record_dashpay_payment(
                    txid.clone(),
                    PaymentEntry::new_sent(contact, 50_000, Some("dinner".into())),
                    &p,
                )
                .expect("record pending sent");
        }

        // Read the current entry under a short-lived read lock.
        async fn read_entry(
            iw: &crate::wallet::identity::IdentityWallet<crate::broadcaster::SpvBroadcaster>,
            wallet_id: &WalletId,
            owner: &Identifier,
            txid: &str,
        ) -> PaymentEntry {
            let wm = iw.wallet_manager.read().await;
            let info = wm.get_wallet_info(wallet_id).expect("info");
            info.identity_manager
                .managed_identity(owner)
                .unwrap()
                .dashpay_payments
                .get(txid)
                .cloned()
                .expect("entry")
        }

        assert_eq!(
            read_entry(iw, &wallet_id, &owner, &txid).await.status,
            PaymentStatus::Pending,
            "precondition: entry starts Pending"
        );

        // A confirmed detection flips it to Confirmed, preserving fields.
        super::confirm_sent_payment_by_txid(&iw.wallet_manager, &wallet_id, &p, &txid).await;
        let entry = read_entry(iw, &wallet_id, &owner, &txid).await;
        assert_eq!(
            entry.status,
            PaymentStatus::Confirmed,
            "a confirmed tx must flip the Sent entry to Confirmed"
        );
        assert_eq!(entry.direction, PaymentDirection::Sent);
        assert_eq!(entry.amount_duffs, 50_000);
        assert_eq!(entry.memo.as_deref(), Some("dinner"), "memo preserved");

        // Idempotent: a second confirmed re-detection changes nothing.
        super::confirm_sent_payment_by_txid(&iw.wallet_manager, &wallet_id, &p, &txid).await;
        assert_eq!(
            read_entry(iw, &wallet_id, &owner, &txid).await.status,
            PaymentStatus::Confirmed
        );
    }

    /// A sent payment confirmed by a block must flip `Pending → Confirmed`.
    ///
    /// The wallet sees its *own* broadcast in the mempool first
    /// (`TransactionDetected`, context `Mempool`), where the confirm hook
    /// early-returns because the transaction is not yet confirmed. The
    /// transaction reaches a confirmed context only when a block mines it —
    /// delivered as [`key_wallet_manager::WalletEvent::BlockProcessed`] with
    /// the record in `updated` (a previously-known record that just
    /// confirmed). Routing the payment hooks only for `TransactionDetected`
    /// would leave the entry `Pending` forever. This drives the real adapter
    /// dispatch
    /// ([`run_dashpay_payment_hooks`](crate::wallet::identity::network::run_dashpay_payment_hooks))
    /// with a `BlockProcessed` event and pins the flip end-to-end, so a
    /// regression that re-narrows the routing to `TransactionDetected` is
    /// caught here. Also pins idempotency across a repeated block-processing
    /// round and that the `matured` bucket (coinbase maturity) never
    /// confirms a payment.
    #[tokio::test]
    async fn block_processed_confirms_sent_payment() {
        use dashcore::blockdata::transaction::Transaction;
        use dashcore::hashes::Hash;
        use dashcore::{BlockHash, TxIn};
        use key_wallet::account::account_type::StandardAccountType;
        use key_wallet::account::AccountType;
        use key_wallet::managed_account::transaction_record::{
            TransactionDirection, TransactionRecord,
        };
        use key_wallet::transaction_checking::{BlockInfo, TransactionContext, TransactionType};
        use key_wallet::WalletCoreBalance;
        use key_wallet_manager::WalletEvent;

        use crate::wallet::identity::types::dashpay::payment::{PaymentEntry, PaymentStatus};

        let (manager, persister, wallet_id) = make_wallet().await;
        let owner = Identifier::from([0xAA; 32]);
        let contact = Identifier::from([0xBB; 32]);

        let wallet = manager.get_wallet(&wallet_id).await.expect("wallet");
        let iw = wallet.identity();
        let p = WalletPersister::new(wallet_id, Arc::clone(&persister) as _);

        // The sent transaction; `tx.txid()` is the payment-entry key, so the
        // entry and the confirming record agree on the same display-order
        // txid string the confirm path looks up.
        let tx = Transaction {
            version: 2,
            lock_time: 0,
            input: vec![TxIn {
                previous_output: dashcore::OutPoint::new(
                    dashcore::Txid::from_byte_array([0x5f; 32]),
                    0,
                ),
                ..Default::default()
            }],
            output: Vec::new(),
            special_transaction_payload: None,
        };
        let txid = tx.txid();

        {
            let mut wm = iw.wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&wallet_id).expect("info");
            info.identity_manager
                .add_identity(bare_identity([0xAA; 32]), 0, wallet_id, &p)
                .expect("add owner");
            info.identity_manager
                .managed_identity_mut(&owner)
                .expect("managed")
                .record_dashpay_payment(
                    txid.to_string(),
                    PaymentEntry::new_sent(contact, 100_000, Some("lunch".into())),
                    &p,
                )
                .expect("record pending sent");
        }

        // A block confirms the transaction; the wallet already knew it from
        // the mempool, so it rides `BlockProcessed.updated`.
        let confirmed = TransactionRecord::new(
            tx,
            AccountType::Standard {
                index: 0,
                standard_account_type: StandardAccountType::BIP44Account,
            },
            TransactionContext::InBlock(BlockInfo::new(1_499_050, BlockHash::all_zeros(), 0)),
            TransactionType::Standard,
            TransactionDirection::Outgoing,
            Vec::new(),
            Vec::new(),
            -100_000,
        );
        assert!(
            confirmed.is_confirmed(),
            "precondition: an InBlock record reports confirmed"
        );

        let event = WalletEvent::BlockProcessed {
            wallet_id,
            height: 1_499_050,
            chain_lock: None,
            inserted: Vec::new(),
            updated: vec![confirmed],
            matured: Vec::new(),
            balance: WalletCoreBalance::default(),
            account_balances: std::collections::BTreeMap::new(),
            addresses_derived: Vec::new(),
        };

        crate::wallet::identity::network::run_dashpay_payment_hooks(
            &iw.wallet_manager,
            &wallet_id,
            &p,
            &event,
        )
        .await;

        // Read the entry under a short-lived read lock so the re-fire below
        // can take the write lock.
        async fn read_status(
            iw: &crate::wallet::identity::IdentityWallet<crate::broadcaster::SpvBroadcaster>,
            wallet_id: &WalletId,
            owner: &Identifier,
            txid: &str,
        ) -> PaymentEntry {
            let wm = iw.wallet_manager.read().await;
            let info = wm.get_wallet_info(wallet_id).expect("info");
            info.identity_manager
                .managed_identity(owner)
                .expect("managed")
                .dashpay_payments
                .get(txid)
                .cloned()
                .expect("entry present under the sent txid")
        }

        let entry = read_status(iw, &wallet_id, &owner, &txid.to_string()).await;
        assert_eq!(
            entry.status,
            PaymentStatus::Confirmed,
            "a sent payment confirmed via BlockProcessed must flip Pending → Confirmed"
        );
        assert_eq!(entry.memo.as_deref(), Some("lunch"), "memo preserved");

        // Idempotent: a repeated block-processing round for the same txid
        // changes nothing (the confirm path skips entries past `Pending`).
        crate::wallet::identity::network::run_dashpay_payment_hooks(
            &iw.wallet_manager,
            &wallet_id,
            &p,
            &event,
        )
        .await;
        assert_eq!(
            read_status(iw, &wallet_id, &owner, &txid.to_string())
                .await
                .status,
            PaymentStatus::Confirmed,
            "re-processing the same block must not change a Confirmed entry"
        );

        // A confirmed record arriving only in the `matured` bucket (coinbase
        // maturity) must NOT confirm a payment — `matured` is never a DashPay
        // payment, so it is excluded from the payment hooks.
        let matured_tx = Transaction {
            version: 2,
            lock_time: 0,
            input: vec![TxIn {
                previous_output: dashcore::OutPoint::new(
                    dashcore::Txid::from_byte_array([0xC0; 32]),
                    0,
                ),
                ..Default::default()
            }],
            output: Vec::new(),
            special_transaction_payload: None,
        };
        let matured_txid = matured_tx.txid();
        {
            let mut wm = iw.wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&wallet_id).expect("info");
            info.identity_manager
                .managed_identity_mut(&owner)
                .expect("managed")
                .record_dashpay_payment(
                    matured_txid.to_string(),
                    PaymentEntry::new_sent(contact, 7_000, None),
                    &p,
                )
                .expect("record pending sent");
        }
        let matured_record = TransactionRecord::new(
            matured_tx,
            AccountType::Standard {
                index: 0,
                standard_account_type: StandardAccountType::BIP44Account,
            },
            TransactionContext::InChainLockedBlock(BlockInfo::new(
                1_499_060,
                BlockHash::all_zeros(),
                0,
            )),
            TransactionType::Standard,
            TransactionDirection::Outgoing,
            Vec::new(),
            Vec::new(),
            -7_000,
        );
        let matured_event = WalletEvent::BlockProcessed {
            wallet_id,
            height: 1_499_060,
            chain_lock: None,
            inserted: Vec::new(),
            updated: Vec::new(),
            matured: vec![matured_record],
            balance: WalletCoreBalance::default(),
            account_balances: std::collections::BTreeMap::new(),
            addresses_derived: Vec::new(),
        };
        crate::wallet::identity::network::run_dashpay_payment_hooks(
            &iw.wallet_manager,
            &wallet_id,
            &p,
            &matured_event,
        )
        .await;
        assert_eq!(
            read_status(iw, &wallet_id, &owner, &matured_txid.to_string())
                .await
                .status,
            PaymentStatus::Pending,
            "a confirmed record in the `matured` bucket must not confirm a payment"
        );
    }

    /// An InstantSend lock applied to a previously-seen sent payment
    /// confirms it without waiting for a block. The lock arrives as
    /// `WalletEvent::TransactionInstantLocked` (no record, just a txid); an
    /// IS lock is final for DashPay display, so the entry flips
    /// `Pending → Confirmed`. Drives the real adapter dispatch.
    #[tokio::test]
    async fn instant_send_lock_confirms_sent_payment() {
        use dashcore::ephemerealdata::instant_lock::InstantLock;
        use key_wallet::WalletCoreBalance;
        use key_wallet_manager::WalletEvent;

        use crate::wallet::identity::types::dashpay::payment::{PaymentEntry, PaymentStatus};

        let (manager, persister, wallet_id) = make_wallet().await;
        let owner = Identifier::from([0xAA; 32]);
        let contact = Identifier::from([0xBB; 32]);
        let txid = dashcore::Txid::from([0x5f; 32]);

        let wallet = manager.get_wallet(&wallet_id).await.expect("wallet");
        let iw = wallet.identity();
        let p = WalletPersister::new(wallet_id, Arc::clone(&persister) as _);
        {
            let mut wm = iw.wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&wallet_id).expect("info");
            info.identity_manager
                .add_identity(bare_identity([0xAA; 32]), 0, wallet_id, &p)
                .expect("add owner");
            info.identity_manager
                .managed_identity_mut(&owner)
                .expect("managed")
                .record_dashpay_payment(
                    txid.to_string(),
                    PaymentEntry::new_sent(contact, 50_000, None),
                    &p,
                )
                .expect("record pending sent");
        }

        let event = WalletEvent::TransactionInstantLocked {
            wallet_id,
            txid,
            instant_lock: InstantLock::default(),
            balance: WalletCoreBalance::default(),
            account_balances: std::collections::BTreeMap::new(),
        };
        crate::wallet::identity::network::run_dashpay_payment_hooks(
            &iw.wallet_manager,
            &wallet_id,
            &p,
            &event,
        )
        .await;

        let wm = iw.wallet_manager.read().await;
        let info = wm.get_wallet_info(&wallet_id).expect("info");
        let entry = info
            .identity_manager
            .managed_identity(&owner)
            .expect("managed")
            .dashpay_payments
            .get(&txid.to_string())
            .cloned()
            .expect("entry present under the sent txid");
        assert_eq!(
            entry.status,
            PaymentStatus::Confirmed,
            "an InstantSend lock must confirm a sent payment"
        );
    }

    /// A transaction first seen *with* an InstantSend lock arrives as a
    /// `TransactionDetected` whose record context is `InstantSend`. The
    /// confirm gate accepts IS context (not just mined), so it flips the
    /// entry `Pending → Confirmed` — a plain mempool sighting would not.
    #[tokio::test]
    async fn instant_send_context_record_confirms_sent_payment() {
        use dashcore::blockdata::transaction::Transaction;
        use dashcore::ephemerealdata::instant_lock::InstantLock;
        use dashcore::TxIn;
        use key_wallet::account::account_type::StandardAccountType;
        use key_wallet::account::AccountType;
        use key_wallet::managed_account::transaction_record::{
            TransactionDirection, TransactionRecord,
        };
        use key_wallet::transaction_checking::{TransactionContext, TransactionType};
        use key_wallet::WalletCoreBalance;
        use key_wallet_manager::WalletEvent;

        use crate::wallet::identity::types::dashpay::payment::{PaymentEntry, PaymentStatus};

        let (manager, persister, wallet_id) = make_wallet().await;
        let owner = Identifier::from([0xAA; 32]);
        let contact = Identifier::from([0xBB; 32]);

        let wallet = manager.get_wallet(&wallet_id).await.expect("wallet");
        let iw = wallet.identity();
        let p = WalletPersister::new(wallet_id, Arc::clone(&persister) as _);

        let tx = Transaction {
            version: 2,
            lock_time: 0,
            input: vec![TxIn {
                previous_output: dashcore::OutPoint::new(dashcore::Txid::from([0x5e; 32]), 0),
                ..Default::default()
            }],
            output: Vec::new(),
            special_transaction_payload: None,
        };
        let txid = tx.txid();
        {
            let mut wm = iw.wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&wallet_id).expect("info");
            info.identity_manager
                .add_identity(bare_identity([0xAA; 32]), 0, wallet_id, &p)
                .expect("add owner");
            info.identity_manager
                .managed_identity_mut(&owner)
                .expect("managed")
                .record_dashpay_payment(
                    txid.to_string(),
                    PaymentEntry::new_sent(contact, 50_000, None),
                    &p,
                )
                .expect("record pending sent");
        }

        let record = TransactionRecord::new(
            tx,
            AccountType::Standard {
                index: 0,
                standard_account_type: StandardAccountType::BIP44Account,
            },
            TransactionContext::InstantSend(InstantLock::default()),
            TransactionType::Standard,
            TransactionDirection::Outgoing,
            Vec::new(),
            Vec::new(),
            -50_000,
        );
        assert!(
            !record.is_confirmed(),
            "precondition: an InstantSend record is not block-confirmed"
        );
        let event = WalletEvent::TransactionDetected {
            wallet_id,
            record: Box::new(record),
            balance: WalletCoreBalance::default(),
            account_balances: std::collections::BTreeMap::new(),
            addresses_derived: Vec::new(),
        };
        crate::wallet::identity::network::run_dashpay_payment_hooks(
            &iw.wallet_manager,
            &wallet_id,
            &p,
            &event,
        )
        .await;

        let wm = iw.wallet_manager.read().await;
        let info = wm.get_wallet_info(&wallet_id).expect("info");
        assert_eq!(
            info.identity_manager
                .managed_identity(&owner)
                .expect("managed")
                .dashpay_payments
                .get(&txid.to_string())
                .expect("entry")
                .status,
            PaymentStatus::Confirmed,
            "an InstantSend-context record must confirm a sent payment"
        );
    }

    /// `reconcile_sent_payments` recovers a `Pending` `Sent` payment whose
    /// live confirm event was missed: it flips the entry to `Confirmed` when
    /// the persisted core tx record reports the transaction final (mined or
    /// IS-locked), leaves a not-yet-final entry `Pending`, and is idempotent.
    #[tokio::test]
    async fn reconcile_sent_payments_confirms_from_persisted_record() {
        use dashcore::blockdata::transaction::Transaction;
        use dashcore::hashes::Hash;
        use dashcore::BlockHash;
        use key_wallet::account::account_type::StandardAccountType;
        use key_wallet::account::AccountType;
        use key_wallet::managed_account::transaction_record::{
            TransactionDirection, TransactionRecord,
        };
        use key_wallet::transaction_checking::{BlockInfo, TransactionContext, TransactionType};

        use crate::wallet::identity::types::dashpay::payment::{PaymentEntry, PaymentStatus};

        // A persisted core tx record carrying only `txid` + `context` (the
        // contract `get_core_tx_record` guarantees).
        fn tx_record(txid: dashcore::Txid, context: TransactionContext) -> TransactionRecord {
            let tx = Transaction {
                version: 2,
                lock_time: 0,
                input: Vec::new(),
                output: Vec::new(),
                special_transaction_payload: None,
            };
            let mut record = TransactionRecord::new(
                tx,
                AccountType::Standard {
                    index: 0,
                    standard_account_type: StandardAccountType::BIP44Account,
                },
                context,
                TransactionType::Standard,
                TransactionDirection::Outgoing,
                Vec::new(),
                Vec::new(),
                0,
            );
            record.txid = txid;
            record
        }

        let persister = Arc::new(RecordStorePersister::default());
        let (manager, wallet_id) = make_wallet_with(Arc::clone(&persister)).await;
        let owner = Identifier::from([0xAA; 32]);
        let contact = Identifier::from([0xBB; 32]);

        let wallet = manager.get_wallet(&wallet_id).await.expect("wallet");
        let iw = wallet.identity();
        let p = WalletPersister::new(wallet_id, Arc::clone(&persister) as _);

        let mined_txid = dashcore::Txid::from([0x21; 32]);
        let mempool_txid = dashcore::Txid::from([0x22; 32]);
        {
            let mut wm = iw.wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&wallet_id).expect("info");
            info.identity_manager
                .add_identity(bare_identity([0xAA; 32]), 0, wallet_id, &p)
                .expect("add owner");
            let managed = info
                .identity_manager
                .managed_identity_mut(&owner)
                .expect("managed");
            managed
                .record_dashpay_payment(
                    mined_txid.to_string(),
                    PaymentEntry::new_sent(contact, 1_000, None),
                    &p,
                )
                .expect("record mined-pending");
            managed
                .record_dashpay_payment(
                    mempool_txid.to_string(),
                    PaymentEntry::new_sent(contact, 2_000, None),
                    &p,
                )
                .expect("record mempool-pending");
        }
        {
            let mut recs = persister.records.lock().unwrap();
            recs.insert(
                mined_txid,
                tx_record(
                    mined_txid,
                    TransactionContext::InChainLockedBlock(BlockInfo::new(
                        100,
                        BlockHash::all_zeros(),
                        0,
                    )),
                ),
            );
            recs.insert(
                mempool_txid,
                tx_record(mempool_txid, TransactionContext::Mempool),
            );
        }

        let n = iw.reconcile_sent_payments().await.expect("reconcile");
        assert_eq!(n, 1, "only the mined payment is confirmed this pass");

        {
            let wm = iw.wallet_manager.read().await;
            let info = wm.get_wallet_info(&wallet_id).expect("info");
            let managed = info
                .identity_manager
                .managed_identity(&owner)
                .expect("managed");
            assert_eq!(
                managed
                    .dashpay_payments
                    .get(&mined_txid.to_string())
                    .expect("mined entry")
                    .status,
                PaymentStatus::Confirmed,
                "a mined tx record must confirm the sent payment"
            );
            assert_eq!(
                managed
                    .dashpay_payments
                    .get(&mempool_txid.to_string())
                    .expect("mempool entry")
                    .status,
                PaymentStatus::Pending,
                "a not-yet-final tx must leave the sent payment Pending"
            );
        }

        // Idempotent: a second pass confirms nothing new (the mined entry
        // is already Confirmed, the mempool one is still not final).
        assert_eq!(
            iw.reconcile_sent_payments().await.expect("second pass"),
            0,
            "reconcile must be idempotent"
        );
    }

    /// The seedless drain path: `register_external_contact_account` with a
    /// **precomputed** ECDH shared secret (the Keychain signer computed it; the
    /// scalar never entered this crate) decrypts the contact's xpub and builds
    /// the `DashpayExternalAccount` — same result as the resident path. Pins the
    /// reuse that lets the deferred-crypto drain complete an external-account
    /// build once a signer is available. The contact identity is `bare` here,
    /// proving the `Some` path skips the peer-key derivation entirely.
    #[tokio::test]
    async fn register_external_with_precomputed_shared_key_builds_account() {
        let (manager, persister, wallet_id) = make_wallet().await;
        let wallet_arc = manager.get_wallet(&wallet_id).await.expect("wallet");
        let iw = wallet_arc.identity();

        let owner_id = Identifier::from([0x11; 32]);
        let contact_id = Identifier::from([0x22; 32]);
        {
            let mut wm = iw.wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&wallet_id).expect("info");
            info.identity_manager
                .add_identity(
                    bare_identity([0x11; 32]),
                    0,
                    wallet_id,
                    &WalletPersister::new(wallet_id, Arc::clone(&persister) as _),
                )
                .expect("add owner");
        }

        // A real 69-byte compact xpub encrypted under a known shared key — the
        // wire shape a contact would have sent us.
        let shared_key = [0x55u8; 32];
        let iv = [0x11u8; 16];
        let compact = {
            let wm = iw.wallet_manager.read().await;
            let w = wm.get_wallet(&wallet_id).expect("wallet");
            crate::wallet::identity::crypto::dip14::derive_contact_xpub(
                w,
                Network::Testnet,
                0,
                &owner_id,
                &contact_id,
            )
            .expect("derive a valid compact xpub")
            .compact
            .to_bytes()
        };
        let encrypted =
            platform_encryption::encrypt_extended_public_key(&shared_key, &iv, &compact);

        // Bare contact identity: the `Some` path must NOT touch the contact's
        // encryption key (the signer derives the secret out-of-crate).
        let contact = bare_identity([0x22; 32]);
        iw.register_external_contact_account(&owner_id, &contact, &encrypted, shared_key)
            .await
            .expect("register external with a signer-derived shared key");

        let wm = iw.wallet_manager.read().await;
        let info = wm.get_wallet_info(&wallet_id).expect("info");
        use key_wallet::account::account_collection::DashpayAccountKey;
        let key = DashpayAccountKey {
            index: 0,
            user_identity_id: owner_id.to_buffer(),
            friend_identity_id: contact_id.to_buffer(),
        };
        assert!(
            info.core_wallet
                .accounts
                .dashpay_external_accounts
                .contains_key(&key),
            "the precomputed-shared-key path must build the external account (the drain's path)"
        );
    }

    /// The seedless drain's RegisterReceiving path: `register_contact_account`
    /// with a **precomputed** receiving xpub (the Keychain signer derived our
    /// friendship key) builds the `DashpayReceivingFunds` account without
    /// touching the wallet seed. Pins the reuse the drain needs when the
    /// receiving account was never persisted (restore / first-time edge).
    #[tokio::test]
    async fn register_contact_account_with_precomputed_xpub_builds_account() {
        let (manager, _persister, wallet_id) = make_wallet().await;
        let wallet_arc = manager.get_wallet(&wallet_id).await.expect("wallet");
        let iw = wallet_arc.identity();

        let owner = Identifier::from([0x11; 32]);
        let contact = Identifier::from([0x22; 32]);

        // A valid ExtendedPubKey to supply as the signer would.
        let supplied_xpub = {
            let wm = iw.wallet_manager.read().await;
            let w = wm.get_wallet(&wallet_id).expect("wallet");
            crate::wallet::identity::crypto::dip14::derive_contact_xpub(
                w,
                Network::Testnet,
                0,
                &owner,
                &contact,
            )
            .expect("derive a valid receiving xpub")
            .xpub
        };

        iw.register_contact_account(&owner, &contact, 0, Some(supplied_xpub))
            .await
            .expect("register receiving account with a precomputed xpub");

        let wm = iw.wallet_manager.read().await;
        let info = wm.get_wallet_info(&wallet_id).expect("info");
        use key_wallet::account::account_collection::DashpayAccountKey;
        let key = DashpayAccountKey {
            index: 0,
            user_identity_id: owner.to_buffer(),
            friend_identity_id: contact.to_buffer(),
        };
        assert!(
            info.core_wallet
                .accounts
                .dashpay_receival_accounts
                .contains_key(&key),
            "the precomputed-xpub path must build the receiving account (the drain's RegisterReceiving)"
        );
    }

    /// End-to-end drain of a `RegisterReceiving` entry on a SEEDLESS wallet: the
    /// `SeedCryptoProvider` (the faithful test stand-in for the Keychain signer)
    /// supplies the receiving xpub, the drain builds the receiving account with
    /// that EXACT signer-derived xpub, and the entry is cleared from the queue.
    /// Pins that a wallet with no resident seed becomes payable purely through
    /// the signer-backed drain.
    #[tokio::test]
    async fn drain_completes_register_receiving_and_clears_queue() {
        use crate::changeset::{PendingContactCrypto, PendingContactCryptoOp};
        use crate::wallet::identity::network::contact_requests::SeedCryptoProvider;

        let (manager, _persister, wallet_id) = make_watch_only_wallet().await;
        let wallet_arc = manager.get_wallet(&wallet_id).await.expect("wallet");
        let iw = wallet_arc.identity();

        let owner = Identifier::from([0x11; 32]);
        let contact = Identifier::from([0x22; 32]);

        // The signer's seed (the faithful test stand-in derives from it).
        let seed = {
            let mnemonic =
                Mnemonic::from_phrase(TEST_MNEMONIC, Language::English).expect("valid mnemonic");
            mnemonic.to_seed("")
        };

        // Enqueue a RegisterReceiving op (as the seedless sweep would).
        {
            let mut wm = iw.wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&wallet_id).expect("info");
            info.pending_contact_crypto.push(PendingContactCrypto {
                owner_identity_id: owner,
                contact_id: contact,
                op: PendingContactCryptoOp::RegisterReceiving,
                enqueued_at_ms: 0,
            });
        }

        let provider = SeedCryptoProvider::from_seed(seed, Network::Testnet);
        let drained = iw.drain_pending_contact_crypto(&provider).await;
        assert_eq!(drained, 1, "the RegisterReceiving entry must be drained");

        let wm = iw.wallet_manager.read().await;
        let info = wm.get_wallet_info(&wallet_id).expect("info");
        assert!(
            info.pending_contact_crypto.is_empty(),
            "the queue must be cleared after a successful drain"
        );
        use key_wallet::account::account_collection::DashpayAccountKey;
        let key = DashpayAccountKey {
            index: 0,
            user_identity_id: owner.to_buffer(),
            friend_identity_id: contact.to_buffer(),
        };
        assert!(
            info.core_wallet
                .accounts
                .dashpay_receival_accounts
                .contains_key(&key),
            "the seedless drain must build the receiving account via the signer provider"
        );
    }

    /// A `RegisterExternal` entry the drain cannot complete (here: the owner
    /// isn't wallet-owned, so no HD index → it bails before any network fetch)
    /// must be **left queued**, never dropped or crashed — so a later drain can
    /// retry. Pins the deferral safety of the external op without needing a
    /// configured mock fetch.
    #[tokio::test]
    async fn drain_leaves_register_external_it_cannot_complete() {
        use crate::changeset::{PendingContactCrypto, PendingContactCryptoOp};
        use crate::wallet::identity::network::contact_requests::ContactCryptoProvider;

        let (manager, _persister, wallet_id) = make_wallet().await;
        let wallet_arc = manager.get_wallet(&wallet_id).await.expect("wallet");
        let iw = wallet_arc.identity();
        let owner = Identifier::from([0x11; 32]);
        let contact = Identifier::from([0x22; 32]);

        // Owner is NOT added as a managed identity → identity_index lookup
        // fails → the drain leaves the entry before reaching the fetch.
        {
            let mut wm = iw.wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&wallet_id).expect("info");
            info.pending_contact_crypto.push(PendingContactCrypto {
                owner_identity_id: owner,
                contact_id: contact,
                op: PendingContactCryptoOp::RegisterExternal {
                    encrypted_public_key: vec![7u8; 96],
                    our_decryption_key_index: 0,
                    contact_encryption_key_index: 0,
                },
                enqueued_at_ms: 0,
            });
        }

        struct UnusedProvider;
        #[async_trait::async_trait]
        impl ContactCryptoProvider for UnusedProvider {
            async fn receiving_xpub(
                &self,
                _path: &key_wallet::bip32::DerivationPath,
            ) -> Result<key_wallet::bip32::ExtendedPubKey, crate::error::PlatformWalletError>
            {
                Err(crate::error::PlatformWalletError::InvalidIdentityData(
                    "unused in this test".to_string(),
                ))
            }
            async fn ecdh_shared_secret(
                &self,
                _path: &key_wallet::bip32::DerivationPath,
                _peer: &dashcore::secp256k1::PublicKey,
            ) -> Result<[u8; 32], crate::error::PlatformWalletError> {
                Ok([0u8; 32])
            }
            async fn account_reference(
                &self,
                _path: &key_wallet::bip32::DerivationPath,
                _compact_xpub: &[u8],
                _account_index: u32,
                _version: u32,
            ) -> Result<u32, crate::error::PlatformWalletError> {
                unimplemented!("accountReference is a send-path method, not exercised by the drain")
            }
            async fn unmask_account_reference(
                &self,
                _path: &key_wallet::bip32::DerivationPath,
                _compact_xpub: &[u8],
                _account_reference: u32,
            ) -> Result<(u32, u32), crate::error::PlatformWalletError> {
                unimplemented!("accountReference is a send-path method, not exercised by the drain")
            }
        }

        let drained = iw.drain_pending_contact_crypto(&UnusedProvider).await;
        assert_eq!(
            drained, 0,
            "an un-completable RegisterExternal entry must not be counted as drained"
        );
        let wm = iw.wallet_manager.read().await;
        let info = wm.get_wallet_info(&wallet_id).expect("info");
        assert_eq!(
            info.pending_contact_crypto.len(),
            1,
            "the deferred entry must remain in the queue for a later drain"
        );
    }
}
