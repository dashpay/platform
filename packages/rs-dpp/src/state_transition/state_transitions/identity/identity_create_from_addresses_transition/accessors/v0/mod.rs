use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;

pub trait IdentityCreateFromAddressesTransitionAccessorsV0 {
    /// Get identity public keys
    fn public_keys(&self) -> &[IdentityPublicKeyInCreation];

    /// Get identity public keys as a mutable vec
    fn public_keys_mut(&mut self) -> &mut Vec<IdentityPublicKeyInCreation>;

    /// Replaces existing set of public keys with a new one
    fn set_public_keys(&mut self, public_keys: Vec<IdentityPublicKeyInCreation>);
    /// Adds public keys to the existing public keys array
    fn add_public_keys(&mut self, public_keys: &mut Vec<IdentityPublicKeyInCreation>);
}
