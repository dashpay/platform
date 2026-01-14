use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::address_funds::PlatformAddress;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    pub(super) fn prove_balances_with_nonces_v0<'a, I>(
        &self,
        addresses: I,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error>
    where
        I: IntoIterator<Item = &'a PlatformAddress>,
    {
        self.prove_balances_with_nonces_operations_v0(
            addresses,
            transaction,
            &mut vec![],
            platform_version,
        )
    }

    pub(super) fn prove_balances_with_nonces_operations_v0<'a, I>(
        &self,
        addresses: I,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error>
    where
        I: IntoIterator<Item = &'a PlatformAddress>,
    {
        let path_query = Drive::balances_for_clear_addresses_query(addresses);
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
    use std::collections::BTreeMap;

    const PLATFORM_ADDRESS_1: PlatformAddress = PlatformAddress::P2pkh([10; 20]);
    const PLATFORM_ADDRESS_2: PlatformAddress = PlatformAddress::P2sh([11; 20]);
    const UNKNOWN_PLATFORM_ADDRESS: PlatformAddress = PlatformAddress::P2pkh([200; 20]);
    const PLATFORM_ADDRESS_1_NONCE: AddressNonce = 5;
    const PLATFORM_ADDRESS_2_NONCE: AddressNonce = 7;
    const PLATFORM_ADDRESS_1_BALANCE: Credits = 1_000_000;
    const PLATFORM_ADDRESS_2_BALANCE: Credits = 2_000_000;

    fn setup_address_balances(drive: &Drive, platform_version: &PlatformVersion) {
        let operations = vec![
            DriveOperation::AddressFundsOperation(AddressFundsOperationType::SetBalanceToAddress {
                address: PLATFORM_ADDRESS_1,
                nonce: PLATFORM_ADDRESS_1_NONCE,
                balance: PLATFORM_ADDRESS_1_BALANCE,
            }),
            DriveOperation::AddressFundsOperation(AddressFundsOperationType::SetBalanceToAddress {
                address: PLATFORM_ADDRESS_2,
                nonce: PLATFORM_ADDRESS_2_NONCE,
                balance: PLATFORM_ADDRESS_2_BALANCE,
            }),
        ];

        drive
            .apply_drive_operations(
                operations,
                true,
                &BlockInfo::default(),
                None,
                platform_version,
                None,
            )
            .expect("expected to apply operations");
    }

    #[test]
    fn should_prove_and_verify_single_address() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        setup_address_balances(&drive, platform_version);

        // Prove single address
        let proof = drive
            .prove_balances_with_nonces_v0(&[PLATFORM_ADDRESS_1], None, platform_version)
            .expect("should prove single address");

        // Verify the proof
        let result: BTreeMap<PlatformAddress, Option<(AddressNonce, Credits)>> =
            Drive::verify_addresses_infos(
                proof.as_slice(),
                &[PLATFORM_ADDRESS_1],
                false,
                platform_version,
            )
            .expect("should verify proof")
            .1;

        assert_eq!(result.len(), 1);
        let (nonce, balance) = result
            .get(&PLATFORM_ADDRESS_1)
            .expect("should have address 1")
            .expect("should have value");
        assert_eq!(nonce, PLATFORM_ADDRESS_1_NONCE);
        assert_eq!(balance, PLATFORM_ADDRESS_1_BALANCE);
    }

    #[test]
    fn should_prove_and_verify_multiple_addresses_existing_only() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        setup_address_balances(&drive, platform_version);

        // Prove only existing addresses
        let addresses = [PLATFORM_ADDRESS_1, PLATFORM_ADDRESS_2];
        let proof = drive
            .prove_balances_with_nonces_v0(addresses.iter(), None, platform_version)
            .expect("should prove multiple addresses");

        // Verify the proof
        let result: BTreeMap<PlatformAddress, Option<(AddressNonce, Credits)>> =
            Drive::verify_addresses_infos(
                proof.as_slice(),
                addresses.iter(),
                false,
                platform_version,
            )
            .expect("should verify proof")
            .1;

        assert_eq!(result.len(), 2);

        // Check address 1
        let (nonce, balance) = result
            .get(&PLATFORM_ADDRESS_1)
            .expect("should have address 1")
            .expect("should have value");
        assert_eq!(nonce, PLATFORM_ADDRESS_1_NONCE);
        assert_eq!(balance, PLATFORM_ADDRESS_1_BALANCE);

        // Check address 2
        let (nonce, balance) = result
            .get(&PLATFORM_ADDRESS_2)
            .expect("should have address 2")
            .expect("should have value");
        assert_eq!(nonce, PLATFORM_ADDRESS_2_NONCE);
        assert_eq!(balance, PLATFORM_ADDRESS_2_BALANCE);
    }

    /// This test fails due to GroveDB bug in ProvableCountSumTree absence proofs.
    #[test]
    fn should_prove_and_verify_only_unknown_address() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        setup_address_balances(&drive, platform_version);

        // Prove only the unknown address
        let addresses = [UNKNOWN_PLATFORM_ADDRESS];
        let proof = drive
            .prove_balances_with_nonces_v0(addresses.iter(), None, platform_version)
            .expect("should prove unknown address");

        // Verify the proof - THIS FAILS due to GroveDB bug
        let result: BTreeMap<PlatformAddress, Option<(AddressNonce, Credits)>> =
            Drive::verify_addresses_infos(
                proof.as_slice(),
                addresses.iter(),
                false,
                platform_version,
            )
            .expect("should verify proof")
            .1;

        assert_eq!(result.len(), 1);

        // Check unknown address is absent
        assert!(
            result
                .get(&UNKNOWN_PLATFORM_ADDRESS)
                .expect("should have unknown address entry")
                .is_none(),
            "unknown address should be absent"
        );
    }

    /// This test fails due to GroveDB bug in ProvableCountSumTree absence proofs.
    #[test]
    fn should_prove_and_verify_multiple_addresses_with_unknown() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        setup_address_balances(&drive, platform_version);

        // Prove multiple addresses including one that doesn't exist
        let addresses = [
            PLATFORM_ADDRESS_1,
            PLATFORM_ADDRESS_2,
            UNKNOWN_PLATFORM_ADDRESS,
        ];
        let proof = drive
            .prove_balances_with_nonces_v0(addresses.iter(), None, platform_version)
            .expect("should prove multiple addresses");

        // Verify the proof - THIS FAILS due to GroveDB bug
        let result: BTreeMap<PlatformAddress, Option<(AddressNonce, Credits)>> =
            Drive::verify_addresses_infos(
                proof.as_slice(),
                addresses.iter(),
                false,
                platform_version,
            )
            .expect("should verify proof")
            .1;

        assert_eq!(result.len(), 3);

        // Check address 1
        let (nonce, balance) = result
            .get(&PLATFORM_ADDRESS_1)
            .expect("should have address 1")
            .expect("should have value");
        assert_eq!(nonce, PLATFORM_ADDRESS_1_NONCE);
        assert_eq!(balance, PLATFORM_ADDRESS_1_BALANCE);

        // Check address 2
        let (nonce, balance) = result
            .get(&PLATFORM_ADDRESS_2)
            .expect("should have address 2")
            .expect("should have value");
        assert_eq!(nonce, PLATFORM_ADDRESS_2_NONCE);
        assert_eq!(balance, PLATFORM_ADDRESS_2_BALANCE);

        // Check unknown address is absent
        assert!(
            result
                .get(&UNKNOWN_PLATFORM_ADDRESS)
                .expect("should have unknown address entry")
                .is_none(),
            "unknown address should be absent"
        );
    }
}
