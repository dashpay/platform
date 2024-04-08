// MIT LICENSE
//
// Copyright (c) 2021 Dash Core Group
//
// Permission is hereby granted, free of charge, to any
// person obtaining a copy of this software and associated
// documentation files (the "Software"), to deal in the
// Software without restriction, including without
// limitation the rights to use, copy, modify, merge,
// publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software
// is furnished to do so, subject to the following
// conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions
// of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
// ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
// TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
// PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
// SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
// CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
// OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
// IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.
//

//! DriveDataContracts.
//!
//! This module defines functions pertinent toDataContracts stored in Drive.
//!

#[cfg(feature = "full")]
mod apply;
#[cfg(any(feature = "full", feature = "verify"))]
mod contract_fetch_info;
#[cfg(feature = "full")]
mod estimation_costs;
#[cfg(feature = "full")]
mod get_fetch;
#[cfg(feature = "full")]
mod insert;
/// Various paths for contract operations
#[cfg(any(feature = "full", feature = "verify"))]
pub mod paths;
#[cfg(feature = "full")]
pub(crate) mod prove;
#[cfg(any(feature = "full", feature = "verify"))]
pub(crate) mod queries;
#[cfg(feature = "fixtures-and-mocks")]
/// Test helpers and utility methods
pub mod test_helpers;
#[cfg(feature = "full")]
mod update;
#[cfg(any(feature = "full", feature = "verify"))]
pub use contract_fetch_info::*;

/// How many contracts to fetch at once. This is an arbitrary number and is needed to prevent
/// the server from being overloaded with requests.
pub const MAX_CONTRACT_HISTORY_FETCH_LIMIT: u16 = 10;

#[cfg(feature = "full")]
#[cfg(test)]
mod tests {
    use dpp::block::block_info::BlockInfo;
    use rand::prelude::StdRng;
    use rand::{random, SeedableRng};
    use std::borrow::Cow;
    use std::option::Option::None;

    use crate::drive::flags::StorageFlags;
    use crate::drive::object_size_info::{
        DocumentAndContractInfo, DocumentInfo, OwnedDocumentInfo,
    };
    use crate::drive::Drive;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
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
    use crate::tests::helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::version::PlatformVersion;

    #[allow(dead_code)]
    #[deprecated(note = "This function is marked as unused.")]
    #[allow(deprecated)]
    fn setup_deep_nested_50_contract() -> (Drive, DataContract) {
        let drive: Drive = setup_drive_with_initial_state_structure();
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
        let drive: Drive = setup_drive_with_initial_state_structure();
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
        let drive: Drive = setup_drive_with_initial_state_structure();
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
        let drive: Drive = setup_drive_with_initial_state_structure();
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
        let drive: Drive = setup_drive_with_initial_state_structure();
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
    //         .add_document_for_contract(
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
        assert_eq!(&identity_keys.first().unwrap().1, &encryption_key);
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
        assert_eq!(&identity_keys.first().unwrap().1, &encryption_key);
    }

    #[test]
    fn test_create_reference_contract_without_apply() {
        let drive: Drive = setup_drive_with_initial_state_structure();
        let platform_version = PlatformVersion::latest();

        let contract_path = "tests/supporting_files/contract/references/references.json";

        // let's construct the grovedb structure for the dashpay data contract
        let contract = json_document_to_contract(contract_path, false, platform_version)
            .expect("expected to get cbor document");
        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                false,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("expected to apply contract successfully");
    }

    #[test]
    fn test_create_reference_contract_with_history_without_apply() {
        let drive: Drive = setup_drive_with_initial_state_structure();
        let platform_version = PlatformVersion::latest();

        let contract_path =
            "tests/supporting_files/contract/references/references_with_contract_history.json";

        // let's construct the grovedb structure for the dashpay data contract
        let contract = json_document_to_contract(contract_path, false, platform_version)
            .expect("expected to get contract");
        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                false,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("expected to apply contract successfully");
    }

    #[test]
    fn test_update_reference_contract_without_apply() {
        let drive: Drive = setup_drive_with_initial_state_structure();
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
            )
            .expect("expected to apply contract successfully");
    }
}
