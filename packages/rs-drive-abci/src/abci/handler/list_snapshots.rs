use crate::abci::app::PlatformApplication;
use crate::abci::AbciError;
use crate::error::Error;
use crate::platform_types::snapshot::encode_snapshot_metadata;
use crate::rpc::core::CoreRPCLike;
use dpp::version::PlatformVersion;
use tenderdash_abci::proto::abci as proto;

/// Lists the state sync snapshots this node can serve.
///
/// Snapshots are the rocksdb checkpoints Drive already keeps (`drive.checkpoints`).
/// Only checkpoints that contain the reduced platform state are offered: a checkpoint
/// taken before the protocol version that introduced it (v15) cannot be restored, since
/// a state-synced node would have no way to reconstruct its platform state.
pub fn list_snapshots<A, C>(
    app: &A,
    _request: proto::RequestListSnapshots,
) -> Result<proto::ResponseListSnapshots, Error>
where
    A: PlatformApplication<C>,
    C: CoreRPCLike,
{
    tracing::trace!("[state_sync] api list_snapshots called");

    if !app.platform().config.abci.state_sync.snapshots_enabled {
        return Ok(Default::default());
    }

    let checkpoints = app.platform().drive.checkpoints.load();

    let mut snapshots = Vec::new();
    for (height, checkpoint_info) in checkpoints.iter() {
        let checkpoint = &checkpoint_info.checkpoint;

        // Read the checkpoint under the version IT was written at, not this node's
        // current one: a node that has since upgraded still serves older checkpoints, and
        // grovedb's tree opening and root-hash rules are version gated. The same version
        // is stamped into the snapshot metadata so the consuming node — which has no way
        // to derive it — restores under exactly these rules.
        let Some(snapshot_protocol_version) =
            checkpoint.current_protocol_version().map_err(|e| {
                AbciError::StateSyncInternalError(format!(
                    "list_snapshots unable to read the protocol version of the checkpoint at \
                     height {}: {}",
                    height, e
                ))
            })?
        else {
            continue;
        };
        let Ok(snapshot_platform_version) = PlatformVersion::get(snapshot_protocol_version) else {
            continue;
        };
        let grove_version = &snapshot_platform_version.drive.grove_version;

        let restorable = checkpoint
            .has_reduced_platform_state(grove_version)
            .map_err(|e| {
                AbciError::StateSyncInternalError(format!(
                    "list_snapshots unable to inspect checkpoint at height {}: {}",
                    height, e
                ))
            })?;
        if !restorable {
            continue;
        }

        let root_hash = checkpoint
            .grove_db
            .root_hash(None, grove_version)
            .unwrap()
            .map_err(|e| {
                AbciError::StateSyncInternalError(format!(
                    "list_snapshots unable to get root hash of checkpoint at height {}: {}",
                    height, e
                ))
            })?;

        snapshots.push(proto::Snapshot {
            height: *height,
            version: snapshot_platform_version
                .drive_abci
                .state_sync
                .protocol_version as u32,
            hash: root_hash.to_vec(),
            metadata: encode_snapshot_metadata(snapshot_protocol_version),
        });
    }

    Ok(proto::ResponseListSnapshots { snapshots })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abci::app::FullAbciApplication;
    use crate::config::PlatformConfig;
    use crate::test::helpers::fast_forward_to_block::fast_forward_to_block;
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::version::PlatformVersion;

    fn config_with_snapshots_enabled() -> PlatformConfig {
        let mut config = PlatformConfig::default_local();
        config.abci.state_sync.snapshots_enabled = true;
        config
    }

    #[test]
    fn list_snapshots_returns_nothing_when_serving_is_disabled() {
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();
        let app = FullAbciApplication::new(&platform);

        let response = list_snapshots(&app, Default::default()).expect("should list snapshots");
        assert!(response.snapshots.is_empty());
    }

    #[test]
    fn list_snapshots_serves_only_checkpoints_with_reduced_platform_state() {
        let platform = TestPlatformBuilder::new()
            .with_config(config_with_snapshots_enabled())
            .build_with_mock_rpc()
            .set_genesis_state();
        let platform_version = PlatformVersion::latest();
        let app = FullAbciApplication::new(&platform);

        // A checkpoint taken before the reduced platform state exists (pre-v15
        // activation) is unrestorable and must not be offered.
        fast_forward_to_block(&platform, 1_000_000, 10, 42, 0, false);
        platform
            .create_grovedb_checkpoint(platform_version)
            .expect("should create checkpoint");

        let response = list_snapshots(&app, Default::default()).expect("should list snapshots");
        assert!(
            response.snapshots.is_empty(),
            "checkpoints without the reduced platform state must be filtered out"
        );

        // Once the reduced platform state is in the replicated state, new checkpoints
        // are restorable and must be offered.
        let reduced_platform_state = platform.state.load().to_reduced_platform_state(None, 42);
        platform
            .store_reduced_platform_state(&reduced_platform_state, None, platform_version)
            .expect("should store reduced platform state");

        // A real node always has its protocol version in aux (Drive::open reads it to
        // decide whether there is saved state at all); snapshots are stamped with the
        // checkpoint's own version, so the test platform has to have one too.
        platform
            .drive
            .store_current_protocol_version(platform_version.protocol_version, None)
            .expect("should store protocol version");

        fast_forward_to_block(&platform, 2_000_000, 20, 43, 0, false);
        platform
            .create_grovedb_checkpoint(platform_version)
            .expect("should create checkpoint");

        let response = list_snapshots(&app, Default::default()).expect("should list snapshots");
        assert_eq!(response.snapshots.len(), 1);
        let snapshot = &response.snapshots[0];
        assert_eq!(snapshot.height, 20);
        assert_eq!(
            snapshot.version,
            platform_version.drive_abci.state_sync.protocol_version as u32
        );
        assert_eq!(snapshot.hash.len(), 32);
    }
}
