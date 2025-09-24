use super::error_mapping::build_state_transition_error;
use crate::error::DapiError;
use crate::services::platform_service::PlatformServiceImpl;
use crate::services::streaming_service::FilterType;
use base64::Engine;
use dapi_grpc::platform::v0::{
    Proof, ResponseMetadata, WaitForStateTransitionResultRequest,
    WaitForStateTransitionResultResponse, wait_for_state_transition_result_request,
    wait_for_state_transition_result_response,
};
use dapi_grpc::tonic::{Request, Response, metadata::MetadataValue};
use serde_json::Value as JsonValue;
use std::time::Duration;
use tokio::time::timeout;
use tracing::{debug, info, trace, warn};

impl PlatformServiceImpl {
    pub async fn wait_for_state_transition_result_impl(
        &self,
        request: Request<WaitForStateTransitionResultRequest>,
    ) -> Result<Response<WaitForStateTransitionResultResponse>, DapiError> {
        let inner = request.into_inner();
        let v0 = match inner.version {
            Some(wait_for_state_transition_result_request::Version::V0(v0)) => v0,
            None => {
                return Err(DapiError::InvalidArgument(
                    "wait_for_state_transition_result request must have v0".to_string(),
                ));
            }
        };

        // Validate state transition hash
        let state_transition_hash = v0.state_transition_hash;
        if state_transition_hash.is_empty() {
            return Err(DapiError::InvalidArgument(
                "state transition hash is not specified".to_string(),
            ));
        }

        // Convert hash to commonly used representations
        let hash_hex = hex::encode(&state_transition_hash).to_uppercase();
        let hash_base64 = base64::prelude::BASE64_STANDARD.encode(&state_transition_hash);

        info!("waitForStateTransitionResult called for hash: {}", hash_hex);

        // Check if WebSocket is connected
        if !self.websocket_client.is_connected() {
            return Err(DapiError::Unavailable(
                "Tenderdash is not available".to_string(),
            ));
        }

        // RACE-FREE IMPLEMENTATION: Subscribe via subscription manager BEFORE checking existing state
        trace!(
            "Subscribing (manager) to platform tx for hash: {}",
            hash_hex
        );
        let sub_handle = self
            .subscriber_manager
            .add_subscription(FilterType::PlatformTxId(hash_hex.clone()))
            .await;

        // Check if transaction already exists (after subscription is active)
        trace!("Checking existing transaction for hash: {}", hash_hex);
        match self.tenderdash_client.tx(hash_base64).await {
            Ok(tx) => {
                debug!(tx = hash_hex, "Transaction already exists, returning it");
                return self.build_response_from_existing_tx(tx, v0.prove).await;
            }
            Err(error) => {
                debug!(
                    tx = hash_hex,
                    ?error,
                    "Transaction not found, will wait for future events"
                );
            }
        };

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
                    debug!(tx = hash_hex, "Received matching transaction event");
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
                    return Err(DapiError::Unavailable(
                        "Platform tx subscription channel closed unexpectedly".to_string(),
                    ));
                }
                Err(_) => {
                    // Timeout occurred
                    return Err(DapiError::Timeout(format!(
                        "Waiting period for state transition {} exceeded",
                        hash_hex
                    )));
                }
            }
        }
    }

    async fn build_response_from_existing_tx(
        &self,
        tx_response: crate::clients::tenderdash_client::TxResponse,
        prove: bool,
    ) -> Result<Response<WaitForStateTransitionResultResponse>, DapiError> {
        let mut response_v0 =
            wait_for_state_transition_result_response::WaitForStateTransitionResultResponseV0 {
                result: None,
                metadata: None,
            };

        // Check if transaction had an error
        if let Some(tx_result) = &tx_response.tx_result
            && tx_result.code != 0
        {
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

        // Generate proof if requested and no error
        if prove
            && response_v0.result.is_none()
            && let Some(tx_bytes) = &tx_response.tx
            && let Ok(tx_data) =
                base64::prelude::Engine::decode(&base64::prelude::BASE64_STANDARD, tx_bytes)
        {
            match self.fetch_proof_for_state_transition(tx_data).await {
                Ok((proof, metadata)) => {
                    response_v0.result = Some(
                        wait_for_state_transition_result_response::wait_for_state_transition_result_response_v0::Result::Proof(proof),
                    );
                    response_v0.metadata = Some(metadata);
                }
                Err(e) => {
                    warn!("Failed to fetch proof: {}", e);
                    // Continue without proof
                }
            }
        }

        let body = WaitForStateTransitionResultResponse {
            version: Some(wait_for_state_transition_result_response::Version::V0(
                response_v0,
            )),
        };

        Ok(response_with_consensus_metadata(body))
    }

    async fn build_response_from_event(
        &self,
        transaction_event: crate::clients::TransactionEvent,
        prove: bool,
    ) -> Result<Response<WaitForStateTransitionResultResponse>, DapiError> {
        let mut response_v0 =
            wait_for_state_transition_result_response::WaitForStateTransitionResultResponseV0 {
                result: None,
                metadata: None,
            };

        // Check transaction result
        match transaction_event.result {
            crate::clients::TransactionResult::Success => {
                // Success case - generate proof if requested
                if prove && let Some(tx_bytes) = transaction_event.tx {
                    match self.fetch_proof_for_state_transition(tx_bytes).await {
                        Ok((proof, metadata)) => {
                            response_v0.result = Some(
                        wait_for_state_transition_result_response::wait_for_state_transition_result_response_v0::Result::Proof(proof),
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
            crate::clients::TransactionResult::Error { code, info, data } => {
                // Error case - create error response
                let error = build_state_transition_error(code, &info, data.as_deref());
                response_v0.result = Some(
                    wait_for_state_transition_result_response::wait_for_state_transition_result_response_v0::Result::Error(error)
                );
            }
        }

        let body = WaitForStateTransitionResultResponse {
            version: Some(wait_for_state_transition_result_response::Version::V0(
                response_v0,
            )),
        };

        Ok(response_with_consensus_metadata(body))
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

fn map_dapi_error_to_state_transition_broadcast_error(
    error: &DapiError,
) -> dapi_grpc::platform::v0::StateTransitionBroadcastError {
    match error {
        DapiError::TenderdashRestError(value) => map_tenderdash_rest_error(value),
        other => {
            let status = other.to_status();
            dapi_grpc::platform::v0::StateTransitionBroadcastError {
                code: status.code() as u32,
                message: status.message().to_string(),
                data: Vec::new(),
            }
        }
    }
}

pub(super) fn build_wait_for_state_transition_error_response(
    error: &DapiError,
) -> Response<WaitForStateTransitionResultResponse> {
    use wait_for_state_transition_result_response::wait_for_state_transition_result_response_v0::Result as WaitForResult;

    let response_v0 =
        wait_for_state_transition_result_response::WaitForStateTransitionResultResponseV0 {
            result: Some(WaitForResult::Error(
                map_dapi_error_to_state_transition_broadcast_error(error),
            )),
            metadata: None,
        };

    let body = WaitForStateTransitionResultResponse {
        version: Some(wait_for_state_transition_result_response::Version::V0(
            response_v0,
        )),
    };

    response_with_consensus_metadata(body)
}

fn map_tenderdash_rest_error(
    value: &JsonValue,
) -> dapi_grpc::platform::v0::StateTransitionBroadcastError {
    use dapi_grpc::platform::v0::StateTransitionBroadcastError;

    let mut code = 0u32;
    let mut message = String::new();
    let mut data = Vec::new();

    if let JsonValue::Object(object) = value {
        if let Some(code_value) = extract_number(object.get("code"))
            && code_value >= 0
        {
            code = code_value as u32;
        }

        if let Some(msg) = object.get("message").and_then(JsonValue::as_str) {
            message = msg.to_string();
        }

        if let Some(data_value) = object.get("data") {
            if let JsonValue::Object(data_object) = data_value {
                if code == 0
                    && let Some(inner_code) = extract_number(data_object.get("code"))
                    && inner_code >= 0
                {
                    code = inner_code as u32;
                }

                if message.is_empty() {
                    if let Some(info) = data_object.get("info").and_then(JsonValue::as_str) {
                        message = info.to_string();
                    } else if let Some(log) = data_object.get("log").and_then(JsonValue::as_str) {
                        message = log.to_string();
                    }
                }
            }

            data = match data_value {
                JsonValue::String(data_string) => data_string.as_bytes().to_vec(),
                other => serde_json::to_vec(other).unwrap_or_default(),
            };
        }
    } else {
        message = value.to_string();
    }

    if message.is_empty() {
        message = value.to_string();
    }

    StateTransitionBroadcastError {
        code,
        message,
        data,
    }
}

fn extract_number(value: Option<&JsonValue>) -> Option<i64> {
    match value? {
        JsonValue::Number(num) => num.as_i64(),
        JsonValue::String(text) => text.parse::<i64>().ok(),
        _ => None,
    }
}

fn response_with_consensus_metadata(
    body: WaitForStateTransitionResultResponse,
) -> Response<WaitForStateTransitionResultResponse> {
    use wait_for_state_transition_result_response::Version;
    use wait_for_state_transition_result_response::wait_for_state_transition_result_response_v0::Result as WaitForResult;

    let mut response = Response::new(body);

    let consensus_bytes = response
        .get_ref()
        .version
        .as_ref()
        .and_then(|version| match version {
            Version::V0(v0) => v0.result.as_ref().and_then(|result| match result {
                WaitForResult::Error(error) => (!error.data.is_empty()).then_some(&error.data),
                _ => None,
            }),
        })
        .cloned();

    if let Some(bytes) = consensus_bytes {
        let value = MetadataValue::from_bytes(bytes.as_slice());
        response
            .metadata_mut()
            .insert_bin("dash-serialized-consensus-error-bin", value);
    }

    response
}
