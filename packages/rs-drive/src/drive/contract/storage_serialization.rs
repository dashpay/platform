//! The bytes Drive stores a contract as.

use crate::error::contract::DataContractError;
use crate::error::Error;
use dpp::data_contract::DataContract;
use dpp::serialization::PlatformSerializableWithPlatformVersion;
use dpp::version::PlatformVersion;

/// The bytes Drive stores `contract` as at `platform_version`
/// (`contract_storage_serialization_version`), refusing a contract whose bytes in the format
/// state transitions carry (`contract_serialization_version`) are over `max_serialized_size`:
/// the cap applies to that format whatever format the contract is stored in. Through protocol
/// version 14 the two formats are the same, and the stored bytes are the ones checked.
pub(crate) fn serialize_contract_for_storage(
    contract: &DataContract,
    platform_version: &PlatformVersion,
) -> Result<Vec<u8>, Error> {
    let contract_versions = &platform_version.dpp.contract_versions;
    let stored = contract.serialize_to_storage_bytes(platform_version)?;
    let capped_size = if contract_versions.contract_storage_serialization_version
        == contract_versions
            .contract_serialization_version
            .default_current_version
    {
        stored.len()
    } else {
        contract
            .serialize_to_bytes_with_platform_version(platform_version)?
            .len()
    };
    if capped_size as u64 > u32::MAX as u64
        || capped_size as u32 > contract_versions.max_serialized_size
    {
        // This should normally be caught by DPP, but there is a rare possibility that the
        // re-serialized size is bigger than the original serialized data contract.
        return Err(Error::DataContract(DataContractError::ContractTooBig(
            format!(
                "Trying to insert a data contract of size {} that is over the max allowed insertion size {}",
                capped_size, contract_versions.max_serialized_size
            ),
        )));
    }
    Ok(stored)
}

#[cfg(test)]
mod tests {
    use crate::drive::contract::paths::contract_root_path;
    use crate::drive::Drive;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
    use dpp::data_contract::DataContract;
    use dpp::tests::fixtures::get_data_contract_fixture;
    use dpp::version::PlatformVersion;
    use grovedb::Element;

    fn protocol_version_14() -> &'static PlatformVersion {
        PlatformVersion::get(14).expect("expected protocol version 14")
    }

    fn fixture_contract(platform_version: &PlatformVersion) -> DataContract {
        get_data_contract_fixture(None, 0, platform_version.protocol_version).data_contract_owned()
    }

    /// The bytes stored at the contract's key 0.
    fn stored_bytes(drive: &Drive, contract: &DataContract) -> Vec<u8> {
        let element = drive
            .grove
            .get(
                &contract_root_path(contract.id_ref().as_bytes()),
                &[0],
                None,
                &PlatformVersion::latest().drive.grove_version,
            )
            .unwrap()
            .expect("expected the contract to be stored");
        match element {
            Element::Item(bytes, _) => bytes,
            other => panic!("expected an item, got {other:?}"),
        }
    }

    fn apply(drive: &Drive, contract: &DataContract, platform_version: &PlatformVersion) {
        drive
            .apply_contract(
                contract,
                BlockInfo::default(),
                true,
                None,
                None,
                platform_version,
            )
            .expect("expected to apply the contract");
    }

    #[test]
    fn should_store_a_contract_compiled_from_protocol_version_15() {
        for (platform_version, format) in
            [(PlatformVersion::latest(), 2), (protocol_version_14(), 1)]
        {
            let drive = setup_drive_with_initial_state_structure(None);
            let contract = fixture_contract(platform_version);
            apply(&drive, &contract, platform_version);
            assert_eq!(
                stored_bytes(&drive, &contract)[0],
                format,
                "protocol version {}",
                platform_version.protocol_version
            );
            let fetched = drive
                .fetch_contract(
                    contract.id().to_buffer(),
                    None,
                    None,
                    None,
                    platform_version,
                )
                .value
                .expect("expected the fetch to succeed")
                .expect("expected the contract");
            assert_eq!(fetched.contract, contract);
        }
    }

    #[test]
    fn should_not_rewrite_an_unchanged_contract_stored_before_protocol_version_15() {
        let drive = setup_drive_with_initial_state_structure(None);
        let contract = fixture_contract(protocol_version_14());
        apply(&drive, &contract, protocol_version_14());
        let stored = stored_bytes(&drive, &contract);
        assert_eq!(stored[0], 1);

        apply(&drive, &contract, PlatformVersion::latest());
        assert_eq!(stored_bytes(&drive, &contract), stored);
    }

    #[test]
    fn should_store_a_contract_updated_from_protocol_version_15_compiled() {
        let drive = setup_drive_with_initial_state_structure(None);
        let mut contract = fixture_contract(protocol_version_14());
        apply(&drive, &contract, protocol_version_14());
        assert_eq!(stored_bytes(&drive, &contract)[0], 1);

        contract.set_version(2);
        apply(&drive, &contract, PlatformVersion::latest());
        assert_eq!(stored_bytes(&drive, &contract)[0], 2);
    }

    #[test]
    fn should_prove_a_contract_stored_compiled() {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(None);
        let contract = fixture_contract(platform_version);
        apply(&drive, &contract, platform_version);

        let proof = drive
            .prove_contract(contract.id().to_buffer(), None, platform_version)
            .expect("expected to prove the contract");
        let (_, verified) = Drive::verify_contract(
            &proof,
            Some(false),
            false,
            false,
            contract.id().to_buffer(),
            platform_version,
        )
        .expect("expected the proof to verify");
        assert_eq!(verified, Some(contract));
    }
}
