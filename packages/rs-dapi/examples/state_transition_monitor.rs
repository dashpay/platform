use dapi_grpc::platform::v0::{
    wait_for_state_transition_result_request::{Version, WaitForStateTransitionResultRequestV0},
    wait_for_state_transition_result_response::{
        self, wait_for_state_transition_result_response_v0,
    },
    WaitForStateTransitionResultRequest,
};
use dapi_grpc::tonic::{transport::Channel, Request};
use std::env;
use tracing::{error, info, warn};

// Import the generated gRPC client
use dapi_grpc::platform::v0::platform_client::PlatformClient;

/// Example application that waits for a specific state transition to be processed
/// and shows the result, including proofs if requested.
///
/// This demonstrates the WaitForStateTransitionResult gRPC endpoint which:
/// 1. Waits for a state transition to be included in a block
/// 2. Returns the result (success/error) of the state transition processing
/// 3. Optionally provides cryptographic proofs of the state transition
///
/// Usage: state_transition_monitor <dapi-grpc-url> <state-transition-hash> [prove]
///
/// Arguments:
///   dapi-grpc-url: URL of the DAPI gRPC server (e.g., http://localhost:3010)
///   state-transition-hash: Hex-encoded hash of the state transition to monitor
///   prove: Optional flag to request cryptographic proof (true/false, default: false)
///
/// Example:
///   state_transition_monitor http://localhost:3010 4bc5547b87323ef4efd9ef3ebfee4aec53a3e31877f6498126318839a01cd943 true

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        eprintln!("Wait for a state transition result from DAPI");
        eprintln!();
        eprintln!(
            "Usage: {} <dapi-grpc-url> <state-transition-hash> [prove]",
            args[0]
        );
        eprintln!();
        eprintln!("Arguments:");
        eprintln!("  dapi-grpc-url         URL of the DAPI gRPC server");
        eprintln!("  state-transition-hash Hex-encoded hash of the state transition");
        eprintln!(
            "  prove                 Request cryptographic proof (true/false, default: false)"
        );
        eprintln!();
        eprintln!("Example:");
        eprintln!("  {} http://localhost:3010 4bc5547b87323ef4efd9ef3ebfee4aec53a3e31877f6498126318839a01cd943 true", args[0]);
        eprintln!();
        eprintln!("The state transition hash should be the hash of a previously broadcast state transition.");
        eprintln!("This tool will wait until that state transition is processed by the platform.");
        std::process::exit(1);
    }

    let dapi_url = &args[1];
    let state_transition_hash_hex = &args[2];
    let prove = args.get(3).map(|s| s == "true").unwrap_or(false);

    info!("Connecting to DAPI at: {}", dapi_url);
    info!("Monitoring state transition: {}", state_transition_hash_hex);
    info!("Request proof: {}", prove);

    // Parse the state transition hash from hex
    let state_transition_hash = match hex::decode(state_transition_hash_hex) {
        Ok(hash) => hash,
        Err(e) => {
            error!("Invalid state transition hash format: {}", e);
            std::process::exit(1);
        }
    };

    // Connect to DAPI gRPC service
    let channel = match Channel::from_shared(dapi_url.to_string()) {
        Ok(channel) => channel,
        Err(e) => {
            error!("Invalid DAPI URL: {}", e);
            std::process::exit(1);
        }
    };

    let mut client = match PlatformClient::connect(channel).await {
        Ok(client) => client,
        Err(e) => {
            error!("Failed to connect to DAPI: {}", e);
            std::process::exit(1);
        }
    };

    info!("Successfully connected to DAPI");

    // Create the wait for state transition result request
    let request = Request::new(WaitForStateTransitionResultRequest {
        version: Some(Version::V0(WaitForStateTransitionResultRequestV0 {
            state_transition_hash,
            prove,
        })),
    });

    info!("Waiting for state transition result...");

    // Send the request and wait for response
    let response = match client.wait_for_state_transition_result(request).await {
        Ok(response) => response,
        Err(status) => {
            match status.code() {
                tonic::Code::DeadlineExceeded => {
                    error!("Timeout: State transition was not processed within the timeout period");
                    error!("This could mean:");
                    error!("  1. The state transition was never broadcast");
                    error!("  2. The state transition is taking longer than expected to process");
                    error!("  3. There are network connectivity issues");
                }
                tonic::Code::InvalidArgument => {
                    error!("Invalid request: {}", status.message());
                }
                tonic::Code::Unavailable => {
                    error!("DAPI service unavailable: {}", status.message());
                }
                _ => {
                    error!("gRPC error: {} - {}", status.code(), status.message());
                }
            }
            std::process::exit(1);
        }
    };

    let response_inner = response.into_inner();

    // Process the response
    match response_inner.version {
        Some(wait_for_state_transition_result_response::Version::V0(v0)) => {
            print_response_metadata(&v0.metadata);

            match v0.result {
                Some(wait_for_state_transition_result_response_v0::Result::Proof(proof)) => {
                    info!("✅ State transition processed successfully!");
                    print_proof_info(&proof);
                }
                Some(wait_for_state_transition_result_response_v0::Result::Error(error)) => {
                    warn!("❌ State transition failed with error:");
                    print_error_info(&error);
                }
                None => {
                    info!("✅ State transition processed successfully (no proof requested)");
                }
            }
        }
        None => {
            error!("Invalid response format");
            std::process::exit(1);
        }
    }

    Ok(())
}

fn print_response_metadata(metadata: &Option<dapi_grpc::platform::v0::ResponseMetadata>) {
    if let Some(metadata) = metadata {
        info!("Response Metadata:");
        info!("  Block Height: {}", metadata.height);
        info!(
            "  Core Chain Locked Height: {}",
            metadata.core_chain_locked_height
        );
        info!("  Epoch: {}", metadata.epoch);
        info!("  Time: {} ms", metadata.time_ms);
        info!("  Protocol Version: {}", metadata.protocol_version);
        info!("  Chain ID: {}", metadata.chain_id);
    }
}

fn print_proof_info(proof: &dapi_grpc::platform::v0::Proof) {
    info!("Cryptographic Proof:");
    info!("  GroveDB Proof Size: {} bytes", proof.grovedb_proof.len());
    info!("  Quorum Hash: {}", hex::encode(&proof.quorum_hash));
    info!("  Signature Size: {} bytes", proof.signature.len());
    info!("  Round: {}", proof.round);
    info!("  Block ID Hash: {}", hex::encode(&proof.block_id_hash));
    info!("  Quorum Type: {}", proof.quorum_type);

    if !proof.grovedb_proof.is_empty() {
        info!("  GroveDB Proof: {}", hex::encode(&proof.grovedb_proof));
    }

    if !proof.signature.is_empty() {
        info!("  Signature: {}", hex::encode(&proof.signature));
    }
}

fn print_error_info(error: &dapi_grpc::platform::v0::StateTransitionBroadcastError) {
    error!("Error Details:");
    error!("  Code: {}", error.code);
    error!("  Message: {}", error.message);

    if !error.data.is_empty() {
        error!("  Data: {}", hex::encode(&error.data));

        // Try to decode data as UTF-8 string if possible
        if let Ok(data_str) = String::from_utf8(error.data.clone()) {
            error!("  Data (as string): {}", data_str);
        }
    }

    // Provide helpful error interpretations
    match error.code {
        1 => error!("  → Invalid state transition structure"),
        2 => error!("  → Consensus validation failed"),
        3 => error!("  → State validation failed (e.g., document not found, insufficient balance)"),
        4 => error!("  → Basic validation failed (e.g., invalid signature)"),
        _ => error!("  → Unknown error code"),
    }
}
