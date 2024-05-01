use platform_value::Identifier;

pub trait IdentityTopUpTransitionAccessorsV0 {
    /// Set identity id
    fn set_identity_id(&mut self, identity_id: Identifier);

    /// Returns identity id
    fn identity_id(&self) -> &Identifier;
}
