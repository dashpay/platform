use platform_value::Identifier;
use platform_version::version::PlatformVersion;
use crate::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
use crate::data_contract::associated_token::token_configuration::TokenConfiguration;
use crate::data_contract::associated_token::token_distribution_rules::accessors::v0::TokenDistributionRulesV0Getters;
use crate::ProtocolError;
use crate::state_transition::batch_transition::batched_transition::multi_party_action::AllowedAsMultiPartyAction;
use crate::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
use crate::state_transition::batch_transition::token_base_transition::v0::v0_methods::TokenBaseTransitionV0Methods;
use crate::state_transition::batch_transition::token_mint_transition::TokenMintTransitionV0;
use crate::state_transition::batch_transition::TokenMintTransition;
use crate::tokens::errors::TokenError;

impl TokenBaseTransitionAccessors for TokenMintTransitionV0 {
    fn base(&self) -> &TokenBaseTransition {
        &self.base
    }

    fn base_mut(&mut self) -> &mut TokenBaseTransition {
        &mut self.base
    }

    fn set_base(&mut self, base: TokenBaseTransition) {
        self.base = base;
    }
}

pub trait TokenMintTransitionV0Methods:
    TokenBaseTransitionAccessors + AllowedAsMultiPartyAction
{
    fn amount(&self) -> u64;

    fn set_amount(&mut self, amount: u64);

    /// Returns the `public_note` field of the `TokenMintTransitionV0`.
    fn public_note(&self) -> Option<&String>;

    /// Returns the owned `public_note` field of the `TokenMintTransitionV0`.
    fn public_note_owned(self) -> Option<String>;

    /// Sets the value of the `public_note` field in the `TokenMintTransitionV0`.
    fn set_public_note(&mut self, public_note: Option<String>);

    /// Returns the `issued_to_identity_id` field of the `TokenMintTransitionV0`.
    fn issued_to_identity_id(&self) -> Option<Identifier>;
    fn recipient_id(
        &self,
        token_configuration: &TokenConfiguration,
    ) -> Result<Identifier, ProtocolError>;

    /// Sets the value of the `issued_to_identity_id` field in the `TokenMintTransitionV0`.
    fn set_issued_to_identity_id(&mut self, issued_to_identity_id: Option<Identifier>);
}

impl TokenMintTransitionV0Methods for TokenMintTransitionV0 {
    fn amount(&self) -> u64 {
        self.amount
    }

    fn set_amount(&mut self, amount: u64) {
        self.amount = amount;
    }

    fn public_note(&self) -> Option<&String> {
        self.public_note.as_ref()
    }

    fn public_note_owned(self) -> Option<String> {
        self.public_note
    }

    fn set_public_note(&mut self, public_note: Option<String>) {
        self.public_note = public_note;
    }

    fn issued_to_identity_id(&self) -> Option<Identifier> {
        self.issued_to_identity_id
    }

    fn recipient_id(
        &self,
        token_configuration: &TokenConfiguration,
    ) -> Result<Identifier, ProtocolError> {
        match self.issued_to_identity_id() {
            None => token_configuration
                .distribution_rules()
                .new_tokens_destination_identity()
                .copied()
                .ok_or(ProtocolError::Token(
                    TokenError::TokenNoMintingRecipient.into(),
                )),
            Some(recipient) => Ok(recipient),
        }
    }

    fn set_issued_to_identity_id(&mut self, issued_to_identity_id: Option<Identifier>) {
        self.issued_to_identity_id = issued_to_identity_id;
    }
}

impl AllowedAsMultiPartyAction for TokenMintTransitionV0 {
    fn calculate_action_id(
        &self,
        owner_id: Identifier,
        _platform_version: &PlatformVersion,
    ) -> Result<Identifier, ProtocolError> {
        let TokenMintTransitionV0 { base, amount, .. } = self;

        Ok(TokenMintTransition::calculate_action_id_with_fields(
            base.token_id().as_bytes(),
            owner_id.as_bytes(),
            base.identity_contract_nonce(),
            *amount,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_contract::associated_token::token_configuration::v0::TokenConfigurationV0;
    use crate::data_contract::associated_token::token_configuration_convention::v0::TokenConfigurationConventionV0;
    use crate::data_contract::associated_token::token_configuration_convention::TokenConfigurationConvention;
    use crate::data_contract::associated_token::token_distribution_rules::v0::TokenDistributionRulesV0;
    use crate::data_contract::associated_token::token_distribution_rules::TokenDistributionRules;
    use crate::data_contract::associated_token::token_keeps_history_rules::v0::TokenKeepsHistoryRulesV0;
    use crate::data_contract::associated_token::token_keeps_history_rules::TokenKeepsHistoryRules;
    use crate::data_contract::associated_token::token_marketplace_rules::v0::{
        TokenMarketplaceRulesV0, TokenTradeMode,
    };
    use crate::data_contract::associated_token::token_marketplace_rules::TokenMarketplaceRules;
    use crate::data_contract::change_control_rules::authorized_action_takers::AuthorizedActionTakers;
    use crate::data_contract::change_control_rules::v0::ChangeControlRulesV0;
    use crate::data_contract::change_control_rules::ChangeControlRules;
    use crate::tokens::errors::TokenError;
    use std::collections::BTreeMap;

    fn no_one_rules() -> ChangeControlRules {
        ChangeControlRules::V0(ChangeControlRulesV0 {
            authorized_to_make_change: AuthorizedActionTakers::NoOne,
            admin_action_takers: AuthorizedActionTakers::NoOne,
            changing_authorized_action_takers_to_no_one_allowed: false,
            changing_admin_action_takers_to_no_one_allowed: false,
            self_changing_admin_action_takers_allowed: false,
        })
    }

    fn make_config(new_tokens_dest: Option<Identifier>) -> TokenConfiguration {
        TokenConfiguration::V0(TokenConfigurationV0 {
            conventions: TokenConfigurationConvention::V0(TokenConfigurationConventionV0 {
                localizations: BTreeMap::new(),
                decimals: 8,
            }),
            conventions_change_rules: no_one_rules(),
            base_supply: 1000,
            max_supply: None,
            keeps_history: TokenKeepsHistoryRules::V0(TokenKeepsHistoryRulesV0 {
                keeps_transfer_history: true,
                keeps_freezing_history: true,
                keeps_minting_history: true,
                keeps_burning_history: true,
                keeps_direct_pricing_history: true,
                keeps_direct_purchase_history: true,
            }),
            start_as_paused: false,
            allow_transfer_to_frozen_balance: true,
            max_supply_change_rules: no_one_rules(),
            distribution_rules: TokenDistributionRules::V0(TokenDistributionRulesV0 {
                perpetual_distribution: None,
                perpetual_distribution_rules: no_one_rules(),
                pre_programmed_distribution: None,
                new_tokens_destination_identity: new_tokens_dest,
                new_tokens_destination_identity_rules: no_one_rules(),
                minting_allow_choosing_destination: true,
                minting_allow_choosing_destination_rules: no_one_rules(),
                change_direct_purchase_pricing_rules: no_one_rules(),
            }),
            marketplace_rules: TokenMarketplaceRules::V0(TokenMarketplaceRulesV0 {
                trade_mode: TokenTradeMode::NotTradeable,
                trade_mode_change_rules: no_one_rules(),
            }),
            manual_minting_rules: no_one_rules(),
            manual_burning_rules: no_one_rules(),
            freeze_rules: no_one_rules(),
            unfreeze_rules: no_one_rules(),
            destroy_frozen_funds_rules: no_one_rules(),
            emergency_action_rules: no_one_rules(),
            main_control_group: None,
            main_control_group_can_be_modified: AuthorizedActionTakers::NoOne,
            description: None,
        })
    }

    // -----------------------------------------------------------------------
    // recipient_id — explicit recipient takes precedence
    // -----------------------------------------------------------------------

    #[test]
    fn recipient_id_returns_explicit_recipient_when_set() {
        let explicit_recipient = Identifier::from([20u8; 32]);
        let transition = TokenMintTransitionV0 {
            base: TokenBaseTransition::default(),
            issued_to_identity_id: Some(explicit_recipient),
            amount: 100,
            public_note: None,
        };

        let config_dest = Identifier::from([30u8; 32]);
        let config = make_config(Some(config_dest));

        let result = transition
            .recipient_id(&config)
            .expect("should return explicit recipient");
        assert_eq!(result, explicit_recipient);
    }

    // -----------------------------------------------------------------------
    // recipient_id — fallback to config destination
    // -----------------------------------------------------------------------

    #[test]
    fn recipient_id_falls_back_to_config_destination_identity() {
        let transition = TokenMintTransitionV0 {
            base: TokenBaseTransition::default(),
            issued_to_identity_id: None,
            amount: 100,
            public_note: None,
        };

        let config_dest = Identifier::from([30u8; 32]);
        let config = make_config(Some(config_dest));

        let result = transition
            .recipient_id(&config)
            .expect("should fall back to config dest");
        assert_eq!(result, config_dest);
    }

    // -----------------------------------------------------------------------
    // recipient_id — error TokenNoMintingRecipient when nothing available
    // -----------------------------------------------------------------------

    #[test]
    fn recipient_id_errors_with_token_no_minting_recipient() {
        let transition = TokenMintTransitionV0 {
            base: TokenBaseTransition::default(),
            issued_to_identity_id: None,
            amount: 100,
            public_note: None,
        };

        let config = make_config(None);

        let result = transition.recipient_id(&config);
        assert!(result.is_err());
        match result.unwrap_err() {
            ProtocolError::Token(boxed_err) => {
                let token_err: &TokenError = boxed_err.as_ref();
                assert!(
                    matches!(token_err, TokenError::TokenNoMintingRecipient),
                    "Expected TokenNoMintingRecipient, got {:?}",
                    token_err
                );
            }
            other => panic!("Expected ProtocolError::Token, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // recipient_id — explicit recipient overrides config even when both set
    // -----------------------------------------------------------------------

    #[test]
    fn recipient_id_prefers_explicit_over_config() {
        let explicit = Identifier::from([11u8; 32]);
        let config_dest = Identifier::from([22u8; 32]);

        let transition = TokenMintTransitionV0 {
            base: TokenBaseTransition::default(),
            issued_to_identity_id: Some(explicit),
            amount: 500,
            public_note: None,
        };

        let config = make_config(Some(config_dest));

        let result = transition
            .recipient_id(&config)
            .expect("explicit should win");
        assert_eq!(result, explicit);
        assert_ne!(result, config_dest);
    }
}
