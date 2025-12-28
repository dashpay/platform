use platform_value::BinaryData;

pub trait StateTransitionSingleSigned: Sized {
    /// returns the signature as a byte-array
    fn signature(&self) -> &BinaryData;
    /// set a new signature
    fn set_signature(&mut self, signature: BinaryData);
    /// sets the signature bytes
    fn set_signature_bytes(&mut self, signature: Vec<u8>);
}
