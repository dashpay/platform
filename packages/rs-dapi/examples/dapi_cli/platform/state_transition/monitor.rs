use clap::Args;
use dapi_grpc::platform::v0::{
    WaitForStateTransitionResultRequest,
    platform_client::PlatformClient,
    wait_for_state_transition_result_request::{Version, WaitForStateTransitionResultRequestV0},
    wait_for_state_transition_result_response::{
        self, wait_for_state_transition_result_response_v0,
    },
};
use dapi_grpc::tonic::{Request, transport::Channel};
use tracing::{info, warn};

use crate::error::{CliError, CliResult};

#[derive(Args, Debug)]
pub struct MonitorCommand {
    /// Hex-encoded state transition hash to monitor
    #[arg(long, value_name = "HASH")]
    pub hash: String,

    /// Request cryptographic proof in the response
    #[arg(long, default_value_t = false)]
    pub prove: bool,
}

pub async fn run(url: &str, cmd: MonitorCommand) -> CliResult<()> {
    info!(hash = %cmd.hash, prove = cmd.prove, "Monitoring state transition");

    let state_transition_hash = hex::decode(&cmd.hash).map_err(|source| CliError::InvalidHash {
        hash: cmd.hash.clone(),
        source,
    })?;

    let channel = Channel::from_shared(url.to_string()).map_err(|source| CliError::InvalidUrl {
        url: url.to_string(),
        source: Box::new(source),
    })?;
    let mut client = PlatformClient::connect(channel).await?;

    let request = Request::new(WaitForStateTransitionResultRequest {
        version: Some(Version::V0(WaitForStateTransitionResultRequestV0 {
            state_transition_hash,
            prove: cmd.prove,
        })),
    });

    let response = client.wait_for_state_transition_result(request).await?;

    let response_inner = response.into_inner();

    match response_inner.version {
        Some(wait_for_state_transition_result_response::Version::V0(v0)) => {
            print_response_metadata(&v0.metadata);

            match v0.result {
                Some(wait_for_state_transition_result_response_v0::Result::Proof(proof)) => {
                    info!("✅ State transition processed successfully");
                    print_proof_info(&proof);
                }
                Some(wait_for_state_transition_result_response_v0::Result::Error(error)) => {
                    warn!("⚠️ State transition failed");
                    print_error_info(&error);
                }
                None => {
                    info!("✅ State transition processed (no proof requested)");
                }
            }
        }
        None => return Err(CliError::EmptyResponse("waitForStateTransitionResult")),
    }

    Ok(())
}

pub(super) fn print_response_metadata(
    metadata: &Option<dapi_grpc::platform::v0::ResponseMetadata>,
) {
    if let Some(metadata) = metadata {
        info!("Response metadata:");
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

pub(super) fn print_proof_info(proof: &dapi_grpc::platform::v0::Proof) {
    info!("Cryptographic proof details:");
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

pub(super) fn print_error_info(error: &dapi_grpc::platform::v0::StateTransitionBroadcastError) {
    warn!("Error details:");
    warn!("  Code: {}", error.code);
    warn!("  Message: {}", error.message);
    if !error.data.is_empty() {
        warn!("  Data: {}", hex::encode(&error.data));
    }
}
