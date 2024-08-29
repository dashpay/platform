use crate::drive::votes::paths::{
    vote_contested_resource_tree_path_vec, vote_root_path_vec, ACTIVE_POLLS_TREE_KEY,
    CONTESTED_RESOURCE_TREE_KEY, END_DATE_QUERIES_TREE_KEY, IDENTITY_VOTES_TREE_KEY,
    VOTE_DECISIONS_TREE_KEY,
};
use crate::drive::Drive;
use crate::error::Error;
use crate::util::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
use crate::util::batch::GroveDbOpBatch;

impl Drive {
    pub(super) fn add_initial_vote_tree_main_structure_operations_v0(
        batch: &mut GroveDbOpBatch,
    ) -> Result<(), Error> {
        batch.add_insert_empty_tree(vote_root_path_vec(), vec![VOTE_DECISIONS_TREE_KEY as u8]);

        batch.add_insert_empty_tree(
            vote_root_path_vec(),
            vec![CONTESTED_RESOURCE_TREE_KEY as u8],
        );

        batch.add_insert_empty_tree(vote_root_path_vec(), vec![END_DATE_QUERIES_TREE_KEY as u8]);

        batch.add_insert_empty_tree(
            vote_contested_resource_tree_path_vec(),
            vec![ACTIVE_POLLS_TREE_KEY as u8],
        );

        batch.add_insert_empty_tree(
            vote_contested_resource_tree_path_vec(),
            vec![IDENTITY_VOTES_TREE_KEY as u8],
        );

        Ok(())
    }
}
