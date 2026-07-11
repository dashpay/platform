use crate::identity::core_script::CoreScript;
use crate::shielded::SerializedAction;
use crate::withdrawal::Pooling;

pub trait ShieldedWithdrawalTransitionAccessorsV0 {
    /// Get the serialized Orchard actions.
    fn actions(&self) -> &[SerializedAction];
    /// Replace the serialized Orchard actions.
    fn set_actions(&mut self, actions: Vec<SerializedAction>);

    /// Get the total credits leaving the shielded pool (recipient amount + fee).
    fn unshielding_amount(&self) -> u64;
    /// Set the total credits leaving the shielded pool.
    fn set_unshielding_amount(&mut self, unshielding_amount: u64);

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

    /// Get the core transaction fee rate.
    fn core_fee_per_byte(&self) -> u32;
    /// Set the core transaction fee rate.
    fn set_core_fee_per_byte(&mut self, core_fee_per_byte: u32);

    /// Get the withdrawal pooling strategy.
    fn pooling(&self) -> Pooling;
    /// Set the withdrawal pooling strategy.
    fn set_pooling(&mut self, pooling: Pooling);

    /// Get the output script receiving withdrawn funds.
    fn output_script(&self) -> &CoreScript;
    /// Set the output script receiving withdrawn funds.
    fn set_output_script(&mut self, output_script: CoreScript);

    /// Extract nullifier bytes from each action.
    /// Generic over the element type: use `Vec<u8>` or `[u8; 32]` as needed.
    fn nullifiers<T: From<[u8; 32]>>(&self) -> Vec<T> {
        self.actions()
            .iter()
            .map(|a| T::from(a.nullifier))
            .collect()
    }
}
