use crate::drive::prefunded_specialized_balances::{
    prefunded_specialized_balances_path, PREFUNDED_BALANCES_FOR_READINESS,
};
use crate::drive::votes::paths::{
    readiness_tree_path_vec, vote_root_path_vec, READINESS_CONTRACTS_TREE_KEY,
    READINESS_DEADLINES_TREE_KEY, READINESS_RETIRED_ROUNDS_TREE_KEY, READINESS_TREE_KEY,
};
use crate::drive::Drive;
use crate::error::Error;
use crate::util::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
use crate::util::batch::GroveDbOpBatch;

impl Drive {
    /// Generation 1 adds the compilation readiness structures on top of generation 0: the
    /// `[Votes] / r` tree with its contracts, deadlines and retired-rounds children, and the
    /// readiness fund sum tree `[PreFundedSpecializedBalances] / 129`.
    ///
    /// The fund tree is created here and not by the prefunded balances helper because that
    /// helper is unversioned and genesis builds the lower layers in one batch; this dispatcher
    /// has its own version slot, so the shape of genesis at protocol version 14 is unchanged.
    /// The upgrade path (`Platform::transition_to_version_17_compilation_readiness`) creates
    /// the same elements with insert-if-not-exists through `add_readiness_structure_operations`.
    pub(super) fn add_initial_vote_tree_main_structure_operations_v1(
        batch: &mut GroveDbOpBatch,
    ) -> Result<(), Error> {
        Self::add_initial_vote_tree_main_structure_operations_v0(batch)?;
        Self::add_readiness_structure_operations(batch);
        Ok(())
    }

    /// The operations that create the compilation readiness structures. Shared by genesis
    /// (generation 1 of the vote setup) and the upgrade path so both produce byte-identical
    /// elements; the caller decides whether the inserts are plain or if-not-exists.
    pub fn add_readiness_structure_operations(batch: &mut GroveDbOpBatch) {
        batch.add_insert_empty_tree(vote_root_path_vec(), vec![READINESS_TREE_KEY as u8]);
        batch.add_insert_empty_tree(
            readiness_tree_path_vec(),
            vec![READINESS_CONTRACTS_TREE_KEY],
        );
        batch.add_insert_empty_tree(
            readiness_tree_path_vec(),
            vec![READINESS_DEADLINES_TREE_KEY],
        );
        batch.add_insert_empty_tree(
            readiness_tree_path_vec(),
            vec![READINESS_RETIRED_ROUNDS_TREE_KEY],
        );
        batch.add_insert_empty_sum_tree(
            prefunded_specialized_balances_path()
                .iter()
                .map(|segment| segment.to_vec())
                .collect(),
            vec![PREFUNDED_BALANCES_FOR_READINESS],
        );
    }
}
