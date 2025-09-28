use crate::drive::subscriptions::TransitionCheckResult;
use crate::error::query::QuerySyntaxError;
use crate::query::{QuerySyntaxSimpleValidationResult, ValueClause, WhereOperator};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
use dpp::data_contract::associated_token::token_configuration::TokenConfiguration;
use dpp::data_contract::associated_token::token_distribution_rules::accessors::v0::TokenDistributionRulesV0Getters;
use dpp::data_contract::associated_token::token_pre_programmed_distribution::TokenPreProgrammedDistribution;
use dpp::data_contract::associated_token::token_pre_programmed_distribution::accessors::v0::TokenPreProgrammedDistributionV0Methods;
use dpp::data_contract::serialized_version::DataContractInSerializationFormat;
use dpp::data_contract::group::accessors::v0::GroupV0Getters;
use dpp::data_contract::group::Group;
use dpp::data_contract::{DataContract, GroupContractPosition, TokenContractPosition};
use dpp::platform_value::Value;
use dpp::prelude::Identifier;
use dpp::state_transition::data_contract_create_transition::accessors::DataContractCreateTransitionAccessorsV0;
use dpp::state_transition::data_contract_create_transition::DataContractCreateTransition;
use dpp::state_transition::data_contract_update_transition::accessors::DataContractUpdateTransitionAccessorsV0;
use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use dpp::state_transition::StateTransition;
use std::collections::{BTreeMap, BTreeSet};

/// Filter describing constraints for contract create/update transitions.
#[derive(Debug, Clone, PartialEq)]
pub struct DriveContractQueryFilter {
    /// Action-specific clauses.
    pub action_clauses: ContractActionMatchClauses,
}

/// Clauses that reference immutable contract properties (id/owner).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BaseContractClauses {
    /// Optional clause evaluated against the contract identifier.
    pub contract_id_clause: Option<ValueClause>,
    /// Optional clause evaluated against the contract owner identifier.
    pub owner_clause: Option<ValueClause>,
}

impl BaseContractClauses {
    fn is_empty(&self) -> bool {
        self.contract_id_clause.is_none() && self.owner_clause.is_none()
    }
}

/// Clauses describing token configurations that must exist on the contract.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TokenConfigurationClauses {
    /// Token requirements that must be satisfied by tokens on the contract.
    pub requirements: Vec<TokenConfigurationRequirement>,
}

/// Requirement expressing constraints for an individual token configuration.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TokenConfigurationRequirement {
    /// Optional explicit position on the contract. When `None`, any token may satisfy the requirement.
    pub position: Option<TokenContractPosition>,
    /// Optional constraint on whether the token defines a perpetual distribution section.
    pub has_perpetual_distribution: Option<bool>,
    /// Optional constraint on whether the token contains pre-programmed distribution rules.
    pub has_pre_programmed_distribution: Option<bool>,
    /// Optional clause for identities present within the pre-programmed distribution map.
    pub pre_programmed_distribution_identity_clause: Option<ValueClause>,
}

/// Clauses describing document types that must be present on the contract.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DocumentTypeClauses {
    /// Document type names that must exist.
    pub required_types: Vec<String>,
}

/// Clauses describing group configurations that must exist on the contract.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct GroupClauses {
    /// Group requirements that must be satisfied.
    pub requirements: Vec<GroupRequirement>,
}

/// Requirement describing a single group condition.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct GroupRequirement {
    /// Optional explicit position for the group. When `None`, any group may satisfy the requirement.
    pub position: Option<GroupContractPosition>,
    /// Optional clause constraining group membership by identity.
    pub member_identity_clause: Option<ValueClause>,
}

/// Clauses describing token configurations that must be added during an update.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AddedTokenConfigurationClauses {
    /// Token requirements that must be satisfied by newly-added tokens.
    pub requirements: Vec<TokenConfigurationRequirement>,
}

impl AddedTokenConfigurationClauses {
    fn is_empty(&self) -> bool {
        self.requirements.is_empty()
    }
}

/// Clauses applied depending on the contract transition variant.
#[derive(Debug, Clone, PartialEq)]
pub enum ContractActionMatchClauses {
    /// Clauses evaluated against a data contract create transition.
    Create {
        /// Base contract clauses to enforce.
        base_contract_clauses: BaseContractClauses,
        /// Token configuration clauses required on creation.
        token_configuration_clauses: TokenConfigurationClauses,
        /// Document type clauses that must be satisfied on creation.
        document_type_clauses: DocumentTypeClauses,
        /// Group clauses that must be satisfied on creation.
        group_clauses: GroupClauses,
    },
    /// Clauses evaluated against a data contract update transition.
    Update {
        /// Base contract clauses enforced across the update.
        base_contract_clauses: BaseContractClauses,
        /// Token configuration clauses that must remain satisfied after the update.
        token_configuration_clauses: TokenConfigurationClauses,
        /// Token configuration clauses describing new tokens introduced by the update.
        added_token_configuration_clauses: AddedTokenConfigurationClauses,
    },
}

impl DriveContractQueryFilter {
    /// Validates operator/value compatibility for all configured clauses.
    pub fn validate(&self) -> QuerySyntaxSimpleValidationResult {
        match &self.action_clauses {
            ContractActionMatchClauses::Create {
                base_contract_clauses,
                token_configuration_clauses,
                document_type_clauses,
                group_clauses,
            } => {
                let result = validate_base_contract_clauses(base_contract_clauses);
                if result.is_err() {
                    return result;
                }
                let result = validate_token_configuration_clauses(token_configuration_clauses);
                if result.is_err() {
                    return result;
                }
                let result = validate_document_type_clauses(document_type_clauses);
                if result.is_err() {
                    return result;
                }
                validate_group_clauses(group_clauses)
            }
            ContractActionMatchClauses::Update {
                base_contract_clauses,
                token_configuration_clauses,
                added_token_configuration_clauses,
            } => {
                let result = validate_base_contract_clauses(base_contract_clauses);
                if result.is_err() {
                    return result;
                }
                let result = validate_token_configuration_clauses(token_configuration_clauses);
                if result.is_err() {
                    return result;
                }
                validate_added_token_configuration_clauses(added_token_configuration_clauses)
            }
        }
    }

    /// Evaluates the filter against a state transition.
    pub fn matches_state_transition(&self, state_transition: &StateTransition) -> TransitionCheckResult {
        match (self, state_transition) {
            (
                DriveContractQueryFilter {
                    action_clauses: ContractActionMatchClauses::Create { .. },
                },
                StateTransition::DataContractCreate(create_transition),
            ) => self.matches_create_transition(create_transition),
            (
                DriveContractQueryFilter {
                    action_clauses: ContractActionMatchClauses::Update { .. },
                },
                StateTransition::DataContractUpdate(update_transition),
            ) => self.matches_update_transition(update_transition),
            _ => TransitionCheckResult::Fail,
        }
    }

    fn matches_create_transition(
        &self,
        transition: &DataContractCreateTransition,
    ) -> TransitionCheckResult {
        let ContractActionMatchClauses::Create {
            base_contract_clauses,
            token_configuration_clauses,
            document_type_clauses,
            group_clauses,
        } = &self.action_clauses else {
            return TransitionCheckResult::Fail;
        };

        let contract = transition.data_contract();

        if !matches_base_contract_clauses(base_contract_clauses, contract.id(), contract.owner_id()) {
            return TransitionCheckResult::Fail;
        }

        if !tokens_present_in_contract(contract, &token_configuration_clauses.requirements) {
            return TransitionCheckResult::Fail;
        }

        if !document_types_present_in_contract(contract, &document_type_clauses.required_types) {
            return TransitionCheckResult::Fail;
        }

        if !groups_present_in_contract(contract, &group_clauses.requirements) {
            return TransitionCheckResult::Fail;
        }

        TransitionCheckResult::Pass
    }

    fn matches_update_transition(
        &self,
        transition: &DataContractUpdateTransition,
    ) -> TransitionCheckResult {
        let ContractActionMatchClauses::Update {
            base_contract_clauses,
            token_configuration_clauses,
            added_token_configuration_clauses,
        } = &self.action_clauses else {
            return TransitionCheckResult::Fail;
        };

        let contract = transition.data_contract();

        if !matches_base_contract_clauses(base_contract_clauses, contract.id(), contract.owner_id()) {
            return TransitionCheckResult::Fail;
        }

        if !tokens_present_in_contract(contract, &token_configuration_clauses.requirements) {
            return TransitionCheckResult::Fail;
        }

        if !tokens_present_in_contract(contract, &added_token_configuration_clauses.requirements) {
            return TransitionCheckResult::Fail;
        }

        if base_contract_clauses.is_empty() && added_token_configuration_clauses.is_empty() {
            TransitionCheckResult::Pass
        } else {
            TransitionCheckResult::NeedsOriginal
        }
    }

    /// Evaluates clauses that require the original contract (pre-update) state.
    pub fn matches_original_contract(
        &self,
        original_contract: &DataContract,
    ) -> bool {
        match &self.action_clauses {
            ContractActionMatchClauses::Update {
                base_contract_clauses,
                added_token_configuration_clauses,
                ..
            } => {
                if !matches_base_contract_clauses(
                    base_contract_clauses,
                    original_contract.id(),
                    original_contract.owner_id(),
                ) {
                    return false;
                }

                return false;

                //tokens_added_between(original_contract, added_token_configuration_clauses)
            }
            _ => false,
        }
    }
}

fn matches_base_contract_clauses(
    clauses: &BaseContractClauses,
    contract_id: Identifier,
    owner_id: Identifier,
) -> bool {
    if let Some(clause) = &clauses.contract_id_clause {
        if !clause.matches_value(&Value::from(contract_id)) {
            return false;
        }
    }

    if let Some(clause) = &clauses.owner_clause {
        if !clause.matches_value(&Value::from(owner_id)) {
            return false;
        }
    }

    true
}

fn validate_base_contract_clauses(clauses: &BaseContractClauses) -> QuerySyntaxSimpleValidationResult {
    if let Some(clause) = &clauses.contract_id_clause {
        if !identifier_clause_is_valid(clause) {
            return invalid_clause_result("invalid contract id clause");
        }
    }

    if let Some(clause) = &clauses.owner_clause {
        if !identifier_clause_is_valid(clause) {
            return invalid_clause_result("invalid owner clause");
        }
    }

    QuerySyntaxSimpleValidationResult::new()
}

fn validate_token_configuration_clauses(
    clauses: &TokenConfigurationClauses,
) -> QuerySyntaxSimpleValidationResult {
    let mut seen_positions = BTreeSet::new();
    for requirement in &clauses.requirements {
        if let Some(position) = &requirement.position {
            if !seen_positions.insert(position.clone()) {
                return invalid_clause_result("duplicate token position in clause");
            }
        }

        if let Some(identity_clause) = &requirement.pre_programmed_distribution_identity_clause {
            if !identifier_clause_is_valid(identity_clause) {
                return invalid_clause_result(
                    "invalid pre-programmed distribution identity clause",
                );
            }
        }

        if requirement.pre_programmed_distribution_identity_clause.is_some()
            && matches!(requirement.has_pre_programmed_distribution, Some(false))
        {
            return invalid_clause_result(
                "pre-programmed distribution identity clause requires distribution to exist",
            );
        }
    }

    QuerySyntaxSimpleValidationResult::new()
}

fn validate_document_type_clauses(
    clauses: &DocumentTypeClauses,
) -> QuerySyntaxSimpleValidationResult {
    let mut seen_types = BTreeSet::new();
    for document_type in &clauses.required_types {
        if document_type.is_empty() {
            return invalid_clause_result("document type requirement must not be empty");
        }

        if !seen_types.insert(document_type.clone()) {
            return invalid_clause_result("duplicate document type requirement");
        }
    }

    QuerySyntaxSimpleValidationResult::new()
}

fn validate_group_clauses(clauses: &GroupClauses) -> QuerySyntaxSimpleValidationResult {
    let mut seen_positions = BTreeSet::new();
    for requirement in &clauses.requirements {
        if let Some(position) = &requirement.position {
            if !seen_positions.insert(position.clone()) {
                return invalid_clause_result("duplicate group position in clause");
            }
        }

        if let Some(identity_clause) = &requirement.member_identity_clause {
            if !identifier_clause_is_valid(identity_clause) {
                return invalid_clause_result("invalid group member identity clause");
            }
        }
    }

    QuerySyntaxSimpleValidationResult::new()
}

fn validate_added_token_configuration_clauses(
    clauses: &AddedTokenConfigurationClauses,
) -> QuerySyntaxSimpleValidationResult {
    let mut seen_positions = BTreeSet::new();
    for requirement in &clauses.requirements {
        let Some(position) = &requirement.position else {
            return invalid_clause_result("added token clause must specify a position");
        };

        if !seen_positions.insert(position.clone()) {
            return invalid_clause_result("duplicate added token position");
        }

        if let Some(identity_clause) = &requirement.pre_programmed_distribution_identity_clause {
            if !identifier_clause_is_valid(identity_clause) {
                return invalid_clause_result(
                    "invalid pre-programmed distribution identity clause",
                );
            }
        }

        if requirement.pre_programmed_distribution_identity_clause.is_some()
            && matches!(requirement.has_pre_programmed_distribution, Some(false))
        {
            return invalid_clause_result(
                "pre-programmed distribution identity clause requires distribution to exist",
            );
        }
    }

    QuerySyntaxSimpleValidationResult::new()
}

fn identifier_clause_is_valid(clause: &ValueClause) -> bool {
    match clause.operator {
        WhereOperator::Equal => matches!(clause.value, Value::Identifier(_)),
        WhereOperator::In => match &clause.value {
            Value::Array(values) => values.iter().all(|v| matches!(v, Value::Identifier(_))),
            _ => false,
        },
        _ => false,
    }
}

fn invalid_clause_result(message: &'static str) -> QuerySyntaxSimpleValidationResult {
    QuerySyntaxSimpleValidationResult::new_with_error(
        QuerySyntaxError::InvalidWhereClauseComponents(message),
    )
}

fn tokens_present_in_contract(
    contract_serialized: &DataContractInSerializationFormat,
    requirements: &[TokenConfigurationRequirement],
) -> bool {
    if requirements.is_empty() {
        return true;
    }

    let tokens = contract_tokens(contract_serialized);
    token_requirements_satisfied(tokens, requirements)
}

fn document_types_present_in_contract(
    contract_serialized: &DataContractInSerializationFormat,
    required_types: &[String],
) -> bool {
    if required_types.is_empty() {
        return true;
    }

    let document_schemas = contract_serialized.document_schemas();
    required_types
        .iter()
        .all(|document_type| document_schemas.contains_key(document_type))
}

fn tokens_added_between(
    original_contract: &DataContract,
    updated_contract_serialized: &DataContractInSerializationFormat,
    clauses: &AddedTokenConfigurationClauses,
) -> bool {
    if clauses.is_empty() {
        return true;
    }

    let original_positions = contract_token_positions(original_contract);
    let updated_tokens = contract_tokens(updated_contract_serialized);

    clauses.requirements.iter().all(|requirement| {
        let position = requirement.position.as_ref().expect("validated to exist");
        match updated_tokens.get(position) {
            Some(token_config) => {
                if original_positions.contains(position) {
                    return false;
                }
                token_requirement_is_met(token_config, requirement)
            }
            None => false,
        }
    })
}

fn token_requirements_satisfied(
    tokens: &BTreeMap<TokenContractPosition, TokenConfiguration>,
    requirements: &[TokenConfigurationRequirement],
) -> bool {
    requirements.iter().all(|requirement| match &requirement.position {
        Some(position) => tokens
            .get(position)
            .map(|token| token_requirement_is_met(token, requirement))
            .unwrap_or(false),
        None => tokens
            .values()
            .any(|token| token_requirement_is_met(token, requirement)),
    })
}

fn token_requirement_is_met(
    token_config: &TokenConfiguration,
    requirement: &TokenConfigurationRequirement,
) -> bool {
    if let Some(expected) = requirement.has_perpetual_distribution {
        if token_has_perpetual_distribution(token_config) != expected {
            return false;
        }
    }

    if let Some(expected) = requirement.has_pre_programmed_distribution {
        if token_has_pre_programmed_distribution(token_config) != expected {
            return false;
        }
    }

    if let Some(identity_clause) = &requirement.pre_programmed_distribution_identity_clause {
        let Some(distribution) = token_pre_programmed_distribution(token_config) else {
            return false;
        };

        if !pre_programmed_distribution_identity_matches(distribution, identity_clause) {
            return false;
        }
    }

    true
}

fn token_has_perpetual_distribution(token_config: &TokenConfiguration) -> bool {
    match token_config {
        TokenConfiguration::V0(v0) => v0.distribution_rules().perpetual_distribution().is_some(),
    }
}

fn token_has_pre_programmed_distribution(token_config: &TokenConfiguration) -> bool {
    token_pre_programmed_distribution(token_config).is_some()
}

fn token_pre_programmed_distribution(
    token_config: &TokenConfiguration,
) -> Option<&TokenPreProgrammedDistribution> {
    match token_config {
        TokenConfiguration::V0(v0) => v0.distribution_rules().pre_programmed_distribution(),
    }
}

fn pre_programmed_distribution_identity_matches(
    distribution: &TokenPreProgrammedDistribution,
    clause: &ValueClause,
) -> bool {
    distribution
        .distributions()
        .values()
        .flat_map(|entries| entries.keys())
        .any(|identity| clause.matches_value(&Value::from(identity.clone())))
}

fn groups_present_in_contract(
    contract_serialized: &DataContractInSerializationFormat,
    requirements: &[GroupRequirement],
) -> bool {
    if requirements.is_empty() {
        return true;
    }

    let groups = contract_serialized.groups();
    group_requirements_satisfied(groups, requirements)
}

fn group_requirements_satisfied(
    groups: &BTreeMap<GroupContractPosition, Group>,
    requirements: &[GroupRequirement],
) -> bool {
    requirements.iter().all(|requirement| match &requirement.position {
        Some(position) => groups
            .get(position)
            .map(|group| group_requirement_is_met(group, requirement))
            .unwrap_or(false),
        None => groups
            .values()
            .any(|group| group_requirement_is_met(group, requirement)),
    })
}

fn group_requirement_is_met(group: &Group, requirement: &GroupRequirement) -> bool {
    if let Some(identity_clause) = &requirement.member_identity_clause {
        if !group
            .members()
            .keys()
            .any(|identity| identity_clause.matches_value(&Value::from(identity.clone())))
        {
            return false;
        }
    }

    true
}

fn contract_tokens(
    serialized: &DataContractInSerializationFormat,
) -> &BTreeMap<TokenContractPosition, TokenConfiguration> {
    serialized.tokens()
}

fn contract_token_positions(contract: &DataContract) -> BTreeSet<TokenContractPosition> {
    match contract {
        DataContract::V0(_) => BTreeSet::new(),
        DataContract::V1(v1) => v1.tokens.keys().copied().collect(),
    }
}
