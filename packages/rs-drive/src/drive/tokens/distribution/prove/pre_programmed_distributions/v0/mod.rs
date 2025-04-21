use crate::drive::tokens::distribution::queries::QueryPreProgrammedDistributionStartAt;
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;

impl Drive {
    /// Fetches the pre‑programmed distributions for a token as a proof.
    ///
    /// This method queries the backing store for the pre‑programmed distributions tree at the path
    /// defined by `token_pre_programmed_distributions_path_vec(token_id)`. It then extracts a nested
    /// mapping where:
    ///
    /// - **Outer keys:** Are timestamps (`TimestampMillis`) representing each distribution time,
    ///   extracted from the 5th path component (index 4). The time is expected to be stored as 4 bytes in big‑endian.
    /// - **Inner keys:** Are recipient identifiers (`Identifier`) derived from the query key.
    /// - **Values:** Are token amounts (`TokenAmount`), extracted from elements that are sum items.
    ///
    /// # Parameters
    ///
    /// - `token_id`: The 32‑byte identifier for the token.
    /// - `drive_operations`: A mutable vector to accumulate low-level drive operations.
    /// - `transaction`: The current GroveDB transaction.
    /// - `platform_version`: The platform version to use.
    ///
    /// # Returns
    ///
    /// A `Result` containing a nested `BTreeMap` on success or an `Error` on failure.
    pub(super) fn prove_token_pre_programmed_distributions_operations_v0(
        &self,
        token_id: [u8; 32],
        start_at: Option<QueryPreProgrammedDistributionStartAt>,
        limit: Option<u16>,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let path_query = Drive::pre_programmed_distributions_query(token_id, start_at, limit);

        self.grove_get_proved_path_query(
            &path_query,
            transaction,
            drive_operations,
            &platform_version.drive,
        )
    }
}
