/*!
 * Complex implementation of broadcastStateTransition
 *
 * This module implements the full logic for broadcasting state transitions
 * to the Tenderdash network, including validation, error handling, and
 * duplicate detection, following the JavaScript DAPI implementation.
 */

use base64::prelude::*;
use dapi_grpc::platform::v0::{BroadcastStateTransitionRequest, BroadcastStateTransitionResponse};
use sha2::{Digest, Sha256};
use tonic::{Request, Response};
use tracing::{debug, error, info, warn};

use super::error_mapping::map_drive_code_to_status;
use crate::error::DapiError;
use crate::services::PlatformServiceImpl;

impl PlatformServiceImpl {
    /// Complex implementation of broadcastStateTransition
    ///
    /// This method:
    /// 1. Validates the state transition request
    /// 2. Converts the state transition to base64 for Tenderdash
    /// 3. Broadcasts via Tenderdash RPC
    /// 4. Handles complex error scenarios including duplicates
    /// 5. Returns appropriate gRPC responses
    pub async fn broadcast_state_transition_impl(
        &self,
        request: Request<BroadcastStateTransitionRequest>,
    ) -> Result<Response<BroadcastStateTransitionResponse>, DapiError> {
        let st_bytes_vec = request.get_ref().state_transition.clone();

        // Validate that state transition is provided
        if st_bytes_vec.is_empty() {
            error!("State transition is empty");
            return Err(DapiError::InvalidArgument(
                "State Transition is not specified".to_string(),
            ));
        }

        let st_bytes = st_bytes_vec.as_slice();
        let st_hash = hex::encode(Sha256::digest(st_bytes));

        // Convert to base64 for Tenderdash RPC
        let tx_base64 = BASE64_STANDARD.encode(st_bytes);

        // Attempt to broadcast the transaction
        let broadcast_result = match self.tenderdash_client.broadcast_tx(tx_base64.clone()).await {
            Ok(response) => response,
            Err(DapiError::Client(message)) => {
                error!(
                    error = %message,
                    st_hash = %st_hash,
                    "Failed to broadcast state transition to Tenderdash"
                );

                if message.contains("ECONNRESET") || message.contains("socket hang up") {
                    return Err(DapiError::Unavailable(
                        "Tenderdash is not available".to_string(),
                    ));
                }

                return Err(DapiError::Internal(format!(
                    "Failed broadcasting state transition: {}",
                    message
                )));
            }
            Err(DapiError::TenderdashRestError(value)) => {
                error!(
                    error = ?value,
                    st_hash = %st_hash,
                    "Tenderdash REST error while broadcasting state transition"
                );
                return Err(DapiError::TenderdashRestError(value));
            }
            Err(other) => {
                error!(
                    error = %other,
                    st_hash = %st_hash,
                    "Failed to broadcast state transition to Tenderdash"
                );
                return Err(other);
            }
        };

        // Check broadcast result
        if broadcast_result.code != 0 {
            debug!(
                code = broadcast_result.code,
                info = ?broadcast_result.info,
                st_hash = %st_hash,
                "State transition broadcast failed - service error"
            );

            // Handle specific error cases
            if let Some(data) = broadcast_result.data.as_deref() {
                return Err(self
                    .handle_broadcast_error(data, st_bytes, &tx_base64)
                    .await);
            }

            return Err(DapiError::from(map_drive_code_to_status(
                broadcast_result.code,
                broadcast_result.info.as_deref(),
            )));
        }

        info!(st_hash = %st_hash, "State transition broadcasted successfully");
        Ok(Response::new(BroadcastStateTransitionResponse {}))
    }

    /// Handle specific broadcast error cases
    async fn handle_broadcast_error(
        &self,
        error_data: &str,
        st_bytes: &[u8],
        tx_base64: &str,
    ) -> DapiError {
        if error_data == "tx already exists in cache" {
            return self.handle_duplicate_transaction(st_bytes, tx_base64).await;
        }

        if error_data.starts_with("Tx too large.") {
            let message = error_data.replace("Tx too large. ", "");
            return DapiError::InvalidArgument(format!(
                "state transition is too large. {}",
                message
            ));
        }

        if error_data.starts_with("mempool is full") {
            return DapiError::ResourceExhausted(error_data.to_string());
        }

        if error_data.contains("context deadline exceeded") {
            return DapiError::ResourceExhausted(
                "broadcasting state transition is timed out".to_string(),
            );
        }

        if error_data.contains("too_many_resets") {
            return DapiError::ResourceExhausted(
                "tenderdash is not responding: too many requests".to_string(),
            );
        }

        if error_data.starts_with("broadcast confirmation not received:") {
            error!("Failed broadcasting state transition: {}", error_data);
            return DapiError::Unavailable(error_data.to_string());
        }

        // Unknown error
        error!(
            "Unexpected error during broadcasting state transition: {}",
            error_data
        );
        DapiError::Internal(format!("Unexpected error: {}", error_data))
    }

    /// Handle duplicate transaction scenarios
    async fn handle_duplicate_transaction(&self, st_bytes: &[u8], tx_base64: &str) -> DapiError {
        // Compute state transition hash
        let mut hasher = Sha256::new();
        hasher.update(st_bytes);
        let st_hash = hasher.finalize();
        let st_hash_base64 = BASE64_STANDARD.encode(st_hash);
        let tx_base64_owned = tx_base64.to_string();

        debug!(
            "Checking duplicate state transition with hash: {}",
            hex::encode(st_hash)
        );

        // Check if the ST is in the mempool
        match self.tenderdash_client.unconfirmed_txs(Some(100)).await {
            Ok(unconfirmed_response) => {
                if let Some(txs) = &unconfirmed_response.txs {
                    if txs.contains(&tx_base64_owned) {
                        return DapiError::AlreadyExists(
                            "state transition already in mempool".to_string(),
                        );
                    }
                }
            }
            Err(e) => {
                error!(
                    "Failed to check unconfirmed transactions - technical failure: {}",
                    e
                );
            }
        }

        // Check if the ST is already committed to the blockchain
        match self.tenderdash_client.tx(st_hash_base64).await {
            Ok(tx_response) => {
                if tx_response.tx_result.is_some() {
                    return DapiError::AlreadyExists(
                        "state transition already in chain".to_string(),
                    );
                }
            }
            Err(e) => {
                let error_msg = e.to_string();
                if !error_msg.contains("not found") {
                    warn!("Failed to check transaction in chain: {}", e);
                }
            }
        }

        // If not in mempool and not in chain, re-validate with CheckTx
        match self.tenderdash_client.check_tx(tx_base64_owned).await {
            Ok(check_response) => {
                if check_response.code != 0 {
                    return DapiError::from(map_drive_code_to_status(
                        check_response.code,
                        check_response.info.as_deref(),
                    ));
                }

                // CheckTx passes but ST was removed from block - this is a bug
                warn!(
                    "State transition {} is passing CheckTx but removed from the block by proposer",
                    hex::encode(st_hash)
                );

                DapiError::Internal(
                    "State Transition processing error. Please report faulty state transition and try to create a new state transition with different hash as a workaround.".to_string(),
                )
            }
            Err(e) => {
                error!("Failed to check transaction validation: {}", e);
                DapiError::Internal("Failed to validate state transition".to_string())
            }
        }
    }

    // mapping moved to error_mapping.rs for consistency
}
