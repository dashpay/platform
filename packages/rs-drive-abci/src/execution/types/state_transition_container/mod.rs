use crate::execution::types::state_transition_container::v0::{
    DecodedStateTransition, StateTransitionContainerV0,
};
use derive_more::From;

pub(crate) mod v0;

#[derive(Debug, From)]
pub enum StateTransitionContainer<'a> {
    V0(StateTransitionContainerV0<'a>),
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

#[allow(clippy::from_over_into)]
impl<'a> Into<Vec<DecodedStateTransition<'a>>> for StateTransitionContainer<'a> {
    fn into(self) -> Vec<DecodedStateTransition<'a>> {
        match self {
            StateTransitionContainer::V0(v0) => v0.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution::types::state_transition_container::v0::InvalidWithProtocolErrorStateTransition;
    use dpp::ProtocolError;
    use std::time::Duration;

    fn make_test_st<'a>(raw: &'a [u8]) -> DecodedStateTransition<'a> {
        DecodedStateTransition::FailedToDecode(InvalidWithProtocolErrorStateTransition {
            raw,
            error: ProtocolError::Generic("test".to_string()),
            elapsed_time: Duration::from_millis(1),
        })
    }

    #[test]
    fn wrapper_ref_iterator() {
        let raw = b"data";
        let v0 = StateTransitionContainerV0::new(vec![make_test_st(raw)]);
        let container: StateTransitionContainer = v0.into();
        assert_eq!((&container).into_iter().count(), 1);
    }

    #[test]
    fn wrapper_owned_iterator() {
        let raw = b"data";
        let v0 = StateTransitionContainerV0::new(vec![make_test_st(raw)]);
        let container: StateTransitionContainer = v0.into();
        assert_eq!(container.into_iter().count(), 1);
    }

    #[test]
    fn wrapper_into_vec() {
        let raw = b"data";
        let v0 = StateTransitionContainerV0::new(vec![make_test_st(raw), make_test_st(raw)]);
        let container: StateTransitionContainer = v0.into();
        let vec: Vec<DecodedStateTransition> = container.into();
        assert_eq!(vec.len(), 2);
    }

    #[test]
    fn from_v0_conversion() {
        let raw = b"data";
        let v0 = StateTransitionContainerV0::new(vec![make_test_st(raw)]);
        let container: StateTransitionContainer = v0.into();
        assert!(matches!(container, StateTransitionContainer::V0(_)));
    }
}
