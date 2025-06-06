// Evonode queries
pub mod proposed_epoch_blocks_by_ids;
pub mod proposed_epoch_blocks_by_range;

// Re-export all public functions for convenient access
pub use proposed_epoch_blocks_by_ids::dash_sdk_evonode_get_proposed_epoch_blocks_by_ids;
pub use proposed_epoch_blocks_by_range::dash_sdk_evonode_get_proposed_epoch_blocks_by_range;
