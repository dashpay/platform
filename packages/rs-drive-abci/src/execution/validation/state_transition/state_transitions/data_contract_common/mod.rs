/// Validation of the reference declarations a contract's document types carry.
pub mod data_contract_reference_validation;

/// Runs a contract breaking a structural rule of the document type parser through `check_tx`
/// and block processing, for the create and update suites.
#[cfg(test)]
pub(in crate::execution) mod contract_structure_test_harness;
