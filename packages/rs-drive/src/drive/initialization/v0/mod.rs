//! Drive Initialization

use grovedb_path::SubtreePath;

use crate::drive::balances::TOTAL_SYSTEM_CREDITS_STORAGE_KEY;
use crate::util::batch::GroveDbOpBatch;

use crate::drive::system::misc_path_vec;
use crate::drive::{Drive, RootTree};
use crate::error::Error;
use crate::util::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;

use dpp::version::PlatformVersion;
use grovedb::{Element, TransactionArg};
use integer_encoding::VarInt;

impl Drive {
    /// Creates the initial state structure.
    pub(super) fn create_initial_state_structure_0(
        &self,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let drive_version = &platform_version.drive;
        self.create_initial_state_structure_top_level_0(transaction, platform_version)?;

        // On lower layers we can use batching

        let batch =
            self.create_initial_state_structure_lower_layers_operations_0(platform_version)?;

        self.grove_apply_batch(batch, false, transaction, drive_version)?;

        Ok(())
    }

    /// Creates the initial state structure.
    pub(in crate::drive::initialization) fn create_initial_state_structure_top_level_0(
        &self,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let drive_version = &platform_version.drive;
        // We can not use batching to insert the root tree structure

        let mut drive_operations = vec![];

        //Row 0 (Full)

        self.grove_insert_empty_tree(
            SubtreePath::empty(),
            &[RootTree::DataContractDocuments as u8],
            transaction,
            None,
            &mut drive_operations,
            drive_version,
        )?;

        //Row 1 (Full)

        self.grove_insert_empty_tree(
            SubtreePath::empty(),
            &[RootTree::Identities as u8],
            transaction,
            None,
            &mut drive_operations,
            drive_version,
        )?;

        self.grove_insert_empty_sum_tree(
            SubtreePath::empty(),
            &[RootTree::Balances as u8],
            transaction,
            None,
            &mut drive_operations,
            drive_version,
        )?;

        //Row 2 (Full)

        self.grove_insert_empty_tree(
            SubtreePath::empty(),
            &[RootTree::Tokens as u8],
            transaction,
            None,
            &mut drive_operations,
            drive_version,
        )?;

        self.grove_insert_empty_sum_tree(
            SubtreePath::empty(),
            &[RootTree::Pools as u8],
            transaction,
            None,
            &mut drive_operations,
            drive_version,
        )?;

        self.grove_insert_empty_tree(
            SubtreePath::empty(),
            &[RootTree::WithdrawalTransactions as u8],
            transaction,
            None,
            &mut drive_operations,
            drive_version,
        )?;

        self.grove_insert_empty_tree(
            SubtreePath::empty(),
            &[RootTree::Votes as u8],
            transaction,
            None,
            &mut drive_operations,
            drive_version,
        )?;

        //Row 3 (6/8 taken)

        self.grove_insert_empty_tree(
            SubtreePath::empty(),
            &[RootTree::NonUniquePublicKeyKeyHashesToIdentities as u8],
            transaction,
            None,
            &mut drive_operations,
            drive_version,
        )?;

        self.grove_insert_empty_tree(
            SubtreePath::empty(),
            &[RootTree::UniquePublicKeyHashesToIdentities as u8],
            transaction,
            None,
            &mut drive_operations,
            drive_version,
        )?;

        self.grove_insert_empty_sum_tree(
            SubtreePath::empty(),
            &[RootTree::PreFundedSpecializedBalances as u8],
            transaction,
            None,
            &mut drive_operations,
            drive_version,
        )?;

        self.grove_insert_empty_tree(
            SubtreePath::empty(),
            &[RootTree::SpentAssetLockTransactions as u8],
            transaction,
            None,
            &mut drive_operations,
            drive_version,
        )?;

        self.grove_insert_empty_tree(
            SubtreePath::empty(),
            &[RootTree::Misc as u8],
            transaction,
            None,
            &mut drive_operations,
            drive_version,
        )?;

        self.grove_insert_empty_tree(
            SubtreePath::empty(),
            &[RootTree::Versions as u8],
            transaction,
            None,
            &mut drive_operations,
            drive_version,
        )?;

        Ok(())
    }

    /// Creates the initial state structure.
    pub(in crate::drive::initialization) fn create_initial_state_structure_lower_layers_operations_0(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<GroveDbOpBatch, Error> {
        // On lower layers we can use batching

        let mut batch = GroveDbOpBatch::new();

        // In Misc
        batch.add_insert(
            misc_path_vec(),
            TOTAL_SYSTEM_CREDITS_STORAGE_KEY.to_vec(),
            Element::Item(0u64.encode_var_vec(), None),
        );

        // In Pools: initialize the pools with epochs
        Drive::add_create_fee_pool_trees_operations(
            &mut batch,
            self.config.epochs_per_era,
            platform_version.protocol_version,
        )?;

        // In Withdrawals
        Drive::add_initial_withdrawal_state_structure_operations(&mut batch, platform_version);

        // For Versioning via forks
        Drive::add_initial_fork_update_structure_operations(&mut batch);

        // Pre funded specialized balances tree
        Drive::add_initial_prefunded_specialized_balances_operations(&mut batch);

        // For the votes tree structure
        Drive::add_initial_vote_tree_main_structure_operations(&mut batch, platform_version)?;

        Ok(batch)
    }
}

#[cfg(feature = "server")]
#[cfg(test)]
mod tests {
    use crate::drive::RootTree;

    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::version::PlatformVersion;
    use grovedb::query_result_type::QueryResultType::QueryElementResultType;
    use grovedb::{PathQuery, Query, SizedQuery};

    #[test]
    fn test_create_initial_state_structure_in_first_protocol_version() {
        let platform_version = PlatformVersion::first();
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));

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
                &platform_version.drive,
            )
            .expect("expected to get root elements");
        assert_eq!(elements.len(), 13);
    }

    #[test]
    fn test_create_initial_state_structure_in_latest_protocol_version() {
        let drive = setup_drive_with_initial_state_structure(None);

        let platform_version = PlatformVersion::latest();

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
                &platform_version.drive,
            )
            .expect("expected to get root elements");
        assert_eq!(elements.len(), 14);
    }

    #[test]
    fn test_initial_state_structure_proper_heights_in_first_protocol_version() {
        let platform_version = PlatformVersion::first();
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));

        let platform_version = PlatformVersion::first();
        let drive_version = &platform_version.drive;

        // Merk Level 0
        let mut query = Query::new();
        query.insert_key(vec![RootTree::DataContractDocuments as u8]);
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
            .grove_get_proved_path_query(
                &root_path_query,
                None,
                &mut drive_operations,
                drive_version,
            )
            .expect("expected to get root elements");
        assert_eq!(proof.len(), 112); //it + left + right

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
            .grove_get_proved_path_query(
                &root_path_query,
                None,
                &mut drive_operations,
                drive_version,
            )
            .expect("expected to get root elements");
        assert_eq!(proof.len(), 180); //it + left + right + parent + parent other

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
            .grove_get_proved_path_query(
                &root_path_query,
                None,
                &mut drive_operations,
                drive_version,
            )
            .expect("expected to get root elements");
        assert_eq!(proof.len(), 181); //it + left + right + parent + parent other

        // Merk Level 2
        let mut query = Query::new();
        query.insert_key(vec![RootTree::Tokens as u8]);
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
            .grove_get_proved_path_query(
                &root_path_query,
                None,
                &mut drive_operations,
                drive_version,
            )
            .expect("expected to get root elements");
        assert_eq!(proof.len(), 248); //it + left + right + parent + sibling + parent sibling + grandparent

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
            .grove_get_proved_path_query(
                &root_path_query,
                None,
                &mut drive_operations,
                drive_version,
            )
            .expect("expected to get root elements");
        assert_eq!(proof.len(), 218); //it + left + parent + sibling + parent sibling + grandparent

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
            .grove_get_proved_path_query(
                &root_path_query,
                None,
                &mut drive_operations,
                drive_version,
            )
            .expect("expected to get root elements");
        assert_eq!(proof.len(), 216); //it + left + parent + sibling + parent sibling + grandparent

        let mut query = Query::new();
        query.insert_key(vec![RootTree::Votes as u8]);
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
            .grove_get_proved_path_query(
                &root_path_query,
                None,
                &mut drive_operations,
                drive_version,
            )
            .expect("expected to get root elements");
        assert_eq!(proof.len(), 250); //it + left + right + parent + sibling + parent sibling + grandparent

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
            .grove_get_proved_path_query(
                &root_path_query,
                None,
                &mut drive_operations,
                drive_version,
            )
            .expect("expected to get root elements");
        assert_eq!(proof.len(), 248); //it + parent + sibling + parent sibling + grandparent + grandparent sibling + great-grandparent

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
            .grove_get_proved_path_query(
                &root_path_query,
                None,
                &mut drive_operations,
                drive_version,
            )
            .expect("expected to get root elements");
        assert_eq!(proof.len(), 248); //it + parent + sibling + parent sibling + grandparent + grandparent sibling + great-grandparent

        let mut query = Query::new();
        query.insert_key(vec![RootTree::PreFundedSpecializedBalances as u8]);
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
            .grove_get_proved_path_query(
                &root_path_query,
                None,
                &mut drive_operations,
                drive_version,
            )
            .expect("expected to get root elements");
        assert_eq!(proof.len(), 217); //it + parent + parent sibling + grandparent + grandparent sibling + great-grandparent

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
            .grove_get_proved_path_query(
                &root_path_query,
                None,
                &mut drive_operations,
                drive_version,
            )
            .expect("expected to get root elements");
        assert_eq!(proof.len(), 214); //it + parent + parent sibling + grandparent + grandparent sibling + great-grandparent

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
            .grove_get_proved_path_query(
                &root_path_query,
                None,
                &mut drive_operations,
                drive_version,
            )
            .expect("expected to get root elements");
        assert_eq!(proof.len(), 250); //it + parent + sibling + parent sibling + grandparent + grandparent sibling + great-grandparent

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
            .grove_get_proved_path_query(
                &root_path_query,
                None,
                &mut drive_operations,
                drive_version,
            )
            .expect("expected to get root elements");
        assert_eq!(proof.len(), 250); //it + parent + sibling + parent sibling + grandparent + grandparent sibling + great-grandparent
    }

    #[test]
    fn test_initial_state_structure_proper_heights_in_latest_protocol_version() {
        let drive = setup_drive_with_initial_state_structure(None);

        let platform_version = PlatformVersion::latest();
        let drive_version = &platform_version.drive;

        // Merk Level 0
        let mut query = Query::new();
        query.insert_key(vec![RootTree::DataContractDocuments as u8]);
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
            .grove_get_proved_path_query(
                &root_path_query,
                None,
                &mut drive_operations,
                drive_version,
            )
            .expect("expected to get root elements");
        assert_eq!(proof.len(), 112); //it + left + right

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
            .grove_get_proved_path_query(
                &root_path_query,
                None,
                &mut drive_operations,
                drive_version,
            )
            .expect("expected to get root elements");
        assert_eq!(proof.len(), 180); //it + left + right + parent + parent other

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
            .grove_get_proved_path_query(
                &root_path_query,
                None,
                &mut drive_operations,
                drive_version,
            )
            .expect("expected to get root elements");
        assert_eq!(proof.len(), 181); //it + left + right + parent + parent other

        // Merk Level 2
        let mut query = Query::new();
        query.insert_key(vec![RootTree::Tokens as u8]);
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
            .grove_get_proved_path_query(
                &root_path_query,
                None,
                &mut drive_operations,
                drive_version,
            )
            .expect("expected to get root elements");
        assert_eq!(proof.len(), 250); //it + left + right + parent + sibling + parent sibling + grandparent

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
            .grove_get_proved_path_query(
                &root_path_query,
                None,
                &mut drive_operations,
                drive_version,
            )
            .expect("expected to get root elements");
        assert_eq!(proof.len(), 218); //it + left + parent + sibling + parent sibling + grandparent

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
            .grove_get_proved_path_query(
                &root_path_query,
                None,
                &mut drive_operations,
                drive_version,
            )
            .expect("expected to get root elements");
        assert_eq!(proof.len(), 250); //it + left + right + parent + sibling + parent sibling + grandparent

        let mut query = Query::new();
        query.insert_key(vec![RootTree::Votes as u8]);
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
            .grove_get_proved_path_query(
                &root_path_query,
                None,
                &mut drive_operations,
                drive_version,
            )
            .expect("expected to get root elements");
        assert_eq!(proof.len(), 250); //it + left + right + parent + sibling + parent sibling + grandparent

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
            .grove_get_proved_path_query(
                &root_path_query,
                None,
                &mut drive_operations,
                drive_version,
            )
            .expect("expected to get root elements");
        assert_eq!(proof.len(), 248); //it + parent + sibling + parent sibling + grandparent + grandparent sibling + great-grandparent

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
            .grove_get_proved_path_query(
                &root_path_query,
                None,
                &mut drive_operations,
                drive_version,
            )
            .expect("expected to get root elements");
        assert_eq!(proof.len(), 248); //it + parent + sibling + parent sibling + grandparent + grandparent sibling + great-grandparent

        let mut query = Query::new();
        query.insert_key(vec![RootTree::PreFundedSpecializedBalances as u8]);
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
            .grove_get_proved_path_query(
                &root_path_query,
                None,
                &mut drive_operations,
                drive_version,
            )
            .expect("expected to get root elements");
        assert_eq!(proof.len(), 217); //it + parent + parent sibling + grandparent + grandparent sibling + great-grandparent

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
            .grove_get_proved_path_query(
                &root_path_query,
                None,
                &mut drive_operations,
                drive_version,
            )
            .expect("expected to get root elements");
        assert_eq!(proof.len(), 248); //it + parent + sibling + parent sibling + grandparent + grandparent sibling + great-grandparent

        let mut query = Query::new();
        query.insert_key(vec![RootTree::GroupActions as u8]);
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
            .grove_get_proved_path_query(
                &root_path_query,
                None,
                &mut drive_operations,
                drive_version,
            )
            .expect("expected to get root elements");
        assert_eq!(proof.len(), 248); //it + parent + sibling + parent sibling + grandparent + grandparent sibling + great-grandparent

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
            .grove_get_proved_path_query(
                &root_path_query,
                None,
                &mut drive_operations,
                drive_version,
            )
            .expect("expected to get root elements");
        assert_eq!(proof.len(), 250); //it + parent + sibling + parent sibling + grandparent + grandparent sibling + great-grandparent

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
            .grove_get_proved_path_query(
                &root_path_query,
                None,
                &mut drive_operations,
                drive_version,
            )
            .expect("expected to get root elements");
        assert_eq!(proof.len(), 250); //it + parent + sibling + parent sibling + grandparent + grandparent sibling + great-grandparent
    }
}
