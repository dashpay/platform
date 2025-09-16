use ciborium::de::from_reader;
use clap::Args;
use dapi_grpc::core::v0::{MasternodeListRequest, core_client::CoreClient};
use dapi_grpc::tonic::transport::Channel;
use serde::Deserialize;
use serde_json::Value;
use std::io::Cursor;
use tracing::warn;

use crate::error::{CliError, CliResult};

#[derive(Args, Debug)]
pub struct MasternodeCommand {}

pub async fn run(url: &str, _cmd: MasternodeCommand) -> CliResult<()> {
    let channel = Channel::from_shared(url.to_string())
        .map_err(|source| CliError::InvalidUrl {
            url: url.to_string(),
            source: Box::new(source),
        })?
        .connect()
        .await?;

    let mut client = CoreClient::new(channel);

    println!("📡 Subscribing to masternode list updates at {}", url);

    let response = client
        .subscribe_to_masternode_list(MasternodeListRequest {})
        .await?;

    let mut stream = response.into_inner();
    let mut update_index = 0usize;

    while let Some(update) = stream.message().await? {
        update_index += 1;
        let diff_bytes = update.masternode_list_diff;

        println!("🔁 Masternode list update #{}", update_index);
        println!("   Diff payload size: {} bytes", diff_bytes.len());

        match from_reader::<MasternodeListDiff, _>(Cursor::new(&diff_bytes)) {
            Ok(diff) => print_diff_summary(&diff),
            Err(err) => {
                warn!(error = %err, "Failed to decode masternode diff payload");
                println!("   Unable to decode diff payload (see logs for details).\n");
                continue;
            }
        }

        println!();
    }

    println!("👋 Stream ended");
    Ok(())
}

fn print_diff_summary(diff: &MasternodeListDiff) {
    let base_hash = diff.base_block_hash.as_deref().unwrap_or("<unknown>");
    let block_hash = diff.block_hash.as_deref().unwrap_or("<unknown>");

    println!("   Base block hash : {}", base_hash);
    println!("   Target block hash: {}", block_hash);

    let added = diff.added_mns.len();
    let updated = diff.updated_mns.len();
    let removed = diff.removed_mns.len();

    if added > 0 || updated > 0 || removed > 0 {
        println!(
            "   Added: {} | Updated: {} | Removed: {}",
            added, updated, removed
        );
    }

    let snapshot = if !diff.full_list.is_empty() {
        diff.full_list.len()
    } else if !diff.masternode_list.is_empty() {
        diff.masternode_list.len()
    } else {
        0
    };

    if snapshot > 0 {
        println!("   Snapshot size: {} masternodes", snapshot);
    }

    if let Some(total) = diff.total_mn_count {
        println!("   Reported total masternodes: {}", total);
    }

    let quorum_updates = diff.quorum_diff_updates();
    if quorum_updates > 0 {
        println!("   Quorum updates: {}", quorum_updates);
    }

    if added == 0 && updated == 0 && removed == 0 && snapshot == 0 && quorum_updates == 0 {
        println!(
            "   No masternode or quorum changes detected in this diff (metadata update only)."
        );
    }
}

#[derive(Debug, Deserialize)]
struct MasternodeListDiff {
    #[serde(rename = "baseBlockHash")]
    base_block_hash: Option<String>,
    #[serde(rename = "blockHash")]
    block_hash: Option<String>,
    #[serde(rename = "addedMNs", default)]
    added_mns: Vec<Value>,
    #[serde(rename = "updatedMNs", default)]
    updated_mns: Vec<Value>,
    #[serde(rename = "removedMNs", default)]
    removed_mns: Vec<Value>,
    #[serde(rename = "mnList", default)]
    full_list: Vec<Value>,
    #[serde(rename = "masternodeList", default)]
    masternode_list: Vec<Value>,
    #[serde(rename = "totalMnCount")]
    total_mn_count: Option<u64>,
    #[serde(rename = "quorumDiffs", default)]
    quorum_diffs: Vec<QuorumDiffEntry>,
    #[serde(rename = "newQuorums", default)]
    new_quorums: Vec<Value>,
    #[serde(rename = "deletedQuorums", default)]
    deleted_quorums: Vec<Value>,
    #[serde(default)]
    quorums: Vec<Value>,
}

impl MasternodeListDiff {
    fn quorum_diff_updates(&self) -> usize {
        let nested: usize = self
            .quorum_diffs
            .iter()
            .map(|entry| entry.quorum_updates())
            .sum();

        nested + self.new_quorums.len() + self.deleted_quorums.len() + self.quorums.len()
    }
}

#[derive(Debug, Deserialize)]
struct QuorumDiffEntry {
    #[serde(rename = "newQuorums", default)]
    new_quorums: Vec<Value>,
    #[serde(rename = "deletedQuorums", default)]
    deleted_quorums: Vec<Value>,
    #[serde(default)]
    quorums: Vec<Value>,
}

impl QuorumDiffEntry {
    fn quorum_updates(&self) -> usize {
        self.new_quorums.len() + self.deleted_quorums.len() + self.quorums.len()
    }
}
