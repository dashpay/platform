use crate::shielded::SerializedAction;
use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use platform_value::Identifier;

pub trait IdentityCreateFromShieldedPoolTransitionAccessorsV0 {
    /// Get the serialized Orchard actions (spend/output pairs).
    fn actions(&self) -> &[SerializedAction];

    /// Get the public keys of the new identity.
    fn public_keys(&self) -> &[IdentityPublicKeyInCreation];

    /// Get the fixed exit denomination (in credits).
    fn denomination(&self) -> u64;

    /// Get the id of the new identity (derived from the spend nullifiers).
    fn identity_id(&self) -> Identifier;

    /// Extract nullifier bytes from each action.
    /// Generic over the element type: use `Vec<u8>` or `[u8; 32]` as needed.
    fn nullifiers<T: From<[u8; 32]>>(&self) -> Vec<T> {
        self.actions()
            .iter()
            .map(|a| T::from(a.nullifier))
            .collect()
    }
}
