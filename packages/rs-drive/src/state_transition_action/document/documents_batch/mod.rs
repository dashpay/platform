use crate::state_transition_action::document::documents_batch::document_transition::DocumentTransitionAction;
use crate::state_transition_action::document::documents_batch::v0::DocumentsBatchTransitionActionV0;
use derive_more::From;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::fee::Credits;
use dpp::identity::SecurityLevel;
use dpp::platform_value::Identifier;
use dpp::prelude::UserFeeIncrease;
use dpp::ProtocolError;
use crate::state_transition_action::document::documents_batch::document_transition::document_base_transition_action::DocumentBaseTransitionActionAccessorsV0;

/// document transition
pub mod document_transition;
/// v0
pub mod v0;

/// documents batch transition action
#[derive(Debug, Clone, From)]
pub enum DocumentsBatchTransitionAction {
    /// v0
    V0(DocumentsBatchTransitionActionV0),
}

impl DocumentsBatchTransitionAction {
    /// owner id
    pub fn owner_id(&self) -> Identifier {
        match self {
            DocumentsBatchTransitionAction::V0(v0) => v0.owner_id,
        }
    }

    /// transitions
    pub fn transitions(&self) -> &Vec<DocumentTransitionAction> {
        match self {
            DocumentsBatchTransitionAction::V0(v0) => &v0.transitions,
        }
    }

    /// transitions
    pub fn transitions_mut(&mut self) -> &mut Vec<DocumentTransitionAction> {
        match self {
            DocumentsBatchTransitionAction::V0(v0) => &mut v0.transitions,
        }
    }

    /// transitions
    pub fn transitions_take(&mut self) -> Vec<DocumentTransitionAction> {
        match self {
            DocumentsBatchTransitionAction::V0(v0) => std::mem::take(&mut v0.transitions),
        }
    }

    /// transitions owned
    pub fn transitions_owned(self) -> Vec<DocumentTransitionAction> {
        match self {
            DocumentsBatchTransitionAction::V0(v0) => v0.transitions,
        }
    }

    /// set transitions
    pub fn set_transitions(&mut self, transitions: Vec<DocumentTransitionAction>) {
        match self {
            DocumentsBatchTransitionAction::V0(v0) => v0.transitions = transitions,
        }
    }

    /// fee multiplier
    pub fn user_fee_increase(&self) -> UserFeeIncrease {
        match self {
            DocumentsBatchTransitionAction::V0(transition) => transition.user_fee_increase,
        }
    }
}

impl DocumentsBatchTransitionAction {
    /// The sum of all purchases amount and all conflicting index collateral voting funds
    pub fn all_used_balances(&self) -> Result<Option<Credits>, ProtocolError> {
        match self {
            DocumentsBatchTransitionAction::V0(v0) => v0.all_used_balances(),
        }
    }

    /// The sum of all purchases amounts for all purchase transitions in the batch
    pub fn all_purchases_amount(&self) -> Result<Option<Credits>, ProtocolError> {
        match self {
            DocumentsBatchTransitionAction::V0(v0) => v0.all_purchases_amount(),
        }
    }

    /// The sum of all conflicting index collateral voting funds for all document create transitions in the batch
    pub fn all_conflicting_index_collateral_voting_funds(
        &self,
    ) -> Result<Option<Credits>, ProtocolError> {
        match self {
            DocumentsBatchTransitionAction::V0(v0) => {
                v0.all_conflicting_index_collateral_voting_funds()
            }
        }
    }

    /// Determines the security level requirements for the batch transition action.
    ///
    /// This method performs the following steps:
    ///
    /// 1. Retrieves all document types associated with the state transitions (STs) in the batch.
    /// 2. For each document type, fetches its schema to determine its security level requirement.
    ///    - If the schema specifies a security level, that is used.
    ///    - Otherwise, a default security level is used.
    ///
    /// The method then determines the highest security level (which corresponds to the lowest
    /// integer value of the `SecurityLevel` enum) across all documents affected by the state transitions.
    /// This highest level becomes the signature requirement for the entire batch transition action.
    ///
    /// # Returns
    ///
    /// - Returns a `Result` containing a `Vec<SecurityLevel>` which is the list of security
    ///   levels required for the batch transition action.
    /// - Returns an `Err` of type `ProtocolError` if any error occurs during the process.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // Assuming `batch_transition_action` is an instance of `DocumentsBatchTransitionAction`
    /// let required_levels = batch_transition_action.contract_based_security_level_requirement()?;
    /// ```
    ///
    pub fn contract_based_security_level_requirement(
        &self,
    ) -> Result<Vec<SecurityLevel>, ProtocolError> {
        // Step 1: Get all document types for the ST
        // Step 2: Get document schema for every type
        // If schema has security level, use that, if not, use the default security level
        // Find the highest level (lowest int value) of all documents - the ST's signature
        // requirement is the highest level across all documents affected by the ST./
        let mut highest_security_level = SecurityLevel::lowest_level();

        for transition in self.transitions().iter() {
            let document_type_name = transition
                .base()
                .ok_or(ProtocolError::CorruptedCodeExecution(
                    "expecting action to have a base".to_string(),
                ))?
                .document_type_name();
            let data_contract_info = transition
                .base()
                .ok_or(ProtocolError::CorruptedCodeExecution(
                    "expecting action to have a base".to_string(),
                ))?
                .data_contract_fetch_info();

            let document_type = data_contract_info
                .contract
                .document_type_for_name(document_type_name)?;

            let document_security_level = document_type.security_level_requirement();

            // lower enum enum representation means higher in security
            if document_security_level < highest_security_level {
                highest_security_level = document_security_level
            }
        }
        Ok(if highest_security_level == SecurityLevel::MASTER {
            vec![SecurityLevel::MASTER]
        } else {
            // this might seem wrong until you realize that master is 0, critical 1, etc
            (SecurityLevel::CRITICAL as u8..=highest_security_level as u8)
                .map(|security_level| SecurityLevel::try_from(security_level).unwrap())
                .collect()
        })
    }
}
