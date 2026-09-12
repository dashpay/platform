use crate::prelude::IdentityNonce;
use crate::shielded::SerializedAction;
use platform_value::Identifier;

/// Accessors for the fields of a `ShieldFromIdentityTransition`.
///
/// `signature` / `signature_public_key_id` are exposed through
/// [`StateTransitionSingleSigned`](crate::state_transition::StateTransitionSingleSigned) and
/// [`StateTransitionIdentitySigned`](crate::state_transition::StateTransitionIdentitySigned),
/// and `user_fee_increase` through
/// [`StateTransitionHasUserFeeIncrease`](crate::state_transition::StateTransitionHasUserFeeIncrease),
/// so they are intentionally not duplicated here.
pub trait ShieldFromIdentityTransitionAccessorsV0 {
    /// The identity whose balance funds the shield.
    fn identity_id(&self) -> Identifier;
    /// Set the funding identity.
    fn set_identity_id(&mut self, identity_id: Identifier);

    /// Credits leaving the identity balance and entering the shielded pool.
    fn amount(&self) -> u64;
    /// Set the shielded amount.
    fn set_amount(&mut self, amount: u64);

    /// Get the serialized Orchard actions (spend/output pairs).
    fn actions(&self) -> &[SerializedAction];
    /// Replace the serialized Orchard actions.
    fn set_actions(&mut self, actions: Vec<SerializedAction>);

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

    /// The identity nonce (replay protection).
    fn nonce(&self) -> IdentityNonce;
    /// Set the identity nonce.
    fn set_nonce(&mut self, nonce: IdentityNonce);

    /// Extract nullifier bytes from each action.
    fn nullifiers<T: From<[u8; 32]>>(&self) -> Vec<T> {
        self.actions()
            .iter()
            .map(|a| T::from(a.nullifier))
            .collect()
    }
}
