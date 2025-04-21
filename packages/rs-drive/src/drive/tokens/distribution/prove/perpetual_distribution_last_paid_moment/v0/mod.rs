use crate::drive::tokens::paths::token_perpetual_distributions_identity_last_claimed_time_path_vec;
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use grovedb::{PathQuery, TransactionArg};

use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;

impl Drive {
    /// Produces a GroveDB proof for the **last perpetual‑distribution claim** of a
    /// given `identity_id` under `token_id` (version 0).
    ///
    /// The proof covers the single element stored at
    /// `token_perpetual_distributions_identity_last_claimed_time_path(token_id) / identity_id`.
    ///
    /// # Parameters
    /// * `token_id` – 32‑byte token identifier.
    /// * `identity_id` – identity whose last claim we prove.
    /// * `drive_operations` – accumulator for low‑level operations.
    /// * `transaction` – GroveDB transaction.
    /// * `platform_version` – version selector (only 0 handled here).
    ///
    /// # Returns
    /// * `Ok(Vec<u8>)` – proof bytes (Merkle path + signatures, Drive format).
    /// * `Err(_)` – any retrieval or version‑mismatch error.
    pub(super) fn prove_perpetual_distribution_last_paid_moment_operations_v0(
        &self,
        token_id: [u8; 32],
        identity_id: Identifier,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let path = token_perpetual_distributions_identity_last_claimed_time_path_vec(token_id);
        let path_query = PathQuery::new_single_key(path, identity_id.to_vec());

        self.grove_get_proved_path_query(
            &path_query,
            transaction,
            drive_operations,
            &platform_version.drive,
        )
    }
}
