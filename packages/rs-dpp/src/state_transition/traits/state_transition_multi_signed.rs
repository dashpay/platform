use crate::address_funds::AddressWitness;

pub trait StateTransitionWitnessSigned: Sized {
    /// returns the signatures as an array of byte-arrays
    fn witnesses(&self) -> &Vec<AddressWitness>;
    /// set a new signature
    fn set_witnesses(&mut self, witnesses: Vec<AddressWitness>);
}
