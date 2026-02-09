/// Updating the state cache happens as the final part of block finalization
pub(in crate::execution) mod update_state_cache;
/// Validator set update
pub(in crate::execution) mod validator_set_update;

/// Updating the drive cache happens as the final part of block finalization
pub(in crate::execution) mod update_drive_cache;

/// Determines whether a checkpoint should be created
pub(in crate::execution) mod should_checkpoint;

/// Updates checkpoints (legacy - calls should_checkpoint then creates checkpoint)
pub(in crate::execution) mod update_checkpoints;

/// Creates a GroveDB checkpoint (called after transaction commit)
pub(crate) mod create_grovedb_checkpoint;
