use crate::masternodes::MasternodeListItemWithUpdates;
use crate::query::QueryStrategy;
use crate::BlockHeight;
use dashcore_rpc::dashcore::{Network, PrivateKey};
use dashcore_rpc::dashcore::{ProTxHash, QuorumHash};
use dpp::block::block_info::BlockInfo;
use dpp::state_transition::identity_topup_transition::methods::IdentityTopUpTransitionMethodsV0;
use dpp::ProtocolError;

use dpp::dashcore::secp256k1::SecretKey;
use dpp::data_contract::document_type::random_document::CreateRandomDocument;
use dpp::data_contract::{DataContract, DataContractFactory};
use dpp::state_transition::identity_topup_transition::IdentityTopUpTransition;
use strategy_tests::frequency::Frequency;
use strategy_tests::operations::FinalizeBlockOperation::IdentityAddKeys;
use strategy_tests::operations::{
    DocumentAction, DocumentOp, FinalizeBlockOperation, IdentityUpdateOp, OperationType,
};

use dpp::document::DocumentV0Getters;
use dpp::fee::Credits;
use dpp::identity::{Identity, IdentityPublicKey, KeyID, KeyType, Purpose, SecurityLevel};
use dpp::serialization::PlatformSerializableWithPlatformVersion;
use dpp::state_transition::data_contract_create_transition::DataContractCreateTransition;
use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use dpp::state_transition::StateTransition;
use dpp::util::deserializer::ProtocolVersion;
use dpp::version::PlatformVersion;
use drive::drive::flags::StorageFlags::SingleEpoch;
use drive::drive::identity::key::fetch::{IdentityKeysRequest, KeyRequestType};
use drive::drive::Drive;

use dpp::identity::KeyType::ECDSA_SECP256K1;
use dpp::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
use dpp::state_transition::data_contract_update_transition::methods::DataContractUpdateTransitionMethodsV0;
use drive::query::DriveDocumentQuery;
use drive_abci::mimic::test_quorum::TestQuorumInfo;
use drive_abci::platform_types::platform::Platform;
use drive_abci::rpc::core::MockCoreRPCLike;
use rand::prelude::{IteratorRandom, SliceRandom, StdRng};
use rand::Rng;
use strategy_tests::Strategy;
use strategy_tests::transitions::{create_state_transitions_for_identities, create_state_transitions_for_identities_and_proofs, instant_asset_lock_proof_fixture};
use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::str::FromStr;
use tenderdash_abci::proto::abci::{ExecTxResult, ValidatorSetUpdate};
use dpp::dashcore::hashes::Hash;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::v0::DocumentTypeV0;
use dpp::identifier::MasternodeIdentifiers;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
use dpp::identity::state_transition::asset_lock_proof::InstantAssetLockProof;
use dpp::platform_value::{BinaryData, Value};
use dpp::prelude::{AssetLockProof, Identifier, IdentityNonce};
use dpp::state_transition::documents_batch_transition::document_base_transition::v0::DocumentBaseTransitionV0;
use dpp::state_transition::documents_batch_transition::document_create_transition::{DocumentCreateTransition, DocumentCreateTransitionV0};
use dpp::state_transition::documents_batch_transition::document_transition::document_delete_transition::DocumentDeleteTransitionV0;
use dpp::state_transition::documents_batch_transition::document_transition::document_replace_transition::DocumentReplaceTransitionV0;
use dpp::state_transition::documents_batch_transition::{DocumentsBatchTransition, DocumentsBatchTransitionV0};
use dpp::state_transition::documents_batch_transition::document_transition::{DocumentDeleteTransition, DocumentReplaceTransition, DocumentTransferTransition};
use drive::drive::document::query::QueryDocumentsOutcomeV0Methods;
use dpp::state_transition::data_contract_create_transition::methods::v0::DataContractCreateTransitionMethodsV0;
use dpp::state_transition::documents_batch_transition::document_transition::document_transfer_transition::DocumentTransferTransitionV0;
use dpp::state_transition::masternode_vote_transition::MasternodeVoteTransition;
use dpp::state_transition::masternode_vote_transition::methods::MasternodeVoteTransitionMethodsV0;
use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use dpp::voting::vote_polls::VotePoll;
use dpp::voting::votes::resource_vote::ResourceVote;
use dpp::voting::votes::resource_vote::v0::ResourceVoteV0;
use dpp::voting::votes::Vote;
use drive_abci::abci::app::FullAbciApplication;
use drive_abci::platform_types::platform_state::v0::PlatformStateV0Methods;
use drive_abci::config::PlatformConfig;
use drive_abci::platform_types::signature_verification_quorum_set::{
    QuorumConfig, Quorums, SigningQuorum,
};
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

    #[allow(dead_code)]
    #[deprecated(note = "This function is marked as unused.")]
    #[allow(deprecated)]
    pub fn removed_any_masternode_types(&self) -> bool {
        self.removed_masternodes.is_set() || self.removed_hpmns.is_set()
    }
    #[allow(dead_code)]
    #[deprecated(note = "This function is marked as unused.")]
    #[allow(deprecated)]
    pub fn updated_any_masternode_types(&self) -> bool {
        self.updated_masternodes.is_set() || self.updated_hpmns.is_set()
    }
    #[allow(dead_code)]
    #[deprecated(note = "This function is marked as unused.")]
    #[allow(deprecated)]
    pub fn added_any_masternode_types(&self) -> bool {
        self.new_masternodes.is_set() || self.new_hpmns.is_set()
    }
}

#[allow(dead_code)]
#[deprecated(note = "This function is marked as unused.")]
#[allow(deprecated)]
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
    #[allow(dead_code)] // TODO investigate why this is never constructed according to compiler
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
    #[allow(dead_code)]
    #[deprecated(note = "This function is marked as unused.")]
    #[allow(deprecated)]
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

    #[allow(dead_code)]
    #[deprecated(note = "This function is marked as unused.")]
    #[allow(deprecated)]
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
    pub instant_lock_quorum_count: u16,
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
    pub sign_instant_locks: bool,
}

impl Default for NetworkStrategy {
    fn default() -> Self {
        NetworkStrategy {
            strategy: Default::default(),
            total_hpmns: 100,
            extra_normal_mns: 0,
            validator_quorum_count: 24,
            chain_lock_quorum_count: 24,
            instant_lock_quorum_count: 24,
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
            sign_instant_locks: false,
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
    #[allow(dead_code)]
    #[deprecated(note = "This function is marked as unused.")]
    #[allow(deprecated)]
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
        instant_lock_quorums: &Quorums<SigningQuorum>,
        platform_config: &PlatformConfig,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<(Identity, StateTransition)>, ProtocolError> {
        let mut state_transitions = vec![];
        if block_info.height == 1 {
            if self.strategy.start_identities.number_of_identities > 0 {
                let mut new_transitions = self.create_identities_state_transitions(
                    self.strategy.start_identities.number_of_identities,
                    signer,
                    rng,
                    instant_lock_quorums,
                    platform_config,
                    platform_version,
                );
                state_transitions.append(&mut new_transitions);
            }
            if !self.strategy.start_identities.hard_coded.is_empty() {
                state_transitions.extend(self.strategy.start_identities.hard_coded.clone());
            }
        }
        let frequency = &self.strategy.identity_inserts.frequency;
        if frequency.check_hit(rng) {
            let count = frequency.events(rng);
            let mut new_transitions = self.create_identities_state_transitions(
                count,
                signer,
                rng,
                instant_lock_quorums,
                platform_config,
                platform_version,
            );
            state_transitions.append(&mut new_transitions);
        }
        Ok(state_transitions)
    }

    pub fn initial_contract_state_transitions(
        &mut self,
        current_identities: &Vec<Identity>,
        signer: &SimpleSigner,
        contract_nonce_counter: &mut BTreeMap<(Identifier, Identifier), u64>,
        rng: &mut StdRng,
        platform_version: &PlatformVersion,
    ) -> Vec<StateTransition> {
        self.strategy
            .start_contracts
            .iter_mut()
            .map(|(created_contract, contract_updates)| {
                let identity_num = rng.gen_range(0..current_identities.len());
                let identity = current_identities
                    .get(identity_num)
                    .unwrap()
                    .clone()
                    .into_partial_identity_info();

                let identity_nonce = created_contract.identity_nonce();

                let contract = created_contract.data_contract_mut();

                contract.set_owner_id(identity.id);
                let old_id = contract.id();
                let new_id =
                    DataContract::generate_data_contract_id_v0(identity.id, identity_nonce);
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

                let identity_contract_nonce = contract_nonce_counter
                    .entry((identity.id, contract.id()))
                    .or_default();
                *identity_contract_nonce += 1;

                DataContractCreateTransition::new_from_data_contract(
                    contract.clone(),
                    identity_nonce,
                    &identity,
                    1, //key id 1 should always be a high or critical auth key in these tests
                    signer,
                    platform_version,
                    None,
                )
                .expect("expected to create a create state transition from a data contract")
            })
            .collect()
    }

    pub fn initial_contract_update_state_transitions(
        &mut self,
        current_identities: &Vec<Identity>,
        block_height: u64,
        signer: &SimpleSigner,
        contract_nonce_counter: &mut BTreeMap<(Identifier, Identifier), u64>,
        platform_version: &PlatformVersion,
    ) -> Vec<StateTransition> {
        self.strategy
            .start_contracts
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
                    0,
                    signer,
                    platform_version,
                    None,
                )
                .expect("expected to create a create state transition from a data contract");
                Some(state_transition)
            })
            .collect()
    }

    // TODO: this belongs to `DocumentOp`, also randomization details are common for all operations
    // and could be moved out of here
    pub fn operations_based_transitions(
        &self,
        platform: &Platform<MockCoreRPCLike>,
        block_info: &BlockInfo,
        current_identities: &mut Vec<Identity>,
        signer: &mut SimpleSigner,
        identity_nonce_counter: &mut BTreeMap<Identifier, u64>,
        contract_nonce_counter: &mut BTreeMap<(Identifier, Identifier), u64>,
        // first identifier is the vote poll id
        // second identifier is the identifier
        current_votes: &mut BTreeMap<Identifier, BTreeMap<Identifier, ResourceVoteChoice>>,
        instant_lock_quorums: &Quorums<SigningQuorum>,
        rng: &mut StdRng,
        platform_version: &PlatformVersion,
    ) -> (Vec<StateTransition>, Vec<FinalizeBlockOperation>) {
        let mut maybe_state = None;
        let mut operations = vec![];
        let mut finalize_block_operations = vec![];
        let mut replaced = vec![];
        let mut transferred = vec![];
        let mut deleted = vec![];
        for op in &self.strategy.operations {
            if op.frequency.check_hit(rng) {
                let count = rng.gen_range(op.frequency.times_per_block_range.clone());
                match &op.op_type {
                    OperationType::Document(DocumentOp {
                        action: DocumentAction::DocumentActionInsertRandom(fill_type, fill_size),
                        document_type,
                        contract,
                    }) => {
                        let documents = document_type
                            .random_documents_with_params(
                                count as u32,
                                current_identities,
                                Some(block_info.time_ms),
                                Some(block_info.height),
                                Some(block_info.core_height),
                                *fill_type,
                                *fill_size,
                                rng,
                                platform_version,
                            )
                            .expect("expected random_documents_with_params");
                        documents
                            .into_iter()
                            .for_each(|(document, identity, entropy)| {
                                let identity_contract_nonce = contract_nonce_counter
                                    .entry((identity.id(), contract.id()))
                                    .or_default();
                                let gap = self
                                    .strategy
                                    .identity_contract_nonce_gaps
                                    .as_ref()
                                    .map_or(0, |gap_amount| gap_amount.events_if_hit(rng))
                                    as u64;
                                *identity_contract_nonce += 1 + gap;

                                let prefunded_voting_balances = document_type
                                    .prefunded_voting_balances_for_document(
                                        &document,
                                        platform_version,
                                    )
                                    .expect(
                                        "expected to get prefunded voting balances for document",
                                    );

                                let document_create_transition: DocumentCreateTransition =
                                    DocumentCreateTransitionV0 {
                                        base: DocumentBaseTransitionV0 {
                                            id: document.id(),
                                            identity_contract_nonce: *identity_contract_nonce,
                                            document_type_name: document_type.name().clone(),
                                            data_contract_id: contract.id(),
                                        }
                                        .into(),
                                        entropy: entropy.to_buffer(),
                                        data: document.properties_consumed(),
                                        prefunded_voting_balance: prefunded_voting_balances,
                                    }
                                    .into();

                                let document_batch_transition: DocumentsBatchTransition =
                                    DocumentsBatchTransitionV0 {
                                        owner_id: identity.id(),
                                        transitions: vec![document_create_transition.into()],
                                        user_fee_increase: 0,
                                        signature_public_key_id: 0,
                                        signature: BinaryData::default(),
                                    }
                                    .into();
                                let mut document_batch_transition: StateTransition =
                                    document_batch_transition.into();

                                let identity_public_key = identity
                                    .get_first_public_key_matching(
                                        Purpose::AUTHENTICATION,
                                        HashSet::from([
                                            SecurityLevel::HIGH,
                                            SecurityLevel::CRITICAL,
                                        ]),
                                        HashSet::from([
                                            KeyType::ECDSA_SECP256K1,
                                            KeyType::BLS12_381,
                                        ]),
                                    )
                                    .expect("expected to get a signing key");

                                document_batch_transition
                                    .sign_external(
                                        identity_public_key,
                                        signer,
                                        Some(|_data_contract_id, _document_type_name| {
                                            Ok(SecurityLevel::HIGH)
                                        }),
                                    )
                                    .expect("expected to sign");

                                operations.push(document_batch_transition);
                            });
                    }
                    OperationType::Document(DocumentOp {
                        action:
                            DocumentAction::DocumentActionInsertSpecific(
                                specific_document_key_value_pairs,
                                identifier,
                                fill_type,
                                fill_size,
                            ),
                        document_type,
                        contract,
                    }) => {
                        let documents = if let Some(identifier) = identifier {
                            let held_identity = vec![current_identities
                                .iter()
                                .find(|identity| identity.id() == identifier)
                                .expect("expected to find identifier, review strategy params")
                                .clone()];
                            document_type
                                .random_documents_with_params(
                                    count as u32,
                                    &held_identity,
                                    Some(block_info.time_ms),
                                    Some(block_info.height),
                                    Some(block_info.core_height),
                                    *fill_type,
                                    *fill_size,
                                    rng,
                                    platform_version,
                                )
                                .expect("expected random_documents_with_params")
                        } else {
                            document_type
                                .random_documents_with_params(
                                    count as u32,
                                    current_identities,
                                    Some(block_info.time_ms),
                                    Some(block_info.height),
                                    Some(block_info.core_height),
                                    *fill_type,
                                    *fill_size,
                                    rng,
                                    platform_version,
                                )
                                .expect("expected random_documents_with_params")
                        };

                        documents
                            .into_iter()
                            .for_each(|(mut document, identity, entropy)| {
                                document
                                    .properties_mut()
                                    .append(&mut specific_document_key_value_pairs.clone());

                                let identity_contract_nonce = contract_nonce_counter
                                    .entry((identity.id(), contract.id()))
                                    .or_default();
                                *identity_contract_nonce += 1;

                                let prefunded_voting_balances = document_type
                                    .prefunded_voting_balances_for_document(
                                        &document,
                                        platform_version,
                                    )
                                    .expect(
                                        "expected to get prefunded voting balances for document",
                                    );

                                let document_create_transition: DocumentCreateTransition =
                                    DocumentCreateTransitionV0 {
                                        base: DocumentBaseTransitionV0 {
                                            id: document.id(),
                                            identity_contract_nonce: *identity_contract_nonce,
                                            document_type_name: document_type.name().clone(),
                                            data_contract_id: contract.id(),
                                        }
                                        .into(),
                                        entropy: entropy.to_buffer(),
                                        data: document.properties_consumed(),
                                        prefunded_voting_balance: prefunded_voting_balances,
                                    }
                                    .into();

                                let document_batch_transition: DocumentsBatchTransition =
                                    DocumentsBatchTransitionV0 {
                                        owner_id: identity.id(),
                                        transitions: vec![document_create_transition.into()],
                                        user_fee_increase: 0,
                                        signature_public_key_id: 0,
                                        signature: BinaryData::default(),
                                    }
                                    .into();
                                let mut document_batch_transition: StateTransition =
                                    document_batch_transition.into();

                                let identity_public_key = identity
                                    .get_first_public_key_matching(
                                        Purpose::AUTHENTICATION,
                                        HashSet::from([
                                            SecurityLevel::HIGH,
                                            SecurityLevel::CRITICAL,
                                        ]),
                                        HashSet::from([
                                            KeyType::ECDSA_SECP256K1,
                                            KeyType::BLS12_381,
                                        ]),
                                    )
                                    .expect("expected to get a signing key");

                                document_batch_transition
                                    .sign_external(
                                        identity_public_key,
                                        signer,
                                        Some(|_data_contract_id, _document_type_name| {
                                            Ok(SecurityLevel::HIGH)
                                        }),
                                    )
                                    .expect("expected to sign");

                                operations.push(document_batch_transition);
                            });
                    }
                    OperationType::Document(DocumentOp {
                        action: DocumentAction::DocumentActionDelete,
                        document_type,
                        contract,
                    }) => {
                        let any_item_query =
                            DriveDocumentQuery::any_item_query(contract, document_type.as_ref());
                        let mut items = platform
                            .drive
                            .query_documents(
                                any_item_query,
                                Some(&block_info.epoch),
                                false,
                                None,
                                Some(platform_version.protocol_version),
                            )
                            .expect("expect to execute query")
                            .documents_owned();

                        items.retain(|item| !deleted.contains(&item.id()));

                        items.retain(|item| !replaced.contains(&item.id()));

                        items.retain(|item| !transferred.contains(&item.id()));

                        if !items.is_empty() {
                            let document = items.remove(0);

                            deleted.push(document.id());

                            //todo: fix this into a search key request for the following
                            //let search_key_request = BTreeMap::from([(Purpose::AUTHENTICATION as u8, BTreeMap::from([(SecurityLevel::HIGH as u8, AllKeysOfKindRequest)]))]);

                            let request = IdentityKeysRequest {
                                identity_id: document.owner_id().to_buffer(),
                                request_type: KeyRequestType::SpecificKeys(vec![1]),
                                limit: Some(1),
                                offset: None,
                            };
                            let identity = platform
                                .drive
                                .fetch_identity_balance_with_keys(request, None, platform_version)
                                .expect("expected to be able to get identity")
                                .expect("expected to get an identity");
                            let identity_contract_nonce = contract_nonce_counter
                                .get_mut(&(identity.id, contract.id()))
                                .expect(
                                    "the identity should already have a nonce for that contract",
                                );
                            *identity_contract_nonce += 1;
                            let document_delete_transition: DocumentDeleteTransition =
                                DocumentDeleteTransitionV0 {
                                    base: DocumentBaseTransitionV0 {
                                        id: document.id(),
                                        identity_contract_nonce: *identity_contract_nonce,
                                        document_type_name: document_type.name().clone(),
                                        data_contract_id: contract.id(),
                                    }
                                    .into(),
                                }
                                .into();

                            let document_batch_transition: DocumentsBatchTransition =
                                DocumentsBatchTransitionV0 {
                                    owner_id: identity.id,
                                    transitions: vec![document_delete_transition.into()],
                                    user_fee_increase: 0,
                                    signature_public_key_id: 0,
                                    signature: BinaryData::default(),
                                }
                                .into();

                            let mut document_batch_transition: StateTransition =
                                document_batch_transition.into();

                            let identity_public_key = identity
                                .loaded_public_keys
                                .values()
                                .next()
                                .expect("expected a key");

                            document_batch_transition
                                .sign_external(
                                    identity_public_key,
                                    signer,
                                    Some(|_data_contract_id, _document_type_name| {
                                        Ok(SecurityLevel::HIGH)
                                    }),
                                )
                                .expect("expected to sign");

                            operations.push(document_batch_transition);
                        }
                    }
                    OperationType::Document(DocumentOp {
                        action: DocumentAction::DocumentActionReplaceRandom,
                        document_type,
                        contract,
                    }) => {
                        let any_item_query =
                            DriveDocumentQuery::any_item_query(contract, document_type.as_ref());
                        let mut items = platform
                            .drive
                            .query_documents(
                                any_item_query,
                                Some(&block_info.epoch),
                                false,
                                None,
                                Some(platform_version.protocol_version),
                            )
                            .expect("expect to execute query")
                            .documents_owned();

                        items.retain(|item| !deleted.contains(&item.id()));

                        items.retain(|item| !replaced.contains(&item.id()));

                        items.retain(|item| !transferred.contains(&item.id()));

                        if !items.is_empty() {
                            let document = items.remove(0);

                            replaced.push(document.id());

                            //todo: fix this into a search key request for the following
                            //let search_key_request = BTreeMap::from([(Purpose::AUTHENTICATION as u8, BTreeMap::from([(SecurityLevel::HIGH as u8, AllKeysOfKindRequest)]))]);

                            let random_new_document = document_type
                                .random_document_with_rng(rng, platform_version)
                                .unwrap();
                            let request = IdentityKeysRequest {
                                identity_id: document.owner_id().to_buffer(),
                                request_type: KeyRequestType::SpecificKeys(vec![1]),
                                limit: Some(1),
                                offset: None,
                            };
                            let identity = platform
                                .drive
                                .fetch_identity_balance_with_keys(request, None, platform_version)
                                .expect("expected to be able to get identity")
                                .expect("expected to get an identity");
                            let identity_contract_nonce = contract_nonce_counter
                                .get_mut(&(identity.id, contract.id()))
                                .expect(
                                    "the identity should already have a nonce for that contract",
                                );
                            *identity_contract_nonce += 1;
                            let document_replace_transition: DocumentReplaceTransition =
                                DocumentReplaceTransitionV0 {
                                    base: DocumentBaseTransitionV0 {
                                        id: document.id(),
                                        identity_contract_nonce: *identity_contract_nonce,
                                        document_type_name: document_type.name().clone(),
                                        data_contract_id: contract.id(),
                                    }
                                    .into(),
                                    revision: document
                                        .revision()
                                        .expect("expected to unwrap revision")
                                        + 1,
                                    data: random_new_document.properties_consumed(),
                                }
                                .into();

                            let document_batch_transition: DocumentsBatchTransition =
                                DocumentsBatchTransitionV0 {
                                    owner_id: identity.id,
                                    transitions: vec![document_replace_transition.into()],
                                    user_fee_increase: 0,
                                    signature_public_key_id: 0,
                                    signature: BinaryData::default(),
                                }
                                .into();

                            let mut document_batch_transition: StateTransition =
                                document_batch_transition.into();

                            let identity_public_key = identity
                                .loaded_public_keys
                                .values()
                                .next()
                                .expect("expected a key");

                            document_batch_transition
                                .sign_external(
                                    identity_public_key,
                                    signer,
                                    Some(|_data_contract_id, _document_type_name| {
                                        Ok(SecurityLevel::HIGH)
                                    }),
                                )
                                .expect("expected to sign");

                            operations.push(document_batch_transition);
                        }
                    }
                    OperationType::Document(DocumentOp {
                        action: DocumentAction::DocumentActionTransferRandom,
                        document_type,
                        contract,
                    }) => {
                        let any_item_query =
                            DriveDocumentQuery::any_item_query(contract, document_type.as_ref());
                        let mut items = platform
                            .drive
                            .query_documents(
                                any_item_query,
                                Some(&block_info.epoch),
                                false,
                                None,
                                Some(platform_version.protocol_version),
                            )
                            .expect("expect to execute query")
                            .documents_owned();

                        items.retain(|item| !deleted.contains(&item.id()));

                        items.retain(|item| !replaced.contains(&item.id()));

                        items.retain(|item| !transferred.contains(&item.id()));

                        if !items.is_empty() {
                            let document = items.remove(0);

                            transferred.push(document.id());

                            let random_index = rng.gen_range(0..current_identities.len());
                            let mut random_identity_id = current_identities[random_index].id();

                            if random_identity_id == document.owner_id() {
                                if current_identities.len() == 1 {
                                    continue;
                                }
                                if random_index == current_identities.len() - 1 {
                                    // we are at the end
                                    random_identity_id = current_identities[random_index - 1].id();
                                } else {
                                    random_identity_id = current_identities[random_index + 1].id();
                                }
                            }

                            let request = IdentityKeysRequest {
                                identity_id: document.owner_id().to_buffer(),
                                request_type: KeyRequestType::SpecificKeys(vec![1]),
                                limit: Some(1),
                                offset: None,
                            };
                            let identity = platform
                                .drive
                                .fetch_identity_balance_with_keys(request, None, platform_version)
                                .expect("expected to be able to get identity")
                                .expect("expected to get an identity");
                            let identity_contract_nonce = contract_nonce_counter
                                .get_mut(&(identity.id, contract.id()))
                                .expect(
                                    "the identity should already have a nonce for that contract",
                                );
                            *identity_contract_nonce += 1;
                            let document_transfer_transition: DocumentTransferTransition =
                                DocumentTransferTransitionV0 {
                                    base: DocumentBaseTransitionV0 {
                                        id: document.id(),
                                        identity_contract_nonce: *identity_contract_nonce,
                                        document_type_name: document_type.name().clone(),
                                        data_contract_id: contract.id(),
                                    }
                                    .into(),
                                    revision: document
                                        .revision()
                                        .expect("expected to unwrap revision")
                                        + 1,
                                    recipient_owner_id: random_identity_id,
                                }
                                .into();

                            let document_batch_transition: DocumentsBatchTransition =
                                DocumentsBatchTransitionV0 {
                                    owner_id: identity.id,
                                    transitions: vec![document_transfer_transition.into()],
                                    user_fee_increase: 0,
                                    signature_public_key_id: 0,
                                    signature: BinaryData::default(),
                                }
                                .into();

                            let mut document_batch_transition: StateTransition =
                                document_batch_transition.into();

                            let identity_public_key = identity
                                .loaded_public_keys
                                .values()
                                .next()
                                .expect("expected a key");

                            document_batch_transition
                                .sign_external(
                                    identity_public_key,
                                    signer,
                                    Some(|_data_contract_id, _document_type_name| {
                                        Ok(SecurityLevel::HIGH)
                                    }),
                                )
                                .expect("expected to sign");

                            operations.push(document_batch_transition);
                        }
                    }
                    OperationType::IdentityTopUp if !current_identities.is_empty() => {
                        let indices: Vec<usize> =
                            (0..current_identities.len()).choose_multiple(rng, count as usize);
                        let random_identities: Vec<&Identity> = indices
                            .into_iter()
                            .map(|index| &current_identities[index])
                            .collect();

                        for random_identity in random_identities {
                            operations.push(self.create_identity_top_up_transition(
                                rng,
                                random_identity,
                                instant_lock_quorums,
                                &platform.config,
                                platform_version,
                            ));
                        }
                    }
                    OperationType::IdentityUpdate(update_op) if !current_identities.is_empty() => {
                        let indices: Vec<usize> =
                            (0..current_identities.len()).choose_multiple(rng, count as usize);
                        for index in indices {
                            let random_identity = current_identities.get_mut(index).unwrap();
                            match update_op {
                                IdentityUpdateOp::IdentityUpdateAddKeys(count) => {
                                    let (state_transition, keys_to_add_at_end_block) =
                                        strategy_tests::transitions::create_identity_update_transition_add_keys(
                                            random_identity,
                                            *count,
                                            0,
                                            identity_nonce_counter,
                                            signer,
                                            rng,
                                            platform_version,
                                        );
                                    operations.push(state_transition);
                                    finalize_block_operations.push(IdentityAddKeys(
                                        keys_to_add_at_end_block.0,
                                        keys_to_add_at_end_block.1,
                                    ))
                                }
                                IdentityUpdateOp::IdentityUpdateDisableKey(count) => {
                                    let state_transition =
                                        strategy_tests::transitions::create_identity_update_transition_disable_keys(
                                            random_identity,
                                            *count,
                                            identity_nonce_counter,
                                            block_info.time_ms,
                                            signer,
                                            rng,
                                            platform_version,
                                        );
                                    if let Some(state_transition) = state_transition {
                                        operations.push(state_transition);
                                    }
                                }
                            }
                        }
                    }
                    OperationType::IdentityWithdrawal if !current_identities.is_empty() => {
                        let indices: Vec<usize> =
                            (0..current_identities.len()).choose_multiple(rng, count as usize);
                        for index in indices {
                            let random_identity = current_identities.get_mut(index).unwrap();
                            let state_transition =
                                strategy_tests::transitions::create_identity_withdrawal_transition(
                                    random_identity,
                                    identity_nonce_counter,
                                    signer,
                                    rng,
                                );
                            operations.push(state_transition);
                        }
                    }
                    OperationType::IdentityTransfer if current_identities.len() > 1 => {
                        let identities_clone = current_identities.clone();

                        // Sender is the first in the list, which should be loaded_identity
                        let owner = &mut current_identities[0];
                        // Recipient is the second in the list
                        let recipient = &identities_clone[1];

                        let fetched_owner_balance = platform
                            .drive
                            .fetch_identity_balance(owner.id().to_buffer(), None, platform_version)
                            .expect("expected to be able to get identity")
                            .expect("expected to get an identity");

                        let state_transition =
                            strategy_tests::transitions::create_identity_credit_transfer_transition(
                                owner,
                                recipient,
                                identity_nonce_counter,
                                signer,
                                fetched_owner_balance - 100,
                            );
                        operations.push(state_transition);
                    }
                    OperationType::ContractCreate(params, doc_type_range)
                        if !current_identities.is_empty() =>
                    {
                        let contract_factory = match DataContractFactory::new(
                            platform_version.protocol_version,
                        ) {
                            Ok(contract_factory) => contract_factory,
                            Err(e) => {
                                panic!("Failed to get DataContractFactory while creating random contract: {e}");
                            }
                        };

                        // Create `count` ContractCreate transitions and push to operations vec
                        for _ in 0..count {
                            // Get the contract owner_id from loaded_identity and loaded_identity nonce
                            let identity = &current_identities[0];
                            let identity_nonce =
                                identity_nonce_counter.entry(identity.id()).or_default();
                            *identity_nonce += 1;
                            let owner_id = identity.id();

                            // Generate a contract id
                            let contract_id = DataContract::generate_data_contract_id_v0(
                                owner_id,
                                *identity_nonce,
                            );

                            // Create `doc_type_count` doc types
                            let doc_types = Value::Map(
                                doc_type_range
                                    .clone()
                                    .map(|_| {
                                        match DocumentTypeV0::random_document_type(
                                            params.clone(),
                                            contract_id,
                                            rng,
                                            platform_version,
                                        ) {
                                            Ok(new_document_type) => {
                                                let doc_type_clone =
                                                    new_document_type.schema().clone();

                                                (
                                                    Value::Text(new_document_type.name().clone()),
                                                    doc_type_clone,
                                                )
                                            }
                                            Err(e) => {
                                                panic!(
                                                    "Error generating random document type: {:?}",
                                                    e
                                                );
                                            }
                                        }
                                    })
                                    .collect(),
                            );

                            let created_data_contract = match contract_factory.create(
                                owner_id,
                                *identity_nonce,
                                doc_types,
                                None,
                                None,
                            ) {
                                Ok(contract) => contract,
                                Err(e) => {
                                    panic!("Failed to create random data contract: {e}");
                                }
                            };

                            let transition = match contract_factory
                                .create_data_contract_create_transition(created_data_contract)
                            {
                                Ok(transition) => transition,
                                Err(e) => {
                                    panic!("Failed to create ContractCreate transition: {e}");
                                }
                            };

                            // Sign transition
                            let public_key = identity
                                .get_first_public_key_matching(
                                    Purpose::AUTHENTICATION,
                                    HashSet::from([SecurityLevel::CRITICAL]),
                                    HashSet::from([KeyType::ECDSA_SECP256K1]),
                                )
                                .expect("Expected to get identity public key in ContractCreate");
                            let mut state_transition =
                                StateTransition::DataContractCreate(transition);
                            if let Err(e) = state_transition.sign_external(
                                public_key,
                                signer,
                                None::<
                                    fn(Identifier, String) -> Result<SecurityLevel, ProtocolError>,
                                >,
                            ) {
                                panic!("Error signing state transition: {:?}", e);
                            }

                            operations.push(state_transition);
                        }
                    }
                    OperationType::ResourceVote(resource_vote_op) => {
                        let state = maybe_state.get_or_insert(platform.state.load());
                        let full_masternode_list = state.full_masternode_list();
                        let vote_poll_id = resource_vote_op
                            .resolved_vote_poll
                            .unique_id()
                            .expect("expected a vote poll unique id");
                        let vote_poll_votes = current_votes.entry(vote_poll_id).or_default();
                        for _ in 0..count {
                            let rand_index = rng.gen_range(0..full_masternode_list.len());
                            let (pro_tx_hash, masternode_list_item) =
                                full_masternode_list.iter().nth(rand_index).unwrap();

                            let pro_tx_hash_bytes: [u8; 32] = pro_tx_hash.to_raw_hash().into();
                            let voting_address = masternode_list_item.state.voting_address;

                            let voting_identifier = Identifier::create_voter_identifier(
                                pro_tx_hash.as_byte_array(),
                                &voting_address,
                            );

                            // Choose the resource vote choice based on weights
                            let resource_vote_choice =
                                resource_vote_op.action.choose_weighted_choice(rng);

                            if vote_poll_votes.get(&voting_identifier)
                                == Some(&resource_vote_choice)
                            {
                                continue;
                            }

                            let identity_public_key = IdentityPublicKey::V0(IdentityPublicKeyV0 {
                                id: 0,
                                purpose: Purpose::VOTING,
                                security_level: SecurityLevel::MEDIUM,
                                contract_bounds: None,
                                key_type: KeyType::ECDSA_HASH160,
                                read_only: false,
                                data: voting_address.to_vec().into(),
                                disabled_at: None,
                            });

                            let vote = Vote::ResourceVote(ResourceVote::V0(ResourceVoteV0 {
                                vote_poll: VotePoll::ContestedDocumentResourceVotePoll(
                                    resource_vote_op.resolved_vote_poll.clone().into(),
                                ),
                                resource_vote_choice,
                            }));

                            let identity_nonce =
                                identity_nonce_counter.entry(voting_identifier).or_default();
                            *identity_nonce += 1;

                            let state_transition =
                                MasternodeVoteTransition::try_from_vote_with_signer(
                                    vote,
                                    signer,
                                    Identifier::from(pro_tx_hash_bytes),
                                    &identity_public_key,
                                    *identity_nonce,
                                    platform_version,
                                    None,
                                )
                                .expect("expected to make a masternode vote transition");

                            vote_poll_votes.insert(voting_identifier, resource_vote_choice);

                            operations.push(state_transition);
                        }
                    }
                    _ => {}
                }
            }
        }
        (operations, finalize_block_operations)
    }

    pub fn state_transitions_for_block(
        &mut self,
        platform: &Platform<MockCoreRPCLike>,
        block_info: &BlockInfo,
        current_identities: &mut Vec<Identity>,
        identity_nonce_counter: &mut BTreeMap<Identifier, u64>,
        contract_nonce_counter: &mut BTreeMap<(Identifier, Identifier), u64>,
        current_votes: &mut BTreeMap<Identifier, BTreeMap<Identifier, ResourceVoteChoice>>,
        signer: &mut SimpleSigner,
        rng: &mut StdRng,
        instant_lock_quorums: &Quorums<SigningQuorum>,
    ) -> (Vec<StateTransition>, Vec<FinalizeBlockOperation>) {
        let mut finalize_block_operations = vec![];
        let platform_state = platform.state.load();
        let platform_version = platform_state
            .current_platform_version()
            .expect("expected platform version");

        let identity_state_transitions_result = self.identity_state_transitions_for_block(
            block_info,
            signer,
            rng,
            instant_lock_quorums,
            &platform.config,
            platform_version,
        );

        // Handle the Result returned by identity_state_transitions_for_block
        let (mut identities, mut state_transitions) = match identity_state_transitions_result {
            Ok(transitions) => transitions.into_iter().unzip(),
            Err(error) => {
                eprintln!("Error creating identity state transitions: {:?}", error);
                (vec![], vec![])
            }
        };

        current_identities.append(&mut identities);

        if block_info.height == 1 {
            // add contracts on block 1
            let mut contract_state_transitions = self.initial_contract_state_transitions(
                current_identities,
                signer,
                contract_nonce_counter,
                rng,
                platform_version,
            );
            state_transitions.append(&mut contract_state_transitions);
        } else {
            // Don't do any state transitions on block 1
            let (mut document_state_transitions, mut add_to_finalize_block_operations) = self
                .operations_based_transitions(
                    platform,
                    block_info,
                    current_identities,
                    signer,
                    identity_nonce_counter,
                    contract_nonce_counter,
                    current_votes,
                    instant_lock_quorums,
                    rng,
                    platform_version,
                );
            finalize_block_operations.append(&mut add_to_finalize_block_operations);
            state_transitions.append(&mut document_state_transitions);

            // There can also be contract updates

            let mut contract_update_state_transitions = self
                .initial_contract_update_state_transitions(
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

    // add this because strategy tests library now requires a callback and uses the actual chain.
    fn create_identities_state_transitions(
        &self,
        count: u16,
        signer: &mut SimpleSigner,
        rng: &mut StdRng,
        instant_lock_quorums: &Quorums<SigningQuorum>,
        platform_config: &PlatformConfig,
        platform_version: &PlatformVersion,
    ) -> Vec<(Identity, StateTransition)> {
        let key_count = self.strategy.identity_inserts.start_keys as KeyID;
        let extra_keys = &self.strategy.identity_inserts.extra_keys;

        let (mut identities, mut keys) = Identity::random_identities_with_private_keys_with_rng::<
            Vec<_>,
        >(count, key_count, rng, platform_version)
        .expect("expected to create identities");

        for identity in identities.iter_mut() {
            for (purpose, security_to_key_type_map) in extra_keys {
                for (security_level, key_types) in security_to_key_type_map {
                    for key_type in key_types {
                        let (key, private_key) =
                            IdentityPublicKey::random_key_with_known_attributes(
                                (identity.public_keys().len() + 1) as KeyID,
                                rng,
                                *purpose,
                                *security_level,
                                *key_type,
                                None,
                                platform_version,
                            )
                            .expect("expected to create key");
                        identity.add_public_key(key.clone());
                        keys.push((key, private_key));
                    }
                }
            }
        }

        signer.add_keys(keys);

        if self.sign_instant_locks {
            let identities_with_proofs = create_signed_instant_asset_lock_proofs_for_identities(
                identities,
                rng,
                instant_lock_quorums,
                platform_config,
                platform_version,
            );

            create_state_transitions_for_identities_and_proofs(
                identities_with_proofs,
                signer,
                platform_version,
            )
        } else {
            create_state_transitions_for_identities(identities, signer, rng, platform_version)
        }
    }

    // add this because strategy tests library now requires a callback and uses the actual chain.
    fn create_identity_top_up_transition(
        &self,
        rng: &mut StdRng,
        identity: &Identity,
        instant_lock_quorums: &Quorums<SigningQuorum>,
        platform_config: &PlatformConfig,
        platform_version: &PlatformVersion,
    ) -> StateTransition {
        let (_, pk) = ECDSA_SECP256K1
            .random_public_and_private_key_data(rng, platform_version)
            .unwrap();
        let sk: [u8; 32] = pk.try_into().unwrap();
        let secret_key = SecretKey::from_str(hex::encode(sk).as_str()).unwrap();
        let mut asset_lock_proof =
            instant_asset_lock_proof_fixture(PrivateKey::new(secret_key, Network::Dash));

        // Sign transaction and update signature in instant lock proof
        if self.sign_instant_locks {
            let quorum_config = QuorumConfig {
                quorum_type: platform_config.instant_lock.quorum_type,
                active_signers: platform_config.instant_lock.quorum_active_signers,
                rotation: platform_config.instant_lock.quorum_rotation,
                window: platform_config.instant_lock.quorum_window,
            };

            // Sign transaction and update instant lock
            let AssetLockProof::Instant(InstantAssetLockProof { instant_lock, .. }) =
                &mut asset_lock_proof
            else {
                panic!("must be instant lock proof");
            };

            let request_id = instant_lock
                .request_id()
                .expect("failed to build request id");

            let (quorum_hash, quorum) = instant_lock_quorums
                .choose_quorum(&quorum_config, request_id.as_ref())
                .expect("failed to choose quorum for instant lock transaction signing");

            instant_lock.signature = quorum
                .sign_for_instant_lock(
                    &quorum_config,
                    &quorum_hash,
                    request_id.as_ref(),
                    &instant_lock.txid,
                )
                .expect("failed to sign transaction for instant lock");
        }

        IdentityTopUpTransition::try_from_identity(
            identity,
            asset_lock_proof,
            secret_key.as_ref(),
            0,
            platform_version,
            None,
        )
        .expect("expected to create top up transition")
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
    pub abci_app: FullAbciApplication<'a, MockCoreRPCLike>,
    pub masternode_identity_balances: BTreeMap<[u8; 32], Credits>,
    pub identities: Vec<Identity>,
    pub proposers: Vec<MasternodeListItemWithUpdates>,
    pub validator_quorums: BTreeMap<QuorumHash, TestQuorumInfo>,
    pub current_validator_quorum_hash: QuorumHash,
    pub current_proposer_versions: Option<HashMap<ProTxHash, ValidatorVersionMigration>>,
    pub instant_lock_quorums: Quorums<SigningQuorum>,
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
        self.validator_quorums
            .get::<QuorumHash>(&self.current_validator_quorum_hash)
            .unwrap()
    }
}

pub struct ChainExecutionParameters {
    pub block_start: u64,
    pub core_height_start: u32,
    pub block_count: u64,
    pub proposers: Vec<MasternodeListItemWithUpdates>,
    pub validator_quorums: BTreeMap<QuorumHash, TestQuorumInfo>,
    pub current_validator_quorum_hash: QuorumHash,
    pub instant_lock_quorums: Quorums<SigningQuorum>,
    // the first option is if it is set
    // the second option is if we are even upgrading
    pub current_proposer_versions: Option<Option<HashMap<ProTxHash, ValidatorVersionMigration>>>,
    pub current_identity_nonce_counter: BTreeMap<Identifier, IdentityNonce>,
    pub current_identity_contract_nonce_counter: BTreeMap<(Identifier, Identifier), IdentityNonce>,
    pub current_votes: BTreeMap<Identifier, BTreeMap<Identifier, ResourceVoteChoice>>,
    pub start_time_ms: u64,
    pub current_time_ms: u64,
}

fn create_signed_instant_asset_lock_proofs_for_identities(
    identities: Vec<Identity>,
    rng: &mut StdRng,
    instant_lock_quorums: &Quorums<SigningQuorum>,
    platform_config: &PlatformConfig,
    platform_version: &PlatformVersion,
) -> Vec<(Identity, [u8; 32], AssetLockProof)> {
    let quorum_config = QuorumConfig {
        quorum_type: platform_config.instant_lock.quorum_type,
        active_signers: platform_config.instant_lock.quorum_active_signers,
        rotation: platform_config.instant_lock.quorum_rotation,
        window: platform_config.instant_lock.quorum_window,
    };

    identities
        .into_iter()
        .map(|identity| {
            // Create instant asset lock proof
            let (_, pk) = ECDSA_SECP256K1
                .random_public_and_private_key_data(rng, platform_version)
                .unwrap();

            let pk_fixed: [u8; 32] = pk.try_into().unwrap();
            let secret_key = SecretKey::from_str(hex::encode(pk_fixed).as_str()).unwrap();
            let private_key = PrivateKey::new(secret_key, Network::Dash);

            let mut asset_lock_proof = instant_asset_lock_proof_fixture(private_key);

            // Sign transaction and update instant lock
            let AssetLockProof::Instant(InstantAssetLockProof { instant_lock, .. }) =
                &mut asset_lock_proof
            else {
                panic!("must be instant lock proof");
            };

            let request_id = instant_lock
                .request_id()
                .expect("failed to build request id");

            let (quorum_hash, quorum) = instant_lock_quorums
                .choose_quorum(&quorum_config, request_id.as_ref())
                .expect("failed to choose quorum for instant lock transaction signing");

            instant_lock.signature = quorum
                .sign_for_instant_lock(
                    &quorum_config,
                    &quorum_hash,
                    request_id.as_ref(),
                    &instant_lock.txid,
                )
                .expect("failed to sign transaction for instant lock");

            (identity, pk_fixed, asset_lock_proof)
        })
        .collect()
}
