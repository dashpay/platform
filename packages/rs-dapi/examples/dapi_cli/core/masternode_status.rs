use clap::Args;
use dapi_grpc::core::v0::{
    GetMasternodeStatusRequest, core_client::CoreClient,
    get_masternode_status_response::Status as GrpcStatus,
};
use dapi_grpc::tonic::transport::Channel;

use crate::error::{CliError, CliResult};

#[derive(Args, Debug)]
pub struct MasternodeStatusCommand {}

pub async fn run(url: &str, _cmd: MasternodeStatusCommand) -> CliResult<()> {
    let channel = Channel::from_shared(url.to_string())
        .map_err(|source| CliError::InvalidUrl {
            url: url.to_string(),
            source: Box::new(source),
        })?
        .connect()
        .await?;

    let mut client = CoreClient::new(channel);

    let response = client
        .get_masternode_status(GetMasternodeStatusRequest {})
        .await?
        .into_inner();

    let status = GrpcStatus::try_from(response.status).unwrap_or(GrpcStatus::Unknown);
    let pro_tx_hash = if response.pro_tx_hash.is_empty() {
        "<unset>".to_string()
    } else {
        hex::encode(response.pro_tx_hash)
    };

    println!("Masternode status via {}", url);
    println!("Status        : {}", human_status(status));
    println!("ProTx Hash    : {}", pro_tx_hash);
    println!("PoSe Penalty  : {}", response.pose_penalty);
    println!("Core Synced   : {}", yes_no(response.is_synced));
    println!("Sync Progress : {:.2}%", response.sync_progress * 100.0);

    Ok(())
}

fn human_status(status: GrpcStatus) -> &'static str {
    match status {
        GrpcStatus::Unknown => "Unknown",
        GrpcStatus::WaitingForProtx => "Waiting for ProTx",
        GrpcStatus::PoseBanned => "PoSe banned",
        GrpcStatus::Removed => "Removed",
        GrpcStatus::OperatorKeyChanged => "Operator key changed",
        GrpcStatus::ProtxIpChanged => "ProTx IP changed",
        GrpcStatus::Ready => "Ready",
        GrpcStatus::Error => "Error",
    }
}

fn yes_no(flag: bool) -> &'static str {
    if flag { "yes" } else { "no" }
}
