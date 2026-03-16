use platform_value::Identifier;
use platform_version::version::PlatformVersion;
use crate::ProtocolError;

pub trait AllowedAsMultiPartyAction {
    fn calculate_action_id(&self, owner_id: Identifier, platform_version: &PlatformVersion) -> Result<Identifier, ProtocolError>;
}
