//! Two-step DashPay contact payment: sign and reserve now, broadcast or
//! abandon after the user has seen the exact fee.
//!
//! [`DashPayView::send_payment`] builds, signs and broadcasts in one call and
//! reports the fee only afterwards, so a host has nothing to show on a
//! confirmation screen but the amount. The split here mirrors the L1 path a
//! plain send already takes ([`CoreWallet::finalize_transaction`] →
//! [`CoreWallet::broadcast_finalized_transaction`] /
//! [`CoreWallet::abandon_transaction`]): [`DashPayView::prepare_payment`]
//! claims the contact's next DIP-15 address, funds, reserves and signs, and
//! returns a [`PreparedDashPayPayment`] that carries the exact fee;
//! [`DashPayView::send_prepared_payment`] broadcasts it and records the
//! payment; [`DashPayView::abandon_prepared_payment`] releases both the
//! inputs and the address.
//!
//! Between the two steps the inputs are held by the finalized transaction's
//! owner-guarded reservation, under the same age bound the L1 path enforces —
//! a payment held past it is refused at send time with
//! [`PlatformWalletError::StaleReservation`] and must be prepared again.
//! The contact address is marked used in memory while the payment is
//! prepared; the flip is persisted just before the broadcast, as
//! [`DashPayView::send_payment`] does. Every way out short of a broadcast —
//! a failed build, an abandon, a rejection — returns the address and persists
//! the revert, because another payment to the contact may have stored a pool
//! snapshot showing it used meanwhile.
//!
//! Every address change is bound to the wallet generation the payment was
//! funded from: a payment that outlives its wallet (removed, or re-created
//! under the same id) never touches the new generation's pool.

use dashcore::Address;
use dpp::prelude::Identifier;
use key_wallet::account::account_collection::DashpayAccountKey;
use key_wallet::account::AccountType;
use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
use key_wallet::managed_account::ManagedCoreFundsAccount;
use key_wallet::wallet::managed_wallet_info::coin_selection::SelectionStrategy;
use key_wallet::wallet::managed_wallet_info::transaction_builder::TransactionBuilder;
use std::sync::Arc;

use super::contact_requests::ContactCryptoProvider;
use super::payments::return_contact_payment_address_to_pool;
use super::DashPayView;
use crate::broadcaster::TransactionBroadcaster;
use crate::changeset::{account_address_pool_entries, PlatformWalletChangeSet};
use crate::error::PlatformWalletError;
use crate::wallet::core::{CoreWallet, SignedCoreTransaction, WalletGeneration};
use crate::wallet::identity::types::dashpay::payment::PaymentEntry;
use crate::wallet::platform_wallet::PlatformWalletInfo;
use crate::SEND_FUNDING_SOURCES;

/// The DashPay contact account index every contact payment uses, the same one
/// [`DashPayView::send_payment`] pins.
const CONTACT_ACCOUNT_INDEX: u32 = 0;

/// A signed DashPay contact payment that has not been broadcast yet.
///
/// Its funding inputs stay reserved and its contact address stays claimed
/// until it is handed to [`DashPayView::send_prepared_payment`] or
/// [`DashPayView::abandon_prepared_payment`]. Deliberately not `Clone`: one
/// prepare yields one payment that can be sent or abandoned exactly once.
/// Dropping it without either leaves the reservation to key-wallet's TTL and
/// the address claimed until the process ends; hosts abandon instead.
pub struct PreparedDashPayPayment<B: TransactionBroadcaster + ?Sized> {
    core: CoreWallet<B>,
    signed: SignedCoreTransaction,
    payment_address: Address,
    from_identity_id: Identifier,
    to_contact_id: Identifier,
    amount_duffs: u64,
    memo: Option<String>,
}

impl<B: TransactionBroadcaster + ?Sized> PreparedDashPayPayment<B> {
    /// The amount the contact receives, in duffs.
    pub fn amount_duffs(&self) -> u64 {
        self.amount_duffs
    }

    /// The exact network fee of the signed transaction, in duffs —
    /// Σ(selected inputs) − Σ(outputs), a dropped sub-dust change remainder
    /// included. The broadcast cannot change it.
    pub fn fee_duffs(&self) -> u64 {
        self.signed.fee()
    }

    /// What leaves the wallet: the amount plus the fee.
    pub fn total_duffs(&self) -> u64 {
        self.amount_duffs.saturating_add(self.signed.fee())
    }

    /// The signed transaction's id.
    pub fn txid(&self) -> dashcore::Txid {
        self.signed.transaction().txid()
    }

    /// The signed transaction itself.
    pub fn transaction(&self) -> &dashcore::Transaction {
        self.signed.transaction()
    }

    pub fn from_identity_id(&self) -> &Identifier {
        &self.from_identity_id
    }

    pub fn to_contact_id(&self) -> &Identifier {
        &self.to_contact_id
    }

    /// The core wallet the payment was funded from — the one whose
    /// generation gate a host takes before sending it.
    pub fn core_wallet(&self) -> &CoreWallet<B> {
        &self.core
    }
}

impl<B: TransactionBroadcaster + ?Sized> DashPayView<'_, B> {
    /// Sign a Core payment to a DashPay contact without broadcasting it.
    ///
    /// Runs the first half of [`Self::send_payment`]: drains any deferred
    /// contact-crypto build behind the seed-binding gate, claims the contact's
    /// next unused DIP-15 address, and funds, reserves and signs the
    /// transaction from the same pooled sources a plain send uses
    /// ([`SEND_FUNDING_SOURCES`]: BIP44 first, so change keeps returning to
    /// BIP44). `core` must be this wallet's core handle; it funds the payment
    /// and later broadcasts it.
    ///
    /// A failed build returns the address to the pool, exactly like
    /// [`Self::send_payment`]: nothing was signed or exposed, and leaving it
    /// claimed would let every failed attempt push the next payment further
    /// past the gap limit a restore walks.
    #[allow(clippy::too_many_arguments)]
    pub async fn prepare_payment<Bc, S, C>(
        &self,
        core: &CoreWallet<Bc>,
        from_identity_id: &Identifier,
        to_contact_id: &Identifier,
        amount_duffs: u64,
        memo: Option<String>,
        signer: &S,
        provider: &C,
    ) -> Result<PreparedDashPayPayment<Bc>, PlatformWalletError>
    where
        Bc: TransactionBroadcaster + ?Sized,
        S: key_wallet::signer::Signer + Sync,
        C: ContactCryptoProvider + Sync,
    {
        self.ensure_same_wallet(core)?;

        // Before the address lookup: the signerless sweep only enqueues the
        // external-account build, so the first payment after a contact is
        // established needs this drain to find the account at all. Runs
        // before any lock is taken — the drain takes the manager lock itself.
        self.drain_pending_contact_crypto_verified(provider, None)
            .await?;

        let generation = core.generation();
        let payment_address = self
            .claim_contact_payment_address(generation, from_identity_id, to_contact_id)
            .await?;

        let builder = TransactionBuilder::new()
            .set_selection_strategy(SelectionStrategy::LargestFirst)
            .add_output(&payment_address, amount_duffs);
        let signed = match core
            .finalize_transaction(
                builder,
                &SEND_FUNDING_SOURCES,
                CONTACT_ACCOUNT_INDEX,
                signer,
            )
            .await
        {
            Ok(signed) => signed,
            Err(e) => {
                // Never exposed. Persisted, because the signer ran with no
                // lock held and another payment may have stored the pool
                // with this address used meanwhile.
                self.release_contact_payment_address(
                    generation,
                    from_identity_id,
                    to_contact_id,
                    &payment_address,
                )
                .await;
                return Err(e);
            }
        };

        Ok(PreparedDashPayPayment {
            core: core.clone(),
            signed,
            payment_address,
            from_identity_id: *from_identity_id,
            to_contact_id: *to_contact_id,
            amount_duffs,
            memo,
        })
    }

    /// Broadcast a prepared contact payment and record it on the sender.
    ///
    /// The second half of [`Self::send_payment`], with the same guarantees:
    /// the address's used flip is persisted before anything reaches the
    /// network (a relaunch must never re-hand an address that is on-chain);
    /// a definitive rejection — or a payment held past the reservation age
    /// bound, refused before the broadcaster is touched — releases the inputs
    /// and returns the address to the pool, persisted; an ambiguous outcome
    /// keeps both, since the transaction may have propagated.
    ///
    /// Returns the txid, the recorded [`PaymentEntry`] and the fee — the same
    /// fee [`PreparedDashPayPayment::fee_duffs`] showed.
    pub async fn send_prepared_payment<Bc>(
        &self,
        prepared: PreparedDashPayPayment<Bc>,
    ) -> Result<(dashcore::Txid, PaymentEntry, u64), PlatformWalletError>
    where
        Bc: TransactionBroadcaster + ?Sized,
    {
        if let Err(e) = self.ensure_same_wallet(&prepared.core) {
            // Not this wallet's payment: its address lives in another wallet's
            // pool this view cannot reach, but the inputs can still be freed.
            prepared.core.abandon_transaction(&prepared.signed).await;
            return Err(e);
        }
        let PreparedDashPayPayment {
            core,
            signed,
            payment_address,
            from_identity_id,
            to_contact_id,
            amount_duffs,
            memo,
        } = prepared;

        // Snapshot the pool with the address used, under the write guard;
        // store it after the guard drops — a host persistence callback must
        // never run under the manager lock. The mark is re-applied rather
        // than assumed: it is idempotent, and it is what the snapshot must
        // show whatever touched the pool while the payment waited.
        let generation = core.generation();
        let used_flip = match self
            .commit_contact_payment_address(
                generation,
                &from_identity_id,
                &to_contact_id,
                &payment_address,
            )
            .await
        {
            Ok(changeset) => changeset,
            Err(e) => {
                core.abandon_transaction(&signed).await;
                self.release_contact_payment_address(
                    generation,
                    &from_identity_id,
                    &to_contact_id,
                    &payment_address,
                )
                .await;
                return Err(e);
            }
        };
        if let Err(e) = self.persister.store(used_flip) {
            // Pre-send: nothing is on the wire, so both the inputs and the
            // address are free to hand out again.
            core.abandon_transaction(&signed).await;
            self.release_contact_payment_address(
                generation,
                &from_identity_id,
                &to_contact_id,
                &payment_address,
            )
            .await;
            return Err(PlatformWalletError::Persistence(format!(
                "failed to persist payment-address used flip: {e}"
            )));
        }

        // The age guard, the in-broadcast pin and the owner-guarded release
        // on a definitive rejection all live in the finalized-broadcast path.
        let txid = match core.broadcast_finalized_transaction(&signed).await {
            Ok(txid) => txid,
            Err(e) => {
                // Rejected, or refused as stale before the send: provably not
                // on the wire, and the flip above was persisted, so the revert
                // must be persisted too or a relaunch would undo it. An
                // ambiguous outcome keeps the address consumed.
                if matches!(
                    e,
                    PlatformWalletError::TransactionBroadcast(_)
                        | PlatformWalletError::StaleReservation
                ) {
                    self.release_contact_payment_address(
                        generation,
                        &from_identity_id,
                        &to_contact_id,
                        &payment_address,
                    )
                    .await;
                }
                return Err(e);
            }
        };

        tracing::info!(
            from_identity = %from_identity_id,
            to_contact = %to_contact_id,
            amount_duffs,
            %txid,
            payment_address = %payment_address,
            "DashPay payment broadcast"
        );

        // The payment is on the wire from here on, so nothing below may fail
        // the call: an error would read as "not sent" and invite the user to
        // pay again. A record that cannot be written is logged instead; the
        // sent-payment reconstruction from the wallet's transaction history
        // (`reconcile_sent_payments_from_tx_history`) rebuilds it, without
        // the memo.
        let entry = PaymentEntry::new_sent(to_contact_id, amount_duffs, memo);
        {
            let mut wm = self.wallet_manager.write().await;
            let recorded = match wm.get_wallet_info_mut(&self.wallet_id).and_then(|info| {
                info.identity_manager
                    .managed_identity_mut(&from_identity_id)
            }) {
                Some(managed) => managed
                    .record_dashpay_payment(txid.to_string(), entry.clone(), &self.persister)
                    .map_err(|e| e.to_string()),
                None => Err("sender identity not found".to_string()),
            };
            if let Err(e) = recorded {
                tracing::error!(
                    %txid,
                    error = %e,
                    "DashPay payment broadcast but not recorded locally"
                );
            }
        }

        Ok((txid, entry, signed.fee()))
    }

    /// Drop a prepared contact payment: release its inputs (owner-guarded, so
    /// inputs a TTL sweep has since handed to another build stay with it) and
    /// return its address to the pool. Nothing was broadcast, so the address
    /// was never exposed and the next payment may take it.
    pub async fn abandon_prepared_payment<Bc>(&self, prepared: PreparedDashPayPayment<Bc>)
    where
        Bc: TransactionBroadcaster + ?Sized,
    {
        prepared.core.abandon_transaction(&prepared.signed).await;
        if self.ensure_same_wallet(&prepared.core).is_err() {
            return;
        }
        self.release_contact_payment_address(
            prepared.core.generation(),
            &prepared.from_identity_id,
            &prepared.to_contact_id,
            &prepared.payment_address,
        )
        .await;
    }

    fn ensure_same_wallet<Bc>(&self, core: &CoreWallet<Bc>) -> Result<(), PlatformWalletError>
    where
        Bc: TransactionBroadcaster + ?Sized,
    {
        if core.wallet_id == self.wallet_id
            && Arc::ptr_eq(&core.wallet_manager, &self.wallet_manager)
        {
            Ok(())
        } else {
            Err(PlatformWalletError::InvalidParameter(
                "the core wallet does not belong to this identity wallet".to_string(),
            ))
        }
    }

    /// Take the contact's next unused payment address and mark it used in
    /// memory, so a second payment prepared meanwhile gets a different one.
    async fn claim_contact_payment_address(
        &self,
        generation: &Arc<WalletGeneration>,
        from_identity_id: &Identifier,
        to_contact_id: &Identifier,
    ) -> Result<Address, PlatformWalletError> {
        let key = contact_key(from_identity_id, to_contact_id);
        let mut wm = self.wallet_manager.write().await;
        // The xpub lives on the immutable account; the pool on the managed one.
        let contact_xpub = wm
            .get_wallet(&self.wallet_id)
            .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?
            .accounts
            .dashpay_external_accounts
            .get(&key)
            .map(|account| account.account_xpub)
            .ok_or_else(|| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "No DashpayExternalAccount found for contact {to_contact_id} — call \
                     register_external_contact_account first"
                ))
            })?;
        let external_account = self
            .generation_info_mut(&mut wm, generation)?
            .core_wallet
            .accounts
            .dashpay_external_accounts
            .get_mut(&key)
            .ok_or_else(|| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "No managed DashpayExternalAccount found for contact {to_contact_id}"
                ))
            })?;
        let payment_address = external_account
            .next_address(Some(&contact_xpub), true)
            .map_err(|e| PlatformWalletError::TransactionBuild(e.to_string()))?;
        // `next_address` only selects; the used flag is what keeps DIP-15
        // rotation from handing the same address to the next payment.
        if !external_account.mark_address_used(&payment_address) {
            return Err(PlatformWalletError::TransactionBuild(
                "the derived payment address is not in the contact's address pool — cannot mark \
                 it used"
                    .to_string(),
            ));
        }
        Ok(payment_address)
    }

    /// Mark the claimed address used (again) and snapshot the pool for the
    /// pre-broadcast persist.
    async fn commit_contact_payment_address(
        &self,
        generation: &Arc<WalletGeneration>,
        from_identity_id: &Identifier,
        to_contact_id: &Identifier,
        payment_address: &Address,
    ) -> Result<PlatformWalletChangeSet, PlatformWalletError> {
        let mut wm = self.wallet_manager.write().await;
        let external_account = self
            .generation_info_mut(&mut wm, generation)?
            .core_wallet
            .accounts
            .dashpay_external_accounts
            .get_mut(&contact_key(from_identity_id, to_contact_id))
            .ok_or_else(|| {
                // Not the "account not built yet" wording the prepare path
                // uses: this account existed at prepare and is gone now.
                PlatformWalletError::InvalidIdentityData(format!(
                    "the payment account for contact {to_contact_id} was removed while the \
                     payment waited; nothing was sent"
                ))
            })?;
        // `mark_address_used` answers `false` for an address that is already
        // used as well as for one that is missing, so presence is checked on
        // its own.
        let in_pool = external_account
            .managed_account_type()
            .address_pools()
            .iter()
            .any(|pool| pool.address_index.contains_key(payment_address));
        if !in_pool {
            return Err(PlatformWalletError::TransactionBuild(
                "the payment's address is no longer in the contact's address pool; nothing \
                 was sent — please try again"
                    .to_string(),
            ));
        }
        // Another device on the same seed may have paid the contact at this
        // very address while the payment waited; a second payment there would
        // link the two on-chain. Refused: the caller abandons the inputs, and
        // the address stays used (`release_contact_payment_address` sees the
        // same record), so the next prepare takes a fresh one.
        if paid_on_chain(external_account, &payment_address.script_pubkey()) {
            return Err(PlatformWalletError::TransactionBuild(
                "another payment to this contact used the same address while this one waited; \
                 nothing was sent — please try again"
                    .to_string(),
            ));
        }
        external_account.mark_address_used(payment_address);
        Ok(pool_changeset(
            from_identity_id,
            to_contact_id,
            external_account,
        ))
    }

    /// The wallet info of `generation`, or `WalletNotFound` when the wallet
    /// under this id is gone or is a newer generation.
    fn generation_info_mut<'w>(
        &self,
        wm: &'w mut key_wallet_manager::WalletManager<PlatformWalletInfo>,
        generation: &Arc<WalletGeneration>,
    ) -> Result<&'w mut PlatformWalletInfo, PlatformWalletError> {
        wm.get_wallet_info_mut(&self.wallet_id)
            .filter(|info| Arc::ptr_eq(&info.generation, generation))
            .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))
    }

    /// Return a claimed address to the pool and persist the reverted pool
    /// after the guard drops. A failed store is logged, not fatal: the address
    /// then stays consumed on disk, a one-address gap the pool's gap window
    /// absorbs.
    ///
    /// Skipped for a wallet that is no longer `generation`, and for an address
    /// the wallet has since seen paid on-chain — another device on the same
    /// seed may have paid the contact there while this payment waited, and
    /// that address must never be handed out again.
    async fn release_contact_payment_address(
        &self,
        generation: &Arc<WalletGeneration>,
        from_identity_id: &Identifier,
        to_contact_id: &Identifier,
        payment_address: &Address,
    ) {
        let script = payment_address.script_pubkey();
        let revert = {
            let mut wm = self.wallet_manager.write().await;
            let Ok(info) = self.generation_info_mut(&mut wm, generation) else {
                return;
            };
            let Some(external_account) = info
                .core_wallet
                .accounts
                .dashpay_external_accounts
                .get_mut(&contact_key(from_identity_id, to_contact_id))
            else {
                tracing::warn!(
                    "external account not found while returning an unsent contact payment's address"
                );
                return;
            };
            if paid_on_chain(external_account, &script) {
                return;
            }
            return_contact_payment_address_to_pool(external_account, payment_address);
            pool_changeset(from_identity_id, to_contact_id, external_account)
        };
        if let Err(e) = self.persister.store(revert) {
            tracing::warn!(
                error = %e,
                "failed to persist the payment-address revert of an unsent contact payment"
            );
        }
    }
}

/// Whether the wallet holds a transaction paying `script` in this contact
/// account — a payment made there, by this device or another on the seed.
fn paid_on_chain(external_account: &ManagedCoreFundsAccount, script: &dashcore::ScriptBuf) -> bool {
    external_account.transactions().values().any(|record| {
        record
            .transaction
            .output
            .iter()
            .any(|output| &output.script_pubkey == script)
    })
}

fn contact_key(from_identity_id: &Identifier, to_contact_id: &Identifier) -> DashpayAccountKey {
    DashpayAccountKey {
        index: CONTACT_ACCOUNT_INDEX,
        user_identity_id: from_identity_id.to_buffer(),
        friend_identity_id: to_contact_id.to_buffer(),
    }
}

fn pool_changeset(
    from_identity_id: &Identifier,
    to_contact_id: &Identifier,
    external_account: &ManagedCoreFundsAccount,
) -> PlatformWalletChangeSet {
    PlatformWalletChangeSet {
        account_address_pools: account_address_pool_entries(
            AccountType::DashpayExternalAccount {
                index: CONTACT_ACCOUNT_INDEX,
                user_identity_id: from_identity_id.to_buffer(),
                friend_identity_id: to_contact_id.to_buffer(),
            },
            external_account.managed_account_type().address_pools(),
        ),
        ..Default::default()
    }
}
