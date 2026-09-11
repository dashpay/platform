//! BIP32 path rendering for upstream-emitted [`DerivedAddress`]
//! event payloads.
//!
//! The upstream `key_wallet_manager::DerivedAddress` event payload
//! deliberately omits the rendered derivation-path string —
//! consumers that need one are expected to recompute it
//! deterministically from `(account_type, pool_type,
//! derivation_index)`. This module is the canonical recomputation
//! site so the path-shape rules live next to other protocol-aware
//! address logic in `platform-wallet`, NOT in the FFI shim. The FFI
//! crates' job is marshaling C ABI shapes; deciding what a path
//! looks like is a `key-wallet`/`platform-wallet`-level concern.
//!
//! The layout mirrors what
//! `AddressPool::generate_address_at_index` produces internally:
//!
//! | Pool             | Path layout                  |
//! |------------------|------------------------------|
//! | `External`       | `<account-prefix>/0/<index>` |
//! | `Internal`       | `<account-prefix>/1/<index>` |
//! | `Absent`         | `<account-prefix>/<index>`   |
//! | `AbsentHardened` | `<account-prefix>/<index>'`  |
//!
//! `<account-prefix>` comes from
//! [`AccountType::derivation_path`](key_wallet::account::AccountType::derivation_path).
//! The path's only network-dependent part is the BIP44 coin type
//! (`5'` on mainnet, `1'` otherwise), so resolving mainnet-vs-not off
//! the address is sufficient and callers don't have to thread a
//! network.
//!
//! Returns `None` only for account variants whose
//! `derivation_path()` returns `Err` (some non-Standard variants
//! that have no enumerable BIP32 prefix). Callers that just want a
//! display string fall back to the empty string in that case;
//! correctness is unaffected because the address itself is the
//! authoritative join key downstream.

use key_wallet::bip32::{ChildNumber, DerivationPath};
use key_wallet::managed_account::address_pool::AddressPoolType;
use key_wallet::Network;

use crate::DerivedAddress;

/// Render the BIP32 derivation path for a `DerivedAddress` event
/// payload. See module-level docs for the path layout rules.
pub fn derivation_path_for_derived_address(derived: &DerivedAddress) -> Option<DerivationPath> {
    // `Address` does not expose its network directly — its base58/bech32
    // prefix is ambiguous across testnet/devnet/regtest. The derivation path
    // only distinguishes mainnet (coin type `5'`) from everything else (`1'`),
    // so probe for mainnet and fall back to testnet otherwise.
    let network = if derived
        .address
        .as_unchecked()
        .is_valid_for_network(Network::Mainnet)
    {
        Network::Mainnet
    } else {
        Network::Testnet
    };
    let mut path: DerivationPath = derived.account_type.derivation_path(network).ok()?;
    let leaf = match derived.pool_type {
        AddressPoolType::External => {
            path = path.child(ChildNumber::from_normal_idx(0).ok()?);
            ChildNumber::from_normal_idx(derived.derivation_index).ok()?
        }
        AddressPoolType::Internal => {
            path = path.child(ChildNumber::from_normal_idx(1).ok()?);
            ChildNumber::from_normal_idx(derived.derivation_index).ok()?
        }
        AddressPoolType::Absent => ChildNumber::from_normal_idx(derived.derivation_index).ok()?,
        AddressPoolType::AbsentHardened => {
            ChildNumber::from_hardened_idx(derived.derivation_index).ok()?
        }
    };
    path = path.child(leaf);
    Some(path)
}

/// Display-string variant of
/// [`derivation_path_for_derived_address`]. Returns `None` for the
/// same cases.
pub fn derivation_path_string_for_derived_address(derived: &DerivedAddress) -> Option<String> {
    derivation_path_for_derived_address(derived).map(|p| p.to_string())
}
