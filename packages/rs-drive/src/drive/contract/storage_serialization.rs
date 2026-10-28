//! The bytes Drive stores a contract as.

use crate::error::contract::DataContractError;
use crate::error::Error;
use dpp::data_contract::DataContract;
use dpp::version::PlatformVersion;

/// The bytes Drive stores `contract` as at `platform_version`
/// (`contract_storage_serialization_version`), refusing a contract whose bytes in the format
/// state transitions carry (`contract_serialization_version`) are over `max_serialized_size`:
/// the cap applies to that format whatever format the contract is stored in. Through protocol
/// version 14 the two formats are the same, and the stored bytes are the ones checked. A
/// contract stored compiled is also refused over `max_compiled_contract_size`, which
/// registration, update and token configuration update refuse first as a consensus error.
pub(crate) fn serialize_contract_for_storage(
    contract: &DataContract,
    platform_version: &PlatformVersion,
) -> Result<Vec<u8>, Error> {
    let max_serialized_size = platform_version.dpp.contract_versions.max_serialized_size;
    let (stored, capped_size) =
        contract.serialize_to_storage_bytes_with_carried_size(platform_version)?;
    if capped_size as u64 > u64::from(max_serialized_size) {
        // This should normally be caught by DPP, but there is a rare possibility that the
        // re-serialized size is bigger than the original serialized data contract.
        return Err(Error::DataContract(DataContractError::ContractTooBig(
            format!(
                "Trying to insert a data contract of size {} that is over the max allowed insertion size {}",
                capped_size, max_serialized_size
            ),
        )));
    }
    if let Some(max_compiled_size) = compiled_contract_size_cap(platform_version)
        .filter(|max_compiled_size| stored.len() as u64 > u64::from(*max_compiled_size))
    {
        return Err(Error::DataContract(DataContractError::ContractTooBig(
            format!(
                "Trying to insert a data contract compiled to {} bytes, over the max allowed compiled size {}",
                stored.len(),
                max_compiled_size
            ),
        )));
    }
    Ok(stored)
}

/// The most a contract stored compiled takes at `platform_version`
/// (`max_compiled_contract_size`), or `None` where contracts are not stored compiled.
fn compiled_contract_size_cap(platform_version: &PlatformVersion) -> Option<u32> {
    if platform_version
        .dpp
        .contract_versions
        .contract_storage_serialization_version
        >= 2
    {
        platform_version.system_limits.max_compiled_contract_size
    } else {
        None
    }
}

/// The size a fee estimate reads a stored contract at, as an estimate cannot read it:
/// `max_compiled_contract_size`, the most a contract stored compiled takes, where contracts
/// are stored compiled, and `estimated_contract_max_serialized_size` otherwise.
pub(crate) fn estimated_stored_contract_size(platform_version: &PlatformVersion) -> u32 {
    compiled_contract_size_cap(platform_version).unwrap_or(
        platform_version
            .system_limits
            .estimated_contract_max_serialized_size as u32,
    )
}

#[cfg(test)]
mod tests {
    use crate::drive::contract::paths::contract_root_path;
    use crate::drive::Drive;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::schema::DataContractSchemaMethodsV0;
    use dpp::data_contract::DataContract;
    use dpp::serialization::PlatformSerializableWithPlatformVersion;
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
    fn should_store_and_prove_a_contract_compiled_over_the_cap_its_carried_bytes_are_under() {
        let platform_version = PlatformVersion::latest();
        let max_serialized_size =
            platform_version.dpp.contract_versions.max_serialized_size as usize;
        let mut contract = fixture_contract(platform_version);
        let schema = contract
            .document_schemas()
            .get("indexedDocument")
            .map(|schema| (*schema).clone())
            .expect("expected the fixture's indexed document type");
        // Copies of a document type until the contract's compiled bytes are over the cap
        let mut copies = 0;
        while contract
            .serialize_to_storage_bytes(platform_version)
            .expect("expected to serialize the contract for storage")
            .len()
            <= max_serialized_size
        {
            contract
                .set_document_schema(
                    &format!("copy{copies}"),
                    schema.clone(),
                    false,
                    &mut vec![],
                    platform_version,
                )
                .expect("expected to add a document type");
            copies += 1;
        }
        let carried_size = contract
            .serialize_to_bytes_with_platform_version(platform_version)
            .expect("expected to serialize the contract")
            .len();
        assert!(carried_size <= max_serialized_size);
        assert_eq!(
            contract
                .serialize_to_storage_bytes_with_carried_size(platform_version)
                .expect("expected to serialize the contract for storage")
                .1,
            carried_size
        );

        let drive = setup_drive_with_initial_state_structure(None);
        apply(&drive, &contract, platform_version);
        assert!(stored_bytes(&drive, &contract).len() > max_serialized_size);
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

    #[test]
    fn should_refuse_to_store_a_contract_compiled_over_the_cap() {
        let mut platform_version = PlatformVersion::latest().clone();
        platform_version.system_limits.max_compiled_contract_size = Some(100);
        let contract = fixture_contract(PlatformVersion::latest());
        assert!(matches!(
            super::serialize_contract_for_storage(&contract, &platform_version),
            Err(crate::error::Error::DataContract(
                crate::error::contract::DataContractError::ContractTooBig(_)
            ))
        ));
        // Where contracts are not stored compiled, the cap is not read
        let mut platform_version = protocol_version_14().clone();
        platform_version.system_limits.max_compiled_contract_size = Some(100);
        assert!(super::serialize_contract_for_storage(
            &fixture_contract(protocol_version_14()),
            &platform_version
        )
        .is_ok());
    }
}
