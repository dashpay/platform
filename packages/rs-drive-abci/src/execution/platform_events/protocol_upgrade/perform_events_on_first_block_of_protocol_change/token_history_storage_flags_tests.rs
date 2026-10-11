use crate::rpc::core::MockCoreRPCLike;
use crate::test::helpers::setup::{TempPlatform, TestPlatformBuilder};
use dpp::block::block_info::BlockInfo;
use dpp::block::epoch::Epoch;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::random_document::CreateRandomDocument;
use dpp::data_contracts::SystemDataContract;
use dpp::document::DocumentV0Getters;
use dpp::version::PlatformVersion;
use drive::drive::contract::paths::{
    all_contracts_global_root_path, contract_other_path, contract_root_path, CONTRACT_VERSION_KEY,
};
use drive::fees::op::LowLevelDriveOperation;
use drive::grovedb::batch::QualifiedGroveDbOp;
use drive::grovedb::operations::delete::DeleteOptions;
use drive::grovedb::query_result_type::{QueryResultElement, QueryResultType};
use drive::grovedb::{Element, PathQuery, Query, SizedQuery, TransactionArg};
use drive::util::object_size_info::DocumentInfo::DocumentRefInfo;
use drive::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
use drive::util::storage_flags::StorageFlags;
use std::collections::BTreeMap;

fn block(epoch: u16) -> BlockInfo {
    BlockInfo {
        time_ms: 1_000_000 + u64::from(epoch) * 1000,
        height: 100 + u64::from(epoch),
        core_height: 100,
        epoch: Epoch::new(epoch).expect("epoch"),
    }
}

fn historical_platform() -> TempPlatform<MockCoreRPCLike> {
    let platform = TestPlatformBuilder::new()
        .with_initial_protocol_version(8)
        .build_with_mock_rpc()
        .set_genesis_state();
    let old = PlatformVersion::get(13).expect("PV13");
    let tx = platform.drive.grove.start_transaction();
    platform
        .perform_events_on_first_block_of_protocol_change(
            &platform.state.load(),
            &block(3),
            &tx,
            8,
            old,
        )
        .expect("historical ladder installs TokenHistory through insert_contract");
    platform
        .drive
        .commit_transaction(tx, &old.drive)
        .expect("historical commit");
    platform
        .drive
        .cache
        .data_contracts
        .merge_and_clear_block_cache();
    let info = platform
        .drive
        .fetch_contract(
            SystemDataContract::TokenHistory.id().to_buffer(),
            None,
            None,
            None,
            old,
        )
        .value
        .expect("stored contract")
        .expect("TokenHistory exists");
    assert_eq!(info.contract.version(), 1);
    assert_eq!(
        info.storage_flags,
        Some(StorageFlags::new_single_epoch(3, Some([0; 32])))
    );
    platform
}

fn version_item(platform: &TempPlatform<MockCoreRPCLike>, tx: TransactionArg) -> Element {
    let id = SystemDataContract::TokenHistory.id().to_buffer();
    platform
        .drive
        .grove
        .get(
            &contract_other_path(&id),
            &[CONTRACT_VERSION_KEY],
            tx,
            &PlatformVersion::latest().drive.grove_version,
        )
        .value
        .expect("version item")
}

fn assert_upgraded(platform: &TempPlatform<MockCoreRPCLike>, tx: TransactionArg) {
    let pv = PlatformVersion::latest();
    let info = platform
        .drive
        .fetch_contract(
            SystemDataContract::TokenHistory.id().to_buffer(),
            None,
            None,
            tx,
            pv,
        )
        .value
        .expect("upgraded contract")
        .expect("TokenHistory exists");
    assert_eq!(info.contract.version(), 2);
    let schema = info
        .contract
        .document_type_for_name("claim")
        .expect("claim type")
        .schema()
        .clone()
        .try_into_validating_json()
        .expect("claim schema");
    assert_eq!(
        schema["properties"]["distributionType"]["enum"],
        serde_json::json!([0, 1, 2])
    );
    assert_eq!(
        version_item(platform, tx),
        Element::Item(
            2u32.to_be_bytes().to_vec(),
            Some(StorageFlags::new_single_epoch(3, Some([0; 32])).to_element_flags())
        )
    );
}

#[test]
fn should_upgrade_historically_owned_token_history_without_version_item_cost_mismatch() {
    let platform = historical_platform();
    let pv = PlatformVersion::latest();
    platform.drive.cache.data_contracts.clear_block_cache();
    let tx = platform.drive.grove.start_transaction();
    let result = platform.perform_events_on_first_block_of_protocol_change(
        &platform.state.load(),
        &block(7),
        &tx,
        13,
        pv,
    );
    assert!(
        result.is_ok(),
        "historically flagged TokenHistory upgrade failed: {result:?}"
    );
    assert_upgraded(&platform, Some(&tx));
}

#[test]
fn should_attribute_unflagged_replacement_cost_mismatch_to_token_history_version_item() {
    let platform = historical_platform();
    let pv = PlatformVersion::latest();
    let tx = platform.drive.grove.start_transaction();
    platform
        .drive
        .add_version_items_to_all_contracts(&tx, pv)
        .expect("version migration");
    let old = version_item(&platform, Some(&tx));
    assert_eq!(
        old,
        Element::Item(
            1u32.to_be_bytes().to_vec(),
            Some(StorageFlags::new_single_epoch(3, Some([0; 32])).to_element_flags())
        )
    );
    let new = Element::Item(2u32.to_be_bytes().to_vec(), None);
    assert_eq!(
        old.serialize(&pv.drive.grove_version)
            .expect("flagged encoding")
            .len(),
        43
    );
    assert_eq!(
        new.serialize(&pv.drive.grove_version)
            .expect("unflagged encoding")
            .len(),
        7
    );
    let id = SystemDataContract::TokenHistory.id().to_buffer();
    let op = QualifiedGroveDbOp::insert_or_replace_op(
        contract_other_path(&id)
            .iter()
            .map(|p| p.to_vec())
            .collect(),
        vec![CONTRACT_VERSION_KEY],
        new,
    );
    let error = platform
        .drive
        .grove_apply_batch(
            LowLevelDriveOperation::grovedb_operations_batch_consume(vec![
                LowLevelDriveOperation::GroveOperation(op),
            ]),
            false,
            Some(&tx),
            &pv.drive,
        )
        .expect_err("unflagged replacement must expose the accounting mismatch");
    assert!(
        error
            .to_string()
            .contains("storage_cost cost mismatch added: 0 replaced: 146 actual:110"),
        "{error}"
    );
    eprintln!(
        "attributed path=[64,{},2]/64 old={old:?} error={error}",
        hex::encode(id)
    );
}

fn contract_element(platform: &TempPlatform<MockCoreRPCLike>, tx: TransactionArg) -> Element {
    platform
        .drive
        .grove
        .get_raw(
            (&contract_root_path(SystemDataContract::TokenHistory.id().as_bytes())).into(),
            &[0],
            tx,
            &PlatformVersion::latest().drive.grove_version,
        )
        .value
        .expect("contract element")
}

fn document_elements(
    platform: &TempPlatform<MockCoreRPCLike>,
    path: Vec<Vec<u8>>,
    tx: TransactionArg,
) -> BTreeMap<Vec<Vec<u8>>, Vec<u8>> {
    let pv = PlatformVersion::latest();
    let mut query = Query::new();
    query.insert_all();
    let path_query = PathQuery {
        path: path.clone(),
        query: SizedQuery {
            query,
            limit: None,
            offset: None,
        },
    };
    let (rows, skipped) = platform
        .drive
        .grove
        .query_raw(
            &path_query,
            true,
            true,
            true,
            QueryResultType::QueryKeyElementPairResultType,
            tx,
            &pv.drive.grove_version,
        )
        .value
        .expect("raw document/index entries");
    assert_eq!(skipped, 0);
    let mut output = BTreeMap::new();
    for row in rows.elements {
        let QueryResultElement::KeyElementPairResultItem((key, element)) = row else {
            panic!("key/element row")
        };
        let mut child = path.clone();
        child.push(key);
        output.insert(
            child.clone(),
            element
                .serialize(&pv.drive.grove_version)
                .expect("element bytes"),
        );
        if element.is_any_tree() {
            output.extend(document_elements(platform, child, tx));
        }
    }
    output
}

fn documents_path() -> Vec<Vec<u8>> {
    let id = SystemDataContract::TokenHistory.id().to_buffer();
    let mut path = contract_root_path(&id)
        .iter()
        .map(|part| part.to_vec())
        .collect::<Vec<_>>();
    path.push(vec![1]);
    path
}

fn insert_claim(platform: &TempPlatform<MockCoreRPCLike>) {
    let pv = PlatformVersion::get(13).expect("PV13");
    let stored = platform
        .drive
        .fetch_contract(
            SystemDataContract::TokenHistory.id().to_buffer(),
            None,
            None,
            None,
            pv,
        )
        .value
        .expect("stored contract")
        .expect("TokenHistory");
    let document_type = stored
        .contract
        .document_type_for_name("claim")
        .expect("claim");
    let document = document_type
        .random_document(Some(41), pv)
        .expect("legacy claim");
    platform
        .drive
        .add_document_for_contract(
            DocumentAndContractInfo {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentRefInfo((&document, None)),
                    owner_id: Some(document.owner_id().to_buffer()),
                },
                contract: &stored.contract,
                document_type,
            },
            false,
            block(3),
            true,
            None,
            pv,
            None,
        )
        .expect("persist legacy claim and indexes");
}

#[test]
fn should_preserve_token_history_flag_owners_and_allocation_epochs() {
    let pv = PlatformVersion::latest();
    for (flags, epoch) in [
        (StorageFlags::new_single_epoch(3, Some([0; 32])), 3),
        (StorageFlags::new_single_epoch(3, Some([99; 32])), 7),
        (StorageFlags::new_single_epoch(3, None), 7),
        (
            StorageFlags::MultiEpochOwned(3, BTreeMap::from([(5, 50)]), [99; 32]),
            7,
        ),
        (StorageFlags::MultiEpoch(3, BTreeMap::from([(5, 50)])), 7),
        (
            StorageFlags::MultiEpochOwned(3, BTreeMap::from([(7, 50)]), [99; 32]),
            7,
        ),
        (StorageFlags::MultiEpoch(3, BTreeMap::from([(7, 50)])), 7),
    ] {
        let platform = historical_platform();
        insert_claim(&platform);
        let documents = document_elements(&platform, documents_path(), None);
        assert!(!documents.is_empty());
        let Element::Item(bytes, _) = contract_element(&platform, None) else {
            panic!("contract Item")
        };
        let old_len = bytes.len();
        let tx = platform.drive.grove.start_transaction();
        platform
            .drive
            .grove
            .insert(
                &contract_root_path(SystemDataContract::TokenHistory.id().as_bytes()),
                &[0],
                Element::Item(bytes, Some(flags.to_element_flags())),
                None,
                Some(&tx),
                &pv.drive.grove_version,
            )
            .value
            .expect("fixture flag metadata");
        platform
            .drive
            .commit_transaction(tx, &pv.drive)
            .expect("commit fixture flags");
        let tx = platform.drive.grove.start_transaction();
        platform
            .perform_events_on_first_block_of_protocol_change(
                &platform.state.load(),
                &block(epoch),
                &tx,
                13,
                pv,
            )
            .expect("flagged activation");
        assert_eq!(
            version_item(&platform, Some(&tx)),
            Element::Item(2u32.to_be_bytes().to_vec(), Some(flags.to_element_flags()))
        );
        let Element::Item(bytes, Some(new_flags)) = contract_element(&platform, Some(&tx)) else {
            panic!("flagged contract")
        };
        let new_flags = StorageFlags::from_element_flags_ref(&new_flags)
            .expect("decode flags")
            .expect("nonempty flags");
        assert_eq!(bytes.len(), old_len + 105);
        assert_eq!(new_flags.owner_id(), flags.owner_id());
        assert_eq!(new_flags.base_epoch(), flags.base_epoch());
        if let Some(old_allocations) = flags.epoch_index_map() {
            for (allocation_epoch, bytes) in old_allocations {
                if *allocation_epoch != epoch {
                    assert_eq!(
                        new_flags
                            .epoch_index_map()
                            .and_then(|map| map.get(allocation_epoch)),
                        Some(bytes)
                    );
                }
            }
        }
        if epoch != *flags.base_epoch() {
            let existing_allocation = flags
                .epoch_index_map()
                .is_some_and(|map| map.contains_key(&epoch));
            assert_eq!(
                new_flags.epoch_index_map().and_then(|map| map.get(&epoch)),
                Some(&if existing_allocation { 156 } else { 108 })
            );
            assert_eq!(
                new_flags.epoch_index_map().expect("allocation map").len(),
                flags.epoch_index_map().map_or(0, BTreeMap::len)
                    + usize::from(!existing_allocation)
            );
        } else {
            assert_eq!(new_flags, flags);
        }
        eprintln!("flags={flags:?} activation_epoch={epoch} before_bytes={old_len} after_bytes={} merged={new_flags:?}", bytes.len());
        assert_eq!(
            document_elements(&platform, documents_path(), Some(&tx)),
            documents
        );
        assert!(platform
            .drive
            .grove
            .verify_grovedb(Some(&tx), true, false, &pv.drive.grove_version)
            .expect("native integrity")
            .is_empty());
        platform
            .drive
            .commit_transaction(tx, &pv.drive)
            .expect("commit activation");
        assert_eq!(
            document_elements(&platform, documents_path(), None),
            documents
        );
    }
}

#[test]
fn should_keep_unflagged_and_absent_token_history_unflagged() {
    let pv = PlatformVersion::latest();
    for absent in [false, true] {
        let platform = TestPlatformBuilder::new()
            .with_initial_protocol_version(13)
            .build_with_mock_rpc()
            .set_genesis_state();
        if absent {
            platform
                .drive
                .grove
                .delete(
                    &all_contracts_global_root_path(),
                    SystemDataContract::TokenHistory.id().as_bytes(),
                    Some(DeleteOptions {
                        allow_deleting_non_empty_trees: true,
                        ..Default::default()
                    }),
                    None,
                    &pv.drive.grove_version,
                )
                .value
                .expect("absent contract fixture");
        }
        let tx = platform.drive.grove.start_transaction();
        platform
            .perform_events_on_first_block_of_protocol_change(
                &platform.state.load(),
                &block(7),
                &tx,
                13,
                pv,
            )
            .expect("unflagged or absent activation");
        assert!(matches!(
            contract_element(&platform, Some(&tx)),
            Element::Item(_, None)
        ));
        assert_eq!(
            version_item(&platform, Some(&tx)),
            Element::Item(2u32.to_be_bytes().to_vec(), None)
        );
    }
}

#[test]
fn should_drop_retry_commit_and_reopen_historical_token_history_upgrade() {
    let platform = historical_platform();
    insert_claim(&platform);
    let pv = PlatformVersion::latest();
    let old_element = contract_element(&platform, None);
    let documents = document_elements(&platform, documents_path(), None);
    let old_root = platform
        .drive
        .grove
        .root_hash(None, &pv.drive.grove_version)
        .value
        .expect("old root");
    let mut candidate_roots = Vec::new();
    for commit in [false, true] {
        let tx = platform.drive.grove.start_transaction();
        platform
            .perform_events_on_first_block_of_protocol_change(
                &platform.state.load(),
                &block(7),
                &tx,
                13,
                pv,
            )
            .expect("candidate");
        assert_upgraded(&platform, Some(&tx));
        assert_eq!(contract_element(&platform, None), old_element);
        assert_eq!(
            platform
                .drive
                .grove
                .root_hash(None, &pv.drive.grove_version)
                .value
                .expect("committed root"),
            old_root
        );
        candidate_roots.push(
            platform
                .drive
                .grove
                .root_hash(Some(&tx), &pv.drive.grove_version)
                .value
                .expect("candidate root"),
        );
        if commit {
            platform
                .drive
                .commit_transaction(tx, &pv.drive)
                .expect("storage commit");
        } else {
            drop(tx);
            assert_eq!(contract_element(&platform, None), old_element);
        }
    }
    assert_eq!(candidate_roots[0], candidate_roots[1]);
    let config = platform.config.clone();
    let TempPlatform { platform, tempdir } = platform;
    drop(platform);
    let reopened = TempPlatform::open_with_tempdir(tempdir, config);
    assert_upgraded(&reopened, None);
    assert_eq!(
        document_elements(&reopened, documents_path(), None),
        documents
    );
    assert_eq!(
        reopened
            .drive
            .grove
            .root_hash(None, &pv.drive.grove_version)
            .value
            .expect("reopened root"),
        candidate_roots[1]
    );
}

#[test]
fn should_skip_historical_versions_and_preserve_current_token_history_noop() {
    let pv = PlatformVersion::latest();
    for previous in [8, 9, 12, 13, pv.protocol_version] {
        let platform = TestPlatformBuilder::new()
            .with_initial_protocol_version(previous)
            .build_with_mock_rpc()
            .set_genesis_state();
        let tx = platform.drive.grove.start_transaction();
        platform
            .perform_events_on_first_block_of_protocol_change(
                &platform.state.load(),
                &block(7),
                &tx,
                previous,
                pv,
            )
            .expect("version skip");
        let stored = platform
            .drive
            .fetch_contract(
                SystemDataContract::TokenHistory.id().to_buffer(),
                None,
                None,
                Some(&tx),
                pv,
            )
            .value
            .expect("fetch")
            .expect("TokenHistory");
        assert_eq!(stored.contract.version(), 2);
        let root = platform
            .drive
            .grove
            .root_hash(Some(&tx), &pv.drive.grove_version)
            .value
            .expect("first root");
        platform
            .perform_events_on_first_block_of_protocol_change(
                &platform.state.load(),
                &block(8),
                &tx,
                pv.protocol_version,
                pv,
            )
            .expect("same-version noop");
        assert_eq!(
            platform
                .drive
                .grove
                .root_hash(Some(&tx), &pv.drive.grove_version)
                .value
                .expect("repeat root"),
            root
        );
    }
}

#[test]
fn should_read_token_history_flag_presence_from_the_candidate_transaction() {
    let platform = historical_platform();
    let pv = PlatformVersion::latest();
    let original = contract_element(&platform, None);
    let Element::Item(bytes, _) = original.clone() else {
        panic!("contract Item")
    };
    let flags = StorageFlags::new_single_epoch(3, None);
    let tx = platform.drive.grove.start_transaction();
    platform
        .drive
        .grove
        .insert(
            &contract_root_path(SystemDataContract::TokenHistory.id().as_bytes()),
            &[0],
            Element::Item(bytes, Some(flags.to_element_flags())),
            None,
            Some(&tx),
            &pv.drive.grove_version,
        )
        .value
        .expect("candidate-local ownership metadata");
    platform
        .perform_events_on_first_block_of_protocol_change(
            &platform.state.load(),
            &block(7),
            &tx,
            13,
            pv,
        )
        .expect("same-transaction flag lookup");
    assert_eq!(
        version_item(&platform, Some(&tx)),
        Element::Item(2u32.to_be_bytes().to_vec(), Some(flags.to_element_flags()))
    );
    assert_eq!(contract_element(&platform, None), original);
}

#[test]
fn should_preserve_already_current_flagged_token_history_payload() {
    let pv = PlatformVersion::latest();
    let platform = TestPlatformBuilder::new()
        .with_latest_protocol_version()
        .build_with_mock_rpc()
        .set_genesis_state();
    let Element::Item(bytes, _) = contract_element(&platform, None) else {
        panic!("contract Item")
    };
    let flags = StorageFlags::new_single_epoch(3, Some([99; 32])).to_element_flags();
    let id = SystemDataContract::TokenHistory.id().to_buffer();
    let tx = platform.drive.grove.start_transaction();
    platform
        .drive
        .grove
        .insert(
            &contract_root_path(&id),
            &[0],
            Element::Item(bytes, Some(flags.clone())),
            None,
            Some(&tx),
            &pv.drive.grove_version,
        )
        .value
        .expect("current payload flags");
    platform
        .drive
        .grove
        .insert(
            &contract_other_path(&id),
            &[CONTRACT_VERSION_KEY],
            Element::Item(2u32.to_be_bytes().to_vec(), Some(flags)),
            None,
            Some(&tx),
            &pv.drive.grove_version,
        )
        .value
        .expect("current version flags");
    platform
        .drive
        .commit_transaction(tx, &pv.drive)
        .expect("current fixture commit");
    let original = contract_element(&platform, None);
    let tx = platform.drive.grove.start_transaction();
    platform
        .perform_events_on_first_block_of_protocol_change(
            &platform.state.load(),
            &block(7),
            &tx,
            13,
            pv,
        )
        .expect("already-current contract apply");
    assert_eq!(contract_element(&platform, Some(&tx)), original);
}

#[test]
fn should_leave_historical_pv13_dispatch_token_history_unchanged() {
    let platform = historical_platform();
    let old = PlatformVersion::get(13).expect("PV13");
    let original = contract_element(&platform, None);
    let root = platform
        .drive
        .grove
        .root_hash(None, &old.drive.grove_version)
        .value
        .expect("PV13 root");
    let tx = platform.drive.grove.start_transaction();
    platform
        .perform_events_on_first_block_of_protocol_change(
            &platform.state.load(),
            &block(7),
            &tx,
            13,
            old,
        )
        .expect("historical replay dispatch");
    assert_eq!(contract_element(&platform, Some(&tx)), original);
    assert_eq!(
        platform
            .drive
            .grove
            .root_hash(Some(&tx), &old.drive.grove_version)
            .value
            .expect("replay root"),
        root
    );
}
