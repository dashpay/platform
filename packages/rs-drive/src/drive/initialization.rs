use crate::drive::batch::GroveDbOpBatch;
use crate::drive::contract::add_init_contracts_structure_operations;
use crate::drive::{Drive, RootTree};
use crate::error::Error;
use crate::fee_pools::add_create_fee_pool_trees_operations;
use grovedb::TransactionArg;

impl Drive {
    pub fn create_initial_state_structure(&self, transaction: TransactionArg) -> Result<(), Error> {
        let mut batch = GroveDbOpBatch::new();

        batch.add_insert_empty_tree(vec![], vec![RootTree::Identities as u8]);

        add_init_contracts_structure_operations(&mut batch);

        batch.add_insert_empty_tree(vec![], vec![RootTree::PublicKeyHashesToIdentities as u8]);

        batch.add_insert_empty_tree(vec![], vec![RootTree::SpentAssetLockTransactions as u8]);

        batch.add_insert_empty_tree(vec![], vec![RootTree::Pools as u8]);

        // initialize the pools with epochs
        add_create_fee_pool_trees_operations(&mut batch);

        self.grove_apply_batch(batch, false, transaction)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::drive::Drive;
    use grovedb::query_result_type::QueryResultType::QueryElementResultType;
    use grovedb::{PathQuery, Query, SizedQuery};
    use tempfile::TempDir;

    #[test]
    fn test_create_initial_state_structure() {
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir, None).expect("should open Drive successfully");

        drive
            .create_initial_state_structure(None)
            .expect("expected to create structure");
        let mut query = Query::new();
        query.insert_all();
        let root_path_query = PathQuery::new(
            vec![],
            SizedQuery {
                query,
                limit: None,
                offset: None,
            },
        );
        let mut drive_operations = vec![];
        let (elements, _) = drive
            .grove_get_raw_path_query(
                &root_path_query,
                None,
                QueryElementResultType,
                &mut drive_operations,
            )
            .expect("expected to get root elements");
        assert_eq!(elements.len(), 5);
    }
}
