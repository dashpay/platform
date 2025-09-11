use dapi_grpc::core::v0::{
    core_client::CoreClient, transactions_with_proofs_request::FromBlock,
    TransactionsWithProofsRequest,
};
use std::env;
use tonic::transport::Channel;
use tracing::{info, warn};
use tracing_subscriber::fmt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    fmt::init();

    // Parse command line arguments
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <dapi-grpc-url>", args[0]);
        eprintln!("Example: {} http://localhost:3005", args[0]);
        std::process::exit(1);
    }

    let dapi_url = &args[1];

    info!("Connecting to DAPI gRPC at: {}", dapi_url);

    // Connect to gRPC service
    let channel = Channel::from_shared(dapi_url.to_string())?
        .connect()
        .await?;

    let mut client = CoreClient::new(channel);

    // Create the subscription request
    let request = TransactionsWithProofsRequest {
        bloom_filter: None,                              // No bloom filter for now
        from_block: Some(FromBlock::FromBlockHeight(1)), // Start from block height 1
        count: 0, // 0 means stream continuously (both historical and new)
        send_transaction_hashes: false, // We want full transaction data, not just hashes
    };

    println!("🚀 Connected to DAPI gRPC at {}", dapi_url);
    println!("📡 Subscribing to transaction stream...");
    println!("Press Ctrl+C to exit\n");

    // Subscribe to the transaction stream
    let response = client.subscribe_to_transactions_with_proofs(request).await;

    let mut stream = match response {
        Ok(response) => response.into_inner(),
        Err(e) => {
            eprintln!("❌ Failed to subscribe to transaction stream: {}", e);
            std::process::exit(1);
        }
    };

    // Process incoming transaction events
    let mut transaction_count = 0;
    let mut merkle_block_count = 0;
    let mut instant_lock_count = 0;

    while let Some(response) = stream.message().await? {
        match response.responses {
            Some(dapi_grpc::core::v0::transactions_with_proofs_response::Responses::RawTransactions(raw_txs)) => {
                transaction_count += raw_txs.transactions.len();
                println!("📦 Received {} transaction(s) (total: {})", 
                    raw_txs.transactions.len(),
                    transaction_count
                );

                for (i, tx_data) in raw_txs.transactions.iter().enumerate() {
                    // Calculate a simple hash representation for display
                    let hash_preview = if tx_data.len() >= 8 {
                        format!("{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}", 
                            tx_data[0], tx_data[1], tx_data[2], tx_data[3],
                            tx_data[4], tx_data[5], tx_data[6], tx_data[7])
                    } else {
                        "short_tx".to_string()
                    };

                    println!("   📝 Transaction {}: {} bytes (preview: {}...)", 
                        i + 1, tx_data.len(), hash_preview);
                }
            }
            Some(dapi_grpc::core::v0::transactions_with_proofs_response::Responses::RawMerkleBlock(merkle_block)) => {
                merkle_block_count += 1;
                println!("🌳 Received Merkle Block #{} ({} bytes)", 
                    merkle_block_count,
                    merkle_block.len()
                );

                // Calculate block header hash preview for identification
                let block_preview = if merkle_block.len() >= 8 {
                    format!("{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}", 
                        merkle_block[0], merkle_block[1], merkle_block[2], merkle_block[3],
                        merkle_block[4], merkle_block[5], merkle_block[6], merkle_block[7])
                } else {
                    "short_block".to_string()
                };

                println!("   🔗 Block preview: {}... ({} bytes)", block_preview, merkle_block.len());
            }
            Some(dapi_grpc::core::v0::transactions_with_proofs_response::Responses::InstantSendLockMessages(locks)) => {
                instant_lock_count += locks.messages.len();
                println!("⚡ Received {} InstantSend lock(s) (total: {})", 
                    locks.messages.len(),
                    instant_lock_count
                );

                for (i, lock_data) in locks.messages.iter().enumerate() {
                    println!("   InstantLock {}: {} bytes", i + 1, lock_data.len());
                }
            }
            None => {
                warn!("⚠️  Received empty response from stream");
            }
        }

        println!(); // Empty line for better readability
    }

    println!("👋 Stream ended, shutting down transaction monitor");
    Ok(())
}
