use crate::drive::shielded::paths::{shielded_credit_pool_path_vec, SHIELDED_TOTAL_BALANCE_KEY};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::fees::op::LowLevelDriveOperation::GroveOperation;
use grovedb::batch::QualifiedGroveDbOp;
use grovedb::Element;

impl Drive {
    /// Version 0 implementation of constructing a total balance update operation.
    ///
    /// Creates an `insert_or_replace_op` that sets the shielded pool's total
    /// balance sum item to the new value.
    pub(in crate::drive) fn update_total_balance_op_v0(
        new_total_balance: u64,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let pool_path = shielded_credit_pool_path_vec();
        let balance_i64 = i64::try_from(new_total_balance).map_err(|_| {
            Error::Drive(DriveError::CorruptedDriveState(
                "shielded pool total balance exceeds i64::MAX".to_string(),
            ))
        })?;
        Ok(vec![GroveOperation(
            QualifiedGroveDbOp::insert_or_replace_op(
                pool_path,
                vec![SHIELDED_TOTAL_BALANCE_KEY],
                Element::new_sum_item(balance_i64),
            ),
        )])
    }
}
