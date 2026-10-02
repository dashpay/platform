use crate::error::execution::ExecutionError;
use crate::error::Error;
use dpp::identity::PartialIdentity;
use dpp::state_transition::{StateTransition, StateTransitionIdentityEstimatedFeeValidation};
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;

/// A trait for validating state transitions within a blockchain.
pub(crate) trait StateTransitionIdentityBalanceValidationV0 {
    /// Validates the state transition by analyzing the changes in the platform state after applying the transaction.
    ///
    /// # Arguments
    ///
    /// * `platform` - A reference to the platform containing the state data.
    /// * `tx` - The transaction argument to be applied.
    ///
    /// # Type Parameters
    ///
    /// * `C: CoreRPCLike` - A type constraint indicating that C should implement `CoreRPCLike`.
    ///
    /// # Returns
    ///
    /// * `Result<ConsensusValidationResult<StateTransitionAction>, Error>` - A result with either a ConsensusValidationResult containing a StateTransitionAction or an Error.
    fn validate_identity_minimum_balance_pre_check(
        &self,
        identity: &PartialIdentity,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;

    /// True if the state transition has a balance validation.
    /// This balance validation is not for the operations of the state transition, but more as a
    /// quick early verification that the user has the balance they want to transfer or withdraw.
    fn has_identity_minimum_balance_pre_check_validation(&self) -> bool {
        true
    }

    /// Whether the signer got through the minimum balance pre-check only because the batch asks
    /// the contract owner to pay its gas: their own balance does not cover the fee minimum that
    /// pays for a failed batch, and a failed batch is never sponsored. Check tx validates such a
    /// batch against the state in full, like a masternode vote, on the first check and on every
    /// recheck, so that a transition nobody can be charged for is kept out of the mempool rather
    /// than executed for free by a proposer.
    fn relies_on_gas_sponsor_to_pay(
        &self,
        identity: &PartialIdentity,
        platform_version: &PlatformVersion,
    ) -> Result<bool, Error>;
}

impl StateTransitionIdentityBalanceValidationV0 for StateTransition {
    fn validate_identity_minimum_balance_pre_check(
        &self,
        identity: &PartialIdentity,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let balance =
            identity
                .balance
                .ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution(
                    "expected to have a balance on identity for credit transfer transition",
                )))?;
        match self {
            StateTransition::IdentityCreditTransfer(st) => st
                .validate_estimated_fee(balance, platform_version)
                .map_err(Error::Protocol),
            StateTransition::IdentityCreditWithdrawal(st) => st
                .validate_estimated_fee(balance, platform_version)
                .map_err(Error::Protocol),
            StateTransition::Batch(st) => match platform_version
                .drive_abci
                .validation_and_processing
                .state_transitions
                .batch_state_transition
                .identity_minimum_balance_pre_check
            {
                0 => st
                    .validate_estimated_fee(balance, platform_version)
                    .map_err(Error::Protocol),
                // A batch asking the contract owner to pay its gas cannot be judged before its
                // contracts are loaded, so the signer funds the principal here and fee
                // validation judges the gas against whoever ends up paying it. The compute fee
                // of the bundles it carries is added to that principal, because the sponsor is
                // not certain to pay it: a sub-transition state validation replaces with a
                // nonce bump takes the sponsor off the whole batch, and then the signer owes the
                // verification that already ran. A batch carrying no bundle adds nothing.
                1 if st.requests_gas_sponsorship() => st
                    .validate_estimated_principal_with_shielded_compute(balance, platform_version)
                    .map_err(Error::Protocol),
                // A batch carrying shielded pool bundles also has to hold the compute fee those
                // bundles will be charged. The flat per-sub-transition minimum is orders of
                // magnitude below one bundle verification, and a claim into a pool has its
                // proof skipped in check tx, so a signer in between passes the mempool without
                // any verification running and every validator then runs it inside block
                // validation, only to refuse the batch unpaid and let the same bytes replay.
                1 => st
                    .validate_estimated_fee_with_shielded_compute(balance, platform_version)
                    .map_err(Error::Protocol),
                version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                    method: "documents batch transition: identity minimum balance pre check"
                        .to_string(),
                    known_versions: vec![0, 1],
                    received: version,
                })),
            },
            StateTransition::IdentityCreditTransferToAddresses(st) => st
                .validate_estimated_fee(balance, platform_version)
                .map_err(Error::Protocol),
            StateTransition::ShieldFromIdentity(st) => st
                .validate_estimated_fee(balance, platform_version)
                .map_err(Error::Protocol),
            StateTransition::DataContractCreate(st) => st
                .validate_estimated_fee(balance, platform_version)
                .map_err(Error::Protocol),
            StateTransition::DataContractUpdate(st) => st
                .validate_estimated_fee(balance, platform_version)
                .map_err(Error::Protocol),
            StateTransition::IdentityUpdate(st) => st
                .validate_estimated_fee(balance, platform_version)
                .map_err(Error::Protocol),
            StateTransition::IdentityKeyLimitsUpdate(st) => st
                .validate_estimated_fee(balance, platform_version)
                .map_err(Error::Protocol),
            StateTransition::ContractUserModeration(st) => st
                .validate_estimated_fee(balance, platform_version)
                .map_err(Error::Protocol),
            StateTransition::ContractFeeClaim(st) => st
                .validate_estimated_fee(balance, platform_version)
                .map_err(Error::Protocol),
            StateTransition::MasternodeVote(_)
            | StateTransition::IdentityCreate(_)
            | StateTransition::IdentityTopUp(_)
            | StateTransition::IdentityCreateFromAddresses(_)
            | StateTransition::IdentityTopUpFromAddresses(_)
            | StateTransition::AddressFundsTransfer(_)
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
            | StateTransition::IdentityCreateFromShieldedPool(_) => {
                Ok(SimpleConsensusValidationResult::new())
            }
        }
    }

    fn relies_on_gas_sponsor_to_pay(
        &self,
        identity: &PartialIdentity,
        platform_version: &PlatformVersion,
    ) -> Result<bool, Error> {
        let StateTransition::Batch(st) = self else {
            return Ok(false);
        };
        // Pre-check v0 asks every batch for the fee minimum, so no signer relies on a sponsor.
        if platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .batch_state_transition
            .identity_minimum_balance_pre_check
            == 0
            || !st.requests_gas_sponsorship()
        {
            return Ok(false);
        }
        let balance =
            identity
                .balance
                .ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution(
                    "expected to have a balance on identity for the gas sponsorship check",
                )))?;
        // The floor an unsponsored batch would have been held to, shielded bundles included,
        // so the answer names the same shortfall the pre-check itself would have found.
        Ok(!st
            .validate_estimated_fee_with_shielded_compute(balance, platform_version)
            .map_err(Error::Protocol)?
            .is_valid())
    }

    fn has_identity_minimum_balance_pre_check_validation(&self) -> bool {
        matches!(
            self,
            StateTransition::IdentityCreditTransfer(_)
                | StateTransition::IdentityCreditWithdrawal(_)
                | StateTransition::DataContractCreate(_)
                | StateTransition::DataContractUpdate(_)
                | StateTransition::Batch(_)
                | StateTransition::IdentityUpdate(_)
                | StateTransition::IdentityKeyLimitsUpdate(_)
                | StateTransition::ContractUserModeration(_)
                | StateTransition::ContractFeeClaim(_)
                | StateTransition::ShieldFromIdentity(_)
                | StateTransition::IdentityCreditTransferToAddresses(_)
        )
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

    mod has_identity_minimum_balance_pre_check_validation {
        use super::*;

        #[test]
        fn should_return_true_for_transitions_with_balance_pre_check() {
            let transitions: Vec<(&str, StateTransition)> = vec![
                (
                    "IdentityCreditTransfer",
                    StateTransition::IdentityCreditTransfer(IdentityCreditTransferTransition::V0(
                        IdentityCreditTransferTransitionV0::default(),
                    )),
                ),
                (
                    "IdentityCreditWithdrawal",
                    StateTransition::IdentityCreditWithdrawal(
                        IdentityCreditWithdrawalTransition::V0(
                            IdentityCreditWithdrawalTransitionV0::default(),
                        ),
                    ),
                ),
                ("DataContractCreate", make_data_contract_create_st()),
                ("DataContractUpdate", make_data_contract_update_st()),
                (
                    "Batch",
                    StateTransition::Batch(BatchTransition::V0(BatchTransitionV0::default())),
                ),
                (
                    "IdentityUpdate",
                    StateTransition::IdentityUpdate(IdentityUpdateTransition::V0(
                        IdentityUpdateTransitionV0::default(),
                    )),
                ),
                (
                    "IdentityCreditTransferToAddresses",
                    StateTransition::IdentityCreditTransferToAddresses(
                        IdentityCreditTransferToAddressesTransition::V0(
                            IdentityCreditTransferToAddressesTransitionV0::default(),
                        ),
                    ),
                ),
            ];
            for (name, st) in transitions {
                assert!(
                    st.has_identity_minimum_balance_pre_check_validation(),
                    "expected true for {}",
                    name
                );
            }
        }

        #[test]
        fn should_return_false_for_transitions_without_balance_pre_check() {
            let transitions: Vec<(&str, StateTransition)> = vec![
                (
                    "MasternodeVote",
                    StateTransition::MasternodeVote(MasternodeVoteTransition::V0(
                        MasternodeVoteTransitionV0::default(),
                    )),
                ),
                (
                    "IdentityCreate",
                    StateTransition::IdentityCreate(IdentityCreateTransition::V0(
                        IdentityCreateTransitionV0::default(),
                    )),
                ),
                (
                    "IdentityTopUp",
                    StateTransition::IdentityTopUp(IdentityTopUpTransition::V0(
                        IdentityTopUpTransitionV0::default(),
                    )),
                ),
                {
                    use dpp::state_transition::state_transitions::shielded::identity_create_from_shielded_pool_transition::v0::IdentityCreateFromShieldedPoolTransitionV0;
                    use dpp::state_transition::state_transitions::shielded::identity_create_from_shielded_pool_transition::IdentityCreateFromShieldedPoolTransition;
                    (
                        "IdentityCreateFromShieldedPool",
                        StateTransition::IdentityCreateFromShieldedPool(
                            IdentityCreateFromShieldedPoolTransition::V0(
                                IdentityCreateFromShieldedPoolTransitionV0 {
                                    public_keys: vec![],
                                    denomination: 0,
                                    actions: vec![],
                                    anchor: [0u8; 32],
                                    proof: vec![],
                                    binding_signature: [0u8; 64],
                                    send_to_address_on_creation_failure:
                                        dpp::address_funds::PlatformAddress::P2pkh([0u8; 20]),
                                    identity_id: Default::default(),
                                },
                            ),
                        ),
                    )
                },
            ];
            for (name, st) in transitions {
                assert!(
                    !st.has_identity_minimum_balance_pre_check_validation(),
                    "expected false for {}",
                    name
                );
            }
        }
    }

    mod validate_identity_minimum_balance_pre_check {
        use super::*;

        #[test]
        fn should_return_error_when_identity_has_no_balance() {
            let identity = PartialIdentity {
                id: Default::default(),
                loaded_public_keys: Default::default(),
                balance: None,
                revision: None,
                not_found_public_keys: Default::default(),
            };
            let st = StateTransition::IdentityCreditTransfer(IdentityCreditTransferTransition::V0(
                IdentityCreditTransferTransitionV0::default(),
            ));
            let platform_version = &platform_version::version::v1::PLATFORM_V1;
            let result =
                st.validate_identity_minimum_balance_pre_check(&identity, platform_version);
            assert!(result.is_err(), "expected error when balance is None");
        }

        /// `count` distinct Orchard actions. Only how many there are is read here; the pre-check
        /// prices a bundle by its action count and never looks inside one.
        fn pool_actions(count: usize) -> Vec<dpp::shielded::SerializedAction> {
            (0..count)
                .map(|index| dpp::shielded::SerializedAction {
                    nullifier: [index as u8; 32],
                    rk: [2u8; 32],
                    cmx: [3u8; 32],
                    encrypted_note: vec![4u8; 216],
                    cv_net: [5u8; 32],
                    spend_auth_sig: [6u8; 64],
                })
                .collect()
        }

        fn batch_of(
            transitions: Vec<
                dpp::state_transition::batch_transition::batched_transition::BatchedTransition,
            >,
        ) -> StateTransition {
            batch_of_with_fee_increase(transitions, 0)
        }

        fn batch_of_with_fee_increase(
            transitions: Vec<
                dpp::state_transition::batch_transition::batched_transition::BatchedTransition,
            >,
            user_fee_increase: dpp::prelude::UserFeeIncrease,
        ) -> StateTransition {
            use dpp::state_transition::batch_transition::BatchTransitionV1;

            StateTransition::Batch(BatchTransition::V1(BatchTransitionV1 {
                owner_id: Default::default(),
                transitions,
                user_fee_increase,
                signature_public_key_id: 0,
                signature: Default::default(),
            }))
        }

        /// A claim into a token's shielded pool, carrying a bundle of `actions` actions.
        fn claim_to_pool(
            actions: usize,
        ) -> dpp::state_transition::batch_transition::batched_transition::BatchedTransition
        {
            use dpp::state_transition::batch_transition::batched_transition::token_claim_to_pool_transition::v0::TokenClaimToPoolTransitionV0;
            use dpp::state_transition::batch_transition::batched_transition::token_transition::TokenTransition;
            use dpp::state_transition::batch_transition::batched_transition::BatchedTransition;
            use dpp::state_transition::batch_transition::TokenClaimToPoolTransition;

            BatchedTransition::Token(TokenTransition::ClaimToPool(
                TokenClaimToPoolTransition::V0(TokenClaimToPoolTransitionV0 {
                    actions: pool_actions(actions),
                    ..Default::default()
                }),
            ))
        }

        /// A document deletion carrying `payment_info`, or none at all, on a format 1 base.
        fn document_delete(
            payment_info: Option<dpp::tokens::token_payment_info::TokenPaymentInfo>,
        ) -> dpp::state_transition::batch_transition::batched_transition::BatchedTransition
        {
            use dpp::state_transition::batch_transition::batched_transition::document_base_transition::v1::DocumentBaseTransitionV1;
            use dpp::state_transition::batch_transition::batched_transition::document_base_transition::DocumentBaseTransition;
            use dpp::state_transition::batch_transition::batched_transition::document_delete_transition::v0::DocumentDeleteTransitionV0;
            use dpp::state_transition::batch_transition::batched_transition::BatchedTransition;
            use dpp::state_transition::batch_transition::batched_transition::DocumentTransition;
            use dpp::state_transition::batch_transition::DocumentDeleteTransition;

            BatchedTransition::Document(DocumentTransition::Delete(DocumentDeleteTransition::V0(
                DocumentDeleteTransitionV0 {
                    base: DocumentBaseTransition::V1(DocumentBaseTransitionV1 {
                        token_payment_info: payment_info,
                        ..Default::default()
                    }),
                },
            )))
        }

        /// A document whose token cost is paid out of the payment token's shielded pool. It
        /// spends pool notes exactly as a token transition does, so it is priced the same way.
        /// `gas_fees_paid_by` is who the document asks to pay its gas.
        fn document_paid_from_pool(
            actions: usize,
            gas_fees_paid_by: dpp::tokens::gas_fees_paid_by::GasFeesPaidBy,
        ) -> dpp::state_transition::batch_transition::batched_transition::BatchedTransition
        {
            use dpp::tokens::token_payment_info::v1::{TokenPaymentInfoV1, TokenShieldedPayment};
            use dpp::tokens::token_payment_info::TokenPaymentInfo;

            document_delete(Some(TokenPaymentInfo::V1(TokenPaymentInfoV1 {
                gas_fees_paid_by,
                shielded_payment: Box::new(TokenShieldedPayment {
                    actions: pool_actions(actions),
                    ..Default::default()
                }),
                ..Default::default()
            })))
        }

        /// A document paying its token cost the ordinary way, out of a token balance. It
        /// carries payment info but no bundle, which is the common shape the floor must leave
        /// alone.
        fn document_paid_from_a_balance(
        ) -> dpp::state_transition::batch_transition::batched_transition::BatchedTransition
        {
            use dpp::tokens::token_payment_info::v0::TokenPaymentInfoV0;
            use dpp::tokens::token_payment_info::TokenPaymentInfo;

            document_delete(Some(TokenPaymentInfo::V0(TokenPaymentInfoV0::default())))
        }

        fn identity_with_balance(balance: u64) -> PartialIdentity {
            PartialIdentity {
                id: Default::default(),
                loaded_public_keys: Default::default(),
                balance: Some(balance),
                revision: None,
                not_found_public_keys: Default::default(),
            }
        }

        /// A claim into a shielded pool costs one Halo 2 bundle verification plus the work of
        /// its action, and the flat per-sub-transition minimum is a fraction of that. A signer
        /// in between has to be refused here, before the proof runs: the claim's proof is
        /// skipped in check tx because the claimable amount is only known against state, so
        /// admitting the batch buys every validator the verification inside block validation,
        /// which then refuses the batch unpaid and leaves the same bytes replayable.
        #[test]
        fn should_refuse_a_batch_claiming_into_a_pool_when_the_balance_cannot_pay_for_the_proof() {
            let platform_version = PlatformVersion::latest();

            for actions in [1usize, 3] {
                let st = batch_of(vec![claim_to_pool(actions)]);

                let flat_minimum = platform_version
                    .fee_version
                    .state_transition_min_fees
                    .document_batch_sub_transition;
                let shielded_fee =
                    dpp::shielded::compute_shielded_verification_fee(actions, platform_version)
                        .expect("shielded compute fee");
                assert!(
                    shielded_fee > flat_minimum,
                    "the proof costs more than the flat minimum, or there is no gap to close"
                );

                // Comfortably over the flat minimum, nowhere near what the proof costs.
                let balance = flat_minimum * 10;
                assert!(balance < shielded_fee);

                let result = st
                    .validate_identity_minimum_balance_pre_check(
                        &identity_with_balance(balance),
                        platform_version,
                    )
                    .expect("pre check should not error");
                assert!(
                    !result.is_valid(),
                    "a balance of {balance} cannot pay the {shielded_fee} a {actions}-action \
                     bundle will be charged"
                );

                // One credit short of the bundle's own fee is still refused, so the floor
                // reserves every action's work and not just the bundle's.
                let result = st
                    .validate_identity_minimum_balance_pre_check(
                        &identity_with_balance(shielded_fee + flat_minimum - 1),
                        platform_version,
                    )
                    .expect("pre check should not error");
                assert!(!result.is_valid());

                // A signer who can pay is admitted, so what the floor refuses is only what fee
                // validation would have refused later anyway.
                let result = st
                    .validate_identity_minimum_balance_pre_check(
                        &identity_with_balance(shielded_fee + flat_minimum),
                        platform_version,
                    )
                    .expect("pre check should not error");
                assert!(result.is_valid());
            }
        }

        /// A document paying its token cost out of the pool spends pool notes exactly as a
        /// token transition does, and is charged the same compute fee when its action is built,
        /// so the floor has to reserve it for a document batch too.
        #[test]
        fn should_refuse_a_document_paid_from_a_pool_when_the_balance_cannot_pay_for_the_proof() {
            use dpp::tokens::gas_fees_paid_by::GasFeesPaidBy;

            let platform_version = PlatformVersion::latest();
            let st = batch_of(vec![document_paid_from_pool(
                2,
                GasFeesPaidBy::DocumentOwner,
            )]);

            let flat_minimum = platform_version
                .fee_version
                .state_transition_min_fees
                .document_batch_sub_transition;
            let shielded_fee =
                dpp::shielded::compute_shielded_verification_fee(2, platform_version)
                    .expect("shielded compute fee");

            let result = st
                .validate_identity_minimum_balance_pre_check(
                    &identity_with_balance(shielded_fee + flat_minimum - 1),
                    platform_version,
                )
                .expect("pre check should not error");
            assert!(
                !result.is_valid(),
                "one credit short of the {shielded_fee} the payment's bundle will be charged"
            );

            let result = st
                .validate_identity_minimum_balance_pre_check(
                    &identity_with_balance(shielded_fee + flat_minimum),
                    platform_version,
                )
                .expect("pre check should not error");
            assert!(result.is_valid());
        }

        /// A batch that asks the contract owner for its gas funds only its principal, and the
        /// shielded compute fee is gas. Reserving it against the signer would refuse a
        /// sponsored document the contract owner was going to pay for.
        #[test]
        fn should_reserve_the_shielded_fee_against_a_sponsored_signer_too() {
            use dpp::tokens::gas_fees_paid_by::GasFeesPaidBy;

            let platform_version = PlatformVersion::latest();
            let st = batch_of(vec![document_paid_from_pool(
                2,
                GasFeesPaidBy::ContractOwner,
            )]);

            let result = st
                .validate_identity_minimum_balance_pre_check(
                    &identity_with_balance(0),
                    platform_version,
                )
                .expect("pre check should not error");
            assert!(
                !result.is_valid(),
                "a sponsor is not certain to pay: a sub-transition replaced by a nonce bump takes \
                 the sponsor off the batch and leaves the signer owing the verification that \
                 already ran, so the signer has to hold it"
            );

            // The mempool still has to know the signer could not have paid unsponsored, so a
            // batch nobody can be charged for is validated against the state in full.
            let identity = identity_with_balance(0);
            assert!(st
                .relies_on_gas_sponsor_to_pay(&identity, platform_version)
                .expect("sponsor reliance should not error"));
        }

        #[test]
        fn should_leave_the_released_floor_alone_when_the_signer_chose_an_increase() {
            use dpp::state_transition::StateTransitionIdentityEstimatedFeeValidation;

            let platform_version = PlatformVersion::latest();

            // Generation 0 of the pre-check is what every protocol version up to 13 selects, and
            // those are released: the balance it admits has to stay exactly what it was, whatever
            // increase the signer chose. A plain delete carries no principal, so its floor is the
            // flat per-sub-transition minimum.
            let flat_minimum = platform_version
                .fee_version
                .state_transition_min_fees
                .document_batch_sub_transition;

            for increase in [0u16, 50] {
                let StateTransition::Batch(batch) =
                    batch_of_with_fee_increase(vec![document_delete(None)], increase)
                else {
                    panic!("the fixture builds a batch");
                };
                assert!(
                    batch
                        .validate_estimated_fee(flat_minimum, platform_version)
                        .expect("estimated fee should not error")
                        .is_valid(),
                    "generation 0 is released: an increase of {increase} must not move its floor"
                );
            }
        }

        #[test]
        fn should_ask_for_the_fee_increase_the_signer_chose_on_top_of_the_compute_fee() {
            let platform_version = PlatformVersion::latest();
            let actions = 2usize;

            // The balance that exactly meets the floor when the signer asked for no increase.
            let flat_minimum = platform_version
                .fee_version
                .state_transition_min_fees
                .document_batch_sub_transition;
            let shielded_fee =
                dpp::shielded::compute_shielded_verification_fee(actions, platform_version)
                    .expect("shielded compute fee");
            let exactly_enough = shielded_fee + flat_minimum;

            // Asserted relationally: the same batch and the same balance, differing only in the
            // increase the signer chose, so the test cannot pass by agreeing with whatever the
            // floor happens to compute.
            assert!(
                batch_of_with_fee_increase(vec![claim_to_pool(actions)], 0)
                    .validate_identity_minimum_balance_pre_check(
                        &identity_with_balance(exactly_enough),
                        platform_version,
                    )
                    .expect("pre check should not error")
                    .is_valid(),
                "the floor without an increase is the baseline this test moves from"
            );
            assert!(
                !batch_of_with_fee_increase(vec![claim_to_pool(actions)], 50)
                    .validate_identity_minimum_balance_pre_check(
                        &identity_with_balance(exactly_enough),
                        platform_version,
                    )
                    .expect("pre check should not error")
                    .is_valid(),
                "settlement charges the increase the signer asked for, so the floor has to ask \
                 for it too: otherwise the proof is verified and then nobody can be charged"
            );
        }

        #[test]
        fn should_hold_a_sponsored_signer_to_the_whole_floor_when_its_batch_carries_a_bundle() {
            use dpp::tokens::gas_fees_paid_by::GasFeesPaidBy;

            let platform_version = PlatformVersion::latest();
            let actions = 2usize;
            let st = batch_of(vec![document_paid_from_pool(
                actions,
                GasFeesPaidBy::ContractOwner,
            )]);

            let flat_minimum = platform_version
                .fee_version
                .state_transition_min_fees
                .document_batch_sub_transition;
            let compute_fee =
                dpp::shielded::compute_shielded_verification_fee(actions, platform_version)
                    .expect("shielded compute fee");

            // The compute fee alone is what a failed event does not cost: dropping the sponsor
            // leaves the signer owing the signature, the contract reads, the pool reads and the
            // nonce bump beside it. Holding a sponsored signer to the floor an unsponsored one
            // meets is what makes the failure chargeable, since that floor already is.
            assert!(
                !st.validate_identity_minimum_balance_pre_check(
                    &identity_with_balance(compute_fee),
                    platform_version,
                )
                .expect("pre check should not error")
                .is_valid(),
                "the compute fee alone leaves the rest of a failed event unfunded"
            );
            assert!(
                st.validate_identity_minimum_balance_pre_check(
                    &identity_with_balance(compute_fee + flat_minimum),
                    platform_version,
                )
                .expect("pre check should not error")
                .is_valid(),
                "the whole floor is what it is asked for, the same one an unsponsored batch meets"
            );
        }

        /// Every bundle in a batch is priced, not just the first: the floor asks for the sum,
        /// so a batch cannot smuggle proof work past it behind one funded bundle.
        #[test]
        fn should_reserve_the_fee_of_every_bundle_a_batch_carries() {
            use dpp::tokens::gas_fees_paid_by::GasFeesPaidBy;

            let platform_version = PlatformVersion::latest();
            let st = batch_of(vec![
                claim_to_pool(1),
                document_paid_from_pool(2, GasFeesPaidBy::DocumentOwner),
            ]);

            let flat_minimum = platform_version
                .fee_version
                .state_transition_min_fees
                .document_batch_sub_transition;
            let both_bundles =
                dpp::shielded::compute_shielded_verification_fee(1, platform_version)
                    .expect("shielded compute fee")
                    + dpp::shielded::compute_shielded_verification_fee(2, platform_version)
                        .expect("shielded compute fee");
            let floor = both_bundles + flat_minimum * 2;

            let result = st
                .validate_identity_minimum_balance_pre_check(
                    &identity_with_balance(floor - 1),
                    platform_version,
                )
                .expect("pre check should not error");
            assert!(
                !result.is_valid(),
                "one credit short of the {both_bundles} both bundles will be charged"
            );

            let result = st
                .validate_identity_minimum_balance_pre_check(
                    &identity_with_balance(floor),
                    platform_version,
                )
                .expect("pre check should not error");
            assert!(result.is_valid());
        }

        /// The shielded floor is scoped to the bundles a batch carries: a batch without one
        /// must still be asked for the flat per-sub-transition minimum and no more, or the
        /// change would raise the cost of every batch on the network. This holds on both
        /// generations of the pre-check; it is what must not change, rather than what did.
        #[test]
        fn should_keep_the_flat_per_sub_transition_minimum_for_a_batch_without_a_pool_bundle() {
            let platform_version = PlatformVersion::latest();

            let flat_minimum = platform_version
                .fee_version
                .state_transition_min_fees
                .document_batch_sub_transition;

            // A document with no payment info at all, and one paying its token cost the
            // ordinary way: neither carries a bundle, so neither is asked for more.
            for transition in [document_delete(None), document_paid_from_a_balance()] {
                let st = batch_of(vec![transition]);

                let result = st
                    .validate_identity_minimum_balance_pre_check(
                        &identity_with_balance(flat_minimum),
                        platform_version,
                    )
                    .expect("pre check should not error");
                assert!(
                    result.is_valid(),
                    "the flat minimum of {flat_minimum} still admits a batch that carries no bundle"
                );

                let result = st
                    .validate_identity_minimum_balance_pre_check(
                        &identity_with_balance(flat_minimum - 1),
                        platform_version,
                    )
                    .expect("pre check should not error");
                assert!(
                    !result.is_valid(),
                    "one credit under the flat minimum is still refused"
                );
            }
        }

        #[test]
        fn should_return_ok_for_transitions_without_balance_check() {
            let identity = PartialIdentity {
                id: Default::default(),
                loaded_public_keys: Default::default(),
                balance: Some(0),
                revision: None,
                not_found_public_keys: Default::default(),
            };
            let st = StateTransition::MasternodeVote(MasternodeVoteTransition::V0(
                MasternodeVoteTransitionV0::default(),
            ));
            let platform_version = &platform_version::version::v1::PLATFORM_V1;
            let result = st
                .validate_identity_minimum_balance_pre_check(&identity, platform_version)
                .expect("should not error");
            assert!(result.is_valid());
        }
    }
}
