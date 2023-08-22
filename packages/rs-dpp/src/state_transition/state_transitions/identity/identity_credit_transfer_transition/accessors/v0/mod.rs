use platform_value::Identifier;

pub trait IdentityCreditTransferTransitionAccessorsV0 {
    fn set_amount(&mut self, amount: u64);
    fn amount(&self) -> u64;
    fn identity_id(&self) -> Identifier;
    fn set_identity_id(&mut self, identity_id: Identifier);
    fn recipient_id(&self) -> Identifier;
    fn set_recipient_id(&mut self, recipient_id: Identifier);
}
