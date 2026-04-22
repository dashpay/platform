mod transformer;

use std::sync::Arc;
use dpp::identifier::Identifier;
use crate::drive::contract::DataContractFetchInfo;
use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::{TokenBaseTransitionAction, TokenBaseTransitionActionAccessorsV0};

/// Token issuance transition action v0
#[derive(Debug, Clone)]
pub struct TokenMintTransitionActionV0 {
    /// Base token transition action
    pub base: TokenBaseTransitionAction,
    /// The amount of tokens to create
    pub mint_amount: u64,
    /// The identity to credit the token to
    pub identity_balance_holder_id: Identifier,
    /// A public note
    pub public_note: Option<String>,
}

/// Accessors for `TokenIssuanceTransitionActionV0`
pub trait TokenMintTransitionActionAccessorsV0 {
    /// Returns a reference to the base token transition action
    fn base(&self) -> &TokenBaseTransitionAction;

    /// Consumes self and returns the base token transition action
    fn base_owned(self) -> TokenBaseTransitionAction;

    /// Returns the amount of tokens to issuance
    fn mint_amount(&self) -> u64;

    /// Sets the amount of tokens to issuance
    fn set_mint_amount(&mut self, amount: u64);

    /// Consumes self and returns the identity balance holder ID
    fn identity_balance_holder_id(&self) -> Identifier;

    /// Sets the identity balance holder ID
    fn set_identity_balance_holder_id(&mut self, id: Identifier);

    /// Returns the token position in the contract
    fn token_position(&self) -> u16 {
        self.base().token_position()
    }

    /// Returns the token ID
    fn token_id(&self) -> Identifier {
        self.base().token_id()
    }

    /// Returns the data contract ID
    fn data_contract_id(&self) -> Identifier {
        self.base().data_contract_id()
    }

    /// Returns a reference to the data contract fetch info
    fn data_contract_fetch_info_ref(&self) -> &Arc<DataContractFetchInfo> {
        self.base().data_contract_fetch_info_ref()
    }

    /// Returns the data contract fetch info
    fn data_contract_fetch_info(&self) -> Arc<DataContractFetchInfo> {
        self.base().data_contract_fetch_info()
    }

    /// Returns the public note (optional)
    fn public_note(&self) -> Option<&String>;

    /// Returns the public note (owned)
    fn public_note_owned(self) -> Option<String>;

    /// Sets the public note
    fn set_public_note(&mut self, public_note: Option<String>);
}

impl TokenMintTransitionActionAccessorsV0 for TokenMintTransitionActionV0 {
    fn base(&self) -> &TokenBaseTransitionAction {
        &self.base
    }

    fn base_owned(self) -> TokenBaseTransitionAction {
        self.base
    }

    fn mint_amount(&self) -> u64 {
        self.mint_amount
    }

    fn set_mint_amount(&mut self, amount: u64) {
        self.mint_amount = amount;
    }

    fn identity_balance_holder_id(&self) -> Identifier {
        self.identity_balance_holder_id
    }

    fn set_identity_balance_holder_id(&mut self, id: Identifier) {
        self.identity_balance_holder_id = id;
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drive::contract::DataContractFetchInfo;
    use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionActionV0;
    use dpp::data_contract::associated_token::token_configuration::v0::TokenConfigurationV0;
    use dpp::data_contract::associated_token::token_configuration::TokenConfiguration;
    use dpp::data_contract::config::v0::DataContractConfigV0;
    use dpp::data_contract::config::DataContractConfig;
    use dpp::data_contract::v1::DataContractV1;
    use dpp::prelude::DataContract;
    use grovedb_costs::OperationCost;
    use std::collections::BTreeMap;

    fn build_contract() -> DataContract {
        let mut tokens = BTreeMap::new();
        tokens.insert(
            0u16,
            TokenConfiguration::V0(TokenConfigurationV0::default_most_restrictive()),
        );
        DataContract::V1(DataContractV1 {
            id: Identifier::new([12u8; 32]),
            version: 1,
            owner_id: Identifier::new([13u8; 32]),
            document_types: Default::default(),
            config: DataContractConfig::V0(DataContractConfigV0 {
                can_be_deleted: false,
                readonly: false,
                keeps_history: false,
                documents_keep_history_contract_default: false,
                documents_mutable_contract_default: false,
                documents_can_be_deleted_contract_default: false,
                requires_identity_encryption_bounded_key: None,
                requires_identity_decryption_bounded_key: None,
            }),
            schema_defs: None,
            created_at: None,
            updated_at: None,
            created_at_block_height: None,
            updated_at_block_height: None,
            created_at_epoch: None,
            updated_at_epoch: None,
            groups: Default::default(),
            tokens,
            keywords: Vec::new(),
            description: None,
        })
    }

    fn fetch_info(contract: DataContract) -> Arc<DataContractFetchInfo> {
        Arc::new(DataContractFetchInfo {
            contract,
            storage_flags: None,
            cost: OperationCost::default(),
            fee: None,
        })
    }

    fn build_mint_v0(position: u16, mint_amount: u64) -> TokenMintTransitionActionV0 {
        use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionAction;
        let base_v0 = TokenBaseTransitionActionV0 {
            token_id: Identifier::new([50u8; 32]),
            identity_contract_nonce: 17,
            token_contract_position: position,
            data_contract: fetch_info(build_contract()),
            store_in_group: None,
            perform_action: true,
        };
        TokenMintTransitionActionV0 {
            base: TokenBaseTransitionAction::V0(base_v0),
            mint_amount,
            identity_balance_holder_id: Identifier::new([51u8; 32]),
            public_note: Some("mint note".to_string()),
        }
    }

    #[test]
    fn getters_return_struct_fields_and_delegate_to_base() {
        let action = build_mint_v0(0, 1_000);

        assert_eq!(action.mint_amount(), 1_000);
        assert_eq!(
            action.identity_balance_holder_id(),
            Identifier::new([51u8; 32])
        );
        assert_eq!(action.public_note(), Some(&"mint note".to_string()));

        // Default-impl delegations into base
        assert_eq!(action.token_position(), 0);
        assert_eq!(action.token_id(), Identifier::new([50u8; 32]));
        assert_eq!(action.data_contract_id(), action.base().data_contract_id());
    }

    #[test]
    fn data_contract_fetch_info_variants_share_pointer_through_base() {
        let action = build_mint_v0(0, 1);
        let reference = action.data_contract_fetch_info_ref();
        let cloned = action.data_contract_fetch_info();
        assert!(Arc::ptr_eq(reference, &cloned));
    }

    #[test]
    fn setters_mutate_the_corresponding_fields() {
        let mut action = build_mint_v0(0, 1);

        action.set_mint_amount(9_999);
        assert_eq!(action.mint_amount(), 9_999);

        let new_holder = Identifier::new([91u8; 32]);
        action.set_identity_balance_holder_id(new_holder);
        assert_eq!(action.identity_balance_holder_id(), new_holder);

        action.set_public_note(None);
        assert!(action.public_note().is_none());

        action.set_public_note(Some("updated".to_string()));
        assert_eq!(action.public_note(), Some(&"updated".to_string()));
    }

    #[test]
    fn public_note_owned_moves_the_string_out() {
        let action = build_mint_v0(0, 1);
        let owned = action.public_note_owned();
        assert_eq!(owned, Some("mint note".to_string()));

        let action_none = TokenMintTransitionActionV0 {
            base: build_mint_v0(0, 1).base,
            mint_amount: 0,
            identity_balance_holder_id: Identifier::new([0u8; 32]),
            public_note: None,
        };
        assert!(action_none.public_note_owned().is_none());
    }

    #[test]
    fn base_owned_consumes_self_and_returns_base() {
        let action = build_mint_v0(0, 7);
        let expected_position = action.base().token_position();
        let expected_token_id = action.base().token_id();

        let base = action.base_owned();
        assert_eq!(base.token_position(), expected_position);
        assert_eq!(base.token_id(), expected_token_id);
    }
}
