use super::broadcast_request::BroadcastRequestForStateTransition;
use crate::platform::block_info_from_metadata::block_info_from_metadata;
use crate::sync::retry;
use crate::{Error, Sdk};
use dapi_grpc::platform::v0::Proof;
use dapi_grpc::platform::VersionedGrpcResponse;
use dpp::state_transition::proof_result::StateTransitionProofResult;
use dpp::state_transition::StateTransition;
use drive::drive::Drive;
use drive_proof_verifier::error::ContextProviderError;
use drive_proof_verifier::DataContractProvider;
use futures::TryFutureExt;
use rs_dapi_client::WrapWithExecutionResult;
use rs_dapi_client::{DapiRequest, ExecutionError, InnerInto, IntoInner, RequestSettings};
use tokio::time::timeout;

#[async_trait::async_trait]
pub trait BroadcastStateTransition {
    async fn broadcast(&self, sdk: &Sdk, settings: Option<RequestSettings>) -> Result<(), Error>;
    async fn wait_for_response(
        &self,
        sdk: &Sdk,
        settings: Option<RequestSettings>,
        time_out_ms: Option<u64>,
    ) -> Result<StateTransitionProofResult, Error>;
    async fn broadcast_and_wait(
        &self,
        sdk: &Sdk,
        settings: Option<RequestSettings>,
        time_out_ms: Option<u64>,
    ) -> Result<StateTransitionProofResult, Error>;
}

#[async_trait::async_trait]
impl BroadcastStateTransition for StateTransition {
    async fn broadcast(&self, sdk: &Sdk, settings: Option<RequestSettings>) -> Result<(), Error> {
        let retry_settings = match settings {
            Some(s) => sdk.dapi_client_settings.override_by(s),
            None => sdk.dapi_client_settings,
        };

        // async fn retry_test_function(settings: RequestSettings) -> ExecutionResult<(), dash_sdk::Error>
        let factory = |request_settings: RequestSettings| async move {
            let request =
                self.broadcast_request_for_state_transition()
                    .map_err(|e| ExecutionError {
                        inner: e,
                        address: None,
                        retries: 0,
                    })?;
            request
                .execute(sdk, request_settings)
                .await
                .map_err(|e| e.inner_into())
        };

        // response is empty for a broadcast, result comes from the stream wait for state transition result
        retry(retry_settings, factory)
            .await
            .into_inner()
            .map(|_| ())
    }
    async fn wait_for_response(
        &self,
        sdk: &Sdk,
        settings: Option<RequestSettings>,
        time_out_ms: Option<u64>,
    ) -> Result<StateTransitionProofResult, Error> {
        let retry_settings = match settings {
            Some(s) => sdk.dapi_client_settings.override_by(s),
            None => sdk.dapi_client_settings,
        };

        let factory = |request_settings: RequestSettings| async move {
            let request = self
                .wait_for_state_transition_result_request()
                .map_err(|e| ExecutionError {
                    inner: e,
                    address: None,
                    retries: 0,
                })?;

            let response = request
                .execute(sdk, request_settings)
                .await
                .map_err(|e| e.inner_into())?;

            let grpc_response = &response.inner;
            let metadata = grpc_response.metadata().wrap(&response)?.inner;
            let block_info = block_info_from_metadata(metadata).wrap(&response)?.inner;
            let proof: &Proof = (*grpc_response).proof().wrap(&response)?.inner;

            let context_provider = sdk.context_provider().ok_or(ExecutionError {
                inner: Error::from(ContextProviderError::Config(
                    "Context provider not initialized".to_string(),
                )),
                address: Some(response.address.clone()),
                retries: response.retries,
            })?;

            let (_, result) = Drive::verify_state_transition_was_executed_with_proof(
                self,
                &block_info,
                proof.grovedb_proof.as_slice(),
                &context_provider.as_contract_lookup_fn(),
                sdk.version(),
            )
            .wrap(&response)?
            .inner;

            Ok::<_, Error>(result).wrap(&response)
        };

        let future = retry(retry_settings, factory);
        match time_out_ms {
            Some(time_out_ms) => {
                let timeout = tokio::time::Duration::from_millis(time_out_ms);
                tokio::time::timeout(timeout, future)
                    .await
                    .map_err(|e| {
                        Error::TimeoutReached(
                            timeout,
                            format!("Timeout waiting for state transition result: {:?}", e),
                        )
                    })?
                    .into_inner()
            }
            None => future.await.into_inner(),
        }
    }

    async fn broadcast_and_wait(
        &self,
        sdk: &Sdk,
        settings: Option<RequestSettings>,
        time_out_ms: Option<u64>,
    ) -> Result<StateTransitionProofResult, Error> {
        let future = async {
            self.broadcast(sdk, settings).await?;
            self.wait_for_response(sdk, settings, time_out_ms).await
        };

        match time_out_ms {
            Some(time_out_ms) => timeout(tokio::time::Duration::from_millis(time_out_ms), future)
                .into_future()
                .await
                .map_err(|e| {
                    Error::TimeoutReached(
                        tokio::time::Duration::from_millis(time_out_ms),
                        format!("Timeout waiting for state transition result: {:?}", e),
                    )
                })?,
            None => future.await,
        }
    }
}
