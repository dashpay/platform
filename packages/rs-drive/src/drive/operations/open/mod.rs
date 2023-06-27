mod v0;

use std::path::Path;
use std::sync::RwLock;
use dpp::version::drive_versions::DriveVersion;
use grovedb::GroveDb;
use crate::drive::cache::{DataContractCache, DriveCache};
use crate::drive::config::DriveConfig;
use crate::drive::Drive;
use crate::drive::system_contracts_cache::SystemContracts;
use crate::error::{Error, DriveError};

impl Drive {
    /// Opens a path in GroveDB.
    ///
    /// This is a versioned method which opens a specified path as a GroveDB instance and returns a `Drive`
    /// instance with this GroveDB, cache and other configurations.
    ///
    /// # Arguments
    ///
    /// * `path` - A reference that implements the `AsRef<Path>` trait. This represents the path to the GroveDB.
    /// * `config` - An `Option` which contains `DriveConfig`. If not specified, default configuration is used.
    /// * `drive_version` - A `DriveVersion` reference that dictates which version of the method to call.
    ///
    /// # Returns
    ///
    /// * `Result<Self, Error>` - On success, returns `Ok(Self)`, where `Self` is a `Drive` instance. On error, returns an `Error`.
    ///
    pub(super) fn open<P: AsRef<Path>>(path: P, config: Option<DriveConfig>, drive_version: &DriveVersion) -> Result<Self, Error> {
        match drive_version.methods.operations.open {
            0 => Self::open_v0(path, config),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "open".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}