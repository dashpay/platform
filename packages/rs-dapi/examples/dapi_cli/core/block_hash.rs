use clap::Args;
use dapi_grpc::core::v0::{GetBlockRequest, core_client::CoreClient};
use dapi_grpc::tonic::transport::Channel;
use tracing::info;

use crate::error::{CliError, CliResult};

#[derive(Args, Debug)]
pub struct BlockHashCommand {
    /// Block height to query (>= 1)
    #[arg(long)]
    pub height: u32,
}

pub async fn run(url: &str, cmd: BlockHashCommand) -> CliResult<()> {
    if cmd.height < 1 {
        return Err(
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "height must be >= 1").into(),
        );
    }

    info!(url = %url, height = cmd.height, "Querying block hash");

    let channel = Channel::from_shared(url.to_string())
        .map_err(|source| CliError::InvalidUrl {
            url: url.to_string(),
            source: Box::new(source),
        })?
        .connect()
        .await?;
    let mut client = CoreClient::new(channel);

    let request = GetBlockRequest {
        block: Some(dapi_grpc::core::v0::get_block_request::Block::Height(
            cmd.height,
        )),
    };

    let response = client.get_block(request).await?;
    let block_bytes = response.into_inner().block;

    // Deserialize and compute hash
    use dashcore_rpc::dashcore::Block;
    use dashcore_rpc::dashcore::consensus::encode::deserialize;

    let block: Block = match deserialize(&block_bytes) {
        Ok(b) => b,
        Err(e) => {
            tracing::error!(block_bytes = hex::encode(&block_bytes), error = %e, "Failed to deserialize block");
            return Err(CliError::DashCoreEncoding(e));
        }
    };
    let block_json = serde_json::to_string_pretty(&block)?;
    let hash_hex = block.block_hash().to_string();

    println!("Block {} hash: {}\n{}\n", cmd.height, hash_hex, block_json);
    Ok(())
}
