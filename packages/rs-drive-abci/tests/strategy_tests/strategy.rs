use crate::masternodes::MasternodeListItemWithUpdates;
use crate::query::QueryStrategy;
use crate::BlockHeight;

use dashcore_rpc::dashcore::{ProTxHash, QuorumHash};
use dpp::block::block_info::BlockInfo;

use dpp::data_contract::document_type::random_document::CreateRandomDocument;
use dpp::data_contract::DataContract;
use strategy_tests::frequency::Frequency;
use strategy_tests::operations::FinalizeBlockOperation::IdentityAddKeys;
use strategy_tests::operations::{
    DocumentAction, DocumentOp, FinalizeBlockOperation, IdentityUpdateOp, OperationType,
};

use dpp::document::DocumentV0Getters;
use dpp::fee::Credits;
use dpp::identity::{Identity, KeyType, Purpose, SecurityLevel};
use dpp::serialization::PlatformSerializableWithPlatformVersion;
use dpp::state_transition::data_contract_create_transition::DataContractCreateTransition;
use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use dpp::state_transition::StateTransition;
use dpp::util::deserializer::ProtocolVersion;
use dpp::version::PlatformVersion;
use drive::drive::flags::StorageFlags::SingleEpoch;
use drive::drive::identity::key::fetch::{IdentityKeysRequest, KeyRequestType};
use drive::drive::Drive;

use dpp::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
use dpp::state_transition::data_contract_update_transition::methods::DataContractUpdateTransitionMethodsV0;
use drive::query::DriveQuery;
use drive_abci::abci::AbciApplication;
use drive_abci::mimic::test_quorum::TestQuorumInfo;
use drive_abci::platform_types::platform::Platform;
use drive_abci::rpc::core::MockCoreRPCLike;
use rand::prelude::{IteratorRandom, SliceRandom, StdRng};
use rand::Rng;
use strategy_tests::Strategy;
use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::ops::AddAssign;
use tenderdash_abci::proto::abci::{ExecTxResult, ValidatorSetUpdate};
use dpp::data_contract::document_type::accessors::{DocumentTypeV0Getters};
use dpp::identity::accessors::IdentityGettersV0;
use dpp::platform_value::BinaryData;
use dpp::prelude::{Identifier, IdentityNonce};
use dpp::state_transition::documents_batch_transition::document_base_transition::v0::DocumentBaseTransitionV0;
use dpp::state_transition::documents_batch_transition::document_create_transition::{DocumentCreateTransition, DocumentCreateTransitionV0};
use dpp::state_transition::documents_batch_transition::document_transition::document_delete_transition::DocumentDeleteTransitionV0;
use dpp::state_transition::documents_batch_transition::document_transition::document_replace_transition::DocumentReplaceTransitionV0;
use dpp::state_transition::documents_batch_transition::{DocumentsBatchTransition, DocumentsBatchTransitionV0};
use dpp::state_transition::documents_batch_transition::document_transition::{DocumentDeleteTransition, DocumentReplaceTransition};
use drive::drive::document::query::QueryDocumentsOutcomeV0Methods;
use dpp::state_transition::data_contract_create_transition::methods::v0::DataContractCreateTransitionMethodsV0;
use drive_abci::platform_types::withdrawal::unsigned_withdrawal_txs::v0::UnsignedWithdrawalTxs;

use crate::strategy::CoreHeightIncrease::NoCoreHeightIncrease;
use simple_signer::signer::SimpleSigner;

#[derive(Clone, Debug, Default)]
pub struct MasternodeListChangesStrategy {
    /// How many new hpmns on average per core chain lock increase
    pub new_hpmns: Frequency,
    /// How many hpmns leave the system
    pub removed_hpmns: Frequency,
    /// How many hpmns update a key
    pub updated_hpmns: Frequency,
    /// How many hpmns are banned
    pub banned_hpmns: Frequency,
    /// How many hpmns are unbanned
    pub unbanned_hpmns: Frequency,
    /// How many hpmns changed ips
    pub changed_ip_hpmns: Frequency,
    /// How many hpmns changed their p2p port
    pub changed_p2p_port_hpmns: Frequency,
    /// How many hpmns changed their http port
    pub changed_http_port_hpmns: Frequency,
    /// How many new masternodes on average per core chain lock increase
    pub new_masternodes: Frequency,
    /// How many masternodes leave the system
    pub removed_masternodes: Frequency,
    /// How many masternodes update a key
    pub updated_masternodes: Frequency,
    /// How many masternodes are banned
    pub banned_masternodes: Frequency,
    /// How many masternodes are unbanned
    pub unbanned_masternodes: Frequency,
    /// How many masternodes are banned
    pub changed_ip_masternodes: Frequency,
}

impl MasternodeListChangesStrategy {
    pub fn any_is_set(&self) -> bool {
        self.new_hpmns.is_set()
            || self.removed_hpmns.is_set()
            || self.updated_hpmns.is_set()
            || self.banned_hpmns.is_set()
            || self.unbanned_hpmns.is_set()
            || self.new_masternodes.is_set()
            || self.removed_masternodes.is_set()
            || self.updated_masternodes.is_set()
            || self.banned_masternodes.is_set()
            || self.unbanned_masternodes.is_set()
            || self.changed_ip_hpmns.is_set()
            || self.changed_http_port_hpmns.is_set()
            || self.changed_p2p_port_hpmns.is_set()
            || self.changed_ip_masternodes.is_set()
    }

    pub fn removed_any_masternode_types(&self) -> bool {
        self.removed_masternodes.is_set() || self.removed_hpmns.is_set()
    }

    pub fn updated_any_masternode_types(&self) -> bool {
        self.updated_masternodes.is_set() || self.updated_hpmns.is_set()
    }

    pub fn added_any_masternode_types(&self) -> bool {
        self.new_masternodes.is_set() || self.new_hpmns.is_set()
    }

    pub fn any_kind_of_update_is_set(&self) -> bool {
        self.updated_hpmns.is_set()
            || self.banned_hpmns.is_set()
            || self.unbanned_hpmns.is_set()
            || self.changed_ip_hpmns.is_set()
            || self.changed_http_port_hpmns.is_set()
            || self.changed_p2p_port_hpmns.is_set()
            || self.updated_masternodes.is_set()
            || self.banned_masternodes.is_set()
            || self.unbanned_masternodes.is_set()
            || self.changed_ip_masternodes.is_set()
    }
}

pub enum StrategyMode {
    ProposerOnly,
    ProposerAndValidatorHashValidationOnly,
    //ProposerAndValidatorSigning, todo
}

#[derive(Clone, Debug, Default)]
pub struct FailureStrategy {
    pub deterministic_start_seed: Option<u64>,
    pub dont_finalize_block: bool,
    pub expect_every_block_errors_with_codes: Vec<u32>,
    pub expect_specific_block_errors_with_codes: HashMap<u64, Vec<u32>>,
    // 1 here would be round 1 is successful
    pub rounds_before_successful_block: Option<u32>,
}

#[derive(Clone, Debug, Default)]
pub struct MasternodeChanges {
    /// The masternode ban chance should be always quite low
    pub masternode_ban_chance: Frequency,
    pub masternode_unban_chance: Frequency,
    pub masternode_change_ip_chance: Frequency,
    pub masternode_change_port_chance: Frequency,
}

#[derive(Clone, Debug, Default)]
pub enum CoreHeightIncrease {
    #[default]
    NoCoreHeightIncrease,
    RandomCoreHeightIncrease(Frequency),
    KnownCoreHeightIncreases(Vec<u32>),
}

impl CoreHeightIncrease {
    pub fn max_core_height(&self, block_count: u64, initial_core_height: u32) -> u32 {
        match self {
            NoCoreHeightIncrease => initial_core_height,
            CoreHeightIncrease::RandomCoreHeightIncrease(frequency) => {
                initial_core_height + frequency.max_event_count() as u32 * block_count as u32
            }
            CoreHeightIncrease::KnownCoreHeightIncreases(values) => {
                values.last().copied().unwrap_or(initial_core_height)
            }
        }
    }
    pub fn average_core_height(&self, block_count: u64, initial_core_height: u32) -> u32 {
        match self {
            NoCoreHeightIncrease => initial_core_height,
            CoreHeightIncrease::RandomCoreHeightIncrease(frequency) => {
                initial_core_height + frequency.average_event_count() as u32 * block_count as u32
            }
            CoreHeightIncrease::KnownCoreHeightIncreases(values) => values
                .get(values.len() / 2)
                .copied()
                .unwrap_or(initial_core_height),
        }
    }

    pub fn add_events_if_hit(&mut self, core_height: u32, rng: &mut StdRng) -> u32 {
        match self {
            NoCoreHeightIncrease => 0,
            CoreHeightIncrease::RandomCoreHeightIncrease(frequency) => {
                core_height + frequency.events_if_hit(rng) as u32
            }
            CoreHeightIncrease::KnownCoreHeightIncreases(values) => {
                if values.len() == 1 {
                    *values.first().unwrap()
                } else {
                    values.pop().unwrap()
                }
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct NetworkStrategy {
    pub strategy: Strategy,
    pub total_hpmns: u16,
    pub extra_normal_mns: u16,
    pub validator_quorum_count: u16,
    pub chain_lock_quorum_count: u16,
    pub initial_core_height: u32,
    pub upgrading_info: Option<UpgradingInfo>,
    pub core_height_increase: CoreHeightIncrease,
    pub proposer_strategy: MasternodeListChangesStrategy,
    pub rotate_quorums: bool,
    pub failure_testing: Option<FailureStrategy>,
    pub query_testing: Option<QueryStrategy>,
    pub verify_state_transition_results: bool,
    pub max_tx_bytes_per_block: u64,
    pub independent_process_proposal_verification: bool,
    pub sign_chain_locks: bool,
}

impl Default for NetworkStrategy {
    fn default() -> Self {
        NetworkStrategy {
            strategy: Default::default(),
            total_hpmns: 100,
            extra_normal_mns: 0,
            validator_quorum_count: 24,
            chain_lock_quorum_count: 24,
            initial_core_height: 1,
            upgrading_info: None,
            core_height_increase: NoCoreHeightIncrease,
            proposer_strategy: Default::default(),
            rotate_quorums: false,
            failure_testing: None,
            query_testing: None,
            verify_state_transition_results: false,
            max_tx_bytes_per_block: 44800,
            independent_process_proposal_verification: false,
            sign_chain_locks: false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct UpgradingInfo {
    pub current_protocol_version: ProtocolVersion,
    pub proposed_protocol_versions_with_weight: Vec<(ProtocolVersion, u16)>,
    /// The upgrade three quarters life is the expected amount of blocks in the window
    /// for three quarters of the network to upgrade
    /// if it is 1, there is a 50/50% chance that the network will upgrade in the first window
    /// if it lower than 1 there is a high chance it will upgrade in the first window
    /// the higher it is the lower the chance it will upgrade in the first window
    pub upgrade_three_quarters_life: f64,
}

impl UpgradingInfo {
    pub fn apply_to_proposers(
        &self,
        proposers: Vec<ProTxHash>,
        blocks_per_epoch: u64,
        rng: &mut StdRng,
    ) -> HashMap<ProTxHash, ValidatorVersionMigration> {
        let expected_blocks = blocks_per_epoch as f64 * self.upgrade_three_quarters_life;
        proposers
            .into_iter()
            .map(|pro_tx_hash| {
                let next_version = self
                    .proposed_protocol_versions_with_weight
                    .choose_weighted(rng, |item| item.1)
                    .unwrap()
                    .0;
                // we generate a random number between 0 and 1
                let u: f64 = rng.gen();
                // we want to alter the randomness so that 75% of time we get
                let change_block_height =
                    (expected_blocks * 0.75 * f64::ln(1.0 - u) / f64::ln(0.5)) as u64;
                (
                    pro_tx_hash,
                    ValidatorVersionMigration {
                        current_protocol_version: self.current_protocol_version,
                        next_protocol_version: next_version,
                        change_block_height,
                    },
                )
            })
            .collect()
    }
}

impl NetworkStrategy {
    pub fn dont_finalize_block(&self) -> bool {
        self.failure_testing
            .as_ref()
            .map(|failure_strategy| failure_strategy.dont_finalize_block)
            .unwrap_or(false)
    }

    // TODO: This belongs to `DocumentOp`
    pub fn add_strategy_contracts_into_drive(
        &mut self,
        drive: &Drive,
        platform_version: &PlatformVersion,
    ) {
        for op in &self.strategy.operations {
            if let OperationType::Document(doc_op) = &op.op_type {
                let serialize = doc_op
                    .contract
                    .serialize_to_bytes_with_platform_version(platform_version)
                    .expect("expected to serialize");
                drive
                    .apply_contract_with_serialization(
                        &doc_op.contract,
                        serialize,
                        BlockInfo::default(),
                        true,
                        Some(Cow::Owned(SingleEpoch(0))),
                        None,
                        platform_version,
                    )
                    .expect("expected to be able to add contract");
            }
        }
    }

    pub fn identity_state_transitions_for_block(
        &self,
        block_info: &BlockInfo,
        signer: &mut SimpleSigner,
        rng: &mut StdRng,
        platform_version: &PlatformVersion,
    ) -> Vec<(Identity, StateTransition)> {
        let mut state_transitions = vec![];
        if block_info.height == 1 && !self.strategy.start_identities.is_empty() {
            state_transitions.append(&mut self.strategy.start_identities.clone());
        }
        let frequency = &self.strategy.identities_inserts;
        if frequency.check_hit(rng) {
            let count = frequency.events(rng);
            state_transitions.append(
                &mut strategy_tests::transitions::create_identities_state_transitions(
                    count,
                    5,
                    signer,
                    rng,
                    platform_version,
                ),
            )
        }
        state_transitions
    }

    pub fn contract_state_transitions(
        &mut self,
        current_identities: &Vec<Identity>,
        signer: &SimpleSigner,
        rng: &mut StdRng,
        platform_version: &PlatformVersion,
    ) -> Vec<StateTransition> {
        self.strategy
            .contracts_with_updates
            .iter_mut()
            .map(|(created_contract, contract_updates)| {
                let identity_num = rng.gen_range(0..current_identities.len());
                let identity = current_identities
                    .get(identity_num)
                    .unwrap()
                    .clone()
                    .into_partial_identity_info();

                let entropy_used = *created_contract.entropy_used();

                let contract = created_contract.data_contract_mut();

                contract.set_owner_id(identity.id);
                let old_id = contract.id();
                let new_id = DataContract::generate_data_contract_id_v0(identity.id, entropy_used);
                contract.set_id(new_id);

                if let Some(contract_updates) = contract_updates {
                    for (_, updated_contract) in contract_updates.iter_mut() {
                        updated_contract.data_contract_mut().set_id(contract.id());
                        updated_contract
                            .data_contract_mut()
                            .set_owner_id(contract.owner_id());
                    }
                }

                // since we are changing the id, we need to update all the strategy
                self.strategy.operations.iter_mut().for_each(|operation| {
                    if let OperationType::Document(document_op) = &mut operation.op_type {
                        if document_op.contract.id() == old_id {
                            document_op.contract.set_id(contract.id());
                            document_op.document_type = document_op
                                .contract
                                .document_type_for_name(document_op.document_type.name())
                                .expect("document type must exist")
                                .to_owned_document_type();
                        }
                    }
                });

                let state_transition = DataContractCreateTransition::new_from_data_contract(
                    contract.clone(),
                    *created_contract.entropy_used(),
                    &identity,
                    1, //key id 1 should always be a high or critical auth key in these tests
                    signer,
                    platform_version,
                    None,
                )
                .expect("expected to create a create state transition from a data contract");
                state_transition
            })
            .collect()
    }

    pub fn contract_update_state_transitions(
        &mut self,
        current_identities: &Vec<Identity>,
        block_height: u64,
        signer: &SimpleSigner,
        contract_nonce_counter: &mut BTreeMap<(Identifier, Identifier), u64>,
        platform_version: &PlatformVersion,
    ) -> Vec<StateTransition> {
        self.strategy
            .contracts_with_updates
            .iter_mut()
            .filter_map(|(_, contract_updates)| {
                let Some(contract_updates) = contract_updates else {
                    return None;
                };
                let Some(contract_update) = contract_updates.get(&block_height) else {
                    return None;
                };
                let identity = current_identities
                    .iter()
                    .find(|identity| identity.id() == contract_update.data_contract().owner_id())
                    .expect("expected to find an identity")
                    .clone()
                    .into_partial_identity_info();

                let identity_contract_nonce = contract_nonce_counter
                    .entry((identity.id, contract_update.data_contract().id()))
                    .or_default();
                *identity_contract_nonce += 1;

                let state_transition = DataContractUpdateTransition::new_from_data_contract(
                    contract_update.data_contract().clone(),
                    &identity,
                    1, //key id 1 should always be a high or critical auth key in these tests
                    *identity_contract_nonce,
                    signer,
                    platform_version,
                    None,
                )
                .expect("expected to create a create state transition from a data contract");
                Some(state_transition)
            })
            .collect()
    }

    pub fn state_transitions_for_block_with_new_identities(
        &mut self,
        platform: &Platform<MockCoreRPCLike>,
        block_info: &BlockInfo,
        current_identities: &mut Vec<Identity>,
        identity_nonce_counter: &mut BTreeMap<Identifier, u64>,
        contract_nonce_counter: &mut BTreeMap<(Identifier, Identifier), u64>,
        signer: &mut SimpleSigner,
        rng: &mut StdRng,
    ) -> (Vec<StateTransition>, Vec<FinalizeBlockOperation>) {
        let mut finalize_block_operations = vec![];
        let platform_state = platform.state.read().unwrap();
        let platform_version = platform_state
            .current_platform_version()
            .expect("expected platform version");
        let identity_state_transitions =
            self.identity_state_transitions_for_block(block_info, signer, rng, platform_version);
        let (mut new_identities, mut state_transitions): (Vec<Identity>, Vec<StateTransition>) =
            identity_state_transitions.into_iter().unzip();
        current_identities.append(&mut new_identities);

        if block_info.height == 1 {
            // add contracts on block 1
            let mut contract_state_transitions =
                self.contract_state_transitions(current_identities, signer, rng, platform_version);
            state_transitions.append(&mut contract_state_transitions);
        } else {
            // Don't do any state transitions on block 1
            let (mut document_state_transitions, mut add_to_finalize_block_operations) =
                self.strategy.state_transitions_for_block(
                    &platform.drive,
                    block_info,
                    current_identities,
                    signer,
                    identity_nonce_counter,
                    contract_nonce_counter,
                    rng,
                    platform_version,
                );
            finalize_block_operations.append(&mut add_to_finalize_block_operations);
            state_transitions.append(&mut document_state_transitions);

            // There can also be contract updates

            let mut contract_update_state_transitions = self.contract_update_state_transitions(
                current_identities,
                block_info.height,
                signer,
                contract_nonce_counter,
                platform_version,
            );
            state_transitions.append(&mut contract_update_state_transitions);
        }

        (state_transitions, finalize_block_operations)
    }
}

pub enum StrategyRandomness {
    SeedEntropy(u64),
    RNGEntropy(StdRng),
}

#[derive(Clone, Debug)]
pub struct ValidatorVersionMigration {
    pub current_protocol_version: ProtocolVersion,
    pub next_protocol_version: ProtocolVersion,
    pub change_block_height: BlockHeight,
}

#[derive(Debug)]
pub struct ChainExecutionOutcome<'a> {
    pub abci_app: AbciApplication<'a, MockCoreRPCLike>,
    pub masternode_identity_balances: BTreeMap<[u8; 32], Credits>,
    pub identities: Vec<Identity>,
    pub proposers: Vec<MasternodeListItemWithUpdates>,
    pub quorums: BTreeMap<QuorumHash, TestQuorumInfo>,
    pub current_quorum_hash: QuorumHash,
    pub current_proposer_versions: Option<HashMap<ProTxHash, ValidatorVersionMigration>>,
    /// Identity nonce counters
    pub identity_nonce_counter: BTreeMap<Identifier, IdentityNonce>,
    /// Identity Contract nonce counters
    pub identity_contract_nonce_counter: BTreeMap<(Identifier, Identifier), IdentityNonce>,
    pub end_epoch_index: u16,
    pub end_time_ms: u64,
    pub strategy: NetworkStrategy,
    pub withdrawals: UnsignedWithdrawalTxs,
    /// height to the validator set update at that height
    pub validator_set_updates: BTreeMap<u64, ValidatorSetUpdate>,
    pub state_transition_results_per_block: BTreeMap<u64, Vec<(StateTransition, ExecTxResult)>>,
}

impl<'a> ChainExecutionOutcome<'a> {
    pub fn current_quorum(&self) -> &TestQuorumInfo {
        self.quorums
            .get::<QuorumHash>(&self.current_quorum_hash)
            .unwrap()
    }
}

pub struct ChainExecutionParameters {
    pub block_start: u64,
    pub core_height_start: u32,
    pub block_count: u64,
    pub proposers: Vec<MasternodeListItemWithUpdates>,
    pub quorums: BTreeMap<QuorumHash, TestQuorumInfo>,
    pub current_quorum_hash: QuorumHash,
    // the first option is if it is set
    // the second option is if we are even upgrading
    pub current_proposer_versions: Option<Option<HashMap<ProTxHash, ValidatorVersionMigration>>>,
    pub current_identity_nonce_counter: BTreeMap<Identifier, IdentityNonce>,
    pub current_identity_contract_nonce_counter: BTreeMap<(Identifier, Identifier), IdentityNonce>,
    pub start_time_ms: u64,
    pub current_time_ms: u64,
}
