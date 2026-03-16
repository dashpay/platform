use platform_value::Identifier;
use crate::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem;
use crate::prelude::IdentityNonce;
use crate::state_transition::batch_transition::batched_transition::multi_party_action::AllowedAsMultiPartyAction;
use crate::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
use crate::state_transition::batch_transition::token_config_update_transition::TokenConfigUpdateTransition;
use crate::state_transition::batch_transition::token_config_update_transition::v0::v0_methods::TokenConfigUpdateTransitionV0Methods;
use crate::util::hash::hash_double;

impl TokenBaseTransitionAccessors for TokenConfigUpdateTransition {
    fn base(&self) -> &TokenBaseTransition {
        match self {
            TokenConfigUpdateTransition::V0(v0) => &v0.base,
        }
    }

    fn base_mut(&mut self) -> &mut TokenBaseTransition {
        match self {
            TokenConfigUpdateTransition::V0(v0) => &mut v0.base,
        }
    }

    fn set_base(&mut self, base: TokenBaseTransition) {
        match self {
            TokenConfigUpdateTransition::V0(v0) => v0.base = base,
        }
    }
}

impl TokenConfigUpdateTransitionV0Methods for TokenConfigUpdateTransition {
    fn update_token_configuration_item(&self) -> &TokenConfigurationChangeItem {
        match self {
            TokenConfigUpdateTransition::V0(v0) => v0.update_token_configuration_item(),
        }
    }

    fn set_update_token_configuration_item(
        &mut self,
        update_token_configuration_item: TokenConfigurationChangeItem,
    ) {
        match self {
            TokenConfigUpdateTransition::V0(v0) => {
                v0.set_update_token_configuration_item(update_token_configuration_item)
            }
        }
    }

    fn public_note(&self) -> Option<&String> {
        match self {
            TokenConfigUpdateTransition::V0(v0) => v0.public_note(),
        }
    }

    fn public_note_owned(self) -> Option<String> {
        match self {
            TokenConfigUpdateTransition::V0(v0) => v0.public_note_owned(),
        }
    }

    fn set_public_note(&mut self, public_note: Option<String>) {
        match self {
            TokenConfigUpdateTransition::V0(v0) => v0.set_public_note(public_note),
        }
    }
}

impl AllowedAsMultiPartyAction for TokenConfigUpdateTransition {
    fn calculate_action_id(&self, owner_id: Identifier) -> Identifier {
        match self {
            TokenConfigUpdateTransition::V0(v0) => v0.calculate_action_id(owner_id),
        }
    }
}

impl TokenConfigUpdateTransition {
    pub fn calculate_action_id_with_fields(
        token_id: &[u8; 32],
        owner_id: &[u8; 32],
        identity_contract_nonce: IdentityNonce,
        update_token_config_item: u8,
    ) -> Identifier {
        let mut bytes = b"action_token_config_update".to_vec();
        bytes.extend_from_slice(token_id);
        bytes.extend_from_slice(owner_id);
        bytes.extend_from_slice(&identity_contract_nonce.to_be_bytes());
        bytes.extend_from_slice(&[update_token_config_item]);

        hash_double(bytes).into()
    }
}

#[cfg(test)]
mod security_tests {
    use super::*;
    use crate::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem;
    use crate::data_contract::change_control_rules::authorized_action_takers::AuthorizedActionTakers;
    use crate::state_transition::batch_transition::batched_transition::multi_party_action::AllowedAsMultiPartyAction;
    use crate::state_transition::batch_transition::token_base_transition::v0::TokenBaseTransitionV0;
    use crate::state_transition::batch_transition::token_config_update_transition::v0::TokenConfigUpdateTransitionV0;
    use crate::state_transition::batch_transition::token_mint_transition::TokenMintTransition;

    /// SECURITY VULNERABILITY: Token config update action_id is derived from only
    /// the item TYPE INDEX (e.g. MaxSupply = 4), not the actual VALUE
    /// (e.g. MaxSupply(100) vs MaxSupply(999999999999)).
    ///
    /// Attack: A group member proposes MaxSupply(100), gets votes approved, then
    /// submits a finalization transaction with MaxSupply(999999999999) using the
    /// same action_id. Drive applies the payload without verifying it matches
    /// the stored group action.
    #[test]
    fn action_id_collision_different_max_supply_values() {
        let token_id = [1u8; 32];
        let owner_id = [2u8; 32];
        let nonce: u64 = 42;

        let conservative = TokenConfigurationChangeItem::MaxSupply(Some(100));
        let malicious = TokenConfigurationChangeItem::MaxSupply(Some(999_999_999_999));

        // Both produce the same u8 index
        assert_eq!(conservative.u8_item_index(), malicious.u8_item_index());

        // Therefore both produce the SAME action_id — this is the vulnerability
        let id_conservative = TokenConfigUpdateTransition::calculate_action_id_with_fields(
            &token_id,
            &owner_id,
            nonce,
            conservative.u8_item_index(),
        );
        let id_malicious = TokenConfigUpdateTransition::calculate_action_id_with_fields(
            &token_id,
            &owner_id,
            nonce,
            malicious.u8_item_index(),
        );

        assert_eq!(
            id_conservative, id_malicious,
            "VULNERABILITY: MaxSupply(100) and MaxSupply(999999999999) produce the same action_id"
        );
    }

    /// Full transition struct path — proves the collision persists through the
    /// AllowedAsMultiPartyAction trait used in production.
    #[test]
    fn action_id_collision_via_full_transition_structs() {
        let token_id = Identifier::new([1u8; 32]);
        let data_contract_id = Identifier::new([3u8; 32]);
        let owner_id = Identifier::new([2u8; 32]);

        let base = TokenBaseTransition::V0(TokenBaseTransitionV0 {
            identity_contract_nonce: 42,
            token_contract_position: 0,
            data_contract_id,
            token_id,
            using_group_info: None,
        });

        let transition_100 = TokenConfigUpdateTransition::V0(TokenConfigUpdateTransitionV0 {
            base: base.clone(),
            update_token_configuration_item: TokenConfigurationChangeItem::MaxSupply(Some(100)),
            public_note: None,
        });

        let transition_max = TokenConfigUpdateTransition::V0(TokenConfigUpdateTransitionV0 {
            base: base.clone(),
            update_token_configuration_item: TokenConfigurationChangeItem::MaxSupply(Some(
                999_999_999_999,
            )),
            public_note: None,
        });

        assert_eq!(
            transition_100.calculate_action_id(owner_id),
            transition_max.calculate_action_id(owner_id),
            "VULNERABILITY: Full transitions with different MaxSupply values produce same action_id"
        );
    }

    /// ManualMinting: NoOne vs ContractOwner produce the same action_id.
    /// An attacker could get approval for revoking minting then grant it instead.
    #[test]
    fn action_id_collision_minting_permissions() {
        let token_id = Identifier::new([1u8; 32]);
        let data_contract_id = Identifier::new([3u8; 32]);
        let owner_id = Identifier::new([2u8; 32]);

        let base = TokenBaseTransition::V0(TokenBaseTransitionV0 {
            identity_contract_nonce: 42,
            token_contract_position: 0,
            data_contract_id,
            token_id,
            using_group_info: None,
        });

        let revoke = TokenConfigUpdateTransition::V0(TokenConfigUpdateTransitionV0 {
            base: base.clone(),
            update_token_configuration_item: TokenConfigurationChangeItem::ManualMinting(
                AuthorizedActionTakers::NoOne,
            ),
            public_note: None,
        });

        let grant = TokenConfigUpdateTransition::V0(TokenConfigUpdateTransitionV0 {
            base: base.clone(),
            update_token_configuration_item: TokenConfigurationChangeItem::ManualMinting(
                AuthorizedActionTakers::ContractOwner,
            ),
            public_note: None,
        });

        assert_eq!(
            revoke.calculate_action_id(owner_id),
            grant.calculate_action_id(owner_id),
            "VULNERABILITY: ManualMinting(NoOne) and ManualMinting(ContractOwner) same action_id"
        );
    }

    /// Contrast: TokenMintTransition correctly binds the amount to the action_id.
    #[test]
    fn contrast_mint_correctly_binds_amount() {
        let token_id = [1u8; 32];
        let owner_id = [2u8; 32];
        let nonce: u64 = 42;

        let id_small =
            TokenMintTransition::calculate_action_id_with_fields(&token_id, &owner_id, nonce, 100);
        let id_large = TokenMintTransition::calculate_action_id_with_fields(
            &token_id, &owner_id, nonce, 999_999,
        );

        assert_ne!(
            id_small, id_large,
            "Mint correctly produces different action_ids for different amounts"
        );
    }
}
