use crate::drive::identity::IdentityDriveQuery;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::query::SingleDocumentDriveQuery;

use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

mod v0;

impl Drive {
    /// This function calls the versioned `prove_multiple`
    /// function based on the version provided in the `DriveVersion` parameter. It panics if the
    /// version doesn't match any existing versioned functions.
    ///
    /// # Parameters
    /// - `identity_queries`: A list of [IdentityDriveQuery]. These specify the identities
    ///   to be proven.
    /// - `contract_ids`: A list of Data Contract IDs to prove
    /// - `document_queries`: A list of [SingleDocumentDriveQuery]. These specify the documents
    ///   to be proven.
    /// - `transaction`: An optional grovedb transaction
    /// - `drive_version`: A reference to the [DriveVersion] object that specifies the version of
    ///   the function to call.
    ///
    /// # Returns
    /// Returns a `Result` with a `Vec<u8>` containing the proof data if the function succeeds,
    /// or an `Error` if the function fails.
    pub fn prove_multiple(
        &self,
        identity_queries: &Vec<IdentityDriveQuery>,
        contract_ids: &[[u8; 32]],
        document_queries: &Vec<SingleDocumentDriveQuery>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        match platform_version.drive.methods.prove.prove_multiple {
            0 => self.prove_multiple_v0(
                identity_queries,
                contract_ids,
                document_queries,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "prove_multiple".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
