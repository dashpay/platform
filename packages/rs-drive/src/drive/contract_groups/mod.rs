//! Contract groups: identity-owned sets of contracts, contract document types and contract tokens.
//!
//! See [`paths`] for the layout of the `ContractGroups` root tree.

#[cfg(feature = "server")]
mod estimated_costs;
#[cfg(feature = "server")]
mod fetch;
#[cfg(feature = "server")]
mod insert;
/// Paths of the `ContractGroups` root tree.
pub mod paths;
#[cfg(feature = "server")]
mod prove;
mod queries;
/// Result types shared by the fetch and verify sides.
pub mod types;

#[cfg(test)]
#[cfg(feature = "server")]
mod tests;
