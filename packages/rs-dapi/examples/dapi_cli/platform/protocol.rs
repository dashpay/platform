use clap::Args;
use dapi_grpc::platform::v0::get_status_response::GetStatusResponseV0;
use dapi_grpc::platform::v0::{
    GetProtocolVersionUpgradeStateRequest,
    GetProtocolVersionUpgradeVoteStatusRequest,
    GetStatusRequest,
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

pub async fn run_get_status(url: &str) -> CliResult<()> {
    let channel = Channel::from_shared(url.to_string()).map_err(|source| CliError::InvalidUrl {
        url: url.to_string(),
        source: Box::new(source),
    })?;
    let mut client = PlatformClient::connect(channel).await?;

    let request = GetStatusRequest {
        version: Some(dapi_grpc::platform::v0::get_status_request::Version::V0(
            dapi_grpc::platform::v0::get_status_request::GetStatusRequestV0 {},
        )),
    };

    let response = client.get_status(Request::new(request)).await?.into_inner();

    let Some(dapi_grpc::platform::v0::get_status_response::Version::V0(v0)) = response.version
    else {
        return Err(CliError::EmptyResponse("getStatus"));
    };

    print_status(&v0);
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

fn print_status(status: &GetStatusResponseV0) {
    if let Some(version) = &status.version {
        println!("📦 Software Versions:");
        if let Some(software) = &version.software {
            println!("  dapi: {}", software.dapi);
            if let Some(drive) = &software.drive {
                println!("  drive: {}", drive);
            }
            if let Some(tenderdash) = &software.tenderdash {
                println!("  tenderdash: {}", tenderdash);
            }
        }
        if let Some(protocol) = &version.protocol {
            if let Some(td) = &protocol.tenderdash {
                println!("🔄 Tenderdash protocol: p2p={}, block={}", td.p2p, td.block);
            }
            if let Some(drive) = &protocol.drive {
                println!(
                    "🔄 Drive protocol: current={} latest={}",
                    drive.current, drive.latest
                );
            }
        }
        println!();
    }

    if let Some(node) = &status.node {
        println!("🖥️  Node Information:");
        if !node.id.is_empty() {
            println!("  id: {}", hex::encode_upper(&node.id));
        }
        if let Some(protx) = &node.pro_tx_hash {
            println!("  proTxHash: {}", hex::encode_upper(protx));
        }
        println!();
    }

    if let Some(chain) = &status.chain {
        println!("⛓️  Chain Info:");
        println!("  catching_up: {}", chain.catching_up);
        println!("  latest_block_height: {}", chain.latest_block_height);
        println!("  max_peer_block_height: {}", chain.max_peer_block_height);
        if let Some(cclh) = chain.core_chain_locked_height {
            println!("  core_chain_locked_height: {}", cclh);
        }
        if !chain.latest_block_hash.is_empty() {
            println!(
                "  latest_block_hash: {}",
                hex::encode_upper(&chain.latest_block_hash)
            );
        }
        println!();
    }

    if let Some(network) = &status.network {
        println!("🌐 Network:");
        println!("  chain_id: {}", network.chain_id);
        println!("  peers_count: {}", network.peers_count);
        println!("  listening: {}", network.listening);
        println!();
    }

    if let Some(state_sync) = &status.state_sync {
        println!("🔁 State Sync:");
        println!("  total_synced_time: {}", state_sync.total_synced_time);
        println!("  remaining_time: {}", state_sync.remaining_time);
        println!("  total_snapshots: {}", state_sync.total_snapshots);
        println!(
            "  chunk_process_avg_time: {}",
            state_sync.chunk_process_avg_time
        );
        println!("  snapshot_height: {}", state_sync.snapshot_height);
        println!(
            "  snapshot_chunks_count: {}",
            state_sync.snapshot_chunks_count
        );
        println!("  backfilled_blocks: {}", state_sync.backfilled_blocks);
        println!(
            "  backfill_blocks_total: {}",
            state_sync.backfill_blocks_total
        );
        println!();
    }

    if let Some(time) = &status.time {
        println!("🕒 Time:");
        println!("  local: {}", time.local);
        if let Some(block) = time.block {
            println!("  block: {}", block);
        }
        if let Some(genesis) = time.genesis {
            println!("  genesis: {}", genesis);
        }
        if let Some(epoch) = time.epoch {
            println!("  epoch: {}", epoch);
        }
        println!();
    }
}

fn print_proof(proof: &dapi_grpc::platform::v0::Proof) {
    println!("🔐 Proof received:");
    println!("    quorum_hash: {}", hex::encode_upper(&proof.quorum_hash));
    println!("    signature bytes: {}", proof.signature.len());
    println!("    grovedb_proof bytes: {}", proof.grovedb_proof.len());
    println!("    round: {}", proof.round);
}
