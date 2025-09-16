use clap::Args;
use dapi_grpc::platform::v0::{
    GetProtocolVersionUpgradeStateRequest,
    GetProtocolVersionUpgradeVoteStatusRequest,
    platform_client::PlatformClient,
    get_protocol_version_upgrade_state_request,
    get_protocol_version_upgrade_state_request::GetProtocolVersionUpgradeStateRequestV0,
    get_protocol_version_upgrade_state_response,
    get_protocol_version_upgrade_state_response::get_protocol_version_upgrade_state_response_v0::Result as UpgradeStateResult,
    get_protocol_version_upgrade_state_response::get_protocol_version_upgrade_state_response_v0::Versions,
    get_protocol_version_upgrade_vote_status_request,
    get_protocol_version_upgrade_vote_status_request::GetProtocolVersionUpgradeVoteStatusRequestV0,
    get_protocol_version_upgrade_vote_status_response,
    get_protocol_version_upgrade_vote_status_response::get_protocol_version_upgrade_vote_status_response_v0::Result as VoteStatusResult,
    get_protocol_version_upgrade_vote_status_response::get_protocol_version_upgrade_vote_status_response_v0::VersionSignals,
};
use dapi_grpc::tonic::{Request, transport::Channel};
use tracing::info;

use crate::error::{CliError, CliResult};

#[derive(Args, Debug)]
pub struct UpgradeStateCommand {
    /// Request cryptographic proof alongside the state information
    #[arg(long, default_value_t = false)]
    pub prove: bool,
}

pub async fn run_upgrade_state(url: &str, cmd: UpgradeStateCommand) -> CliResult<()> {
    info!(
        prove = cmd.prove,
        "Requesting protocol version upgrade state"
    );

    let channel = Channel::from_shared(url.to_string()).map_err(|source| CliError::InvalidUrl {
        url: url.to_string(),
        source: Box::new(source),
    })?;
    let mut client = PlatformClient::connect(channel).await?;

    let request = GetProtocolVersionUpgradeStateRequest {
        version: Some(get_protocol_version_upgrade_state_request::Version::V0(
            GetProtocolVersionUpgradeStateRequestV0 { prove: cmd.prove },
        )),
    };

    let response = client
        .get_protocol_version_upgrade_state(Request::new(request))
        .await?
        .into_inner();

    let Some(get_protocol_version_upgrade_state_response::Version::V0(v0)) = response.version
    else {
        return Err(CliError::EmptyResponse("getProtocolVersionUpgradeState"));
    };

    print_metadata(v0.metadata.as_ref());

    match v0.result {
        Some(UpgradeStateResult::Versions(Versions { versions })) => {
            if versions.is_empty() {
                println!("ℹ️  No protocol version entries returned");
            } else {
                println!("📊 Protocol version entries ({}):", versions.len());
                for entry in versions {
                    println!(
                        "  • version {} => {} vote(s)",
                        entry.version_number, entry.vote_count
                    );
                }
            }
        }
        Some(UpgradeStateResult::Proof(proof)) => {
            print_proof(&proof);
        }
        None => println!("ℹ️  Response did not include version information"),
    }

    Ok(())
}

#[derive(Args, Debug)]
pub struct UpgradeVoteStatusCommand {
    /// Optional starting ProTx hash (hex) for pagination
    #[arg(long, value_name = "HEX")]
    pub start_pro_tx_hash: Option<String>,
    /// Maximum number of vote entries to return (0 means default server limit)
    #[arg(long, default_value_t = 0)]
    pub count: u32,
    /// Request cryptographic proof alongside the vote information
    #[arg(long, default_value_t = false)]
    pub prove: bool,
}

pub async fn run_upgrade_vote_status(url: &str, cmd: UpgradeVoteStatusCommand) -> CliResult<()> {
    info!(
        prove = cmd.prove,
        count = cmd.count,
        "Requesting protocol version upgrade vote status"
    );

    let start_pro_tx_hash = if let Some(ref hash) = cmd.start_pro_tx_hash {
        hex::decode(hash).map_err(|source| CliError::InvalidHash {
            hash: hash.clone(),
            source,
        })?
    } else {
        Vec::new()
    };

    let channel = Channel::from_shared(url.to_string()).map_err(|source| CliError::InvalidUrl {
        url: url.to_string(),
        source: Box::new(source),
    })?;
    let mut client = PlatformClient::connect(channel).await?;

    let request = GetProtocolVersionUpgradeVoteStatusRequest {
        version: Some(
            get_protocol_version_upgrade_vote_status_request::Version::V0(
                GetProtocolVersionUpgradeVoteStatusRequestV0 {
                    start_pro_tx_hash,
                    count: cmd.count,
                    prove: cmd.prove,
                },
            ),
        ),
    };

    let response = client
        .get_protocol_version_upgrade_vote_status(Request::new(request))
        .await?
        .into_inner();

    let Some(get_protocol_version_upgrade_vote_status_response::Version::V0(v0)) = response.version
    else {
        return Err(CliError::EmptyResponse(
            "getProtocolVersionUpgradeVoteStatus",
        ));
    };

    print_metadata(v0.metadata.as_ref());

    match v0.result {
        Some(VoteStatusResult::Versions(VersionSignals { version_signals })) => {
            if version_signals.is_empty() {
                println!("ℹ️  No vote status entries returned");
            } else {
                println!("🗳️  Vote status entries ({}):", version_signals.len());
                for signal in version_signals {
                    let pro_tx_hash = hex::encode_upper(signal.pro_tx_hash);
                    println!(
                        "  • proTxHash {} => version {}",
                        pro_tx_hash, signal.version
                    );
                }
            }
        }
        Some(VoteStatusResult::Proof(proof)) => {
            print_proof(&proof);
        }
        None => println!("ℹ️  Response did not include vote status information"),
    }

    Ok(())
}

fn print_metadata(metadata: Option<&dapi_grpc::platform::v0::ResponseMetadata>) {
    if let Some(meta) = metadata {
        println!("ℹ️  Metadata:");
        println!("    height: {}", meta.height);
        println!(
            "    core_chain_locked_height: {}",
            meta.core_chain_locked_height
        );
        println!("    epoch: {}", meta.epoch);
        println!("    protocol_version: {}", meta.protocol_version);
        println!("    chain_id: {}", meta.chain_id);
        println!("    time_ms: {}", meta.time_ms);
    }
}

fn print_proof(proof: &dapi_grpc::platform::v0::Proof) {
    println!("🔐 Proof received:");
    println!("    quorum_hash: {}", hex::encode_upper(&proof.quorum_hash));
    println!("    signature bytes: {}", proof.signature.len());
    println!("    grovedb_proof bytes: {}", proof.grovedb_proof.len());
    println!("    round: {}", proof.round);
}
