use crate::services::platform_service::PlatformServiceImpl;
use dapi_grpc::platform::v0::{
    wait_for_state_transition_result_request, wait_for_state_transition_result_response, Proof,
    ResponseMetadata, StateTransitionBroadcastError, WaitForStateTransitionResultRequest,
    WaitForStateTransitionResultResponse,
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

        // RACE-FREE IMPLEMENTATION: Subscribe BEFORE checking existing state
        trace!(
            "Subscribing to transaction events for hash: {}",
            hash_string
        );
        let mut event_receiver = self.tenderdash_client.subscribe_to_transactions();

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
            match timeout(timeout_duration, event_receiver.recv()).await {
                Ok(Ok(transaction_event)) => {
                    if transaction_event.hash == hash_string {
                        info!(
                            "Received matching transaction event for hash: {}",
                            hash_string
                        );
                        return self
                            .build_response_from_event(transaction_event, v0.prove)
                            .await;
                    } else {
                        trace!(
                            "Received non-matching transaction event: {} (waiting for: {})",
                            transaction_event.hash,
                            hash_string
                        );
                        // Continue waiting for the right transaction
                        continue;
                    }
                }
                Ok(Err(e)) => {
                    warn!("Error receiving transaction event: {}", e);
                    continue;
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
                let error = self
                    .create_state_transition_error(
                        tx_result.code,
                        tx_result.info.as_deref().unwrap_or(""),
                        tx_result.data.as_deref(),
                    )
                    .await?;

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
                let error = self
                    .create_state_transition_error(code, &info, data.as_deref())
                    .await?;
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

    async fn create_state_transition_error(
        &self,
        code: u32,
        info: &str,
        data: Option<&str>,
    ) -> Result<StateTransitionBroadcastError, Status> {
        // This is similar to the broadcast_state_transition error handling
        // We can reuse the error creation logic from that module

        let mut error = StateTransitionBroadcastError {
            code,
            message: info.to_string(),
            data: Vec::new(),
        };

        // If there's data, try to parse it as base64 and include it
        if let Some(data_str) = data {
            if let Ok(data_bytes) =
                base64::prelude::Engine::decode(&base64::prelude::BASE64_STANDARD, data_str)
            {
                error.data = data_bytes;
            }
        }

        Ok(error)
    }

    async fn fetch_proof_for_state_transition(
        &self,
        _tx_bytes: Vec<u8>,
    ) -> Result<(Proof, ResponseMetadata), anyhow::Error> {
        // TODO: Implement actual proof fetching from Drive
        // For now, return empty proof structures

        let proof = Proof {
            grovedb_proof: Vec::new(),
            quorum_hash: Vec::new(),
            signature: Vec::new(),
            round: 0,
            block_id_hash: Vec::new(),
            quorum_type: 0,
        };

        let metadata = ResponseMetadata {
            height: 0,
            core_chain_locked_height: 0,
            epoch: 0,
            time_ms: 0,
            protocol_version: 0,
            chain_id: String::new(),
        };

        Ok((proof, metadata))
    }
}
