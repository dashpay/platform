use crate::shielded::SerializedAction;

pub trait ShieldedTransferTransitionAccessorsV0 {
    /// Get the serialized Orchard actions
    fn actions(&self) -> &[SerializedAction];

    /// Extract nullifier bytes from each action.
    /// Generic over the element type: use `Vec<u8>` or `[u8; 32]` as needed.
    fn nullifiers<T: From<[u8; 32]>>(&self) -> Vec<T> {
        self.actions()
            .iter()
            .map(|a| T::from(a.nullifier))
            .collect()
    }
}
