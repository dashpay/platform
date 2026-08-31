use crate::abci::AbciError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::snapshot::{
    max_serving_pins, SnapshotManager, MAX_STATE_SYNC_CHUNK_ID_SIZE,
    SUPPORTED_STATE_SYNC_PROTOCOL_VERSIONS,
};
use std::sync::Arc;
use tenderdash_abci::proto::abci as proto;

/// Serves one chunk of a state sync snapshot from the checkpoint registry.
///
/// The served checkpoint is pinned in the snapshot manager so checkpoint pruning cannot
/// delete it from disk while a peer is still downloading it.
///
/// Takes the platform and snapshot manager directly rather than an application trait so
/// the gRPC serving application can run it on the blocking pool from owned `Arc`s.
pub fn load_snapshot_chunk<C>(
    platform: &Platform<C>,
    snapshot_manager: &SnapshotManager,
    request: proto::RequestLoadSnapshotChunk,
) -> Result<proto::ResponseLoadSnapshotChunk, Error> {
    tracing::trace!(
        height = request.height,
        version = request.version,
        chunk_id = hex::encode(&request.chunk_id),
        "[state_sync] api load_snapshot_chunk",
    );

    if !platform.config.abci.state_sync.snapshots_enabled {
        return Err(AbciError::StateSyncBadRequest(
            "load_snapshot_chunk snapshot serving is disabled".to_string(),
        )
        .into());
    }

    // Cap peer-supplied sizes before anything decodes them (issue #3773)
    if request.chunk_id.len() > MAX_STATE_SYNC_CHUNK_ID_SIZE {
        return Err(AbciError::StateSyncBadRequest(format!(
            "load_snapshot_chunk chunk id of {} bytes exceeds the {} byte limit",
            request.chunk_id.len(),
            MAX_STATE_SYNC_CHUNK_ID_SIZE
        ))
        .into());
    }

    let wire_version = u16::try_from(request.version)
        .ok()
        .filter(|version| SUPPORTED_STATE_SYNC_PROTOCOL_VERSIONS.contains(version));
    let Some(wire_version) = wire_version else {
        return Err(AbciError::StateSyncBadRequest(format!(
            "load_snapshot_chunk unsupported state sync protocol version {}, supported: {:?}",
            request.version, SUPPORTED_STATE_SYNC_PROTOCOL_VERSIONS
        ))
        .into());
    };

    // Resolve the checkpoint: from the registry, or — if pruning already dropped it —
    // from the pins of transfers already in flight.
    let checkpoint = platform
        .drive
        .checkpoints
        .load()
        .get(&request.height)
        .map(|checkpoint_info| Arc::clone(&checkpoint_info.checkpoint))
        .or_else(|| snapshot_manager.pinned_checkpoint(request.height))
        .ok_or_else(|| {
            AbciError::StateSyncBadRequest(format!(
                "load_snapshot_chunk no snapshot at height {}",
                request.height
            ))
        })?;

    // Chunks must be generated under the version the checkpoint was WRITTEN at — the same
    // one `list_snapshots` stamped into the snapshot metadata and the consuming node
    // restores under. This node's own current version may have moved on since.
    let snapshot_platform_version = checkpoint
        .platform_version()
        .map_err(|e| {
            AbciError::StateSyncInternalError(format!(
                "load_snapshot_chunk unable to read the protocol version of the checkpoint at \
                 height {}: {}",
                request.height, e
            ))
        })?
        .ok_or_else(|| {
            AbciError::StateSyncInternalError(format!(
                "load_snapshot_chunk checkpoint at height {} has no usable protocol version",
                request.height
            ))
        })?;
    let grove_version = &snapshot_platform_version.drive.grove_version;

    let chunk = checkpoint
        .grove_db
        .fetch_chunk(&request.chunk_id, None, wire_version, grove_version)
        .map_err(|e| {
            AbciError::StateSyncInternalError(format!(
                "load_snapshot_chunk unable to fetch chunk: {}",
                e
            ))
        })?;

    // Pin (or refresh the pin of) the checkpoint only once a chunk was actually served.
    // Pinning before the fetch would let a peer keep a checkpoint — and its directory —
    // alive with a stream of requests that never succeed.
    snapshot_manager.pin_for_serving(
        request.height,
        checkpoint,
        max_serving_pins(platform.config.abci.state_sync.max_num_snapshots),
    );

    Ok(proto::ResponseLoadSnapshotChunk { chunk })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::PlatformConfig;
    use crate::test::helpers::fast_forward_to_block::fast_forward_to_block;
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::version::PlatformVersion;

    #[test]
    fn load_snapshot_chunk_serves_root_chunk_and_rejects_bad_requests() {
        let mut config = PlatformConfig::default_local();
        config.abci.state_sync.snapshots_enabled = true;
        let platform = TestPlatformBuilder::new()
            .with_config(config)
            .build_with_mock_rpc()
            .set_genesis_state();
        let platform_version = PlatformVersion::latest();
        let snapshot_manager = SnapshotManager::new();

        let reduced_platform_state = platform.state.load().to_reduced_platform_state(None, 42);
        platform
            .store_reduced_platform_state(&reduced_platform_state, None, platform_version)
            .expect("should store reduced platform state");
        // Snapshots are served under the checkpoint's OWN protocol version, which a real
        // node always has in aux.
        platform
            .drive
            .store_current_protocol_version(platform_version.protocol_version, None)
            .expect("should store protocol version");

        fast_forward_to_block(&platform, 1_000_000, 10, 42, 0, false);
        platform
            .create_grovedb_checkpoint(platform_version)
            .expect("should create checkpoint");

        let root_hash = platform
            .drive
            .grove
            .root_hash(None, &platform_version.drive.grove_version)
            .unwrap()
            .expect("should get root hash");

        // The root chunk (chunk id == app hash) must be served
        let response = load_snapshot_chunk(
            &platform,
            &snapshot_manager,
            proto::RequestLoadSnapshotChunk {
                height: 10,
                version: 1,
                chunk_id: root_hash.to_vec(),
            },
        )
        .expect("should load root chunk");
        assert!(!response.chunk.is_empty());

        // The served checkpoint must now be pinned against pruning
        assert!(snapshot_manager.pinned_checkpoint(10).is_some());

        // Unknown height is rejected
        assert!(load_snapshot_chunk(
            &platform,
            &snapshot_manager,
            proto::RequestLoadSnapshotChunk {
                height: 999,
                version: 1,
                chunk_id: root_hash.to_vec(),
            },
        )
        .is_err());

        // Unsupported wire version is rejected
        assert!(load_snapshot_chunk(
            &platform,
            &snapshot_manager,
            proto::RequestLoadSnapshotChunk {
                height: 10,
                version: 2,
                chunk_id: root_hash.to_vec(),
            },
        )
        .is_err());

        // Oversized chunk id is rejected before any decoding
        assert!(load_snapshot_chunk(
            &platform,
            &snapshot_manager,
            proto::RequestLoadSnapshotChunk {
                height: 10,
                version: 1,
                chunk_id: vec![0u8; MAX_STATE_SYNC_CHUNK_ID_SIZE + 1],
            },
        )
        .is_err());
    }
}
