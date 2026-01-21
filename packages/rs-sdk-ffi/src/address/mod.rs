//! Address operations (queries and state transitions)

pub mod queries;
pub mod transitions;

// Re-export query functions
pub use queries::*;

// Re-export transition functions
pub use transitions::*;
