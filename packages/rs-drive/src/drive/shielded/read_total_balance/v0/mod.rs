use crate::drive::shielded::paths::{shielded_credit_pool_path, SHIELDED_TOTAL_BALANCE_KEY};
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::DirectQueryType;
use dpp::fee::Credits;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Version 0 implementation of reading the shielded pool total balance.
    ///
    /// Reads the total balance from `[AddressBalances, "s", [5]]`.
    /// Returns 0 if the key does not exist yet.
    pub(in crate::drive) fn read_shielded_pool_total_balance_v0(
        &self,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Credits, Error> {
        let pool_path = shielded_credit_pool_path();
        Ok(self
            .grove_get_raw_value_u64_from_encoded_var_vec(
                (&pool_path).into(),
                &[SHIELDED_TOTAL_BALANCE_KEY],
                DirectQueryType::StatefulDirectQuery,
                transaction,
                drive_operations,
                &platform_version.drive,
            )?
            .unwrap_or(0))
    }
}
