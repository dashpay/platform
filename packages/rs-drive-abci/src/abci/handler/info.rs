use crate::abci::app::PlatformApplication;
use crate::abci::AbciError;
use crate::error::Error;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::rpc::core::CoreRPCLike;
use dpp::dashcore::Network;
use dpp::version::DESIRED_PLATFORM_VERSION;
use tenderdash_abci::proto::abci as proto;

pub fn info<A, C>(app: &A, request: proto::RequestInfo) -> Result<proto::ResponseInfo, Error>
where
    A: PlatformApplication<C>,
    C: CoreRPCLike,
{
    if !tenderdash_abci::check_version(&request.abci_version) {
        return Err(AbciError::AbciVersionMismatch {
            tenderdash: request.abci_version,
            drive: tenderdash_abci::proto::ABCI_VERSION.to_string(),
        }
        .into());
    }

    let platform_state = app.platform().state.load();

    let block_height = platform_state.last_committed_block_height();

    if tracing::enabled!(tracing::Level::TRACE) {
        tracing::trace!(
            block_height,
            platform_state = ?platform_state,
            "state_info"
        );
    }

    let last_block_height = platform_state.last_committed_block_height() as i64;

    // Verify that Platform State corresponds to Drive commited state
    let platform_state_app_hash = platform_state
        .last_committed_block_app_hash()
        .unwrap_or_default();

    let grove_version = &platform_state
        .current_platform_version()?
        .drive
        .grove_version;

    let drive_storage_root_hash = app
        .platform()
        .drive
        .grove
        .root_hash(None, grove_version)
        .unwrap()?;

    // We had a sequence of errors on the mainnet started since block 32326.
    // We got RocksDB's "transaction is busy" error because of a bug (https://github.com/dashpay/platform/pull/2309).
    // Due to another bug in Tenderdash (https://github.com/dashpay/tenderdash/pull/966),
    // validators just proceeded to the next block partially committing the state and updating the cache.
    // Full nodes are stuck and proceeded after re-sync.
    // For the mainnet chain, we enable these fixes at the block when we consider the state is consistent.
    let config = &app.platform().config;

    #[allow(clippy::collapsible_if)]
    if !(config.network == Network::Dash
        && config.abci.chain_id == "evo1"
        && last_block_height < 33000)
    {
        // App hash in memory must be equal to app hash on disk
        if drive_storage_root_hash != platform_state_app_hash {
            // We panic because we can't recover from this situation.
            // Better to restart the Drive, so we might self-heal the node
            // reloading state form the disk
            panic!(
                "drive and platform state app hash mismatch (info): drive_storage_root_hash: {:?}, platform_state_app_hash: {:?}",
                drive_storage_root_hash, platform_state_app_hash
            );
        }
    }

    let desired_protocol_version = DESIRED_PLATFORM_VERSION.protocol_version;

    let response = proto::ResponseInfo {
        data: "".to_string(),
        app_version: desired_protocol_version as u64,
        last_block_height,
        version: env!("CARGO_PKG_VERSION").to_string(),
        last_block_app_hash: platform_state_app_hash.to_vec(),
    };

    tracing::debug!(
        desired_protocol_version,
        software_version = env!("CARGO_PKG_VERSION"),
        block_version = request.block_version,
        p2p_version = request.p2p_version,
        app_hash = hex::encode(platform_state_app_hash),
        last_block_height,
        "Handshake with consensus engine",
    );

    if tracing::enabled!(tracing::Level::TRACE) {
        tracing::trace!(
            platform_state_fingerprint = hex::encode(platform_state.fingerprint()?),
            "platform runtime state",
        );
    }

    Ok(response)
}
