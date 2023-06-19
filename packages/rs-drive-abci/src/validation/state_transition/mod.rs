mod common;
mod document_state_validation;
mod key_validation;
pub(crate) mod processor;
mod state_transitions;
/// Transforming a state transition into a state transition action
pub mod transformer;

use dpp::identity::PartialIdentity;
use dpp::state_transition::{StateTransition, StateTransitionAction};
use dpp::validation::{ConsensusValidationResult, SimpleConsensusValidationResult};
use drive::drive::Drive;
use drive::query::TransactionArg;

pub use state_transitions::*;

use crate::error::Error;
use crate::execution::execution_event::ExecutionEvent;
use crate::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;
