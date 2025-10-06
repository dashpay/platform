use super::monitor::{print_error_info, print_proof_info, print_response_metadata};
use clap::Args;
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
use std::time::Duration;
use tokio::time::timeout;
use tracing::{info, warn};

use crate::error::{CliError, CliResult};

#[derive(Args, Debug)]
pub struct WorkflowCommand {
    /// Hex-encoded state transition to broadcast
    #[arg(long, value_name = "HEX")]
    pub state_transition_hex: String,

    /// Request cryptographic proof in the result
    #[arg(long, default_value_t = false)]
    pub prove: bool,

    /// Timeout (seconds) when waiting for the result
    #[arg(long, default_value_t = 60)]
    pub timeout_secs: u64,
}

pub async fn run(url: &str, cmd: WorkflowCommand) -> CliResult<()> {
    info!(prove = cmd.prove, "Starting state transition workflow");

    let state_transition =
        hex::decode(&cmd.state_transition_hex).map_err(CliError::InvalidStateTransition)?;

    info!(bytes = state_transition.len(), "Parsed state transition");

    let hash = Sha256::digest(&state_transition).to_vec();
    let hash_hex = hex::encode(&hash);
    info!(hash = %hash_hex, "Computed state transition hash");

    let channel = Channel::from_shared(url.to_string()).map_err(|source| CliError::InvalidUrl {
        url: url.to_string(),
        source: Box::new(source),
    })?;
    let mut client = PlatformClient::connect(channel).await?;

    info!("Broadcasting state transition");
    let broadcast_request = Request::new(BroadcastStateTransitionRequest {
        state_transition: state_transition.clone(),
    });

    let broadcast_start = std::time::Instant::now();
    let response = client.broadcast_state_transition(broadcast_request).await?;

    info!(duration = ?broadcast_start.elapsed(), "Broadcast succeeded");
    info!("Response: {:?}", response.into_inner());

    info!(
        timeout_secs = cmd.timeout_secs,
        "Waiting for state transition result"
    );
    let wait_request = Request::new(WaitForStateTransitionResultRequest {
        version: Some(Version::V0(WaitForStateTransitionResultRequestV0 {
            state_transition_hash: hash,
            prove: cmd.prove,
        })),
    });

    let wait_future = client.wait_for_state_transition_result(wait_request);
    let wait_start = std::time::Instant::now();

    let response = match timeout(Duration::from_secs(cmd.timeout_secs), wait_future).await {
        Ok(result) => result?,
        Err(elapsed) => return Err(CliError::Timeout(elapsed)),
    };

    info!(duration = ?wait_start.elapsed(), "State transition result received");

    let response_inner = response.into_inner();

    match response_inner.version {
        Some(wait_for_state_transition_result_response::Version::V0(v0)) => {
            print_response_metadata(&v0.metadata);

            match v0.result {
                Some(wait_for_state_transition_result_response_v0::Result::Proof(proof)) => {
                    info!("State transition processed successfully with proof");
                    print_proof_info(&proof);
                }
                Some(wait_for_state_transition_result_response_v0::Result::Error(error)) => {
                    warn!("State transition failed during processing");
                    print_error_info(&error);
                }
                None => {
                    info!("State transition processed successfully (no proof requested)");
                }
            }
        }
        None => return Err(CliError::EmptyResponse("waitForStateTransitionResult")),
    }

    Ok(())
}
