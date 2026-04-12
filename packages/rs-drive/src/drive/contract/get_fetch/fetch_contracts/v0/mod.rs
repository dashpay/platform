use crate::drive::contract::paths;
use crate::drive::{Drive, RootTree};
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::data_contract::DataContract;
use dpp::serialization::PlatformDeserializableWithPotentialValidationFromVersionedStructure;
use dpp::version::PlatformVersion;
use grovedb::{Element, PathQuery, Query, QueryItem, SizedQuery, TransactionArg};
use std::ops::RangeFull;

impl Drive {
    pub(super) fn fetch_contracts_v0(
        &self,
        start_at: Option<([u8; 32], bool)>,
        limit: u16,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<DataContract>, Error> {
        let contracts_root_path =
            vec![Into::<&[u8; 1]>::into(RootTree::DataContractDocuments).to_vec()];

        let mut query = Query::new();
        if let Some((start_at_id, start_at_included)) = start_at {
            if start_at_included {
                query.insert_item(QueryItem::RangeFrom(start_at_id.to_vec()..));
            } else {
                query.insert_item(QueryItem::RangeAfter(start_at_id.to_vec()..));
            }
        } else {
            query.insert_item(QueryItem::RangeFull(RangeFull));
        }
        // Descend one level into each contract subtree to fetch key [0].
        // For non-historical contracts this is the Item with the contract bytes.
        // For historical contracts this is a Tree (the history subtree), which
        // we handle with a follow-up read below.
        query.set_subquery_key(vec![0]);

        let path_query = PathQuery::new(
            contracts_root_path,
            SizedQuery::new(query, Some(limit), None),
        );

        let (result_items, _) = self.grove_get_raw_path_query(
            &path_query,
            transaction,
            crate::query::QueryResultType::QueryPathKeyElementTrioResultType,
            &mut vec![],
            &platform_version.drive,
        )?;

        let mut contracts = Vec::new();

        for (path, _key, element) in result_items.to_path_key_elements() {
            let stored_bytes = match element {
                Element::Item(bytes, _) => {
                    // Non-historical contract — bytes are the serialized contract.
                    bytes
                }
                Element::Tree(..) => {
                    // Historical contract — the element at key [0] is the history
                    // subtree. Read via get_caching_optional which follows the
                    // reference at [root, id, 0, 0] to the latest revision.
                    let contract_id_bytes =
                        path.last()
                            .ok_or(Error::Drive(DriveError::CorruptedDriveState(
                                "Empty path in contract query result".to_string(),
                            )))?;
                    let history_path =
                        paths::contract_keeping_history_root_path(contract_id_bytes.as_slice());
                    let history_element = self
                        .grove
                        .get_caching_optional(
                            (&history_path).into(),
                            &[0],
                            false,
                            transaction,
                            &platform_version.drive.grove_version,
                        )
                        .value
                        .map_err(|e| Error::GroveDB(Box::new(e)))?;
                    match history_element {
                        Element::Item(bytes, _) => bytes,
                        _ => {
                            return Err(Error::Drive(DriveError::CorruptedDriveState(format!(
                                "Could not resolve historical contract element for {}",
                                hex::encode(contract_id_bytes)
                            ))));
                        }
                    }
                }
                _ => {
                    return Err(Error::Drive(DriveError::CorruptedDriveState(
                        "Unexpected element type in contract query result".to_string(),
                    )));
                }
            };

            let contract =
                DataContract::versioned_deserialize(&stored_bytes, false, platform_version)
                    .map_err(Error::from)?;

            contracts.push(contract);
        }

        Ok(contracts)
    }
}

#[cfg(test)]
mod tests {
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
    use dpp::data_contract::config::v0::DataContractConfigSettersV0;
    use dpp::tests::json_document::json_document_to_contract;
    use dpp::version::PlatformVersion;
    use grovedb_epoch_based_storage_flags::StorageFlags;

    fn setup_contracts(count: usize) -> (crate::drive::Drive, Vec<[u8; 32]>) {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let contract_path = "tests/supporting_files/contract/family/family-contract.json";

        let mut ids = Vec::new();
        for i in 0..count {
            let mut contract = json_document_to_contract(contract_path, false, platform_version)
                .expect("expected to get contract");
            let mut id = [0u8; 32];
            id[0] = (i + 1) as u8;
            contract.set_id(id.into());
            drive
                .apply_contract(
                    &contract,
                    BlockInfo::default(),
                    true,
                    StorageFlags::optional_default_as_cow(),
                    None,
                    platform_version,
                )
                .expect("expected to apply contract");
            ids.push(id);
        }
        ids.sort();
        (drive, ids)
    }

    #[test]
    fn fetch_all_contracts() {
        let (drive, expected_ids) = setup_contracts(3);
        let platform_version = PlatformVersion::latest();

        let contracts = drive
            .fetch_contracts(None, 100, None, platform_version)
            .expect("expected to fetch contracts");

        assert_eq!(contracts.len(), 3);
        let fetched_ids: Vec<[u8; 32]> = contracts.iter().map(|c| c.id().to_buffer()).collect();
        assert_eq!(fetched_ids, expected_ids);
    }

    #[test]
    fn fetch_contracts_with_limit() {
        let (drive, expected_ids) = setup_contracts(5);
        let platform_version = PlatformVersion::latest();

        let contracts = drive
            .fetch_contracts(None, 2, None, platform_version)
            .expect("expected to fetch contracts");

        assert_eq!(contracts.len(), 2);
        let fetched_ids: Vec<[u8; 32]> = contracts.iter().map(|c| c.id().to_buffer()).collect();
        assert_eq!(fetched_ids, &expected_ids[..2]);
    }

    #[test]
    fn fetch_contracts_with_pagination() {
        let (drive, expected_ids) = setup_contracts(4);
        let platform_version = PlatformVersion::latest();

        // First page
        let page1 = drive
            .fetch_contracts(None, 2, None, platform_version)
            .expect("expected to fetch page 1");
        assert_eq!(page1.len(), 2);

        // Second page (start after last of page 1)
        let last_id = page1.last().unwrap().id().to_buffer();
        let page2 = drive
            .fetch_contracts(Some((last_id, false)), 2, None, platform_version)
            .expect("expected to fetch page 2");
        assert_eq!(page2.len(), 2);

        // All pages combined
        let mut all_ids: Vec<[u8; 32]> = Vec::new();
        all_ids.extend(page1.iter().map(|c| c.id().to_buffer()));
        all_ids.extend(page2.iter().map(|c| c.id().to_buffer()));
        assert_eq!(all_ids, expected_ids);
    }

    #[test]
    fn fetch_contracts_returns_valid_data() {
        let (drive, _) = setup_contracts(1);
        let platform_version = PlatformVersion::latest();

        let contracts = drive
            .fetch_contracts(None, 100, None, platform_version)
            .expect("expected to fetch contracts");

        assert_eq!(contracts.len(), 1);
        let contract = &contracts[0];
        // The family contract should have document types
        assert!(!contract.document_types().is_empty());
    }

    #[test]
    fn fetch_contracts_empty_state() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let contracts = drive
            .fetch_contracts(None, 100, None, platform_version)
            .expect("expected to fetch");

        assert!(contracts.is_empty());
    }

    #[test]
    fn fetch_contracts_includes_historical_contract() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        // Insert a normal (non-historical) contract
        let contract_path = "tests/supporting_files/contract/family/family-contract.json";
        let mut normal_contract = json_document_to_contract(contract_path, false, platform_version)
            .expect("expected to get contract");
        let normal_id = [1u8; 32];
        normal_contract.set_id(normal_id.into());
        drive
            .apply_contract(
                &normal_contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("expected to apply normal contract");

        // Insert a historical contract (keeps_history = true)
        let history_path =
            "tests/supporting_files/contract/references/references_with_contract_history.json";
        let mut history_contract = json_document_to_contract(history_path, false, platform_version)
            .expect("expected to get contract");
        let history_id = [2u8; 32];
        history_contract.set_id(history_id.into());
        history_contract.config_mut().set_keeps_history(true);
        history_contract.config_mut().set_readonly(false);
        drive
            .apply_contract(
                &history_contract,
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
            .expect("expected to apply historical contract");

        // Fetch all contracts — should get both
        let contracts = drive
            .fetch_contracts(None, 100, None, platform_version)
            .expect("expected to fetch contracts");

        assert_eq!(contracts.len(), 2);
        let fetched_ids: Vec<[u8; 32]> = contracts.iter().map(|c| c.id().to_buffer()).collect();
        assert!(fetched_ids.contains(&normal_id));
        assert!(fetched_ids.contains(&history_id));

        // Verify the historical contract has the right document types
        let hist = contracts
            .iter()
            .find(|c| c.id().to_buffer() == history_id)
            .unwrap();
        assert!(!hist.document_types().is_empty());
    }
}
