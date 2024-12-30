use platform_value::Identifier;

pub trait AllowedAsMultiPartyAction {
    fn action_id(&self, owner_id: Identifier) -> Identifier;
}
