mod v0;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::state_transition_container::StateTransitionContainer;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::version::PlatformVersion;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Processes the given raw state transitions based on the `block_info` and `transaction`.
    ///
    /// # Arguments
    ///
    /// * `raw_state_transitions` - A reference to a vector of raw state transitions.
    /// * `platform_version` - A `PlatformVersion` reference that dictates which version of
    ///   the method to call.
    ///
    /// # Returns
    ///
    /// * `Result<(FeeResult, Vec<ExecTxResult>), Error>` - If the processing is successful, it returns
    ///   a tuple consisting of a `FeeResult` and a vector of `ExecTxResult`. If the processing fails,
    ///   it returns an `Error`.
    ///
    /// # Errors
    ///
    /// This function may return an `Error` variant if there is a problem with deserializing the raw
    /// state transitions, processing state transitions, or executing events.
    pub(in crate::execution) fn decode_raw_state_transitions<'a>(
        &self,
        raw_state_transitions: &'a [impl AsRef<[u8]>],
        platform_version: &PlatformVersion,
    ) -> Result<StateTransitionContainer<'a>, Error> {
        match platform_version
            .drive_abci
            .methods
            .state_transition_processing
            .decode_raw_state_transitions
        {
            0 => Ok(self
                .decode_raw_state_transitions_v0(raw_state_transitions, platform_version)
                .into()),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "decode_raw_state_transitions".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
