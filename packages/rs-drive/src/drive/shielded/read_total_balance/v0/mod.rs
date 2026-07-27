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
    /// Reads the total balance from `[AddressBalances, "s", [32]]`.
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

#[cfg(test)]
mod tests {
    use crate::drive::Drive;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::version::PlatformVersion;

    #[test]
    fn read_total_balance_returns_zero_on_fresh_pool() {
        // Fresh initial state inserts SumItem(0) at the total balance key, so
        // read must return 0 (exercises Some(0) path, not unwrap_or).
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let mut drive_ops = vec![];

        let bal = drive
            .read_shielded_pool_total_balance_v0(None, &mut drive_ops, platform_version)
            .expect("read fresh total");
        assert_eq!(bal, 0);
    }

    #[test]
    fn update_then_read_total_balance_roundtrips() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        // Apply an update op moving the total balance to a specific value.
        let ops =
            Drive::update_total_balance_op(123_456_789, platform_version).expect("build update op");
        let grove_ops =
            crate::fees::op::LowLevelDriveOperation::grovedb_operations_batch_consume(ops);
        drive
            .grove_apply_batch_with_add_costs(
                grove_ops,
                false,
                None,
                &mut vec![],
                &platform_version.drive,
            )
            .expect("apply update op");

        let mut drive_ops = vec![];
        let bal = drive
            .read_shielded_pool_total_balance_v0(None, &mut drive_ops, platform_version)
            .expect("read updated total");
        assert_eq!(bal, 123_456_789);
    }
}
