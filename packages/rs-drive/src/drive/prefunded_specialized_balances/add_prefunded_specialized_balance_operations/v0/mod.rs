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
use dpp::balances::credits::MAX_CREDITS;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::batch::{KeyInfoPath, QualifiedGroveDbOp};
use grovedb::{Element, EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// The operations to add to the specialized balance
    #[inline(always)]
    pub(super) fn add_prefunded_specialized_balance_operations_v0(
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
        let path_holding_specialized_balances = prefunded_specialized_balances_for_voting_path();
        let previous_credits_in_specialized_balance = self
            .grove_get_raw_value_u64_from_encoded_var_vec(
                (&path_holding_specialized_balances).into(),
                specialized_balance_id.as_slice(),
                DirectQueryType::StatefulDirectQuery,
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

    /// Creates an existing specialized balance at `amount` credits.
    fn seed_balance(
        drive: &crate::drive::Drive,
        id: Identifier,
        amount: u64,
        platform_version: &PlatformVersion,
    ) {
        drive
            .add_prefunded_specialized_balance(id, amount, None, platform_version)
            .expect("seed balance");
    }

    #[test]
    fn new_balance_uses_insert_or_replace_op_and_creates_entry() {
        // First-time insert branch: previous balance is None, so the op must
        // be an insert_or_replace_op and the fetched balance must equal `amount`.
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let id = Identifier::from([7u8; 32]);

        let mut estimated = None;
        let ops = drive
            .add_prefunded_specialized_balance_operations_v0(
                id,
                1_000,
                &mut estimated,
                None,
                platform_version,
            )
            .expect("create op for new balance");
        // At least one operation (the GroveDB insert op); the preceding read may
        // push additional cost operations into the drive_operations vec.
        assert!(!ops.is_empty());

        // Apply the operations that the function actually returned (rather
        // than going through the seed helper) so the insert branch's
        // GroveDB write is what we verify.
        let mut drive_ops = vec![];
        drive
            .apply_batch_low_level_drive_operations(
                None,
                None,
                ops,
                &mut drive_ops,
                &platform_version.drive,
            )
            .expect("apply add ops");

        let fetched = drive
            .fetch_prefunded_specialized_balance(id.to_buffer(), None, platform_version)
            .expect("fetch balance");
        assert_eq!(fetched, Some(1_000));
    }

    #[test]
    fn existing_balance_accumulates_across_two_adds() {
        // Second add exercises the `had_previous_balance == true` branch, and
        // the replace_op path inside v0.
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let id = Identifier::from([8u8; 32]);

        seed_balance(&drive, id, 500, platform_version);
        seed_balance(&drive, id, 250, platform_version);

        let fetched = drive
            .fetch_prefunded_specialized_balance(id.to_buffer(), None, platform_version)
            .expect("fetch balance");
        assert_eq!(fetched, Some(750));
    }

    #[test]
    fn add_amount_causing_u64_overflow_returns_critical_corrupted_state() {
        // `previous + amount` overflows u64 when previous = u64::MAX - 1 and amount = 2,
        // so `checked_add` returns None and we expect CriticalCorruptedState error.
        // We synthesize this by first using direct deduction: seed to max_credits - 1, then
        // attempt to add enough to overflow. MAX_CREDITS = i64::MAX (< u64::MAX) so the
        // `>= MAX_CREDITS` check triggers first for amounts in the valid range; we instead
        // verify the overflow branch by a pure-computation simulation using u64::MAX.
        // Here we exercise the MAX_CREDITS overflow branch (more realistic in the wild):
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let id = Identifier::from([9u8; 32]);

        // Seed a balance that is exactly MAX_CREDITS - 1.
        seed_balance(&drive, id, MAX_CREDITS - 1, platform_version);

        // Adding 1 more pushes new_total to MAX_CREDITS which triggers the
        // `>= MAX_CREDITS` guard and returns CriticalBalanceOverflow.
        let mut estimated = None;
        let err = drive
            .add_prefunded_specialized_balance_operations_v0(
                id,
                1,
                &mut estimated,
                None,
                platform_version,
            )
            .expect_err("expected overflow guard to trip");
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
    fn add_amount_causing_checked_add_overflow_is_critical_corrupted_state() {
        // Here we ask for an amount that would cause checked_add itself to
        // return None. u64::MAX added to any non-zero existing balance does this.
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let id = Identifier::from([10u8; 32]);

        // Seed 1 credit, then add u64::MAX.
        seed_balance(&drive, id, 1, platform_version);

        let mut estimated = None;
        let err = drive
            .add_prefunded_specialized_balance_operations_v0(
                id,
                u64::MAX,
                &mut estimated,
                None,
                platform_version,
            )
            .expect_err("expected checked_add overflow error");
        assert!(
            matches!(err, Error::Drive(DriveError::CriticalCorruptedState(_))),
            "expected CriticalCorruptedState, got: {:?}",
            err
        );
    }

    #[test]
    fn estimation_path_produces_estimation_entries() {
        // When estimated_costs_only_with_layer_info is Some(_), add_estimation_costs_* must be
        // invoked and populate the map even for a fresh (non-existent) identifier.
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let id = Identifier::from([11u8; 32]);

        let mut estimated = Some(std::collections::HashMap::new());
        let _ops = drive
            .add_prefunded_specialized_balance_operations_v0(
                id,
                42,
                &mut estimated,
                None,
                platform_version,
            )
            .expect("create op with estimation");

        let map = estimated.expect("estimated map present");
        assert!(
            !map.is_empty(),
            "expected add_estimation_costs_* to populate layer info"
        );
    }

    #[test]
    fn add_zero_to_nonexistent_creates_zero_entry() {
        // An add of 0 to a non-existent balance takes the insert_or_replace branch
        // and must not error. checked_add(0) is always Some, new_total = 0 < MAX_CREDITS.
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let id = Identifier::from([12u8; 32]);

        let mut estimated = None;
        let ops = drive
            .add_prefunded_specialized_balance_operations_v0(
                id,
                0,
                &mut estimated,
                None,
                platform_version,
            )
            .expect("zero add");

        // Apply the emitted ops and confirm the entry exists with value 0 —
        // this verifies the insert_or_replace_op was emitted correctly rather
        // than merely that *some* ops were produced.
        let mut drive_ops = vec![];
        drive
            .apply_batch_low_level_drive_operations(
                None,
                None,
                ops,
                &mut drive_ops,
                &platform_version.drive,
            )
            .expect("apply zero add ops");

        let fetched = drive
            .fetch_prefunded_specialized_balance(id.to_buffer(), None, platform_version)
            .expect("fetch balance");
        assert_eq!(fetched, Some(0));
    }
}
