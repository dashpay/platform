use crate::state_transition_action::batch::batched_transition::token_transition::TokenTransitionAction;
use dpp::state_transition::batch_transition::batched_transition::token_transition_action_type::{
    TokenTransitionActionType, TokenTransitionActionTypeGetter,
};

impl TokenTransitionActionTypeGetter for TokenTransitionAction {
    fn action_type(&self) -> TokenTransitionActionType {
        match self {
            TokenTransitionAction::BurnAction(_) => TokenTransitionActionType::Burn,
            TokenTransitionAction::MintAction(_) => TokenTransitionActionType::Mint,
            TokenTransitionAction::TransferAction(_) => TokenTransitionActionType::Transfer,
            TokenTransitionAction::FreezeAction(_) => TokenTransitionActionType::Freeze,
            TokenTransitionAction::UnfreezeAction(_) => TokenTransitionActionType::Unfreeze,
            TokenTransitionAction::ClaimAction(_) => TokenTransitionActionType::Claim,
            TokenTransitionAction::EmergencyActionAction(_) => {
                TokenTransitionActionType::EmergencyAction
            }
            TokenTransitionAction::DestroyFrozenFundsAction(_) => {
                TokenTransitionActionType::DestroyFrozenFunds
            }
            TokenTransitionAction::ConfigUpdateAction(_) => TokenTransitionActionType::ConfigUpdate,
            TokenTransitionAction::DirectPurchaseAction(_) => {
                TokenTransitionActionType::DirectPurchase
            }
            TokenTransitionAction::SetPriceForDirectPurchaseAction(_) => {
                TokenTransitionActionType::SetPriceForDirectPurchase
            }
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
    use crate::state_transition_action::batch::batched_transition::token_transition::token_burn_transition_action::{
        TokenBurnTransitionAction, TokenBurnTransitionActionV0,
    };
    use crate::state_transition_action::batch::batched_transition::token_transition::token_claim_transition_action::{
        TokenClaimTransitionAction, TokenClaimTransitionActionV0,
    };
    use crate::state_transition_action::batch::batched_transition::token_transition::token_config_update_transition_action::{
        TokenConfigUpdateTransitionAction, TokenConfigUpdateTransitionActionV0,
    };
    use crate::state_transition_action::batch::batched_transition::token_transition::token_destroy_frozen_funds_transition_action::{
        TokenDestroyFrozenFundsTransitionAction, TokenDestroyFrozenFundsTransitionActionV0,
    };
    use crate::state_transition_action::batch::batched_transition::token_transition::token_direct_purchase_transition_action::{
        TokenDirectPurchaseTransitionAction, TokenDirectPurchaseTransitionActionV0,
    };
    use crate::state_transition_action::batch::batched_transition::token_transition::token_emergency_action_transition_action::{
        TokenEmergencyActionTransitionAction, TokenEmergencyActionTransitionActionV0,
    };
    use crate::state_transition_action::batch::batched_transition::token_transition::token_freeze_transition_action::{
        TokenFreezeTransitionAction, TokenFreezeTransitionActionV0,
    };
    use crate::state_transition_action::batch::batched_transition::token_transition::token_mint_transition_action::{
        TokenMintTransitionAction, TokenMintTransitionActionV0,
    };
    use crate::state_transition_action::batch::batched_transition::token_transition::token_set_price_for_direct_purchase_transition_action::{
        TokenSetPriceForDirectPurchaseTransitionAction, TokenSetPriceForDirectPurchaseTransitionActionV0,
    };
    use crate::state_transition_action::batch::batched_transition::token_transition::token_transfer_transition_action::{
        TokenTransferTransitionAction, TokenTransferTransitionActionV0,
    };
    use crate::state_transition_action::batch::batched_transition::token_transition::token_unfreeze_transition_action::{
        TokenUnfreezeTransitionAction, TokenUnfreezeTransitionActionV0,
    };
    use dpp::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem;
    use dpp::data_contract::associated_token::token_distribution_key::TokenDistributionInfo;
    use dpp::identifier::Identifier;
    use dpp::tokens::emergency_action::TokenEmergencyAction;
    use platform_version::version::PlatformVersion;
    use std::sync::Arc;

    fn make_base() -> TokenBaseTransitionAction {
        let fetch_info = DataContractFetchInfo::dpns_contract_fixture(
            PlatformVersion::latest().protocol_version,
        );
        TokenBaseTransitionAction::V0(TokenBaseTransitionActionV0 {
            token_id: Identifier::new([1u8; 32]),
            identity_contract_nonce: 1,
            token_contract_position: 0,
            data_contract: Arc::new(fetch_info),
            store_in_group: None,
            perform_action: true,
        })
    }

    #[test]
    fn action_type_burn() {
        let action = TokenTransitionAction::BurnAction(TokenBurnTransitionAction::V0(
            TokenBurnTransitionActionV0 {
                base: make_base(),
                burn_from_identifier: Identifier::new([2u8; 32]),
                burn_amount: 1,
                public_note: None,
            },
        ));
        assert_eq!(action.action_type(), TokenTransitionActionType::Burn);
    }

    #[test]
    fn action_type_mint() {
        let action = TokenTransitionAction::MintAction(TokenMintTransitionAction::V0(
            TokenMintTransitionActionV0 {
                base: make_base(),
                mint_amount: 100,
                identity_balance_holder_id: Identifier::new([3u8; 32]),
                public_note: None,
            },
        ));
        assert_eq!(action.action_type(), TokenTransitionActionType::Mint);
    }

    #[test]
    fn action_type_transfer() {
        let action = TokenTransitionAction::TransferAction(TokenTransferTransitionAction::V0(
            TokenTransferTransitionActionV0 {
                base: make_base(),
                amount: 1,
                recipient_id: Identifier::new([4u8; 32]),
                public_note: None,
                shared_encrypted_note: None,
                private_encrypted_note: None,
            },
        ));
        assert_eq!(action.action_type(), TokenTransitionActionType::Transfer);
    }

    #[test]
    fn action_type_freeze() {
        let action = TokenTransitionAction::FreezeAction(TokenFreezeTransitionAction::V0(
            TokenFreezeTransitionActionV0 {
                base: make_base(),
                identity_to_freeze_id: Identifier::new([5u8; 32]),
                public_note: None,
            },
        ));
        assert_eq!(action.action_type(), TokenTransitionActionType::Freeze);
    }

    #[test]
    fn action_type_unfreeze() {
        let action = TokenTransitionAction::UnfreezeAction(TokenUnfreezeTransitionAction::V0(
            TokenUnfreezeTransitionActionV0 {
                base: make_base(),
                frozen_identity_id: Identifier::new([6u8; 32]),
                public_note: None,
            },
        ));
        assert_eq!(action.action_type(), TokenTransitionActionType::Unfreeze);
    }

    #[test]
    fn action_type_claim() {
        let action = TokenTransitionAction::ClaimAction(TokenClaimTransitionAction::V0(
            TokenClaimTransitionActionV0 {
                base: make_base(),
                amount: 42,
                distribution_info: TokenDistributionInfo::PreProgrammed(
                    0,
                    Identifier::new([7u8; 32]),
                ),
                public_note: None,
            },
        ));
        assert_eq!(action.action_type(), TokenTransitionActionType::Claim);
    }

    #[test]
    fn action_type_emergency_action() {
        let action = TokenTransitionAction::EmergencyActionAction(
            TokenEmergencyActionTransitionAction::V0(TokenEmergencyActionTransitionActionV0 {
                base: make_base(),
                emergency_action: TokenEmergencyAction::Pause,
                public_note: None,
            }),
        );
        assert_eq!(
            action.action_type(),
            TokenTransitionActionType::EmergencyAction
        );
    }

    #[test]
    fn action_type_destroy_frozen_funds() {
        let action = TokenTransitionAction::DestroyFrozenFundsAction(
            TokenDestroyFrozenFundsTransitionAction::V0(
                TokenDestroyFrozenFundsTransitionActionV0 {
                    base: make_base(),
                    frozen_identity_id: Identifier::new([8u8; 32]),
                    amount: 9,
                    public_note: None,
                },
            ),
        );
        assert_eq!(
            action.action_type(),
            TokenTransitionActionType::DestroyFrozenFunds
        );
    }

    #[test]
    fn action_type_config_update() {
        let action = TokenTransitionAction::ConfigUpdateAction(
            TokenConfigUpdateTransitionAction::V0(TokenConfigUpdateTransitionActionV0 {
                base: make_base(),
                update_token_configuration_item:
                    TokenConfigurationChangeItem::TokenConfigurationNoChange,
                public_note: None,
            }),
        );
        assert_eq!(
            action.action_type(),
            TokenTransitionActionType::ConfigUpdate
        );
    }

    #[test]
    fn action_type_direct_purchase() {
        let action = TokenTransitionAction::DirectPurchaseAction(
            TokenDirectPurchaseTransitionAction::V0(TokenDirectPurchaseTransitionActionV0 {
                base: make_base(),
                token_count: 10,
                total_agreed_price: 100,
            }),
        );
        assert_eq!(
            action.action_type(),
            TokenTransitionActionType::DirectPurchase
        );
    }

    #[test]
    fn action_type_set_price_for_direct_purchase() {
        let action = TokenTransitionAction::SetPriceForDirectPurchaseAction(
            TokenSetPriceForDirectPurchaseTransitionAction::V0(
                TokenSetPriceForDirectPurchaseTransitionActionV0 {
                    base: make_base(),
                    price: None,
                    public_note: None,
                },
            ),
        );
        assert_eq!(
            action.action_type(),
            TokenTransitionActionType::SetPriceForDirectPurchase
        );
    }

    /// All eleven variants map to distinct `TokenTransitionActionType` values.
    #[test]
    fn every_variant_maps_to_distinct_action_type() {
        let all = vec![
            TokenTransitionAction::BurnAction(TokenBurnTransitionAction::V0(
                TokenBurnTransitionActionV0 {
                    base: make_base(),
                    burn_from_identifier: Identifier::new([2u8; 32]),
                    burn_amount: 1,
                    public_note: None,
                },
            )),
            TokenTransitionAction::MintAction(TokenMintTransitionAction::V0(
                TokenMintTransitionActionV0 {
                    base: make_base(),
                    mint_amount: 1,
                    identity_balance_holder_id: Identifier::new([3u8; 32]),
                    public_note: None,
                },
            )),
            TokenTransitionAction::TransferAction(TokenTransferTransitionAction::V0(
                TokenTransferTransitionActionV0 {
                    base: make_base(),
                    amount: 1,
                    recipient_id: Identifier::new([4u8; 32]),
                    public_note: None,
                    shared_encrypted_note: None,
                    private_encrypted_note: None,
                },
            )),
            TokenTransitionAction::FreezeAction(TokenFreezeTransitionAction::V0(
                TokenFreezeTransitionActionV0 {
                    base: make_base(),
                    identity_to_freeze_id: Identifier::new([5u8; 32]),
                    public_note: None,
                },
            )),
            TokenTransitionAction::UnfreezeAction(TokenUnfreezeTransitionAction::V0(
                TokenUnfreezeTransitionActionV0 {
                    base: make_base(),
                    frozen_identity_id: Identifier::new([6u8; 32]),
                    public_note: None,
                },
            )),
            TokenTransitionAction::ClaimAction(TokenClaimTransitionAction::V0(
                TokenClaimTransitionActionV0 {
                    base: make_base(),
                    amount: 1,
                    distribution_info: TokenDistributionInfo::PreProgrammed(
                        0,
                        Identifier::new([7u8; 32]),
                    ),
                    public_note: None,
                },
            )),
            TokenTransitionAction::EmergencyActionAction(TokenEmergencyActionTransitionAction::V0(
                TokenEmergencyActionTransitionActionV0 {
                    base: make_base(),
                    emergency_action: TokenEmergencyAction::Resume,
                    public_note: None,
                },
            )),
            TokenTransitionAction::DestroyFrozenFundsAction(
                TokenDestroyFrozenFundsTransitionAction::V0(
                    TokenDestroyFrozenFundsTransitionActionV0 {
                        base: make_base(),
                        frozen_identity_id: Identifier::new([8u8; 32]),
                        amount: 1,
                        public_note: None,
                    },
                ),
            ),
            TokenTransitionAction::ConfigUpdateAction(TokenConfigUpdateTransitionAction::V0(
                TokenConfigUpdateTransitionActionV0 {
                    base: make_base(),
                    update_token_configuration_item:
                        TokenConfigurationChangeItem::TokenConfigurationNoChange,
                    public_note: None,
                },
            )),
            TokenTransitionAction::DirectPurchaseAction(TokenDirectPurchaseTransitionAction::V0(
                TokenDirectPurchaseTransitionActionV0 {
                    base: make_base(),
                    token_count: 1,
                    total_agreed_price: 1,
                },
            )),
            TokenTransitionAction::SetPriceForDirectPurchaseAction(
                TokenSetPriceForDirectPurchaseTransitionAction::V0(
                    TokenSetPriceForDirectPurchaseTransitionActionV0 {
                        base: make_base(),
                        price: None,
                        public_note: None,
                    },
                ),
            ),
        ];

        let types: Vec<TokenTransitionActionType> = all.iter().map(|a| a.action_type()).collect();

        // All 11 types are unique.
        let mut unique = types.clone();
        unique.sort_by_key(|t| format!("{}", t));
        unique.dedup();
        assert_eq!(unique.len(), 11, "every variant has a distinct action_type");
    }
}
