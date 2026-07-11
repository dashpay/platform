use super::broadcast_request::BroadcastRequestForStateTransition;
use super::put_settings::PutSettings;
use crate::error::StateTransitionBroadcastError;
use crate::sync::retry;
use crate::{Error, Sdk};
use dapi_grpc::platform::v0::wait_for_state_transition_result_response::wait_for_state_transition_result_response_v0;
use dapi_grpc::platform::v0::{
    wait_for_state_transition_result_response, BroadcastStateTransitionRequest, ResponseMetadata,
    WaitForStateTransitionResultResponse,
};
use dash_context_provider::ContextProviderError;
use dpp::state_transition::proof_result::StateTransitionProofResult;
use dpp::state_transition::StateTransition;
use drive_proof_verifier::FromProof;
use rs_dapi_client::WrapToExecutionResult;
use rs_dapi_client::{DapiRequest, ExecutionError, InnerInto, IntoInner, RequestSettings};
use tracing::{trace, warn};

#[async_trait::async_trait]
pub trait BroadcastStateTransition {
    async fn broadcast(&self, sdk: &Sdk, settings: Option<PutSettings>) -> Result<(), Error>;
    async fn wait_for_response<T: TryFrom<StateTransitionProofResult> + Send>(
        &self,
        sdk: &Sdk,
        settings: Option<PutSettings>,
    ) -> Result<T, Error>;
    /// Like [`wait_for_response`](Self::wait_for_response), but also
    /// returns the quorum-authenticated response metadata.
    /// `metadata.height` is the committed block the proof attests —
    /// callers that persist proof-attested absolute balances need it as
    /// the balance's height pin
    /// (`dash_sdk::platform::address_sync::AddressFunds::as_of_height`).
    async fn wait_for_response_with_metadata<T: TryFrom<StateTransitionProofResult> + Send>(
        &self,
        sdk: &Sdk,
        settings: Option<PutSettings>,
    ) -> Result<(T, ResponseMetadata), Error>;
    async fn broadcast_and_wait<T: TryFrom<StateTransitionProofResult> + Send>(
        &self,
        sdk: &Sdk,
        settings: Option<PutSettings>,
    ) -> Result<T, Error>;
    /// Like [`broadcast_and_wait`](Self::broadcast_and_wait), but also
    /// returns the quorum-authenticated response metadata (see
    /// [`wait_for_response_with_metadata`](Self::wait_for_response_with_metadata)).
    async fn broadcast_and_wait_with_metadata<T: TryFrom<StateTransitionProofResult> + Send>(
        &self,
        sdk: &Sdk,
        settings: Option<PutSettings>,
    ) -> Result<(T, ResponseMetadata), Error>;
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
            Err(e) => {
                warn!(error = ?e, "broadcast: failed after retries");
                if let Some(owner_id) = self.owner_id() {
                    sdk.refresh_identity_nonce(&owner_id).await;
                }
            }
        }
        result
    }
    async fn wait_for_response<T: TryFrom<StateTransitionProofResult> + Send>(
        &self,
        sdk: &Sdk,
        settings: Option<PutSettings>,
    ) -> Result<T, Error> {
        self.wait_for_response_with_metadata::<T>(sdk, settings)
            .await
            .map(|(result, _metadata)| result)
    }

    async fn wait_for_response_with_metadata<T: TryFrom<StateTransitionProofResult> + Send>(
        &self,
        sdk: &Sdk,
        settings: Option<PutSettings>,
    ) -> Result<(T, ResponseMetadata), Error> {
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
                warn!(error=?e, "wait: state transition broadcast error detected");
                let state_transition_broadcast_error: StateTransitionBroadcastError =
                    StateTransitionBroadcastError::try_from(e.clone())
                        .wrap_to_execution_result(&response)?
                        .inner;

                return Err(Error::from(state_transition_broadcast_error))
                    .wrap_to_execution_result(&response);
            }

            let context_provider = sdk.context_provider().ok_or(ExecutionError {
                inner: Error::from(ContextProviderError::Config(
                    "Context provider not initialized".to_string(),
                )),
                address: Some(response.address.clone()),
                retries: response.retries,
            })?;

            // Verify through the `FromProof` impl: it runs the GroveDB structural check AND
            // `verify_tenderdash_proof` (the quorum BLS signature gate) that authenticates
            // `metadata`. The request must be reconstructed to feed that verifier.
            let request: BroadcastStateTransitionRequest = self
                .broadcast_request_for_state_transition()
                .wrap_to_execution_result(&response)?
                .inner;

            trace!("wait: verifying proof and quorum signature");
            let (maybe_result, metadata, _proof) = <StateTransitionProofResult as FromProof<
                BroadcastStateTransitionRequest,
            >>::maybe_from_proof_with_metadata(
                request,
                grpc_response.clone(),
                sdk.network,
                sdk.version(),
                &context_provider,
            )
            .map_err(Error::from)
            .wrap_to_execution_result(&response)?
            .inner;

            // The current `FromProof` impl always yields `Some`; this guards only a future
            // impl change, so it stays a typed error rather than an unwrap.
            let result: StateTransitionProofResult = maybe_result
                .ok_or_else(|| {
                    Error::InvalidProvedResponse(
                        "state transition result missing from verified proof".to_string(),
                    )
                })
                .wrap_to_execution_result(&response)?
                .inner;

            // `metadata` is quorum-authenticated only after the verification above, so the
            // protocol-version ratchet must run here, never before. A `StaleNode` error is
            // retryable and prompts another server.
            let _: () = sdk
                .verify_response_metadata("wait_for_state_transition_result", &metadata)
                .wrap_to_execution_result(&response)?
                .inner;

            trace!("wait: proof verification successful");
            trace!(result_variant = %result.to_string(), "wait: result variant");

            let variant_name = result.to_string();
            let conversion_result = T::try_from(result)
                .map(|converted| (converted, metadata))
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

    async fn broadcast_and_wait<T: TryFrom<StateTransitionProofResult> + Send>(
        &self,
        sdk: &Sdk,
        settings: Option<PutSettings>,
    ) -> Result<T, Error> {
        self.broadcast_and_wait_with_metadata::<T>(sdk, settings)
            .await
            .map(|(result, _metadata)| result)
    }

    async fn broadcast_and_wait_with_metadata<T: TryFrom<StateTransitionProofResult> + Send>(
        &self,
        sdk: &Sdk,
        settings: Option<PutSettings>,
    ) -> Result<(T, ResponseMetadata), Error> {
        trace!(state_transition = %self.name(), "broadcast_and_wait: start");
        trace!("broadcast_and_wait: step 1 - broadcasting");
        self.broadcast(sdk, settings).await?;
        trace!("broadcast_and_wait: step 2 - waiting for response");
        let result = self
            .wait_for_response_with_metadata::<T>(sdk, settings)
            .await;
        match &result {
            Ok(_) => trace!("broadcast_and_wait: complete success"),
            Err(e) => warn!(error = ?e, "broadcast_and_wait: failed"),
        }
        result
    }
}
