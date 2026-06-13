use crate::address_funds::PlatformAddress;
use crate::shielded::SerializedAction;

/// Accessors for the fields of a `ShieldFromAssetLockTransition`.
///
/// `asset_lock_proof` is exposed through
/// [`AssetLockProved`](crate::identity::state_transition::AssetLockProved) and the ECDSA
/// `signature` through
/// [`StateTransitionSingleSigned`](crate::state_transition::StateTransitionSingleSigned), so they
/// are intentionally not duplicated here.
pub trait ShieldFromAssetLockTransitionAccessorsV0 {
    /// Get the serialized Orchard actions (spend/output pairs).
    fn actions(&self) -> &[SerializedAction];
    /// Replace the serialized Orchard actions.
    fn set_actions(&mut self, actions: Vec<SerializedAction>);

    /// Get the amount of credits flowing into the shielded pool from the asset lock.
    fn value_balance(&self) -> u64;
    /// Set the amount of credits flowing into the shielded pool from the asset lock.
    fn set_value_balance(&mut self, value_balance: u64);

    /// Get the Orchard anchor (Sinsemilla root of the note commitment tree).
    fn anchor(&self) -> [u8; 32];
    /// Set the Orchard anchor.
    fn set_anchor(&mut self, anchor: [u8; 32]);

    /// Get the Halo2 proof bytes.
    fn proof(&self) -> &[u8];
    /// Set the Halo2 proof bytes.
    fn set_proof(&mut self, proof: Vec<u8>);

    /// Get the RedPallas binding signature.
    fn binding_signature(&self) -> [u8; 64];
    /// Set the RedPallas binding signature.
    fn set_binding_signature(&mut self, binding_signature: [u8; 64]);

    /// Get the optional platform-address output that receives the asset-lock surplus.
    fn surplus_output(&self) -> Option<&PlatformAddress>;
    /// Set the optional platform-address output that receives the asset-lock surplus.
    fn set_surplus_output(&mut self, surplus_output: Option<PlatformAddress>);

    /// Extract nullifier bytes from each action.
    /// Generic over the element type: use `Vec<u8>` or `[u8; 32]` as needed.
    fn nullifiers<T: From<[u8; 32]>>(&self) -> Vec<T> {
        self.actions()
            .iter()
            .map(|a| T::from(a.nullifier))
            .collect()
    }
}
