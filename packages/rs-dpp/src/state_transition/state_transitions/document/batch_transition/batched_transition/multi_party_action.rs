use platform_value::Identifier;

pub trait AllowedAsMultiPartyAction {
    fn calculate_action_id(&self, owner_id: Identifier) -> Identifier;
}
