//! Two-step DashPay contact payment: sign and reserve now, broadcast or
//! abandon after the user has seen the exact fee.
//!
//! [`DashPayView::send_payment`] builds, signs and broadcasts in one call and
//! reports the fee only afterwards, so a host has nothing to show on a
//! confirmation screen but the amount. The split here mirrors the L1 path a
//! plain send already takes ([`CoreWallet::finalize_transaction`] →
//! [`CoreWallet::broadcast_finalized_transaction`] /
//! [`CoreWallet::abandon_transaction`]): [`DashPayView::prepare_payment`]
//! picks the contact's next DIP-15 address, funds, reserves and signs, and
//! returns a [`PreparedDashPayPayment`] that carries the exact fee;
//! [`DashPayView::send_prepared_payment`] takes the address, broadcasts and
//! records the payment; [`DashPayView::abandon_prepared_payment`] releases
//! the inputs.
//!
//! Between the two steps the inputs are held by the finalized transaction's
//! owner-guarded reservation, under the same age bound the L1 path enforces —
//! a payment held past it is refused at send time with
//! [`PlatformWalletError::StaleReservation`] and must be prepared again.
//!
//! The contact address is NOT claimed while the payment waits. Prepare only
//! reads the contact's next available DIP-15 address, so a preview never
//! writes the address pool, and nothing has to be undone when it is
//! abandoned, when its build fails, or when the future preparing it is
//! dropped.
//!
//! Send claims the address in two steps that keep the claim apart from the
//! evidence that the address was paid:
//!
//! 1. Under the manager lock it checks that the address is still available —
//!    not used, not reserved, and paid by no resident transaction record —
//!    and marks it **reserved**. The pool is stored, and flushed when the
//!    backend buffers, before anything reaches the network. The FFI host
//!    stores a reservation as used, so a relaunch after a crash mid-send does
//!    not hand the address out again. SQLite stores it reversibly
//!    (`reserved_at`) but does not yet restore a contact's external pool on
//!    load, used rows included (dashpay/platform#5548).
//! 2. When the broadcast went out, or may have (an ambiguous outcome), the
//!    reservation becomes **used** and is stored. When it provably did not —
//!    the store failed, the network rejected it, or it was refused as stale
//!    at dispatch — only the reservation is released, and that release is
//!    stored.
//!
//! The used flag is never reverted. key-wallet sets it when it observes a
//! payment to the address, and a reservation it promotes that way is no
//! longer a reservation to release: a payment another device on the same
//! seed made there while this one was in flight keeps the address used, even
//! after the ChainLock evicted its record. The release lowers only what the
//! reservation raised: SQLite clears `reserved_at`, and the FFI host, which
//! stores a reservation as used, stores the released address as unused.
//!
//! Every address change is bound to the wallet generation the payment was
//! funded from: a payment that outlives its wallet (removed, or re-created
//! under the same id) never touches the new generation's pool. A send
//! dropped between reserve and settle leaves the address reserved, one
//! skipped index: the reservation sweep covers only Standard accounts.

use dashcore::Address;
use dpp::prelude::Identifier;
use key_wallet::account::account_collection::DashpayAccountKey;
use key_wallet::account::AccountType;
use key_wallet::managed_account::address_pool::AddressState;
use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
use key_wallet::managed_account::ManagedCoreFundsAccount;
use key_wallet::wallet::managed_wallet_info::coin_selection::SelectionStrategy;
use key_wallet::wallet::managed_wallet_info::transaction_builder::TransactionBuilder;
use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;
use std::sync::Arc;

use super::contact_requests::ContactCryptoProvider;
use super::payments::contact_address_paid_on_chain;
use super::DashPayView;
use crate::broadcaster::TransactionBroadcaster;
use crate::changeset::{account_address_pool_entries, PlatformWalletChangeSet};
use crate::error::PlatformWalletError;
use crate::util::now_secs;
use crate::wallet::core::{CoreWallet, SignedCoreTransaction, WalletGeneration};
use crate::wallet::identity::types::dashpay::payment::PaymentEntry;
use crate::wallet::platform_wallet::PlatformWalletInfo;
use crate::wallet::reservations::reservation_expired;
use crate::SEND_FUNDING_SOURCES;

/// The DashPay contact account index every contact payment uses, the same one
/// [`DashPayView::send_payment`] pins.
const CONTACT_ACCOUNT_INDEX: u32 = 0;

/// A signed DashPay contact payment that has not been broadcast yet.
///
/// Its funding inputs stay reserved until it is handed to
/// [`DashPayView::send_prepared_payment`] or
/// [`DashPayView::abandon_prepared_payment`]. Deliberately not `Clone`: one
/// prepare yields one payment that can be sent or abandoned exactly once.
/// Dropping it without either leaves the reservation to key-wallet's TTL;
/// hosts abandon instead.
#[must_use = "a prepared payment holds its funding inputs until it is sent or abandoned"]
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

    /// The core wallet the payment was funded from. A host compares its
    /// generation with its own handle's; it must not take that generation's
    /// lifecycle gate around [`DashPayView::send_prepared_payment`], which
    /// takes it itself.
    pub fn core_wallet(&self) -> &CoreWallet<B> {
        &self.core
    }
}

#[cfg(any(test, feature = "test-utils"))]
impl<B: TransactionBroadcaster + ?Sized> PreparedDashPayPayment<B> {
    /// A prepared payment around an already signed transaction, for tests
    /// that need a payment to hand over without funding and signing one.
    pub fn new_for_test(
        core: CoreWallet<B>,
        signed: SignedCoreTransaction,
        payment_address: Address,
        from_identity_id: Identifier,
        to_contact_id: Identifier,
        amount_duffs: u64,
    ) -> Self {
        Self {
            core,
            signed,
            payment_address,
            from_identity_id,
            to_contact_id,
            amount_duffs,
            memo: None,
        }
    }
}

impl<B: TransactionBroadcaster + ?Sized> DashPayView<'_, B> {
    /// Sign a Core payment to a DashPay contact without broadcasting it.
    ///
    /// Runs the first half of [`Self::send_payment`]: drains any deferred
    /// contact-crypto build behind the seed-binding gate, reads the contact's
    /// next available DIP-15 address, and funds, reserves and signs the
    /// transaction from the same pooled sources a plain send uses
    /// ([`SEND_FUNDING_SOURCES`]: BIP44 first, so change keeps returning to
    /// BIP44). `core` must be this wallet's core handle; it funds the payment
    /// and later broadcasts it.
    ///
    /// The address pool is left as it was: [`Self::send_prepared_payment`]
    /// reserves the address, stores that reservation before the broadcast,
    /// then makes it used or releases it (see the module docs). Two payments
    /// prepared to one contact at the same time therefore pay the same
    /// address, and the second one sent is refused.
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

        let payment_address = self
            .next_contact_payment_address(core.generation(), from_identity_id, to_contact_id)
            .await?;

        let builder = TransactionBuilder::new()
            .set_selection_strategy(SelectionStrategy::LargestFirst)
            .add_output(&payment_address, amount_duffs);
        let signed = core
            .finalize_transaction(
                builder,
                &SEND_FUNDING_SOURCES,
                CONTACT_ACCOUNT_INDEX,
                signer,
            )
            .await?;

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
    /// The second half of [`Self::send_payment`]: under the manager lock the
    /// payment's address is checked to be still available and its funding
    /// reservation still fresh, then reserved; the reservation is stored and
    /// flushed before anything reaches the network. An address another
    /// payment took meanwhile, a stale reservation, or a failed store sends
    /// nothing and releases the inputs. A definitive rejection releases the
    /// inputs too; an ambiguous outcome keeps them fenced, since the
    /// transaction may have propagated. The address ends used when the
    /// transaction went out or may have, and available again when it
    /// provably did not (see the module docs).
    ///
    /// Returns the txid, the payment's [`PaymentEntry`] and the fee — the same
    /// fee [`PreparedDashPayPayment::fee_duffs`] showed. Once the broadcast
    /// succeeded the call succeeds even if the entry could not be recorded on
    /// the sender (that failure is logged): the entry is then not in the
    /// wallet's payment map until the reconstruction from transaction history
    /// rebuilds it, without the memo.
    ///
    /// Takes the funding generation's lifecycle gate itself: a caller must
    /// not hold it, since a teardown queued between the two reads would wait
    /// on the outer one forever. A wallet removed or re-created since the
    /// payment was prepared fails it with
    /// [`PlatformWalletError::WalletRemovedBeforeSend`], nothing sent.
    pub async fn send_prepared_payment<Bc>(
        &self,
        prepared: PreparedDashPayPayment<Bc>,
    ) -> Result<(dashcore::Txid, PaymentEntry, u64), PlatformWalletError>
    where
        Bc: TransactionBroadcaster + ?Sized,
    {
        if let Err(e) = self.ensure_same_wallet(&prepared.core) {
            // Not this wallet's payment: free its inputs and send nothing.
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

        // The generation's lifecycle gate, held across the liveness check and
        // everything after it: a teardown cannot complete between the check
        // and the broadcast, so a removed wallet's payment never reaches the
        // network. Taken here rather than by each host wrapper, so a native
        // caller gets the same exclusion.
        let _lifecycle = core.generation_payment_guard().await;
        if !core.is_current_generation().await {
            core.abandon_transaction(&signed).await;
            return Err(PlatformWalletError::WalletRemovedBeforeSend(hex::encode(
                self.wallet_id,
            )));
        }

        // Every contact-pool snapshot below is taken and stored under the
        // contact-payment gate, which `send_payment` and
        // `reserve_payment_address` take as well: two sends to one contact
        // store their snapshots in the order they took them, so an older
        // snapshot never overwrites a newer reservation. The gate is not held
        // across the broadcast.
        let contact_gate = self.persister.lock_contact_payments().await;

        // Reserve under the write guard; store after it drops — a host
        // persistence callback must never run under the manager lock.
        let generation = core.generation();
        let reserved = match self
            .reserve_contact_payment_address(
                generation,
                &signed,
                &from_identity_id,
                &to_contact_id,
                &payment_address,
            )
            .await
        {
            Ok(changeset) => changeset,
            Err(refusal) => {
                // A failed store outranks the refusal: the caller learns the
                // store is failing, not just to try again. The mark stays in
                // memory and rides the contact's next pool snapshot.
                let stored = refusal
                    .found_paid
                    .map_or(Ok(()), |found_paid| self.store_durably(*found_paid));
                core.abandon_transaction(&signed).await;
                return Err(match stored {
                    Ok(()) => refusal.error,
                    Err(e) => {
                        tracing::warn!(
                            refusal = %refusal.error,
                            "a contact payment was refused and the address found paid could \
                             not be stored"
                        );
                        e
                    }
                });
            }
        };
        let stored = self.store_durably(reserved);
        // `settle_payment_address` takes the gate itself.
        drop(contact_gate);
        if let Err(e) = stored {
            // Nothing is on the wire: release the address first, so a build
            // that takes the freed inputs never sees it still reserved.
            self.settle_payment_address(
                generation,
                &from_identity_id,
                &to_contact_id,
                &payment_address,
                AddressOutcome::NotSent,
            )
            .await;
            core.abandon_transaction(&signed).await;
            return Err(e);
        }

        // The age guard, the in-broadcast pin and the owner-guarded release
        // on a definitive rejection all live in the finalized-broadcast path.
        let broadcast = core.broadcast_finalized_transaction(&signed).await;
        // Rejected, or refused as stale at dispatch: provably not on the
        // wire. Anything else went out or may have.
        let outcome = match &broadcast {
            Err(
                PlatformWalletError::TransactionBroadcast(_)
                | PlatformWalletError::StaleReservation,
            ) => AddressOutcome::NotSent,
            _ => AddressOutcome::Sent,
        };
        self.settle_payment_address(
            generation,
            &from_identity_id,
            &to_contact_id,
            &payment_address,
            outcome,
        )
        .await;
        let txid = broadcast?;

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
            // Bound to the funding generation like every other write here: a
            // wallet re-created meanwhile must not receive this payment.
            let recorded = match self
                .generation_info_mut(&mut wm, core.generation())
                .ok()
                .and_then(|info| {
                    info.identity_manager
                        .managed_identity_mut(&from_identity_id)
                }) {
                Some(managed) => managed
                    .record_dashpay_payment(txid.to_string(), entry.clone(), &self.persister)
                    .map_err(|e| e.to_string()),
                None => Err(
                    "sender identity not found, or the wallet was removed or re-created"
                        .to_string(),
                ),
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
    /// inputs a TTL sweep has since handed to another build stay with it).
    /// The address pool was never touched, so there is nothing else to undo
    /// and nothing is persisted.
    pub async fn abandon_prepared_payment<Bc>(&self, prepared: PreparedDashPayPayment<Bc>)
    where
        Bc: TransactionBroadcaster + ?Sized,
    {
        prepared.core.abandon_transaction(&prepared.signed).await;
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

    /// The contact's next available payment address. Marks nothing: the only
    /// change it may make is deriving lookahead addresses into the in-memory
    /// pool, which `send_payment`'s selection does as well.
    async fn next_contact_payment_address(
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
        self.generation_info_mut(&mut wm, generation)?
            .core_wallet
            .accounts
            .dashpay_external_accounts
            .get_mut(&key)
            .ok_or_else(|| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "No managed DashpayExternalAccount found for contact {to_contact_id}"
                ))
            })?
            .next_address(Some(&contact_xpub), true)
            .map_err(|e| PlatformWalletError::TransactionBuild(e.to_string()))
    }

    /// Check that the payment can still go out — its address still available
    /// and its funding reservation still fresh — then reserve the address and
    /// snapshot the pool for the pre-broadcast store. On a refusal nothing is
    /// changed, except an available address a resident record pays, which is
    /// marked used.
    async fn reserve_contact_payment_address(
        &self,
        generation: &Arc<WalletGeneration>,
        signed: &SignedCoreTransaction,
        from_identity_id: &Identifier,
        to_contact_id: &Identifier,
        payment_address: &Address,
    ) -> Result<PlatformWalletChangeSet, ReserveRefusal> {
        let mut wm = self.wallet_manager.write().await;
        let info = self.generation_info_mut(&mut wm, generation)?;
        // Checked here as well as at dispatch: a payment held past the bound
        // is the common way a confirmation screen goes stale, and refusing it
        // before the mark keeps its address available. The dispatch check stays
        // the authoritative one.
        if reservation_expired(
            signed.reservation_height(),
            Some(info.core_wallet.last_processed_height()),
        ) {
            return Err(PlatformWalletError::StaleReservation.into());
        }
        let external_account = info
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
        let state = external_account
            .managed_account_type()
            .address_pools()
            .iter()
            .find_map(|pool| pool.address_info(payment_address))
            .map(|info| info.is_available());
        match state {
            None => {
                return Err(PlatformWalletError::TransactionBuild(
                    "the payment's address is no longer in the contact's address pool; nothing \
                     was sent — please try again"
                        .to_string(),
                )
                .into())
            }
            // Available, and no resident record pays it. Otherwise another
            // payment to the contact went to this address while this one
            // waited — sent from here, or seen on-chain from another device
            // on the same seed — or another hand-out reserved it. A second
            // payment there would link the two on-chain.
            Some(true)
                if !contact_address_paid_on_chain(
                    external_account,
                    &payment_address.script_pubkey(),
                ) => {}
            Some(available) => {
                // Paid on-chain but not yet marked: record that, persisted,
                // or the next prepare would pick this address again and be
                // refused the same way.
                let found_paid = (available && external_account.mark_address_used(payment_address))
                    .then(|| {
                        Box::new(pool_changeset(
                            from_identity_id,
                            to_contact_id,
                            external_account,
                        ))
                    });
                return Err(ReserveRefusal {
                    error: PlatformWalletError::TransactionBuild(
                        "another payment to this contact took the same address while this one \
                         waited; nothing was sent — please try again"
                            .to_string(),
                    ),
                    found_paid,
                });
            }
        }
        reserve_address(external_account, payment_address);
        Ok(pool_changeset(
            from_identity_id,
            to_contact_id,
            external_account,
        ))
    }

    /// Settle the reservation a send took: promote it to used when the
    /// transaction went out or may have, release it when it provably did not.
    /// The result is stored after the guard drops; a failed store is logged —
    /// the payment's outcome no longer depends on it.
    ///
    /// Only the reservation is touched. An address key-wallet has meanwhile
    /// seen paid is already used and stays so — the reservation it promoted
    /// is gone, and the release finds nothing to release.
    pub(super) async fn settle_payment_address(
        &self,
        generation: &Arc<WalletGeneration>,
        from_identity_id: &Identifier,
        to_contact_id: &Identifier,
        payment_address: &Address,
        outcome: AddressOutcome,
    ) {
        // Held from the snapshot through its store, like the reservation.
        let _contact_gate = self.persister.lock_contact_payments().await;
        self.settle_payment_address_gated(
            generation,
            from_identity_id,
            to_contact_id,
            payment_address,
            outcome,
        )
        .await;
    }

    /// [`Self::settle_payment_address`] for a caller already holding the
    /// contact-payment gate.
    pub(super) async fn settle_payment_address_gated(
        &self,
        generation: &Arc<WalletGeneration>,
        from_identity_id: &Identifier,
        to_contact_id: &Identifier,
        payment_address: &Address,
        outcome: AddressOutcome,
    ) {
        let settled = {
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
                    to_contact = %to_contact_id,
                    ?outcome,
                    "external account not found while settling a contact payment's address"
                );
                return;
            };
            // A resident record paying the address is evidence too, even if
            // whatever made it resident did not mark the address.
            let paid =
                contact_address_paid_on_chain(external_account, &payment_address.script_pubkey());
            let changed = match outcome {
                AddressOutcome::NotSent if !paid => {
                    release_address_reservation(external_account, payment_address)
                }
                _ => external_account.mark_address_used(payment_address),
            };
            if !changed {
                let in_pool = external_account
                    .managed_account_type()
                    .address_pools()
                    .iter()
                    .any(|pool| pool.address_info(payment_address).is_some());
                if !in_pool {
                    tracing::warn!(
                        to_contact = %to_contact_id,
                        payment_address = %payment_address,
                        ?outcome,
                        "a contact payment's address left the pool while it was sent; \
                         it could not be settled"
                    );
                    return;
                }
                // Nothing to change in memory — typically already used: the
                // wallet observed the payment, this one or another device's,
                // while it was in flight, and that observation stores no
                // contact pool. The pool is stored all the same, so disk
                // matches memory rather than the reservation.
            }
            pool_changeset(from_identity_id, to_contact_id, external_account)
        };
        // A release is flushed like the reservation, so a relaunch does not
        // read back the reservation it replaces. A used address needs no
        // flush: the reservation already stored guards it as the used mark
        // would — the FFI host reloads either as used.
        let stored = match outcome {
            AddressOutcome::NotSent => self.store_durably(settled),
            AddressOutcome::Sent => self
                .persister
                .store(settled)
                .map_err(|e| self.persister.classify_store_failure(e)),
        };
        if let Err(e) = stored {
            tracing::warn!(
                error = %e,
                ?outcome,
                "failed to persist a contact payment's settled address"
            );
        }
    }

    /// Store `changeset` and, on a backend that buffers, flush it, so it is
    /// on disk when this returns. A failed flush is `PersisterFlush` with the
    /// backend's own kind: the round may still be buffered and commit at a
    /// later flush, so it is not proof that nothing was stored.
    pub(super) fn store_durably(
        &self,
        changeset: PlatformWalletChangeSet,
    ) -> Result<(), PlatformWalletError> {
        self.persister
            .store(changeset)
            .map_err(|e| self.persister.classify_store_failure(e))?;
        if !self.persister.store_commits_inline() {
            self.persister
                .flush()
                .map_err(PlatformWalletError::from_flush_failure)?;
        }
        Ok(())
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
}

/// Whether a send's transaction went out (or may have), or provably did not.
#[derive(Debug, Clone, Copy)]
pub(super) enum AddressOutcome {
    Sent,
    NotSent,
}

/// Why [`DashPayView::reserve_contact_payment_address`] refused a send, and —
/// when it found the address paid on-chain but not yet marked — the pool
/// snapshot recording that, to store after the guard drops.
struct ReserveRefusal {
    error: PlatformWalletError,
    found_paid: Option<Box<PlatformWalletChangeSet>>,
}

impl From<PlatformWalletError> for ReserveRefusal {
    fn from(error: PlatformWalletError) -> Self {
        Self {
            error,
            found_paid: None,
        }
    }
}

/// Mark `address` reserved in whichever of `account`'s pools holds it, if it
/// is available there — a used address is never downgraded to a reservation
/// a release could then free. Returns whether it was reserved.
pub(super) fn reserve_address(account: &mut ManagedCoreFundsAccount, address: &Address) -> bool {
    let reserved_at = now_secs();
    let mut reserved = false;
    for pool in account.managed_account_type_mut().address_pools_mut() {
        if let Some(info) = pool.address_info_mut(address) {
            if info.is_available() {
                info.state = AddressState::Reserved { at: reserved_at };
                reserved = true;
            }
        }
    }
    reserved
}

/// Release `address`'s reservation. A used or available address is left as
/// it is. Returns whether a reservation was released.
pub(super) fn release_address_reservation(
    account: &mut ManagedCoreFundsAccount,
    address: &Address,
) -> bool {
    account
        .managed_account_type_mut()
        .address_pools_mut()
        .into_iter()
        .any(|pool| {
            pool.address_index(address)
                .is_some_and(|index| pool.release_reservation(index))
        })
}

pub(super) fn contact_key(
    from_identity_id: &Identifier,
    to_contact_id: &Identifier,
) -> DashpayAccountKey {
    DashpayAccountKey {
        index: CONTACT_ACCOUNT_INDEX,
        user_identity_id: from_identity_id.to_buffer(),
        friend_identity_id: to_contact_id.to_buffer(),
    }
}

pub(super) fn pool_changeset(
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
