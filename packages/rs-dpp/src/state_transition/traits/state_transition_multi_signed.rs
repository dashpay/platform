use std::collections::BTreeMap;

use crate::address_funds::{AddressWitness, PlatformAddress};
use crate::fee::Credits;
use crate::prelude::AddressNonce;

pub trait StateTransitionWitnessSigned: Sized {
    /// Get inputs
    fn inputs(&self) -> &BTreeMap<PlatformAddress, (AddressNonce, Credits)>;
    /// Get inputs as a mutable map
    fn inputs_mut(&mut self) -> &mut BTreeMap<PlatformAddress, (AddressNonce, Credits)>;
    /// Set inputs
    fn set_inputs(&mut self, inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>);

    /// returns the witnesses as an array
    fn witnesses(&self) -> &Vec<AddressWitness>;
    /// set new witnesses
    fn set_witnesses(&mut self, witnesses: Vec<AddressWitness>);
}
