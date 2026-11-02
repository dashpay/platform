mod v0;
mod v1;
mod v2;

use crate::util::batch::DriveOperation;

use crate::drive::Drive;
use crate::error::{drive::DriveError, Error};

use dpp::block::block_info::BlockInfo;

use dpp::fee::default_costs::CachedEpochIndexFeeVersions;
use dpp::fee::fee_result::FeeResult;
use grovedb::TransactionArg;

use dpp::version::PlatformVersion;

impl Drive {
    /// Applies a list of high level DriveOperations to the drive, and calculates the fee for them.
    ///
    /// This method converts Drive operations to low-level operations, applies them if `apply` is true,
    /// and calculates the associated fee.
    ///
    /// # Arguments
    ///
    /// * `operations` - A vector of `DriveOperation`s to apply to the drive.
    /// * `apply` - A boolean flag indicating whether to apply the changes or only estimate costs.
    /// * `block_info` - A reference to information about the current block.
    /// * `transaction` - Transaction arguments.
    /// * `drive_version` - A `DriveVersion` reference that dictates which version of the method to call.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing the `FeeResult` if the operations are successfully applied,
    /// otherwise an `Error`.
    ///
    pub fn apply_drive_operations(
        &self,
        operations: Vec<DriveOperation>,
        apply: bool,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
        previous_fee_versions: Option<&CachedEpochIndexFeeVersions>,
    ) -> Result<FeeResult, Error> {
        match platform_version
            .drive
            .methods
            .batch_operations
            .apply_drive_operations
        {
            0 => self.apply_drive_operations_v0(
                operations,
                apply,
                block_info,
                transaction,
                platform_version,
                previous_fee_versions,
            ),
            1 => self.apply_drive_operations_v1(
                operations,
                apply,
                block_info,
                transaction,
                platform_version,
                previous_fee_versions,
            ),
            2 => self.apply_drive_operations_v2(
                operations,
                apply,
                block_info,
                transaction,
                platform_version,
                previous_fee_versions,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "apply_drive_operations".to_string(),
                known_versions: vec![0, 1, 2],
                received: version,
            })),
        }
    }
}

/// Operations that describe shared layers differently, and the tests of how each generation
/// estimates them, on both sides of protocol version 15.
#[cfg(test)]
mod tests {
    use crate::drive::Drive;
    use crate::error::Error;
    use crate::util::batch::drive_op_batch::{
        PrefundedSpecializedBalanceOperationType, TokenOperationType,
    };
    use crate::util::batch::{DriveOperation, IdentityOperationType, SystemOperationType};
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::fee::fee_result::FeeResult;
    use dpp::identifier::Identifier;
    use dpp::version::PlatformVersion;

    /// A write that describes the root layer shallowly (an identity contract nonce, level 1).
    pub(super) fn shallow() -> DriveOperation<'static> {
        DriveOperation::IdentityOperation(IdentityOperationType::UpdateIdentityContractNonce {
            identity_id: [1; 32],
            contract_id: [2; 32],
            nonce: 3,
        })
    }

    /// A write that describes the root layer deeper (a prefunded specialized balance, level 3).
    pub(super) fn deep() -> DriveOperation<'static> {
        DriveOperation::PrefundedSpecializedBalanceOperation(
            PrefundedSpecializedBalanceOperationType::CreateNewPrefundedBalance {
                prefunded_specialized_balance_id: Identifier::from([4; 32]),
                add_balance: 1_000,
            },
        )
    }

    /// A token mint: its supply estimator describes the root at level 3 and `Misc` as holding
    /// the token supplies tree, then its balance estimator describes the root at level 2.
    pub(super) fn mint() -> DriveOperation<'static> {
        DriveOperation::TokenOperation(TokenOperationType::TokenMint {
            token_id: Identifier::from([5; 32]),
            identity_balance_holder_id: Identifier::from([6; 32]),
            mint_amount: 100,
            allow_first_mint: true,
            allow_saturation: false,
        })
    }

    /// A system credits update. Its estimator describes `Misc` as holding the total credits item
    /// alone before protocol version 15, and with its subtrees from protocol version 15.
    pub(super) fn system_credits() -> DriveOperation<'static> {
        DriveOperation::SystemOperation(SystemOperationType::AddToSystemCredits { amount: 5 })
    }

    pub(super) fn estimate(
        drive: &Drive,
        operations: Vec<DriveOperation<'static>>,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        drive.apply_drive_operations(
            operations,
            false,
            &BlockInfo::default(),
            None,
            platform_version,
            None,
        )
    }

    fn generations() -> (&'static PlatformVersion, &'static PlatformVersion) {
        (
            PlatformVersion::get(14).expect("expected protocol version 14"),
            PlatformVersion::latest(),
        )
    }

    /// A lone token mint: protocol version 14 prices its root at the balance estimator's level
    /// 2, written last; from protocol version 15 the root is priced at its supply's level 3.
    #[test]
    fn should_price_a_lone_mint_at_its_supply_depth_from_protocol_version_15() {
        let (before, latest) = generations();
        let drive = setup_drive_with_initial_state_structure(Some(latest));
        let estimated_before =
            estimate(&drive, vec![mint()], before).expect("expected an estimate");
        let estimated_latest =
            estimate(&drive, vec![mint()], latest).expect("expected an estimate");
        assert!(estimated_latest.processing_fee > estimated_before.processing_fee);
    }

    /// Protocol version 14 estimates a batch by whichever write describes a shared layer last,
    /// so the order of its operations matters; from protocol version 15 it does not, and the
    /// estimate is at least either order's at protocol version 14.
    #[test]
    fn should_estimate_a_batch_by_its_order_only_before_protocol_version_15() {
        let (before, latest) = generations();
        let drive = setup_drive_with_initial_state_structure(Some(latest));
        let shallow_first_before =
            estimate(&drive, vec![shallow(), deep()], before).expect("expected an estimate");
        let deep_first_before =
            estimate(&drive, vec![deep(), shallow()], before).expect("expected an estimate");
        assert_ne!(shallow_first_before, deep_first_before);

        let shallow_first =
            estimate(&drive, vec![shallow(), deep()], latest).expect("expected an estimate");
        let deep_first =
            estimate(&drive, vec![deep(), shallow()], latest).expect("expected an estimate");
        assert_eq!(shallow_first, deep_first);
        assert!(
            shallow_first.processing_fee
                >= shallow_first_before
                    .processing_fee
                    .max(deep_first_before.processing_fee)
        );
    }

    /// At protocol version 14 a mint then a system credits update leaves `Misc` described as
    /// the credits item alone, and GroveDB refuses to price the token supplies tree's new root
    /// key there without subtree sizes; the other order is estimated. From protocol version 15
    /// both orders are estimated, alike.
    #[test]
    fn should_refuse_misc_described_last_as_an_item_only_before_protocol_version_15() {
        let (before, latest) = generations();
        let drive = setup_drive_with_initial_state_structure(Some(latest));
        let error = estimate(&drive, vec![mint(), system_credits()], before)
            .expect_err("expected the estimate to be refused");
        assert!(
            error
                .to_string()
                .contains("this layer does not have costs for trees"),
            "unexpected error {error}"
        );
        estimate(&drive, vec![system_credits(), mint()], before).expect("expected an estimate");

        assert_eq!(
            estimate(&drive, vec![mint(), system_credits()], latest).expect("expected an estimate"),
            estimate(&drive, vec![system_credits(), mint()], latest).expect("expected an estimate")
        );
    }
}
