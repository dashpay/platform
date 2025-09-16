use clap::Args;
use dapi_grpc::core::v0::{
    BlockHeadersWithChainLocksRequest, block_headers_with_chain_locks_request::FromBlock,
    core_client::CoreClient,
};
use dapi_grpc::tonic::transport::Channel;
use tracing::{info, warn};

use crate::error::{CliError, CliResult};

#[derive(Args, Debug)]
pub struct ChainLocksCommand {
    /// Optional starting block height for historical context
    #[arg(long)]
    pub from_height: Option<u32>,
}

pub async fn run(url: &str, cmd: ChainLocksCommand) -> CliResult<()> {
    info!(url = %url, "Connecting to DAPI Core gRPC for chain locks");

    let channel = Channel::from_shared(url.to_string())
        .map_err(|source| CliError::InvalidUrl {
            url: url.to_string(),
            source: Box::new(source),
        })?
        .connect()
        .await?;
    let mut client = CoreClient::new(channel);

    let request = BlockHeadersWithChainLocksRequest {
        count: 0,
        from_block: cmd.from_height.map(FromBlock::FromBlockHeight),
    };

    println!("📡 Subscribing to chain locks at {}", url);
    if let Some(height) = cmd.from_height {
        println!(
            "   Requesting history starting from block height {}",
            height
        );
    } else {
        println!("   Streaming live chain locks\n");
    }

    let response = client
        .subscribe_to_block_headers_with_chain_locks(request)
        .await?;

    let mut stream = response.into_inner();
    let mut block_header_batches = 0usize;
    let mut chain_locks = 0usize;

    while let Some(message) = stream.message().await? {
        use dapi_grpc::core::v0::block_headers_with_chain_locks_response::Responses;

        match message.responses {
            Some(Responses::BlockHeaders(headers)) => {
                block_header_batches += 1;
                let header_count = headers.headers.len();
                let total_bytes: usize = headers.headers.iter().map(|h| h.len()).sum();
                println!(
                    "🧱 Received block headers batch #{} ({} header(s), {} bytes)",
                    block_header_batches, header_count, total_bytes
                );
            }
            Some(Responses::ChainLock(data)) => {
                chain_locks += 1;
                println!(
                    "🔒 Received chain lock #{}, payload size {} bytes",
                    chain_locks,
                    data.len()
                );
            }
            None => {
                warn!("Received empty chain lock response message");
            }
        }
        println!();
    }

    println!("👋 Chain lock stream ended");
    Ok(())
}
