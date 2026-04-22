use derive_more::From;
use dpp::identifier::Identifier;

/// transformer module for token issuance transition action
pub mod transformer;
mod v0;

pub use v0::*; // re-export the v0 module items (including TokenIssuanceTransitionActionV0)

use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionAction;

/// Token issuance transition action
#[derive(Debug, Clone, From)]
pub enum TokenMintTransitionAction {
    /// v0
    V0(TokenMintTransitionActionV0),
}

impl TokenMintTransitionActionAccessorsV0 for TokenMintTransitionAction {
    fn base(&self) -> &TokenBaseTransitionAction {
        match self {
            TokenMintTransitionAction::V0(v0) => &v0.base,
        }
    }

    fn base_owned(self) -> TokenBaseTransitionAction {
        match self {
            TokenMintTransitionAction::V0(v0) => v0.base,
        }
    }

    fn mint_amount(&self) -> u64 {
        match self {
            TokenMintTransitionAction::V0(v0) => v0.mint_amount,
        }
    }

    fn set_mint_amount(&mut self, amount: u64) {
        match self {
            TokenMintTransitionAction::V0(v0) => v0.mint_amount = amount,
        }
    }

    fn identity_balance_holder_id(&self) -> Identifier {
        match self {
            TokenMintTransitionAction::V0(v0) => v0.identity_balance_holder_id,
        }
    }

    fn set_identity_balance_holder_id(&mut self, id: Identifier) {
        match self {
            TokenMintTransitionAction::V0(v0) => v0.identity_balance_holder_id = id,
        }
    }

    fn public_note(&self) -> Option<&String> {
        match self {
            TokenMintTransitionAction::V0(v0) => v0.public_note.as_ref(),
        }
    }

    fn public_note_owned(self) -> Option<String> {
        match self {
            TokenMintTransitionAction::V0(v0) => v0.public_note,
        }
    }

    fn set_public_note(&mut self, public_note: Option<String>) {
        match self {
            TokenMintTransitionAction::V0(v0) => v0.public_note = public_note,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drive::contract::DataContractFetchInfo;
    use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::{
        TokenBaseTransitionAction, TokenBaseTransitionActionV0,
    };
    use dpp::data_contract::associated_token::token_configuration::v0::TokenConfigurationV0;
    use dpp::data_contract::associated_token::token_configuration::TokenConfiguration;
    use dpp::data_contract::config::v0::DataContractConfigV0;
    use dpp::data_contract::config::DataContractConfig;
    use dpp::data_contract::v1::DataContractV1;
    use dpp::prelude::DataContract;
    use grovedb_costs::OperationCost;
    use std::collections::BTreeMap;
    use std::sync::Arc;

    fn build_contract() -> DataContract {
        let mut tokens = BTreeMap::new();
        tokens.insert(
            0u16,
            TokenConfiguration::V0(TokenConfigurationV0::default_most_restrictive()),
        );
        DataContract::V1(DataContractV1 {
            id: Identifier::new([15u8; 32]),
            version: 1,
            owner_id: Identifier::new([16u8; 32]),
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

    fn build_mint_enum(
        token_id: Identifier,
        holder: Identifier,
        mint_amount: u64,
        public_note: Option<String>,
    ) -> TokenMintTransitionAction {
        let base_v0 = TokenBaseTransitionActionV0 {
            token_id,
            identity_contract_nonce: 5,
            token_contract_position: 0,
            data_contract: Arc::new(DataContractFetchInfo {
                contract: build_contract(),
                storage_flags: None,
                cost: OperationCost::default(),
                fee: None,
            }),
            store_in_group: None,
            perform_action: true,
        };
        let v0 = TokenMintTransitionActionV0 {
            base: TokenBaseTransitionAction::V0(base_v0),
            mint_amount,
            identity_balance_holder_id: holder,
            public_note,
        };
        TokenMintTransitionAction::V0(v0)
    }

    #[test]
    fn enum_getters_forward_to_v0() {
        let token_id = Identifier::new([100u8; 32]);
        let holder = Identifier::new([101u8; 32]);
        let action = build_mint_enum(token_id, holder, 1234, Some("hello".to_string()));

        // base() returns a reference into the V0 variant
        match action.base() {
            TokenBaseTransitionAction::V0(v0) => {
                assert_eq!(v0.token_id, token_id);
                assert_eq!(v0.token_contract_position, 0);
            }
        }
        assert_eq!(action.mint_amount(), 1234);
        assert_eq!(action.identity_balance_holder_id(), holder);
        assert_eq!(action.public_note(), Some(&"hello".to_string()));
    }

    #[test]
    fn enum_setters_mutate_inner_v0() {
        let mut action = build_mint_enum(
            Identifier::new([1u8; 32]),
            Identifier::new([2u8; 32]),
            10,
            None,
        );

        action.set_mint_amount(500);
        assert_eq!(action.mint_amount(), 500);

        let new_holder = Identifier::new([77u8; 32]);
        action.set_identity_balance_holder_id(new_holder);
        assert_eq!(action.identity_balance_holder_id(), new_holder);

        action.set_public_note(Some("set via enum".to_string()));
        assert_eq!(action.public_note(), Some(&"set via enum".to_string()));

        action.set_public_note(None);
        assert!(action.public_note().is_none());
    }

    #[test]
    fn enum_base_owned_and_public_note_owned_consume_self() {
        let action = build_mint_enum(
            Identifier::new([3u8; 32]),
            Identifier::new([4u8; 32]),
            42,
            Some("owned".to_string()),
        );

        // Clone to verify both owned methods without borrow issues
        let base = action.clone().base_owned();
        match base {
            TokenBaseTransitionAction::V0(v0) => {
                assert_eq!(v0.token_id, Identifier::new([3u8; 32]));
            }
        }

        let note = action.public_note_owned();
        assert_eq!(note, Some("owned".to_string()));
    }

    #[test]
    fn enum_from_v0_conversion_preserves_fields() {
        let base_v0 = TokenBaseTransitionActionV0 {
            token_id: Identifier::new([60u8; 32]),
            identity_contract_nonce: 1,
            token_contract_position: 0,
            data_contract: Arc::new(DataContractFetchInfo {
                contract: build_contract(),
                storage_flags: None,
                cost: OperationCost::default(),
                fee: None,
            }),
            store_in_group: None,
            perform_action: true,
        };
        let v0 = TokenMintTransitionActionV0 {
            base: TokenBaseTransitionAction::V0(base_v0),
            mint_amount: 555,
            identity_balance_holder_id: Identifier::new([61u8; 32]),
            public_note: Some("derive_more From".to_string()),
        };

        // The derive_more::From impl must yield an equivalent enum
        let wrapped: TokenMintTransitionAction = v0.into();
        assert_eq!(wrapped.mint_amount(), 555);
        assert_eq!(
            wrapped.identity_balance_holder_id(),
            Identifier::new([61u8; 32])
        );
        assert_eq!(wrapped.public_note(), Some(&"derive_more From".to_string()));
    }
}
