//! DashPay payment recording and send-to-contact flows.

use dpp::prelude::Identifier;

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
        use key_wallet::account::account_collection::DashpayAccountKey;
        use key_wallet::wallet::managed_wallet_info::coin_selection::SelectionStrategy;
        use key_wallet::wallet::managed_wallet_info::transaction_builder::TransactionBuilder;
        use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;

        let account_index: u32 = 0;

        // --- 1. Derive the next unused payment address from the external account
        //        and collect the UTXOs / change address from BIP-44 account 0.  ---
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
                .next_address(Some(&contact_xpub))
                .map_err(|e| PlatformWalletError::TransactionBuild(e.to_string()))?;

            // --- 2. Build the transaction from BIP-44 account 0 UTXOs. ---
            let current_height = info.core_wallet.synced_height();

            let bip44_xpub = wallet
                .accounts
                .standard_bip44_accounts
                .get(&0)
                .map(|a| a.account_xpub)
                .ok_or_else(|| {
                    PlatformWalletError::TransactionBuild(
                        "BIP-44 account 0 not found in wallet".to_string(),
                    )
                })?;

            let spendable: Vec<_> = info
                .core_wallet
                .accounts
                .standard_bip44_accounts
                .get(&0)
                .ok_or_else(|| {
                    PlatformWalletError::TransactionBuild(
                        "BIP-44 managed account 0 not found".to_string(),
                    )
                })?
                .spendable_utxos(current_height)
                .into_iter()
                .cloned()
                .collect();

            // Derive change address from BIP-44 account 0.
            let change_address = info
                .core_wallet
                .accounts
                .standard_bip44_accounts
                .get_mut(&0)
                .ok_or_else(|| {
                    PlatformWalletError::TransactionBuild(
                        "BIP-44 managed account 0 not found for change".to_string(),
                    )
                })?
                .next_change_address(Some(&bip44_xpub))
                .map_err(|e| PlatformWalletError::TransactionBuild(e.to_string()))?;

            let mut builder = TransactionBuilder::new();
            builder = builder
                .add_output(&payment_address, amount_duffs)
                .map_err(|e| PlatformWalletError::TransactionBuild(e.to_string()))?;
            builder = builder.set_change_address(change_address);
            builder = builder
                .select_inputs(
                    &spendable,
                    SelectionStrategy::LargestFirst,
                    current_height,
                    |utxo| {
                        for account in info.core_wallet.accounts.all_accounts() {
                            if let Some(path) = account.address_derivation_path(&utxo.address) {
                                if let Ok(key) = wallet.derive_private_key(&path) {
                                    return Some(key);
                                }
                            }
                        }
                        None
                    },
                )
                .map_err(|e| PlatformWalletError::TransactionBuild(e.to_string()))?;

            let tx = builder
                .build()
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
                    managed.record_dashpay_payment(
                        txid.to_string(),
                        entry.clone(),
                        &self.persister,
                    );
                }
            }
        }

        Ok((txid, entry))
    }
}
