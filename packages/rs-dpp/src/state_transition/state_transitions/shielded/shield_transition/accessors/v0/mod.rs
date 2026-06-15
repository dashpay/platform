use crate::address_funds::AddressFundsFeeStrategy;
use crate::shielded::SerializedAction;

/// Accessors for the fields of a `ShieldTransition`.
///
/// `inputs` / `input_witnesses` are exposed through
/// [`StateTransitionWitnessSigned`](crate::state_transition::StateTransitionWitnessSigned) and
/// `user_fee_increase` through
/// [`StateTransitionHasUserFeeIncrease`](crate::state_transition::StateTransitionHasUserFeeIncrease),
/// so they are intentionally not duplicated here.
pub trait ShieldTransitionAccessorsV0 {
    /// Get the serialized Orchard actions (spend/output pairs).
    fn actions(&self) -> &[SerializedAction];
    /// Replace the serialized Orchard actions.
    fn set_actions(&mut self, actions: Vec<SerializedAction>);

    /// Get the amount of credits being shielded (entering the shielded pool).
    fn amount(&self) -> u64;
    /// Set the amount of credits being shielded.
    fn set_amount(&mut self, amount: u64);

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

    /// Get the fee payment strategy.
    fn fee_strategy(&self) -> &AddressFundsFeeStrategy;
    /// Set the fee payment strategy.
    fn set_fee_strategy(&mut self, fee_strategy: AddressFundsFeeStrategy);

    /// Extract nullifier bytes from each action.
    /// Generic over the element type: use `Vec<u8>` or `[u8; 32]` as needed.
    fn nullifiers<T: From<[u8; 32]>>(&self) -> Vec<T> {
        self.actions()
            .iter()
            .map(|a| T::from(a.nullifier))
            .collect()
    }
}
