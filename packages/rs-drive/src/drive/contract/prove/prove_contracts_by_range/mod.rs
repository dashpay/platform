mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;

use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Proves one page of the contract enumeration (`getDataContractsByRange`).
    ///
    /// Contracts are paged in ascending contract id order. With `ids_only` the proof
    /// covers only the contract subtrees keyed by id; otherwise it carries the serialized
    /// contracts, whichever storage layout (plain or history-keeping) each contract uses.
    ///
    /// # Arguments
    ///
    /// * `start_at` - Optional cursor: the contract id to start from and whether it is
    ///   included (`true` = `startAt`, `false` = `startAfter`). `None` proves the first page.
    /// * `limit` - Maximum number of contracts in the page.
    /// * `ids_only` - Prove the contract ids only, without contract bytes.
    /// * `transaction` - The transaction to prove against.
    /// * `platform_version` - The platform version for version dispatch.
    ///
    /// # Returns
    ///
    /// * `Result<Vec<u8>, Error>` - The proof bytes.
    ///
    /// # Errors
    ///
    /// Returns an error if proof generation fails or the method version is unknown.
    pub fn prove_contracts_by_range(
        &self,
        start_at: Option<([u8; 32], bool)>,
        limit: u16,
        ids_only: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        match platform_version
            .drive
            .methods
            .contract
            .prove
            .prove_contracts_by_range
        {
            0 => self.prove_contracts_by_range_v0(
                start_at,
                limit,
                ids_only,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "prove_contracts_by_range".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
