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
    use rand::Rng;
    use std::borrow::Cow;
    use std::option::Option::None;
    use tempfile::TempDir;

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
    use dpp::platform_value::platform_value;
    use dpp::tests::json_document::json_document_to_contract;

    use dpp::version::PlatformVersion;

    fn setup_deep_nested_50_contract() -> (Drive, DataContract) {
        // Todo: make TempDir based on _prefix
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir, None).expect("expected to open Drive successfully");

        let platform_version = PlatformVersion::latest();
        drive
            .create_initial_state_structure(None, platform_version)
            .expect("expected to create root tree successfully");

        let contract_path = "tests/supporting_files/contract/deepNested/deep-nested50.json";
        // let's construct the grovedb structure for the dashpay data contract
        let contract = json_document_to_contract(contract_path, platform_version)
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
    fn setup_deep_nested_10_contract() -> (Drive, DataContract) {
        // Todo: make TempDir based on _prefix
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir, None).expect("expected to open Drive successfully");
        let platform_version = PlatformVersion::latest();

        drive
            .create_initial_state_structure(None, platform_version)
            .expect("expected to create root tree successfully");

        let contract_path = "tests/supporting_files/contract/deepNested/deep-nested10.json";
        // let's construct the grovedb structure for the dashpay data contract
        let contract = json_document_to_contract(contract_path, platform_version)
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
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir, None).expect("expected to open Drive successfully");
        let platform_version = PlatformVersion::latest();

        drive
            .create_initial_state_structure(None, platform_version)
            .expect("expected to create root tree successfully");

        let contract_path = "tests/supporting_files/contract/references/references.json";

        // let's construct the grovedb structure for the dashpay data contract
        let contract = json_document_to_contract(contract_path, platform_version)
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

    #[test]
    fn test_create_and_update_contract() {
        let (drive, mut contract) = setup_reference_contract();

        let platform_version = PlatformVersion::latest();

        let note2_schema = platform_value!({
            "type": "object",
            "properties": {
                "last_name": {
                    "type": "number"
                },
                "first_name": {
                    "type": "integer"
                }
            },
            "additionalProperties": false,
        });

        contract
            .set_document_schema("note2", note2_schema, true, platform_version)
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

    #[test]
    fn test_create_deep_nested_contract_50() {
        let (drive, contract) = setup_deep_nested_50_contract();
        let platform_version = PlatformVersion::latest();

        let document_type = contract
            .document_type_for_name("nest")
            .expect("expected to get document type");

        let document = document_type
            .random_document(Some(5), platform_version)
            .expect("expected to get random document");

        let nested_value = document.properties().get("abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0");

        assert!(nested_value.is_some());

        let storage_flags = Some(Cow::Owned(StorageFlags::SingleEpoch(0)));

        let random_owner_id = rand::thread_rng().gen::<[u8; 32]>();
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

        let random_owner_id = rand::thread_rng().gen::<[u8; 32]>();
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
    fn test_create_reference_contract_without_apply() {
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir, None).expect("expected to open Drive successfully");
        let platform_version = PlatformVersion::latest();

        drive
            .create_initial_state_structure(None, platform_version)
            .expect("expected to create root tree successfully");

        let contract_path = "tests/supporting_files/contract/references/references.json";

        // let's construct the grovedb structure for the dashpay data contract
        let contract = json_document_to_contract(contract_path, platform_version)
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
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir, None).expect("expected to open Drive successfully");
        let platform_version = PlatformVersion::latest();

        drive
            .create_initial_state_structure(None, platform_version)
            .expect("expected to create root tree successfully");

        let contract_path =
            "tests/supporting_files/contract/references/references_with_contract_history.json";

        // let's construct the grovedb structure for the dashpay data contract
        let contract = json_document_to_contract(contract_path, platform_version)
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
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir, None).expect("expected to open Drive successfully");
        let platform_version = PlatformVersion::latest();

        drive
            .create_initial_state_structure(None, platform_version)
            .expect("expected to create root tree successfully");

        let contract_path = "tests/supporting_files/contract/references/references.json";

        // let's construct the grovedb structure for the dashpay data contract
        let contract = json_document_to_contract(contract_path, platform_version)
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
