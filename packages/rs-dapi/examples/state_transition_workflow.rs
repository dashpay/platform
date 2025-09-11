use dapi_grpc::platform::v0::{
    platform_client::PlatformClient,
    wait_for_state_transition_result_request::{Version, WaitForStateTransitionResultRequestV0},
    wait_for_state_transition_result_response::{
        self, wait_for_state_transition_result_response_v0,
    },
    BroadcastStateTransitionRequest, WaitForStateTransitionResultRequest,
};
use dapi_grpc::tonic::{transport::Channel, Request};
use sha2::{Digest, Sha256};
use std::env;
use std::time::Duration;
use tracing::{error, info, warn};

/// Comprehensive example demonstrating the complete state transition workflow:
/// 1. Broadcast a state transition to the Platform
/// 2. Wait for the state transition to be processed
/// 3. Display the result, including proofs if requested
///
/// This example shows how both broadcastStateTransition and waitForStateTransitionResult
/// work together to provide a complete state transition processing experience.
///
/// Usage: state_transition_workflow <dapi-grpc-url> <state-transition-hex> [prove]
///
/// Arguments:
///   dapi-grpc-url: URL of the DAPI gRPC server (e.g., http://localhost:3010)  
///   state-transition-hex: Hex-encoded state transition data to broadcast
///   prove: Optional flag to request cryptographic proof (true/false, default: false)
///
/// Example:
///   state_transition_workflow http://localhost:3010 "01020304..." true
///
/// Note: The state transition data should be a valid, serialized state transition.
/// This example demonstrates the API usage pattern rather than creating valid state transitions.

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        eprintln!("Complete state transition workflow example");
        eprintln!();
        eprintln!(
            "Usage: {} <dapi-grpc-url> <state-transition-hex> [prove]",
            args[0]
        );
        eprintln!();
        eprintln!("Arguments:");
        eprintln!("  dapi-grpc-url         URL of the DAPI gRPC server");
        eprintln!("  state-transition-hex  Hex-encoded state transition data");
        eprintln!(
            "  prove                 Request cryptographic proof (true/false, default: false)"
        );
        eprintln!();
        eprintln!("Example:");
        eprintln!(
            "  {} http://localhost:3010 \"01020304abcdef...\" true",
            args[0]
        );
        eprintln!();
        eprintln!("This example demonstrates:");
        eprintln!("  1. Broadcasting a state transition to the Platform");
        eprintln!("  2. Waiting for the state transition to be processed");
        eprintln!("  3. Displaying the result with optional cryptographic proof");
        std::process::exit(1);
    }

    let dapi_url = &args[1];
    let state_transition_hex = &args[2];
    let prove = args.get(3).map(|s| s == "true").unwrap_or(false);

    info!("🚀 Starting state transition workflow");
    info!("📡 DAPI URL: {}", dapi_url);
    info!(
        "📦 State transition size: {} characters",
        state_transition_hex.len()
    );
    info!("🔍 Request proof: {}", prove);

    // Parse the state transition data from hex
    let state_transition_data = match hex::decode(state_transition_hex) {
        Ok(data) => data,
        Err(e) => {
            error!("❌ Invalid state transition hex format: {}", e);
            std::process::exit(1);
        }
    };

    info!(
        "✅ State transition parsed successfully ({} bytes)",
        state_transition_data.len()
    );

    // Calculate the state transition hash for monitoring
    let state_transition_hash = Sha256::digest(&state_transition_data).to_vec();
    let hash_hex = hex::encode(&state_transition_hash);
    info!("🔑 State transition hash: {}", hash_hex);

    // Connect to DAPI gRPC service
    let channel = match Channel::from_shared(dapi_url.to_string()) {
        Ok(channel) => channel,
        Err(e) => {
            error!("❌ Invalid DAPI URL: {}", e);
            std::process::exit(1);
        }
    };

    let mut client = match PlatformClient::connect(channel).await {
        Ok(client) => client,
        Err(e) => {
            error!("❌ Failed to connect to DAPI: {}", e);
            std::process::exit(1);
        }
    };

    info!("✅ Connected to DAPI Platform service");

    // Step 1: Broadcast the state transition
    info!("📤 Step 1: Broadcasting state transition...");

    let broadcast_request = Request::new(BroadcastStateTransitionRequest {
        state_transition: state_transition_data.clone(),
    });

    let broadcast_start = std::time::Instant::now();

    match client.broadcast_state_transition(broadcast_request).await {
        Ok(response) => {
            let broadcast_duration = broadcast_start.elapsed();
            info!("✅ State transition broadcasted successfully!");
            info!("⏱️  Broadcast took: {:?}", broadcast_duration);
            info!("📋 Response: {:?}", response.into_inner());
        }
        Err(status) => {
            error!(
                "❌ Failed to broadcast state transition: {} - {}",
                status.code(),
                status.message()
            );
            error!("💡 Common causes:");
            error!("   • Invalid state transition format");
            error!("   • Insufficient balance for fees");
            error!("   • State transition already exists");
            error!("   • Network connectivity issues");
            std::process::exit(1);
        }
    }

    // Step 2: Wait for the state transition to be processed
    info!("⏳ Step 2: Waiting for state transition to be processed...");

    let wait_request = Request::new(WaitForStateTransitionResultRequest {
        version: Some(Version::V0(WaitForStateTransitionResultRequestV0 {
            state_transition_hash: state_transition_hash.clone(),
            prove,
        })),
    });

    let wait_start = std::time::Instant::now();

    // Add a timeout for the wait operation
    let wait_future = client.wait_for_state_transition_result(wait_request);

    match tokio::time::timeout(Duration::from_secs(60), wait_future).await {
        Ok(result) => {
            match result {
                Ok(response) => {
                    let wait_duration = wait_start.elapsed();
                    let response_inner = response.into_inner();

                    info!("✅ State transition result received!");
                    info!("⏱️  Wait took: {:?}", wait_duration);

                    // Process the response
                    match response_inner.version {
                        Some(wait_for_state_transition_result_response::Version::V0(v0)) => {
                            print_response_metadata(&v0.metadata);

                            match v0.result {
                                Some(
                                    wait_for_state_transition_result_response_v0::Result::Proof(
                                        proof,
                                    ),
                                ) => {
                                    info!("🎉 State transition processed successfully!");
                                    print_proof_info(&proof);
                                    info!("🏆 Workflow completed successfully!");
                                }
                                Some(
                                    wait_for_state_transition_result_response_v0::Result::Error(
                                        error,
                                    ),
                                ) => {
                                    warn!("⚠️ State transition failed during processing:");
                                    print_error_info(&error);
                                    error!("❌ Workflow completed with error");
                                    std::process::exit(1);
                                }
                                None => {
                                    info!("🎉 State transition processed successfully (no proof requested)!");
                                    info!("🏆 Workflow completed successfully!");
                                }
                            }
                        }
                        None => {
                            error!("❌ Invalid response format from waitForStateTransitionResult");
                            std::process::exit(1);
                        }
                    }
                }
                Err(status) => {
                    handle_wait_error(status);
                    std::process::exit(1);
                }
            }
        }
        Err(_) => {
            error!("⏰ Timeout: State transition was not processed within 60 seconds");
            error!("💡 This could mean:");
            error!("   • The Platform network is experiencing high load");
            error!("   • There are consensus issues");
            error!("   • The state transition contains errors that prevent processing");
            std::process::exit(1);
        }
    }

    Ok(())
}

fn handle_wait_error(status: tonic::Status) {
    match status.code() {
        tonic::Code::DeadlineExceeded => {
            error!("⏰ Timeout: State transition processing exceeded the timeout period");
            error!("💡 Possible reasons:");
            error!("   • Network is under high load");
            error!("   • State transition contains complex operations");
            error!("   • Temporary consensus delays");
        }
        tonic::Code::InvalidArgument => {
            error!("❌ Invalid request: {}", status.message());
            error!("💡 Check that:");
            error!("   • State transition hash is correctly formatted");
            error!("   • Hash corresponds to a previously broadcast state transition");
        }
        tonic::Code::Unavailable => {
            error!("❌ DAPI service unavailable: {}", status.message());
            error!("💡 Possible issues:");
            error!("   • DAPI server is down or restarting");
            error!("   • Network connectivity problems");
            error!("   • WebSocket connection issues for real-time monitoring");
        }
        tonic::Code::NotFound => {
            error!("❌ State transition not found: {}", status.message());
            error!("💡 This could mean:");
            error!("   • The broadcast step failed silently");
            error!("   • The state transition hash is incorrect");
            error!("   • There's a delay in transaction propagation");
        }
        _ => {
            error!(
                "❌ Unexpected gRPC error: {} - {}",
                status.code(),
                status.message()
            );
        }
    }
}

fn print_response_metadata(metadata: &Option<dapi_grpc::platform::v0::ResponseMetadata>) {
    if let Some(metadata) = metadata {
        info!("📊 Response Metadata:");
        info!("   📏 Block Height: {}", metadata.height);
        info!(
            "   🔗 Core Chain Locked Height: {}",
            metadata.core_chain_locked_height
        );
        info!("   🌍 Epoch: {}", metadata.epoch);
        info!("   ⏰ Timestamp: {} ms", metadata.time_ms);
        info!("   📋 Protocol Version: {}", metadata.protocol_version);
        info!("   🏷️  Chain ID: {}", metadata.chain_id);
    } else {
        info!("📊 No metadata provided");
    }
}

fn print_proof_info(proof: &dapi_grpc::platform::v0::Proof) {
    info!("🔐 Cryptographic Proof:");
    info!(
        "   📊 GroveDB Proof Size: {} bytes",
        proof.grovedb_proof.len()
    );

    if !proof.quorum_hash.is_empty() {
        info!("   👥 Quorum Hash: {}", hex::encode(&proof.quorum_hash));
    }

    info!("   ✍️  Signature Size: {} bytes", proof.signature.len());
    info!("   🔄 Round: {}", proof.round);

    if !proof.block_id_hash.is_empty() {
        info!("   🆔 Block ID Hash: {}", hex::encode(&proof.block_id_hash));
    }

    info!("   🏛️  Quorum Type: {}", proof.quorum_type);

    // Show detailed proof data if available (truncated for readability)
    if !proof.grovedb_proof.is_empty() {
        let proof_preview = if proof.grovedb_proof.len() > 32 {
            format!(
                "{}...{}",
                hex::encode(&proof.grovedb_proof[..16]),
                hex::encode(&proof.grovedb_proof[proof.grovedb_proof.len() - 16..])
            )
        } else {
            hex::encode(&proof.grovedb_proof)
        };
        info!("   🌳 GroveDB Proof: {}", proof_preview);
    }

    if !proof.signature.is_empty() {
        let sig_preview = if proof.signature.len() > 32 {
            format!(
                "{}...{}",
                hex::encode(&proof.signature[..16]),
                hex::encode(&proof.signature[proof.signature.len() - 16..])
            )
        } else {
            hex::encode(&proof.signature)
        };
        info!("   📝 Signature: {}", sig_preview);
    }
}

fn print_error_info(error: &dapi_grpc::platform::v0::StateTransitionBroadcastError) {
    error!("❌ Error Details:");
    error!("   🔢 Code: {}", error.code);
    error!("   💬 Message: {}", error.message);

    if !error.data.is_empty() {
        let data_preview = if error.data.len() > 32 {
            format!(
                "{}...{}",
                hex::encode(&error.data[..16]),
                hex::encode(&error.data[error.data.len() - 16..])
            )
        } else {
            hex::encode(&error.data)
        };
        error!("   📄 Data: {}", data_preview);

        // Try to decode data as UTF-8 string if possible
        if let Ok(data_str) = String::from_utf8(error.data.clone()) {
            if data_str.len() <= 200 {
                // Only show if reasonably short
                error!("   📝 Data (as text): {}", data_str);
            }
        }
    }

    // Provide helpful error interpretations based on common error codes
    match error.code {
        1 => error!("   💡 Interpretation: Invalid state transition structure"),
        2 => error!("   💡 Interpretation: Consensus validation failed"),
        3 => error!("   💡 Interpretation: State validation failed (e.g., document not found, insufficient balance)"),
        4 => error!("   💡 Interpretation: Basic validation failed (e.g., invalid signature)"),
        10 => error!("   💡 Interpretation: Identity not found"),
        11 => error!("   💡 Interpretation: Insufficient credits for operation"),
        _ => error!("   💡 Interpretation: Unknown error code - check Platform documentation"),
    }
}
