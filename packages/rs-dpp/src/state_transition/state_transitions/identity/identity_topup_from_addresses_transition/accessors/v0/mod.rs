use crate::address_funds::PlatformAddress;
use crate::fee::Credits;
use platform_value::Identifier;

pub trait IdentityTopUpFromAddressesTransitionAccessorsV0 {
    /// Set identity id
    fn set_identity_id(&mut self, identity_id: Identifier);

    /// Returns identity id
    fn identity_id(&self) -> &Identifier;

    /// Get the optional output (address, credits)
    fn output(&self) -> Option<&(PlatformAddress, Credits)>;

    /// Set the optional output
    fn set_output(&mut self, output: Option<(PlatformAddress, Credits)>);
}
