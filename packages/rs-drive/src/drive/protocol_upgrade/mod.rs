#[cfg(feature = "full")]
use crate::drive::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
#[cfg(feature = "full")]
use crate::drive::batch::GroveDbOpBatch;
#[cfg(any(feature = "full", feature = "verify"))]
use crate::drive::RootTree;

#[cfg(feature = "full")]
mod clear_version_information;
#[cfg(feature = "full")]
mod fetch_proved_validator_version_votes;
#[cfg(feature = "full")]
mod fetch_proved_versions_with_counter;
#[cfg(feature = "full")]
mod fetch_validator_version_votes;
#[cfg(feature = "full")]
mod fetch_versions_with_counter;
#[cfg(feature = "full")]
mod remove_validators_proposed_app_versions;
#[cfg(feature = "full")]
mod update_validator_proposed_app_version;
#[cfg(any(feature = "full", feature = "verify"))]
/// constant id for various versions counter
pub const VERSIONS_COUNTER: [u8; 1] = [0];

#[cfg(any(feature = "full", feature = "verify"))]
/// constant id for subtree containing the desired versions for each validator
pub const VALIDATOR_DESIRED_VERSIONS: [u8; 1] = [1];

#[cfg(feature = "full")]
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

#[cfg(any(feature = "full", feature = "verify"))]
/// versions counter path
pub fn versions_counter_path() -> [&'static [u8]; 2] {
    [
        Into::<&[u8; 1]>::into(RootTree::Versions),
        VERSIONS_COUNTER.as_slice(),
    ]
}

#[cfg(any(feature = "full", feature = "verify"))]
/// versions counter path
pub fn versions_counter_path_vec() -> Vec<Vec<u8>> {
    vec![vec![RootTree::Versions as u8], VERSIONS_COUNTER.to_vec()]
}

#[cfg(any(feature = "full", feature = "verify"))]
/// desired version for validators path
pub fn desired_version_for_validators_path() -> [&'static [u8]; 2] {
    [
        Into::<&[u8; 1]>::into(RootTree::Versions),
        VALIDATOR_DESIRED_VERSIONS.as_slice(),
    ]
}

#[cfg(any(feature = "full", feature = "verify"))]
/// desired version for validators path
pub fn desired_version_for_validators_path_vec() -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::Versions as u8],
        VALIDATOR_DESIRED_VERSIONS.to_vec(),
    ]
}
