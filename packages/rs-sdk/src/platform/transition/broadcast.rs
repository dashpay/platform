//! Broadcast state transition to the network
use super::broadcast_request::BroadcastRequestForStateTransition;
use crate::{Error, Sdk};
use dapi_grpc::platform::VersionedGrpcResponse;
use dpp::state_transition::proof_result::StateTransitionProofResult;
use dpp::state_transition::StateTransition;
use drive::drive::Drive;
use drive::error::contract::DataContractError;
use drive_proof_verifier::ContextProvider;
use rs_dapi_client::{DapiRequest, RequestSettings};

/// Broadcast state transition to the network.
///
/// This trait is implemented state transitions that can be broadcasted to the network.
#[async_trait::async_trait]
pub trait BroadcastStateTransition {
    /// Broadcast state transition to the network.
    ///
    /// Broadcast state transition to the network and return immediately.
    ///
    /// It is not guaranteed that the state transition was accepted and executed by the network.
    async fn broadcast(&self, sdk: &Sdk) -> Result<(), Error>;
    /// Broadcast state transition to the network and wait for the confirmation proof.
    ///
    /// Broadcast state transition to the network and wait for the confirmation proof.
    ///
    /// ## Arguments
    ///
    /// * `timeout` - Maximum wait time. Once the time is exceeded, the function will return an error, and
    /// the result of state transition execution is unspecified.
    ///
    /// // TODO: How to check result afterwards?
    ///
    /// ## Returns
    ///
    /// * `StateTransitionProofResult` - Result of the state transition execution, containing the state after transition
    /// was executed
    async fn broadcast_and_wait(
        &self,
        sdk: &Sdk,
        timeout: Option<std::time::Duration>,
    ) -> Result<StateTransitionProofResult, Error>;
}

#[async_trait::async_trait]
impl BroadcastStateTransition for StateTransition {
    async fn broadcast(&self, sdk: &Sdk) -> Result<(), Error> {
        let request = self.broadcast_request_for_state_transition()?;

        request.execute(sdk, RequestSettings::default()).await?;

        // response is empty for a broadcast, result comes from the stream wait for state transition result

        Ok(())
    }

    async fn broadcast_and_wait(
        &self,
        sdk: &Sdk,
        timeout: Option<std::time::Duration>,
    ) -> Result<StateTransitionProofResult, Error> {
        let request = self.broadcast_request_for_state_transition()?;

        request
            .clone()
            .execute(sdk, RequestSettings::default())
            .await?;

        let request = self.wait_for_state_transition_result_request()?;
        let settings = RequestSettings {
            timeout,
            ..Default::default()
        };
        let response = request.execute(sdk, settings).await?;

        let proof = response.proof_owned()?;

        let (_, result) = Drive::verify_state_transition_was_executed_with_proof(
            self,
            proof.grovedb_proof.as_slice(),
            &|id| {
                sdk.get_data_contract(id).map_err(|e| {
                    drive::error::Error::DataContract(DataContractError::MissingContract(
                        e.to_string(),
                    ))
                })
            },
            sdk.version(),
        )?;

        Ok(result)
    }
}
