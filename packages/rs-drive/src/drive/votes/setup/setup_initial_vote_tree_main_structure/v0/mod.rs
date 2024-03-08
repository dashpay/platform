use crate::error::Error;
use crate::drive::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
use crate::drive::batch::GroveDbOpBatch;
use crate::drive::Drive;
use crate::drive::votes::{CONTESTED_RESOURCE_TREE_KEY, END_DATE_QUERIES_TREE_KEY, IDENTITY_VOTES_TREE_KEY, vote_contested_resource_tree_path_vec, VOTE_DECISIONS_TREE_KEY, vote_root_path_vec};

impl Drive {
    pub(super) fn add_initial_vote_tree_main_structure_operations_v0(
        batch: &mut GroveDbOpBatch,
    ) -> Result<(), Error> {

        batch.add_insert_empty_tree(vote_root_path_vec(), vec![VOTE_DECISIONS_TREE_KEY as u8]);

        batch.add_insert_empty_tree(vote_root_path_vec(), vec![CONTESTED_RESOURCE_TREE_KEY as u8]);

        batch.add_insert_empty_tree(vote_contested_resource_tree_path_vec(), vec![END_DATE_QUERIES_TREE_KEY as u8]);

        batch.add_insert_empty_tree(vote_contested_resource_tree_path_vec(), vec![IDENTITY_VOTES_TREE_KEY as u8]);

        Ok(())
    }
}
