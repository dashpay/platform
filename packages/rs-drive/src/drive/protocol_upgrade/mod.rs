use crate::drive::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
use crate::drive::batch::GroveDbOpBatch;
use crate::drive::RootTree;

mod change_to_new_version_and_clear_version_information;
mod clear_version_information;
mod fetch_versions_with_counter;
mod remove_validators_proposed_app_versions;
mod update_validator_proposed_app_version;

/// constant id for various versions counter
pub const VERSIONS_COUNTER: [u8; 1] = [0];
/// constant id for subtree containing the desired versions for each validator
pub const VALIDATOR_DESIRED_VERSIONS: [u8; 1] = [1];

/// Add operations for creating initial versioning state structure
pub fn add_initial_fork_update_structure_operations(batch: &mut GroveDbOpBatch) {
    batch.add_insert_empty_tree(
        vec![vec![RootTree::Versions as u8]],
        VERSIONS_COUNTER.to_vec(),
    );

    batch.add_insert_empty_tree(
        vec![vec![RootTree::Versions as u8]],
        VALIDATOR_DESIRED_VERSIONS.to_vec(),
    );
}

pub(crate) fn versions_counter_path() -> [&'static [u8]; 2] {
    [
        Into::<&[u8; 1]>::into(RootTree::Versions),
        VERSIONS_COUNTER.as_slice(),
    ]
}

fn versions_counter_path_vec() -> Vec<Vec<u8>> {
    vec![vec![RootTree::Versions as u8], VERSIONS_COUNTER.to_vec()]
}

pub(crate) fn desired_version_for_validators_path() -> [&'static [u8]; 2] {
    [
        Into::<&[u8; 1]>::into(RootTree::Versions),
        VALIDATOR_DESIRED_VERSIONS.as_slice(),
    ]
}

fn desired_version_for_validators_path_vec() -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::Versions as u8],
        VALIDATOR_DESIRED_VERSIONS.to_vec(),
    ]
}
