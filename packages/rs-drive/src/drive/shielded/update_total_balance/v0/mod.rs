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

#[cfg(test)]
mod tests {
    use crate::drive::Drive;
    use crate::error::drive::DriveError;
    use crate::error::Error;

    #[test]
    fn balance_at_i64_max_converts_successfully() {
        // i64::MAX as u64 converts without overflow.
        let ops =
            Drive::update_total_balance_op_v0(i64::MAX as u64).expect("i64::MAX should be valid");
        assert_eq!(ops.len(), 1);
    }

    #[test]
    fn balance_exceeding_i64_max_returns_corrupted_drive_state() {
        // Values > i64::MAX trip the try_from guard and produce CorruptedDriveState.
        let err = Drive::update_total_balance_op_v0(u64::MAX)
            .expect_err("u64::MAX > i64::MAX should fail");
        match err {
            Error::Drive(DriveError::CorruptedDriveState(msg)) => {
                assert!(msg.contains("exceeds i64::MAX"));
            }
            other => panic!("expected CorruptedDriveState, got: {:?}", other),
        }

        let err2 = Drive::update_total_balance_op_v0(i64::MAX as u64 + 1)
            .expect_err("i64::MAX+1 should fail");
        assert!(matches!(
            err2,
            Error::Drive(DriveError::CorruptedDriveState(_))
        ));
    }

    #[test]
    fn balance_zero_produces_single_op() {
        let ops = Drive::update_total_balance_op_v0(0).expect("zero balance");
        assert_eq!(ops.len(), 1);
    }
}
