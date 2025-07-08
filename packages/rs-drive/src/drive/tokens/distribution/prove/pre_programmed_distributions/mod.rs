mod v0;

use crate::drive::tokens::distribution::queries::QueryPreProgrammedDistributionStartAt;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Proves the pre‑programmed distributions for a token, using the appropriate versioned method.
    ///
    /// This method queries the pre‑programmed distributions tree at the path
    /// `token_pre_programmed_distributions_path_vec(token_id)`. It constructs a nested mapping where:
    ///
    /// - **Outer keys:** Timestamps (`TimestampMillis`) representing each distribution time.
    /// - **Inner keys:** Recipient identifiers (`Identifier`).
    /// - **Values:** Token amounts (`TokenAmount`).
    ///
    /// The method dispatches to the correct versioned implementation based on the `platform_version`.
    ///
    /// # Parameters
    ///
    /// - `token_id`: The 32‑byte identifier for the token.
    /// - `transaction`: The current GroveDB transaction.
    /// - `platform_version`: The platform version to determine the method variant.
    ///
    /// # Returns
    ///
    /// A `Result` containing a nested `BTreeMap` on success or an `Error` on failure.
    pub fn prove_token_pre_programmed_distributions(
        &self,
        token_id: [u8; 32],
        start_at: Option<QueryPreProgrammedDistributionStartAt>,
        limit: Option<u16>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        self.prove_token_pre_programmed_distributions_operations(
            token_id,
            start_at,
            limit,
            &mut vec![],
            transaction,
            platform_version,
        )
    }
    /// Proves the pre‑programmed distributions for a token, using the appropriate versioned method.
    ///
    /// This method queries the pre‑programmed distributions tree at the path
    /// `token_pre_programmed_distributions_path_vec(token_id)`. It constructs a nested mapping where:
    ///
    /// - **Outer keys:** Timestamps (`TimestampMillis`) representing each distribution time.
    /// - **Inner keys:** Recipient identifiers (`Identifier`).
    /// - **Values:** Token amounts (`TokenAmount`).
    ///
    /// The method dispatches to the correct versioned implementation based on the `platform_version`.
    ///
    /// # Parameters
    ///
    /// - `token_id`: The 32‑byte identifier for the token.
    /// - `drive_operations`: A mutable vector to accumulate low-level drive operations.
    /// - `transaction`: The current GroveDB transaction.
    /// - `platform_version`: The platform version to determine the method variant.
    ///
    /// # Returns
    ///
    /// A `Result` containing a nested `BTreeMap` on success or an `Error` on failure.
    pub(crate) fn prove_token_pre_programmed_distributions_operations(
        &self,
        token_id: [u8; 32],
        start_at: Option<QueryPreProgrammedDistributionStartAt>,
        limit: Option<u16>,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        match platform_version
            .drive
            .methods
            .token
            .prove
            .pre_programmed_distributions
        {
            0 => self.prove_token_pre_programmed_distributions_operations_v0(
                token_id,
                start_at,
                limit,
                drive_operations,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "prove_pre_programmed_distributions_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
