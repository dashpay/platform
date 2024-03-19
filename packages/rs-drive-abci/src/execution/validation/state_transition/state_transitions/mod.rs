/// Module containing functionality related to batch processing of documents.
pub mod documents_batch;

/// Module for creating an identity entity.
pub mod identity_create;

/// Module for managing transfers of credit between identity entities.
pub mod identity_credit_transfer;

/// Module for managing withdrawals of credit from an identity entity.
pub mod identity_credit_withdrawal;

/// Module for topping up credit in an identity entity.
pub mod identity_top_up;

/// Module for updating an existing identity entity.
pub mod identity_update;

/// Module for creating a data contract entity.
pub mod data_contract_create;

/// Module for updating an existing data contract entity.
pub mod data_contract_update;

/// The validation mode we are using
#[derive(Clone, Copy, Debug)]
pub enum ValidationMode {
    /// The basic checktx before the state transition is put into mempool
    CheckTx,
    /// Rechecking a state transition every block
    RecheckTx,
    /// The validation of the validator
    Validator,
}

impl ValidationMode {
    /// Can this validation mode alter cache on drive?
    pub fn can_alter_cache(&self) -> bool {
        match self {
            ValidationMode::CheckTx => false,
            ValidationMode::RecheckTx => false,
            ValidationMode::Validator => true,
        }
    }
}
