use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::address_funds::PlatformAddress;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    pub(super) fn prove_balance_and_nonce_v0(
        &self,
        address: &PlatformAddress,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        self.prove_balance_and_nonce_operations_v0(
            address,
            transaction,
            &mut vec![],
            platform_version,
        )
    }

    pub(super) fn prove_balance_and_nonce_operations_v0(
        &self,
        address: &PlatformAddress,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let path_query = Drive::balance_for_clear_address_query(address);
        self.grove_get_proved_path_query(
            &path_query,
            transaction,
            drive_operations,
            &platform_version.drive,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::batch::drive_op_batch::{AddressFundsOperationType, DriveOperation};
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::fee::Credits;
    use dpp::prelude::AddressNonce;

    const ADDR_A: PlatformAddress = PlatformAddress::P2pkh([42; 20]);
    const ADDR_MISSING: PlatformAddress = PlatformAddress::P2pkh([201; 20]);

    fn insert_balance(drive: &Drive, addr: PlatformAddress, nonce: AddressNonce, balance: Credits) {
        let platform_version = PlatformVersion::latest();
        let ops = vec![DriveOperation::AddressFundsOperation(
            AddressFundsOperationType::SetBalanceToAddress {
                address: addr,
                nonce,
                balance,
            },
        )];
        drive
            .apply_drive_operations(
                ops,
                true,
                &BlockInfo::default(),
                None,
                platform_version,
                None,
            )
            .expect("apply");
    }

    /// Prove-and-verify for an existing address returns the exact balance/nonce.
    #[test]
    fn prove_single_existing_address_round_trips() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        insert_balance(&drive, ADDR_A, 11, 9_999_999);

        let proof = drive
            .prove_balance_and_nonce_v0(&ADDR_A, None, platform_version)
            .expect("prove");
        assert!(!proof.is_empty());

        // Round-trip via the verifier: single-address proof exposes the row.
        let (_, result) =
            Drive::verify_address_info(proof.as_slice(), &ADDR_A, false, platform_version)
                .expect("verify");
        let (nonce, balance) = result.expect("should have value");
        assert_eq!(nonce, 11);
        assert_eq!(balance, 9_999_999);
    }

    /// prove_balance_and_nonce_operations_v0 populates drive_operations with at
    /// least one entry tracking the proof cost.
    #[test]
    fn prove_operations_populates_drive_operations() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        insert_balance(&drive, ADDR_A, 1, 100);

        let mut ops = vec![];
        let proof = drive
            .prove_balance_and_nonce_operations_v0(&ADDR_A, None, &mut ops, platform_version)
            .expect("prove ops");
        assert!(!proof.is_empty());
        assert!(!ops.is_empty(), "drive_operations should track the proof");
    }

    /// Proving a non-existent address produces an absence proof that verifies
    /// cleanly: the proof parses via `Drive::verify_address_info`, the root
    /// hash is not empty, and the returned balance/nonce is `None`.
    #[test]
    fn prove_nonexistent_address_returns_absence_proof() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        // Populate with a different address so the tree isn't empty.
        insert_balance(&drive, ADDR_A, 2, 50);

        let proof = drive
            .prove_balance_and_nonce_v0(&ADDR_MISSING, None, platform_version)
            .expect("prove missing");
        assert!(!proof.is_empty());

        // Verify the absence proof round-trips: known-absent address must
        // decode as `None` and must produce a non-empty root hash.
        let (root_hash, result) =
            Drive::verify_address_info(proof.as_slice(), &ADDR_MISSING, false, platform_version)
                .expect("absence proof should verify");
        assert!(!root_hash.is_empty(), "root hash should not be empty");
        assert!(
            result.is_none(),
            "absent address must decode as None, got {:?}",
            result
        );
    }

    /// Attempting to prove from within a transaction is explicitly not
    /// supported by GroveDB's prove path. This pins that behavior so a future
    /// change that silently ignores the transaction is caught.
    #[test]
    fn prove_within_transaction_returns_not_supported() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        insert_balance(&drive, ADDR_A, 1, 100);

        let transaction = drive.grove.start_transaction();
        let err = drive
            .prove_balance_and_nonce_v0(&ADDR_A, Some(&transaction), platform_version)
            .expect_err("prove in tx must error");
        // The exact error variant is GroveDB NotSupported; we just ensure it does
        // not silently succeed (which would hide a serious correctness bug).
        let _ = err;
    }
}
