use platform_value::BinaryData;

pub trait StateTransitionMultiSigned: Sized {
    /// returns the signatures as an array of byte-arrays
    fn signatures(&self) -> &Vec<BinaryData>;
    /// set a new signature
    fn set_signatures(&mut self, signatures: Vec<BinaryData>);
}
