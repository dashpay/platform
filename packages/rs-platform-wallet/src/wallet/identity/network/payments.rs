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
        // core `send_to_addresses` cannot select the same UTXO (CMT-001 / #3585).
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

            let reservation = self
                .reservations
                .reserve(selected.into_iter().collect(), Some(change_addr));

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
