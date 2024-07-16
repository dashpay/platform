use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::verify::RootHash;
use dpp::version::PlatformVersion;
use grovedb::Element;
use std::collections::BTreeMap;

mod v0;

impl Drive {
    /// Verifies a proof containing potentially multiple epoch infos.
    ///
    /// # Parameters
    ///
    /// - `proof`: A byte slice representing the proof to be verified.
    /// - `path`: The path where elements should be.
    /// - `keys`: The requested keyes.
    /// - `platform_version`: the platform version,
    ///
    /// # Returns
    ///
    /// Returns a `Result` with a tuple of `RootHash` and `Vec<Element>`. The `BTreeMap<Vec<u8>, Option<Element>>`
    /// is the map of elements we get back.
    ///
    /// # Errors
    ///
    /// Returns an `Error` if:
    ///
    /// - The proof is corrupted.
    /// - The GroveDb query fails.
    pub fn verify_elements(
        proof: &[u8],
        path: Vec<Vec<u8>>,
        keys: Vec<Vec<u8>>,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, BTreeMap<Vec<u8>, Option<Element>>), Error> {
        match platform_version.drive.methods.verify.system.verify_elements {
            0 => Drive::verify_elements_v0(proof, path, keys, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_elements".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
