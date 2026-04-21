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
    /// Call this on wallet open and on periodic refresh. Failures in either
    /// step propagate immediately; partial progress is not rolled back.
    pub async fn dashpay_sync(&self) -> Result<(), PlatformWalletError> {
        // Contact requests first — may establish new contacts.
        self.sync_contact_requests().await?;
        // Then profiles for all managed identities.
        self.sync_profiles().await?;
        Ok(())
    }
}
