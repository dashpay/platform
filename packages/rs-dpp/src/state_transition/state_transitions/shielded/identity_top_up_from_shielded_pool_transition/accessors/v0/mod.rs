use crate::shielded::SerializedAction;
use platform_value::Identifier;

/// Accessors for the fields of an `IdentityTopUpFromShieldedPoolTransition`.
pub trait IdentityTopUpFromShieldedPoolTransitionAccessorsV0 {
    /// The identity whose balance receives the top-up.
    fn identity_id(&self) -> Identifier;
    /// Set the target identity.
    fn set_identity_id(&mut self, identity_id: Identifier);

    /// Get the serialized Orchard actions (spend/output pairs).
    fn actions(&self) -> &[SerializedAction];
    /// Replace the serialized Orchard actions.
    fn set_actions(&mut self, actions: Vec<SerializedAction>);

    /// Gross credits leaving the pool (the bundle's value balance). The identity
    /// receives this minus the flat shielded top-up fee.
    fn top_up_amount(&self) -> u64;
    /// Set the gross top-up amount.
    fn set_top_up_amount(&mut self, top_up_amount: u64);

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

    /// Extract nullifier bytes from each action.
    fn nullifiers<T: From<[u8; 32]>>(&self) -> Vec<T> {
        self.actions()
            .iter()
            .map(|a| T::from(a.nullifier))
            .collect()
    }
}
