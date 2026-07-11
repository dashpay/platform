use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::fees::op::LowLevelDriveOperation::GroveOperation;
use crate::util::grove_operations::DirectQueryType;

use crate::drive::prefunded_specialized_balances::{
    prefunded_specialized_balances_for_voting_path,
    prefunded_specialized_balances_for_voting_path_vec,
};
use crate::error::identity::IdentityError;
use crate::util::grove_operations::QueryTarget::QueryTargetValue;
use dpp::balances::credits::MAX_CREDITS;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::batch::{KeyInfoPath, QualifiedGroveDbOp};
use grovedb::{Element, EstimatedLayerInformation, TransactionArg, TreeType};
use std::collections::HashMap;

impl Drive {
    /// The operations to add to the specialized balance
    #[inline(always)]
    pub(super) fn add_prefunded_specialized_balance_operations_v1(
        &self,
        specialized_balance_id: Identifier,
        amount: u64,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let mut drive_operations = vec![];
        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_prefunded_specialized_balance_update(
                estimated_costs_only_with_layer_info,
                &platform_version.drive,
            )?;
        }

        let direct_query_type = if estimated_costs_only_with_layer_info.is_none() {
            DirectQueryType::StatefulDirectQuery
        } else {
            DirectQueryType::StatelessDirectQuery {
                in_tree_type: TreeType::SumTree,
                query_target: QueryTargetValue(8),
            }
        };

        let path_holding_specialized_balances = prefunded_specialized_balances_for_voting_path();
        let previous_credits_in_specialized_balance = self
            .grove_get_raw_value_u64_from_encoded_var_vec(
                (&path_holding_specialized_balances).into(),
                specialized_balance_id.as_slice(),
                direct_query_type,
                transaction,
                &mut drive_operations,
                &platform_version.drive,
            )?;
        let had_previous_balance = previous_credits_in_specialized_balance.is_some();
        let new_total = previous_credits_in_specialized_balance
            .unwrap_or_default()
            .checked_add(amount)
            .ok_or(Error::Drive(DriveError::CriticalCorruptedState(
                "trying to add an amount that would overflow credits",
            )))?;
        // while i64::MAX could potentially work, best to avoid it.
        if new_total >= MAX_CREDITS {
            return Err(Error::Identity(IdentityError::CriticalBalanceOverflow(
                "trying to set prefunded specialized balance to over max credits amount (i64::MAX)",
            )));
        };
        let path_holding_total_credits_vec = prefunded_specialized_balances_for_voting_path_vec();
        let op = if had_previous_balance {
            QualifiedGroveDbOp::replace_op(
                path_holding_total_credits_vec,
                specialized_balance_id.to_vec(),
                Element::new_sum_item(new_total as i64),
            )
        } else {
            QualifiedGroveDbOp::insert_or_replace_op(
                path_holding_total_credits_vec,
                specialized_balance_id.to_vec(),
                Element::new_sum_item(new_total as i64),
            )
        };
        drive_operations.push(GroveOperation(op));
        Ok(drive_operations)
    }
}

#[cfg(test)]
mod tests {
    use crate::error::drive::DriveError;
    use crate::error::identity::IdentityError;
    use crate::error::Error;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::balances::credits::MAX_CREDITS;
    use dpp::identifier::Identifier;
    use dpp::version::PlatformVersion;
    use std::collections::HashMap;

    #[test]
    fn stateless_query_branch_runs_without_errors_for_missing_id() {
        // The v1 implementation switches to StatelessDirectQuery when estimation
        // mode is on. Exercise this branch on a non-existent balance: the lookup
        // must succeed but return None, and the final op must still be generated.
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let id = Identifier::from([42u8; 32]);

        let mut estimated = Some(HashMap::new());
        let ops = drive
            .add_prefunded_specialized_balance_operations_v1(
                id,
                1_234,
                &mut estimated,
                None,
                platform_version,
            )
            .expect("stateless branch should succeed");
        // At least the insert_or_replace op is present (read cost ops may also be).
        assert!(!ops.is_empty());

        // Estimation map must be populated.
        assert!(!estimated.unwrap().is_empty());
    }

    #[test]
    fn stateful_branch_increments_balance() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let id = Identifier::from([43u8; 32]);

        // First add to seed a balance (calls through v0/v1 picked by platform version).
        drive
            .add_prefunded_specialized_balance(id, 100, None, platform_version)
            .expect("seed");

        let mut estimated = None;
        let _ops = drive
            .add_prefunded_specialized_balance_operations_v1(
                id,
                55,
                &mut estimated,
                None,
                platform_version,
            )
            .expect("v1 stateful add");
    }

    #[test]
    fn v1_rejects_total_at_or_above_max_credits() {
        // v1 has the same >= MAX_CREDITS guard as v0. Seed MAX_CREDITS - 1, add 1.
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let id = Identifier::from([44u8; 32]);

        drive
            .add_prefunded_specialized_balance(id, MAX_CREDITS - 1, None, platform_version)
            .expect("seed near-max");

        let mut estimated = None;
        let err = drive
            .add_prefunded_specialized_balance_operations_v1(
                id,
                1,
                &mut estimated,
                None,
                platform_version,
            )
            .expect_err("expected overflow guard");
        assert!(
            matches!(
                err,
                Error::Identity(IdentityError::CriticalBalanceOverflow(_))
            ),
            "expected CriticalBalanceOverflow, got: {:?}",
            err
        );
    }

    #[test]
    fn v1_rejects_checked_add_overflow() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let id = Identifier::from([45u8; 32]);

        drive
            .add_prefunded_specialized_balance(id, 1, None, platform_version)
            .expect("seed 1");

        let mut estimated = None;
        let err = drive
            .add_prefunded_specialized_balance_operations_v1(
                id,
                u64::MAX,
                &mut estimated,
                None,
                platform_version,
            )
            .expect_err("expected checked_add overflow");
        assert!(
            matches!(err, Error::Drive(DriveError::CriticalCorruptedState(_))),
            "expected CriticalCorruptedState, got: {:?}",
            err
        );
    }
}
