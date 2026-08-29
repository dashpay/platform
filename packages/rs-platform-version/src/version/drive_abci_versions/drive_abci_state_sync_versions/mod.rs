pub mod v1;

use versioned_feature_core::FeatureVersion;

/// Versions for ABCI state sync (snapshot serving and consumption).
#[derive(Clone, Debug, Default)]
pub struct DriveAbciStateSyncVersions {
    /// The grovedb state sync protocol version used for snapshots this node creates
    /// and serves. Exactly one version exists (grovedb updates its replication
    /// protocol in place and stays at version 1); snapshots offered by peers are
    /// validated against the supported set in `drive-abci`'s snapshot module so any
    /// future incompatible protocol change fails fast on both sides.
    pub protocol_version: FeatureVersion,
}
