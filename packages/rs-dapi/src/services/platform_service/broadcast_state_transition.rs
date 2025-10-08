/*!
 * Complex implementation of broadcastStateTransition
 *
 * This module implements the full logic for broadcasting state transitions
 * to the Tenderdash network, including validation, error handling, and
 * duplicate detection, following the JavaScript DAPI implementation.
 */

use crate::error::DapiError;
use crate::services::PlatformServiceImpl;
use crate::services::platform_service::TenderdashStatus;
use crate::services::platform_service::error_mapping::decode_consensus_error;
use base64::prelude::*;
use dapi_grpc::platform::v0::{BroadcastStateTransitionRequest, BroadcastStateTransitionResponse};
use sha2::{Digest, Sha256};
use tonic::Request;
use tracing::{Instrument, debug, trace};

impl PlatformServiceImpl {
    /// Complex implementation of broadcastStateTransition
    ///
    /// This method:
    /// 1. Validates the state transition request
    /// 2. Converts the state transition to base64 for Tenderdash
    /// 3. Broadcasts via Tenderdash RPC
    /// 4. Handles complex error scenarios including duplicates
    /// 5. Returns appropriate gRPC responses
    ///
    /// ## Returned Values
    ///
    /// code: non-zero on error
    /// data: string error message or null
    /// info: base64-encoded CBOR with error details or null
    /// hash: base64-encoded hash of the state transition or null
    pub async fn broadcast_state_transition_impl(
        &self,
        request: Request<BroadcastStateTransitionRequest>,
    ) -> Result<BroadcastStateTransitionResponse, DapiError> {
        let tx = request.get_ref().state_transition.clone();

        // Validate that state transition is provided
        if tx.is_empty() {
            debug!("State transition is empty");
            return Err(DapiError::InvalidArgument(
                "State Transition is not specified".to_string(),
            ));
        }

        let txid = Sha256::digest(&tx).to_vec();
        let txid_hex = hex::encode(&txid);

        let span = tracing::trace_span!("broadcast_state_transition_impl", tx = %txid_hex);

        async move {
            // Convert to base64 for Tenderdash RPC
            let tx_base64 = BASE64_STANDARD.encode(&tx);

            // Attempt to broadcast the transaction; note that both Ok and Err can contain
            // information about the broadcast result, so we need to handle both.
            let error_result = match self.tenderdash_client.broadcast_tx(tx_base64.clone()).await {
                Ok(broadcast_result) => {
                    if broadcast_result.code == 0 {
                        trace!(
                            st_hash = %txid_hex,
                            "broadcast_state_transition: state transition broadcasted successfully"
                        );
                        // we are good, no need to return anything specific
                        return Ok(BroadcastStateTransitionResponse {});
                    } else {
                        debug!(
                            code = broadcast_result.code,
                            info = ?broadcast_result.info,
                            data = ?broadcast_result.data,
                            tx = %txid_hex,
                            "broadcast_state_transition: State transition broadcast failed - service error"
                        );

                        // Prefer detailed error message if provided in `data`, otherwise fallback to `info`.
                        let error_message = if broadcast_result.data.is_empty() {
                            broadcast_result.info.clone()
                        } else {
                            broadcast_result.data.clone()
                        };

                        let info = if broadcast_result.info.is_empty() {
                            None
                        } else {
                            Some(broadcast_result.info.as_str())
                        };

                        map_broadcast_error(broadcast_result.code, &error_message, info)
                    }
                }
                Err(DapiError::TenderdashClientError(e)) => DapiError::TenderdashClientError(e),
                Err(error) => {
                    tracing::debug!(
                        error = %error,
                        tx = %txid_hex,
                        "broadcast_state_transition: Error broadcasting state transition to Tenderdash"
                    );
                    return Err(error);
                }
            };

            let response: Result<BroadcastStateTransitionResponse, DapiError> = match error_result {
                DapiError::AlreadyExists(_) => self.handle_duplicate_transaction(&tx, &txid).await,
                e => Err(e),
            };

            response.inspect_err(|e| {
                debug!(
                    error = %e,
                    st_hash = %txid_hex,
                    "broadcast_state_transition: failed to broadcast state transition to Tenderdash"
                );
            })
        }
        .instrument(span)
        .await
    }

    /// Handle duplicate transaction scenarios
    async fn handle_duplicate_transaction(
        &self,
        st_bytes: &[u8],
        txid: &[u8],
    ) -> Result<BroadcastStateTransitionResponse, DapiError> {
        let txid_base64 = BASE64_STANDARD.encode(txid);

        debug!(tx = txid_base64, "Checking duplicate state transition",);

        // Check if the ST is in the mempool
        match self.tenderdash_client.unconfirmed_tx(&txid_base64).await {
            Ok(_) => {
                return Err(DapiError::AlreadyExists(
                    "state transition already in mempool".to_string(),
                ));
            }
            Err(DapiError::TenderdashClientError(status)) => {
                let is_not_found = status
                    .message
                    .as_deref()
                    .map(|message| message.contains("not found"))
                    .unwrap_or(false);

                if !is_not_found {
                    return Err(DapiError::TenderdashClientError(status));
                }
            }
            Err(DapiError::NotFound(_)) => {}
            Err(e) => return Err(e),
        }

        // Check if the ST is already committed to the blockchain
        match self.tenderdash_client.tx(txid_base64.clone()).await {
            Ok(tx_response) => {
                if !tx_response.tx_result.is_empty() || !tx_response.tx.is_empty() {
                    return Err(DapiError::AlreadyExists(
                        "state transition already in chain".to_string(),
                    ));
                }
            }
            Err(DapiError::NotFound(e)) => {
                tracing::trace!(
                    error = %e,
                    "State transition not found in chain, will re-validate with CheckTx"
                );
            }
            Err(e) => return Err(e),
        }

        // If not in mempool and not in chain, re-validate with CheckTx
        let st_base64 = BASE64_STANDARD.encode(st_bytes);
        match self.tenderdash_client.check_tx(st_base64).await {
            Ok(check_response) => {
                if check_response.code != 0 {
                    let val = serde_json::to_value(check_response)?;
                    return Err(DapiError::from_tenderdash_error(val));
                }

                // CheckTx passes but ST was removed from block - this is a bug
                debug!(
                    tx_bytes = hex::encode(st_bytes),
                    "State transition is passing CheckTx but removed from the block by proposer; potential bug, please report",
                );

                Err(DapiError::Internal("State Transition processing error. Please report faulty state transition and try to create a new state transition with different hash as a workaround.".to_string()))
            }
            Err(DapiError::Client(message)) => {
                if message.contains("ECONNRESET") || message.contains("socket hang up") {
                    Err(DapiError::Unavailable(
                        "Tenderdash is not available".to_string(),
                    ))
                } else {
                    Err(DapiError::Internal(format!(
                        "Failed checking state transition: {}",
                        message
                    )))
                }
            }
            Err(DapiError::TenderdashClientError(rpc_error)) => {
                Err(DapiError::TenderdashClientError(rpc_error))
            }
            Err(other) => Err(DapiError::Internal(format!(
                "State transition check failed: {}",
                other
            ))),
        }
    }
}

/// Convert Tenderdash broadcast error details into a structured `DapiError`.
fn map_broadcast_error(code: u32, error_message: &str, info: Option<&str>) -> DapiError {
    // TODO: prefer code over message when possible
    tracing::trace!(
        "broadcast_state_transition: Classifying broadcast error {}: {}",
        code,
        error_message
    );
    if error_message == "tx already exists in cache" {
        return DapiError::AlreadyExists(error_message.to_string());
    }

    if error_message.starts_with("Tx too large.") {
        let message = error_message.replace("Tx too large. ", "");
        return DapiError::InvalidArgument(
            "state transition is too large. ".to_string() + &message,
        );
    }

    if error_message.starts_with("mempool is full") {
        return DapiError::ResourceExhausted(error_message.to_string());
    }

    if error_message.contains("context deadline exceeded") {
        return DapiError::Timeout("broadcasting state transition is timed out".to_string());
    }

    if error_message.contains("too_many_requests") {
        return DapiError::ResourceExhausted(
            "tenderdash is not responding: too many requests".to_string(),
        );
    }

    if error_message.starts_with("broadcast confirmation not received:") {
        return DapiError::Timeout(error_message.to_string());
    }
    let consensus_error = info.and_then(|x| decode_consensus_error(x.to_string()));
    let message = if error_message.is_empty() {
        None
    } else {
        Some(error_message.to_string())
    };
    DapiError::TenderdashClientError(TenderdashStatus::new(
        i64::from(code),
        message,
        consensus_error,
    ))
}
