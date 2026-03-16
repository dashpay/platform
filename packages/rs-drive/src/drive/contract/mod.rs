//! DriveDataContracts.
//!
//! This module defines functions pertinent toDataContracts stored in Drive.
//!

#[cfg(feature = "server")]
mod apply;
#[cfg(feature = "server")]
mod contract_fetch_info;
#[cfg(feature = "server")]
mod estimation_costs;
#[cfg(feature = "server")]
mod get_fetch;
#[cfg(feature = "server")]
mod insert;
/// Various paths for contract operations
#[cfg(any(feature = "server", feature = "verify"))]
pub mod paths;
#[cfg(feature = "server")]
pub(crate) mod prove;
#[cfg(any(feature = "server", feature = "verify"))]
pub(crate) mod queries;
#[cfg(feature = "fixtures-and-mocks")]
/// Test helpers and utility methods
pub mod test_helpers;
#[cfg(feature = "server")]
mod update;
#[cfg(feature = "server")]
pub use contract_fetch_info::*;

/// How many contracts to fetch at once. This is an arbitrary number and is needed to prevent
/// the server from being overloaded with requests.
pub const MAX_CONTRACT_HISTORY_FETCH_LIMIT: u16 = 10;

#[cfg(feature = "server")]
#[cfg(test)]
mod tests {
    use dpp::block::block_info::BlockInfo;
    use dpp::block::epoch::Epoch;
    use dpp::data_contract::config::v0::DataContractConfigSettersV0;
    use rand::prelude::StdRng;
    use rand::{random, SeedableRng};
    use std::borrow::Cow;
    use std::option::Option::None;

    use crate::drive::contract::paths::contract_root_path_vec;
    use crate::drive::Drive;
    use crate::error::drive::DriveError;
    use crate::error::Error;
    use crate::util::object_size_info::{DocumentAndContractInfo, DocumentInfo, OwnedDocumentInfo};
    use crate::util::storage_flags::StorageFlags;
    use dpp::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
    use dpp::data_contract::document_type::random_document::CreateRandomDocument;
    use dpp::data_contract::schema::DataContractSchemaMethodsV0;
    use dpp::data_contract::DataContract;
    use dpp::document::DocumentV0Getters;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::contract_bounds::ContractBounds;
    use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use dpp::identity::{Identity, KeyType, Purpose, SecurityLevel};
    use dpp::platform_value::{platform_value, BinaryData};
    use dpp::prelude::IdentityPublicKey;
    use dpp::tests::fixtures::{
        get_dashpay_contract_fixture, get_dashpay_contract_with_generalized_encryption_key_fixture,
    };
    use dpp::tests::json_document::json_document_to_contract;

    use crate::drive::identity::key::fetch::{IdentityKeysRequest, KeyIDIdentityPublicKeyPairVec};
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::version::PlatformVersion;

    #[allow(dead_code)]
    #[deprecated(note = "This function is marked as unused.")]
    #[allow(deprecated)]
    fn setup_deep_nested_50_contract() -> (Drive, DataContract) {
        let drive: Drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let contract_path = "tests/supporting_files/contract/deepNested/deep-nested50.json";
        // let's construct the grovedb structure for the dashpay data contract
        let contract = json_document_to_contract(contract_path, false, platform_version)
            .expect("expected to get a contract");
        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("expected to apply contract successfully");

        (drive, contract)
    }

    #[allow(dead_code)]
    #[deprecated(note = "This function is marked as unused.")]
    #[allow(deprecated)]
    fn setup_deep_nested_10_contract() -> (Drive, DataContract) {
        let drive: Drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let contract_path = "tests/supporting_files/contract/deepNested/deep-nested10.json";
        // let's construct the grovedb structure for the dashpay data contract
        let contract = json_document_to_contract(contract_path, false, platform_version)
            .expect("expected to get a contract");
        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("expected to apply contract successfully");

        (drive, contract)
    }

    pub(in crate::drive::contract) fn setup_reference_contract() -> (Drive, DataContract) {
        let drive: Drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let contract_path = "tests/supporting_files/contract/references/references.json";

        // let's construct the grovedb structure for the dashpay data contract
        let contract = json_document_to_contract(contract_path, false, platform_version)
            .expect("expected to get a contract");
        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("expected to apply contract successfully");

        (drive, contract)
    }

    pub(in crate::drive::contract) fn setup_dashpay() -> (Drive, DataContract) {
        let drive: Drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        // let's construct the grovedb structure for the dashpay data contract
        let contract = get_dashpay_contract_fixture(None, 0, 1).data_contract_owned();
        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("expected to apply contract successfully");

        (drive, contract)
    }

    pub(in crate::drive::contract) fn setup_dashpay_with_generalized_encryption_contract(
    ) -> (Drive, DataContract) {
        let drive: Drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        // let's construct the grovedb structure for the dashpay data contract
        let contract = get_dashpay_contract_with_generalized_encryption_key_fixture(None, 0, 1)
            .data_contract_owned();
        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("expected to apply contract successfully");

        (drive, contract)
    }

    /// Sets up a drive instance with a history-enabled contract applied at time_ms=1000.
    pub(in crate::drive::contract) fn setup_history_contract() -> (Drive, DataContract) {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let contract_path =
            "tests/supporting_files/contract/references/references_with_contract_history.json";
        let contract = json_document_to_contract(contract_path, false, platform_version)
            .expect("expected to get contract");

        drive
            .apply_contract(
                &contract,
                BlockInfo {
                    time_ms: 1000,
                    height: 100,
                    core_height: 10,
                    epoch: Default::default(),
                },
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("expected to apply contract successfully");

        (drive, contract)
    }

    #[test]
    fn test_create_and_update_contract() {
        let (drive, mut contract) = setup_reference_contract();

        let platform_version = PlatformVersion::latest();

        let note2_schema = platform_value!({
            "type": "object",
            "properties": {
                "last_name": {
                    "type": "number",
                    "position": 0
                },
                "first_name": {
                    "type": "integer",
                    "position": 1
                }
            },
            "additionalProperties": false,
        });

        contract
            .set_document_schema("note2", note2_schema, true, &mut vec![], platform_version)
            .expect("should set a document schema");

        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("expected to apply contract successfully");
    }
    //
    // #[test]
    // fn test_create_deep_nested_contract_50() {
    //     let (drive, contract) = setup_deep_nested_50_contract();
    //     let platform_version = PlatformVersion::latest();
    //
    //     let document_type = contract
    //         .document_type_for_name("nest")
    //         .expect("expected to get document type");
    //
    //     let document = document_type
    //         .random_document(Some(5), platform_version)
    //         .expect("expected to get random document");
    //
    //     let nested_value = document.properties().get("abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0");
    //
    //     assert!(nested_value.is_some());
    //
    //     let storage_flags = Some(Cow::Owned(StorageFlags::SingleEpoch(0)));
    //
    //     let random_owner_id = rand::thread_rng().gen::<[u8; 32]>();
    //     drive
    //         .add_contested_document_for_contract(
    //             DocumentAndContractInfo {
    //                 owned_document_info: OwnedDocumentInfo {
    //                     document_info: DocumentInfo::DocumentRefInfo((&document, storage_flags)),
    //                     owner_id: Some(random_owner_id),
    //                 },
    //                 contract: &contract,
    //                 document_type,
    //             },
    //             false,
    //             BlockInfo::default(),
    //             true,
    //             None,
    //             platform_version,
    //         )
    //         .expect("expected to insert a document successfully");
    // }

    #[test]
    fn test_create_reference_contract() {
        let (drive, contract) = setup_reference_contract();
        let platform_version = PlatformVersion::latest();

        let document_type = contract
            .document_type_for_name("note")
            .expect("expected to get document type");

        let document = document_type
            .random_document(Some(5), platform_version)
            .expect("expected to get random document");

        let ref_value = document.properties().get("abc17");

        assert!(ref_value.is_some());

        let storage_flags = Some(Cow::Owned(StorageFlags::SingleEpoch(0)));

        let random_owner_id = random::<[u8; 32]>();
        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentInfo::DocumentRefInfo((&document, storage_flags)),
                        owner_id: Some(random_owner_id),
                    },
                    contract: &contract,
                    document_type,
                },
                false,
                BlockInfo::default(),
                true,
                None,
                platform_version,
                None,
            )
            .expect("expected to insert a document successfully");
    }

    #[test]
    fn test_create_contract_with_encryption_keys() {
        let (drive, contract) = setup_dashpay_with_generalized_encryption_contract();
        let platform_version = PlatformVersion::latest();

        // Let's insert an identity with an encryption key for this contract

        let mut identity = Identity::random_identity(5, Some(12345), platform_version)
            .expect("expected a random identity");

        let mut rng = StdRng::from_entropy();

        let encryption_key: IdentityPublicKey = IdentityPublicKeyV0 {
            id: 5,
            purpose: Purpose::ENCRYPTION,
            security_level: SecurityLevel::MEDIUM,
            contract_bounds: Some(ContractBounds::SingleContract { id: contract.id() }),
            key_type: KeyType::ECDSA_SECP256K1,
            read_only: false,
            data: BinaryData::new(
                KeyType::ECDSA_SECP256K1
                    .random_public_key_data(&mut rng, platform_version)
                    .expect("expected a random key"),
            ),
            disabled_at: None,
        }
        .into();
        identity.add_public_key(encryption_key.clone());

        let db_transaction = drive.grove.start_transaction();

        drive
            .add_new_identity(
                identity.clone(),
                false,
                &BlockInfo::default(),
                true,
                Some(&db_transaction),
                platform_version,
            )
            .expect("expected to insert identity");

        drive
            .grove
            .commit_transaction(db_transaction)
            .unwrap()
            .expect("expected to be able to commit a transaction");

        let request = IdentityKeysRequest::new_contract_encryption_keys_query(
            identity.id().to_buffer(),
            contract.id().to_buffer(),
        );
        let identity_keys = drive
            .fetch_identity_keys::<KeyIDIdentityPublicKeyPairVec>(request, None, platform_version)
            .expect("expected keys");
        assert_eq!(identity_keys.len(), 1);
        assert_eq!(
            &identity_keys
                .first()
                .expect("expected at least one identity key")
                .1,
            &encryption_key
        );
    }

    #[test]
    fn test_create_contract_with_encryption_keys_on_document_type() {
        let (drive, contract) = setup_dashpay();
        let platform_version = PlatformVersion::latest();

        // Let's insert an identity with an encryption key for this contract

        let mut identity = Identity::random_identity(0, Some(12345), platform_version)
            .expect("expected a random identity");

        let mut rng = StdRng::from_entropy();

        let encryption_key: IdentityPublicKey = IdentityPublicKeyV0 {
            id: 5,
            purpose: Purpose::ENCRYPTION,
            security_level: SecurityLevel::MEDIUM,
            contract_bounds: Some(ContractBounds::SingleContractDocumentType {
                id: contract.id(),
                document_type_name: "contactRequest".to_string(),
            }),
            key_type: KeyType::ECDSA_SECP256K1,
            read_only: false,
            data: BinaryData::new(
                KeyType::ECDSA_SECP256K1
                    .random_public_key_data(&mut rng, platform_version)
                    .expect("expected a random key"),
            ),
            disabled_at: None,
        }
        .into();
        identity.add_public_key(encryption_key.clone());

        let db_transaction = drive.grove.start_transaction();

        drive
            .add_new_identity(
                identity.clone(),
                false,
                &BlockInfo::default(),
                true,
                Some(&db_transaction),
                platform_version,
            )
            .expect("expected to insert identity");

        drive
            .grove
            .commit_transaction(db_transaction)
            .unwrap()
            .expect("expected to be able to commit a transaction");

        let request = IdentityKeysRequest::new_document_type_encryption_keys_query(
            identity.id().to_buffer(),
            contract.id().to_buffer(),
            "contactRequest".to_string(),
        );
        let identity_keys = drive
            .fetch_identity_keys::<KeyIDIdentityPublicKeyPairVec>(request, None, platform_version)
            .expect("expected keys");
        assert_eq!(identity_keys.len(), 1);
        assert_eq!(
            &identity_keys
                .first()
                .expect("expected at least one identity key")
                .1,
            &encryption_key
        );
    }

    #[test]
    fn test_create_reference_contract_without_apply() {
        let drive: Drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let contract_path = "tests/supporting_files/contract/references/references.json";

        // let's construct the grovedb structure for the dashpay data contract
        let contract = json_document_to_contract(contract_path, false, platform_version)
            .expect("expected to get cbor document");
        let fee = drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                false,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("expected to apply contract successfully");

        // Estimation (apply=false) should produce non-zero processing fee
        assert!(
            fee.processing_fee > 0,
            "estimation should produce non-zero processing fee"
        );
    }

    #[test]
    fn test_create_reference_contract_with_history_without_apply() {
        let drive: Drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let contract_path =
            "tests/supporting_files/contract/references/references_with_contract_history.json";

        // let's construct the grovedb structure for the dashpay data contract
        let contract = json_document_to_contract(contract_path, false, platform_version)
            .expect("expected to get contract");
        let fee = drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                false,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("expected to apply contract successfully");

        // Estimation (apply=false) for a history contract should produce non-zero processing fee
        assert!(
            fee.processing_fee > 0,
            "history estimation should produce non-zero processing fee"
        );
    }

    #[test]
    fn test_update_reference_contract_without_apply() {
        let drive: Drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let contract_path = "tests/supporting_files/contract/references/references.json";

        // let's construct the grovedb structure for the dashpay data contract
        let contract = json_document_to_contract(contract_path, false, platform_version)
            .expect("expected to get cbor document");

        // Create a contract first
        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("expected to apply contract successfully");

        // Update existing contract
        drive
            .update_contract(
                &contract,
                BlockInfo::default(),
                false,
                None,
                platform_version,
                None,
            )
            .expect("expected to apply contract successfully");
    }

    #[test]
    fn test_get_cached_contract_returns_none_when_not_cached() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let result = drive
            .get_cached_contract_with_fetch_info([0u8; 32], None, &platform_version.drive)
            .expect("should not error");

        assert!(result.is_none());
    }

    #[test]
    fn test_get_cached_contract_returns_some_after_fetch() {
        let (drive, contract) = setup_reference_contract();
        let platform_version = PlatformVersion::latest();

        // First fetch to populate cache
        let fetched = drive
            .get_contract_with_fetch_info(contract.id().to_buffer(), true, None, platform_version)
            .expect("should fetch contract");
        assert!(fetched.is_some());

        // Now check cache returns the contract
        let cached = drive
            .get_cached_contract_with_fetch_info(
                contract.id().to_buffer(),
                None,
                &platform_version.drive,
            )
            .expect("should not error");
        assert!(cached.is_some());
        assert_eq!(
            cached
                .expect("cached contract should be present")
                .contract
                .id(),
            contract.id()
        );
    }

    #[test]
    fn test_get_cached_contract_in_transaction_context() {
        let (drive, contract) = setup_reference_contract();
        let platform_version = PlatformVersion::latest();

        // Populate the non-tx cache by fetching the contract outside any transaction
        let fetched = drive
            .get_contract_with_fetch_info(contract.id().to_buffer(), true, None, platform_version)
            .expect("should fetch contract");
        assert!(fetched.is_some(), "contract should be fetchable");
        assert_eq!(
            fetched
                .expect("fetched contract should exist")
                .contract
                .version(),
            1
        );

        let transaction = drive.grove.start_transaction();

        // Insert a modified contract in transaction
        let mut updated_contract = contract.clone();
        updated_contract.increment_version();

        drive
            .update_contract(
                &updated_contract,
                BlockInfo::default(),
                true,
                Some(&transaction),
                platform_version,
                None,
            )
            .expect("should update contract");

        // Cache in transaction context should have updated version
        let cached_in_tx = drive
            .get_cached_contract_with_fetch_info(
                contract.id().to_buffer(),
                Some(&transaction),
                &platform_version.drive,
            )
            .expect("should not error");
        assert!(cached_in_tx.is_some());
        assert_eq!(
            cached_in_tx
                .expect("tx-cached contract should be present")
                .contract
                .version(),
            2
        );

        // Non-transaction cache should still see the original version (v1)
        let cached_outside_tx = drive
            .get_cached_contract_with_fetch_info(
                contract.id().to_buffer(),
                None,
                &platform_version.drive,
            )
            .expect("should not error");
        assert!(
            cached_outside_tx.is_some(),
            "non-tx cache should still have the contract"
        );
        assert_eq!(
            cached_outside_tx
                .expect("non-tx cached contract should be present")
                .contract
                .version(),
            1,
            "non-tx cache should still see original v1 before commit"
        );
    }

    #[test]
    fn test_get_contracts_with_fetch_info_multiple() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        // Create two contracts
        let contract_path1 = "tests/supporting_files/contract/references/references.json";
        let contract1 = json_document_to_contract(contract_path1, false, platform_version)
            .expect("expected to get contract");
        drive
            .apply_contract(
                &contract1,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("expected to apply contract successfully");

        let contract2 = get_dashpay_contract_fixture(None, 0, 1).data_contract_owned();
        drive
            .apply_contract(
                &contract2,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("expected to apply contract successfully");

        let contract_ids = [contract1.id().to_buffer(), contract2.id().to_buffer()];
        let result = drive
            .get_contracts_with_fetch_info(&contract_ids, true, None, platform_version)
            .expect("should get contracts");

        assert_eq!(result.len(), 2);
        assert!(result
            .get(&contract1.id().to_buffer())
            .expect("result should contain contract1 id")
            .is_some());
        assert!(result
            .get(&contract2.id().to_buffer())
            .expect("result should contain contract2 id")
            .is_some());
    }

    #[test]
    fn test_get_contracts_with_fetch_info_nonexistent() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let contract_ids = [[0u8; 32], [1u8; 32]];
        let result = drive
            .get_contracts_with_fetch_info(&contract_ids, true, None, platform_version)
            .expect("should get contracts");

        assert_eq!(result.len(), 2);
        assert!(result
            .get(&[0u8; 32])
            .expect("result should contain zeroed id")
            .is_none());
        assert!(result
            .get(&[1u8; 32])
            .expect("result should contain ones id")
            .is_none());
    }

    #[test]
    fn test_prove_contracts_multiple() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let contract_path = "tests/supporting_files/contract/references/references.json";
        let contract = json_document_to_contract(contract_path, false, platform_version)
            .expect("expected to get contract");
        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("expected to apply contract successfully");

        let contract2 = get_dashpay_contract_fixture(None, 0, 1).data_contract_owned();
        drive
            .apply_contract(
                &contract2,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("expected to apply contract successfully");

        let contract_ids = [contract.id().to_buffer(), contract2.id().to_buffer()];
        let proof = drive
            .prove_contracts(&contract_ids, None, platform_version)
            .expect("should prove contracts");

        // Verify proof against root hash using Drive::verify_contracts
        let (_root_hash, verified) =
            Drive::verify_contracts(&proof, false, &contract_ids, platform_version)
                .expect("should verify contracts proof");
        assert_eq!(verified.len(), 2);
        assert!(
            verified
                .get(&contract.id().to_buffer())
                .expect("should contain first contract id")
                .is_some(),
            "first contract should be present in proof"
        );
        assert!(
            verified
                .get(&contract2.id().to_buffer())
                .expect("should contain second contract id")
                .is_some(),
            "second contract should be present in proof"
        );
    }

    #[test]
    fn test_prove_contracts_with_nonexistent() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        // Prove a contract that does not exist
        let contract_ids = [[42u8; 32]];
        let proof = drive
            .prove_contracts(&contract_ids, None, platform_version)
            .expect("should prove contracts even if nonexistent");

        // Verify proof proves non-existence using Drive::verify_contract
        let (_root_hash, verified_contract) = Drive::verify_contract(
            &proof,
            Some(false),
            false,
            true,
            [42u8; 32],
            platform_version,
        )
        .expect("should verify absence proof for nonexistent contract");
        assert!(
            verified_contract.is_none(),
            "nonexistent contract should verify as None"
        );
    }

    #[test]
    fn test_prove_single_contract() {
        let (drive, contract) = setup_reference_contract();
        let platform_version = PlatformVersion::latest();

        let proof = drive
            .prove_contract(contract.id().to_buffer(), None, platform_version)
            .expect("should prove contract");

        // Verify proof against root hash using Drive::verify_contract
        let (_root_hash, verified_contract) = Drive::verify_contract(
            &proof,
            Some(false),
            false,
            false,
            contract.id().to_buffer(),
            platform_version,
        )
        .expect("should verify single contract proof");
        assert!(verified_contract.is_some(), "contract should be in proof");
        assert_eq!(
            verified_contract
                .expect("verified contract should exist")
                .id(),
            contract.id()
        );
    }

    #[test]
    fn test_prove_single_contract_nonexistent() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let proof = drive
            .prove_contract([99u8; 32], None, platform_version)
            .expect("should prove contract nonexistence");

        // Verify proof proves non-existence using Drive::verify_contract
        let (_root_hash, verified_contract) = Drive::verify_contract(
            &proof,
            Some(false),
            false,
            false,
            [99u8; 32],
            platform_version,
        )
        .expect("should verify nonexistent contract proof");
        assert!(
            verified_contract.is_none(),
            "nonexistent contract should verify as None"
        );
    }

    #[test]
    fn test_prove_contract_history() {
        use dpp::tests::fixtures::get_data_contract_fixture;

        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let mut contract = get_data_contract_fixture(None, 0, platform_version.protocol_version)
            .data_contract_owned();
        contract.config_mut().set_keeps_history(true);
        contract.config_mut().set_readonly(false);

        drive
            .apply_contract(
                &contract,
                BlockInfo {
                    time_ms: 1000,
                    height: 100,
                    core_height: 10,
                    epoch: Default::default(),
                },
                true,
                None,
                None,
                platform_version,
            )
            .expect("expected to apply contract successfully");

        let proof = drive
            .prove_contract_history(
                contract.id().to_buffer(),
                None,
                0,
                Some(10),
                None,
                platform_version,
            )
            .expect("should prove contract history");

        // Verify proof using Drive::verify_contract_history
        let (_root_hash, verified_history) = Drive::verify_contract_history(
            &proof,
            contract.id().to_buffer(),
            0,
            Some(10),
            None,
            platform_version,
        )
        .expect("should verify contract history proof");
        let history = verified_history.expect("history should be Some");
        assert!(
            history.contains_key(&1000),
            "history should contain entry at time 1000"
        );
    }

    #[test]
    fn test_update_contract_errors_on_readonly() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        // Create a mutable contract first, then make it readonly before inserting
        let mut contract = get_dashpay_contract_fixture(None, 0, 1).data_contract_owned();
        contract.config_mut().set_readonly(true);

        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("expected to apply contract successfully");

        // Attempting to update a readonly contract should fail
        let result = drive.update_contract(
            &contract,
            BlockInfo::default(),
            true,
            None,
            platform_version,
            None,
        );

        assert!(
            matches!(
                result,
                Err(Error::Drive(DriveError::UpdatingReadOnlyImmutableContract(
                    _
                )))
            ),
            "Expected UpdatingReadOnlyImmutableContract error, got: {:?}",
            result
        );
    }

    #[test]
    fn test_update_contract_with_new_document_type() {
        let (drive, mut contract) = setup_reference_contract();
        let platform_version = PlatformVersion::latest();

        // Add a new document type
        let new_schema = platform_value!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "position": 0
                }
            },
            "additionalProperties": false,
        });

        contract
            .set_document_schema(
                "brand_new_type",
                new_schema,
                true,
                &mut vec![],
                platform_version,
            )
            .expect("should set document schema");

        contract.increment_version();

        drive
            .update_contract(
                &contract,
                BlockInfo::default(),
                true,
                None,
                platform_version,
                None,
            )
            .expect("should update contract with new document type");

        // Verify the contract can be fetched with the new type
        let fetched = drive
            .get_contract_with_fetch_info(contract.id().to_buffer(), true, None, platform_version)
            .expect("should fetch updated contract")
            .expect("contract should exist");

        assert!(fetched
            .contract
            .document_type_for_name("brand_new_type")
            .is_ok());
    }

    #[test]
    fn test_fetch_contract_with_epoch_calculates_fee() {
        let (drive, contract) = setup_reference_contract();
        let platform_version = PlatformVersion::latest();

        let epoch = Epoch::new(0).expect("should create epoch");
        let result = drive
            .fetch_contract(
                contract.id().to_buffer(),
                Some(&epoch),
                None,
                None,
                platform_version,
            )
            .unwrap_add_cost(&mut Default::default())
            .expect("should fetch contract");

        let fetch_info = result.expect("should have contract");
        assert!(
            fetch_info.fee.is_some(),
            "should have fee when epoch is provided"
        );
    }

    #[test]
    fn test_fetch_contract_without_epoch_no_fee() {
        let (drive, contract) = setup_reference_contract();
        let platform_version = PlatformVersion::latest();

        let result = drive
            .fetch_contract(
                contract.id().to_buffer(),
                None,
                None,
                None,
                platform_version,
            )
            .unwrap_add_cost(&mut Default::default())
            .expect("should fetch contract");

        let fetch_info = result.expect("should have contract");
        assert!(
            fetch_info.fee.is_none(),
            "should have no fee when epoch is not provided"
        );
    }

    #[test]
    fn test_fetch_nonexistent_contract_returns_none() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let result = drive
            .fetch_contract([0u8; 32], None, None, None, platform_version)
            .unwrap_add_cost(&mut Default::default())
            .expect("should not error");

        assert!(result.is_none());
    }

    #[test]
    fn test_fetch_contract_and_add_operations_with_epoch() {
        let (drive, contract) = setup_reference_contract();
        let platform_version = PlatformVersion::latest();

        let epoch = Epoch::new(0).expect("should create epoch");
        let mut operations = vec![];
        let result = drive
            .fetch_contract_and_add_operations(
                contract.id().to_buffer(),
                Some(&epoch),
                None,
                &mut operations,
                platform_version,
            )
            .expect("should fetch contract and add operations");

        assert!(result.is_some());
        assert!(
            !operations.is_empty(),
            "should have operations when epoch is set"
        );
    }

    #[test]
    fn test_fetch_contract_and_add_operations_nonexistent_with_epoch() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let epoch = Epoch::new(0).expect("should create epoch");
        let mut operations = vec![];
        let result = drive
            .fetch_contract_and_add_operations(
                [0u8; 32],
                Some(&epoch),
                None,
                &mut operations,
                platform_version,
            )
            .expect("should not error for nonexistent contract");

        assert!(result.is_none());
        // Even for nonexistent contracts with an epoch, we should get a cost operation
        assert!(
            !operations.is_empty(),
            "should have cost operation for nonexistent contract with epoch"
        );
    }

    #[test]
    fn test_contract_history_with_apply_and_updates() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let contract_path =
            "tests/supporting_files/contract/references/references_with_contract_history.json";
        let mut contract = json_document_to_contract(contract_path, false, platform_version)
            .expect("expected to get contract");

        // Make sure keeps_history is set and contract is mutable
        contract.config_mut().set_keeps_history(true);
        contract.config_mut().set_readonly(false);

        // Initial insert with timestamp
        drive
            .apply_contract(
                &contract,
                BlockInfo {
                    time_ms: 1000,
                    height: 100,
                    core_height: 10,
                    epoch: Default::default(),
                },
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("expected to apply contract successfully");

        // Add a new property and update the contract at a different time
        let new_schema = platform_value!({
            "type": "object",
            "properties": {
                "updated_field": {
                    "type": "string",
                    "position": 0
                }
            },
            "additionalProperties": false,
        });
        contract
            .set_document_schema("newDoc", new_schema, true, &mut vec![], platform_version)
            .expect("should set schema");
        contract.increment_version();

        drive
            .update_contract(
                &contract,
                BlockInfo {
                    time_ms: 2000,
                    height: 200,
                    core_height: 20,
                    epoch: Default::default(),
                },
                true,
                None,
                platform_version,
                None,
            )
            .expect("should update contract with history");

        // Fetch history
        let history = drive
            .fetch_contract_with_history(
                contract.id().to_buffer(),
                None,
                0,
                Some(10),
                None,
                platform_version,
            )
            .expect("should fetch history");

        assert_eq!(history.len(), 2, "should have 2 history entries");
        assert!(history.contains_key(&1000));
        assert!(history.contains_key(&2000));
    }

    #[test]
    fn test_paths_data_contract_paths_trait() {
        use crate::drive::contract::paths::DataContractPaths;

        let contract = get_dashpay_contract_fixture(None, 0, 1).data_contract_owned();

        // Test root_path
        let root = contract.root_path();
        assert_eq!(root.len(), 2);

        // Test documents_path
        let docs_path = contract.documents_path();
        assert_eq!(docs_path.len(), 3);
        assert_eq!(docs_path[2], &[1u8]);

        // Test document_type_path
        let type_path = contract.document_type_path("contactRequest");
        assert_eq!(type_path.len(), 4);
        assert_eq!(type_path[3], "contactRequest".as_bytes());

        // Test documents_primary_key_path
        let pk_path = contract.documents_primary_key_path("contactRequest");
        assert_eq!(pk_path.len(), 5);
        assert_eq!(pk_path[4], &[0u8]);

        // Test documents_with_history_primary_key_path
        let history_pk_path =
            contract.documents_with_history_primary_key_path("contactRequest", &[1, 2, 3]);
        assert_eq!(history_pk_path.len(), 6);
        assert_eq!(history_pk_path[5], &[1, 2, 3]);
    }

    #[test]
    fn test_paths_free_functions() {
        use crate::drive::contract::paths::*;

        let contract_id = [42u8; 32];

        // all_contracts_global_root_path
        let root = all_contracts_global_root_path();
        assert_eq!(root.len(), 1);

        // contract_root_path
        let root_path = contract_root_path(&contract_id);
        assert_eq!(root_path.len(), 2);
        assert_eq!(root_path[1], &contract_id);

        // contract_root_path_vec
        let root_path_vec = contract_root_path_vec(&contract_id);
        assert_eq!(root_path_vec.len(), 2);
        assert_eq!(root_path_vec[1], contract_id.to_vec());

        // contract_storage_path_vec
        let storage_path = contract_storage_path_vec(&contract_id);
        assert_eq!(storage_path.len(), 3);
        assert_eq!(storage_path[2], vec![0]);

        // contract_keeping_history_root_path_vec
        let history_root = contract_keeping_history_root_path_vec(&contract_id);
        assert_eq!(history_root.len(), 3);

        // contract_keeping_history_root_path
        let history_root_ref = contract_keeping_history_root_path(&contract_id);
        assert_eq!(history_root_ref.len(), 3);
        assert_eq!(history_root_ref[1], &contract_id);

        // contract_keeping_history_storage_time_reference_path
        let time_path =
            contract_keeping_history_storage_time_reference_path(&contract_id, vec![1, 2, 3, 4]);
        assert_eq!(time_path.len(), 4);
        assert_eq!(time_path[3], vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_queries_fetch_contract_query_with_and_without_limit() {
        let contract_id = [42u8; 32];

        let query_with_limit = Drive::fetch_contract_query(contract_id, true);
        assert_eq!(query_with_limit.query.limit, Some(1));
        assert_eq!(query_with_limit.path, contract_root_path_vec(&contract_id));

        let query_without_limit = Drive::fetch_contract_query(contract_id, false);
        assert!(query_without_limit.query.limit.is_none());
        assert_eq!(
            query_without_limit.path,
            contract_root_path_vec(&contract_id)
        );
    }

    #[test]
    fn test_queries_fetch_contract_with_history_latest_query() {
        use crate::drive::contract::paths::contract_keeping_history_root_path_vec;

        let contract_id = [42u8; 32];

        let query_with_limit = Drive::fetch_contract_with_history_latest_query(contract_id, true);
        assert_eq!(query_with_limit.query.limit, Some(1));
        assert_eq!(
            query_with_limit.path,
            contract_keeping_history_root_path_vec(&contract_id)
        );

        let query_without_limit =
            Drive::fetch_contract_with_history_latest_query(contract_id, false);
        assert!(query_without_limit.query.limit.is_none());
        assert_eq!(
            query_without_limit.path,
            contract_keeping_history_root_path_vec(&contract_id)
        );
    }

    #[test]
    fn test_queries_fetch_non_historical_contracts_query() {
        use crate::drive::RootTree;

        let ids = [[1u8; 32], [2u8; 32], [3u8; 32]];
        let query = Drive::fetch_non_historical_contracts_query(&ids);
        assert_eq!(query.query.limit, Some(3));
        assert_eq!(
            query.path,
            vec![Into::<&[u8; 1]>::into(RootTree::DataContractDocuments).to_vec()]
        );
    }

    #[test]
    fn test_queries_fetch_historical_contracts_query() {
        use crate::drive::RootTree;

        let ids = [[1u8; 32], [2u8; 32]];
        let query = Drive::fetch_historical_contracts_query(&ids);
        assert_eq!(query.query.limit, Some(2));
        assert_eq!(
            query.path,
            vec![Into::<&[u8; 1]>::into(RootTree::DataContractDocuments).to_vec()]
        );
    }

    #[test]
    fn test_queries_fetch_contracts_query_non_historical_only() {
        let platform_version = PlatformVersion::latest();
        let non_hist = [[1u8; 32]];
        let hist: &[[u8; 32]] = &[];
        let query = Drive::fetch_contracts_query(&non_hist, hist, platform_version)
            .expect("should create query");
        assert_eq!(query.query.limit, Some(1));
    }

    #[test]
    fn test_queries_fetch_contracts_query_historical_only() {
        let platform_version = PlatformVersion::latest();
        let non_hist: &[[u8; 32]] = &[];
        let hist = [[2u8; 32]];
        let query = Drive::fetch_contracts_query(non_hist, &hist, platform_version)
            .expect("should create query");
        assert_eq!(query.query.limit, Some(1));
    }

    #[test]
    fn test_queries_fetch_contracts_query_mixed() {
        let platform_version = PlatformVersion::latest();
        let non_hist = [[1u8; 32]];
        let hist = [[2u8; 32]];
        let query = Drive::fetch_contracts_query(&non_hist, &hist, platform_version)
            .expect("should create merged query");
        // Merged query has no individual limit since it merges two sub-queries
        assert!(
            query.query.limit.is_none(),
            "merged query should have no limit"
        );
    }

    #[test]
    fn test_queries_fetch_contract_history_query_valid_limits() {
        use crate::drive::contract::paths::contract_keeping_history_root_path_vec;

        let contract_id = [42u8; 32];

        // Default limit (None -> 10)
        let query = Drive::fetch_contract_history_query(contract_id, 0, None, None)
            .expect("should create query");
        assert_eq!(query.query.limit, Some(10));
        assert_eq!(
            query.path,
            contract_keeping_history_root_path_vec(&contract_id)
        );

        // Explicit valid limit
        let query = Drive::fetch_contract_history_query(contract_id, 0, Some(5), None)
            .expect("should create query");
        assert_eq!(query.query.limit, Some(5));

        // With offset
        let query = Drive::fetch_contract_history_query(contract_id, 0, Some(5), Some(3))
            .expect("should create query");
        assert_eq!(query.query.offset, Some(3));
    }

    #[test]
    fn test_queries_fetch_contract_history_query_invalid_limits() {
        let contract_id = [42u8; 32];

        // Limit = 0 (below min)
        let result = Drive::fetch_contract_history_query(contract_id, 0, Some(0), None);
        assert!(result.is_err());

        // Limit = 11 (above max)
        let result = Drive::fetch_contract_history_query(contract_id, 0, Some(11), None);
        assert!(result.is_err());
    }

    #[test]
    fn test_fetch_contract_with_known_keeps_history_true() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let contract_path =
            "tests/supporting_files/contract/references/references_with_contract_history.json";
        let mut contract = json_document_to_contract(contract_path, false, platform_version)
            .expect("expected to get contract");
        contract.config_mut().set_keeps_history(true);
        contract.config_mut().set_readonly(false);

        drive
            .apply_contract(
                &contract,
                BlockInfo {
                    time_ms: 1000,
                    height: 100,
                    core_height: 10,
                    epoch: Default::default(),
                },
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("expected to apply contract successfully");

        // Fetch with known_keeps_history = Some(true) - directly queries
        // the contract_keeping_history_root_path
        let result = drive
            .fetch_contract(
                contract.id().to_buffer(),
                None,
                Some(true),
                None,
                platform_version,
            )
            .unwrap_add_cost(&mut Default::default())
            .expect("should fetch contract with history");

        assert!(result.is_some());
    }

    #[test]
    fn test_fetch_contract_with_known_keeps_history_false_discovers_tree() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let contract_path =
            "tests/supporting_files/contract/references/references_with_contract_history.json";
        let mut contract = json_document_to_contract(contract_path, false, platform_version)
            .expect("expected to get contract");
        contract.config_mut().set_keeps_history(true);
        contract.config_mut().set_readonly(false);

        drive
            .apply_contract(
                &contract,
                BlockInfo {
                    time_ms: 1000,
                    height: 100,
                    core_height: 10,
                    epoch: Default::default(),
                },
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("expected to apply contract successfully");

        // Fetch with known_keeps_history = Some(false) will get the raw element
        // at contract_root_path, discover it's a Tree, and follow the history path.
        // This exercises the Element::Tree(..) branch in fetch_contract_v0.
        let result = drive
            .fetch_contract(
                contract.id().to_buffer(),
                None,
                Some(false),
                None,
                platform_version,
            )
            .unwrap_add_cost(&mut Default::default())
            .expect("should fetch contract via tree branch");

        assert!(result.is_some());
    }

    #[test]
    fn test_contract_fetch_info_fixtures_can_be_applied_and_fetched() {
        use crate::drive::contract::DataContractFetchInfo;

        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        // Build fixture contracts
        let fixtures = [
            DataContractFetchInfo::dpns_contract_fixture(platform_version.protocol_version),
            DataContractFetchInfo::dashpay_contract_fixture(platform_version.protocol_version),
            DataContractFetchInfo::masternode_rewards_contract_fixture(
                platform_version.protocol_version,
            ),
            DataContractFetchInfo::withdrawals_contract_fixture(platform_version.protocol_version),
        ];

        // Apply each fixture to drive, then fetch it back and verify
        for fixture in &fixtures {
            assert!(fixture.fee.is_some(), "fixture should have a fee");

            drive
                .apply_contract(
                    &fixture.contract,
                    BlockInfo::default(),
                    true,
                    StorageFlags::optional_default_as_cow(),
                    None,
                    platform_version,
                )
                .expect("expected to apply fixture contract successfully");

            let fetched = drive
                .get_contract_with_fetch_info(
                    fixture.contract.id().to_buffer(),
                    true,
                    None,
                    platform_version,
                )
                .expect("should fetch contract")
                .expect("contract should exist");

            assert_eq!(
                fetched.contract.id(),
                fixture.contract.id(),
                "fetched contract id should match fixture"
            );
        }
    }

    #[test]
    fn test_apply_contract_with_history_keeps_history_insert_and_update() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let contract_path =
            "tests/supporting_files/contract/references/references_with_contract_history.json";
        let mut contract = json_document_to_contract(contract_path, false, platform_version)
            .expect("expected to get contract");

        // Apply initial contract (insert path with history)
        let fee = drive
            .apply_contract(
                &contract,
                BlockInfo {
                    time_ms: 1000,
                    height: 100,
                    core_height: 10,
                    epoch: Default::default(),
                },
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("expected to apply contract successfully");
        assert!(fee.processing_fee > 0);

        // Now update via apply_contract (update path with history)
        let new_schema = platform_value!({
            "type": "object",
            "properties": {
                "extra_field": {
                    "type": "string",
                    "position": 0
                }
            },
            "additionalProperties": false,
        });
        contract
            .set_document_schema("extraDoc", new_schema, true, &mut vec![], platform_version)
            .expect("should set schema");
        contract.increment_version();

        let fee2 = drive
            .apply_contract(
                &contract,
                BlockInfo {
                    time_ms: 2000,
                    height: 200,
                    core_height: 20,
                    epoch: Default::default(),
                },
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("expected to apply updated contract with history successfully");
        assert!(fee2.processing_fee > 0);
    }

    #[test]
    fn test_prove_contract_with_history() {
        let (drive, contract) = setup_history_contract();
        let platform_version = PlatformVersion::latest();

        // Prove a contract that keeps history - exercises the Tree branch in prove_contract_v0
        let proof = drive
            .prove_contract(contract.id().to_buffer(), None, platform_version)
            .expect("should prove history contract");

        // Verify proof using Drive::verify_contract
        let (_root_hash, verified_contract) = Drive::verify_contract(
            &proof,
            None,
            false,
            false,
            contract.id().to_buffer(),
            platform_version,
        )
        .expect("should verify history contract proof");
        assert!(
            verified_contract.is_some(),
            "history contract should be present in proof"
        );
        assert_eq!(
            verified_contract
                .expect("verified contract should exist")
                .id(),
            contract.id()
        );
    }

    #[test]
    fn test_prove_contracts_with_mix_of_historical_and_non_historical() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        // Create a non-historical contract
        let contract_path = "tests/supporting_files/contract/references/references.json";
        let non_hist_contract = json_document_to_contract(contract_path, false, platform_version)
            .expect("expected to get contract");
        drive
            .apply_contract(
                &non_hist_contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("expected to apply contract");

        // Create a historical contract
        let hist_contract_path =
            "tests/supporting_files/contract/references/references_with_contract_history.json";
        let hist_contract = json_document_to_contract(hist_contract_path, false, platform_version)
            .expect("expected to get contract");
        drive
            .apply_contract(
                &hist_contract,
                BlockInfo {
                    time_ms: 1000,
                    height: 100,
                    core_height: 10,
                    epoch: Default::default(),
                },
                true,
                None,
                None,
                platform_version,
            )
            .expect("expected to apply contract");

        // Prove both - exercises the mixed historical/non-historical path in prove_contracts_v0
        let contract_ids = [
            non_hist_contract.id().to_buffer(),
            hist_contract.id().to_buffer(),
        ];
        let proof = drive
            .prove_contracts(&contract_ids, None, platform_version)
            .expect("should prove mixed contracts");

        // Verify each contract individually as a subset of the proof
        let (_root_hash_1, verified_non_hist) = Drive::verify_contract(
            &proof,
            Some(false),
            true,
            true,
            non_hist_contract.id().to_buffer(),
            platform_version,
        )
        .expect("should verify non-historical contract from mixed proof");
        assert!(
            verified_non_hist.is_some(),
            "non-historical contract should be present in proof"
        );

        let (_root_hash_2, verified_hist) = Drive::verify_contract(
            &proof,
            None,
            true,
            true,
            hist_contract.id().to_buffer(),
            platform_version,
        )
        .expect("should verify historical contract from mixed proof");
        assert!(
            verified_hist.is_some(),
            "historical contract should be present in proof"
        );
    }
}
