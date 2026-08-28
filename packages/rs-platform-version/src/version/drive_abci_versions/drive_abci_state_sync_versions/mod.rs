pub mod v1;

use versioned_feature_core::FeatureVersion;

/// Versions for ABCI state sync (snapshot serving and consumption).
#[derive(Clone, Debug, Default)]
pub struct DriveAbciStateSyncVersions {
    /// The grovedb state sync wire protocol version used for snapshots this node
    /// creates and serves. Snapshots offered by peers are validated against the
    /// supported set in `drive-abci`'s snapshot module; bumping to a new grovedb
    /// wire version means adding a new `DriveAbciStateSyncVersions` const here and
    /// extending that supported set.
    pub protocol_version: FeatureVersion,
}
