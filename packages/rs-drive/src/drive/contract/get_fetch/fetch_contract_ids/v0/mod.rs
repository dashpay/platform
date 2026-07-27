use crate::drive::{Drive, RootTree};
use crate::error::Error;
use crate::query::QueryResultType;
use dpp::version::drive_versions::DriveVersion;
use grovedb::{PathQuery, Query, QueryItem, SizedQuery, TransactionArg};
use std::ops::RangeFull;

impl Drive {
    pub(crate) fn fetch_contract_ids_v0(
        &self,
        start_at: Option<([u8; 32], bool)>,
        limit: u16,
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<Vec<[u8; 32]>, Error> {
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

        let path_query = PathQuery::new(
            contracts_root_path,
            SizedQuery::new(query, Some(limit), None),
        );

        let (result_items, _) = self.grove_get_raw_path_query(
            &path_query,
            transaction,
            QueryResultType::QueryKeyElementPairResultType,
            &mut vec![],
            drive_version,
        )?;

        result_items
            .to_keys()
            .into_iter()
            .map(|key| {
                let arr: [u8; 32] = key.try_into().map_err(|v: Vec<u8>| {
                    Error::Drive(crate::error::drive::DriveError::CorruptedDriveState(
                        format!(
                            "Contract ID key has unexpected length {}, expected 32",
                            v.len()
                        ),
                    ))
                })?;
                Ok(arr)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::data_contract::accessors::v0::DataContractV0Setters;
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
            // Give each contract a unique ID
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
    fn fetch_all_contract_ids() {
        let (drive, expected_ids) = setup_contracts(3);
        let platform_version = PlatformVersion::latest();

        let ids = drive
            .fetch_contract_ids(None, 100, None, platform_version)
            .expect("expected to fetch contract ids");

        assert_eq!(ids, expected_ids);
    }

    #[test]
    fn fetch_contract_ids_with_limit() {
        let (drive, expected_ids) = setup_contracts(5);
        let platform_version = PlatformVersion::latest();

        let ids = drive
            .fetch_contract_ids(None, 2, None, platform_version)
            .expect("expected to fetch contract ids");

        assert_eq!(ids.len(), 2);
        assert_eq!(ids, &expected_ids[..2]);
    }

    #[test]
    fn fetch_contract_ids_with_pagination() {
        let (drive, expected_ids) = setup_contracts(5);
        let platform_version = PlatformVersion::latest();

        // First page
        let page1 = drive
            .fetch_contract_ids(None, 2, None, platform_version)
            .expect("expected to fetch page 1");
        assert_eq!(page1.len(), 2);

        // Second page (start after last of page 1)
        let page2 = drive
            .fetch_contract_ids(Some((page1[1], false)), 2, None, platform_version)
            .expect("expected to fetch page 2");
        assert_eq!(page2.len(), 2);

        // Third page
        let page3 = drive
            .fetch_contract_ids(Some((page2[1], false)), 2, None, platform_version)
            .expect("expected to fetch page 3");
        assert_eq!(page3.len(), 1);

        // All pages combined should match the full set
        let mut all: Vec<[u8; 32]> = Vec::new();
        all.extend(&page1);
        all.extend(&page2);
        all.extend(&page3);
        assert_eq!(all, expected_ids);
    }

    #[test]
    fn fetch_contract_ids_start_at_included() {
        let (drive, expected_ids) = setup_contracts(3);
        let platform_version = PlatformVersion::latest();

        let ids = drive
            .fetch_contract_ids(Some((expected_ids[1], true)), 100, None, platform_version)
            .expect("expected to fetch");

        assert_eq!(ids, &expected_ids[1..]);
    }

    #[test]
    fn fetch_contract_ids_start_at_excluded() {
        let (drive, expected_ids) = setup_contracts(3);
        let platform_version = PlatformVersion::latest();

        let ids = drive
            .fetch_contract_ids(Some((expected_ids[1], false)), 100, None, platform_version)
            .expect("expected to fetch");

        assert_eq!(ids, &expected_ids[2..]);
    }

    #[test]
    fn fetch_contract_ids_empty_state() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let ids = drive
            .fetch_contract_ids(None, 100, None, platform_version)
            .expect("expected to fetch");

        assert!(ids.is_empty());
    }
}
