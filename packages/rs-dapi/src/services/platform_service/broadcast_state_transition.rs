/*!
 * Complex implementation of broadcastStateTransition
 *
 * This module implements the full logic for broadcasting state transitions
 * to the Tenderdash network, including validation, error handling, and
 * duplicate detection, following the JavaScript DAPI implementation.
 */

use base64::prelude::*;
use ciborium::{ser, value::Value};
use dapi_grpc::platform::v0::BroadcastStateTransitionRequest;
use sha2::{Digest, Sha256};
use tonic::{Code, Request};
use tracing::{debug, error, info, warn};

use crate::clients::tenderdash_client::BroadcastTxResponse;
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
    ) -> BroadcastTxResponse {
        let st_bytes_vec = request.get_ref().state_transition.clone();

        // Validate that state transition is provided
        if st_bytes_vec.is_empty() {
            error!("State transition is empty");
            return grpc_error_response(Code::InvalidArgument, "State Transition is not specified");
        }

        let st_bytes = st_bytes_vec.as_slice();
        let st_hash = hex::encode(Sha256::digest(st_bytes));

        // Convert to base64 for Tenderdash RPC
        let tx_base64 = BASE64_STANDARD.encode(st_bytes);

        // Attempt to broadcast the transaction
        let broadcast_result = match self.tenderdash_client.broadcast_tx(tx_base64.clone()).await {
            Ok(response) => response,
            Err(error) => {
                return self
                    .map_broadcast_error(error, st_bytes, &tx_base64, &st_hash)
                    .await;
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
            if let Some(data) = broadcast_result.data.as_deref()
                && !data.is_empty()
            {
                return self
                    .handle_broadcast_error(data, st_bytes, &tx_base64, &st_hash)
                    .await;
            }

            return broadcast_result;
        }

        info!(st_hash = %st_hash, "State transition broadcasted successfully");
        broadcast_result
    }

    /// Handle specific broadcast error cases
    async fn handle_broadcast_error(
        &self,
        error_data: &str,
        st_bytes: &[u8],
        tx_base64: &str,
        st_hash_hex: &str,
    ) -> BroadcastTxResponse {
        match classify_broadcast_error(error_data) {
            BroadcastErrorHandling::Duplicate => {
                self.handle_duplicate_transaction(st_bytes, tx_base64).await
            }
            BroadcastErrorHandling::Response(response) => {
                if error_data.starts_with("broadcast confirmation not received:") {
                    error!("Failed broadcasting state transition: {}", error_data);
                }
                response
            }
            BroadcastErrorHandling::Unknown => {
                error!(
                    st_hash = %st_hash_hex,
                    "Unexpected error during broadcasting state transition: {}",
                    error_data
                );
                grpc_error_response(Code::Internal, format!("Unexpected error: {}", error_data))
            }
        }
    }

    /// Handle duplicate transaction scenarios
    async fn handle_duplicate_transaction(
        &self,
        st_bytes: &[u8],
        tx_base64: &str,
    ) -> BroadcastTxResponse {
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
                if let Some(txs) = &unconfirmed_response.txs
                    && txs.contains(&tx_base64_owned)
                {
                    return grpc_error_response(
                        Code::AlreadyExists,
                        "state transition already in mempool",
                    );
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
                    return grpc_error_response(
                        Code::AlreadyExists,
                        "state transition already in chain",
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
                    return BroadcastTxResponse {
                        code: check_response.code,
                        data: check_response.data,
                        info: check_response.info,
                        hash: None,
                    };
                }

                // CheckTx passes but ST was removed from block - this is a bug
                warn!(
                    "State transition {} is passing CheckTx but removed from the block by proposer",
                    hex::encode(st_hash)
                );

                grpc_error_response(
                    Code::Internal,
                    "State Transition processing error. Please report faulty state transition and try to create a new state transition with different hash as a workaround.",
                )
            }
            Err(e) => {
                error!("Failed to check transaction validation: {}", e);
                match e {
                    DapiError::Client(message) => {
                        if message.contains("ECONNRESET") || message.contains("socket hang up") {
                            grpc_error_response(Code::Unavailable, "Tenderdash is not available")
                        } else {
                            grpc_error_response(
                                Code::Internal,
                                format!("Failed broadcasting state transition: {}", message),
                            )
                        }
                    }
                    DapiError::TenderdashRestError(rpc_error) => {
                        if let Some(code) = rpc_error.code
                            && (10000..50000).contains(&code)
                            && let Some(info) = rpc_error.data_as_str()
                        {
                            return BroadcastTxResponse {
                                code,
                                data: None,
                                info: Some(info.to_string()),
                                hash: None,
                            };
                        }

                        if let Some(data) = rpc_error.data_as_str()
                            && let BroadcastErrorHandling::Response(response) =
                                classify_broadcast_error(data)
                        {
                            if data.starts_with("broadcast confirmation not received:") {
                                error!("Failed broadcasting state transition: {}", data);
                            }
                            return response;
                        }

                        let message = rpc_error
                            .message
                            .clone()
                            .unwrap_or_else(|| "Tenderdash error".to_string());

                        grpc_error_response(Code::Internal, message)
                    }
                    other => grpc_error_response(Code::Internal, other.to_string()),
                }
            }
        }
    }

    async fn map_broadcast_error(
        &self,
        error: DapiError,
        st_bytes: &[u8],
        tx_base64: &str,
        st_hash_hex: &str,
    ) -> BroadcastTxResponse {
        match error {
            DapiError::Client(message) => {
                error!(
                    error = %message,
                    st_hash = %st_hash_hex,
                    "Failed to broadcast state transition to Tenderdash"
                );

                if message.contains("ECONNRESET") || message.contains("socket hang up") {
                    grpc_error_response(Code::Unavailable, "Tenderdash is not available")
                } else {
                    grpc_error_response(
                        Code::Internal,
                        format!("Failed broadcasting state transition: {}", message),
                    )
                }
            }
            DapiError::TenderdashRestError(rpc_error) => {
                error!(
                    error = %rpc_error,
                    st_hash = %st_hash_hex,
                    "Tenderdash REST error while broadcasting state transition"
                );

                if let Some(code) = rpc_error.code
                    && (10000..50000).contains(&code)
                    && let Some(info) = rpc_error.data_as_str()
                {
                    return BroadcastTxResponse {
                        code,
                        data: None,
                        info: Some(info.to_string()),
                        hash: None,
                    };
                }

                if let Some(data) = rpc_error.data_as_str() {
                    match classify_broadcast_error(data) {
                        BroadcastErrorHandling::Duplicate => {
                            return self.handle_duplicate_transaction(st_bytes, tx_base64).await;
                        }
                        BroadcastErrorHandling::Response(response) => {
                            if data.starts_with("broadcast confirmation not received:") {
                                error!("Failed broadcasting state transition: {}", data);
                            }
                            return response;
                        }
                        BroadcastErrorHandling::Unknown => {
                            // fall through to generic handling below
                        }
                    }
                }

                let message = rpc_error
                    .message
                    .clone()
                    .unwrap_or_else(|| "Tenderdash error".to_string());

                grpc_error_response(Code::Internal, message)
            }
            other => {
                error!(
                    error = %other,
                    st_hash = %st_hash_hex,
                    "Failed to broadcast state transition to Tenderdash"
                );
                grpc_error_response(Code::Internal, other.to_string())
            }
        }
    }
}

fn grpc_error_response(code: Code, message: impl AsRef<str>) -> BroadcastTxResponse {
    BroadcastTxResponse {
        code: code as i32 as i64,
        data: None,
        info: encode_message_to_info(message.as_ref()),
        hash: None,
    }
}

enum BroadcastErrorHandling {
    Duplicate,
    Response(BroadcastTxResponse),
    Unknown,
}

fn classify_broadcast_error(error_data: &str) -> BroadcastErrorHandling {
    if error_data == "tx already exists in cache" {
        return BroadcastErrorHandling::Duplicate;
    }

    if error_data.starts_with("Tx too large.") {
        let message = error_data.replace("Tx too large. ", "");
        return BroadcastErrorHandling::Response(grpc_error_response(
            Code::InvalidArgument,
            format!("state transition is too large. {}", message),
        ));
    }

    if error_data.starts_with("mempool is full") {
        return BroadcastErrorHandling::Response(grpc_error_response(
            Code::ResourceExhausted,
            error_data,
        ));
    }

    if error_data.contains("context deadline exceeded") {
        return BroadcastErrorHandling::Response(grpc_error_response(
            Code::ResourceExhausted,
            "broadcasting state transition is timed out",
        ));
    }

    if error_data.contains("too_many_resets") {
        return BroadcastErrorHandling::Response(grpc_error_response(
            Code::ResourceExhausted,
            "tenderdash is not responding: too many requests",
        ));
    }

    if error_data.starts_with("broadcast confirmation not received:") {
        return BroadcastErrorHandling::Response(grpc_error_response(
            Code::Unavailable,
            error_data,
        ));
    }

    BroadcastErrorHandling::Unknown
}

fn encode_message_to_info(message: &str) -> Option<String> {
    let map_entries = vec![(
        Value::Text("message".to_string()),
        Value::Text(message.to_string()),
    )];

    let mut buffer = Vec::new();
    if ser::into_writer(&Value::Map(map_entries), &mut buffer).is_ok() {
        Some(BASE64_STANDARD.encode(buffer))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use base64::prelude::*;
    use ciborium::{ser, value::Value};
    use tonic::Code;

    use crate::clients::tenderdash_client::BroadcastTxResponse;
    use crate::services::platform_service::error_mapping::map_drive_code_to_status;
    use crate::services::platform_service::map_broadcast_tx_response;

    fn make_consensus_info(serialized_error: &[u8]) -> String {
        let info_value = Value::Map(vec![(
            Value::Text("data".to_string()),
            Value::Map(vec![(
                Value::Text("serializedError".to_string()),
                Value::Bytes(serialized_error.to_vec()),
            )]),
        )]);

        let mut buffer = Vec::new();
        ser::into_writer(&info_value, &mut buffer).expect("expected to encode consensus info");
        BASE64_STANDARD.encode(buffer)
    }

    #[test]
    fn consensus_info_populates_consensus_metadata() {
        let serialized_error = vec![1_u8, 2, 3, 4, 5];
        let info = make_consensus_info(&serialized_error);
        let response = BroadcastTxResponse {
            code: 10010,
            data: Some(String::new()),
            info: Some(info),
            hash: None,
        };

        let status = map_drive_code_to_status(response.code, response.info.as_deref());

        assert_eq!(status.code(), Code::InvalidArgument);

        let metadata = status.metadata();
        let encoded = metadata
            .get_bin("dash-serialized-consensus-error-bin")
            .expect("consensus metadata should be present");
        let encoded_bytes = encoded
            .to_bytes()
            .expect("consensus metadata must contain valid bytes");
        assert_eq!(encoded_bytes.as_ref(), serialized_error.as_slice());

        let code_metadata = metadata
            .get("code")
            .expect("consensus code metadata should be present");
        assert_eq!(code_metadata.to_str().unwrap(), "10010");

        let mapped = map_broadcast_tx_response(response).expect_err("should map to status");
        let mapped_metadata = mapped.metadata();
        let mapped_bytes = mapped_metadata
            .get_bin("dash-serialized-consensus-error-bin")
            .expect("consensus metadata should be preserved");
        let mapped_value = mapped_bytes
            .to_bytes()
            .expect("consensus metadata must contain valid bytes");
        assert_eq!(mapped_value.as_ref(), serialized_error.as_slice());
    }
}
