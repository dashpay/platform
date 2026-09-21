use crate::error::Error;
use crate::execution::types::execution_event::ExecutionEvent;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::processor::address_balances_and_nonces::StateTransitionAddressBalancesAndNoncesValidation;
use crate::execution::validation::state_transition::processor::address_witnesses::{
    StateTransitionAddressWitnessValidationV0, StateTransitionHasAddressWitnessValidationV0,
};
use crate::execution::validation::state_transition::processor::addresses_minimum_balance::StateTransitionAddressesMinimumBalanceValidationV0;
use crate::execution::validation::state_transition::processor::advanced_structure_with_state::StateTransitionStructureKnownInStateValidationV0;
use crate::execution::validation::state_transition::processor::advanced_structure_without_state::StateTransitionAdvancedStructureValidationV0;
use crate::execution::validation::state_transition::processor::basic_structure::StateTransitionBasicStructureValidationV0;
use crate::execution::validation::state_transition::processor::identity_balance::StateTransitionIdentityBalanceValidationV0;
use crate::execution::validation::state_transition::processor::identity_based_signature::StateTransitionIdentityBasedSignatureValidationV0;
use crate::execution::validation::state_transition::processor::identity_nonces::{
    StateTransitionHasIdentityNonceValidationV0, StateTransitionIdentityNonceValidationV0,
};
use crate::execution::validation::state_transition::processor::is_allowed::StateTransitionIsAllowedValidationV0;
use crate::execution::validation::state_transition::processor::prefunded_specialized_balance::StateTransitionPrefundedSpecializedBalanceValidationV0;
use crate::execution::validation::state_transition::processor::state::StateTransitionStateValidation;
use crate::execution::validation::state_transition::processor::traits::shielded_proof::{
    StateTransitionHasShieldedProofValidationV0, StateTransitionShieldedMinimumFeeValidationV0,
    StateTransitionShieldedProofValidationV0,
};
use crate::execution::validation::state_transition::transformer::StateTransitionSignerAwareActionTransformer;
use crate::execution::validation::state_transition::ValidationMode;
use crate::platform_types::platform::PlatformRef;
use crate::platform_types::platform_state::PlatformStateV0Methods;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::StateTransition;
use dpp::version::{DefaultForPlatformVersion, PlatformVersion};
use dpp::ProtocolError;
use drive::grovedb::TransactionArg;

/// v1 (protocol version 14) = v0 with the masternode vote's prefunded balance pre-check acted
/// upon: a vote whose poll has no fund, or one below the single vote cost, is refused as an
/// unpaid consensus error after state validation, where v0 ignored the pre-check's result and
/// let the vote fail inside execution as an internal error. Every other step is identical to v0.
pub(super) fn process_state_transition_v1<'a, C: CoreRPCLike>(
    platform: &'a PlatformRef<C>,
    block_info: &BlockInfo,
    state_transition: StateTransition,
    transaction: TransactionArg,
    platform_version: &PlatformVersion,
) -> Result<ConsensusValidationResult<ExecutionEvent<'a>>, Error> {
    let mut state_transition_execution_context =
        StateTransitionExecutionContext::default_for_platform_version(platform_version)?;

    if state_transition.has_is_allowed_validation()? {
        let result = state_transition.validate_is_allowed(platform, platform_version)?;

        if !result.is_valid() {
            return Ok(ConsensusValidationResult::<ExecutionEvent>::new_with_errors(result.errors));
        }
    }

    // Only identity create does not use identity in state validation, because it doesn't yet have the identity in state
    let mut maybe_identity = if state_transition.uses_identity_in_state() {
        // Validating signature for identity based state transitions (all those except identity create and identity top up)
        // As we already have removed identity create above, it just splits between identity top up (below - false) and
        // all other state transitions (above - true)
        let result = if state_transition.validates_signature_based_on_identity_info() {
            state_transition.validate_identity_signed_state_transition(
                platform.drive,
                transaction,
                &mut state_transition_execution_context,
                platform_version,
            )
        } else {
            // Currently only identity top up and identity top up from addresses uses this,
            // We will add the cost for a balance retrieval
            state_transition.retrieve_identity_info(
                platform.drive,
                transaction,
                &mut state_transition_execution_context,
                platform_version,
            )
        }?;
        if !result.is_valid() {
            // If the signature is not valid or if we could not retrieve identity info
            // we do not have the user pay for the state transition.
            // Since it is most likely not from them
            // Proposers should remove such transactions from the block
            // Other validators should reject blocks with such transactions
            return Ok(ConsensusValidationResult::<ExecutionEvent>::new_with_errors(result.errors));
        }
        Some(result.into_data()?)
    } else {
        // Currently only identity create
        None
    };

    if state_transition.has_address_witness_validation(platform_version)? {
        let result = state_transition.validate_address_witnesses(
            &mut state_transition_execution_context,
            platform_version,
        )?;
        if !result.is_valid() {
            // If the witnesses are not valid
            // Proposers should remove such transactions from the block
            // Other validators should reject blocks with such transactions
            return Ok(ConsensusValidationResult::<ExecutionEvent>::new_with_errors(result.errors));
        }
    }

    // Start by validating addresses if the transition has input addresses
    let remaining_address_balances = if state_transition
        .has_addresses_balances_and_nonces_validation()
    {
        // Here we validate that all input addresses have enough balance
        // We also validate that nonces are bumped
        let result = state_transition.validate_address_balances_and_nonces(
            platform.drive,
            &mut state_transition_execution_context,
            transaction,
            platform_version,
        )?;
        if !result.is_valid() {
            // The nonces are not valid or there is not enough balance. The transaction is each replaying an input or there
            // isn't enough balance, either way the transaction should be rejected.
            return Ok(ConsensusValidationResult::<ExecutionEvent>::new_with_errors(result.errors));
        }
        Some(result.into_data()?)
    } else {
        None
    };

    // Only identity top up and identity create do not have nonces validation
    if state_transition.has_identity_nonce_validation(platform_version)? {
        // Validating identity contract nonce, this must happen after validating the signature
        let result = state_transition.validate_identity_nonces(
            &platform.into(),
            platform.state.last_block_info(),
            transaction,
            &mut state_transition_execution_context,
            platform_version,
        )?;

        if !result.is_valid() {
            // If the nonce is not valid the state transition is not paid for, most likely because
            // this is just a replayed block
            // Proposers should remove such transactions from the block
            // Other validators should reject blocks with such transactions
            return Ok(ConsensusValidationResult::<ExecutionEvent>::new_with_errors(result.errors));
        }
    }

    // Only Data contract state transitions and Masternode vote do not have basic structure validation
    if state_transition.has_basic_structure_validation(platform_version) {
        // We validate basic structure validation after verifying the identity,
        // this is structure validation that does not require state and is already checked on check_tx
        let consensus_result =
            state_transition.validate_basic_structure(platform.config.network, platform_version)?;

        if !consensus_result.is_valid() {
            // Basic structure validation is extremely cheap to process, because of this attacks are
            // not likely.
            // Often the basic structure validation is necessary for estimated costs
            // Proposers should remove such transactions from the block
            // Other validators should reject blocks with such transactions
            return Ok(
                ConsensusValidationResult::<ExecutionEvent>::new_with_errors(
                    consensus_result.errors,
                ),
            );
        }
    }

    // For identity credit withdrawal and identity credit transfers we have a balance pre-check that includes a
    // processing amount and the transfer amount.
    // For other state transitions we only check a min balance for an amount set per version.
    // This is not done for identity create and identity top up who don't have this check here
    if state_transition.has_identity_minimum_balance_pre_check_validation() {
        // Validating that we have sufficient balance for a transfer or withdrawal,
        // this must happen after validating the signature

        let identity = maybe_identity
            .as_mut()
            .ok_or(ProtocolError::CorruptedCodeExecution(
                "identity must be known to validate the balance".to_string(),
            ))?;
        let result = state_transition
            .validate_identity_minimum_balance_pre_check(identity, platform_version)?;

        if !result.is_valid() {
            return Ok(ConsensusValidationResult::<ExecutionEvent>::new_with_errors(result.errors));
        }
    }

    // For address-based state transitions that transfer or withdraw, we have a balance pre-check
    // that validates addresses have enough remaining balance after the input amounts to cover fees.
    if state_transition.has_addresses_minimum_balance_pre_check_validation() {
        // Validating that addresses have sufficient remaining balance for fees,
        // this must happen after validating the address balances and nonces

        let address_balances =
            remaining_address_balances
                .as_ref()
                .ok_or(ProtocolError::CorruptedCodeExecution(
                    "address balances must be known to validate the minimum balance".to_string(),
                ))?;
        let result = state_transition
            .validate_addresses_minimum_balance_pre_check(address_balances, platform_version)?;

        if !result.is_valid() {
            return Ok(ConsensusValidationResult::<ExecutionEvent>::new_with_errors(result.errors));
        }
    }

    // Validate minimum fee for shielded spending transitions (stateless, uses public value_balance).
    // This is cheaper than proof verification so we check it first.
    // Only applies to ShieldedTransfer/Unshield/ShieldedWithdrawal — Shield pays from address
    // inputs and ShieldFromAssetLock pays from the asset lock.
    if state_transition.has_shielded_minimum_fee_validation() {
        let result = state_transition.validate_minimum_shielded_fee(platform_version)?;
        if !result.is_valid() {
            return Ok(ConsensusValidationResult::<ExecutionEvent>::new_with_errors(result.errors));
        }
    }

    // Verify ZK proof for shielded transitions (stateless, like signature verification).
    if state_transition.has_shielded_proof_validation() {
        let result = state_transition.validate_shielded_proof(platform_version)?;
        if !result.is_valid() {
            return Ok(ConsensusValidationResult::<ExecutionEvent>::new_with_errors(result.errors));
        }
    }

    // Only identity update and data contract create have advanced structure validation without state
    if state_transition.has_advanced_structure_validation_without_state() {
        // Currently only used for Identity Update, Data Contract Create and Identity Create From Addresses
        // Next we have advanced structure validation, this is structure validation that does not require
        // state but isn't checked on check_tx. If advanced structure fails identity nonces or identity
        // contract nonces will be bumped
        let identity = maybe_identity
            .as_ref()
            .ok_or(ProtocolError::CorruptedCodeExecution(
                "the identity should always be known on advanced structure validation".to_string(),
            ))?;
        let consensus_result = state_transition.validate_advanced_structure(
            identity,
            &mut state_transition_execution_context,
            platform_version,
        )?;

        if !consensus_result.is_valid() {
            return consensus_result.map_result(|action| {
                ExecutionEvent::create_from_state_transition_action(
                    action,
                    maybe_identity,
                    platform.state.last_committed_block_epoch_ref(),
                    state_transition_execution_context,
                    platform_version,
                )
            });
        }
    }

    // Identity create, documents batch and masternode vote all have advanced structure validation with state
    let action = if state_transition.has_advanced_structure_validation_with_state() {
        // Currently used for identity create and documents batch
        let state_transition_action_result = state_transition.transform_into_action_for_signer(
            platform,
            block_info,
            &remaining_address_balances,
            maybe_identity.as_ref(),
            ValidationMode::Validator,
            &mut state_transition_execution_context,
            transaction,
        )?;
        if !state_transition_action_result.is_valid_with_data() {
            return state_transition_action_result.map_result(|action| {
                ExecutionEvent::create_from_state_transition_action(
                    action,
                    maybe_identity,
                    platform.state.last_committed_block_epoch_ref(),
                    state_transition_execution_context,
                    platform_version,
                )
            });
        }
        let action = state_transition_action_result.into_data()?;

        // Validating structure
        let result = state_transition.validate_advanced_structure_from_state(
            block_info,
            platform.config.network,
            &action,
            maybe_identity.as_ref(),
            &mut state_transition_execution_context,
            platform_version,
        )?;
        if !result.is_valid() {
            return result.map_result(|action| {
                ExecutionEvent::create_from_state_transition_action(
                    action,
                    maybe_identity,
                    platform.state.last_committed_block_epoch_ref(),
                    state_transition_execution_context,
                    platform_version,
                )
            });
        }

        Some(action)
    } else {
        None
    };

    // Validating state
    // Only identity Top up does not validate state and instead just returns the action for topping up
    let result = if state_transition.has_state_validation() {
        state_transition.validate_state(
            action,
            platform,
            ValidationMode::Validator,
            block_info,
            &mut state_transition_execution_context,
            transaction,
        )?
    } else if let Some(action) = action {
        ConsensusValidationResult::new_with_data(action)
    } else {
        state_transition.transform_into_action_for_signer(
            platform,
            block_info,
            &remaining_address_balances,
            maybe_identity.as_ref(),
            ValidationMode::Validator,
            &mut state_transition_execution_context,
            transaction,
        )?
    };

    // A masternode vote is paid by its vote poll's prefunded specialized balance, never by the
    // voter. When that fund does not exist or cannot cover the vote, nobody can be charged
    // for the vote, so it is refused unpaid: proposers strip it from their block and other
    // validators reject a block that carries it, exactly like a vote that fails its nonce
    // check. v0 ran this pre-check but ignored its result, and such a vote failed inside
    // execution, when its cost was deducted, as an internal error.
    //
    // The fund is checked last, once state validation has found the poll and seen it open.
    // Settling a poll deletes its fund, so a check ahead of state validation would report a
    // missing fund for every vote that arrives after the poll ended and hide the poll's real
    // status from the voter.
    if result.is_valid() && state_transition.uses_prefunded_specialized_balance_for_payment() {
        let fund_result = state_transition
            .validate_minimum_prefunded_specialized_balance_pre_check(
                platform.drive,
                transaction,
                &mut state_transition_execution_context,
                platform_version,
            )?;

        if !fund_result.is_valid() {
            return Ok(
                ConsensusValidationResult::<ExecutionEvent>::new_with_errors(fund_result.errors),
            );
        }
    }

    result.map_result(|action| {
        ExecutionEvent::create_from_state_transition_action(
            action,
            maybe_identity,
            platform.state.last_committed_block_epoch_ref(),
            state_transition_execution_context,
            platform_version,
        )
    })
}

#[cfg(test)]
mod tests {
    use crate::execution::validation::state_transition::state_transitions::tests::{
        create_dpns_identity_name_contest, setup_masternode_voting_identity,
    };
    use crate::platform_types::state_transitions_processing_result::StateTransitionExecutionResult;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::{TempPlatform, TestPlatformBuilder};
    use assert_matches::assert_matches;
    use dpp::block::block_info::BlockInfo;
    use dpp::consensus::state::state_error::StateError;
    use dpp::consensus::ConsensusError;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::identifier::Identifier;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::{Identity, IdentityPublicKey};
    use dpp::platform_value::Value;
    use dpp::prelude::DataContract;
    use dpp::serialization::PlatformSerializable;
    use dpp::state_transition::masternode_vote_transition::methods::MasternodeVoteTransitionMethodsV0;
    use dpp::state_transition::masternode_vote_transition::MasternodeVoteTransition;
    use dpp::util::strings::convert_to_homograph_safe_chars;
    use dpp::version::PlatformVersion;
    use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
    use dpp::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
    use dpp::voting::vote_polls::VotePoll;
    use dpp::voting::votes::resource_vote::v0::ResourceVoteV0;
    use dpp::voting::votes::resource_vote::ResourceVote;
    use dpp::voting::votes::Vote;
    use simple_signer::signer::SimpleSigner;
    use std::sync::Arc;

    const NAME: &str = "quantum";

    /// A DPNS name contest whose vote poll's fund the test then tampers with, and one
    /// masternode ready to vote on it.
    struct Contest {
        platform: TempPlatform<MockCoreRPCLike>,
        dpns_contract: Arc<DataContract>,
        contender: Identity,
        fund_id: Identifier,
        voter: Voter,
    }

    struct Voter {
        pro_tx_hash: Identifier,
        identity: Identity,
        signer: SimpleSigner,
        voting_key: IdentityPublicKey,
    }

    fn vote_poll(dpns_contract: &DataContract) -> ContestedDocumentResourceVotePoll {
        vote_poll_for(dpns_contract, NAME)
    }

    fn vote_poll_for(
        dpns_contract: &DataContract,
        name: &str,
    ) -> ContestedDocumentResourceVotePoll {
        ContestedDocumentResourceVotePoll {
            contract_id: dpns_contract.id(),
            document_type_name: "domain".to_string(),
            index_name: "parentNameAndLabel".to_string(),
            index_values: vec![
                Value::Text("dash".to_string()),
                Value::Text(convert_to_homograph_safe_chars(name)),
            ],
        }
    }

    async fn contest_at(platform_version: &PlatformVersion) -> Contest {
        let mut platform = TestPlatformBuilder::new()
            .with_initial_protocol_version(platform_version.protocol_version)
            .build_with_mock_rpc()
            .set_genesis_state();
        let platform_state = platform.state.load();
        let (contender, _, dpns_contract) = create_dpns_identity_name_contest(
            &mut platform,
            &platform_state,
            7,
            NAME,
            platform_version,
        )
        .await;
        let fund_id = vote_poll(&dpns_contract)
            .specialized_balance_id()
            .expect("expected the poll's prefunded balance id");
        let (pro_tx_hash, identity, signer, voting_key) =
            setup_masternode_voting_identity(&mut platform, 29, platform_version);
        Contest {
            platform,
            dpns_contract,
            contender,
            fund_id,
            voter: Voter {
                pro_tx_hash,
                identity,
                signer,
                voting_key,
            },
        }
    }

    impl Contest {
        fn fund(&self, platform_version: &PlatformVersion) -> Option<u64> {
            self.platform
                .drive
                .fetch_prefunded_specialized_balance(
                    self.fund_id.to_buffer(),
                    None,
                    platform_version,
                )
                .expect("expected to fetch the poll's fund")
        }

        /// Deletes the poll's fund, as settling the poll does
        fn remove_fund(&self, platform_version: &PlatformVersion) {
            self.platform
                .drive
                .empty_prefunded_specialized_balance(self.fund_id, true, None, platform_version)
                .expect("expected to remove the poll's fund");
        }

        /// Leaves the poll's fund at `credits`
        fn set_fund(&self, credits: u64, platform_version: &PlatformVersion) {
            self.remove_fund(platform_version);
            self.platform
                .drive
                .add_prefunded_specialized_balance(self.fund_id, credits, None, platform_version)
                .expect("expected to fund the poll");
            assert_eq!(self.fund(platform_version), Some(credits));
        }

        /// Casts the voter's vote for the contender through block processing, as a proposer
        /// or a validator would, and returns how the block treated it
        async fn vote(
            &mut self,
            platform_version: &PlatformVersion,
        ) -> StateTransitionExecutionResult {
            self.vote_on(NAME, platform_version).await
        }

        /// The same vote on the poll of another name
        async fn vote_on(
            &mut self,
            name: &str,
            platform_version: &PlatformVersion,
        ) -> StateTransitionExecutionResult {
            let serialized_transition = self.serialized_vote(name, platform_version).await;
            let platform_state = self.platform.state.load();
            let transaction = self.platform.drive.grove.start_transaction();
            let processing_result = self
                .platform
                .platform
                .process_raw_state_transitions(
                    &[serialized_transition],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process the vote");
            self.platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit the transaction");
            processing_result.into_execution_results().remove(0)
        }

        /// The voter's signed vote for the contender on the poll of `name`, as broadcast
        async fn serialized_vote(&self, name: &str, platform_version: &PlatformVersion) -> Vec<u8> {
            let vote = Vote::ResourceVote(ResourceVote::V0(ResourceVoteV0 {
                vote_poll: VotePoll::ContestedDocumentResourceVotePoll(vote_poll_for(
                    &self.dpns_contract,
                    name,
                )),
                resource_vote_choice: ResourceVoteChoice::TowardsIdentity(self.contender.id()),
            }));
            MasternodeVoteTransition::try_from_vote_with_signer(
                vote,
                &self.voter.signer,
                self.voter.pro_tx_hash,
                &self.voter.voting_key,
                1,
                platform_version,
                None,
            )
            .await
            .expect("expected to make the vote")
            .serialize_to_bytes()
            .expect("expected to serialize the vote")
        }

        /// The voter's identity nonce: 0 until an executed vote bumps it
        fn voter_nonce(&self, platform_version: &PlatformVersion) -> Option<u64> {
            self.platform
                .drive
                .fetch_identity_nonce(
                    self.voter.identity.id().to_buffer(),
                    true,
                    None,
                    platform_version,
                )
                .expect("expected to fetch the voter's nonce")
        }
    }

    #[tokio::test]
    async fn should_refuse_a_vote_unpaid_when_the_poll_has_no_fund() {
        let platform_version = PlatformVersion::latest();
        let mut contest = contest_at(platform_version).await;
        contest.remove_fund(platform_version);

        let result = contest.vote(platform_version).await;

        assert_matches!(
            result,
            StateTransitionExecutionResult::UnpaidConsensusError(ConsensusError::StateError(
                StateError::PrefundedSpecializedBalanceNotFoundError(error)
            )) if *error.balance_id() == contest.fund_id
        );
        // Nothing of the vote reached the state
        assert_eq!(contest.voter_nonce(platform_version), Some(0));
        assert_eq!(contest.fund(platform_version), None);
    }

    fn single_vote_cost(platform_version: &PlatformVersion) -> u64 {
        platform_version
            .fee_version
            .vote_resolution_fund_fees
            .contested_document_single_vote_cost
    }

    /// The fund is one credit short of the single vote cost that executing the vote deducts.
    /// It still covers the vote's minimum fee, the smaller amount the v0 pre-check required,
    /// so under v0 the vote passed the pre-check and failed inside execution.
    #[tokio::test]
    async fn should_refuse_a_vote_unpaid_when_the_poll_fund_is_below_the_single_vote_cost() {
        let platform_version = PlatformVersion::latest();
        let mut contest = contest_at(platform_version).await;
        let single_vote_cost = single_vote_cost(platform_version);
        assert!(
            single_vote_cost
                > platform_version
                    .fee_version
                    .state_transition_min_fees
                    .masternode_vote,
            "the fund must satisfy the v0 threshold for this test to pin the v1 one"
        );
        contest.set_fund(single_vote_cost - 1, platform_version);

        let result = contest.vote(platform_version).await;

        assert_matches!(
            result,
            StateTransitionExecutionResult::UnpaidConsensusError(ConsensusError::StateError(
                StateError::PrefundedSpecializedBalanceInsufficientError(error)
            )) if *error.balance_id() == contest.fund_id
                && error.balance() == single_vote_cost - 1
                && error.required_balance() == single_vote_cost
        );
        assert_eq!(contest.voter_nonce(platform_version), Some(0));
        assert_eq!(contest.fund(platform_version), Some(single_vote_cost - 1));
    }

    #[tokio::test]
    async fn should_accept_a_vote_when_the_poll_fund_covers_exactly_the_single_vote_cost() {
        let platform_version = PlatformVersion::latest();
        let mut contest = contest_at(platform_version).await;
        let single_vote_cost = single_vote_cost(platform_version);
        contest.set_fund(single_vote_cost, platform_version);

        let result = contest.vote(platform_version).await;

        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        assert_eq!(contest.voter_nonce(platform_version), Some(1));
        assert_eq!(contest.fund(platform_version), Some(0));
    }

    /// A poll that never opened has no fund either; the voter is told about the poll, which is
    /// what state validation checks, not about the fund, which is checked after it.
    #[tokio::test]
    async fn should_report_a_missing_poll_rather_than_its_missing_fund() {
        let platform_version = PlatformVersion::latest();
        let mut contest = contest_at(platform_version).await;

        let result = contest.vote_on("nowhere", platform_version).await;

        assert_matches!(
            result,
            StateTransitionExecutionResult::UnpaidConsensusError(ConsensusError::StateError(
                StateError::VotePollNotFoundError(_)
            ))
        );
        assert_eq!(contest.voter_nonce(platform_version), Some(0));
    }

    /// v0, selected by every protocol version before 14, ignores the pre-check: the vote fails
    /// inside execution instead, as an internal error. A block never carries the vote under
    /// either generation, so the two agree on every block.
    #[tokio::test]
    async fn should_fail_a_vote_on_an_unfunded_poll_inside_execution_before_protocol_version_14() {
        let platform_version = PlatformVersion::get(13).expect("protocol version 13");
        let mut contest = contest_at(platform_version).await;
        contest.remove_fund(platform_version);

        let result = contest.vote(platform_version).await;

        assert_matches!(
            result,
            StateTransitionExecutionResult::InternalError(message)
                if message.contains("prefunded specialized balance does not exist")
        );
        assert_eq!(contest.voter_nonce(platform_version), Some(0));
        assert_eq!(contest.fund(platform_version), None);
    }
}
