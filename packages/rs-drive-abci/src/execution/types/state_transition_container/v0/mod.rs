use dpp::consensus::ConsensusError;
use dpp::state_transition::StateTransition;
use dpp::ProtocolError;
use std::time::Duration;

/// Decoded state transition result
#[derive(Debug)]
pub enum DecodedStateTransition<'a> {
    SuccessfullyDecoded(SuccessfullyDecodedStateTransition<'a>),
    InvalidEncoding(InvalidStateTransition<'a>),
    FailedToDecode(InvalidWithProtocolErrorStateTransition<'a>),
}

/// Invalid encoded state transition
#[derive(Debug)]
pub struct InvalidStateTransition<'a> {
    pub raw: &'a [u8],
    pub error: ConsensusError,
    pub elapsed_time: Duration,
}

/// State transition that failed to decode
#[derive(Debug)]
pub struct InvalidWithProtocolErrorStateTransition<'a> {
    pub raw: &'a [u8],
    pub error: ProtocolError,
    pub elapsed_time: Duration,
}

/// Successfully decoded state transition
#[derive(Debug)]
pub struct SuccessfullyDecodedStateTransition<'a> {
    pub decoded: StateTransition,
    pub raw: &'a [u8],
    pub elapsed_time: Duration,
}

/// This is a container that holds state transitions
#[derive(Debug)]
pub struct StateTransitionContainerV0<'a> {
    // We collect all decoding results in the same vector because we want to
    // keep the original input order when we process them and log results we can
    // easily match with txs in block
    state_transitions: Vec<DecodedStateTransition<'a>>,
}

impl<'a> StateTransitionContainerV0<'a> {
    pub fn new(state_transitions: Vec<DecodedStateTransition<'a>>) -> Self {
        Self { state_transitions }
    }
}

impl<'a> IntoIterator for &'a StateTransitionContainerV0<'a> {
    type Item = &'a DecodedStateTransition<'a>;
    type IntoIter = std::slice::Iter<'a, DecodedStateTransition<'a>>;

    fn into_iter(self) -> Self::IntoIter {
        self.state_transitions.iter()
    }
}

impl<'a> IntoIterator for StateTransitionContainerV0<'a> {
    type Item = DecodedStateTransition<'a>;
    type IntoIter = std::vec::IntoIter<DecodedStateTransition<'a>>;

    fn into_iter(self) -> Self::IntoIter {
        self.state_transitions.into_iter()
    }
}

#[allow(clippy::from_over_into)]
impl<'a> Into<Vec<DecodedStateTransition<'a>>> for StateTransitionContainerV0<'a> {
    fn into(self) -> Vec<DecodedStateTransition<'a>> {
        self.state_transitions
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_st<'a>(raw: &'a [u8]) -> DecodedStateTransition<'a> {
        // Use FailedToDecode variant since ProtocolError is easy to construct
        DecodedStateTransition::FailedToDecode(InvalidWithProtocolErrorStateTransition {
            raw,
            error: ProtocolError::Generic("test".to_string()),
            elapsed_time: Duration::from_millis(10),
        })
    }

    #[test]
    fn new_container_stores_transitions() {
        let raw = b"test_data";
        let transitions = vec![make_test_st(raw)];
        let container = StateTransitionContainerV0::new(transitions);
        let items: Vec<_> = container.into_iter().collect();
        assert_eq!(items.len(), 1);
    }

    #[test]
    fn ref_iterator_yields_references() {
        let raw1 = b"data1";
        let raw2 = b"data2";
        let transitions = vec![make_test_st(raw1), make_test_st(raw2)];
        let container = StateTransitionContainerV0::new(transitions);
        let items: Vec<_> = (&container).into_iter().collect();
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn owned_iterator_consumes_container() {
        let raw = b"data";
        let transitions = vec![make_test_st(raw)];
        let container = StateTransitionContainerV0::new(transitions);
        let items: Vec<_> = container.into_iter().collect();
        assert_eq!(items.len(), 1);
    }

    #[test]
    fn into_vec_conversion() {
        let raw = b"data";
        let transitions = vec![make_test_st(raw), make_test_st(raw)];
        let container = StateTransitionContainerV0::new(transitions);
        let vec: Vec<DecodedStateTransition> = container.into();
        assert_eq!(vec.len(), 2);
    }

    #[test]
    fn empty_container_iterates_zero_times() {
        let container = StateTransitionContainerV0::new(vec![]);
        assert_eq!((&container).into_iter().count(), 0);
        assert_eq!(container.into_iter().count(), 0);
    }

    fn make_invalid_encoding_st<'a>(raw: &'a [u8]) -> DecodedStateTransition<'a> {
        use dpp::consensus::basic::unsupported_version_error::UnsupportedVersionError;
        use dpp::consensus::basic::BasicError;
        DecodedStateTransition::InvalidEncoding(InvalidStateTransition {
            raw,
            error: ConsensusError::BasicError(BasicError::UnsupportedVersionError(
                UnsupportedVersionError::new(99, 0, 1),
            )),
            elapsed_time: Duration::from_millis(5),
        })
    }

    #[test]
    fn invalid_encoding_variant() {
        let raw = b"bad_data";
        let st = make_invalid_encoding_st(raw);
        let container = StateTransitionContainerV0::new(vec![st]);
        let items: Vec<_> = container.into_iter().collect();
        assert_eq!(items.len(), 1);
        assert!(matches!(
            items[0],
            DecodedStateTransition::InvalidEncoding(_)
        ));
    }

    #[test]
    fn mixed_variants_in_container() {
        let raw1 = b"good";
        let raw2 = b"bad";
        let st1 = make_test_st(raw1);
        let st2 = make_invalid_encoding_st(raw2);
        let container = StateTransitionContainerV0::new(vec![st1, st2]);
        let items: Vec<_> = container.into_iter().collect();
        assert_eq!(items.len(), 2);
        assert!(matches!(
            items[0],
            DecodedStateTransition::FailedToDecode(_)
        ));
        assert!(matches!(
            items[1],
            DecodedStateTransition::InvalidEncoding(_)
        ));
    }
}
