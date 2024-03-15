pub(crate) mod v0;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::execution_event::ExecutionEvent;
use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::StateTransition;

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
pub(in crate::execution) fn process_state_transition<'a, C: CoreRPCLike>(
    platform: &'a PlatformRef<C>,
    block_info: &BlockInfo,
    state_transition: StateTransition,
    transaction: TransactionArg,
) -> Result<ConsensusValidationResult<ExecutionEvent<'a>>, Error> {
    let platform_version = platform.state.current_platform_version()?;
    match platform_version
        .drive_abci
        .validation_and_processing
        .process_state_transition
    {
        0 => v0::process_state_transition_v0(
            platform,
            block_info,
            state_transition,
            transaction,
            platform_version,
        ),
        version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
            method: "process_state_transition".to_string(),
            known_versions: vec![0],
            received: version,
        })),
    }
}
