use crate::execution::types::state_transition_container::v0::{
    DecodedStateTransition, StateTransitionContainerGettersV0, StateTransitionContainerV0,
};
use derive_more::From;

pub(crate) mod v0;

#[derive(Debug, From)]
pub enum StateTransitionContainer<'a> {
    V0(StateTransitionContainerV0<'a>),
}
impl<'a> StateTransitionContainerGettersV0<'a> for StateTransitionContainer<'a> {
    // The destructure method's signature and return type need to be adjusted to match the trait.
    fn into_vec(self) -> Vec<DecodedStateTransition<'a>> {
        match self {
            StateTransitionContainer::V0(v0) => v0.into_vec(),
        }
    }
}

impl<'a> IntoIterator for &'a StateTransitionContainer<'a> {
    type Item = &'a DecodedStateTransition<'a>;
    type IntoIter = std::slice::Iter<'a, DecodedStateTransition<'a>>;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            StateTransitionContainer::V0(v0) => v0.into_iter(),
        }
    }
}

impl<'a> IntoIterator for StateTransitionContainer<'a> {
    type Item = DecodedStateTransition<'a>;
    type IntoIter = std::vec::IntoIter<DecodedStateTransition<'a>>;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            StateTransitionContainer::V0(v0) => v0.into_iter(),
        }
    }
}
