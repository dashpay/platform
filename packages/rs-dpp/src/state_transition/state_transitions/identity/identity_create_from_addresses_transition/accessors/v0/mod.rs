use crate::address_funds::{AddressFundsFeeStrategy, PlatformAddress};
use crate::fee::Credits;
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

    /// Get the optional output (address, credits)
    fn output(&self) -> Option<&(PlatformAddress, Credits)>;

    /// Set the optional output
    fn set_output(&mut self, output: Option<(PlatformAddress, Credits)>);

    /// Get fee strategy
    fn fee_strategy(&self) -> &AddressFundsFeeStrategy;

    /// Set fee strategy
    fn set_fee_strategy(&mut self, fee_strategy: AddressFundsFeeStrategy);
}
