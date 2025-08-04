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
use tonic::{Request, Response, Status};
use tracing::{debug, error, info, warn};

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
    ) -> Result<Response<BroadcastStateTransitionResponse>, Status> {
        let st_bytes_vec = request.get_ref().state_transition.clone();

        // Validate that state transition is provided
        if st_bytes_vec.is_empty() {
            error!("State transition is empty");
            return Err(Status::invalid_argument(
                "State Transition is not specified",
            ));
        }

        let st_bytes = st_bytes_vec.as_slice();
        let st_hash = hex::encode(Sha256::digest(st_bytes));

        // Convert to base64 for Tenderdash RPC
        let tx_base64 = BASE64_STANDARD.encode(st_bytes);

        // Attempt to broadcast the transaction
        let broadcast_result = match self.tenderdash_client.broadcast_tx(tx_base64.clone()).await {
            Ok(response) => response,
            Err(e) => {
                let error_msg = e.to_string();
                warn!(
                    error = %error_msg,
                    st_hash = %st_hash,
                    "Failed to broadcast state transition to Tenderdash"
                );

                if error_msg.contains("ECONNRESET") || error_msg.contains("socket hang up") {
                    return Err(Status::unavailable("Tenderdash is not available"));
                }

                return Err(Status::internal(format!(
                    "Failed broadcasting state transition: {}",
                    error_msg
                )));
            }
        };

        // Check broadcast result
        if broadcast_result.code != 0 {
            warn!(
                code = broadcast_result.code,
                info = ?broadcast_result.info,
                st_hash = %st_hash,
                "State transition broadcast failed"
            );

            // Handle specific error cases
            if let Some(data) = &broadcast_result.data {
                return self
                    .handle_broadcast_error(data, st_bytes, &tx_base64)
                    .await;
            }

            // Convert Drive error response
            return self
                .create_grpc_error_from_drive_response(broadcast_result.code, broadcast_result.info)
                .await;
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
    ) -> Result<Response<BroadcastStateTransitionResponse>, Status> {
        if error_data == "tx already exists in cache" {
            return self.handle_duplicate_transaction(st_bytes, tx_base64).await;
        }

        if error_data.starts_with("Tx too large.") {
            let message = error_data.replace("Tx too large. ", "");
            return Err(Status::invalid_argument(format!(
                "state transition is too large. {}",
                message
            )));
        }

        if error_data.starts_with("mempool is full") {
            return Err(Status::resource_exhausted(error_data));
        }

        if error_data.contains("context deadline exceeded") {
            return Err(Status::resource_exhausted(
                "broadcasting state transition is timed out",
            ));
        }

        if error_data.contains("too_many_resets") {
            return Err(Status::resource_exhausted(
                "tenderdash is not responding: too many requests",
            ));
        }

        if error_data.starts_with("broadcast confirmation not received:") {
            error!("Failed broadcasting state transition: {}", error_data);
            return Err(Status::unavailable(error_data));
        }

        // Unknown error
        error!(
            "Unexpected error during broadcasting state transition: {}",
            error_data
        );
        Err(Status::internal(format!(
            "Unexpected error: {}",
            error_data
        )))
    }

    /// Handle duplicate transaction scenarios
    async fn handle_duplicate_transaction(
        &self,
        st_bytes: &[u8],
        tx_base64: &str,
    ) -> Result<Response<BroadcastStateTransitionResponse>, Status> {
        // Compute state transition hash
        let mut hasher = Sha256::new();
        hasher.update(st_bytes);
        let st_hash = hasher.finalize();
        let st_hash_base64 = BASE64_STANDARD.encode(st_hash);

        debug!(
            "Checking duplicate state transition with hash: {}",
            hex::encode(st_hash)
        );

        // Check if the ST is in the mempool
        match self.tenderdash_client.unconfirmed_txs(Some(100)).await {
            Ok(unconfirmed_response) => {
                if let Some(txs) = &unconfirmed_response.txs {
                    if txs.contains(&tx_base64.to_string()) {
                        return Err(Status::already_exists(
                            "state transition already in mempool",
                        ));
                    }
                }
            }
            Err(e) => {
                warn!("Failed to check unconfirmed transactions: {}", e);
            }
        }

        // Check if the ST is already committed to the blockchain
        match self.tenderdash_client.tx(st_hash_base64).await {
            Ok(tx_response) => {
                if tx_response.tx_result.is_some() {
                    return Err(Status::already_exists("state transition already in chain"));
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
        match self.tenderdash_client.check_tx(tx_base64.to_string()).await {
            Ok(check_response) => {
                if check_response.code != 0 {
                    // Return validation error
                    return self
                        .create_grpc_error_from_drive_response(
                            check_response.code,
                            check_response.info,
                        )
                        .await;
                } else {
                    // CheckTx passes but ST was removed from block - this is a bug
                    warn!(
                        "State transition {} is passing CheckTx but removed from the block by proposer",
                        hex::encode(st_hash)
                    );

                    Err(Status::internal(
                        "State Transition processing error. Please report faulty state transition and try to create a new state transition with different hash as a workaround."
                    ))
                }
            }
            Err(e) => {
                error!("Failed to check transaction validation: {}", e);
                Err(Status::internal("Failed to validate state transition"))
            }
        }
    }

    /// Convert Drive error codes to appropriate gRPC Status
    async fn create_grpc_error_from_drive_response(
        &self,
        code: u32,
        info: Option<String>,
    ) -> Result<Response<BroadcastStateTransitionResponse>, Status> {
        let message = info.unwrap_or_else(|| format!("Drive error code: {}", code));

        // Map common Drive error codes to gRPC status codes
        let status = match code {
            1 => Status::invalid_argument(message),
            2 => Status::failed_precondition(message),
            3 => Status::out_of_range(message),
            4 => Status::unimplemented(message),
            5 => Status::internal(message),
            6 => Status::unavailable(message),
            7 => Status::unauthenticated(message),
            8 => Status::permission_denied(message),
            9 => Status::aborted(message),
            10 => Status::out_of_range(message),
            11 => Status::unimplemented(message),
            12 => Status::internal(message),
            13 => Status::internal(message),
            14 => Status::unavailable(message),
            15 => Status::data_loss(message),
            16 => Status::unauthenticated(message),
            _ => Status::unknown(message),
        };

        Err(status)
    }
}
