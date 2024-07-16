mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;

use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// This function calls the versioned `prove_elements`
    /// function based on the version provided in the `PlatformVersion` parameter. It panics if the
    /// version doesn't match any existing versioned functions.
    ///
    /// # Parameters
    /// - `path`: The path at which we want to prove the elements
    /// - `keys`: The keys that we want to prove
    /// - `transaction`: An optional grovedb transaction
    /// - `platform_version`: A reference to the [PlatformVersion] object that specifies the version of
    ///   the function to call.
    ///
    /// # Returns
    /// Returns a `Result` with a `Vec<u8>` containing the proof data if the function succeeds,
    /// or an `Error` if the function fails.
    pub fn prove_elements(
        &self,
        path: Vec<Vec<u8>>,
        keys: Vec<Vec<u8>>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        match platform_version.drive.methods.prove.prove_elements {
            0 => self.prove_elements_v0(path, keys, transaction, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "prove_elements".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
