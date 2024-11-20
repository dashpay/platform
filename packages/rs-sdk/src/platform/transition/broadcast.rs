use super::broadcast_request::BroadcastRequestForStateTransition;
use super::put_settings::PutSettings;
use crate::platform::block_info_from_metadata::block_info_from_metadata;
use crate::sync::retry;
use crate::{Error, Sdk};
use dapi_grpc::platform::v0::{Proof, WaitForStateTransitionResultResponse};
use dapi_grpc::platform::VersionedGrpcResponse;
use dpp::state_transition::proof_result::StateTransitionProofResult;
use dpp::state_transition::StateTransition;
use drive::drive::Drive;
use drive_proof_verifier::error::ContextProviderError;
use drive_proof_verifier::DataContractProvider;
use rs_dapi_client::WrapWithExecutionResult;
use rs_dapi_client::{DapiRequest, ExecutionError, InnerInto, IntoInner, RequestSettings};

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
        let retry_settings = match settings {
            Some(s) => sdk.dapi_client_settings.override_by(s.request_settings),
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
    async fn wait_for_response<T: TryFrom<StateTransitionProofResult>>(
        &self,
        sdk: &Sdk,
        settings: Option<PutSettings>,
    ) -> Result<T, Error> {
        let retry_settings = match settings {
            Some(s) => sdk.dapi_client_settings.override_by(s.request_settings),
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

            let response = request.execute(sdk, request_settings).await.inner_into()?;

            let grpc_response: &WaitForStateTransitionResultResponse = &response.inner;
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

            let variant_name = result.to_string();
            T::try_from(result)
                .map_err(|_| {
                    Error::InvalidProvedResponse(format!(
                        "invalid proved response: cannot convert from {} to {}",
                        variant_name,
                        std::any::type_name::<T>(),
                    ))
                })
                .wrap(&response)
        };

        let future = retry(retry_settings, factory);
        let wait_timeout = settings.and_then(|s| s.wait_timeout);
        match wait_timeout {
            Some(timeout) => tokio::time::timeout(timeout, future)
                .await
                .map_err(|e| {
                    Error::TimeoutReached(
                        timeout,
                        format!("Timeout waiting for result of {} (tx id: {}) affecting object {}: {:?}",
                        self.name(),
                        self.transaction_id().map(hex::encode).unwrap_or("UNKNOWN".to_string()),
                        self.unique_identifiers().join(","),
                         e),
                    )
                })?
                .into_inner(),
            None => future.await.into_inner(),
        }
    }

    async fn broadcast_and_wait<T: TryFrom<StateTransitionProofResult>>(
        &self,
        sdk: &Sdk,
        settings: Option<PutSettings>,
    ) -> Result<T, Error> {
        self.broadcast(sdk, settings).await?;
        self.wait_for_response::<T>(sdk, settings).await
    }
}
