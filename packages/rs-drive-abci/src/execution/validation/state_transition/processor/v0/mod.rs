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

pub(super) fn process_state_transition_v0<'a, C: CoreRPCLike>(
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

    // A masternode vote is paid by its vote poll's prefunded specialized balance, never by the
    // voter. When that fund does not exist or cannot cover the vote, nobody can be charged for
    // the vote, so it is refused unpaid: proposers strip it from their block and other
    // validators reject a block that carries it, exactly like a vote that fails its nonce check.
    // Until 4.2 the pre-check ran here but its result was never read, and such a vote failed
    // inside execution, when its cost was deducted, as an internal error; both outcomes keep the
    // vote out of every block and no chain ever held one, so acting on it is not versioned.
    if state_transition.uses_prefunded_specialized_balance_for_payment() {
        let result = state_transition.validate_minimum_prefunded_specialized_balance_pre_check(
            platform.drive,
            transaction,
            &mut state_transition_execution_context,
            platform_version,
        )?;

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
        create_dpns_identity_name_contest, dpns_name_vote_poll, serialized_dpns_name_vote,
        setup_masternode_voting_identity,
    };
    use crate::platform_types::state_transitions_processing_result::StateTransitionExecutionResult;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::{TempPlatform, TestPlatformBuilder};
    use assert_matches::assert_matches;
    use dpp::block::block_info::BlockInfo;
    use dpp::consensus::state::state_error::StateError;
    use dpp::consensus::ConsensusError;
    use dpp::identifier::Identifier;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::{Identity, IdentityPublicKey};
    use dpp::prelude::DataContract;
    use dpp::version::PlatformVersion;
    use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
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
        let fund_id = dpns_name_vote_poll(&dpns_contract, NAME)
            .specialized_balance_id()
            .expect("expected the poll's prefunded balance id");
        let voter = Voter::new(&mut platform, 29, platform_version);
        Contest {
            platform,
            dpns_contract,
            contender,
            fund_id,
            voter,
        }
    }

    impl Voter {
        fn new(
            platform: &mut TempPlatform<MockCoreRPCLike>,
            seed: u64,
            platform_version: &PlatformVersion,
        ) -> Self {
            let (pro_tx_hash, identity, signer, voting_key) =
                setup_masternode_voting_identity(platform, seed, platform_version);
            Voter {
                pro_tx_hash,
                identity,
                signer,
                voting_key,
            }
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

        /// Casts the voter's vote for the contender on the poll of `name` through block
        /// processing, as a proposer or a validator would, and returns how the block treated it
        async fn vote_on(
            &mut self,
            name: &str,
            platform_version: &PlatformVersion,
        ) -> StateTransitionExecutionResult {
            let serialized_transition = self
                .serialized_vote_by(&self.voter, name, platform_version)
                .await;
            self.process_block(vec![serialized_transition], platform_version)
                .remove(0)
        }

        /// Processes the transitions as one block and returns how the block treated each
        fn process_block(
            &mut self,
            serialized_transitions: Vec<Vec<u8>>,
            platform_version: &PlatformVersion,
        ) -> Vec<StateTransitionExecutionResult> {
            let platform_state = self.platform.state.load();
            let transaction = self.platform.drive.grove.start_transaction();
            let processing_result = self
                .platform
                .platform
                .process_raw_state_transitions(
                    &serialized_transitions,
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process the votes");
            self.platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit the transaction");
            processing_result.into_execution_results()
        }

        /// `voter`'s signed vote for the contender on the poll of `name`, as broadcast
        async fn serialized_vote_by(
            &self,
            voter: &Voter,
            name: &str,
            platform_version: &PlatformVersion,
        ) -> Vec<u8> {
            serialized_dpns_name_vote(
                &self.dpns_contract,
                ResourceVoteChoice::TowardsIdentity(self.contender.id()),
                name,
                &voter.signer,
                voter.pro_tx_hash,
                &voter.voting_key,
                1,
                platform_version,
            )
            .await
        }

        /// `voter`'s identity nonce: 0 until an executed vote bumps it
        fn nonce_of(&self, voter: &Voter, platform_version: &PlatformVersion) -> Option<u64> {
            self.platform
                .drive
                .fetch_identity_nonce(
                    voter.identity.id().to_buffer(),
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

        let result = contest.vote_on(NAME, platform_version).await;

        assert_matches!(
            result,
            StateTransitionExecutionResult::UnpaidConsensusError(ConsensusError::StateError(
                StateError::PrefundedSpecializedBalanceNotFoundError(error)
            )) if *error.balance_id() == contest.fund_id
        );
        // Nothing of the vote reached the state
        assert_eq!(contest.nonce_of(&contest.voter, platform_version), Some(0));
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

        let result = contest.vote_on(NAME, platform_version).await;

        assert_matches!(
            result,
            StateTransitionExecutionResult::UnpaidConsensusError(ConsensusError::StateError(
                StateError::PrefundedSpecializedBalanceInsufficientError(error)
            )) if *error.balance_id() == contest.fund_id
                && error.balance() == single_vote_cost - 1
                && error.required_balance() == single_vote_cost
        );
        assert_eq!(contest.nonce_of(&contest.voter, platform_version), Some(0));
        assert_eq!(contest.fund(platform_version), Some(single_vote_cost - 1));
    }

    #[tokio::test]
    async fn should_accept_a_vote_when_the_poll_fund_covers_exactly_the_single_vote_cost() {
        let platform_version = PlatformVersion::latest();
        let mut contest = contest_at(platform_version).await;
        let single_vote_cost = single_vote_cost(platform_version);
        contest.set_fund(single_vote_cost, platform_version);

        let result = contest.vote_on(NAME, platform_version).await;

        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        assert_eq!(contest.nonce_of(&contest.voter, platform_version), Some(1));
        assert_eq!(contest.fund(platform_version), Some(0));
    }

    /// The fund is read through the block's transaction: a fund that covers exactly one vote
    /// lets the first vote of a block in, and the second, whose pre-check sees that deduction,
    /// is refused for the credits the first one took.
    #[tokio::test]
    async fn should_refuse_the_second_vote_of_a_block_once_the_first_took_the_fund() {
        let platform_version = PlatformVersion::latest();
        let mut contest = contest_at(platform_version).await;
        let single_vote_cost = single_vote_cost(platform_version);
        contest.set_fund(single_vote_cost, platform_version);
        let second_voter = Voter::new(&mut contest.platform, 31, platform_version);
        let first_vote = contest
            .serialized_vote_by(&contest.voter, NAME, platform_version)
            .await;
        let second_vote = contest
            .serialized_vote_by(&second_voter, NAME, platform_version)
            .await;

        let results = contest.process_block(vec![first_vote, second_vote], platform_version);

        assert_matches!(
            results[0],
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        assert_matches!(
            &results[1],
            StateTransitionExecutionResult::UnpaidConsensusError(ConsensusError::StateError(
                StateError::PrefundedSpecializedBalanceInsufficientError(error)
            )) if *error.balance_id() == contest.fund_id
                && error.balance() == 0
                && error.required_balance() == single_vote_cost
        );
        assert_eq!(contest.nonce_of(&contest.voter, platform_version), Some(1));
        assert_eq!(contest.nonce_of(&second_voter, platform_version), Some(0));
        assert_eq!(contest.fund(platform_version), Some(0));
    }

    /// The refusal is not versioned: a chain still on protocol version 13 refuses the vote the
    /// same way, and it never entered a block there either.
    #[tokio::test]
    async fn should_refuse_a_vote_the_same_way_at_protocol_version_13() {
        let platform_version = PlatformVersion::get(13).expect("protocol version 13");
        let mut contest = contest_at(platform_version).await;
        contest.remove_fund(platform_version);

        let result = contest.vote_on(NAME, platform_version).await;

        assert_matches!(
            result,
            StateTransitionExecutionResult::UnpaidConsensusError(ConsensusError::StateError(
                StateError::PrefundedSpecializedBalanceNotFoundError(error)
            )) if *error.balance_id() == contest.fund_id
        );
        assert_eq!(contest.nonce_of(&contest.voter, platform_version), Some(0));
        assert_eq!(contest.fund(platform_version), None);
    }
}
