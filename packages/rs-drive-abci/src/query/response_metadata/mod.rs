mod v0;

use crate::platform_types::platform_state::PlatformState;
use std::sync::Arc;

/// Specifies which GroveDB instance was used for a query, along with the associated platform state
#[derive(Debug, Clone)]
pub enum CheckpointUsed {
    /// The current (main) GroveDB was used
    Current,
    /// A checkpoint was used, with the platform state at that checkpoint
    Checkpoint(Arc<PlatformState>),
}
