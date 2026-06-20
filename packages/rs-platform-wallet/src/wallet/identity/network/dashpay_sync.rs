//! Comprehensive DashPay-sync aggregator.
//!
//! Glue method that drives the two step DashPay refresh (contact
//! requests then profiles). Lives in its own file so the
//! `IdentityWallet` handle's operation surface stays one-method-per-file.

use super::*;
use crate::broadcaster::TransactionBroadcaster;
use crate::error::PlatformWalletError;

impl<B: TransactionBroadcaster + ?Sized> IdentityWallet<B> {
    /// Comprehensive DashPay sync: contact requests followed by profiles.
    ///
    /// Call this on wallet open and on periodic refresh (the recurring
    /// [`DashPaySyncManager`](crate::manager::dashpay_sync::DashPaySyncManager)
    /// loop drives it per sweep). Partial progress is not rolled back.
    ///
    /// **Step independence (log-and-continue):** the two steps are run
    /// independently — a failure in the contact-request step is logged
    /// but does **not** skip the profile step, and vice versa. The first
    /// error encountered is returned after both steps have been
    /// attempted, so the caller (the recurring sweep) can record this
    /// wallet's outcome as failed while the rest of the sweep continues.
    /// The per-*identity* continue (so one identity's fetch failure
    /// doesn't abort the others within a step) lives inside
    /// `sync_contact_requests` / `sync_profiles` themselves.
    pub async fn dashpay_sync(&self) -> Result<(), PlatformWalletError> {
        // Contact requests first — may establish new contacts.
        let contact_result = self.sync_contact_requests().await;
        if let Err(e) = &contact_result {
            tracing::warn!(
                wallet_id = %hex::encode(self.wallet_id()),
                error = %e,
                "DashPay contact-request sync failed; continuing to profile sync"
            );
        }

        // Then profiles for all managed identities — attempted even if
        // the contact-request step failed.
        let profile_result = self.sync_profiles().await;
        if let Err(e) = &profile_result {
            tracing::warn!(
                wallet_id = %hex::encode(self.wallet_id()),
                error = %e,
                "DashPay profile sync failed"
            );
        }

        // Contact profiles (established contacts + pending senders) so the
        // UI shows their name/avatar. A distinct step from `sync_profiles`
        // (own identities) — different target set and cache. Log-and-continue:
        // a fetch failure degrades display only, never the sweep outcome.
        if let Err(e) = self.sync_contact_profiles().await {
            tracing::warn!(
                wallet_id = %hex::encode(self.wallet_id()),
                error = %e,
                "DashPay contact-profile sync failed"
            );
        }

        // Step 3: contactInfo (alias/note/hidden) — cross-device
        // metadata. Log-and-continue like the steps above; a failure
        // here must not abort the payment reconcile below.
        if let Err(e) = self.sync_contact_infos().await {
            tracing::warn!(
                wallet_id = %hex::encode(self.wallet_id()),
                error = %e,
                "DashPay contactInfo sync failed"
            );
        }

        // Local-only third step: derive any missing `Received` payment
        // entries from the receival accounts' restored UTXO sets (see
        // `reconcile_incoming_payments`). Runs after the contact step so
        // freshly established contacts' accounts are registered first.
        // Never fails the pass — it touches no network and its error is
        // a wallet-lookup miss at worst.
        if let Err(e) = self.reconcile_incoming_payments().await {
            tracing::warn!(
                wallet_id = %hex::encode(self.wallet_id()),
                error = %e,
                "DashPay incoming-payment reconcile failed"
            );
        }

        // Local-only fourth step: confirm any `Pending` `Sent` payment whose
        // transaction the persisted core record reports final (mined or
        // InstantSend-locked). Recovers a sent payment whose live
        // confirm event was missed (lagged broadcast, or relaunch after the
        // tx confirmed) — see `reconcile_sent_payments`. Never fails the
        // pass.
        if let Err(e) = self.reconcile_sent_payments().await {
            tracing::warn!(
                wallet_id = %hex::encode(self.wallet_id()),
                error = %e,
                "DashPay sent-payment reconcile failed"
            );
        }

        // Surface the first error (if any) so the recurring sweep records
        // a failed outcome for this wallet; both steps have already run.
        contact_result?;
        profile_result?;
        Ok(())
    }
}
