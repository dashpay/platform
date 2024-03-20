use crate::error::Error;

pub(crate) struct StateTransitionAwareErrorV0<'a> {
    pub(crate) error: Error,
    pub(crate) raw_state_transition: &'a Vec<u8>,
}
