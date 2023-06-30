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

//! Drive Contracts.
//!
//! This module defines functions pertinent to Contracts stored in Drive.
//!

#[cfg(feature = "full")]
mod estimation_costs;
/// Various paths for contract operations
#[cfg(any(feature = "full", feature = "verify"))]
pub(crate) mod paths;
#[cfg(feature = "full")]
pub(crate) mod prove;
#[cfg(any(feature = "full", feature = "verify"))]
pub(crate) mod queries;
#[cfg(feature = "full")]
mod insert;
#[cfg(feature = "full")]
mod update;
#[cfg(feature = "full")]
mod get_fetch;
#[cfg(feature = "full")]
mod apply;

/// How many contracts to fetch at once. This is an arbitrary number and is needed to prevent
/// the server from being overloaded with requests.
pub const MAX_CONTRACT_HISTORY_FETCH_LIMIT: u16 = 10;

#[cfg(feature = "full")]
/// Adds operations to the op batch relevant to initializing the contract's structure.
/// Namely it inserts an empty tree at the contract's root path.
pub fn add_init_contracts_structure_operations(batch: &mut GroveDbOpBatch) {
    batch.add_insert_empty_tree(vec![], vec![RootTree::ContractDocuments as u8]);
}

#[cfg(any(feature = "full", feature = "verify"))]
/// Contract and fetch information
#[derive(Default, PartialEq, Debug, Clone)]
pub struct ContractFetchInfo {
    /// The contract
    pub contract: Contract,
    /// The contract's potential storage flags
    pub storage_flags: Option<StorageFlags>,
    /// These are the operations that are used to fetch a contract
    /// This is only used on epoch change
    pub(crate) cost: OperationCost,
    /// The fee is updated every epoch based on operation costs
    /// Except if protocol version has changed in which case all the cache is cleared
    pub fee: Option<FeeResult>,
}



impl Drive {
    /// Applies a contract CBOR.
    pub fn apply_contract_cbor(
        &self,
        contract_cbor: Vec<u8>,
        contract_id: Option<[u8; 32]>,
        block_info: BlockInfo,
        apply: bool,
        storage_flags: Option<Cow<StorageFlags>>,
        transaction: TransactionArg,
    ) -> Result<FeeResult, Error> {
        // first we need to deserialize the contract
        let contract =
            DataContract::from_cbor_with_id(contract_cbor, contract_id.map(Identifier::from))?;

        self.apply_contract(&contract, block_info, apply, storage_flags, transaction)
    }
}

    /// Returns the contract with fetch info and operations with the given ID.
    pub fn query_contract_as_serialized(
        &self,
        contract_id: [u8; 32],
        encoding: QueryResultEncoding,
        transaction: TransactionArg,
    ) -> Result<Vec<u8>, Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = Vec::new();

        let contract_fetch_info = self.get_contract_with_fetch_info_and_add_to_operations(
            contract_id,
            None,
            false, //querying the contract should not lead to it being added to the cache
            transaction,
            &mut drive_operations,
        )?;

        let contract_value = match contract_fetch_info {
            None => Value::Null,
            Some(contract_fetch_info) => {
                let contract = &contract_fetch_info.contract;
                contract.to_object()?
            }
        };

        let value = platform_value!({ "contract": contract_value });

        encoding.encode_value(&value)
    }
}






#[cfg(feature = "full")]
#[cfg(test)]
mod tests {
    use crate::contract::CreateRandomDocument;
    use rand::Rng;
    use std::option::Option::None;
    use tempfile::TempDir;

    use super::*;
    use crate::contract::Contract;
    use crate::drive::flags::StorageFlags;
    use crate::drive::object_size_info::{
        DocumentAndContractInfo, DocumentInfo, OwnedDocumentInfo,
    };
    use crate::drive::Drive;
    use dpp::data_contract::extra::common::json_document_to_contract;

    use crate::tests::helpers::setup::setup_drive_with_initial_state_structure;

    fn setup_deep_nested_50_contract() -> (Drive, DataContract) {
        // Todo: make TempDir based on _prefix
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir, None).expect("expected to open Drive successfully");

        drive
            .create_initial_state_structure_0(None)
            .expect("expected to create root tree successfully");

        let contract_path = "tests/supporting_files/contract/deepNested/deep-nested50.json";
        // let's construct the grovedb structure for the dashpay data contract
        let contract =
            json_document_to_contract(contract_path).expect("expected to get a contract");
        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
            )
            .expect("expected to apply contract successfully");

        (drive, contract)
    }

    fn setup_deep_nested_10_contract() -> (Drive, DataContract) {
        // Todo: make TempDir based on _prefix
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir, None).expect("expected to open Drive successfully");

        drive
            .create_initial_state_structure_0(None)
            .expect("expected to create root tree successfully");

        let contract_path = "tests/supporting_files/contract/deepNested/deep-nested10.json";
        // let's construct the grovedb structure for the dashpay data contract
        let contract =
            json_document_to_contract(contract_path).expect("expected to get a contract");
        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
            )
            .expect("expected to apply contract successfully");

        (drive, contract)
    }

    fn setup_reference_contract() -> (Drive, Contract) {
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir, None).expect("expected to open Drive successfully");

        drive
            .create_initial_state_structure_0(None)
            .expect("expected to create root tree successfully");

        let contract_path = "tests/supporting_files/contract/references/references.json";

        // let's construct the grovedb structure for the dashpay data contract
        let contract =
            json_document_to_contract(contract_path).expect("expected to get a contract");
        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
            )
            .expect("expected to apply contract successfully");

        (drive, contract)
    }

    #[test]
    fn test_create_and_update_contract() {
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir, None).expect("expected to open Drive successfully");

        drive
            .create_initial_state_structure_0(None)
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
            )
            .expect("should update initial contract");
    }

    #[test]
    fn test_create_deep_nested_contract_50() {
        let (drive, contract) = setup_deep_nested_50_contract();

        let document_type = contract
            .document_type_for_name("nest")
            .expect("expected to get document type");

        let document = document_type.random_document(Some(5));

        let nested_value = document.properties.get("abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0");

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
            )
            .expect("expected to insert a document successfully");
    }

    #[test]
    fn test_create_reference_contract() {
        let (drive, contract) = setup_reference_contract();

        let document_type = contract
            .document_type_for_name("note")
            .expect("expected to get document type");

        let document = document_type.random_document(Some(5));

        let ref_value = document.properties.get("abc17");

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
            )
            .expect("expected to insert a document successfully");
    }

    #[test]
    fn test_create_reference_contract_without_apply() {
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir, None).expect("expected to open Drive successfully");

        drive
            .create_initial_state_structure_0(None)
            .expect("expected to create root tree successfully");

        let contract_path = "tests/supporting_files/contract/references/references.json";

        // let's construct the grovedb structure for the dashpay data contract
        let contract =
            json_document_to_contract(contract_path).expect("expected to get cbor document");
        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                false,
                StorageFlags::optional_default_as_cow(),
                None,
            )
            .expect("expected to apply contract successfully");
    }

    #[test]
    fn test_create_reference_contract_with_history_without_apply() {
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir, None).expect("expected to open Drive successfully");

        drive
            .create_initial_state_structure_0(None)
            .expect("expected to create root tree successfully");

        let contract_path =
            "tests/supporting_files/contract/references/references_with_contract_history.json";

        // let's construct the grovedb structure for the dashpay data contract
        let contract = json_document_to_contract(contract_path).expect("expected to get contract");
        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                false,
                StorageFlags::optional_default_as_cow(),
                None,
            )
            .expect("expected to apply contract successfully");
    }

    #[test]
    fn test_update_reference_contract_without_apply() {
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir, None).expect("expected to open Drive successfully");

        drive
            .create_initial_state_structure_0(None)
            .expect("expected to create root tree successfully");

        let contract_path = "tests/supporting_files/contract/references/references.json";

        // let's construct the grovedb structure for the dashpay data contract
        let contract =
            json_document_to_contract(contract_path).expect("expected to get cbor document");

        // Create a contract first
        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
            )
            .expect("expected to apply contract successfully");

        // Update existing contract
        drive
            .update_contract(&contract, BlockInfo::default(), false, None)
            .expect("expected to apply contract successfully");
    }

    mod get_contract_with_fetch_info {
        use super::*;
        use dpp::prelude::Identifier;

        #[test]
        fn should_get_contract_from_global_and_block_cache() {
            let (drive, mut contract) = setup_reference_contract();

            let transaction = drive.grove.start_transaction();

            contract.increment_version();

            drive
                .update_contract(&contract, BlockInfo::default(), true, Some(&transaction))
                .expect("should update contract");

            let fetch_info_from_database = drive
                .get_contract_with_fetch_info_and_fee(contract.id().to_buffer(), None, true, None)
                .expect("should get contract")
                .1
                .expect("should be present");

            assert_eq!(fetch_info_from_database.contract.version, 1);

            let fetch_info_from_cache = drive
                .get_contract_with_fetch_info_and_fee(
                    contract.id().to_buffer(),
                    None,
                    true,
                    Some(&transaction),
                )
                .expect("should get contract")
                .1
                .expect("should be present");

            assert_eq!(fetch_info_from_cache.contract.version, 2);
        }

        #[test]
        fn should_return_none_if_contract_not_exist() {
            let drive = setup_drive_with_initial_state_structure();

            let result = drive
                .get_contract_with_fetch_info_and_fee([0; 32], None, true, None)
                .expect("should get contract");

            assert!(result.0.is_none());
            assert!(result.1.is_none());
        }

        #[test]
        fn should_return_fees_for_non_existing_contract_if_epoch_is_passed() {
            let drive = setup_drive_with_initial_state_structure();

            let result = drive
                .get_contract_with_fetch_info_and_fee(
                    [0; 32],
                    Some(&Epoch::new(0).unwrap()),
                    true,
                    None,
                )
                .expect("should get contract");

            assert_eq!(
                result.0,
                Some(FeeResult {
                    processing_fee: 4060,
                    ..Default::default()
                })
            );

            assert!(result.1.is_none());
        }

        #[test]
        fn should_always_have_then_same_cost() {
            // Merk trees have own cache and depends on does contract node cached or not
            // we get could get different costs. To avoid of it we fetch contracts without tree caching

            let (drive, mut ref_contract) = setup_reference_contract();

            /*
             * Firstly, we create multiple contracts during block processing (in transaction)
             */

            let ref_contract_id_buffer = Identifier::from([0; 32]).to_buffer();

            let transaction = drive.grove.start_transaction();

            // Create more contracts to trigger re-balancing
            for i in 0..150u8 {
                ref_contract.id() = Identifier::from([i; 32]);

                drive
                    .apply_contract(
                        &ref_contract,
                        BlockInfo::default(),
                        true,
                        StorageFlags::optional_default_as_cow(),
                        Some(&transaction),
                    )
                    .expect("expected to apply contract successfully");
            }

            // Create a deep placed contract
            let contract_path = "tests/supporting_files/contract/deepNested/deep-nested10.json";
            let deep_contract =
                json_document_to_contract(contract_path).expect("expected to get cbor document");
            drive
                .apply_contract(
                    &deep_contract,
                    BlockInfo::default(),
                    true,
                    StorageFlags::optional_default_as_cow(),
                    Some(&transaction),
                )
                .expect("expected to apply contract successfully");

            let mut ref_contract_fetch_info_transactional = drive
                .get_contract_with_fetch_info_and_fee(
                    ref_contract_id_buffer,
                    Some(&Epoch::new(0).unwrap()),
                    true,
                    Some(&transaction),
                )
                .expect("got contract")
                .1
                .expect("got contract fetch info");

            let mut deep_contract_fetch_info_transactional = drive
                .get_contract_with_fetch_info_and_fee(
                    deep_contract.id().to_buffer(),
                    Some(&Epoch::new(0).unwrap()),
                    true,
                    Some(&transaction),
                )
                .expect("got contract")
                .1
                .expect("got contract fetch info");

            /*
             * Then we commit the block
             */

            // Commit transaction and merge block (transactional) cache to global cache
            transaction.commit().expect("expected to commit");

            let mut drive_cache = drive.cache.write().unwrap();
            drive_cache.cached_contracts.merge_block_cache();
            drop(drive_cache);

            /*
             * Contracts fetched with user query and during block execution must have equal costs
             */

            let deep_contract_fetch_info = drive
                .get_contract_with_fetch_info_and_fee(
                    deep_contract.id().to_buffer(),
                    None,
                    true,
                    None,
                )
                .expect("got contract")
                .1
                .expect("got contract fetch info");

            let ref_contract_fetch_info = drive
                .get_contract_with_fetch_info_and_fee(ref_contract_id_buffer, None, true, None)
                .expect("got contract")
                .1
                .expect("got contract fetch info");

            assert_eq!(
                deep_contract_fetch_info_transactional,
                deep_contract_fetch_info
            );

            assert_eq!(
                ref_contract_fetch_info_transactional,
                ref_contract_fetch_info
            );

            /*
             * User restarts the node
             */

            // Drop cache so contract will be fetched once again
            drive.drop_cache();

            /*
             * Other nodes weren't restarted so contracts queried by user after restart
             * must have the same costs as transactional contracts and contracts before
             * restart
             */

            let deep_contract_fetch_info_without_cache = drive
                .get_contract_with_fetch_info_and_fee(
                    deep_contract.id().to_buffer(),
                    None,
                    true,
                    None,
                )
                .expect("got contract")
                .1
                .expect("got contract fetch info");

            let ref_contract_fetch_info_without_cache = drive
                .get_contract_with_fetch_info_and_fee(ref_contract_id_buffer, None, true, None)
                .expect("got contract")
                .1
                .expect("got contract fetch info");

            // Remove fees to match with fetch with epoch provided
            let mut deep_contract_fetch_info_transactional_without_arc =
                Arc::make_mut(&mut deep_contract_fetch_info_transactional);

            deep_contract_fetch_info_transactional_without_arc.fee = None;

            let mut ref_contract_fetch_info_transactional_without_arc =
                Arc::make_mut(&mut ref_contract_fetch_info_transactional);

            ref_contract_fetch_info_transactional_without_arc.fee = None;

            assert_eq!(
                deep_contract_fetch_info_transactional,
                deep_contract_fetch_info_without_cache
            );
            assert_eq!(
                ref_contract_fetch_info_transactional,
                ref_contract_fetch_info_without_cache
            );

            /*
             * Let's imagine that many blocks were executed and the node is restarted again
             */
            drive.drop_cache();

            /*
             * Drive executes a new block
             */

            let transaction = drive.grove.start_transaction();

            // Create more contracts to trigger re-balancing
            for i in 150..200u8 {
                ref_contract.id() = Identifier::from([i; 32]);

                drive
                    .apply_contract(
                        &ref_contract,
                        BlockInfo::default(),
                        true,
                        StorageFlags::optional_default_as_cow(),
                        Some(&transaction),
                    )
                    .expect("expected to apply contract successfully");
            }

            /*
             * Other nodes weren't restarted so contracts fetched during block execution
             * should have the same cost as previously fetched contracts
             */

            let mut deep_contract_fetch_info_transactional2 = drive
                .get_contract_with_fetch_info_and_fee(
                    deep_contract.id().to_buffer(),
                    Some(&Epoch::new(0).unwrap()),
                    true,
                    Some(&transaction),
                )
                .expect("got contract")
                .1
                .expect("got contract fetch info");

            let mut ref_contract_fetch_info_transactional2 = drive
                .get_contract_with_fetch_info_and_fee(
                    ref_contract_id_buffer,
                    Some(&Epoch::new(0).unwrap()),
                    true,
                    Some(&transaction),
                )
                .expect("got contract")
                .1
                .expect("got contract fetch info");

            // Remove fees to match with fetch with epoch provided
            let mut deep_contract_fetch_info_transactional_without_arc =
                Arc::make_mut(&mut deep_contract_fetch_info_transactional2);

            deep_contract_fetch_info_transactional_without_arc.fee = None;

            let mut ref_contract_fetch_info_transactional_without_arc =
                Arc::make_mut(&mut ref_contract_fetch_info_transactional2);

            ref_contract_fetch_info_transactional_without_arc.fee = None;

            assert_eq!(
                ref_contract_fetch_info_transactional,
                ref_contract_fetch_info_transactional2,
            );

            assert_eq!(
                deep_contract_fetch_info_transactional,
                deep_contract_fetch_info_transactional2
            );
        }
    }

    pub mod fetch_contract_with_history {
        use super::*;
        use crate::error::drive::DriveError;
        use crate::error::Error;
        use dpp::block::extended_block_info::BlockInfo;
        use dpp::data_contract::DataContract;
        use dpp::tests::fixtures::get_data_contract_fixture;
        use serde_json::json;

        struct TestData {
            data_contract: DataContract,
            drive: Drive,
        }

        fn apply_contract(drive: &Drive, data_contract: &DataContract, block_info: BlockInfo) {
            drive
                .apply_contract(data_contract, block_info, true, None, None)
                .expect("to apply contract");
        }

        fn insert_n_contract_updates(data_contract: &DataContract, drive: &Drive, n: u64) {
            let updated_document_template = json!({
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string"
                    },
                    "newProp": {
                        "type": "integer",
                        "minimum": 0
                    }
                },
                "required": [
                "$createdAt"
                ],
                "additionalProperties": false
            });

            let mut data_contract = data_contract.clone();
            for i in 0..n {
                let mut updated_document = updated_document_template.clone();
                updated_document
                    .as_object_mut()
                    .expect("document to be an object")
                    .get_mut("properties")
                    .expect("properties to be present")
                    .as_object_mut()
                    .expect("properties to be an object")
                    .insert(
                        format!("newProp{}", i),
                        json!({"type": "integer", "minimum": 0}),
                    );

                data_contract
                    .set_document_schema("niceDocument".into(), updated_document)
                    .expect("to be able to set document schema");
                data_contract.increment_version();

                apply_contract(
                    drive,
                    &data_contract,
                    BlockInfo {
                        time_ms: 1000 * (i + 2),
                        height: 100 + i,
                        core_height: (10 + i) as u32,
                        epoch: Default::default(),
                    },
                );
            }
        }

        pub fn setup_history_test_with_n_updates(
            mut data_contract: DataContract,
            drive: &Drive,
            n: u64,
        ) -> DataContract {
            data_contract.config.keeps_history = true;
            data_contract.config.readonly = false;

            let original_data_contract = data_contract.clone();

            apply_contract(
                drive,
                &data_contract,
                BlockInfo {
                    time_ms: 1000,
                    height: 100,
                    core_height: 10,
                    epoch: Default::default(),
                },
            );

            insert_n_contract_updates(&data_contract, drive, n);

            original_data_contract
        }

        pub fn assert_property_exists(data_contract: &DataContract, property: &str) {
            let updated_document = data_contract
                .get_document_schema("niceDocument")
                .expect("to get document schema");
            let updated_document = updated_document.as_object().expect("to be an object");
            let properties = updated_document
                .get("properties")
                .expect("to have properties")
                .as_object()
                .expect("properties to be an object");

            let property_keys = properties
                .keys()
                .map(|key| key.to_string())
                .collect::<Vec<String>>();

            assert!(
                properties.contains_key(property),
                "expect property {} to exist. Instead found properties {:?}",
                property,
                property_keys
            );
        }

        fn setup_test() -> TestData {
            let data_contract = get_data_contract_fixture(None).data_contract;

            TestData {
                data_contract,
                drive: setup_drive_with_initial_state_structure(),
            }
        }

        #[test]
        pub fn should_fetch_10_latest_contract_without_offset_and_limit_and_start_date_0() {
            let test_case = TestCase {
                total_updates_to_apply: 20,
                start_at_date: 0,
                limit: None,
                offset: None,
                expected_length: 10,
                expected_error: None,
                query_non_existent_contract_id: false,
                contract_created_date_ms: 1000,
                update_period_interval_ms: 1000,
                // Contract created at 1000, 20 updates applied. The last update is at 21000
                // The 5th update from the latest update is 21000 - 10000 = 11000, plus since
                // the latest update is included into result, the expected oldest update date
                // is 12000.
                expected_oldest_update_date_in_result_ms: 12000,
                // 10th oldest update after 20 is 10.
                expected_oldest_update_index_in_result: 10,
                expect_result_to_include_original_contract: false,
            };

            run_single_test_case(test_case);
        }

        #[test]
        pub fn should_fetch_with_limit_without_offset() {
            let test_case = TestCase {
                total_updates_to_apply: 20,
                start_at_date: 0,
                limit: Some(5),
                offset: None,
                expected_length: 5,
                expected_error: None,
                query_non_existent_contract_id: false,
                contract_created_date_ms: 1000,
                update_period_interval_ms: 1000,
                expected_oldest_update_date_in_result_ms: 17000,
                expected_oldest_update_index_in_result: 15,
                expect_result_to_include_original_contract: false,
            };

            run_single_test_case(test_case);
        }

        #[test]
        pub fn should_fetch_without_limit_with_offset() {
            let test_case = TestCase {
                total_updates_to_apply: 20,
                start_at_date: 0,
                limit: None,
                offset: Some(5),
                expected_length: 10,
                expected_error: None,
                query_non_existent_contract_id: false,
                contract_created_date_ms: 1000,
                update_period_interval_ms: 1000,
                // Same as test case above, but with offset 5
                expected_oldest_update_date_in_result_ms: 7000,
                expected_oldest_update_index_in_result: 5,
                expect_result_to_include_original_contract: false,
            };

            run_single_test_case(test_case);
        }

        #[test]
        pub fn should_fetch_with_limit_with_offset() {
            let test_case = TestCase {
                total_updates_to_apply: 20,
                start_at_date: 0,
                limit: Some(5),
                offset: Some(5),
                expected_length: 5,
                expected_error: None,
                query_non_existent_contract_id: false,
                contract_created_date_ms: 1000,
                update_period_interval_ms: 1000,
                expected_oldest_update_date_in_result_ms: 12000,
                expected_oldest_update_index_in_result: 10,
                expect_result_to_include_original_contract: false,
            };

            run_single_test_case(test_case);
        }

        #[test]
        pub fn should_fetch_with_non_zero_start_date() {
            let test_case = TestCase {
                total_updates_to_apply: 20,
                start_at_date: 5000,
                limit: None,
                offset: None,
                expected_length: 10,
                expected_error: None,
                query_non_existent_contract_id: false,
                contract_created_date_ms: 1000,
                update_period_interval_ms: 1000,
                expected_oldest_update_date_in_result_ms: 12000,
                expected_oldest_update_index_in_result: 10,
                expect_result_to_include_original_contract: false,
            };

            run_single_test_case(test_case);
        }

        #[test]
        pub fn should_fail_with_limit_higher_than_10() {
            let test_case = TestCase {
                total_updates_to_apply: 20,
                start_at_date: 5000,
                limit: Some(11),
                offset: None,
                expected_length: 0,
                expected_error: Some(Error::Drive(DriveError::InvalidContractHistoryFetchLimit(
                    11,
                ))),
                query_non_existent_contract_id: false,
                contract_created_date_ms: 1000,
                update_period_interval_ms: 1000,
                expected_oldest_update_date_in_result_ms: 0,
                expected_oldest_update_index_in_result: 0,
                expect_result_to_include_original_contract: false,
            };

            run_single_test_case(test_case);
        }

        #[test]
        pub fn should_fail_with_limit_smaller_than_1() {
            let test_case = TestCase {
                total_updates_to_apply: 20,
                start_at_date: 5000,
                limit: Some(0),
                offset: None,
                expected_length: 0,
                expected_error: Some(Error::Drive(DriveError::InvalidContractHistoryFetchLimit(
                    0,
                ))),
                query_non_existent_contract_id: false,
                contract_created_date_ms: 1000,
                update_period_interval_ms: 1000,
                expected_oldest_update_date_in_result_ms: 0,
                expected_oldest_update_index_in_result: 0,
                expect_result_to_include_original_contract: false,
            };

            run_single_test_case(test_case);
        }

        #[test]
        pub fn should_fetch_empty_with_start_date_after_latest_update() {
            let test_case = TestCase {
                total_updates_to_apply: 20,
                start_at_date: 21001,
                limit: None,
                offset: None,
                expected_length: 0,
                expected_error: None,
                query_non_existent_contract_id: false,
                contract_created_date_ms: 1000,
                update_period_interval_ms: 1000,
                expected_oldest_update_date_in_result_ms: 0,
                expected_oldest_update_index_in_result: 0,
                expect_result_to_include_original_contract: false,
            };

            run_single_test_case(test_case);
        }

        #[test]
        pub fn should_return_empty_result_with_non_existent_contract_id() {
            let test_case = TestCase {
                total_updates_to_apply: 20,
                start_at_date: 5000,
                limit: None,
                offset: None,
                expected_length: 0,
                expected_error: None,
                query_non_existent_contract_id: true,
                contract_created_date_ms: 1000,
                update_period_interval_ms: 1000,
                expected_oldest_update_date_in_result_ms: 0,
                expected_oldest_update_index_in_result: 0,
                expect_result_to_include_original_contract: false,
            };

            run_single_test_case(test_case);
        }

        #[test]
        pub fn should_fetch_only_oldest_updates_with_offset_regardless_of_limit_when_not_enough_updates(
        ) {
            let test_case = TestCase {
                total_updates_to_apply: 15,
                start_at_date: 0,
                limit: Some(10),
                offset: Some(10),
                // 5 updates and the original contract
                expected_length: 6,
                expected_error: None,
                query_non_existent_contract_id: false,
                contract_created_date_ms: 1000,
                update_period_interval_ms: 1000,
                // The same as created date, since we only have 5 updates with such offset
                expected_oldest_update_date_in_result_ms: 1000,
                expected_oldest_update_index_in_result: 0,
                expect_result_to_include_original_contract: true,
            };

            run_single_test_case(test_case);
        }

        #[test]
        pub fn should_fetch_empty_history_when_offset_is_so_large_that_no_updates_can_be_fetched() {
            let test_case = TestCase {
                total_updates_to_apply: 15,
                start_at_date: 0,
                limit: Some(10),
                offset: Some(20),
                // With offset being larger than total updates, we should offset - total_updates
                // results, even if limit is set to 10
                expected_length: 0,
                expected_error: None,
                query_non_existent_contract_id: false,
                contract_created_date_ms: 1000,
                update_period_interval_ms: 1000,
                expected_oldest_update_date_in_result_ms: 0,
                expected_oldest_update_index_in_result: 0,
                expect_result_to_include_original_contract: false,
            };

            run_single_test_case(test_case);
        }

        #[test]
        pub fn should_fetch_with_limit_equals_total_updates() {
            let test_case = TestCase {
                total_updates_to_apply: 10,
                start_at_date: 0,
                limit: Some(10), // limit equals to total updates
                offset: None,
                expected_length: 10, // still should return 10 due to the constraint of maximum 10 results
                expected_error: None,
                query_non_existent_contract_id: false,
                contract_created_date_ms: 1000,
                update_period_interval_ms: 1000,
                expected_oldest_update_date_in_result_ms: 2000,
                expected_oldest_update_index_in_result: 0,
                expect_result_to_include_original_contract: false,
            };

            run_single_test_case(test_case);
        }

        #[test]
        pub fn should_fetch_only_latest_updates_if_updates_count_lower_than_the_limit() {
            let test_case = TestCase {
                total_updates_to_apply: 7,
                start_at_date: 0,
                limit: Some(10), // limit larger than total updates
                offset: None,
                expected_length: 8,
                expected_error: None,
                query_non_existent_contract_id: false,
                contract_created_date_ms: 1000,
                update_period_interval_ms: 1000,
                expected_oldest_update_date_in_result_ms: 1000,
                expected_oldest_update_index_in_result: 0,
                expect_result_to_include_original_contract: true,
            };

            run_single_test_case(test_case);
        }

        #[test]
        pub fn should_handle_when_no_updates_at_all() {
            let test_case = TestCase {
                total_updates_to_apply: 0,
                start_at_date: 0,
                limit: None,
                offset: None,
                expected_length: 1,
                expected_error: None,
                query_non_existent_contract_id: false,
                contract_created_date_ms: 1000,
                update_period_interval_ms: 1000,
                expected_oldest_update_date_in_result_ms: 1000,
                expected_oldest_update_index_in_result: 0,
                expect_result_to_include_original_contract: true,
            };

            run_single_test_case(test_case);
        }

        #[test]
        pub fn should_fetch_empty_when_start_date_is_in_future() {
            let test_case = TestCase {
                total_updates_to_apply: 10,
                start_at_date: 20000, // future date
                limit: None,
                offset: None,
                expected_length: 0,
                expected_error: None,
                query_non_existent_contract_id: false,
                contract_created_date_ms: 1000,
                update_period_interval_ms: 1000,
                expected_oldest_update_date_in_result_ms: 0,
                expected_oldest_update_index_in_result: 0,
                expect_result_to_include_original_contract: false,
            };

            run_single_test_case(test_case);
        }

        #[test]
        pub fn should_fetch_when_start_date_is_same_as_latest_update() {
            let test_case = TestCase {
                total_updates_to_apply: 10,
                // TODO: important! This date is exclusive, that's why we can't query
                //  with the same date as the latest update. Check if this is the correct
                //  behavior
                start_at_date: 10999,
                limit: None,
                offset: None,
                expected_length: 1,
                expected_error: None,
                query_non_existent_contract_id: false,
                contract_created_date_ms: 1000,
                update_period_interval_ms: 1000,
                expected_oldest_update_date_in_result_ms: 11000,
                expected_oldest_update_index_in_result: 9,
                expect_result_to_include_original_contract: false,
            };

            run_single_test_case(test_case);
        }

        struct TestCase {
            // Test set up parameters
            total_updates_to_apply: usize,
            contract_created_date_ms: u64,
            update_period_interval_ms: u64,

            // The query parameters
            start_at_date: u64,
            limit: Option<u16>,
            offset: Option<u16>,
            query_non_existent_contract_id: bool,

            // Expected outcomes
            expected_length: usize,
            expected_error: Option<Error>,
            expected_oldest_update_date_in_result_ms: u64,
            // The index of the oldest update in the result. So if we expect the oldest result
            // to be 10th update, then this value should be 9, because the index starts from 0
            // and not 1. It is used to generate property names in the updated contract, so we
            // can verify that the result is correct.
            expected_oldest_update_index_in_result: u64,

            expect_result_to_include_original_contract: bool,
        }

        fn run_single_test_case(test_case: TestCase) {
            let TestData {
                data_contract,
                drive,
            } = setup_test();

            let contract_id = if test_case.query_non_existent_contract_id {
                [0u8; 32]
            } else {
                *data_contract.id().as_bytes()
            };
            let original_data_contract = setup_history_test_with_n_updates(
                data_contract,
                &drive,
                test_case.total_updates_to_apply as u64,
            );

            let contract_history_result = drive.fetch_contract_with_history(
                contract_id,
                None,
                test_case.start_at_date,
                test_case.limit,
                test_case.offset,
            );

            match &test_case.expected_error {
                Some(expected_error) => {
                    assert!(contract_history_result.is_err());
                    // Error doesn't implement PartialEq, so we have to compare the strings
                    assert_eq!(
                        contract_history_result.unwrap_err().to_string(),
                        expected_error.to_string()
                    );
                }
                None => {
                    assert!(contract_history_result.is_ok());
                    let contract_history = contract_history_result.unwrap();
                    assert_eq!(contract_history.len(), test_case.expected_length);

                    for (i, (key, contract)) in contract_history.iter().enumerate() {
                        if i == 0 && test_case.expect_result_to_include_original_contract {
                            // TODO: this doesn't work because when we deserialize the contract
                            //  keeps_history is false for some reason!
                            assert_eq!(key, &test_case.contract_created_date_ms);
                            assert_eq!(contract, &original_data_contract);
                            continue;
                        }

                        let expected_key: u64 = test_case.expected_oldest_update_date_in_result_ms
                            + i as u64 * test_case.update_period_interval_ms;
                        assert_eq!(key, &expected_key);

                        let prop_index = if test_case.expect_result_to_include_original_contract {
                            // If we expect the result to include the original contract, then
                            // the first update will be the original contract, so we need to
                            // offset the index by 1
                            i - 1 + test_case.expected_oldest_update_index_in_result as usize
                        } else {
                            i + test_case.expected_oldest_update_index_in_result as usize
                        };

                        // When updating a contract, we add a new property to it
                        // TODO: this test actually applies incompatible updates to the contract
                        //  because we don't validate the contract in the apply function
                        assert_property_exists(contract, format!("newProp{}", prop_index).as_str());
                    }
                }
            }
        }
    }
}
