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

use crate::drive::balances::TOTAL_SYSTEM_CREDITS_STORAGE_KEY;
use crate::drive::batch::GroveDbOpBatch;

use crate::drive::protocol_upgrade::add_initial_fork_update_structure_operations;
use crate::drive::{Drive, RootTree};
use crate::error::Error;
use crate::fee_pools::add_create_fee_pool_trees_operations;
use grovedb::{Element, TransactionArg};
use integer_encoding::VarInt;

use super::identity::add_initial_withdrawal_state_structure_operations;

impl Drive {
    /// Creates the initial state structure.
    pub fn create_initial_state_structure(&self, transaction: TransactionArg) -> Result<(), Error> {
        // We can not use batching to insert the root tree structure

        let mut drive_operations = vec![];

        //Row 0 (Full)

        self.grove_insert_empty_tree(
            [],
            &[RootTree::ContractDocuments as u8],
            transaction,
            None,
            &mut drive_operations,
        )?;

        //Row 1 (Full)

        self.grove_insert_empty_tree(
            [],
            &[RootTree::Identities as u8],
            transaction,
            None,
            &mut drive_operations,
        )?;

        self.grove_insert_empty_sum_tree(
            [],
            &[RootTree::Balances as u8],
            transaction,
            None,
            &mut drive_operations,
        )?;

        //Row 2 (Full)

        self.grove_insert_empty_tree(
            [],
            &[RootTree::TokenBalances as u8],
            transaction,
            None,
            &mut drive_operations,
        )?;

        self.grove_insert_empty_sum_tree(
            [],
            &[RootTree::Pools as u8],
            transaction,
            None,
            &mut drive_operations,
        )?;

        self.grove_insert_empty_tree(
            [],
            &[RootTree::WithdrawalTransactions as u8],
            transaction,
            None,
            &mut drive_operations,
        )?;

        self.grove_insert_empty_tree(
            [],
            &[RootTree::Misc as u8],
            transaction,
            None,
            &mut drive_operations,
        )?;

        //Row 3 (3/8 taken)

        self.grove_insert_empty_tree(
            [],
            &[RootTree::NonUniquePublicKeyKeyHashesToIdentities as u8],
            transaction,
            None,
            &mut drive_operations,
        )?;

        self.grove_insert_empty_tree(
            [],
            &[RootTree::UniquePublicKeyHashesToIdentities as u8],
            transaction,
            None,
            &mut drive_operations,
        )?;

        self.grove_insert_empty_tree(
            [],
            &[RootTree::SpentAssetLockTransactions as u8],
            transaction,
            None,
            &mut drive_operations,
        )?;

        self.grove_insert_empty_tree(
            [],
            &[RootTree::Versions as u8],
            transaction,
            None,
            &mut drive_operations,
        )?;

        // On lower layers we can use batching

        let mut batch = GroveDbOpBatch::new();

        // In Misc
        batch.add_insert(
            vec![vec![RootTree::Misc as u8]],
            TOTAL_SYSTEM_CREDITS_STORAGE_KEY.to_vec(),
            Element::Item(0.encode_var_vec(), None),
        );

        // In Pools: initialize the pools with epochs
        add_create_fee_pool_trees_operations(&mut batch)?;

        // In Withdrawals
        add_initial_withdrawal_state_structure_operations(&mut batch);

        // For Versioning via forks
        add_initial_fork_update_structure_operations(&mut batch);

        self.grove_apply_batch(batch, false, transaction)?;

        Ok(())
    }
}

#[cfg(feature = "full")]
#[cfg(test)]
mod tests {
    use crate::drive::{Drive, RootTree};
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
        assert_eq!(elements.len(), 11);
    }

    #[test]
    fn test_initial_state_structure_proper_heights() {
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir, None).expect("should open Drive successfully");

        drive
            .create_initial_state_structure(None)
            .expect("expected to create structure");

        // Merk Level 0
        let mut query = Query::new();
        query.insert_key(vec![RootTree::ContractDocuments as u8]);
        let root_path_query = PathQuery::new(
            vec![],
            SizedQuery {
                query,
                limit: None,
                offset: None,
            },
        );
        let mut drive_operations = vec![];
        let proof = drive
            .grove_get_proved_path_query(&root_path_query, false, None, &mut drive_operations)
            .expect("expected to get root elements");
        assert_eq!(proof.len(), 110); //it + left + right

        // Merk Level 1
        let mut query = Query::new();
        query.insert_key(vec![RootTree::Identities as u8]);
        let root_path_query = PathQuery::new(
            vec![],
            SizedQuery {
                query,
                limit: None,
                offset: None,
            },
        );
        let mut drive_operations = vec![];
        let proof = drive
            .grove_get_proved_path_query(&root_path_query, false, None, &mut drive_operations)
            .expect("expected to get root elements");
        assert_eq!(proof.len(), 179); //it + left + right + parent + parent other

        let mut query = Query::new();
        query.insert_key(vec![RootTree::Balances as u8]);
        let root_path_query = PathQuery::new(
            vec![],
            SizedQuery {
                query,
                limit: None,
                offset: None,
            },
        );
        let mut drive_operations = vec![];
        let proof = drive
            .grove_get_proved_path_query(&root_path_query, false, None, &mut drive_operations)
            .expect("expected to get root elements");
        assert_eq!(proof.len(), 180); //it + left + right + parent + parent other

        // Merk Level 2
        let mut query = Query::new();
        query.insert_key(vec![RootTree::TokenBalances as u8]);
        let root_path_query = PathQuery::new(
            vec![],
            SizedQuery {
                query,
                limit: None,
                offset: None,
            },
        );
        let mut drive_operations = vec![];
        let proof = drive
            .grove_get_proved_path_query(&root_path_query, false, None, &mut drive_operations)
            .expect("expected to get root elements");
        assert_eq!(proof.len(), 247);
        //it + left + right + parent + sibling + parent sibling + grandparent

        let mut query = Query::new();
        query.insert_key(vec![RootTree::Pools as u8]);
        let root_path_query = PathQuery::new(
            vec![],
            SizedQuery {
                query,
                limit: None,
                offset: None,
            },
        );
        let mut drive_operations = vec![];
        let proof = drive
            .grove_get_proved_path_query(&root_path_query, false, None, &mut drive_operations)
            .expect("expected to get root elements");
        assert_eq!(proof.len(), 183); //it + parent + sibling + parent sibling + grandparent

        let mut query = Query::new();
        query.insert_key(vec![RootTree::WithdrawalTransactions as u8]);
        let root_path_query = PathQuery::new(
            vec![],
            SizedQuery {
                query,
                limit: None,
                offset: None,
            },
        );
        let mut drive_operations = vec![];
        let proof = drive
            .grove_get_proved_path_query(&root_path_query, false, None, &mut drive_operations)
            .expect("expected to get root elements");
        assert_eq!(proof.len(), 215); //it + left + parent + sibling + parent sibling + grandparent

        let mut query = Query::new();
        query.insert_key(vec![RootTree::Misc as u8]);
        let root_path_query = PathQuery::new(
            vec![],
            SizedQuery {
                query,
                limit: None,
                offset: None,
            },
        );
        let mut drive_operations = vec![];
        let proof = drive
            .grove_get_proved_path_query(&root_path_query, false, None, &mut drive_operations)
            .expect("expected to get root elements");
        assert_eq!(proof.len(), 215); //it + right + parent + sibling + parent sibling + grandparent

        // Merk Level 3

        let mut query = Query::new();
        query.insert_key(vec![RootTree::UniquePublicKeyHashesToIdentities as u8]);
        let root_path_query = PathQuery::new(
            vec![],
            SizedQuery {
                query,
                limit: None,
                offset: None,
            },
        );
        let mut drive_operations = vec![];
        let proof = drive
            .grove_get_proved_path_query(&root_path_query, false, None, &mut drive_operations)
            .expect("expected to get root elements");
        assert_eq!(proof.len(), 247);

        let mut query = Query::new();
        query.insert_key(vec![
            RootTree::NonUniquePublicKeyKeyHashesToIdentities as u8,
        ]);
        let root_path_query = PathQuery::new(
            vec![],
            SizedQuery {
                query,
                limit: None,
                offset: None,
            },
        );
        let mut drive_operations = vec![];
        let proof = drive
            .grove_get_proved_path_query(&root_path_query, false, None, &mut drive_operations)
            .expect("expected to get root elements");
        assert_eq!(proof.len(), 247);

        let mut query = Query::new();
        query.insert_key(vec![RootTree::SpentAssetLockTransactions as u8]);
        let root_path_query = PathQuery::new(
            vec![],
            SizedQuery {
                query,
                limit: None,
                offset: None,
            },
        );
        let mut drive_operations = vec![];
        let proof = drive
            .grove_get_proved_path_query(&root_path_query, false, None, &mut drive_operations)
            .expect("expected to get root elements");
        assert_eq!(proof.len(), 213);

        let mut query = Query::new();
        query.insert_key(vec![RootTree::Versions as u8]);
        let root_path_query = PathQuery::new(
            vec![],
            SizedQuery {
                query,
                limit: None,
                offset: None,
            },
        );
        let mut drive_operations = vec![];
        let proof = drive
            .grove_get_proved_path_query(&root_path_query, false, None, &mut drive_operations)
            .expect("expected to get root elements");
        assert_eq!(proof.len(), 215);
    }
}
