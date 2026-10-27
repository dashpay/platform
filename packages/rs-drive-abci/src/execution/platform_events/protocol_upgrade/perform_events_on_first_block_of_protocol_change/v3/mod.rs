use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use dpp::block::block_info::BlockInfo;
use dpp::version::{PlatformVersion, ProtocolVersion};
use drive::grovedb::Transaction;

impl<C> Platform<C> {
    /// Clears stale contract fees and runs protocol activation events before migrating history.
    pub(super) fn perform_events_on_first_block_of_protocol_change_v3(
        &self,
        platform_state: &PlatformState,
        block_info: &BlockInfo,
        transaction: &Transaction,
        previous_protocol_version: ProtocolVersion,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        self.perform_events_on_first_block_of_protocol_change_v2(
            platform_state,
            block_info,
            transaction,
            previous_protocol_version,
            platform_version,
        )?;
        // Any migration error deliberately halts this activation block for every
        // validator; the shared transaction prevents a partial migration from
        // committing. Its corruption checks are unreachable for valid pre-15
        // state and must never be downgraded to best-effort recovery.
        if previous_protocol_version < 15 && platform_version.protocol_version >= 15 {
            let stats = self
                .drive
                .migrate_document_history_storage(transaction, platform_version)?;
            tracing::info!(?stats, "Migrated document history storage");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::helpers::setup::TestPlatformBuilder;

    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::document_type::accessors::{
        DocumentTypeV0Getters, DocumentTypeV2Getters,
    };
    use dpp::data_contract::document_type::random_document::CreateRandomDocument;
    use dpp::document::{DocumentV0Getters, DocumentV0Setters};
    use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
    use dpp::tests::json_document::json_document_to_contract;
    use drive::drive::{Drive, RootTree};
    use drive::grovedb::query_result_type::{PathKeyElementTrio, QueryResultType};
    use drive::grovedb::{PathQuery, Query, TransactionArg};
    use drive::query::document_history_drive_query::{
        DocumentHistoryDriveQuery, DocumentHistoryFilter,
    };
    use drive::util::object_size_info::{DocumentAndContractInfo, DocumentInfo, OwnedDocumentInfo};

    fn document_type_path(contract_id: &[u8], name: &str) -> Vec<Vec<u8>> {
        vec![
            vec![RootTree::DataContractDocuments as u8],
            contract_id.to_vec(),
            vec![1],
            name.as_bytes().to_vec(),
        ]
    }

    fn subtree_rows(
        drive: &Drive,
        path: Vec<Vec<u8>>,
        transaction: TransactionArg,
        version: &PlatformVersion,
    ) -> Vec<PathKeyElementTrio> {
        let query = PathQuery::new_unsized(path, Query::new_range_full());
        let (rows, _) = drive
            .grove_get_raw_path_query(
                &query,
                transaction,
                QueryResultType::QueryPathKeyElementTrioResultType,
                &mut vec![],
                &version.drive,
            )
            .unwrap();
        let mut snapshot = vec![];
        for (path, key, element) in rows.to_path_key_elements() {
            if element.is_any_tree() {
                let mut child = path.clone();
                child.push(key.clone());
                snapshot.extend(subtree_rows(drive, child, transaction, version));
            }
            snapshot.push((path, key, element));
        }
        snapshot
    }

    #[test]
    fn should_preserve_activation_state_across_grove_versions() {
        let old = PlatformVersion::get(14).unwrap();
        let latest = PlatformVersion::get(15).unwrap();
        let platform = TestPlatformBuilder::new()
            .with_initial_protocol_version(14)
            .build_with_mock_rpc()
            .set_genesis_state();
        let state = platform.state.load();
        let dpns = load_system_data_contract(SystemDataContract::DPNS, old).unwrap();
        let history = json_document_to_contract(concat!(env!("CARGO_MANIFEST_DIR"), "/../rs-drive/tests/supporting_files/contract/dashpay/dashpay-contract-with-profile-history.json"), false, old).unwrap();
        platform
            .drive
            .apply_contract(&history, BlockInfo::default(), true, None, None, old)
            .unwrap();
        let mut history_id = [0; 32];
        for (contract, name, revisions) in [
            (&dpns, "preorder", 1),
            (&dpns, "domain", 1),
            (&history, "contactInfo", 1),
            (&history, "profile", 2),
        ] {
            let document_type = contract.document_type_for_name(name).unwrap();
            let mut document = document_type.random_document(Some(99), old).unwrap();
            document.set_created_at(document.created_at().map(|_| 2000));
            document.set_updated_at(document.updated_at().map(|_| 2000));
            document.set_transferred_at(document.transferred_at().map(|_| 2000));
            for revision in 1..=revisions {
                document.set_revision(Some(revision));
                platform
                    .drive
                    .add_document_for_contract(
                        DocumentAndContractInfo {
                            owned_document_info: OwnedDocumentInfo {
                                document_info: DocumentInfo::DocumentRefInfo((&document, None)),
                                owner_id: None,
                            },
                            contract,
                            document_type,
                        },
                        true,
                        BlockInfo::default_with_time(2000 + revision),
                        true,
                        None,
                        old,
                        None,
                    )
                    .unwrap();
            }
            if name == "profile" {
                history_id = document.id().to_buffer();
            }
        }
        let domain_path = document_type_path(dpns.id().as_bytes(), "domain");
        let ordinary_path = document_type_path(history.id().as_bytes(), "contactInfo");
        let domain_before = subtree_rows(&platform.drive, domain_path.clone(), None, old);
        let ordinary_before = subtree_rows(&platform.drive, ordinary_path.clone(), None, old);
        let fresh = TestPlatformBuilder::new()
            .with_initial_protocol_version(15)
            .build_with_mock_rpc()
            .set_genesis_state();
        let preorder_path = document_type_path(dpns.id().as_bytes(), "preorder");
        let registered = subtree_rows(&fresh.drive, preorder_path.clone(), None, latest);
        let mut v4 = latest.clone();
        v4.drive.grove_version = old.drive.grove_version.clone();
        let mut roots = vec![];
        for (version, commit) in [(&v4, false), (latest, true)] {
            platform.drive.cache.data_contracts.clear_block_cache();
            let transaction = platform.drive.grove.start_transaction();
            platform
                .perform_events_on_first_block_of_protocol_change(
                    &state,
                    &BlockInfo::default_with_time(3000),
                    &transaction,
                    14,
                    version,
                )
                .unwrap();
            assert_eq!(
                subtree_rows(
                    &platform.drive,
                    preorder_path.clone(),
                    Some(&transaction),
                    version
                ),
                registered,
                "rebuilt type has registration layout"
            );
            assert_eq!(
                subtree_rows(
                    &platform.drive,
                    domain_path.clone(),
                    Some(&transaction),
                    version
                ),
                domain_before
            );
            assert_eq!(
                subtree_rows(
                    &platform.drive,
                    ordinary_path.clone(),
                    Some(&transaction),
                    version
                ),
                ordinary_before
            );
            let contract = platform
                .drive
                .get_contract_with_fetch_info(
                    dpns.id().to_buffer(),
                    false,
                    Some(&transaction),
                    version,
                )
                .unwrap()
                .unwrap();
            let preorder = contract
                .contract
                .document_type_for_name("preorder")
                .unwrap();
            assert!(preorder.indexes().contains_key("ownerAndNameHash"));
            assert_eq!(preorder.documents_ttl_seconds(), Some(3600));
            let query = DocumentHistoryDriveQuery {
                contract_id: history.id().to_buffer(),
                document_type_name: "profile".into(),
                document_id: history_id,
                filter: DocumentHistoryFilter::StartAtTime(0),
                limit: None,
            };
            // V4 cannot serve the new page query; state migration itself must
            // still produce exactly the same bytes as the selected engine.
            let page = query
                .execute_no_proof(
                    &platform.drive,
                    history.document_type_for_name("profile").unwrap(),
                    Some(&transaction),
                    latest,
                )
                .unwrap();
            assert_eq!(
                page.entries
                    .iter()
                    .map(|entry| entry.revision)
                    .collect::<Vec<_>>(),
                vec![1, 2]
            );
            let root = platform
                .drive
                .grove
                .root_hash(Some(&transaction), &version.drive.grove_version)
                .unwrap()
                .unwrap();
            assert_eq!(
                hex::encode(root),
                "9e2aef15983ee44cceaed85ad737f27b3f32fe3f2fc3bb0743cb5230eada61d6",
                "activation must preserve its historical state commitment",
            );
            roots.push(root);
            if commit {
                platform
                    .drive
                    .grove
                    .commit_transaction(transaction)
                    .unwrap()
                    .unwrap();
            }
        }
        assert_eq!(roots[0], roots[1]);
        assert_eq!(
            platform
                .drive
                .grove
                .root_hash(None, &latest.drive.grove_version)
                .unwrap()
                .unwrap(),
            roots[1]
        );
    }

    #[test]
    fn should_not_migrate_history_before_protocol_15() {
        let platform_version = PlatformVersion::get(14).expect("protocol version 14 should exist");
        let platform = TestPlatformBuilder::new()
            .with_initial_protocol_version(14)
            .build_with_mock_rpc()
            .set_genesis_state();
        let transaction = platform.drive.grove.start_transaction();
        let platform_state = platform.state.load();

        platform
            .perform_events_on_first_block_of_protocol_change_v3(
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                14,
                platform_version,
            )
            .expect("a protocol 14 transition must not run the protocol 15 migration");
    }
}
