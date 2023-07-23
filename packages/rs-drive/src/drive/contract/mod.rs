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
pub(crate) mod paths;
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
    use dpp::data_contract::base::DataContractBaseMethodsV0;
    use rand::Rng;
    use std::borrow::Cow;
    use std::option::Option::None;
    use tempfile::TempDir;

    use super::*;
    use crate::drive::flags::StorageFlags;
    use crate::drive::object_size_info::{
        DocumentAndContractInfo, DocumentInfo, OwnedDocumentInfo,
    };
    use crate::drive::Drive;
    use dpp::data_contract::document_type::random_document::CreateRandomDocument;
    use dpp::data_contract::extra::common::json_document_to_contract;
    use dpp::data_contract::DataContract;
    use dpp::document::DocumentV0Getters;
    use dpp::version::drive_versions::DriveVersion;
    use dpp::version::PlatformVersion;

    use crate::tests::helpers::setup::setup_drive_with_initial_state_structure;

    fn setup_deep_nested_50_contract() -> (Drive, DataContract) {
        // Todo: make TempDir based on _prefix
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir, None).expect("expected to open Drive successfully");

        let platform_version = PlatformVersion::latest();
        drive
            .create_initial_state_structure(None, &platform_version)
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
                &platform_version,
            )
            .expect("expected to apply contract successfully");

        (drive, contract)
    }

    fn setup_deep_nested_10_contract() -> (Drive, DataContract) {
        // Todo: make TempDir based on _prefix
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir, None).expect("expected to open Drive successfully");
        let platform_version = PlatformVersion::latest();

        drive
            .create_initial_state_structure(None, &platform_version)
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
                &platform_version,
            )
            .expect("expected to apply contract successfully");

        (drive, contract)
    }

    pub(in crate::drive::contract) fn setup_reference_contract() -> (Drive, DataContract) {
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir, None).expect("expected to open Drive successfully");
        let platform_version = PlatformVersion::latest();

        drive
            .create_initial_state_structure(None, &platform_version)
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
                &platform_version,
            )
            .expect("expected to apply contract successfully");

        (drive, contract)
    }

    #[test]
    fn test_create_and_update_contract() {
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir, None).expect("expected to open Drive successfully");
        let platform_version = PlatformVersion::latest();

        drive
            .create_initial_state_structure(None, &platform_version)
            .expect("expected to create root tree successfully");

        let initial_contract_cbor = hex::decode("01a66324696458209c2b800c5ea525d032a9fda4dda22a896f1e763af5f0e15ae7f93882b7439d77652464656673a1686c6173744e616d65a1647479706566737472696e676724736368656d61783468747470733a2f2f736368656d612e646173682e6f72672f6470702d302d342d302f6d6574612f646174612d636f6e7472616374676f776e657249645820636d3188dfffe62efb10e20347ec6c41b3e49fa31cb757ef4bad6cd8f1c7f4b66776657273696f6e0169646f63756d656e7473a76b756e697175654461746573a56474797065666f626a65637467696e646963657382a3646e616d6566696e6465783166756e69717565f56a70726f7065727469657382a16a2463726561746564417463617363a16a2475706461746564417463617363a2646e616d6566696e646578326a70726f7065727469657381a16a2475706461746564417463617363687265717569726564836966697273744e616d656a246372656174656441746a247570646174656441746a70726f70657274696573a2686c6173744e616d65a1647479706566737472696e676966697273744e616d65a1647479706566737472696e67746164646974696f6e616c50726f70657274696573f46c6e696365446f63756d656e74a46474797065666f626a656374687265717569726564816a246372656174656441746a70726f70657274696573a1646e616d65a1647479706566737472696e67746164646974696f6e616c50726f70657274696573f46e6e6f54696d65446f63756d656e74a36474797065666f626a6563746a70726f70657274696573a1646e616d65a1647479706566737472696e67746164646974696f6e616c50726f70657274696573f46e707265747479446f63756d656e74a46474797065666f626a65637468726571756972656482686c6173744e616d656a247570646174656441746a70726f70657274696573a1686c6173744e616d65a1642472656670232f24646566732f6c6173744e616d65746164646974696f6e616c50726f70657274696573f46e7769746842797465417272617973a56474797065666f626a65637467696e646963657381a2646e616d6566696e646578316a70726f7065727469657381a16e6279746541727261794669656c6463617363687265717569726564816e6279746541727261794669656c646a70726f70657274696573a26e6279746541727261794669656c64a36474797065656172726179686d61784974656d731069627974654172726179f56f6964656e7469666965724669656c64a56474797065656172726179686d61784974656d731820686d696e4974656d73182069627974654172726179f570636f6e74656e744d656469615479706578216170706c69636174696f6e2f782e646173682e6470702e6964656e746966696572746164646974696f6e616c50726f70657274696573f46f696e6465786564446f63756d656e74a56474797065666f626a65637467696e646963657386a3646e616d6566696e6465783166756e69717565f56a70726f7065727469657382a168246f776e6572496463617363a16966697273744e616d656464657363a3646e616d6566696e6465783266756e69717565f56a70726f7065727469657382a168246f776e6572496463617363a1686c6173744e616d656464657363a2646e616d6566696e646578336a70726f7065727469657381a1686c6173744e616d6563617363a2646e616d6566696e646578346a70726f7065727469657382a16a2463726561746564417463617363a16a2475706461746564417463617363a2646e616d6566696e646578356a70726f7065727469657381a16a2475706461746564417463617363a2646e616d6566696e646578366a70726f7065727469657381a16a2463726561746564417463617363687265717569726564846966697273744e616d656a246372656174656441746a24757064617465644174686c6173744e616d656a70726f70657274696573a2686c6173744e616d65a2647479706566737472696e67696d61784c656e6774681901006966697273744e616d65a2647479706566737472696e67696d61784c656e677468190100746164646974696f6e616c50726f70657274696573f4781d6f7074696f6e616c556e69717565496e6465786564446f63756d656e74a56474797065666f626a65637467696e646963657383a3646e616d6566696e6465783166756e69717565f56a70726f7065727469657381a16966697273744e616d656464657363a3646e616d6566696e6465783266756e69717565f56a70726f7065727469657383a168246f776e6572496463617363a16966697273744e616d6563617363a1686c6173744e616d6563617363a3646e616d6566696e6465783366756e69717565f56a70726f7065727469657382a167636f756e74727963617363a1646369747963617363687265717569726564826966697273744e616d65686c6173744e616d656a70726f70657274696573a46463697479a2647479706566737472696e67696d61784c656e67746819010067636f756e747279a2647479706566737472696e67696d61784c656e677468190100686c6173744e616d65a2647479706566737472696e67696d61784c656e6774681901006966697273744e616d65a2647479706566737472696e67696d61784c656e677468190100746164646974696f6e616c50726f70657274696573f4").unwrap();

        drive
            .apply_contract_cbor(
                initial_contract_cbor,
                None,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("expected to apply contract successfully");

        let updated_contract_cbor = hex::decode("01a66324696458209c2b800c5ea525d032a9fda4dda22a896f1e763af5f0e15ae7f93882b7439d77652464656673a1686c6173744e616d65a1647479706566737472696e676724736368656d61783468747470733a2f2f736368656d612e646173682e6f72672f6470702d302d342d302f6d6574612f646174612d636f6e7472616374676f776e657249645820636d3188dfffe62efb10e20347ec6c41b3e49fa31cb757ef4bad6cd8f1c7f4b66776657273696f6e0269646f63756d656e7473a86b756e697175654461746573a56474797065666f626a65637467696e646963657382a3646e616d6566696e6465783166756e69717565f56a70726f7065727469657382a16a2463726561746564417463617363a16a2475706461746564417463617363a2646e616d6566696e646578326a70726f7065727469657381a16a2475706461746564417463617363687265717569726564836966697273744e616d656a246372656174656441746a247570646174656441746a70726f70657274696573a2686c6173744e616d65a1647479706566737472696e676966697273744e616d65a1647479706566737472696e67746164646974696f6e616c50726f70657274696573f46c6e696365446f63756d656e74a46474797065666f626a656374687265717569726564816a246372656174656441746a70726f70657274696573a1646e616d65a1647479706566737472696e67746164646974696f6e616c50726f70657274696573f46e6e6f54696d65446f63756d656e74a36474797065666f626a6563746a70726f70657274696573a1646e616d65a1647479706566737472696e67746164646974696f6e616c50726f70657274696573f46e707265747479446f63756d656e74a46474797065666f626a65637468726571756972656482686c6173744e616d656a247570646174656441746a70726f70657274696573a1686c6173744e616d65a1642472656670232f24646566732f6c6173744e616d65746164646974696f6e616c50726f70657274696573f46e7769746842797465417272617973a56474797065666f626a65637467696e646963657381a2646e616d6566696e646578316a70726f7065727469657381a16e6279746541727261794669656c6463617363687265717569726564816e6279746541727261794669656c646a70726f70657274696573a26e6279746541727261794669656c64a36474797065656172726179686d61784974656d731069627974654172726179f56f6964656e7469666965724669656c64a56474797065656172726179686d61784974656d731820686d696e4974656d73182069627974654172726179f570636f6e74656e744d656469615479706578216170706c69636174696f6e2f782e646173682e6470702e6964656e746966696572746164646974696f6e616c50726f70657274696573f46f696e6465786564446f63756d656e74a56474797065666f626a65637467696e646963657386a3646e616d6566696e6465783166756e69717565f56a70726f7065727469657382a168246f776e6572496463617363a16966697273744e616d656464657363a3646e616d6566696e6465783266756e69717565f56a70726f7065727469657382a168246f776e6572496463617363a1686c6173744e616d656464657363a2646e616d6566696e646578336a70726f7065727469657381a1686c6173744e616d6563617363a2646e616d6566696e646578346a70726f7065727469657382a16a2463726561746564417463617363a16a2475706461746564417463617363a2646e616d6566696e646578356a70726f7065727469657381a16a2475706461746564417463617363a2646e616d6566696e646578366a70726f7065727469657381a16a2463726561746564417463617363687265717569726564846966697273744e616d656a246372656174656441746a24757064617465644174686c6173744e616d656a70726f70657274696573a2686c6173744e616d65a2647479706566737472696e67696d61784c656e6774681901006966697273744e616d65a2647479706566737472696e67696d61784c656e677468190100746164646974696f6e616c50726f70657274696573f4716d79417765736f6d65446f63756d656e74a56474797065666f626a65637467696e646963657382a3646e616d656966697273744e616d6566756e69717565f56a70726f7065727469657381a16966697273744e616d6563617363a3646e616d657166697273744e616d654c6173744e616d6566756e69717565f56a70726f7065727469657382a16966697273744e616d6563617363a1686c6173744e616d6563617363687265717569726564846966697273744e616d656a246372656174656441746a24757064617465644174686c6173744e616d656a70726f70657274696573a2686c6173744e616d65a2647479706566737472696e67696d61784c656e6774681901006966697273744e616d65a2647479706566737472696e67696d61784c656e677468190100746164646974696f6e616c50726f70657274696573f4781d6f7074696f6e616c556e69717565496e6465786564446f63756d656e74a56474797065666f626a65637467696e646963657383a3646e616d6566696e6465783166756e69717565f56a70726f7065727469657381a16966697273744e616d656464657363a3646e616d6566696e6465783266756e69717565f56a70726f7065727469657383a168246f776e6572496463617363a16966697273744e616d6563617363a1686c6173744e616d6563617363a3646e616d6566696e6465783366756e69717565f56a70726f7065727469657382a167636f756e74727963617363a1646369747963617363687265717569726564826966697273744e616d65686c6173744e616d656a70726f70657274696573a46463697479a2647479706566737472696e67696d61784c656e67746819010067636f756e747279a2647479706566737472696e67696d61784c656e677468190100686c6173744e616d65a2647479706566737472696e67696d61784c656e6774681901006966697273744e616d65a2647479706566737472696e67696d61784c656e677468190100746164646974696f6e616c50726f70657274696573f4").unwrap();

        drive
            .apply_contract_cbor(
                updated_contract_cbor,
                None,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("should update initial contract");
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
                &platform_version,
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
                &platform_version,
            )
            .expect("expected to insert a document successfully");
    }

    #[test]
    fn test_create_reference_contract_without_apply() {
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir, None).expect("expected to open Drive successfully");
        let platform_version = PlatformVersion::latest();

        drive
            .create_initial_state_structure(None, &platform_version)
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
                &platform_version,
            )
            .expect("expected to apply contract successfully");
    }

    #[test]
    fn test_create_reference_contract_with_history_without_apply() {
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir, None).expect("expected to open Drive successfully");
        let platform_version = PlatformVersion::latest();

        drive
            .create_initial_state_structure(None, &platform_version)
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
                &platform_version,
            )
            .expect("expected to apply contract successfully");
    }

    #[test]
    fn test_update_reference_contract_without_apply() {
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir, None).expect("expected to open Drive successfully");
        let platform_version = PlatformVersion::latest();

        drive
            .create_initial_state_structure(None, &platform_version)
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
                &platform_version,
            )
            .expect("expected to apply contract successfully");
    }
}
