mod common;
mod document_state_validation;
mod key_validation;
pub(crate) mod processor;
mod state_transitions;
/// Transforming a state transition into a state transition action
pub mod transformer;

pub use state_transitions::*;
