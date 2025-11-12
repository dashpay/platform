use super::broadcast_request::BroadcastRequestForStateTransition;
use super::put_settings::PutSettings;
use crate::error::StateTransitionBroadcastError;
use crate::platform::block_info_from_metadata::block_info_from_metadata;
use crate::sync::retry;
use crate::{Error, Sdk};
use dapi_grpc::platform::v0::wait_for_state_transition_result_response::wait_for_state_transition_result_response_v0;
use dapi_grpc::platform::v0::{
    wait_for_state_transition_result_response, Proof, WaitForStateTransitionResultResponse,
};
use dapi_grpc::platform::VersionedGrpcResponse;
use dash_context_provider::ContextProviderError;
use dpp::state_transition::proof_result::StateTransitionProofResult;
use dpp::state_transition::StateTransition;
use drive::drive::Drive;
use drive_proof_verifier::DataContractProvider;
use rs_dapi_client::{DapiRequest, ExecutionError, InnerInto, IntoInner, RequestSettings};
use rs_dapi_client::{ExecutionResponse, WrapToExecutionResult};
use tracing::{trace, warn};

#[async_trait::async_trait]
pub trait BroadcastStateTransition {
    async fn broadcast(&self, sdk: &Sdk, settings: Option<PutSettings>) -> Result<(), Error>;
    async fn wait_for_response<T: TryFrom<StateTransitionProofResult>>(
        &self,
        sdk: &Sdk,
        settings: Option<PutSettings>,
    ) -> Result<T, Error>;
    async fn broadcast_and_wait<T: TryFrom<StateTransitionProofResult>>(
        &self,
        sdk: &Sdk,
        settings: Option<PutSettings>,
    ) -> Result<T, Error>;
}

#[async_trait::async_trait]
impl BroadcastStateTransition for StateTransition {
    async fn broadcast(&self, sdk: &Sdk, settings: Option<PutSettings>) -> Result<(), Error> {
        trace!(
            state_transition = %self.name(),
            transaction_id = %self
                .transaction_id()
                .map(hex::encode)
                .unwrap_or("UNKNOWN".to_string()),
            "broadcast: start"
        );

        let retry_settings = match settings {
            Some(s) => sdk.dapi_client_settings.override_by(s.request_settings),
            None => sdk.dapi_client_settings,
        };

        // async fn retry_test_function(settings: RequestSettings) -> ExecutionResult<(), dash_sdk::Error>
        let factory = |request_settings: RequestSettings| async move {
            trace!("broadcast: creating request");
            let request =
                self.broadcast_request_for_state_transition()
                    .map_err(|e| ExecutionError {
                        inner: e,
                        address: None,
                        retries: 0,
                    })?;
            trace!("broadcast: executing request");
            let result = request
                .execute(sdk, request_settings)
                .await
                .map_err(|e| e.inner_into());

            match &result {
                Ok(_) => trace!("broadcast: request succeeded"),
                Err(e) => warn!(error = ?e, "broadcast: request failed"),
            }
            result
        };

        // response is empty for a broadcast, result comes from the stream wait for state transition result
        trace!("broadcast: starting retry mechanism");
        let result = retry(sdk.address_list(), retry_settings, factory)
            .await
            .into_inner()
            .map(|_| ());

        match &result {
            Ok(_) => trace!("broadcast: completed successfully"),
            Err(e) => warn!(error = ?e, "broadcast: failed after retries"),
        }
        result
    }
    async fn wait_for_response<T: TryFrom<StateTransitionProofResult>>(
        &self,
        sdk: &Sdk,
        settings: Option<PutSettings>,
    ) -> Result<T, Error> {
        trace!(
            transaction_id = %self
                .transaction_id()
                .map(hex::encode)
                .unwrap_or("UNKNOWN".to_string()),
            "wait: start"
        );

        let retry_settings = match settings {
            Some(s) => sdk.dapi_client_settings.override_by(s.request_settings),
            None => sdk.dapi_client_settings,
        };

        // prepare a factory that will generate closure which executes actual code
        let factory = |request_settings: RequestSettings| async move {
            trace!("wait: creating request");
            let request = self
                .wait_for_state_transition_result_request()
                .map_err(|e| ExecutionError {
                    inner: e,
                    address: None,
                    retries: 0,
                })?;

            trace!("wait: executing request");
            let response = request.execute(sdk, request_settings).await.inner_into()?;
            trace!("wait: received response");

            let grpc_response: &WaitForStateTransitionResultResponse = &response.inner;

            // We use match here to have a compilation error if a new version of the response is introduced
            let state_transition_broadcast_error = match &grpc_response.version {
                Some(wait_for_state_transition_result_response::Version::V0(result)) => {
                    match &result.result {
                        Some(wait_for_state_transition_result_response_v0::Result::Error(e)) => {
                            Some(e)
                        }
                        _ => None,
                    }
                }
                None => None,
            };

            if let Some(e) = state_transition_broadcast_error {
                warn!("wait: state transition broadcast error detected");
                let state_transition_broadcast_error: StateTransitionBroadcastError =
                    StateTransitionBroadcastError::try_from(e.clone())
                        .wrap_to_execution_result(&response)?
                        .inner;

                return Err(Error::from(state_transition_broadcast_error))
                    .wrap_to_execution_result(&response);
            }

            trace!("wait: extracting metadata");
            let metadata = grpc_response
                .metadata()
                .wrap_to_execution_result(&response)?
                .inner;
            let block_info = block_info_from_metadata(metadata)
                .wrap_to_execution_result(&response)?
                .inner;
            trace!(block_info = ?block_info, "wait: block info extracted");

            trace!("wait: extracting proof");
            let proof: &Proof = (*grpc_response)
                .proof()
                .wrap_to_execution_result(&response)?
                .inner;
            trace!(
                proof_size = proof.grovedb_proof.len(),
                "wait: proof extracted"
            );

            let context_provider = sdk.context_provider().ok_or(ExecutionError {
                inner: Error::from(ContextProviderError::Config(
                    "Context provider not initialized".to_string(),
                )),
                address: Some(response.address.clone()),
                retries: response.retries,
            })?;

            trace!("wait: verifying proof");
            let (_, result) = match Drive::verify_state_transition_was_executed_with_proof(
                self,
                &block_info,
                proof.grovedb_proof.as_slice(),
                &context_provider.as_contract_lookup_fn(sdk.version()),
                sdk.version(),
            ) {
                Ok(r) => Ok(ExecutionResponse {
                    inner: r,
                    retries: response.retries,
                    address: response.address.clone(),
                }),
                Err(drive::error::Error::Proof(proof_error)) => Err(ExecutionError {
                    inner: Error::DriveProofError(
                        proof_error,
                        proof.grovedb_proof.clone(),
                        block_info,
                    ),
                    retries: response.retries,
                    address: Some(response.address.clone()),
                }),
                Err(e) => Err(ExecutionError {
                    inner: e.into(),
                    retries: response.retries,
                    address: Some(response.address.clone()),
                }),
            }?
            .inner;

            trace!("wait: proof verification successful");
            trace!(result_variant = %result.to_string(), "wait: result variant");

            let variant_name = result.to_string();
            let conversion_result = T::try_from(result)
                .map_err(|_| {
                    Error::InvalidProvedResponse(format!(
                        "invalid proved response: cannot convert from {} to {}",
                        variant_name,
                        std::any::type_name::<T>(),
                    ))
                })
                .wrap_to_execution_result(&response);

            match &conversion_result {
                Ok(_) => trace!("wait: converted result to expected type"),
                Err(e) => warn!(error = ?e, "wait: failed to convert result"),
            }
            conversion_result
        };

        let future = retry(sdk.address_list(), retry_settings, factory);
        // run the future with or without timeout, depending on the settings
        let wait_timeout = settings.and_then(|s| s.wait_timeout);

        trace!(timeout = ?wait_timeout, "wait: starting retry mechanism");

        match wait_timeout {
            Some(timeout) => {
                trace!(?timeout, "wait: waiting with timeout");
                tokio::time::timeout(timeout, future)
                    .await
                    .map_err(|e| {
                        warn!(?timeout, "wait: timeout reached");
                        Error::TimeoutReached(
                            timeout,
                            format!("Timeout waiting for result of {} (tx id: {}) affecting object {}: {:?}",
                            self.name(),
                            self.transaction_id().map(hex::encode).unwrap_or("UNKNOWN".to_string()),
                            self.unique_identifiers().join(","),
                             e),
                        )
                    })?
                    .into_inner()
            }
            None => {
                trace!("wait: waiting without timeout");
                future.await.into_inner()
            }
        }
    }

    async fn broadcast_and_wait<T: TryFrom<StateTransitionProofResult>>(
        &self,
        sdk: &Sdk,
        settings: Option<PutSettings>,
    ) -> Result<T, Error> {
        trace!(state_transition = %self.name(), "broadcast_and_wait: start");
        trace!("broadcast_and_wait: step 1 - broadcasting");
        self.broadcast(sdk, settings).await?;
        trace!("broadcast_and_wait: step 2 - waiting for response");
        let result = self.wait_for_response::<T>(sdk, settings).await;
        match &result {
            Ok(_) => trace!("broadcast_and_wait: complete success"),
            Err(e) => warn!(error = ?e, "broadcast_and_wait: failed"),
        }
        result
    }
}
