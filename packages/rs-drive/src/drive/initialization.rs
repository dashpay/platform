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

//! Drive Initialization
//!

use crate::drive::batch::GroveDbOpBatch;
use crate::drive::contract::add_init_contracts_structure_operations;
use crate::drive::{Drive, RootTree};
use crate::error::Error;
use crate::fee_pools::add_create_fee_pool_trees_operations;
use grovedb::TransactionArg;

impl Drive {
    /// Creates the initial state structure.
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
