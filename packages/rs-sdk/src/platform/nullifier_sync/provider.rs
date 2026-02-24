//! Nullifier provider trait for nullifier synchronization.

use super::types::NullifierKey;
use std::collections::BTreeSet;

/// Trait for providing nullifier keys to check against the nullifier tree.
///
/// Unlike [`AddressProvider`](crate::platform::address_sync::AddressProvider),
/// this trait is simpler — no gap limit, no index, no callbacks that extend the set.
/// It just provides a fixed set of nullifier keys to check.
///
/// # Example
///
/// ```rust,ignore
/// use dash_sdk::platform::nullifier_sync::NullifierProvider;
///
/// let nullifiers: Vec<[u8; 32]> = vec![[0u8; 32], [1u8; 32]];
/// // Vec<[u8; 32]> implements NullifierProvider directly
/// let result = sdk.sync_nullifiers(&nullifiers, None).await?;
/// ```
pub trait NullifierProvider: Send {
    /// Get the set of nullifier keys to check.
    fn nullifiers_to_check(&self) -> Vec<NullifierKey>;
}

impl NullifierProvider for Vec<NullifierKey> {
    fn nullifiers_to_check(&self) -> Vec<NullifierKey> {
        self.clone()
    }
}

impl NullifierProvider for BTreeSet<NullifierKey> {
    fn nullifiers_to_check(&self) -> Vec<NullifierKey> {
        self.iter().copied().collect()
    }
}

impl NullifierProvider for &[NullifierKey] {
    fn nullifiers_to_check(&self) -> Vec<NullifierKey> {
        self.to_vec()
    }
}
