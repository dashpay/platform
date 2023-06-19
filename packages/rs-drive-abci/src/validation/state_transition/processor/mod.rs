pub(crate) mod v0;

use crate::error::Error;
use crate::execution::execution_event::ExecutionEvent;
use crate::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;
use dpp::identity::PartialIdentity;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::{StateTransition, StateTransitionAction};
use dpp::validation::SimpleConsensusValidationResult;
use drive::drive::Drive;
use drive::grovedb::TransactionArg;

/// There are 3 stages in a state transition processing:
/// Structure, Signature and State validation,
///
/// The structure validation verifies that the form of the state transition is good, for example
/// that a contract is well formed, or that a document is valid against the contract.
///
/// Signature validation verifies signatures of a state transition, it will also verify
/// signatures of keys for identity create and identity update. At this stage we will get back
/// a partial identity.
///
/// Validate state verifies that there are no state based conflicts, for example that a document
/// with a unique index isn't already taken.
///
pub fn process_state_transition<'a, C: CoreRPCLike>(
    platform: &'a PlatformRef<C>,
    state_transition: StateTransition,
    transaction: TransactionArg,
) -> Result<ConsensusValidationResult<ExecutionEvent<'a>>, Error> {
    //Todo: feature type (next versioning pr)
    // We will need to check the protocol version and use feature type to determine the version of
    // the processing.
    v0::process_state_transition_v0(platform, state_transition, transaction)
}
