//! The capability table.

use alloc::vec::Vec;

use crate::declare::{CapabilityRequirement, CapabilityStatus};

/// One required capability with its catalogue status.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct CapabilityEntry {
    /// The requirement.
    pub requirement: CapabilityRequirement,
    /// Its status in the catalogue.
    pub status: CapabilityStatus,
}

/// Every capability the contract needs, explicit and derived, sorted.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CapabilityTable {
    /// The entries.
    pub entries: Vec<CapabilityEntry>,
}

impl CapabilityTable {
    /// Whether the table contains the requirement.
    pub fn requires(&self, requirement: CapabilityRequirement) -> bool {
        self.entries
            .iter()
            .any(|entry| entry.requirement == requirement)
    }

    /// The requirements whose status is not [`CapabilityStatus::Native`].
    pub fn pending(&self) -> impl Iterator<Item = &CapabilityEntry> {
        self.entries
            .iter()
            .filter(|entry| entry.status != CapabilityStatus::Native)
    }
}
