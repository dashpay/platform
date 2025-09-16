use clap::Args;
use dapi_grpc::core::v0::{
    TransactionsWithProofsRequest, core_client::CoreClient,
    transactions_with_proofs_request::FromBlock,
};
use dapi_grpc::tonic::transport::Channel;
use tracing::{info, warn};

use crate::error::{CliError, CliResult};

#[derive(Args, Debug)]
pub struct TransactionsCommand {
    /// Starting block height for historical streaming
    #[arg(long, default_value_t = 1)]
    pub from_height: u32,

    /// Send transaction hashes instead of full transactions
    #[arg(long, default_value_t = false)]
    pub hashes_only: bool,
}

pub async fn run(url: &str, cmd: TransactionsCommand) -> CliResult<()> {
    info!(url = %url, "Connecting to DAPI Core gRPC");

    let channel = Channel::from_shared(url.to_string())
        .map_err(|source| CliError::InvalidUrl {
            url: url.to_string(),
            source: Box::new(source),
        })?
        .connect()
        .await?;
    let mut client = CoreClient::new(channel);

    let request = TransactionsWithProofsRequest {
        bloom_filter: None,
        from_block: Some(FromBlock::FromBlockHeight(cmd.from_height)),
        count: 0,
        send_transaction_hashes: cmd.hashes_only,
    };

    println!("📡 Subscribing to transactions with proofs from {}", url);
    println!("   Starting from block height {}", cmd.from_height);
    if cmd.hashes_only {
        println!("   Streaming transaction hashes only\n");
    } else {
        println!("   Streaming full transaction payloads\n");
    }

    let response = client
        .subscribe_to_transactions_with_proofs(request)
        .await?;
    let mut stream = response.into_inner();

    let mut transaction_count = 0usize;
    let mut merkle_block_count = 0usize;
    let mut instant_lock_count = 0usize;

    while let Some(response) = stream.message().await? {
        match response.responses {
            Some(dapi_grpc::core::v0::transactions_with_proofs_response::Responses::RawTransactions(raw_txs)) => {
                transaction_count += raw_txs.transactions.len();
                println!(
                    "📦 Received {} transaction(s) (total: {})",
                    raw_txs.transactions.len(),
                    transaction_count
                );

                if !cmd.hashes_only {
                    for (i, tx_data) in raw_txs.transactions.iter().enumerate() {
                        let hash_preview = hash_preview(tx_data);
                        println!(
                            "   📝 Transaction {}: {} bytes (preview: {}...)",
                            i + 1,
                            tx_data.len(),
                            hash_preview
                        );
                    }
                }
            }
            Some(dapi_grpc::core::v0::transactions_with_proofs_response::Responses::RawMerkleBlock(merkle_block)) => {
                merkle_block_count += 1;
                println!(
                    "🌳 Received Merkle Block #{} ({} bytes)",
                    merkle_block_count,
                    merkle_block.len()
                );

                println!(
                    "   🔗 Block preview: {}...",
                    hash_preview(&merkle_block)
                );
            }
            Some(dapi_grpc::core::v0::transactions_with_proofs_response::Responses::InstantSendLockMessages(locks)) => {
                instant_lock_count += locks.messages.len();
                println!(
                    "⚡ Received {} InstantSend lock(s) (total: {})",
                    locks.messages.len(),
                    instant_lock_count
                );

                for (i, lock_data) in locks.messages.iter().enumerate() {
                    println!("   InstantLock {}: {} bytes", i + 1, lock_data.len());
                }
            }
            other => {
                warn!(?other, "Received unexpected transactions response variant");
            }
        }

        println!();
    }

    println!("👋 Stream ended");
    Ok(())
}

fn hash_preview(data: &[u8]) -> String {
    if data.len() >= 8 {
        format!(
            "{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
            data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7]
        )
    } else {
        "short".to_string()
    }
}
