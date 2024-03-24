use crate::error::Error;
use crate::execution::validation::state_transition::processor::process_state_transition;
use crate::platform_types::platform::{Platform, PlatformRef};
use crate::platform_types::platform_state::PlatformState;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::consensus::basic::decode::SerializedObjectParsingError;
use dpp::dashcore::hashes::Hash;
use dpp::fee::fee_result::FeeResult;
use dpp::identity::state_transition::OptionallyAssetLockProved;
use dpp::serialization::PlatformDeserializable;
use dpp::state_transition::StateTransition;
use dpp::{dashcore, ProtocolError};
use dpp::validation::{ConsensusValidationResult, SimpleConsensusValidationResult};

use crate::platform_types::event_execution_result::EventExecutionResult;
use crate::platform_types::state_transitions_processing_result::{
    StateTransitionExecutionResult, StateTransitionsProcessingResult,
};
use dpp::version::PlatformVersion;
use drive::grovedb::Transaction;
use crate::execution::types::state_transition_aware_error::v0::StateTransitionAwareErrorV0;
use crate::execution::types::state_transition_container::v0::StateTransitionContainerV0;


impl<C> Platform<C>
    where
        C: CoreRPCLike,
{
    /// Processes the given raw state transitions based on the `block_info` and `transaction`.
    ///
    /// This function takes a reference to a vector of raw state transitions, `BlockInfo`, and a `Transaction`
    /// as input and performs the corresponding state transition operations. It deserializes the raw state
    /// transitions into a `StateTransition` and processes them.
    ///
    /// # Arguments
    ///
    /// * `raw_state_transitions` - A reference to a vector of raw state transitions.
    /// * `block_info` - Information about the current block being processed.
    /// * `transaction` - The transaction associated with the raw state transitions.
    ///
    /// # Returns
    ///
    /// * `Result<StateTransitionsProcessingResult, Error>` - If the processing is successful, it returns
    ///   a `StateTransitionsProcessingResult` with state transition execution results and aggregated information.
    ///   If the processing fails, it returns an `Error`.
    ///
    /// # Errors
    ///
    /// This function may return an `Error` variant if there is a problem with deserializing the raw
    /// state transitions, processing state transitions, or executing events.
    ///
    pub(super) fn decode_raw_state_transitions_v0(
        &self,
        raw_state_transitions: &Vec<Vec<u8>>,
    ) -> Result<StateTransitionContainerV0, Error> {
        raw_state_transitions.into_iter().map(|raw_state_transition| {
            match StateTransition::deserialize_from_bytes(raw_state_transition) {
                Ok(state_transition) => Ok(ConsensusValidationResult::new_with_data(state_transition)),
                Err(error) => {
                    match error {
                        ProtocolError::PlatformDeserializationError(message) => {
                            let consensus_error =
                                SerializedObjectParsingError::new(message.clone()).into();
                            let errors = vec![&consensus_error];

                            tracing::debug!(
                            ?errors,
                            "Invalid unknown state transition ({}): {}",
                            st_hash,
                            message
                        );

                            Ok(ConsensusValidationResult::new_with_error(consensus_error))
                        }
                        ProtocolError::MaxEncodedBytesReachedError { .. } => {
                            let message = error.to_string();
                            let consensus_error =
                                SerializedObjectParsingError::new(message.clone()).into();
                            let errors = vec![&consensus_error];

                            tracing::debug!(
                            ?errors,
                            "State transition beyond max encoded bytes limit ({}): {}",
                            st_hash,
                            message
                        );

                            Ok(ConsensusValidationResult::new_with_error(consensus_error))
                        }
                        _ => Err(StateTransitionAwareErrorV0 {
                            error: error.into(),
                            raw_state_transition,
                        }),
                    }
                }
            }
        }).collect::<Result<StateTransitionContainerV0, Error>>()
    }
}