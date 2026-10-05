use crate::error::execution::ExecutionError;
use crate::error::Error;
use dpp::consensus::basic::state_transition::StateTransitionNotActiveError;
use dpp::data_contract::associated_token::token_configuration::validate_token_configurations;
use dpp::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::batch_transition::accessors::DocumentsBatchTransitionAccessorsV0;
use dpp::state_transition::batch_transition::batched_transition::document_transition::DocumentTransitionV0Methods;
use dpp::state_transition::batch_transition::batched_transition::token_transition::{
    TokenTransition, TokenTransitionV0Methods,
};
use dpp::state_transition::batch_transition::batched_transition::token_transition_action_type::TokenTransitionActionTypeGetter;
use dpp::state_transition::batch_transition::batched_transition::BatchedTransitionRef;
use dpp::state_transition::batch_transition::document_base_transition::v1::v1_methods::DocumentBaseTransitionV1Methods;
use dpp::state_transition::batch_transition::token_config_update_transition::v0::v0_methods::TokenConfigUpdateTransitionV0Methods;
use dpp::state_transition::data_contract_create_transition::accessors::DataContractCreateTransitionAccessorsV0;
use dpp::state_transition::data_contract_update_transition::accessors::DataContractUpdateTransitionAccessorsV0;
use dpp::state_transition::StateTransition;
use dpp::tokens::token_payment_info::v1::v1_accessors::TokenPaymentInfoAccessorsV1;
use dpp::version::feature_initial_protocol_versions::{
    ADDRESS_FUNDS_INITIAL_PROTOCOL_VERSION, CONTRACT_FEE_CLAIM_INITIAL_PROTOCOL_VERSION,
    CONTRACT_USER_MODERATION_INITIAL_PROTOCOL_VERSION,
    IDENTITY_KEY_LIMITS_UPDATE_INITIAL_PROTOCOL_VERSION,
    IDENTITY_TOP_UP_FROM_SHIELDED_POOL_INITIAL_PROTOCOL_VERSION,
    SHIELDED_POOL_INITIAL_PROTOCOL_VERSION, SHIELD_FROM_IDENTITY_INITIAL_PROTOCOL_VERSION,
    TOKEN_SHIELDED_POOL_INITIAL_PROTOCOL_VERSION,
};
use dpp::version::PlatformVersion;

/// A trait for validating state transitions within a blockchain.
pub(crate) trait StateTransitionIsAllowedValidationV0 {
    /// This means we should validate is state transition is allowed
    fn has_is_allowed_validation(&self) -> Result<bool, Error>;
    /// Preliminary validation for a state transition
    fn validate_is_allowed(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<()>, Error>;
}

impl StateTransitionIsAllowedValidationV0 for StateTransition {
    fn has_is_allowed_validation(&self) -> Result<bool, Error> {
        match self {
            StateTransition::Batch(_)
            | StateTransition::IdentityTopUpFromAddresses(_)
            | StateTransition::IdentityCreateFromAddresses(_)
            | StateTransition::AddressFundsTransfer(_)
            | StateTransition::IdentityCreditTransferToAddresses(_)
            | StateTransition::AddressFundingFromAssetLock(_)
            | StateTransition::AddressCreditWithdrawal(_)
            | StateTransition::Shield(_)
            | StateTransition::ShieldedTransfer(_)
            | StateTransition::IdentityTopUpFromShieldedPool(_)
            | StateTransition::TokenShieldedTransferWithShieldedFee(_)
            | StateTransition::TokenUnshieldWithShieldedFee(_)
            | StateTransition::TokenPurchaseFromShieldedPool(_)
            | StateTransition::Unshield(_)
            | StateTransition::ShieldFromAssetLock(_)
            | StateTransition::ShieldedWithdrawal(_)
            | StateTransition::IdentityCreateFromShieldedPool(_)
            | StateTransition::ShieldFromIdentity(_)
            | StateTransition::IdentityKeyLimitsUpdate(_)
            | StateTransition::ContractUserModeration(_)
            | StateTransition::ContractFeeClaim(_) => Ok(true),
            // Newly decoded token formats need an unpaid activation check even while the
            // older contract basic-structure generations remain frozen.
            //
            // This predicate is load-bearing rather than a shortcut: contract basic structure
            // validation 0 and 1, which protocol versions 9 through 13 select, walk the token
            // configurations through the version 0 accessors and never ask what format version
            // they carry. On those versions the format-version check `validate_is_allowed` runs
            // is the only one there is, and it is reached only where this returns true, so a
            // token configuration format they do not admit is refused here or nowhere.
            // Narrowing the predicate means first teaching both of those frozen generations to
            // validate token configurations. Generation 2 does validate them, which makes this
            // redundant from protocol version 14 on, and only from there.
            StateTransition::DataContractCreate(st) => Ok(st
                .data_contract()
                .tokens()
                .values()
                .any(|configuration| configuration.format_version() > 0)),
            StateTransition::DataContractUpdate(st) => Ok(st
                .data_contract()
                .tokens()
                .values()
                .any(|configuration| configuration.format_version() > 0)),
            StateTransition::IdentityCreate(_)
            | StateTransition::IdentityTopUp(_)
            | StateTransition::IdentityCreditWithdrawal(_)
            | StateTransition::IdentityUpdate(_)
            | StateTransition::IdentityCreditTransfer(_)
            | StateTransition::MasternodeVote(_) => Ok(false),
        }
    }

    fn validate_is_allowed(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<()>, Error> {
        let contract = match self {
            StateTransition::DataContractCreate(st) => Some(st.data_contract()),
            StateTransition::DataContractUpdate(st) => Some(st.data_contract()),
            _ => None,
        };
        if let Some(contract) = contract {
            // The pre-activation gate: a token configuration format the protocol version does
            // not admit is refused unpaid, whatever the frozen basic structure generations do.
            return Ok(validate_token_configurations(
                contract.tokens(),
                platform_version,
            ));
        }
        match self {
            StateTransition::Batch(st) => {
                // Token shielded pools (the batch transitions that use them, a document token
                // cost paid from one and the configuration items of a pool's threshold) are a
                // protocol-version feature, not a table-versioned validator, so the gate is
                // applied to the batch as a whole rather than to one of its transitions.
                if platform_version.protocol_version < TOKEN_SHIELDED_POOL_INITIAL_PROTOCOL_VERSION
                {
                    if let Some(transition) =
                        st.transitions_iter()
                            .find_map(|transition| match transition {
                                // Every kind that carries a bundle against a token's shielded
                                // pool, asked as one question so that a kind added later is
                                // gated by this without anyone having to remember to list it.
                                // The names are what they have always been: the action type's
                                // own name under a `Token` prefix.
                                BatchedTransitionRef::Token(token_transition)
                                    if token_transition.shielded_pool_actions().is_some() =>
                                {
                                    Some(format!("Token{}", token_transition.action_type()))
                                }
                                // Only a token with a pool has an outgoing notes threshold,
                                // and software older than it cannot decode these items.
                                BatchedTransitionRef::Token(TokenTransition::ConfigUpdate(
                                    config_update,
                                )) if matches!(
                                    config_update.update_token_configuration_item(),
                                    TokenConfigurationChangeItem::MinimumPoolNotesForOutgoing(_)
                                        | TokenConfigurationChangeItem::MinimumPoolNotesForOutgoingControlGroup(_)
                                        | TokenConfigurationChangeItem::MinimumPoolNotesForOutgoingAdminGroup(_)
                                ) =>
                                {
                                    Some(
                                        "TokenConfigUpdateMinimumPoolNotesForOutgoing".to_string(),
                                    )
                                }
                                BatchedTransitionRef::Document(document_transition)
                                    if document_transition
                                        .base()
                                        .token_payment_info_ref()
                                        .as_ref()
                                        .is_some_and(|info| info.shielded_payment().is_some()) =>
                                {
                                    Some("DocumentShieldedTokenPayment".to_string())
                                }
                                _ => None,
                            })
                    {
                        return Ok(ConsensusValidationResult::new_with_errors(vec![
                            StateTransitionNotActiveError::new(
                                transition,
                                platform_version.protocol_version,
                                TOKEN_SHIELDED_POOL_INITIAL_PROTOCOL_VERSION,
                            )
                            .into(),
                        ]));
                    }
                }
                // A batch needs no further `is_allowed` check: the token shielded pool version
                // gate above is the only one it carries.
                Ok(ConsensusValidationResult::new())
            }
            StateTransition::IdentityTopUpFromAddresses(_)
            | StateTransition::IdentityCreateFromAddresses(_)
            | StateTransition::AddressFundsTransfer(_)
            | StateTransition::IdentityCreditTransferToAddresses(_)
            | StateTransition::AddressFundingFromAssetLock(_)
            | StateTransition::AddressCreditWithdrawal(_) => {
                if platform_version.protocol_version >= ADDRESS_FUNDS_INITIAL_PROTOCOL_VERSION {
                    Ok(ConsensusValidationResult::new())
                } else {
                    Ok(ConsensusValidationResult::new_with_errors(vec![
                        StateTransitionNotActiveError::new(
                            self.state_transition_type().to_string(),
                            platform_version.protocol_version,
                            ADDRESS_FUNDS_INITIAL_PROTOCOL_VERSION,
                        )
                        .into(),
                    ]))
                }
            }
            StateTransition::Shield(_)
            | StateTransition::ShieldedTransfer(_)
            | StateTransition::Unshield(_)
            | StateTransition::ShieldFromAssetLock(_)
            | StateTransition::ShieldedWithdrawal(_)
            | StateTransition::IdentityCreateFromShieldedPool(_) => {
                if platform_version.protocol_version >= SHIELDED_POOL_INITIAL_PROTOCOL_VERSION {
                    Ok(ConsensusValidationResult::new())
                } else {
                    Ok(ConsensusValidationResult::new_with_errors(vec![
                        StateTransitionNotActiveError::new(
                            self.state_transition_type().to_string(),
                            platform_version.protocol_version,
                            SHIELDED_POOL_INITIAL_PROTOCOL_VERSION,
                        )
                        .into(),
                    ]))
                }
            }
            StateTransition::IdentityTopUpFromShieldedPool(_) => {
                if platform_version.protocol_version
                    >= IDENTITY_TOP_UP_FROM_SHIELDED_POOL_INITIAL_PROTOCOL_VERSION
                {
                    Ok(ConsensusValidationResult::new())
                } else {
                    Ok(ConsensusValidationResult::new_with_errors(vec![
                        StateTransitionNotActiveError::new(
                            self.state_transition_type().to_string(),
                            platform_version.protocol_version,
                            IDENTITY_TOP_UP_FROM_SHIELDED_POOL_INITIAL_PROTOCOL_VERSION,
                        )
                        .into(),
                    ]))
                }
            }
            StateTransition::TokenShieldedTransferWithShieldedFee(_)
            | StateTransition::TokenUnshieldWithShieldedFee(_)
            | StateTransition::TokenPurchaseFromShieldedPool(_) => {
                if platform_version.protocol_version >= TOKEN_SHIELDED_POOL_INITIAL_PROTOCOL_VERSION
                {
                    Ok(ConsensusValidationResult::new())
                } else {
                    Ok(ConsensusValidationResult::new_with_errors(vec![
                        StateTransitionNotActiveError::new(
                            self.state_transition_type().to_string(),
                            platform_version.protocol_version,
                            TOKEN_SHIELDED_POOL_INITIAL_PROTOCOL_VERSION,
                        )
                        .into(),
                    ]))
                }
            }
            StateTransition::ShieldFromIdentity(_) => {
                if platform_version.protocol_version
                    >= SHIELD_FROM_IDENTITY_INITIAL_PROTOCOL_VERSION
                {
                    Ok(ConsensusValidationResult::new())
                } else {
                    Ok(ConsensusValidationResult::new_with_errors(vec![
                        StateTransitionNotActiveError::new(
                            self.state_transition_type().to_string(),
                            platform_version.protocol_version,
                            SHIELD_FROM_IDENTITY_INITIAL_PROTOCOL_VERSION,
                        )
                        .into(),
                    ]))
                }
            }
            StateTransition::IdentityKeyLimitsUpdate(_) => {
                if platform_version.protocol_version
                    >= IDENTITY_KEY_LIMITS_UPDATE_INITIAL_PROTOCOL_VERSION
                {
                    Ok(ConsensusValidationResult::new())
                } else {
                    Ok(ConsensusValidationResult::new_with_errors(vec![
                        StateTransitionNotActiveError::new(
                            self.state_transition_type().to_string(),
                            platform_version.protocol_version,
                            IDENTITY_KEY_LIMITS_UPDATE_INITIAL_PROTOCOL_VERSION,
                        )
                        .into(),
                    ]))
                }
            }
            StateTransition::ContractUserModeration(_) => {
                if platform_version.protocol_version
                    >= CONTRACT_USER_MODERATION_INITIAL_PROTOCOL_VERSION
                {
                    Ok(ConsensusValidationResult::new())
                } else {
                    Ok(ConsensusValidationResult::new_with_errors(vec![
                        StateTransitionNotActiveError::new(
                            self.state_transition_type().to_string(),
                            platform_version.protocol_version,
                            CONTRACT_USER_MODERATION_INITIAL_PROTOCOL_VERSION,
                        )
                        .into(),
                    ]))
                }
            }
            StateTransition::ContractFeeClaim(_) => {
                if platform_version.protocol_version >= CONTRACT_FEE_CLAIM_INITIAL_PROTOCOL_VERSION
                {
                    Ok(ConsensusValidationResult::new())
                } else {
                    Ok(ConsensusValidationResult::new_with_errors(vec![
                        StateTransitionNotActiveError::new(
                            self.state_transition_type().to_string(),
                            platform_version.protocol_version,
                            CONTRACT_FEE_CLAIM_INITIAL_PROTOCOL_VERSION,
                        )
                        .into(),
                    ]))
                }
            }
            _ => Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                "validate_is_allowed is not implemented for this state transition",
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn make_data_contract_create_st() -> StateTransition {
        use dpp::tests::fixtures::get_data_contract_fixture;
        use platform_version::TryIntoPlatformVersioned;
        let platform_version = platform_version::version::PlatformVersion::latest();
        let created_data_contract =
            get_data_contract_fixture(None, 1, platform_version.protocol_version);
        let transition: dpp::state_transition::data_contract_create_transition::DataContractCreateTransition =
            created_data_contract.try_into_platform_versioned(platform_version).unwrap();
        transition.into()
    }

    fn make_data_contract_update_st() -> StateTransition {
        use dpp::data_contract::accessors::v0::DataContractV0Getters;
        use dpp::tests::fixtures::get_data_contract_fixture;
        use platform_version::TryIntoPlatformVersioned;
        let platform_version = platform_version::version::PlatformVersion::latest();
        let created_data_contract =
            get_data_contract_fixture(None, 1, platform_version.protocol_version);
        let data_contract = created_data_contract.data_contract().clone();
        let transition: dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition =
            (data_contract, 2u64).try_into_platform_versioned(platform_version).unwrap();
        transition.into()
    }

    use dpp::state_transition::batch_transition::BatchTransition;
    use dpp::state_transition::batch_transition::BatchTransitionV0;
    use dpp::state_transition::data_contract_create_transition::DataContractCreateTransition;
    use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition;
    use dpp::state_transition::identity_create_transition::IdentityCreateTransition;
    use dpp::state_transition::identity_create_transition::v0::IdentityCreateTransitionV0;
    use dpp::state_transition::identity_credit_transfer_transition::IdentityCreditTransferTransition;
    use dpp::state_transition::identity_credit_transfer_transition::v0::IdentityCreditTransferTransitionV0;
    use dpp::state_transition::identity_credit_withdrawal_transition::IdentityCreditWithdrawalTransition;
    use dpp::state_transition::identity_credit_withdrawal_transition::v0::IdentityCreditWithdrawalTransitionV0;
    use dpp::state_transition::identity_topup_transition::IdentityTopUpTransition;
    use dpp::state_transition::identity_topup_transition::v0::IdentityTopUpTransitionV0;
    use dpp::state_transition::identity_update_transition::IdentityUpdateTransition;
    use dpp::state_transition::identity_update_transition::v0::IdentityUpdateTransitionV0;
    use dpp::state_transition::masternode_vote_transition::MasternodeVoteTransition;
    use dpp::state_transition::masternode_vote_transition::v0::MasternodeVoteTransitionV0;
    use dpp::state_transition::state_transitions::identity::identity_credit_transfer_to_addresses_transition::IdentityCreditTransferToAddressesTransition;
    use dpp::state_transition::state_transitions::identity::identity_credit_transfer_to_addresses_transition::v0::IdentityCreditTransferToAddressesTransitionV0;
    use dpp::state_transition::state_transitions::identity::identity_create_from_addresses_transition::IdentityCreateFromAddressesTransition;
    use dpp::state_transition::state_transitions::identity::identity_create_from_addresses_transition::v0::IdentityCreateFromAddressesTransitionV0;
    use dpp::state_transition::state_transitions::identity::identity_topup_from_addresses_transition::IdentityTopUpFromAddressesTransition;
    use dpp::state_transition::state_transitions::identity::identity_topup_from_addresses_transition::v0::IdentityTopUpFromAddressesTransitionV0;
    use dpp::state_transition::state_transitions::address_funds::address_funds_transfer_transition::AddressFundsTransferTransition;
    use dpp::state_transition::state_transitions::address_funds::address_funds_transfer_transition::v0::AddressFundsTransferTransitionV0;
    use dpp::state_transition::state_transitions::address_funds::address_funding_from_asset_lock_transition::AddressFundingFromAssetLockTransition;
    use dpp::state_transition::state_transitions::address_funds::address_funding_from_asset_lock_transition::v0::AddressFundingFromAssetLockTransitionV0;
    use dpp::state_transition::state_transitions::address_funds::address_credit_withdrawal_transition::AddressCreditWithdrawalTransition;
    use dpp::state_transition::state_transitions::address_funds::address_credit_withdrawal_transition::v0::AddressCreditWithdrawalTransitionV0;
    use dpp::state_transition::shield_transition::ShieldTransition;
    use dpp::state_transition::shield_transition::v0::ShieldTransitionV0;
    use dpp::state_transition::shielded_transfer_transition::ShieldedTransferTransition;
    use dpp::state_transition::shielded_transfer_transition::v0::ShieldedTransferTransitionV0;
    use dpp::state_transition::unshield_transition::UnshieldTransition;
    use dpp::state_transition::unshield_transition::v0::UnshieldTransitionV0;
    use dpp::state_transition::shield_from_asset_lock_transition::ShieldFromAssetLockTransition;
    use dpp::state_transition::shield_from_asset_lock_transition::v0::ShieldFromAssetLockTransitionV0;
    use dpp::state_transition::shielded_withdrawal_transition::ShieldedWithdrawalTransition;
    use dpp::state_transition::shielded_withdrawal_transition::v0::ShieldedWithdrawalTransitionV0;

    fn make_shield_transition() -> StateTransition {
        StateTransition::Shield(ShieldTransition::V0(ShieldTransitionV0 {
            inputs: Default::default(),
            actions: vec![],
            amount: 0,
            anchor: [0u8; 32],
            proof: vec![],
            binding_signature: [0u8; 64],
            fee_strategy: vec![],
            user_fee_increase: 0,
            input_witnesses: vec![],
        }))
    }

    fn make_shielded_transfer_transition() -> StateTransition {
        StateTransition::ShieldedTransfer(ShieldedTransferTransition::V0(
            ShieldedTransferTransitionV0 {
                actions: vec![],
                value_balance: 0,
                anchor: [0u8; 32],
                proof: vec![],
                binding_signature: [0u8; 64],
            },
        ))
    }

    fn make_unshield_transition() -> StateTransition {
        StateTransition::Unshield(UnshieldTransition::V0(UnshieldTransitionV0 {
            output_address: Default::default(),
            actions: vec![],
            unshielding_amount: 0,
            anchor: [0u8; 32],
            proof: vec![],
            binding_signature: [0u8; 64],
        }))
    }

    fn make_shield_from_asset_lock_transition() -> StateTransition {
        StateTransition::ShieldFromAssetLock(ShieldFromAssetLockTransition::V0(
            ShieldFromAssetLockTransitionV0 {
                asset_lock_proof: Default::default(),
                actions: vec![],
                value_balance: 0,
                anchor: [0u8; 32],
                proof: vec![],
                binding_signature: [0u8; 64],
                surplus_output: None,
                signature: Default::default(),
            },
        ))
    }

    fn make_shielded_withdrawal_transition() -> StateTransition {
        StateTransition::ShieldedWithdrawal(ShieldedWithdrawalTransition::V0(
            ShieldedWithdrawalTransitionV0 {
                actions: vec![],
                unshielding_amount: 0,
                anchor: [0u8; 32],
                proof: vec![],
                binding_signature: [0u8; 64],
                core_fee_per_byte: 0,
                pooling: Default::default(),
                output_script: Default::default(),
            },
        ))
    }

    fn make_identity_create_from_shielded_pool_transition() -> StateTransition {
        use dpp::state_transition::state_transitions::shielded::identity_create_from_shielded_pool_transition::v0::IdentityCreateFromShieldedPoolTransitionV0;
        use dpp::state_transition::state_transitions::shielded::identity_create_from_shielded_pool_transition::IdentityCreateFromShieldedPoolTransition;
        StateTransition::IdentityCreateFromShieldedPool(
            IdentityCreateFromShieldedPoolTransition::V0(
                IdentityCreateFromShieldedPoolTransitionV0 {
                    public_keys: vec![],
                    denomination: 0,
                    actions: vec![],
                    anchor: [0u8; 32],
                    proof: vec![],
                    binding_signature: [0u8; 64],
                    send_to_address_on_creation_failure: dpp::address_funds::PlatformAddress::P2pkh(
                        [0u8; 20],
                    ),
                    identity_id: Default::default(),
                },
            ),
        )
    }

    /// Returns all state transitions grouped by expected `has_is_allowed_validation` result.
    fn transitions_requiring_allowed_validation() -> Vec<StateTransition> {
        vec![
            // A batch carries the gate that refuses the token shielded pool's transitions, the
            // pool's configuration items and a document token cost paid from a pool before the
            // protocol version admitting them.
            StateTransition::Batch(BatchTransition::V0(BatchTransitionV0::default())),
            StateTransition::IdentityTopUpFromAddresses(IdentityTopUpFromAddressesTransition::V0(
                IdentityTopUpFromAddressesTransitionV0::default(),
            )),
            StateTransition::IdentityCreateFromAddresses(
                IdentityCreateFromAddressesTransition::V0(
                    IdentityCreateFromAddressesTransitionV0::default(),
                ),
            ),
            StateTransition::AddressFundsTransfer(AddressFundsTransferTransition::V0(
                AddressFundsTransferTransitionV0::default(),
            )),
            StateTransition::IdentityCreditTransferToAddresses(
                IdentityCreditTransferToAddressesTransition::V0(
                    IdentityCreditTransferToAddressesTransitionV0::default(),
                ),
            ),
            StateTransition::AddressFundingFromAssetLock(
                AddressFundingFromAssetLockTransition::V0(
                    AddressFundingFromAssetLockTransitionV0::default(),
                ),
            ),
            StateTransition::AddressCreditWithdrawal(AddressCreditWithdrawalTransition::V0(
                AddressCreditWithdrawalTransitionV0::default(),
            )),
            make_shield_transition(),
            make_shielded_transfer_transition(),
            make_unshield_transition(),
            make_shield_from_asset_lock_transition(),
            make_shielded_withdrawal_transition(),
            make_identity_create_from_shielded_pool_transition(),
        ]
    }

    fn transitions_not_requiring_allowed_validation() -> Vec<StateTransition> {
        vec![
            make_data_contract_create_st(),
            make_data_contract_update_st(),
            StateTransition::IdentityCreate(IdentityCreateTransition::V0(
                IdentityCreateTransitionV0::default(),
            )),
            StateTransition::IdentityTopUp(IdentityTopUpTransition::V0(
                IdentityTopUpTransitionV0::default(),
            )),
            StateTransition::IdentityCreditWithdrawal(IdentityCreditWithdrawalTransition::V0(
                IdentityCreditWithdrawalTransitionV0::default(),
            )),
            StateTransition::IdentityUpdate(IdentityUpdateTransition::V0(
                IdentityUpdateTransitionV0::default(),
            )),
            StateTransition::IdentityCreditTransfer(IdentityCreditTransferTransition::V0(
                IdentityCreditTransferTransitionV0::default(),
            )),
            StateTransition::MasternodeVote(MasternodeVoteTransition::V0(
                MasternodeVoteTransitionV0::default(),
            )),
        ]
    }

    mod has_is_allowed_validation {
        use super::*;

        #[test]
        fn should_return_true_for_transitions_requiring_allowed_check() {
            for st in transitions_requiring_allowed_validation() {
                assert!(
                    st.has_is_allowed_validation().unwrap(),
                    "expected has_is_allowed_validation=true for {:?}",
                    std::mem::discriminant(&st)
                );
            }
        }

        /// The two fixtures are written by hand, so a transition kind absent from both is invisible
        /// to the tests that consume them — which is how a kind can lose its `is_allowed` phase
        /// with every test still green. The match below is exhaustive, so a new kind stops
        /// compiling here until its expected answer is stated, and the fixtures are then held to
        /// that answer for every kind they carry.
        ///
        /// The shielded and token kinds have no fixture entry: their `V0` bodies carry bundles,
        /// proofs and signatures and implement no `Default`. What the gate does for them is pinned
        /// by the block-level tests instead; what this test adds is that they cannot be forgotten.
        #[test]
        fn every_transition_kind_states_whether_it_has_an_is_allowed_phase() {
            fn expected(st: &StateTransition) -> Option<bool> {
                match st {
                    StateTransition::Batch(_)
                    | StateTransition::IdentityTopUpFromAddresses(_)
                    | StateTransition::IdentityCreateFromAddresses(_)
                    | StateTransition::AddressFundsTransfer(_)
                    | StateTransition::IdentityCreditTransferToAddresses(_)
                    | StateTransition::AddressFundingFromAssetLock(_)
                    | StateTransition::AddressCreditWithdrawal(_)
                    | StateTransition::Shield(_)
                    | StateTransition::ShieldedTransfer(_)
                    | StateTransition::Unshield(_)
                    | StateTransition::ShieldFromAssetLock(_)
                    | StateTransition::ShieldedWithdrawal(_)
                    | StateTransition::IdentityCreateFromShieldedPool(_)
                    | StateTransition::ShieldFromIdentity(_)
                    | StateTransition::IdentityTopUpFromShieldedPool(_)
                    | StateTransition::IdentityKeyLimitsUpdate(_)
                    | StateTransition::ContractUserModeration(_)
                    | StateTransition::ContractFeeClaim(_)
                    | StateTransition::TokenShieldedTransferWithShieldedFee(_)
                    | StateTransition::TokenUnshieldWithShieldedFee(_)
                    | StateTransition::TokenPurchaseFromShieldedPool(_) => Some(true),
                    StateTransition::IdentityCreate(_)
                    | StateTransition::IdentityTopUp(_)
                    | StateTransition::IdentityCreditWithdrawal(_)
                    | StateTransition::IdentityUpdate(_)
                    | StateTransition::IdentityCreditTransfer(_)
                    | StateTransition::MasternodeVote(_) => Some(false),
                    // These two answer from the contract they carry, not from their kind: a token
                    // configuration format the protocol version does not admit needs the check.
                    StateTransition::DataContractCreate(_)
                    | StateTransition::DataContractUpdate(_) => None,
                }
            }

            for st in transitions_requiring_allowed_validation() {
                assert_ne!(
                    expected(&st),
                    Some(false),
                    "a kind in the requiring fixture is stated as needing no check: {:?}",
                    std::mem::discriminant(&st)
                );
            }
            for st in transitions_not_requiring_allowed_validation() {
                assert_ne!(
                    expected(&st),
                    Some(true),
                    "a kind in the not-requiring fixture is stated as needing the check: {:?}",
                    std::mem::discriminant(&st)
                );
            }
        }

        #[test]
        fn should_return_false_for_transitions_not_requiring_allowed_check() {
            for st in transitions_not_requiring_allowed_validation() {
                assert!(
                    !st.has_is_allowed_validation().unwrap(),
                    "expected has_is_allowed_validation=false for {:?}",
                    std::mem::discriminant(&st)
                );
            }
        }
    }

    mod token_shielded_pool_gate {
        use super::*;
        use assert_matches::assert_matches;
        use dpp::consensus::basic::BasicError;
        use dpp::consensus::ConsensusError;
        use dpp::state_transition::batch_transition::batched_transition::token_burn_from_pool_transition::{TokenBurnFromPoolTransition, TokenBurnFromPoolTransitionV0};
        use dpp::state_transition::batch_transition::batched_transition::token_claim_to_pool_transition::{TokenClaimToPoolTransition, TokenClaimToPoolTransitionV0};
        use dpp::state_transition::batch_transition::batched_transition::token_direct_purchase_to_pool_transition::{TokenDirectPurchaseToPoolTransition, TokenDirectPurchaseToPoolTransitionV0};
        use dpp::state_transition::batch_transition::batched_transition::token_mint_to_pool_transition::{TokenMintToPoolTransition, TokenMintToPoolTransitionV0};
        use dpp::state_transition::batch_transition::batched_transition::token_shield_transition::{TokenShieldTransition, TokenShieldTransitionV0};
        use dpp::state_transition::batch_transition::batched_transition::token_shielded_transfer_transition::{TokenShieldedTransferTransition, TokenShieldedTransferTransitionV0};
        use dpp::state_transition::batch_transition::batched_transition::token_unshield_transition::{TokenUnshieldTransition, TokenUnshieldTransitionV0};
        use dpp::state_transition::batch_transition::batched_transition::BatchedTransition;
        use dpp::state_transition::batch_transition::BatchTransitionV1;
        use dpp::platform_value::BinaryData;

        /// A batch whose only transition is `token_transition`.
        fn batch_with(token_transition: TokenTransition) -> StateTransition {
            StateTransition::Batch(BatchTransition::V1(BatchTransitionV1 {
                owner_id: Default::default(),
                transitions: vec![BatchedTransition::Token(token_transition)],
                user_fee_increase: 0,
                signature_public_key_id: 0,
                signature: BinaryData::default(),
            }))
        }

        /// Every kind that carries a bundle against a token's shielded pool, with the name the
        /// gate reports it under.
        fn every_pool_kind() -> Vec<(TokenTransition, &'static str)> {
            vec![
                (
                    TokenTransition::Shield(TokenShieldTransition::V0(
                        TokenShieldTransitionV0::default(),
                    )),
                    "TokenShield",
                ),
                (
                    TokenTransition::Unshield(TokenUnshieldTransition::V0(
                        TokenUnshieldTransitionV0::default(),
                    )),
                    "TokenUnshield",
                ),
                (
                    TokenTransition::ShieldedTransfer(TokenShieldedTransferTransition::V0(
                        TokenShieldedTransferTransitionV0::default(),
                    )),
                    "TokenShieldedTransfer",
                ),
                (
                    TokenTransition::MintToPool(TokenMintToPoolTransition::V0(
                        TokenMintToPoolTransitionV0::default(),
                    )),
                    "TokenMintToPool",
                ),
                (
                    TokenTransition::BurnFromPool(TokenBurnFromPoolTransition::V0(
                        TokenBurnFromPoolTransitionV0::default(),
                    )),
                    "TokenBurnFromPool",
                ),
                (
                    TokenTransition::ClaimToPool(TokenClaimToPoolTransition::V0(
                        TokenClaimToPoolTransitionV0::default(),
                    )),
                    "TokenClaimToPool",
                ),
                (
                    TokenTransition::DirectPurchaseToPool(TokenDirectPurchaseToPoolTransition::V0(
                        TokenDirectPurchaseToPoolTransitionV0::default(),
                    )),
                    "TokenDirectPurchaseToPool",
                ),
            ]
        }

        /// Before the version that introduced the token shielded pools, every pool kind is
        /// refused, and refused under the name a client is told. The names are part of the
        /// answer a node of an older version gives, so they are pinned here rather than left to
        /// whatever the gate happens to build them from.
        #[test]
        fn every_pool_kind_is_refused_under_its_own_name_before_the_pools_exist() {
            let platform_version =
                PlatformVersion::get(TOKEN_SHIELDED_POOL_INITIAL_PROTOCOL_VERSION - 1)
                    .expect("the version before token shielded pools");

            for (token_transition, expected_name) in every_pool_kind() {
                let result = batch_with(token_transition)
                    .validate_is_allowed(platform_version)
                    .expect("should not error");
                assert_matches!(
                    result.errors.as_slice(),
                    [ConsensusError::BasicError(BasicError::StateTransitionNotActiveError(error))]
                        if error.state_transition_type() == expected_name
                            && error.required_protocol_version()
                                == TOKEN_SHIELDED_POOL_INITIAL_PROTOCOL_VERSION,
                    "expected {expected_name} to be refused, got {:?}",
                    result.errors
                );
            }
        }

        /// And from that version on, none of them is refused by this gate.
        #[test]
        fn no_pool_kind_is_refused_once_the_pools_exist() {
            let platform_version =
                PlatformVersion::get(TOKEN_SHIELDED_POOL_INITIAL_PROTOCOL_VERSION)
                    .expect("the version that introduced token shielded pools");

            for (token_transition, name) in every_pool_kind() {
                let result = batch_with(token_transition)
                    .validate_is_allowed(platform_version)
                    .expect("should not error");
                assert!(result.is_valid(), "{name}: {:?}", result.errors);
            }
        }
    }
}
