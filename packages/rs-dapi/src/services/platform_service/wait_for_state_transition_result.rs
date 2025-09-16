use super::error_mapping::build_state_transition_error;
use crate::services::platform_service::PlatformServiceImpl;
use crate::services::streaming_service::FilterType;
use dapi_grpc::platform::v0::{
    Proof, ResponseMetadata, WaitForStateTransitionResultRequest,
    WaitForStateTransitionResultResponse, wait_for_state_transition_result_request,
    wait_for_state_transition_result_response,
};
use dapi_grpc::tonic::{Request, Response, Status};
use std::time::Duration;
use tokio::time::timeout;
use tracing::{debug, info, trace, warn};

impl PlatformServiceImpl {
    pub async fn wait_for_state_transition_result_impl(
        &self,
        request: Request<WaitForStateTransitionResultRequest>,
    ) -> Result<Response<WaitForStateTransitionResultResponse>, Status> {
        let inner = request.into_inner();
        let v0 = match inner.version {
            Some(wait_for_state_transition_result_request::Version::V0(v0)) => v0,
            None => {
                return Err(Status::invalid_argument(
                    "wait_for_state_transition_result request must have v0",
                ));
            }
        };

        // Validate state transition hash
        let state_transition_hash = v0.state_transition_hash;
        if state_transition_hash.is_empty() {
            return Err(Status::invalid_argument(
                "state transition hash is not specified",
            ));
        }

        // Convert to hex string for Tenderdash queries
        let hash_string = hex::encode(&state_transition_hash).to_uppercase();

        info!(
            "waitForStateTransitionResult called for hash: {}",
            hash_string
        );

        // Check if WebSocket is connected
        if !self.tenderdash_client.is_websocket_connected() {
            return Err(Status::unavailable("Tenderdash is not available"));
        }

        // RACE-FREE IMPLEMENTATION: Subscribe via subscription manager BEFORE checking existing state
        trace!(
            "Subscribing (manager) to platform tx for hash: {}",
            hash_string
        );
        let sub_handle = self
            .subscriber_manager
            .add_subscription(FilterType::PlatformTxId(hash_string.clone()))
            .await;

        // Check if transaction already exists (after subscription is active)
        trace!("Checking existing transaction for hash: {}", hash_string);
        match self.tenderdash_client.tx(hash_string.clone()).await {
            Ok(existing_tx) => {
                info!("Found existing transaction for hash: {}", hash_string);
                return self
                    .build_response_from_existing_tx(existing_tx, v0.prove)
                    .await;
            }
            Err(e) => {
                debug!("Transaction not found (will wait for future events): {}", e);
                // Transaction not found, proceed to wait for future events
            }
        }

        // Wait for transaction event with timeout
        let timeout_duration =
            Duration::from_millis(self.config.dapi.state_transition_wait_timeout);

        trace!(
            "Waiting for transaction event with timeout: {:?}",
            timeout_duration
        );

        // Filter events to find our specific transaction
        loop {
            match timeout(timeout_duration, sub_handle.recv()).await {
                Ok(Some(crate::services::streaming_service::StreamingEvent::PlatformTx {
                    event,
                })) => {
                    info!(
                        "Received matching transaction event for hash: {}",
                        hash_string
                    );
                    return self.build_response_from_event(event, v0.prove).await;
                }
                Ok(Some(message)) => {
                    // Ignore other message types
                    warn!(
                        ?message,
                        "Received non-matching message, ignoring; this should not happen due to filtering"
                    );
                    continue;
                }
                Ok(None) => {
                    warn!("Platform tx subscription channel closed unexpectedly");
                    return Err(Status::unavailable(
                        "Platform tx subscription channel closed unexpectedly",
                    ));
                }
                Err(_) => {
                    // Timeout occurred
                    return Err(Status::deadline_exceeded(format!(
                        "Waiting period for state transition {} exceeded",
                        hash_string
                    )));
                }
            }
        }
    }

    async fn build_response_from_existing_tx(
        &self,
        tx_response: crate::clients::tenderdash_client::TxResponse,
        prove: bool,
    ) -> Result<Response<WaitForStateTransitionResultResponse>, Status> {
        let mut response_v0 =
            wait_for_state_transition_result_response::WaitForStateTransitionResultResponseV0 {
                result: None,
                metadata: None,
            };

        // Check if transaction had an error
        if let Some(tx_result) = &tx_response.tx_result {
            if tx_result.code != 0 {
                // Transaction had an error
                let error = build_state_transition_error(
                    tx_result.code,
                    tx_result.info.as_deref().unwrap_or(""),
                    tx_result.data.as_deref(),
                );

                response_v0.result = Some(
                    wait_for_state_transition_result_response::wait_for_state_transition_result_response_v0::Result::Error(error)
                );
            }
        }

        // Generate proof if requested and no error
        if prove && response_v0.result.is_none() {
            if let Some(tx_bytes) = &tx_response.tx {
                if let Ok(tx_data) =
                    base64::prelude::Engine::decode(&base64::prelude::BASE64_STANDARD, tx_bytes)
                {
                    match self.fetch_proof_for_state_transition(tx_data).await {
                        Ok((proof, metadata)) => {
                            response_v0.result = Some(
                                wait_for_state_transition_result_response::wait_for_state_transition_result_response_v0::Result::Proof(proof)
                            );
                            response_v0.metadata = Some(metadata);
                        }
                        Err(e) => {
                            warn!("Failed to fetch proof: {}", e);
                            // Continue without proof
                        }
                    }
                }
            }
        }

        let response = WaitForStateTransitionResultResponse {
            version: Some(wait_for_state_transition_result_response::Version::V0(
                response_v0,
            )),
        };

        Ok(Response::new(response))
    }

    async fn build_response_from_event(
        &self,
        transaction_event: crate::clients::TransactionEvent,
        prove: bool,
    ) -> Result<Response<WaitForStateTransitionResultResponse>, Status> {
        let mut response_v0 =
            wait_for_state_transition_result_response::WaitForStateTransitionResultResponseV0 {
                result: None,
                metadata: None,
            };

        // Check transaction result
        match transaction_event.result {
            crate::clients::TransactionResult::Success => {
                // Success case - generate proof if requested
                if prove {
                    if let Some(tx_bytes) = transaction_event.tx {
                        match self.fetch_proof_for_state_transition(tx_bytes).await {
                            Ok((proof, metadata)) => {
                                response_v0.result = Some(
                                    wait_for_state_transition_result_response::wait_for_state_transition_result_response_v0::Result::Proof(proof)
                                );
                                response_v0.metadata = Some(metadata);
                            }
                            Err(e) => {
                                warn!("Failed to fetch proof: {}", e);
                                // Continue without proof
                            }
                        }
                    }
                }
            }
            crate::clients::TransactionResult::Error { code, info, data } => {
                // Error case - create error response
                let error = build_state_transition_error(code, &info, data.as_deref());
                response_v0.result = Some(
                    wait_for_state_transition_result_response::wait_for_state_transition_result_response_v0::Result::Error(error)
                );
            }
        }

        let response = WaitForStateTransitionResultResponse {
            version: Some(wait_for_state_transition_result_response::Version::V0(
                response_v0,
            )),
        };

        Ok(Response::new(response))
    }

    async fn fetch_proof_for_state_transition(
        &self,
        tx_bytes: Vec<u8>,
    ) -> crate::DAPIResult<(Proof, ResponseMetadata)> {
        // Create a GetProofsRequest with the state transition
        let request = dapi_grpc::drive::v0::GetProofsRequest {
            state_transition: tx_bytes.clone(),
        };

        // Get the internal client and make the request
        let mut internal_client = self.drive_client.get_internal_client();

        match internal_client.get_proofs(request).await {
            Ok(response) => {
                let inner = response.into_inner();

                let proof = inner
                    .proof
                    .ok_or(crate::DapiError::no_valid_tx_proof(&tx_bytes))?;
                let metadata = inner
                    .metadata
                    .ok_or(crate::DapiError::no_valid_tx_proof(&tx_bytes))?;

                Ok((proof, metadata))
            }
            Err(e) => {
                warn!("Failed to fetch proof from Drive: {}", e);
                Err(crate::DapiError::Client(format!(
                    "Failed to fetch proof: {}",
                    e
                )))
            }
        }
    }
}
