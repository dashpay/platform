use crate::state_transition_action::batch::batched_transition::BatchedTransitionAction;
use crate::state_transition_action::batch::v0::BatchTransitionActionV0;
use derive_more::From;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::drive::contract_groups::types::ContractGroupMembershipsForContract;
use dpp::fee::fee_result::FeeResult;
use dpp::consensus::ConsensusError;
use dpp::fee::Credits;
use dpp::identity::SecurityLevel;
use dpp::platform_value::Identifier;
use dpp::prelude::UserFeeIncrease;
use dpp::ProtocolError;
use std::collections::BTreeSet;
use crate::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::DocumentBaseTransitionActionAccessorsV0;

/// batched transition
pub mod batched_transition;
/// v0
pub mod v0;

#[cfg(test)]
mod tests;

/// A contract's group memberships as the batch transformer read them, with the fee of that
/// read. The fee travels with the data, like a contract's fetch info: the transformer does not
/// bill it, the check that uses the answer does, so a batch signed by an ordinary key pays
/// nothing for it.
#[derive(Debug, Clone, Default)]
pub struct ResolvedContractGroupMemberships {
    /// The groups the contract, its document types and its tokens belong to
    pub memberships: ContractGroupMembershipsForContract,
    /// What reading them cost
    pub fee: FeeResult,
}

/// The contract owner a batch asks to pay its gas, with the balance the batch transformer read
/// for them. Resolved from protocol version 14, and only when the contract owner is not the
/// batch's own signer: fee validation checks the fee against this balance, and execution charges
/// this identity instead of the signer when it covers the fee. A batch that failed validation
/// is never sponsored, whatever it asked for: its signer pays for the work that ran.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedGasSponsor {
    /// The contract owner
    pub identity_id: Identifier,
    /// Their balance when the batch was transformed
    pub balance: Credits,
    /// Whether the batch insists on the contract owner paying (`GasFeesPaidBy::ContractOwner`):
    /// then a balance that does not cover the fee refuses the batch unpaid. Otherwise
    /// (`PreferContractOwner`) the signer pays instead.
    pub strict: bool,
}

impl ResolvedGasSponsor {
    /// Whether the contract owner pays: their balance covers the whole fee. Fee validation and
    /// execution both decide by this, on the same estimated fee, so they always agree.
    pub fn covers(&self, required_balance: Credits) -> bool {
        self.balance >= required_balance
    }
}

/// Who pays the gas of a whole batch, as `GasFeesPaidBy::resolve` names it for each of its
/// transitions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GasPayer {
    /// The signer of the batch
    DocumentOwner,
    /// The owner of the contract every transition of the batch is on
    ContractOwner {
        /// The contract owner
        identity_id: Identifier,
        /// Whether any transition insists on the contract owner paying
        strict: bool,
    },
}

/// documents batch transition action
#[derive(Debug, Clone, From)]
pub enum BatchTransitionAction {
    /// v0
    V0(BatchTransitionActionV0),
}

impl BatchTransitionAction {
    /// owner id
    pub fn owner_id(&self) -> Identifier {
        match self {
            BatchTransitionAction::V0(v0) => v0.owner_id,
        }
    }

    /// transitions
    pub fn transitions(&self) -> &Vec<BatchedTransitionAction> {
        match self {
            BatchTransitionAction::V0(v0) => &v0.transitions,
        }
    }

    /// transitions
    pub fn transitions_mut(&mut self) -> &mut Vec<BatchedTransitionAction> {
        match self {
            BatchTransitionAction::V0(v0) => &mut v0.transitions,
        }
    }

    /// transitions
    pub fn transitions_take(&mut self) -> Vec<BatchedTransitionAction> {
        match self {
            BatchTransitionAction::V0(v0) => std::mem::take(&mut v0.transitions),
        }
    }

    /// transitions owned
    pub fn transitions_owned(self) -> Vec<BatchedTransitionAction> {
        match self {
            BatchTransitionAction::V0(v0) => v0.transitions,
        }
    }

    /// set transitions
    pub fn set_transitions(&mut self, transitions: Vec<BatchedTransitionAction>) {
        match self {
            BatchTransitionAction::V0(v0) => v0.transitions = transitions,
        }
    }

    /// fee multiplier
    pub fn user_fee_increase(&self) -> UserFeeIncrease {
        match self {
            BatchTransitionAction::V0(transition) => transition.user_fee_increase,
        }
    }

    /// The group memberships the transformer resolved for a contract the batch touches
    pub fn contract_group_memberships(
        &self,
        contract_id: &Identifier,
    ) -> Option<&ResolvedContractGroupMemberships> {
        match self {
            BatchTransitionAction::V0(v0) => v0.contract_group_memberships.get(contract_id),
        }
    }

    /// Records the group memberships of a contract the batch touches
    pub fn set_contract_group_memberships(
        &mut self,
        contract_id: Identifier,
        resolved: ResolvedContractGroupMemberships,
    ) {
        match self {
            BatchTransitionAction::V0(v0) => {
                v0.contract_group_memberships.insert(contract_id, resolved);
            }
        }
    }

    /// The contracts on which the batch sweeps its owner's lapsed suspension when it executes
    pub fn lapsed_suspensions(&self) -> &BTreeSet<Identifier> {
        match self {
            BatchTransitionAction::V0(v0) => &v0.lapsed_suspensions,
        }
    }
}

impl BatchTransitionAction {
    /// The sum of all purchases amount and all conflicting index collateral voting funds
    pub fn all_used_balances(&self) -> Result<Option<Credits>, ProtocolError> {
        match self {
            BatchTransitionAction::V0(v0) => v0.all_used_balances(),
        }
    }

    /// The contract owner the batch transformer resolved as the gas sponsor, if any
    pub fn gas_sponsor(&self) -> Option<&ResolvedGasSponsor> {
        match self {
            BatchTransitionAction::V0(v0) => v0.gas_sponsor.as_ref(),
        }
    }

    /// Records the contract owner who sponsors the batch's gas, with their balance
    pub fn set_gas_sponsor(&mut self, gas_sponsor: Option<ResolvedGasSponsor>) {
        match self {
            BatchTransitionAction::V0(v0) => v0.gas_sponsor = gas_sponsor,
        }
    }

    /// Who pays the gas of the batch, or the consensus error explaining why the batch's
    /// requests cannot be honoured: a transition asking for more than its document type's token
    /// cost offers, or a batch whose transitions do not all name the same payer.
    pub fn resolve_gas_payer(&self) -> Result<GasPayer, ConsensusError> {
        match self {
            BatchTransitionAction::V0(v0) => v0.resolve_gas_payer(),
        }
    }

    /// The sum of all purchases amounts for all purchase transitions in the batch
    pub fn all_purchases_amount(&self) -> Result<Option<Credits>, ProtocolError> {
        match self {
            BatchTransitionAction::V0(v0) => v0.all_purchases_amount(),
        }
    }

    /// The sum of all conflicting index collateral voting funds for all document create transitions in the batch
    pub fn all_conflicting_index_collateral_voting_funds(
        &self,
    ) -> Result<Option<Credits>, ProtocolError> {
        match self {
            BatchTransitionAction::V0(v0) => v0.all_conflicting_index_collateral_voting_funds(),
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
    pub fn combined_security_level_requirement(&self) -> Result<Vec<SecurityLevel>, ProtocolError> {
        // Step 1: Get all document types for the ST
        // Step 2: Get document schema for every type
        // If schema has security level, use that, if not, use the default security level
        // Find the highest level (lowest int value) of all documents - the ST's signature
        // requirement is the highest level across all documents affected by the ST./
        let mut highest_security_level = SecurityLevel::lowest_level();

        for transition in self.transitions().iter() {
            match transition {
                BatchedTransitionAction::DocumentAction(document_transition) => {
                    let document_type_name = document_transition.base().document_type_name();
                    let data_contract_info = document_transition.base().data_contract_fetch_info();

                    let document_type = data_contract_info
                        .contract
                        .document_type_for_name(document_type_name)?;

                    let document_security_level = document_type.security_level_requirement();

                    // lower enum representation means higher in security
                    if document_security_level < highest_security_level {
                        highest_security_level = document_security_level
                    }
                }
                BatchedTransitionAction::TokenAction(_) => {
                    // lower enum representation means higher in security
                    if highest_security_level != SecurityLevel::MASTER {
                        highest_security_level = SecurityLevel::CRITICAL
                    }
                }
                BatchedTransitionAction::BumpIdentityDataContractNonce(_) => {}
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
