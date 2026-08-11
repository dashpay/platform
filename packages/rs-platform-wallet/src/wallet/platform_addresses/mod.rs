//! DIP-17 platform payment address wallet and provider.

use std::collections::BTreeMap;

use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
pub use dpp::prelude::AddressNonce;

#[cfg(doc)]
use crate::PlatformWalletError;

mod fund_from_asset_lock;
pub(crate) mod provider;
mod sync;
mod transfer;
mod wallet;
mod withdrawal;

/// Saturating sum over `Credits` (== `u64`) — total credit supply is far
/// below `u64::MAX`, so saturation is unreachable in practice but the policy
/// keeps debug-build panics off the table. Use this only for sums over
/// wallet-derived balances; for caller-supplied input maps prefer
/// [`checked_sum_credits`] so a bogus FFI input is reported as
/// [`crate::PlatformWalletError::InputSumOverflow`] rather than silently
/// saturating to `u64::MAX`.
pub(crate) fn saturating_sum_credits<I>(iter: I) -> Credits
where
    I: IntoIterator<Item = Credits>,
{
    iter.into_iter().fold(0u64, Credits::saturating_add)
}

/// Checked sum over `Credits` for caller-supplied input maps. Returns
/// [`crate::PlatformWalletError::InputSumOverflow`] on overflow so a
/// bogus FFI caller cannot trigger a silent saturation downstream.
pub(crate) fn checked_sum_credits<I>(iter: I) -> Result<Credits, crate::PlatformWalletError>
where
    I: IntoIterator<Item = Credits>,
{
    iter.into_iter()
        .try_fold(0u64, |acc, c| acc.checked_add(c))
        .ok_or(crate::PlatformWalletError::InputSumOverflow)
}

pub use provider::{
    PerAccountPlatformAddressState, PerWalletPlatformAddressState, PlatformAddressTag,
};
pub(crate) use wallet::merge_platform_payment_candidate_addresses;
pub use wallet::PlatformAddressWallet;
pub use withdrawal::WithdrawalPlan;

/// Specifies how input addresses are selected for a transaction.
pub enum InputSelection {
    /// Explicit inputs with balances (nonces fetched automatically).
    Explicit(BTreeMap<PlatformAddress, Credits>),
    /// Explicit inputs with known nonces and balances.
    ExplicitWithNonces(BTreeMap<PlatformAddress, (AddressNonce, Credits)>),
    /// Automatically select inputs from the account.
    ///
    /// Candidates are ordered balance-descending, filtered to balances
    /// `≥ min_input_amount`, and addresses that also appear as outputs
    /// are excluded (DPP rejects same-address input+output). Supported
    /// fee strategies: `[DeductFromInput(0)]` (fee comes out of the
    /// lex-smallest input's remaining balance) and `[ReduceOutput(0)]`
    /// (fee absorbed at chain time from the lex-smallest output);
    /// other shapes must use [`Self::Explicit`].
    ///
    /// # Errors
    ///
    /// Typed variants surface diagnosable failure shapes:
    /// [`PlatformWalletError::OnlyOutputAddressesFunded`] when every
    /// funded address is also a destination,
    /// [`PlatformWalletError::OnlyDustInputs`] when every funded balance
    /// is below `min_input_amount`, and the generic
    /// [`PlatformWalletError::AddressOperation`] otherwise.
    Auto,
}
