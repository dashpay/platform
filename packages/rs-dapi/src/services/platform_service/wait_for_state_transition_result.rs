use crate::error::DapiError;
use crate::services::platform_service::error_mapping::decode_consensus_error;
use crate::services::platform_service::{PlatformServiceImpl, TenderdashStatus};
use crate::services::streaming_service::FilterType;
use base64::Engine;
use dapi_grpc::platform::v0::wait_for_state_transition_result_response::wait_for_state_transition_result_response_v0;
use dapi_grpc::platform::v0::{
    FeeResult, Proof, ResponseMetadata, WaitForStateTransitionResultRequest,
    WaitForStateTransitionResultResponse, wait_for_state_transition_result_request,
    wait_for_state_transition_result_response,
};
use dapi_grpc::tonic::{Request, Response};
use std::time::Duration;
use tokio::time::timeout;
use tracing::{Instrument, debug, trace};

/// Decodes fee result from the encoded byte format.
/// Format: processing_fee (8 bytes LE) | storage_fee (8 bytes LE) | removed_bytes_from_system (4 bytes LE)
fn decode_fee_result(data: &[u8]) -> Option<FeeResult> {
    if data.len() < 20 {
        return None;
    }

    let processing_fee = u64::from_le_bytes(data[0..8].try_into().ok()?);
    let storage_fee = u64::from_le_bytes(data[8..16].try_into().ok()?);
    let removed_from_system = u32::from_le_bytes(data[16..20].try_into().ok()?);

    Some(FeeResult {
        processing_fee,
        storage_fee,
        removed_from_system: removed_from_system as u64,
    })
}

impl PlatformServiceImpl {
    /// Wait for a state transition result by subscribing to platform events and returning proofs when requested.
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

        let span = tracing::trace_span!("wait_for_state_transition_result", tx = %hash_hex);

        async move {
            trace!("waitForStateTransitionResult called for hash: {}", hash_hex);

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
                    debug!(?error, "Transaction not found, will wait for future events");
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
            timeout(timeout_duration, async {
                loop {
                    let result = sub_handle.recv().await;
                    match result {
                        Some(crate::services::streaming_service::StreamingEvent::PlatformTx { event }) => {
                            debug!(tx = hash_hex, "Received matching transaction event");
                            return self.build_response_from_event(event, v0.prove).await;
                        }
                        Some(message) => {
                            // Ignore other message types
                            trace!(
                                ?message,
                                "Received non-matching message, ignoring; this should not happen due to filtering"
                            );
                            continue;
                        }
                        None => {
                            debug!("Platform tx subscription channel closed unexpectedly");
                            return Err(DapiError::Unavailable(
                                "Platform tx subscription channel closed unexpectedly".to_string(),
                            ));
                        }
                    }
                }
            })
            .await
            .map_err(|msg| DapiError::Timeout(msg.to_string()))
            .inspect_err(|e| {
                tracing::debug!(
                    error = %e,
                    tx = %hash_hex,
                    "wait_for_state_transition_result: timed out"
                );
            })?
        }
        .instrument(span)
        .await
    }

    /// Build a response for a transaction already known to Tenderdash, optionally generating proofs.
    async fn build_response_from_existing_tx(
        &self,
        tx_response: crate::clients::tenderdash_client::TxResponse,
        prove: bool,
    ) -> Result<Response<WaitForStateTransitionResultResponse>, DapiError> {
        let mut response_v0 =
            wait_for_state_transition_result_response::WaitForStateTransitionResultResponseV0 {
                result: None,
                metadata: None,
                fee_result: None,
            };

        // Check if transaction had an error
        let tx_result = &tx_response.tx_result;

        if tx_result.code != 0 {
            // Transaction had an error - still try to extract fee for paid consensus errors
            let fee_result = if !tx_result.data.is_empty() {
                base64::prelude::Engine::decode(&base64::prelude::BASE64_STANDARD, &tx_result.data)
                    .ok()
                    .and_then(|data| decode_fee_result(&data))
            } else {
                None
            };

            let consensus_error_serialized = if tx_result.info.is_empty() {
                None
            } else {
                decode_consensus_error(tx_result.info.clone())
            };

            let error = TenderdashStatus::new_with_fees(
                i64::from(tx_result.code),
                if tx_result.data.is_empty() {
                    None
                } else {
                    Some(tx_result.data.clone())
                },
                consensus_error_serialized,
                fee_result,
            );
            return Ok(error.into());
        }

        // Extract fee result for successful transactions
        if !tx_result.data.is_empty() {
            if let Ok(fee_data) =
                base64::prelude::Engine::decode(&base64::prelude::BASE64_STANDARD, &tx_result.data)
            {
                response_v0.fee_result = decode_fee_result(&fee_data);
            }
        }

        // No error; generate proof if requested
        if prove
            && !tx_response.tx.is_empty()
            && let Ok(tx_data) =
                base64::prelude::Engine::decode(&base64::prelude::BASE64_STANDARD, &tx_response.tx)
        {
            match self.fetch_proof_for_state_transition(tx_data).await {
                Ok((proof, metadata)) => {
                    response_v0.result = Some(
                        wait_for_state_transition_result_response_v0::Result::Proof(proof),
                    );
                    response_v0.metadata = Some(metadata);
                }
                Err(e) => {
                    debug!("Failed to fetch proof: {}", e);
                    // Continue without proof
                }
            }
        }

        let body = WaitForStateTransitionResultResponse {
            version: Some(wait_for_state_transition_result_response::Version::V0(
                response_v0,
            )),
        };

        Ok(body.into())
    }

    /// Build a response from a streamed transaction event, handling success and error cases.
    async fn build_response_from_event(
        &self,
        transaction_event: crate::clients::TransactionEvent,
        prove: bool,
    ) -> Result<Response<WaitForStateTransitionResultResponse>, DapiError> {
        // Check transaction result
        match transaction_event.result {
            crate::clients::TransactionResult::Success => {
                // Extract fee result from transaction event
                let fee_result = transaction_event
                    .fee_data
                    .as_ref()
                    .and_then(|data| decode_fee_result(data));

                let mut response_v0 =
                    wait_for_state_transition_result_response::WaitForStateTransitionResultResponseV0 {
                        result: None,
                        metadata: None,
                        fee_result,
                };
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
                            debug!("Failed to fetch proof: {}", e);
                            // Continue without proof
                        }
                    }
                }

                let body = WaitForStateTransitionResultResponse {
                    version: Some(wait_for_state_transition_result_response::Version::V0(
                        response_v0,
                    )),
                };

                Ok(body.into())
            }
            crate::clients::TransactionResult::Error { code, info, data } => {
                // Error case - create error response
                tracing::debug!(
                    code,
                    info = ?info,
                    data = ?data,
                    "Transaction event indicates error"
                );
                let consensus_error = if info.is_empty() {
                    None
                } else {
                    decode_consensus_error(info.clone())
                };
                let error = TenderdashStatus::new(code as i64, data, consensus_error);
                let result: Response<WaitForStateTransitionResultResponse> = error.into();

                Ok(result)
            }
        }
    }

    /// Fetch Drive proofs for the provided state transition bytes, returning proof and metadata.
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
                debug!("Failed to fetch proof from Drive: {}", e);
                Err(crate::DapiError::Client(format!(
                    "Failed to fetch proof: {}",
                    e
                )))
            }
        }
    }
}

/// Convert a `DapiError` into the gRPC error response expected by waitForStateTransitionResult callers.
pub(super) fn build_wait_for_state_transition_error_response(
    error: &DapiError,
) -> Response<WaitForStateTransitionResultResponse> {
    // TenderdashStatus has everything we need
    let tenderdash_status = if let DapiError::TenderdashClientError(e) = error {
        e.clone()
    } else {
        let status = error.to_status();
        let message = if status.message().is_empty() {
            None
        } else {
            Some(status.message().to_string())
        };
        TenderdashStatus::new(status.code() as i64, message, None)
    };

    tracing::debug!(
        error = %error,
        ?tenderdash_status,
        code = tenderdash_status.code,
        "Mapping DapiError to WaitForStateTransitionResultResponse"
    );
    tenderdash_status.into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_fee_result_valid() {
        let processing_fee: u64 = 1234567890123456789;
        let storage_fee: u64 = 9876543210987654321;
        let removed_bytes: u32 = 42;

        // Encode the fee data (matching Drive's encoding)
        let mut data = Vec::with_capacity(20);
        data.extend_from_slice(&processing_fee.to_le_bytes());
        data.extend_from_slice(&storage_fee.to_le_bytes());
        data.extend_from_slice(&removed_bytes.to_le_bytes());

        let result = decode_fee_result(&data).unwrap();

        assert_eq!(result.processing_fee, processing_fee);
        assert_eq!(result.storage_fee, storage_fee);
        assert_eq!(result.removed_from_system, removed_bytes as u64);
    }

    #[test]
    fn test_decode_fee_result_zero_values() {
        let data = vec![0u8; 20];
        let result = decode_fee_result(&data).unwrap();

        assert_eq!(result.processing_fee, 0);
        assert_eq!(result.storage_fee, 0);
        assert_eq!(result.removed_from_system, 0);
    }

    #[test]
    fn test_decode_fee_result_too_short() {
        let data = vec![0u8; 19]; // One byte too short
        let result = decode_fee_result(&data);
        assert!(result.is_none());
    }

    #[test]
    fn test_decode_fee_result_empty() {
        let data: Vec<u8> = vec![];
        let result = decode_fee_result(&data);
        assert!(result.is_none());
    }

    #[test]
    fn test_decode_fee_result_extra_bytes() {
        // Extra bytes should be ignored
        let processing_fee: u64 = 100;
        let storage_fee: u64 = 200;
        let removed_bytes: u32 = 10;

        let mut data = Vec::with_capacity(25);
        data.extend_from_slice(&processing_fee.to_le_bytes());
        data.extend_from_slice(&storage_fee.to_le_bytes());
        data.extend_from_slice(&removed_bytes.to_le_bytes());
        data.extend_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF, 0xFF]); // Extra bytes

        let result = decode_fee_result(&data).unwrap();

        assert_eq!(result.processing_fee, processing_fee);
        assert_eq!(result.storage_fee, storage_fee);
        assert_eq!(result.removed_from_system, removed_bytes as u64);
    }
}
