//! Token subscription filtering
//!
//! This module mirrors the document filtering support but targets token transitions. It
//! enables subscriptions to express which token contract, token identifier, and transition
//! variant they are interested in, along with a handful of scalar constraints (amounts,
//! recipients, etc.).

use crate::drive::subscriptions::TransitionCheckResult;
use crate::error::query::QuerySyntaxError;
use crate::query::{QuerySyntaxSimpleValidationResult, ValueClause};
use crate::util::object_size_info::DataContractOwnedResolvedInfo;
use dpp::data_contract::accessors::v1::DataContractV1Getters;
use dpp::data_contract::associated_token::token_distribution_key::TokenDistributionType;
use dpp::data_contract::{DataContract, TokenContractPosition};
use dpp::identifier::Identifier;
use dpp::platform_value::Value;
use dpp::state_transition::batch_transition::batched_transition::token_base_transition::v0::v0_methods::TokenBaseTransitionV0Methods;
use dpp::state_transition::batch_transition::batched_transition::token_burn_transition::v0::v0_methods::TokenBurnTransitionV0Methods;
use dpp::state_transition::batch_transition::batched_transition::token_claim_transition::v0::v0_methods::TokenClaimTransitionV0Methods;
use dpp::state_transition::batch_transition::batched_transition::token_destroy_frozen_funds_transition::v0::v0_methods::TokenDestroyFrozenFundsTransitionV0Methods;
use dpp::state_transition::batch_transition::batched_transition::token_direct_purchase_transition::v0::v0_methods::TokenDirectPurchaseTransitionV0Methods;
use dpp::state_transition::batch_transition::batched_transition::token_emergency_action_transition::v0::v0_methods::TokenEmergencyActionTransitionV0Methods;
use dpp::state_transition::batch_transition::batched_transition::token_freeze_transition::v0::v0_methods::TokenFreezeTransitionV0Methods;
use dpp::state_transition::batch_transition::batched_transition::token_mint_transition::v0::v0_methods::TokenMintTransitionV0Methods;
use dpp::state_transition::batch_transition::batched_transition::token_set_price_for_direct_purchase_transition::v0::v0_methods::TokenSetPriceForDirectPurchaseTransitionV0Methods;
use dpp::state_transition::batch_transition::batched_transition::token_transfer_transition::v0::v0_methods::TokenTransferTransitionV0Methods;
use dpp::state_transition::batch_transition::batched_transition::token_transition::{
    TokenTransition, TokenTransitionV0Methods,
};
use dpp::state_transition::batch_transition::batched_transition::token_unfreeze_transition::v0::v0_methods::TokenUnfreezeTransitionV0Methods;
use dpp::tokens::emergency_action::TokenEmergencyAction;


/// Filter describing constraints for token transitions.
#[derive(Debug, Clone, PartialEq)]
pub struct DriveTokenQueryFilter {
    /// Data contract that defines the token set.
    pub contract: DataContractOwnedResolvedInfo,
    /// Token position within the contract.
    pub token_contract_position: TokenContractPosition,
    /// Optional explicit token identifier. If present it is validated against the contract.
    pub token_id: Option<Identifier>,
    /// Action-specific clauses.
    pub action_clauses: TokenActionMatchClauses,
}

/// Clauses describing which token transition variants are acceptable and any
/// additional scalar predicates that must hold for a match.
/// Action-specific clauses for token subscriptions.
#[derive(Debug, Clone, PartialEq)]
pub enum TokenActionMatchClauses {
    /// Mint transition constraints.
    Mint {
        /// Optional clause applied to the minted amount.
        amount_clause: Option<ValueClause>,
        /// Optional clause applied to the issued-to identity (if supplied).
        recipient_clause: Option<ValueClause>,
    },
    /// Burn transition constraints.
    Burn {
        /// Optional clause applied to the burn amount.
        amount_clause: Option<ValueClause>,
    },
    /// Transfer transition constraints.
    Transfer {
        /// Optional clause applied to the transfer amount.
        amount_clause: Option<ValueClause>,
        /// Optional clause applied to the recipient identity.
        recipient_clause: Option<ValueClause>,
    },
    /// Freeze transition constraints.
    Freeze {
        /// Optional clause applied to the frozen identity id.
        identity_clause: Option<ValueClause>,
    },
    /// Unfreeze transition constraints.
    Unfreeze {
        /// Optional clause applied to the previously frozen identity id.
        identity_clause: Option<ValueClause>,
    },
    /// Destroy frozen funds transition constraints.
    DestroyFrozenFunds {
        /// Optional clause applied to the frozen identity id.
        identity_clause: Option<ValueClause>,
    },
    /// Claim transition constraints.
    Claim {
        /// Optional list of allowed distribution types.
        distribution_types: Option<Vec<TokenDistributionType>>,
    },
    /// Emergency action transition constraints.
    EmergencyAction {
        /// Optional list of allowed emergency actions.
        allowed_actions: Option<Vec<TokenEmergencyAction>>,
    },
    /// Config update transition (no additional predicates).
    ConfigUpdate,
    /// Direct purchase transition constraints.
    DirectPurchase {
        /// Optional clause applied to the requested token count.
        token_count_clause: Option<ValueClause>,
        /// Optional clause applied to the total agreed price.
        total_price_clause: Option<ValueClause>,
    },
    /// Set price for direct purchase transition constraints.
    SetPriceForDirectPurchase {
        /// Require the transition to set (`true`) or clear (`false`) the price schedule.
        require_price: Option<bool>,
    },
}


impl DriveTokenQueryFilter {
    /// Evaluate a token transition against the filter.
    pub fn matches_token_transition(&self, transition: &TokenTransition) -> TransitionCheckResult {
        if transition.data_contract_id() != self.contract.id() {
            return TransitionCheckResult::Fail;
        }

        if transition.base().token_contract_position() != self.token_contract_position {
            return TransitionCheckResult::Fail;
        }

        if let Some(expected_token_id) = &self.token_id {
            if transition.token_id() != *expected_token_id {
                return TransitionCheckResult::Fail;
            }
        }

        if action_matches(&self.action_clauses, transition) {
            TransitionCheckResult::Pass
        } else {
            TransitionCheckResult::Fail
        }
    }

    /// Validate that the filter references a real token and that the attached clauses
    /// use supported shapes for their scalar comparisons.
    pub fn validate(&self) -> QuerySyntaxSimpleValidationResult {
        let contract = self.contract.as_ref();

        let DataContract::V1(contract_v1) = contract else {
            return QuerySyntaxSimpleValidationResult::new_with_error(
                QuerySyntaxError::InvalidWhereClauseComponents(
                    "token filters require a v1 data contract",
                ),
            );
        };

        if !contract_v1
            .tokens()
            .contains_key(&self.token_contract_position)
        {
            return QuerySyntaxSimpleValidationResult::new_with_error(
                QuerySyntaxError::InvalidWhereClauseComponents("unknown token contract position"),
            );
        }

        if let Some(expected) = contract_v1.token_id(self.token_contract_position) {
            if let Some(token_id) = &self.token_id {
                if token_id != &expected {
                    return QuerySyntaxSimpleValidationResult::new_with_error(
                        QuerySyntaxError::InvalidWhereClauseComponents(
                            "token id does not match contract-derived identifier",
                        ),
                    );
                }
            }
        }

        if let Err(error) = validate_action_clauses(&self.action_clauses) {
            return QuerySyntaxSimpleValidationResult::new_with_error(error);
        }

        QuerySyntaxSimpleValidationResult::new()
    }
}

fn action_matches(action: &TokenActionMatchClauses, transition: &TokenTransition) -> bool {
    match (action, transition) {
        (
            TokenActionMatchClauses::Mint {
                amount_clause,
                recipient_clause,
            },
            TokenTransition::Mint(mint),
        ) => {
            if !integer_clause_matches(amount_clause, mint.amount()) {
                return false;
            }

            if let Some(clause) = recipient_clause {
                match mint.issued_to_identity_id() {
                    Some(recipient) => {
                        if !clause.matches_value(&recipient.into()) {
                            return false;
                        }
                    }
                    None => return false,
                }
            }

            true
        }
        (TokenActionMatchClauses::Burn { amount_clause }, TokenTransition::Burn(burn)) => {
            integer_clause_matches(amount_clause, burn.burn_amount())
        }
        (
            TokenActionMatchClauses::Transfer {
                amount_clause,
                recipient_clause,
            },
            TokenTransition::Transfer(transfer),
        ) => {
            if !integer_clause_matches(amount_clause, transfer.amount()) {
                return false;
            }

            if let Some(clause) = recipient_clause {
                clause.matches_value(&transfer.recipient_id().into())
            } else {
                true
            }
        }
        (TokenActionMatchClauses::Freeze { identity_clause }, TokenTransition::Freeze(freeze)) => {
            match identity_clause {
                Some(clause) => clause.matches_value(&freeze.frozen_identity_id().into()),
                None => true,
            }
        }
        (
            TokenActionMatchClauses::Unfreeze { identity_clause },
            TokenTransition::Unfreeze(unfreeze),
        ) => match identity_clause {
            Some(clause) => clause.matches_value(&unfreeze.frozen_identity_id().into()),
            None => true,
        },
        (
            TokenActionMatchClauses::DestroyFrozenFunds { identity_clause },
            TokenTransition::DestroyFrozenFunds(destroy),
        ) => match identity_clause {
            Some(clause) => clause.matches_value(&destroy.frozen_identity_id().into()),
            None => true,
        },
        (TokenActionMatchClauses::Claim { distribution_types }, TokenTransition::Claim(claim)) => {
            distribution_types.as_ref().map_or(true, |allowed| {
                allowed
                    .iter()
                    .any(|distribution| distribution == &claim.distribution_type())
            })
        }
        (
            TokenActionMatchClauses::EmergencyAction { allowed_actions },
            TokenTransition::EmergencyAction(action),
        ) => allowed_actions.as_ref().map_or(true, |allowed| {
            allowed
                .iter()
                .any(|candidate| candidate == &action.emergency_action())
        }),
        (TokenActionMatchClauses::ConfigUpdate, TokenTransition::ConfigUpdate(_)) => true,
        (
            TokenActionMatchClauses::DirectPurchase {
                token_count_clause,
                total_price_clause,
            },
            TokenTransition::DirectPurchase(purchase),
        ) => {
            integer_clause_matches(token_count_clause, purchase.token_count())
                && integer_clause_matches(total_price_clause, purchase.total_agreed_price())
        }
        (
            TokenActionMatchClauses::SetPriceForDirectPurchase { require_price },
            TokenTransition::SetPriceForDirectPurchase(set_price),
        ) => require_price.map_or(true, |require| set_price.price().is_some() == require),
        _ => false,
    }
}

fn integer_clause_matches(clause: &Option<ValueClause>, candidate: impl Into<u64>) -> bool {
    clause
        .as_ref()
        .map_or(true, |c| c.matches_value(&Value::U64(candidate.into())))
}

fn validate_action_clauses(action: &TokenActionMatchClauses) -> Result<(), QuerySyntaxError> {
    match action {
        TokenActionMatchClauses::Mint {
            amount_clause,
            recipient_clause,
        } => {
            ensure_integer_clause(amount_clause, "mint amount clause expects an integer value")?;
            ensure_identifier_clause(
                recipient_clause,
                "mint recipient clause expects an identifier value",
            )?;
        }
        TokenActionMatchClauses::Burn { amount_clause } => {
            ensure_integer_clause(amount_clause, "burn amount clause expects an integer value")?;
        }
        TokenActionMatchClauses::Transfer {
            amount_clause,
            recipient_clause,
        } => {
            ensure_integer_clause(
                amount_clause,
                "transfer amount clause expects an integer value",
            )?;
            ensure_identifier_clause(
                recipient_clause,
                "transfer recipient clause expects an identifier value",
            )?;
        }
        TokenActionMatchClauses::Freeze { identity_clause }
        | TokenActionMatchClauses::Unfreeze { identity_clause }
        | TokenActionMatchClauses::DestroyFrozenFunds { identity_clause } => {
            ensure_identifier_clause(
                identity_clause,
                "identity clause expects an identifier value",
            )?;
        }
        TokenActionMatchClauses::Claim { distribution_types } => {
            if matches!(distribution_types, Some(set) if set.is_empty()) {
                return Err(QuerySyntaxError::InvalidWhereClauseComponents(
                    "distribution types set must not be empty",
                ));
            }
        }
        TokenActionMatchClauses::EmergencyAction { allowed_actions } => {
            if matches!(allowed_actions, Some(set) if set.is_empty()) {
                return Err(QuerySyntaxError::InvalidWhereClauseComponents(
                    "emergency action set must not be empty",
                ));
            }
        }
        TokenActionMatchClauses::DirectPurchase {
            token_count_clause,
            total_price_clause,
        } => {
            ensure_integer_clause(
                token_count_clause,
                "token count clause expects an integer value",
            )?;
            ensure_integer_clause(
                total_price_clause,
                "total price clause expects an integer value",
            )?;
        }
        TokenActionMatchClauses::SetPriceForDirectPurchase { .. } => {}
        TokenActionMatchClauses::ConfigUpdate => {}
    }

    Ok(())
}

fn ensure_integer_clause(
    clause: &Option<ValueClause>,
    error_message: &'static str,
) -> Result<(), QuerySyntaxError> {
    if let Some(clause) = clause {
        if !clause.value.is_integer_can_fit_in_64_bits() {
            return Err(QuerySyntaxError::InvalidWhereClauseComponents(
                error_message,
            ));
        }
    }
    Ok(())
}

fn ensure_identifier_clause(
    clause: &Option<ValueClause>,
    error_message: &'static str,
) -> Result<(), QuerySyntaxError> {
    if let Some(clause) = clause {
        if !matches!(clause.value, Value::Identifier(_)) {
            return Err(QuerySyntaxError::InvalidWhereClauseComponents(
                error_message,
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::{ValueClause, WhereOperator};
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::associated_token::token_configuration::{
        TokenConfiguration,
    };
    use dpp::data_contract::config::v0::DataContractConfigV0;
    use dpp::data_contract::group::Group;
    use dpp::data_contract::v1::data_contract::DataContractV1;
    use dpp::data_contract::{DataContract};
    use dpp::identifier::Identifier;
    use dpp::tokens::calculate_token_id;
    use dpp::tokens::emergency_action::TokenEmergencyAction;
    use dpp::tokens::token_pricing_schedule::TokenPricingSchedule;
    use dpp::state_transition::batch_transition::batched_transition::token_base_transition::TokenBaseTransition;
    use dpp::state_transition::batch_transition::batched_transition::token_base_transition::v0::TokenBaseTransitionV0;
    use dpp::state_transition::batch_transition::batched_transition::token_emergency_action_transition::{
        TokenEmergencyActionTransition, TokenEmergencyActionTransitionV0,
    };
    use dpp::state_transition::batch_transition::batched_transition::token_direct_purchase_transition::TokenDirectPurchaseTransition;
    use dpp::state_transition::batch_transition::batched_transition::token_direct_purchase_transition::v0::TokenDirectPurchaseTransitionV0;
    use dpp::state_transition::batch_transition::batched_transition::token_mint_transition::TokenMintTransition;
    use dpp::state_transition::batch_transition::batched_transition::token_mint_transition::v0::TokenMintTransitionV0;
    use dpp::state_transition::batch_transition::batched_transition::token_set_price_for_direct_purchase_transition::TokenSetPriceForDirectPurchaseTransition;
    use dpp::state_transition::batch_transition::batched_transition::token_set_price_for_direct_purchase_transition::v0::TokenSetPriceForDirectPurchaseTransitionV0;
    use dpp::state_transition::batch_transition::batched_transition::token_transfer_transition::TokenTransferTransition;
    use dpp::state_transition::batch_transition::batched_transition::token_transfer_transition::v0::TokenTransferTransitionV0;
    use dpp::state_transition::batch_transition::batched_transition::token_transition::TokenTransition;
    use std::collections::BTreeMap;
    use dpp::data_contract::associated_token::token_configuration::v0::TokenConfigurationV0;
    use dpp::data_contract::config::DataContractConfig;
    use dpp::data_contract::document_type::DocumentType;

    fn build_token_contract() -> DataContract {
        let id = Identifier::from([1u8; 32]);
        let owner_id = Identifier::from([2u8; 32]);
        let config = DataContractConfig::V0(DataContractConfigV0::default());
        let document_types: BTreeMap<String, DocumentType> = BTreeMap::new();
        let groups: BTreeMap<_, Group> = BTreeMap::new();
        let mut tokens = BTreeMap::new();
        tokens.insert(
            0u16,
            TokenConfiguration::V0(TokenConfigurationV0::default_most_restrictive()),
        );

        DataContract::V1(DataContractV1 {
            id,
            version: 1,
            owner_id,
            document_types,
            config,
            schema_defs: None,
            created_at: None,
            updated_at: None,
            created_at_block_height: None,
            updated_at_block_height: None,
            created_at_epoch: None,
            updated_at_epoch: None,
            groups,
            tokens,
            keywords: vec![],
            description: None,
        })
    }

    fn token_id_for(contract: &DataContract, position: TokenContractPosition) -> Identifier {
        match contract {
            DataContract::V1(v1) => v1
                .token_id(position)
                .expect("token id available for position"),
            _ => panic!("expected v1 contract"),
        }
    }

    fn build_base(contract: &DataContract, position: TokenContractPosition) -> TokenBaseTransition {
        let id = contract.id();
        let token_id = Identifier::from(calculate_token_id(id.as_bytes(), position));

        TokenBaseTransition::V0(TokenBaseTransitionV0 {
            identity_contract_nonce: 0,
            token_contract_position: position,
            data_contract_id: id,
            token_id,
            using_group_info: None,
        })
    }

    #[test]
    fn validate_rejects_unknown_position() {
        let contract = build_token_contract();
        let filter = DriveTokenQueryFilter {
            contract: DataContractOwnedResolvedInfo::OwnedDataContract(contract),
            token_contract_position: 5,
            token_id: None,
            action_clauses: TokenActionMatchClauses::ConfigUpdate,
        };

        assert!(filter.validate().is_err());
    }

    #[test]
    fn mint_matches_with_amount_clause() {
        let contract = build_token_contract();
        let position = 0u16;
        let expected_token_id = token_id_for(&contract, position);

        let filter = DriveTokenQueryFilter {
            contract: DataContractOwnedResolvedInfo::OwnedDataContract(contract.clone()),
            token_contract_position: position,
            token_id: Some(expected_token_id),
            action_clauses: TokenActionMatchClauses::Mint {
                amount_clause: Some(ValueClause {
                    operator: WhereOperator::GreaterThan,
                    value: Value::U64(5),
                }),
                recipient_clause: Some(ValueClause {
                    operator: WhereOperator::Equal,
                    value: Identifier::from([9u8; 32]).into(),
                }),
            },
        };

        assert!(filter.validate().is_valid());

        let mut base = build_base(&contract, position);
        if let TokenBaseTransition::V0(base_v0) = &mut base {
            base_v0.identity_contract_nonce = 42;
        }

        let token_transition =
            TokenTransition::Mint(TokenMintTransition::V0(TokenMintTransitionV0 {
                base,
                issued_to_identity_id: Some(Identifier::from([9u8; 32])),
                amount: 10,
                public_note: None,
            }));

        assert_eq!(
            filter.matches_token_transition(&token_transition),
            TransitionCheckResult::Pass
        );

        let failing_transition =
            TokenTransition::Mint(TokenMintTransition::V0(TokenMintTransitionV0 {
                base: build_base(&contract, position),
                issued_to_identity_id: Some(Identifier::from([8u8; 32])),
                amount: 3,
                public_note: None,
            }));

        assert_eq!(
            filter.matches_token_transition(&failing_transition),
            TransitionCheckResult::Fail
        );
    }

    #[test]
    fn direct_purchase_filters_on_amount() {
        let contract = build_token_contract();
        let position = 0u16;

        let filter = DriveTokenQueryFilter {
            contract: DataContractOwnedResolvedInfo::OwnedDataContract(contract.clone()),
            token_contract_position: position,
            token_id: None,
            action_clauses: TokenActionMatchClauses::DirectPurchase {
                token_count_clause: Some(ValueClause {
                    operator: WhereOperator::Equal,
                    value: Value::U64(20),
                }),
                total_price_clause: Some(ValueClause {
                    operator: WhereOperator::LessThanOrEquals,
                    value: Value::U64(1_000),
                }),
            },
        };

        assert!(filter.validate().is_valid());

        let transition = TokenTransition::DirectPurchase(TokenDirectPurchaseTransition::V0(
            TokenDirectPurchaseTransitionV0 {
                base: build_base(&contract, position),
                token_count: 20,
                total_agreed_price: 800,
            },
        ));

        assert_eq!(
            filter.matches_token_transition(&transition),
            TransitionCheckResult::Pass
        );

        let mismatch = TokenTransition::DirectPurchase(TokenDirectPurchaseTransition::V0(
            TokenDirectPurchaseTransitionV0 {
                base: build_base(&contract, position),
                token_count: 5,
                total_agreed_price: 200,
            },
        ));

        assert_eq!(
            filter.matches_token_transition(&mismatch),
            TransitionCheckResult::Fail
        );
    }

    #[test]
    fn set_price_respects_requirement() {
        let contract = build_token_contract();
        let position = 0u16;

        let filter = DriveTokenQueryFilter {
            contract: DataContractOwnedResolvedInfo::OwnedDataContract(contract.clone()),
            token_contract_position: position,
            token_id: None,
            action_clauses: TokenActionMatchClauses::SetPriceForDirectPurchase {
                require_price: Some(true),
            },
        };

        assert!(filter.validate().is_valid());

        let schedule = TokenPricingSchedule::SinglePrice(100);
        let matching = TokenTransition::SetPriceForDirectPurchase(
            TokenSetPriceForDirectPurchaseTransition::V0(
                TokenSetPriceForDirectPurchaseTransitionV0 {
                    base: build_base(&contract, position),
                    price: Some(schedule.clone()),
                    public_note: None,
                },
            ),
        );

        assert_eq!(
            filter.matches_token_transition(&matching),
            TransitionCheckResult::Pass
        );

        let missing_price = TokenTransition::SetPriceForDirectPurchase(
            TokenSetPriceForDirectPurchaseTransition::V0(
                TokenSetPriceForDirectPurchaseTransitionV0 {
                    base: build_base(&contract, position),
                    price: None,
                    public_note: None,
                },
            ),
        );

        assert_eq!(
            filter.matches_token_transition(&missing_price),
            TransitionCheckResult::Fail
        );
    }

    #[test]
    fn emergency_action_allows_selected_types() {
        let contract = build_token_contract();
        let position = 0u16;

        let allowed = vec![TokenEmergencyAction::Pause];

        let filter = DriveTokenQueryFilter {
            contract: DataContractOwnedResolvedInfo::OwnedDataContract(contract.clone()),
            token_contract_position: position,
            token_id: None,
            action_clauses: TokenActionMatchClauses::EmergencyAction {
                allowed_actions: Some(allowed.clone()),
            },
        };

        assert!(filter.validate().is_valid());

        let pause = TokenTransition::EmergencyAction(TokenEmergencyActionTransition::V0(
            TokenEmergencyActionTransitionV0 {
                base: build_base(&contract, position),
                emergency_action: TokenEmergencyAction::Pause,
                public_note: None,
            },
        ));

        assert_eq!(
            filter.matches_token_transition(&pause),
            TransitionCheckResult::Pass
        );

        let resume = TokenTransition::EmergencyAction(TokenEmergencyActionTransition::V0(
            TokenEmergencyActionTransitionV0 {
                base: build_base(&contract, position),
                emergency_action: TokenEmergencyAction::Resume,
                public_note: None,
            },
        ));

        assert_eq!(
            filter.matches_token_transition(&resume),
            TransitionCheckResult::Fail
        );
    }

    #[test]
    fn transfer_requires_matching_recipient() {
        let contract = build_token_contract();
        let position = 0u16;
        let recipient = Identifier::from([7u8; 32]);

        let filter = DriveTokenQueryFilter {
            contract: DataContractOwnedResolvedInfo::OwnedDataContract(contract.clone()),
            token_contract_position: position,
            token_id: None,
            action_clauses: TokenActionMatchClauses::Transfer {
                amount_clause: Some(ValueClause {
                    operator: WhereOperator::Equal,
                    value: Value::U64(15),
                }),
                recipient_clause: Some(ValueClause {
                    operator: WhereOperator::Equal,
                    value: recipient.into(),
                }),
            },
        };

        assert!(filter.validate().is_valid());

        let passing =
            TokenTransition::Transfer(TokenTransferTransition::V0(TokenTransferTransitionV0 {
                base: build_base(&contract, position),
                amount: 15,
                recipient_id: recipient,
                ..Default::default()
            }));

        assert_eq!(
            filter.matches_token_transition(&passing),
            TransitionCheckResult::Pass
        );

        let failing =
            TokenTransition::Transfer(TokenTransferTransition::V0(TokenTransferTransitionV0 {
                base: build_base(&contract, position),
                amount: 15,
                recipient_id: Identifier::from([8u8; 32]),
                ..Default::default()
            }));

        assert_eq!(
            filter.matches_token_transition(&failing),
            TransitionCheckResult::Fail
        );
    }
}
