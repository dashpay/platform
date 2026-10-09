/// The schema depth check `check_tx` runs before parsing a contract.
pub(in crate::execution) mod check_tx_schema_depth;

/// Validation of the reference declarations a contract's document types carry.
pub mod data_contract_reference_validation;

/// The refusal of a document type's token cost in another contract's non-transferable token.
pub(in crate::execution) mod non_transferable_token_cost;

/// Runs a contract breaking a structural rule of the document type parser through `check_tx`
/// and block processing, for the create and update suites.
#[cfg(test)]
pub(in crate::execution) mod contract_structure_test_harness;
