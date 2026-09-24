//! The contract fee claim through the whole processing pipeline: who may claim which pot, how a
//! pot is paid out, the once-per-epoch rule of each pot, and the proof of a claim's execution.

use crate::execution::check_tx::CheckTxLevel::FirstTimeCheck;
use crate::execution::validation::state_transition::state_transitions::contract_user_moderation::tests::{
    assert_paid_with_code, assert_success, assert_unpaid_with_code, Actor, CRITICAL_KEY_ID,
};
use crate::platform_types::platform::PlatformRef;
use crate::platform_types::platform_state::PlatformStateV0Methods;
use crate::platform_types::state_transitions_processing_result::StateTransitionExecutionResult;
use crate::rpc::core::MockCoreRPCLike;
use crate::test::helpers::setup::{TempPlatform, TestPlatformBuilder};
use dapi_grpc::platform::v0::get_documents_request::{
    GetDocumentsRequestV0, Version as GetDocumentsRequestVersion,
};
use dapi_grpc::platform::v0::GetDocumentsRequest;
use dpp::block::block_info::BlockInfo;
use dpp::block::epoch::{Epoch, EpochIndex};
use dpp::consensus::codes::ErrorWithCode;
use dpp::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
use dpp::data_contract::config::moderation::{ContractModerationConfig, ContractModerators};
use dpp::data_contract::document_type::action_fees::agreement::{
    AgreedFeeMultiplier, DocumentActionFeeAgreement,
};
use dpp::data_contract::document_type::action_fees::{ContractFeePot, ContractFeePotLastClaim};
use dpp::data_contract::document_type::random_document::{
    CreateRandomDocument, DocumentFieldFillSize, DocumentFieldFillType,
};
use dpp::data_contract::document_type::DocumentType;
use dpp::data_contract::DataContract;
use dpp::fee::fee_result::FeeResult;
use dpp::fee::Credits;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::platform_value::{platform_value, Bytes32, Identifier, Value};
use dpp::serialization::PlatformSerializable;
use dpp::state_transition::batch_transition::batched_transition::document_transition_action_type::DocumentTransitionActionType;
use dpp::state_transition::batch_transition::methods::v0::DocumentsBatchTransitionMethodsV0;
use dpp::state_transition::batch_transition::methods::StateTransitionCreationOptions;
use dpp::state_transition::batch_transition::BatchTransition;
use dpp::state_transition::contract_fee_claim_transition::methods::ContractFeeClaimTransitionMethodsV0;
use dpp::state_transition::contract_fee_claim_transition::ContractFeeClaimTransition;
use dpp::state_transition::data_contract_create_transition::methods::DataContractCreateTransitionMethodsV0;
use dpp::state_transition::data_contract_create_transition::DataContractCreateTransition;
use dpp::state_transition::data_contract_update_transition::methods::DataContractUpdateTransitionMethodsV0;
use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use dpp::state_transition::proof_result::{StateTransitionProofResult,
};
use dpp::state_transition::StateTransition;
use dpp::tests::fixtures::get_data_contract_fixture;
use dpp::version::PlatformVersion;
use drive::drive::contract::fee_pots::types::ContractFeePotState;
use drive::drive::Drive;
use drive::grovedb::Transaction;
use drive::util::batch::drive_op_batch::ContractFeePotOperationType;
use drive::util::batch::DriveOperation;
use rand::rngs::StdRng;
use rand::SeedableRng;
use std::collections::BTreeMap;

const DATA_CONTRACT_NOT_PRESENT: u32 = 10400;
const CONTRACT_FEES_ALREADY_CLAIMED_THIS_EPOCH: u32 = 41111;
const CONTRACT_FEES_NOTHING_TO_CLAIM: u32 = 41112;
const CONTRACT_FEE_CLAIM_NOT_ALLOWED: u32 = 41113;
const DOCUMENT_ACTION_FEES_WITHOUT_MODERATION: u32 = 10902;

/// The document type a contract update adds in these tests
const PAID_NOTE: &str = "paidNote";

/// What the shared `Actor` is funded with: 10 Dash
const STARTING_CREDITS: Credits = 1_000_000_000_000;

/// Which identities the contract appoints as its moderators
#[derive(Clone, Copy)]
enum Team {
    /// The contract declares no moderation at all
    NoModeration,
    /// Moderation with nobody appointed: the owner alone
    OwnerAlone,
    /// The two moderators, not the owner
    TwoModerators,
    /// The two moderators and the owner
    TwoModeratorsAndTheOwner,
}

/// A platform with a contract owner, two moderators and a stranger, and one contract the owner
/// created with the given team.
struct Setup {
    platform: TempPlatform<MockCoreRPCLike>,
    owner: Actor,
    moderator_a: Actor,
    moderator_b: Actor,
    stranger: Actor,
    contract: DataContract,
}

impl Setup {
    async fn new(team: Team) -> Self {
        Self::new_at(team, PlatformVersion::latest()).await
    }

    async fn new_at(team: Team, platform_version: &PlatformVersion) -> Self {
        let mut platform = TestPlatformBuilder::new()
            .with_initial_protocol_version(platform_version.protocol_version)
            .build_with_mock_rpc()
            .set_genesis_state();
        let owner = Actor::new(&mut platform, 21);
        let moderator_a = Actor::new(&mut platform, 22);
        let moderator_b = Actor::new(&mut platform, 23);
        let stranger = Actor::new(&mut platform, 24);

        let identity_nonce = 1;
        let mut contract = get_data_contract_fixture(
            Some(owner.id()),
            identity_nonce,
            platform_version.protocol_version,
        )
        .data_contract_owned();
        let moderators = match team {
            Team::NoModeration => None,
            Team::OwnerAlone => Some(ContractModerators::ContractOwner),
            Team::TwoModerators => Some(ContractModerators::AppointedModerators(
                [moderator_a.id(), moderator_b.id()].into(),
            )),
            Team::TwoModeratorsAndTheOwner => Some(ContractModerators::AppointedModerators(
                [moderator_a.id(), moderator_b.id(), owner.id()].into(),
            )),
        };
        contract.set_config(contract.config().clone().with_moderation(moderators.map(
            |moderators| ContractModerationConfig {
                banlist: true,
                suspensions: false,
                moderators,
                warnings: false,
            },
        )));

        let setup = Self {
            platform,
            owner,
            moderator_a,
            moderator_b,
            stranger,
            contract,
        };

        let create = DataContractCreateTransition::new_from_data_contract(
            setup.contract.clone(),
            identity_nonce,
            &setup.owner.identity.clone().into_partial_identity_info(),
            CRITICAL_KEY_ID,
            &setup.owner.signer,
            platform_version,
            None,
        )
        .await
        .expect("expected to build the contract create");
        let transaction = setup.platform.drive.grove.start_transaction();
        assert_success(&setup.process(&create, 0, &transaction));
        setup.commit(transaction);
        // The create sets the owner's nonce for the contract to the identity nonce it used.
        setup.owner.next_contract_nonce.set(identity_nonce + 1);
        setup
    }

    /// Puts `credits` in `pot`, as document action fees would
    fn fill(&self, pot: ContractFeePot, credits: Credits) {
        self.platform
            .drive
            .apply_drive_operations(
                vec![DriveOperation::ContractFeePotOperation(
                    ContractFeePotOperationType::AddToPot {
                        contract_id: self.contract.id(),
                        pot,
                        amount: credits,
                    },
                )],
                true,
                &BlockInfo::default(),
                None,
                PlatformVersion::latest(),
                None,
            )
            .expect("expected to fill the pot");
    }

    /// The contract at version 2, with a `paidNote` document type that charges `action_fees`,
    /// and the update that carries it, signed by the owner
    async fn add_paid_note(&self, action_fees: Value) -> (DataContract, StateTransition) {
        let platform_version = PlatformVersion::latest();
        let mut updated = self.contract.clone();
        updated.set_version(2);
        let paid_note = DocumentType::try_from_schema(
            updated.id(),
            1,
            updated.config().version(),
            PAID_NOTE,
            platform_value!({
                "type": "object",
                "properties": {
                    "text": {"type": "string", "position": 0, "maxLength": 60_u32},
                },
                "required": ["text"],
                "actionFees": action_fees,
                "additionalProperties": false,
            }),
            None,
            &BTreeMap::new(),
            updated.config(),
            true,
            &mut vec![],
            platform_version,
        )
        .expect("expected the paid note document type to parse");
        updated
            .document_types_mut()
            .insert(PAID_NOTE.to_string(), paid_note);

        let update = DataContractUpdateTransition::new_from_data_contract(
            updated.clone(),
            &self.owner.identity.clone().into_partial_identity_info(),
            CRITICAL_KEY_ID,
            self.owner.contract_nonce(),
            0,
            &self.owner.signer,
            platform_version,
            None,
        )
        .await
        .expect("expected to build the contract update");
        (updated, update)
    }

    /// A claim of `pot` by `actor`, signed with its CRITICAL key
    async fn claim(&self, actor: &Actor, pot: ContractFeePot) -> StateTransition {
        claim_of(actor, self.contract.id(), pot).await
    }

    fn process(
        &self,
        transition: &StateTransition,
        epoch_index: EpochIndex,
        transaction: &Transaction,
    ) -> StateTransitionExecutionResult {
        let state = self.platform.state.load();
        let platform_version = state.current_platform_version().expect("version");
        let result = self
            .platform
            .platform
            .process_raw_state_transitions(
                &vec![transition
                    .serialize_to_bytes()
                    .expect("expected to serialize")],
                &state,
                &BlockInfo {
                    time_ms: block_time_of(epoch_index),
                    epoch: Epoch::new(epoch_index).expect("expected an epoch"),
                    ..Default::default()
                },
                transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process the state transition");
        result.execution_results()[0].clone()
    }

    fn commit(&self, transaction: Transaction) {
        self.platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit");
    }

    fn check_tx_codes(&self, transition: &StateTransition) -> Vec<u32> {
        let version = PlatformVersion::latest();
        let state = self.platform.state.load();
        let platform_ref = PlatformRef {
            drive: &self.platform.drive,
            state: &state,
            config: &self.platform.config,
            core_rpc: &self.platform.core_rpc,
        };
        let raw = transition
            .serialize_to_bytes()
            .expect("expected to serialize");
        self.platform
            .check_tx(&raw, FirstTimeCheck, &platform_ref, version)
            .expect("expected to check tx")
            .errors
            .iter()
            .map(|error| error.code())
            .collect()
    }

    /// Queries the contract's documents as a client's getDocuments request does: against
    /// committed state, caching the contract it pulls
    fn query_documents(&self) {
        let state = self.platform.state.load();
        let request = GetDocumentsRequest {
            version: Some(GetDocumentsRequestVersion::V0(GetDocumentsRequestV0 {
                data_contract_id: self.contract.id().to_vec(),
                document_type: "niceDocument".to_string(),
                r#where: vec![],
                limit: 0,
                order_by: vec![],
                prove: false,
                start: None,
            })),
        };
        let result = self
            .platform
            .query_documents(request, &state, PlatformVersion::latest())
            .expect("expected to query the documents");
        assert!(result.is_valid(), "{:?}", result.errors);
    }

    fn pot(&self, pot: ContractFeePot, transaction: Option<&Transaction>) -> ContractFeePotState {
        self.platform
            .drive
            .fetch_contract_fee_pot(
                self.contract.id(),
                pot,
                transaction,
                PlatformVersion::latest(),
            )
            .expect("expected to fetch the pot")
    }

    fn credits(&self, actor: &Actor, transaction: Option<&Transaction>) -> Credits {
        self.platform
            .drive
            .fetch_identity_balance(
                actor.id().to_buffer(),
                transaction,
                PlatformVersion::latest(),
            )
            .expect("expected to fetch the balance")
            .expect("expected a balance")
    }

    fn credits_in_trees(&self, transaction: &Transaction) -> Credits {
        self.platform
            .drive
            .calculate_total_credits_balance(Some(transaction), &PlatformVersion::latest().drive)
            .expect("expected to sum the credits")
            .total_in_trees()
            .expect("expected the credits to add up")
    }
}

async fn claim_of(actor: &Actor, contract_id: Identifier, pot: ContractFeePot) -> StateTransition {
    ContractFeeClaimTransition::try_from_identity_with_signer(
        &actor.identity,
        &CRITICAL_KEY_ID,
        contract_id,
        pot,
        actor.contract_nonce(),
        0,
        &actor.signer,
        PlatformVersion::latest(),
        None,
    )
    .await
    .expect("expected to build the claim")
}

/// The time of the block `Setup::process` executes a transition in, one per epoch.
fn block_time_of(epoch_index: EpochIndex) -> u64 {
    1_700_000_000_000 + u64::from(epoch_index) * 1_000
}

/// What a claim by `claimant`, processed in `epoch_index`, leaves as the pot's last claim.
fn claim_by(claimant: &Actor, epoch_index: EpochIndex) -> Option<ContractFeePotLastClaim> {
    Some(ContractFeePotLastClaim {
        epoch_index,
        time_ms: block_time_of(epoch_index),
        claimant_id: claimant.identity.id(),
    })
}

/// What `execution` was billed, whether it succeeded or was a paid refusal
fn fees(execution: &StateTransitionExecutionResult) -> &FeeResult {
    match execution {
        StateTransitionExecutionResult::SuccessfulExecution { fee_result, .. } => fee_result,
        StateTransitionExecutionResult::PaidConsensusError { actual_fees, .. } => actual_fees,
        other => panic!("expected a paid result, got {other:?}"),
    }
}

fn gas(execution: &StateTransitionExecutionResult) -> Credits {
    fees(execution).total_base_fee()
}

#[tokio::test]
async fn should_split_the_moderators_pot_equally_and_leave_the_remainder() {
    let setup = Setup::new(Team::TwoModerators).await;
    setup.fill(ContractFeePot::Moderators, 1_001);
    let claim = setup
        .claim(&setup.moderator_a, ContractFeePot::Moderators)
        .await;
    assert_eq!(setup.check_tx_codes(&claim), Vec::<u32>::new());
    let transaction = setup.platform.drive.grove.start_transaction();
    let credits_before = setup.credits_in_trees(&transaction);

    let result = setup.process(&claim, 3, &transaction);

    assert_success(&result);
    let gas = gas(&result);
    // The claimant pays for the claim and is paid a share like the other moderator.
    assert_eq!(
        setup.credits(&setup.moderator_a, Some(&transaction)),
        STARTING_CREDITS - gas + 500
    );
    assert_eq!(
        setup.credits(&setup.moderator_b, Some(&transaction)),
        STARTING_CREDITS + 500
    );
    // The owner is not appointed: they are not on the team.
    assert_eq!(
        setup.credits(&setup.owner, Some(&transaction)),
        setup.credits(&setup.owner, None)
    );
    assert_eq!(
        setup.pot(ContractFeePot::Moderators, Some(&transaction)),
        ContractFeePotState {
            credits: 1,
            last_claim: claim_by(&setup.moderator_a, 3),
        }
    );
    // The pot only moved into balances: what left the trees is the gas of the claim.
    assert_eq!(credits_before - setup.credits_in_trees(&transaction), gas);
}

#[tokio::test]
async fn should_pay_an_appointed_owner_a_share_of_the_moderators_pot() {
    let setup = Setup::new(Team::TwoModeratorsAndTheOwner).await;
    setup.fill(ContractFeePot::Moderators, 900);
    let owner_credits = setup.credits(&setup.owner, None);
    let claim = setup.claim(&setup.owner, ContractFeePot::Moderators).await;
    let transaction = setup.platform.drive.grove.start_transaction();

    let result = setup.process(&claim, 0, &transaction);

    assert_success(&result);
    assert_eq!(
        setup.credits(&setup.owner, Some(&transaction)),
        owner_credits - gas(&result) + 300
    );
    for moderator in [&setup.moderator_a, &setup.moderator_b] {
        assert_eq!(
            setup.credits(moderator, Some(&transaction)),
            STARTING_CREDITS + 300
        );
    }
    assert_eq!(
        setup
            .pot(ContractFeePot::Moderators, Some(&transaction))
            .credits,
        0
    );
}

#[tokio::test]
async fn should_let_only_a_member_of_the_team_claim_the_moderators_pot() {
    let setup = Setup::new(Team::TwoModerators).await;
    setup.fill(ContractFeePot::Moderators, 1_000);

    // Neither a stranger nor the owner, who is not appointed, is on the team.
    for outsider in [&setup.stranger, &setup.owner] {
        let claim = setup.claim(outsider, ContractFeePot::Moderators).await;
        assert_eq!(
            setup.check_tx_codes(&claim),
            vec![CONTRACT_FEE_CLAIM_NOT_ALLOWED]
        );
        let transaction = setup.platform.drive.grove.start_transaction();
        let result = setup.process(&claim, 0, &transaction);
        assert_paid_with_code(&result, CONTRACT_FEE_CLAIM_NOT_ALLOWED);
        assert_eq!(
            setup.pot(ContractFeePot::Moderators, Some(&transaction)),
            ContractFeePotState {
                credits: 1_000,
                last_claim: None,
            },
            "a refused claim pays nothing out and does not use up the epoch"
        );
    }
}

#[tokio::test]
async fn should_let_nobody_but_the_owner_claim_when_nobody_is_appointed() {
    let setup = Setup::new(Team::OwnerAlone).await;
    setup.fill(ContractFeePot::Moderators, 700);
    let owner_credits = setup.credits(&setup.owner, None);

    let claim = setup
        .claim(&setup.moderator_a, ContractFeePot::Moderators)
        .await;
    let transaction = setup.platform.drive.grove.start_transaction();
    assert_paid_with_code(
        &setup.process(&claim, 0, &transaction),
        CONTRACT_FEE_CLAIM_NOT_ALLOWED,
    );

    let claim = setup.claim(&setup.owner, ContractFeePot::Moderators).await;
    let result = setup.process(&claim, 0, &transaction);
    assert_success(&result);
    assert_eq!(
        setup.credits(&setup.owner, Some(&transaction)),
        owner_credits - gas(&result) + 700
    );
}

#[tokio::test]
async fn should_pay_out_a_pot_at_most_once_per_epoch() {
    let setup = Setup::new(Team::TwoModerators).await;
    setup.fill(ContractFeePot::Moderators, 1_000);
    let transaction = setup.platform.drive.grove.start_transaction();

    let first = setup
        .claim(&setup.moderator_a, ContractFeePot::Moderators)
        .await;
    assert_success(&setup.process(&first, 5, &transaction));

    // More fees come in during the epoch; the other moderator cannot pay them out yet, and the
    // refusal is paid for.
    setup.commit(transaction);
    setup.fill(ContractFeePot::Moderators, 400);
    let transaction = setup.platform.drive.grove.start_transaction();
    let second = setup
        .claim(&setup.moderator_b, ContractFeePot::Moderators)
        .await;
    let credits_before = setup.credits(&setup.moderator_b, Some(&transaction));
    let result = setup.process(&second, 5, &transaction);
    assert_paid_with_code(&result, CONTRACT_FEES_ALREADY_CLAIMED_THIS_EPOCH);
    assert_eq!(
        setup.credits(&setup.moderator_b, Some(&transaction)),
        credits_before - gas(&result)
    );
    assert_eq!(
        setup
            .pot(ContractFeePot::Moderators, Some(&transaction))
            .credits,
        400
    );

    // The next epoch it is paid out again.
    let third = setup
        .claim(&setup.moderator_b, ContractFeePot::Moderators)
        .await;
    assert_success(&setup.process(&third, 6, &transaction));
    assert_eq!(
        setup.pot(ContractFeePot::Moderators, Some(&transaction)),
        ContractFeePotState {
            credits: 0,
            // The last claim names the member that made it, not the one of the epoch before.
            last_claim: claim_by(&setup.moderator_b, 6),
        }
    );
}

#[tokio::test]
async fn should_refuse_a_pot_that_holds_less_than_a_credit_for_each_recipient() {
    let setup = Setup::new(Team::TwoModerators).await;
    let claim = setup
        .claim(&setup.moderator_a, ContractFeePot::Moderators)
        .await;
    assert_eq!(
        setup.check_tx_codes(&claim),
        vec![CONTRACT_FEES_NOTHING_TO_CLAIM]
    );

    // One credit between two moderators rounds down to nothing each.
    setup.fill(ContractFeePot::Moderators, 1);
    let transaction = setup.platform.drive.grove.start_transaction();
    assert_paid_with_code(
        &setup.process(&claim, 2, &transaction),
        CONTRACT_FEES_NOTHING_TO_CLAIM,
    );
    assert_eq!(
        setup.pot(ContractFeePot::Moderators, Some(&transaction)),
        ContractFeePotState {
            credits: 1,
            last_claim: None,
        }
    );
}

#[tokio::test]
async fn should_pay_the_owner_pot_to_the_owner_alone_and_on_its_own_epoch_clock() {
    let setup = Setup::new(Team::TwoModerators).await;
    setup.fill(ContractFeePot::Owner, 5_000);
    setup.fill(ContractFeePot::Moderators, 1_000);
    let owner_credits = setup.credits(&setup.owner, None);
    let transaction = setup.platform.drive.grove.start_transaction();

    // A moderator is no recipient of the owner pot.
    let by_a_moderator = setup.claim(&setup.moderator_a, ContractFeePot::Owner).await;
    assert_paid_with_code(
        &setup.process(&by_a_moderator, 4, &transaction),
        CONTRACT_FEE_CLAIM_NOT_ALLOWED,
    );

    let by_the_owner = setup.claim(&setup.owner, ContractFeePot::Owner).await;
    let result = setup.process(&by_the_owner, 4, &transaction);
    assert_success(&result);
    assert_eq!(
        setup.credits(&setup.owner, Some(&transaction)),
        owner_credits - gas(&result) + 5_000
    );
    assert_eq!(
        setup.pot(ContractFeePot::Owner, Some(&transaction)),
        ContractFeePotState {
            credits: 0,
            last_claim: claim_by(&setup.owner, 4),
        }
    );

    // The owner's claim did not use up the team's claim of this epoch.
    assert_eq!(
        setup
            .pot(ContractFeePot::Moderators, Some(&transaction))
            .last_claim,
        None
    );
    let by_the_team = setup
        .claim(&setup.moderator_a, ContractFeePot::Moderators)
        .await;
    assert_success(&setup.process(&by_the_team, 4, &transaction));

    // And the owner pot is paid out once per epoch like the other.
    setup.commit(transaction);
    setup.fill(ContractFeePot::Owner, 10);
    let transaction = setup.platform.drive.grove.start_transaction();
    let again = setup.claim(&setup.owner, ContractFeePot::Owner).await;
    assert_paid_with_code(
        &setup.process(&again, 4, &transaction),
        CONTRACT_FEES_ALREADY_CLAIMED_THIS_EPOCH,
    );
}

#[tokio::test]
async fn should_leave_the_moderators_pot_of_an_unmoderated_contract_to_nobody() {
    let setup = Setup::new(Team::NoModeration).await;
    setup.fill(ContractFeePot::Owner, 100);
    let transaction = setup.platform.drive.grove.start_transaction();

    // No moderation, no team: not even the owner is a recipient of the moderators pot.
    let claim = setup.claim(&setup.owner, ContractFeePot::Moderators).await;
    assert_paid_with_code(
        &setup.process(&claim, 0, &transaction),
        CONTRACT_FEE_CLAIM_NOT_ALLOWED,
    );

    let claim = setup.claim(&setup.owner, ContractFeePot::Owner).await;
    assert_success(&setup.process(&claim, 0, &transaction));
}

#[tokio::test]
async fn should_refuse_a_claim_on_an_unknown_contract_and_charge_for_the_lookup() {
    let setup = Setup::new(Team::TwoModerators).await;
    let unknown_contract_id = Identifier::from([0x55; 32]);
    let nonce = setup.moderator_a.next_contract_nonce.get();
    let claim = claim_of(
        &setup.moderator_a,
        unknown_contract_id,
        ContractFeePot::Moderators,
    )
    .await;
    assert_eq!(
        setup.check_tx_codes(&claim),
        vec![DATA_CONTRACT_NOT_PRESENT]
    );
    let transaction = setup.platform.drive.grove.start_transaction();
    let credits_before = setup.credits(&setup.moderator_a, Some(&transaction));

    let result = setup.process(&claim, 0, &transaction);

    // Paid like every other refusal: the signer is authenticated and the lookup happened.
    assert_paid_with_code(&result, DATA_CONTRACT_NOT_PRESENT);
    let gas = gas(&result);
    assert!(gas > 0, "the contract lookup is billed");
    assert_eq!(
        setup.credits(&setup.moderator_a, Some(&transaction)),
        credits_before - gas
    );
    // The refusal used up the claimant's nonce for that contract id.
    assert_eq!(
        setup
            .platform
            .drive
            .fetch_identity_contract_nonce(
                setup.moderator_a.id().to_buffer(),
                unknown_contract_id.to_buffer(),
                true,
                Some(&transaction),
                PlatformVersion::latest(),
            )
            .expect("expected to fetch the nonce"),
        Some(nonce)
    );
}

#[tokio::test]
async fn should_bill_a_claim_the_same_whether_its_contract_is_cached_or_not() {
    let setup = Setup::new(Team::TwoModerators).await;
    setup.fill(ContractFeePot::Moderators, 1_000);
    let claim = setup
        .claim(&setup.moderator_a, ContractFeePot::Moderators)
        .await;
    let contracts = &setup.platform.drive.cache.data_contracts;
    let contract_id = setup.contract.id().to_buffer();
    let claim_fees = || {
        let transaction = setup.platform.drive.grove.start_transaction();
        let result = setup.process(&claim, 1, &transaction);
        assert_success(&result);
        fees(&result).clone()
    };

    // A node that executed the contract create: the cache refresh after the write stored the
    // contract without a fee.
    let cached = contracts
        .get(contract_id, true)
        .expect("expected the create to cache the contract");
    assert!(!cached.has_fee_for_tests());
    let as_the_create_left_it = claim_fees();

    // A node whose committed cache a getDocuments query filled, also without a fee, which
    // anyone can make happen on the nodes of their choosing.
    contracts.merge_and_clear_block_cache();
    contracts.clear();
    setup.query_documents();
    let cached = contracts
        .get(contract_id, true)
        .expect("expected the query to cache the contract");
    assert!(!cached.has_fee_for_tests());
    let after_a_query = claim_fees();

    // A node that cached the contract with the fee of its read, in an earlier epoch, as the
    // validation of a contract update does.
    contracts.clear();
    setup
        .platform
        .drive
        .get_contract_with_fetch_info_and_fee(
            contract_id,
            Some(&Epoch::new(0).expect("expected an epoch")),
            true,
            None,
            PlatformVersion::latest(),
        )
        .expect("expected to read the contract");
    let cached = contracts
        .get(contract_id, true)
        .expect("expected the read to cache the contract");
    assert!(cached.has_fee_for_tests());
    let with_a_fee = claim_fees();

    // A node that restarted, evicted the contract or joined late.
    contracts.clear();
    assert!(contracts.get(contract_id, true).is_none());
    let cold = claim_fees();

    assert_eq!(as_the_create_left_it, cold);
    assert_eq!(after_a_query, cold);
    assert_eq!(with_a_fee, cold);
}

#[tokio::test]
async fn should_prove_the_pot_and_the_balances_of_everyone_it_paid() {
    let setup = Setup::new(Team::TwoModerators).await;
    setup.fill(ContractFeePot::Moderators, 1_001);
    let claim = setup
        .claim(&setup.moderator_a, ContractFeePot::Moderators)
        .await;
    let transaction = setup.platform.drive.grove.start_transaction();
    assert_success(&setup.process(&claim, 9, &transaction));
    setup.commit(transaction);

    let platform_version = PlatformVersion::latest();
    let proof = setup
        .platform
        .drive
        .prove_state_transition(&claim, None, platform_version)
        .expect("expected to prove the state transition")
        .into_data()
        .expect("expected proof bytes");
    // The recipients are read from the contract, as a client holding the contract does.
    let known_contracts: BTreeMap<Identifier, DataContract> =
        BTreeMap::from([(setup.contract.id(), setup.contract.clone())]);
    let (_, outcome) = Drive::verify_state_transition_was_executed_with_proof(
        &claim,
        &BlockInfo::default(),
        &proof,
        &|id| Ok(known_contracts.get(id).cloned().map(std::sync::Arc::new)),
        platform_version,
    )
    .expect("expected the proof to verify");

    assert!(
        !outcome.is_execution_proved(),
        "expected AffectedState, got {:?}",
        outcome
    );

    let StateTransitionProofResult::VerifiedContractFeeClaim(
        contract_id,
        pot,
        last_claim,
        remaining,
        balances,
    ) = outcome.into_result()
    else {
        panic!("expected a contract fee claim result");
    };
    assert_eq!(contract_id, setup.contract.id());
    assert_eq!(pot, ContractFeePot::Moderators);
    assert_eq!(Some(last_claim), claim_by(&setup.moderator_a, 9));
    assert_eq!(remaining, 1);
    assert_eq!(
        balances,
        BTreeMap::from([
            (
                setup.moderator_a.id(),
                setup.credits(&setup.moderator_a, None)
            ),
            (setup.moderator_b.id(), STARTING_CREDITS + 500),
        ])
    );
}

#[tokio::test]
async fn should_not_verify_a_claim_that_never_executed() {
    let setup = Setup::new(Team::TwoModerators).await;
    setup.fill(ContractFeePot::Moderators, 1_001);
    let claim = setup
        .claim(&setup.moderator_a, ContractFeePot::Moderators)
        .await;

    let platform_version = PlatformVersion::latest();
    let proof = setup
        .platform
        .drive
        .prove_state_transition(&claim, None, platform_version)
        .expect("expected to prove the state transition")
        .into_data()
        .expect("expected proof bytes");
    let known_contracts: BTreeMap<Identifier, DataContract> =
        BTreeMap::from([(setup.contract.id(), setup.contract.clone())]);
    let result = Drive::verify_state_transition_was_executed_with_proof(
        &claim,
        &BlockInfo::default(),
        &proof,
        &|id| Ok(known_contracts.get(id).cloned().map(std::sync::Arc::new)),
        platform_version,
    );
    assert!(
        result.is_err(),
        "a pot that was never claimed proves no claim"
    );
}

#[tokio::test]
async fn should_not_be_active_before_protocol_version_14() {
    let platform_version = PlatformVersion::get(13).expect("protocol version 13");
    let setup = Setup::new_at(Team::NoModeration, platform_version).await;
    let transaction = setup.platform.drive.grove.start_transaction();
    let claim = setup.claim(&setup.owner, ContractFeePot::Owner).await;
    let execution = setup.process(&claim, 0, &transaction);
    // The transition type does not exist before protocol version 14, so the node cannot even
    // decode it as active: the same refusal every transition added since the fork gets.
    assert!(
        matches!(
            &execution,
            StateTransitionExecutionResult::InternalError(message)
                if message.contains("ContractFeeClaim") && message.contains("not active")
        ),
        "expected the transition to be inactive before protocol version 14, got {execution:?}"
    );
}

#[tokio::test]
async fn should_let_a_live_contract_gain_fees_through_a_new_document_type() {
    // The fees of a document type never change, so this is how a contract that is already
    // published starts to charge: an update adds a document type that declares them.
    let setup = Setup::new(Team::TwoModerators).await;
    let (updated, update) = setup
        .add_paid_note(platform_value!({
            "pricing": "fixed",
            "create": {"owner": 700_u64, "moderators": 300_u64},
        }))
        .await;
    let transaction = setup.platform.drive.grove.start_transaction();
    assert_success(&setup.process(&update, 0, &transaction));

    // A stranger writes a paid note and fills both pots.
    let platform_version = PlatformVersion::latest();
    let paid_note = updated
        .document_type_for_name(PAID_NOTE)
        .expect("expected the paid note document type");
    let mut rng = StdRng::seed_from_u64(7);
    let entropy = Bytes32::random_with_rng(&mut rng);
    let mut document = paid_note
        .random_document_with_identifier_and_entropy(
            &mut rng,
            setup.stranger.id(),
            entropy,
            DocumentFieldFillType::FillIfNotRequired,
            DocumentFieldFillSize::MinDocumentFillSize,
            platform_version,
        )
        .expect("expected a random paid note");
    let creation_nonce = setup.stranger.contract_nonce();
    document
        .set_id_for_creation(paid_note, &entropy.0, creation_nonce, platform_version)
        .expect("expected to set the document id");
    let creation = BatchTransition::new_document_creation_transition_from_document(
        document,
        paid_note,
        entropy.0,
        &setup.stranger.key,
        creation_nonce,
        0,
        None,
        &setup.stranger.signer,
        platform_version,
        // The stranger agrees to the fee the new document type declares: it is fixed, so the
        // agreement names no fee multiplier whatever is known.
        Some(StateTransitionCreationOptions {
            action_fee_agreement: DocumentActionFeeAgreement::for_document_type_action(
                paid_note,
                DocumentTransitionActionType::Create,
                AgreedFeeMultiplier {
                    known_permille: 1_000,
                    increase_tolerance_percent: 0,
                },
            ),
            ..Default::default()
        }),
    )
    .await
    .expect("expected to build the paid note creation");
    assert_success(&setup.process(&creation, 0, &transaction));
    assert_eq!(
        setup.pot(ContractFeePot::Owner, Some(&transaction)).credits,
        700
    );
    assert_eq!(
        setup
            .pot(ContractFeePot::Moderators, Some(&transaction))
            .credits,
        300
    );

    // And the team is paid out of what the new document type collected.
    let claim = setup
        .claim(&setup.moderator_b, ContractFeePot::Moderators)
        .await;
    assert_success(&setup.process(&claim, 0, &transaction));
    assert_eq!(
        setup.credits(&setup.moderator_a, Some(&transaction)),
        STARTING_CREDITS + 150
    );
}

#[tokio::test]
async fn should_refuse_an_update_adding_a_moderators_fee_to_an_unmoderated_contract() {
    let setup = Setup::new(Team::NoModeration).await;
    let (_, update) = setup
        .add_paid_note(platform_value!({"create": {"owner": 700_u64, "moderators": 300_u64}}))
        .await;
    let transaction = setup.platform.drive.grove.start_transaction();
    assert_unpaid_with_code(
        &setup.process(&update, 0, &transaction),
        DOCUMENT_ACTION_FEES_WITHOUT_MODERATION,
    );

    // An owner fee needs no team, and the same update without the moderators part goes through.
    let (_, update) = setup
        .add_paid_note(platform_value!({"create": {"owner": 700_u64}}))
        .await;
    assert_success(&setup.process(&update, 0, &transaction));
}
