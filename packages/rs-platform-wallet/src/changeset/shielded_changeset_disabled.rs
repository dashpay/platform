//! Inert stand-in for [`ShieldedChangeSet`] used when the `shielded`
//! feature is off.
//!
//! [`PlatformWalletChangeSet::shielded`] is a field in every feature
//! combination so downstream crates can destructure the changeset
//! exhaustively. They have to: a crate cannot `cfg` on a dependency's
//! feature, so a field that exists only under `platform-wallet/shielded`
//! is a field they can neither name nor omit once Cargo's feature
//! unification turns that feature on behind their back.
//!
//! [`ShieldedChangeSet`]: crate::changeset::ShieldedChangeSet
//! [`PlatformWalletChangeSet::shielded`]: crate::changeset::PlatformWalletChangeSet::shielded

use crate::changeset::merge::Merge;

/// Shielded delta that can never carry data — the `shielded` feature is off.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ShieldedChangeSet;

impl ShieldedChangeSet {
    /// Always `true`; this stand-in has nowhere to hold a delta.
    pub fn is_empty(&self) -> bool {
        true
    }
}

impl Merge for ShieldedChangeSet {
    fn merge(&mut self, _other: Self) {}

    fn is_empty(&self) -> bool {
        true
    }
}
