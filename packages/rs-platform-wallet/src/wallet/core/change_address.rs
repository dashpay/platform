//! Shared change-address selection for the broadcast and DashPay payment
//! send paths.
//!
//! Both build a transaction under the wallet write lock and must pick a
//! BIP-44 change address that no concurrent in-flight send has already
//! peeked into `pending_change`. The selection rule is identical at both
//! sites, so it lives here once.

use dashcore::Address;
use key_wallet::bip32::ExtendedPubKey;
use key_wallet::managed_account::managed_core_funds_account::ManagedCoreFundsAccount;

use super::reservations::OutpointReservations;
use crate::error::PlatformWalletError;

/// Pick the next change address that is not already pending from a
/// concurrent in-flight send, committing the derivation-index advance
/// under the caller's wallet write lock.
///
/// Peeks `next_change_address(.., advance=false)`; if the peeked address is
/// in the `pending_change` snapshot it advances past the index
/// (`advance=true`) and retries, otherwise it commits the advance and
/// returns the peeked address. Advancing burns at most one index per
/// concurrent in-flight send — a bounded, acceptable privacy cost; on
/// broadcast failure a single index is burned, also acceptable. Index
/// reuse is not.
///
/// The caller must hold the wallet write lock across this call and record
/// the returned address into `reservations.pending_change` (via
/// `reserve(.., Some(addr))`) before releasing the lock, so a concurrent
/// caller cannot select it.
pub(crate) fn pick_and_reserve_change_address(
    reservations: &OutpointReservations,
    managed_account: &mut ManagedCoreFundsAccount,
    xpub: &ExtendedPubKey,
) -> Result<Address, PlatformWalletError> {
    let pending_change = reservations.pending_change_snapshot();
    loop {
        let peeked = managed_account
            .next_change_address(Some(xpub), false)
            .map_err(|e| PlatformWalletError::TransactionBuild(e.to_string()))?;
        // Commit the advance under the write lock the caller holds. On
        // broadcast failure a single index is burned; on success the
        // pending-change entry is released when the reservation guard drops
        // post-`check_core_transaction`.
        let _ = managed_account
            .next_change_address(Some(xpub), true)
            .map_err(|e| PlatformWalletError::TransactionBuild(e.to_string()))?;
        if !pending_change.contains(&peeked) {
            return Ok(peeked);
        }
    }
}
