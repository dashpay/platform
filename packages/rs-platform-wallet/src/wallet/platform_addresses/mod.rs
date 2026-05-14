//! DIP-17 platform payment address wallet and provider.

use std::collections::BTreeMap;

use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
pub use dpp::prelude::AddressNonce;

mod fund_from_asset_lock;
pub(crate) mod provider;
mod sync;
mod transfer;
mod wallet;
mod withdrawal;

/// Saturating sum over `Credits` (== `u64`) — total credit supply is far
/// below `u64::MAX`, so saturation is unreachable in practice but the policy
/// keeps debug-build panics off the table.
pub(crate) fn saturating_sum_credits<I>(iter: I) -> Credits
where
    I: IntoIterator<Item = Credits>,
{
    iter.into_iter().fold(0u64, Credits::saturating_add)
}

pub use provider::{
    PerAccountPlatformAddressState, PerWalletPlatformAddressState, PlatformAddressTag,
};
pub use wallet::PlatformAddressWallet;

/// Specifies how input addresses are selected for a transaction.
pub enum InputSelection {
    /// Explicit inputs with balances (nonces fetched automatically).
    Explicit(BTreeMap<PlatformAddress, Credits>),
    /// Explicit inputs with known nonces and balances.
    ExplicitWithNonces(BTreeMap<PlatformAddress, (AddressNonce, Credits)>),
    /// Automatically select inputs from the account, consuming addresses
    /// from lowest derivation index to highest until the required amount
    /// plus estimated fees is covered.
    Auto,
}
