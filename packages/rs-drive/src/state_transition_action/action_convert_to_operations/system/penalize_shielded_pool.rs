use crate::error::Error;
use crate::state_transition_action::action_convert_to_operations::DriveHighLevelOperationConverter;
use crate::state_transition_action::system::penalize_shielded_pool_action::PenalizeShieldedPoolAction;
use crate::util::batch::drive_op_batch::ShieldedPoolOperationType;
use crate::util::batch::DriveOperation;
use dpp::block::epoch::Epoch;
use dpp::version::PlatformVersion;

impl DriveHighLevelOperationConverter for PenalizeShieldedPoolAction {
    fn into_high_level_drive_operations<'a>(
        self,
        _epoch: &Epoch,
        _platform_version: &PlatformVersion,
    ) -> Result<Vec<DriveOperation<'a>>, Error> {
        match self {
            PenalizeShieldedPoolAction::V0(v0) => {
                let mut ops: Vec<DriveOperation<'a>> = Vec::new();

                // 1. Record nullifiers as spent (prevents replaying the same invalid proof)
                for nullifier in v0.nullifiers.iter() {
                    ops.push(DriveOperation::ShieldedPoolOperation(
                        ShieldedPoolOperationType::InsertNullifier {
                            nullifier: *nullifier,
                        },
                    ));
                }

                // 2. Deduct penalty from pool total balance
                let new_total_balance = v0.current_total_balance.saturating_sub(v0.penalty_amount);
                ops.push(DriveOperation::ShieldedPoolOperation(
                    ShieldedPoolOperationType::UpdateTotalBalance { new_total_balance },
                ));

                Ok(ops)
            }
        }
    }
}
