//! Identity lifecycle status and DPNS name metadata for managed identities.

/// Identity lifecycle status on Platform.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum IdentityStatus {
    #[default]
    Unknown,
    PendingCreation,
    Active,
    FailedCreation,
    NotFound,
}

/// DPNS username associated with an identity.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DpnsNameInfo {
    pub label: String,
    pub acquired_at: Option<u64>,
}
