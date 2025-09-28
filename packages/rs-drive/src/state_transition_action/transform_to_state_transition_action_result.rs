use crate::drive::subscriptions::HitFiltersType;
use crate::state_transition_action::StateTransitionAction;

/// The result of transforming an input into a concrete
/// [`StateTransitionAction`], together with information
/// about whether any subscription filters were hit during the process.
///
/// This type is typically produced by higher-level logic that
/// validates and converts documents or proofs into actionable
/// state transitions.
#[derive(Clone, Debug)]
pub struct TransformToStateTransitionActionResult<'a> {
    /// The resulting state transition action derived
    /// from the input data.
    pub action: StateTransitionAction,

    /// Information about subscription filters that were evaluated
    /// while producing this action.
    ///
    /// Indicates whether no filters matched (`NoFilterHit`), or provides
    /// the original GroveDB proof along with the list of filters that
    /// matched (`DidHitFilters`).
    pub filters_hit: HitFiltersType<'a>,
}

impl From<StateTransitionAction> for TransformToStateTransitionActionResult<'_> {
    fn from(action: StateTransitionAction) -> Self {
        TransformToStateTransitionActionResult {
            action,
            filters_hit: HitFiltersType::NoFilterHit,
        }
    }
}
