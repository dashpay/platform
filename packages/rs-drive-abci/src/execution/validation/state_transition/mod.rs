pub(crate) mod check_tx_verification;
mod common;
pub(crate) mod processor;
mod state_transitions;
/// Transforming a state transition into a state transition action
pub mod transformer;

pub use state_transitions::*;

#[cfg(test)]
pub(in crate::execution) use state_transitions::tests;
