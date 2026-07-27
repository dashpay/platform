use crate::drive::Drive;
use crate::error::proof::ProofError;
use crate::error::Error;
use crate::verify::RootHash;
use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use dpp::prelude::AddressNonce;
use grovedb::{Element, GroveDb};
use platform_version::version::PlatformVersion;

impl Drive {
    pub(super) fn verify_address_info_v0(
        proof: &[u8],
        address: &PlatformAddress,
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Option<(AddressNonce, Credits)>), Error> {
        let path_query = Self::balance_for_clear_address_query(address);

        let (root_hash, mut proved_key_values) = if verify_subset_of_proof {
            GroveDb::verify_subset_query_with_absence_proof(
                proof,
                &path_query,
                &platform_version.drive.grove_version,
            )?
        } else {
            GroveDb::verify_query_with_absence_proof(
                proof,
                &path_query,
                &platform_version.drive.grove_version,
            )?
        };

        if proved_key_values.len() != 1 {
            return Err(Error::Proof(ProofError::CorruptedProof(
                "we should always get back one element".to_string(),
            )));
        }

        let element = proved_key_values.remove(0).2;

        let balance_info = element
            .map(|element| {
                let Element::ItemWithSumItem(nonce_vec, balance_i64, _) = element else {
                    return Err(Error::Proof(ProofError::CorruptedProof(
                        "expected an item with sum item element".to_string(),
                    )));
                };

                let nonce_bytes: [u8; 4] = nonce_vec.try_into().map_err(|_| {
                    Error::Proof(ProofError::IncorrectValueSize("nonce should be 4 bytes"))
                })?;
                let nonce = AddressNonce::from_be_bytes(nonce_bytes);

                if balance_i64 < 0 {
                    return Err(Error::Proof(ProofError::CorruptedProof(
                        "balance cannot be negative".to_string(),
                    )));
                }
                let balance = balance_i64 as Credits;

                Ok((nonce, balance))
            })
            .transpose()?;

        Ok((root_hash, balance_info))
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

    const ADDRESS_1: PlatformAddress = PlatformAddress::P2pkh([10; 20]);
    const ADDRESS_1_NONCE: AddressNonce = 5;
    const ADDRESS_1_BALANCE: Credits = 1_000_000;
    const UNKNOWN_ADDRESS: PlatformAddress = PlatformAddress::P2pkh([200; 20]);

    fn setup_address(drive: &Drive, platform_version: &PlatformVersion) {
        let operations = vec![DriveOperation::AddressFundsOperation(
            AddressFundsOperationType::SetBalanceToAddress {
                address: ADDRESS_1,
                nonce: ADDRESS_1_NONCE,
                balance: ADDRESS_1_BALANCE,
            },
        )];

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
    fn should_prove_and_verify_single_address_info() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        setup_address(&drive, platform_version);

        // Prove single address
        let proof = drive
            .prove_balance_and_nonce(&ADDRESS_1, None, platform_version)
            .expect("should prove address");

        // Verify the proof
        let (root_hash, result) =
            Drive::verify_address_info(proof.as_slice(), &ADDRESS_1, false, platform_version)
                .expect("should verify proof");

        assert!(!root_hash.is_empty(), "root hash should not be empty");
        let (nonce, balance) = result.expect("should have value");
        assert_eq!(nonce, ADDRESS_1_NONCE);
        assert_eq!(balance, ADDRESS_1_BALANCE);
    }

    #[test]
    fn should_prove_and_verify_absent_address_info() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        setup_address(&drive, platform_version);

        // Prove unknown address
        let proof = drive
            .prove_balance_and_nonce(&UNKNOWN_ADDRESS, None, platform_version)
            .expect("should prove unknown address");

        // Verify the proof
        let (root_hash, result) =
            Drive::verify_address_info(proof.as_slice(), &UNKNOWN_ADDRESS, false, platform_version)
                .expect("should verify proof");

        assert!(!root_hash.is_empty(), "root hash should not be empty");
        assert!(result.is_none(), "unknown address should be absent");
    }
}
