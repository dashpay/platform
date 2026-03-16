use platform_value::Identifier;
use platform_version::version::PlatformVersion;

pub trait AllowedAsMultiPartyAction {
    fn calculate_action_id(&self, owner_id: Identifier) -> Identifier;

    /// Version-aware action_id calculation. By default, delegates to the
    /// non-versioned [`calculate_action_id`](Self::calculate_action_id).
    /// Transition types that need version-dependent behaviour (e.g.
    /// `TokenConfigUpdateTransition`) override this method.
    fn calculate_action_id_versioned(
        &self,
        owner_id: Identifier,
        _platform_version: &PlatformVersion,
    ) -> Identifier {
        self.calculate_action_id(owner_id)
    }
}
