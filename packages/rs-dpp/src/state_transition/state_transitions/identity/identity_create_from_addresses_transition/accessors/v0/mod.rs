use std::collections::BTreeMap;

use crate::fee::Credits;
use crate::identity::KeyOfType;
use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use platform_value::Identifier;

pub trait IdentityCreateFromAddressesTransitionAccessorsV0 {
    /// Get identity public keys
    fn public_keys(&self) -> &[IdentityPublicKeyInCreation];

    /// Get identity public keys as a mutable vec
    fn public_keys_mut(&mut self) -> &mut Vec<IdentityPublicKeyInCreation>;

    /// Replaces existing set of public keys with a new one
    fn set_public_keys(&mut self, public_keys: Vec<IdentityPublicKeyInCreation>);
    /// Adds public keys to the existing public keys array
    fn add_public_keys(&mut self, public_keys: &mut Vec<IdentityPublicKeyInCreation>);
    /// Returns identity id
    fn identity_id(&self) -> Identifier;

    /// Get inputs
    fn inputs(&self) -> &[KeyOfType];
    /// Get inputs as a mutable vec
    fn inputs_mut(&mut self) -> &mut Vec<KeyOfType>;
    /// Set inputs
    fn set_inputs(&mut self, inputs: Vec<KeyOfType>);

    /// Get outputs
    fn outputs(&self) -> &BTreeMap<KeyOfType, Credits>;
    /// Get outputs as a mutable map
    fn outputs_mut(&mut self) -> &mut BTreeMap<KeyOfType, Credits>;
    /// Set outputs
    fn set_outputs(&mut self, outputs: BTreeMap<KeyOfType, Credits>);
}
