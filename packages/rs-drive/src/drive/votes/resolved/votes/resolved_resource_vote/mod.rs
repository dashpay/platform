use crate::drive::votes::resolved::votes::resolved_resource_vote::v0::ResolvedResourceVoteV0;

/// Module containing accessors for various components.
pub mod accessors;

/// Module containing version 0 of the implementation.
pub mod v0;

/// Module containing logic to resolve resources.
#[cfg(feature = "server")]
pub(crate) mod resolve;

/// Represents a resolved resource vote in the system.
#[derive(Debug, Clone, PartialEq)]
pub enum ResolvedResourceVote {
    /// Version 0 of the resolved resource vote.
    V0(ResolvedResourceVoteV0),
}
