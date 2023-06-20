mod common;
pub(crate) mod processor;
mod state_transitions;
/// Transforming a state transition into a state transition action
pub mod transformer;

pub use state_transitions::*;
