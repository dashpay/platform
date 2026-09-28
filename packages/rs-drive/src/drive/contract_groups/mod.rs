//! Contract groups: identity-owned sets of contracts, contract document types and contract tokens.
//!
//! See [`paths`] for the layout of the `ContractGroups` root tree.

#[cfg(feature = "server")]
mod estimated_costs;
#[cfg(feature = "server")]
mod fetch;
#[cfg(feature = "server")]
mod insert;
#[cfg(feature = "server")]
mod insert_contract_groups_structure;
/// Paths of the `ContractGroups` root tree.
pub mod paths;
#[cfg(feature = "server")]
mod prove;
mod queries;
#[cfg(all(feature = "server", any(test, feature = "structure")))]
pub(crate) mod structure;
/// Result types shared by the fetch and verify sides.
pub mod types;

#[cfg(test)]
#[cfg(feature = "server")]
mod tests;
