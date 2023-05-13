use crate::frequency::Frequency;
use crate::masternodes::MasternodeListItemWithUpdates;
use crate::operations::FinalizeBlockOperation::IdentityAddKeys;
use crate::operations::{
    DocumentAction, DocumentOp, FinalizeBlockOperation, IdentityUpdateOp, Operation, OperationType,
};
use crate::query::QueryStrategy;
use crate::signer::SimpleSigner;
use crate::BlockHeight;
use dashcore_rpc::dashcore;
use dashcore_rpc::dashcore::{ProTxHash, QuorumHash};
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::document_type::random_document::CreateRandomDocument;
use dpp::data_contract::state_transition::data_contract_create_transition::DataContractCreateTransition;
use dpp::data_contract::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use dpp::data_contract::{generate_data_contract_id, DataContract as Contract};
use dpp::document::document_transition::document_base_transition::DocumentBaseTransition;
use dpp::document::document_transition::{
    Action, DocumentCreateTransition, DocumentDeleteTransition, DocumentReplaceTransition,
};
use dpp::document::{Document, DocumentsBatchTransition};
use dpp::identity::{Identity, KeyType, Purpose, SecurityLevel};
use dpp::serialization_traits::PlatformSerializable;
use dpp::state_transition::{StateTransition, StateTransitionIdentitySigned, StateTransitionType};
use dpp::util::deserializer::ProtocolVersion;
use dpp::version::LATEST_VERSION;
use drive::drive::flags::StorageFlags::SingleEpoch;
use drive::drive::identity::key::fetch::{IdentityKeysRequest, KeyRequestType};
use drive::drive::Drive;
use drive::fee::credits::Credits;
use drive::query::DriveQuery;
use drive_abci::abci::AbciApplication;
use drive_abci::execution::test_quorum::TestQuorumInfo;
use drive_abci::platform::Platform;
use drive_abci::rpc::core::MockCoreRPCLike;
use rand::prelude::{IteratorRandom, SliceRandom, StdRng};
use rand::Rng;
use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap, HashSet};

#[derive(Clone, Debug, Default)]
pub struct MasternodeListChangesStrategy {
    /// How many new masternodes on average per core chain lock increase
    pub new_hpmns: Frequency,
    /// How many masternodes leave the system
    pub removed_hpmns: Frequency,
    /// How many masternodes update a key
    pub updated_hpmns: Frequency,
    /// How many masternodes are banned
    pub banned_hpmns: Frequency,
    /// How many masternodes are unbanned
    pub unbanned_hpmns: Frequency,
    /// How many new masternodes on average per core chain lock increase
    pub new_masternodes: Frequency,
    /// How many masternodes leave the system
    pub removed_masternodes: Frequency,
    /// How many masternodes update a key
    pub updated_mastenodes: Frequency,
    /// How many masternodes are banned
    pub banned_masternodes: Frequency,
    /// How many masternodes are unbanned
    pub unbanned_masternodes: Frequency,
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
            || self.updated_mastenodes.is_set()
            || self.banned_masternodes.is_set()
            || self.unbanned_masternodes.is_set()
    }

    pub fn removed_any_masternode_types(&self) -> bool {
        self.removed_masternodes.is_set() || self.removed_hpmns.is_set()
    }

    pub fn updated_any_masternode_types(&self) -> bool {
        self.updated_mastenodes.is_set() || self.updated_hpmns.is_set()
    }

    pub fn added_any_masternode_types(&self) -> bool {
        self.new_masternodes.is_set() || self.new_hpmns.is_set()
    }

    pub fn any_update_is_set(&self) -> bool {
        self.updated_hpmns.is_set()
            || self.banned_hpmns.is_set()
            || self.unbanned_hpmns.is_set() | self.updated_mastenodes.is_set()
            || self.banned_masternodes.is_set()
            || self.unbanned_masternodes.is_set()
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
}

#[derive(Clone, Debug)]
pub struct Strategy {
    pub contracts_with_updates: Vec<(Contract, Option<BTreeMap<u64, Contract>>)>,
    pub operations: Vec<Operation>,
    pub identities_inserts: Frequency,
    pub total_hpmns: u16,
    pub extra_normal_mns: u16,
    pub quorum_count: u16,
    pub upgrading_info: Option<UpgradingInfo>,
    pub core_height_increase: Frequency,
    pub proposer_strategy: MasternodeListChangesStrategy,
    pub rotate_quorums: bool,
    pub failure_testing: Option<FailureStrategy>,
    pub query_testing: Option<QueryStrategy>,
    pub verify_state_transition_results: bool,
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

impl Strategy {
    pub fn dont_finalize_block(&self) -> bool {
        self.failure_testing
            .as_ref()
            .map(|failure_strategy| failure_strategy.dont_finalize_block)
            .unwrap_or(false)
    }

    // TODO: This belongs to `DocumentOp`
    pub fn add_strategy_contracts_into_drive(&mut self, drive: &Drive) {
        for op in &self.operations {
            match &op.op_type {
                OperationType::Document(doc_op) => {
                    let serialize = doc_op.contract.serialize().expect("expected to serialize");
                    drive
                        .apply_contract_with_serialization(
                            &doc_op.contract,
                            serialize,
                            BlockInfo::default(),
                            true,
                            Some(Cow::Owned(SingleEpoch(0))),
                            None,
                        )
                        .expect("expected to be able to add contract");
                }
                _ => {}
            }
        }
    }

    pub fn identity_state_transitions_for_block(
        &self,
        signer: &mut SimpleSigner,
        rng: &mut StdRng,
    ) -> Vec<(Identity, StateTransition)> {
        let frequency = &self.identities_inserts;
        if frequency.check_hit(rng) {
            let count = frequency.events(rng);
            crate::transitions::create_identities_state_transitions(count, 5, signer, rng)
        } else {
            vec![]
        }
    }

    pub fn contract_state_transitions(
        &mut self,
        current_identities: &Vec<Identity>,
        signer: &SimpleSigner,
        rng: &mut StdRng,
    ) -> Vec<StateTransition> {
        self.contracts_with_updates
            .iter_mut()
            .map(|(contract, contract_updates)| {
                let identity_num = rng.gen_range(0..current_identities.len());
                let identity = current_identities
                    .get(identity_num)
                    .unwrap()
                    .clone()
                    .into_partial_identity_info();

                contract.owner_id = identity.id;
                let old_id = contract.id;
                contract.id = generate_data_contract_id(identity.id, contract.entropy);
                contract
                    .document_types
                    .iter_mut()
                    .for_each(|(_, document_type)| document_type.data_contract_id = contract.id);

                if let Some(contract_updates) = contract_updates {
                    for (_, updated_contract) in contract_updates.iter_mut() {
                        updated_contract.id = contract.id;
                        updated_contract.owner_id = contract.owner_id;
                        updated_contract.document_types.iter_mut().for_each(
                            |(_, document_type)| document_type.data_contract_id = contract.id,
                        );
                    }
                }

                // since we are changing the id, we need to update all the strategy
                self.operations.iter_mut().for_each(|operation| {
                    if let OperationType::Document(document_op) = &mut operation.op_type {
                        if document_op.contract.id == old_id {
                            document_op.contract.id = contract.id;
                            document_op.contract.document_types.iter_mut().for_each(
                                |(_, document_type)| document_type.data_contract_id = contract.id,
                            );
                            document_op.document_type.data_contract_id = contract.id;
                        }
                    }
                });

                let state_transition = DataContractCreateTransition::new_from_data_contract(
                    contract.clone(),
                    &identity,
                    1, //key id 1 should always be a high or critical auth key in these tests
                    signer,
                )
                .expect("expected to create a create state transition from a data contract");
                state_transition.into()
            })
            .collect()
    }

    pub fn contract_update_state_transitions(
        &mut self,
        current_identities: &Vec<Identity>,
        block_height: u64,
        signer: &SimpleSigner,
    ) -> Vec<StateTransition> {
        self.contracts_with_updates
            .iter_mut()
            .filter_map(|(_, contract_updates)| {
                let Some(contract_updates) = contract_updates else {
                    return None;
                };
                let Some(contract_update) = contract_updates.get(&block_height) else {
                    return None
                };
                let identity = current_identities
                    .iter()
                    .find(|identity| identity.id == contract_update.owner_id)
                    .expect("expected to find an identity")
                    .clone()
                    .into_partial_identity_info();

                let state_transition = DataContractUpdateTransition::new_from_data_contract(
                    contract_update.clone(),
                    &identity,
                    1, //key id 1 should always be a high or critical auth key in these tests
                    signer,
                )
                .expect("expected to create a create state transition from a data contract");
                Some(state_transition.into())
            })
            .collect()
    }

    // TODO: this belongs to `DocumentOp`, also randomization details are common for all operations
    // and could be moved out of here
    pub fn state_transitions_for_block(
        &self,
        platform: &Platform<MockCoreRPCLike>,
        block_info: &BlockInfo,
        current_identities: &mut Vec<Identity>,
        signer: &mut SimpleSigner,
        rng: &mut StdRng,
    ) -> (Vec<StateTransition>, Vec<FinalizeBlockOperation>) {
        let mut operations = vec![];
        let mut finalize_block_operations = vec![];
        for op in &self.operations {
            if op.frequency.check_hit(rng) {
                let count = rng.gen_range(op.frequency.times_per_block_range.clone());
                match &op.op_type {
                    OperationType::Document(DocumentOp {
                        action: DocumentAction::DocumentActionInsert,
                        document_type,
                        contract,
                    }) => {
                        let documents = document_type.random_documents_with_params(
                            count as u32,
                            current_identities,
                            block_info.time_ms,
                            rng,
                        );
                        documents
                            .into_iter()
                            .for_each(|(document, identity, entropy)| {
                                let updated_at =
                                    if document_type.required_fields.contains("$updatedAt") {
                                        document.created_at
                                    } else {
                                        None
                                    };
                                let document_create_transition = DocumentCreateTransition {
                                    base: DocumentBaseTransition {
                                        id: document.id,
                                        document_type_name: document_type.name.clone(),
                                        action: Action::Create,
                                        data_contract_id: contract.id,
                                        data_contract: contract.clone(),
                                    },
                                    entropy: entropy.to_buffer(),
                                    created_at: document.created_at,
                                    updated_at,
                                    data: document.properties.into(),
                                };

                                let mut document_batch_transition = DocumentsBatchTransition {
                                    protocol_version: LATEST_VERSION,
                                    transition_type: StateTransitionType::DocumentsBatch,
                                    owner_id: identity.id,
                                    transitions: vec![document_create_transition.into()],
                                    signature_public_key_id: None,
                                    signature: None,
                                };

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
                                    .sign_external(identity_public_key, signer)
                                    .expect("expected to sign");

                                operations.push(document_batch_transition.into());
                            });
                    }
                    OperationType::Document(DocumentOp {
                        action: DocumentAction::DocumentActionDelete,
                        document_type,
                        contract,
                    }) => {
                        let any_item_query = DriveQuery::any_item_query(contract, document_type);
                        let mut items = platform
                            .drive
                            .query_documents_as_serialized(
                                any_item_query,
                                Some(&block_info.epoch),
                                None,
                            )
                            .expect("expect to execute query")
                            .items;

                        if !items.is_empty() {
                            let first_item = items.remove(0);
                            let document =
                                Document::from_bytes(first_item.as_slice(), document_type)
                                    .expect("expected to deserialize document");

                            //todo: fix this into a search key request for the following
                            //let search_key_request = BTreeMap::from([(Purpose::AUTHENTICATION as u8, BTreeMap::from([(SecurityLevel::HIGH as u8, AllKeysOfKindRequest)]))]);

                            let request = IdentityKeysRequest {
                                identity_id: document.owner_id.to_buffer(),
                                request_type: KeyRequestType::SpecificKeys(vec![1]),
                                limit: Some(1),
                                offset: None,
                            };
                            let identity = platform
                                .drive
                                .fetch_identity_balance_with_keys(request, None)
                                .expect("expected to be able to get identity")
                                .expect("expected to get an identity");
                            let document_delete_transition = DocumentDeleteTransition {
                                base: DocumentBaseTransition {
                                    id: document.id,
                                    document_type_name: document_type.name.clone(),
                                    action: Action::Delete,
                                    data_contract_id: contract.id,
                                    data_contract: contract.clone(),
                                },
                            };

                            let mut document_batch_transition = DocumentsBatchTransition {
                                protocol_version: LATEST_VERSION,
                                transition_type: StateTransitionType::DocumentsBatch,
                                owner_id: identity.id,
                                transitions: vec![document_delete_transition.into()],
                                signature_public_key_id: None,
                                signature: None,
                            };

                            let identity_public_key = identity
                                .loaded_public_keys
                                .values()
                                .next()
                                .expect("expected a key");

                            document_batch_transition
                                .sign_external(identity_public_key, signer)
                                .expect("expected to sign");

                            operations.push(document_batch_transition.into());
                        }
                    }
                    OperationType::Document(DocumentOp {
                        action: DocumentAction::DocumentActionReplace,
                        document_type,
                        contract,
                    }) => {
                        let any_item_query = DriveQuery::any_item_query(contract, document_type);
                        let mut items = platform
                            .drive
                            .query_documents_as_serialized(
                                any_item_query,
                                Some(&block_info.epoch),
                                None,
                            )
                            .expect("expect to execute query")
                            .items;

                        if !items.is_empty() {
                            let first_item = items.remove(0);
                            let document =
                                Document::from_bytes(first_item.as_slice(), document_type)
                                    .expect("expected to deserialize document");

                            //todo: fix this into a search key request for the following
                            //let search_key_request = BTreeMap::from([(Purpose::AUTHENTICATION as u8, BTreeMap::from([(SecurityLevel::HIGH as u8, AllKeysOfKindRequest)]))]);

                            let random_new_document = document_type.random_document_with_rng(rng);
                            let request = IdentityKeysRequest {
                                identity_id: document.owner_id.to_buffer(),
                                request_type: KeyRequestType::SpecificKeys(vec![1]),
                                limit: Some(1),
                                offset: None,
                            };
                            let identity = platform
                                .drive
                                .fetch_identity_balance_with_keys(request, None)
                                .expect("expected to be able to get identity")
                                .expect("expected to get an identity");
                            let document_replace_transition = DocumentReplaceTransition {
                                base: DocumentBaseTransition {
                                    id: document.id,
                                    document_type_name: document_type.name.clone(),
                                    action: Action::Replace,
                                    data_contract_id: contract.id,
                                    data_contract: contract.clone(),
                                },
                                revision: document.revision.expect("expected to unwrap revision")
                                    + 1,
                                updated_at: Some(block_info.time_ms),
                                data: Some(random_new_document.properties),
                            };

                            let mut document_batch_transition = DocumentsBatchTransition {
                                protocol_version: LATEST_VERSION,
                                transition_type: StateTransitionType::DocumentsBatch,
                                owner_id: identity.id,
                                transitions: vec![document_replace_transition.into()],
                                signature_public_key_id: None,
                                signature: None,
                            };

                            let identity_public_key = identity
                                .loaded_public_keys
                                .values()
                                .next()
                                .expect("expected a key");

                            document_batch_transition
                                .sign_external(identity_public_key, signer)
                                .expect("expected to sign");

                            operations.push(document_batch_transition.into());
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
                            operations.push(crate::transitions::create_identity_top_up_transition(
                                rng,
                                random_identity,
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
                                        crate::transitions::create_identity_update_transition_add_keys(
                                            random_identity,
                                            *count,
                                            signer,
                                            rng,
                                        );
                                    operations.push(state_transition);
                                    finalize_block_operations.push(IdentityAddKeys(
                                        keys_to_add_at_end_block.0,
                                        keys_to_add_at_end_block.1,
                                    ))
                                }
                                IdentityUpdateOp::IdentityUpdateDisableKey(count) => {
                                    let state_transition =
                                        crate::transitions::create_identity_update_transition_disable_keys(
                                            random_identity,
                                            *count,
                                            block_info.time_ms,
                                            signer,
                                            rng,
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
                                crate::transitions::create_identity_withdrawal_transition(
                                    random_identity,
                                    signer,
                                    rng,
                                );
                            operations.push(state_transition);
                        }
                    }
                    // OperationType::ContractCreate(new_fields_optional_count_range, new_fields_required_count_range, new_index_count_range, document_type_count)
                    // if !current_identities.is_empty() => {
                    //     DataContract::;
                    //
                    //     DocumentType::random_document()
                    // }
                    // OperationType::ContractUpdate(DataContractNewDocumentTypes(count))
                    //     if !current_identities.is_empty() => {
                    //
                    // }
                    _ => {}
                }
            }
        }
        (operations, finalize_block_operations)
    }

    pub fn state_transitions_for_block_with_new_identities(
        &mut self,
        platform: &Platform<MockCoreRPCLike>,
        block_info: &BlockInfo,
        current_identities: &mut Vec<Identity>,
        signer: &mut SimpleSigner,
        rng: &mut StdRng,
    ) -> (Vec<StateTransition>, Vec<FinalizeBlockOperation>) {
        let mut finalize_block_operations = vec![];
        let identity_state_transitions = self.identity_state_transitions_for_block(signer, rng);
        let (mut identities, mut state_transitions): (Vec<Identity>, Vec<StateTransition>) =
            identity_state_transitions.into_iter().unzip();
        current_identities.append(&mut identities);

        if block_info.height == 1 {
            // add contracts on block 1
            let mut contract_state_transitions =
                self.contract_state_transitions(current_identities, signer, rng);
            state_transitions.append(&mut contract_state_transitions);
        } else {
            // Don't do any state transitions on block 1
            let (mut document_state_transitions, mut add_to_finalize_block_operations) = self
                .state_transitions_for_block(platform, block_info, current_identities, signer, rng);
            finalize_block_operations.append(&mut add_to_finalize_block_operations);
            state_transitions.append(&mut document_state_transitions);

            // There can also be contract updates

            let mut contract_update_state_transitions = self.contract_update_state_transitions(
                current_identities,
                block_info.height,
                signer,
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

pub struct ChainExecutionOutcome<'a> {
    pub abci_app: AbciApplication<'a, MockCoreRPCLike>,
    pub masternode_identity_balances: BTreeMap<[u8; 32], Credits>,
    pub identities: Vec<Identity>,
    pub proposers: Vec<MasternodeListItemWithUpdates>,
    pub quorums: BTreeMap<QuorumHash, TestQuorumInfo>,
    pub current_quorum_hash: QuorumHash,
    pub current_proposer_versions: Option<HashMap<ProTxHash, ValidatorVersionMigration>>,
    pub end_epoch_index: u16,
    pub end_time_ms: u64,
    pub strategy: Strategy,
    pub withdrawals: Vec<dashcore::Transaction>,
}

impl<'a> ChainExecutionOutcome<'a> {
    pub fn current_quorum(&self) -> &TestQuorumInfo {
        self.quorums.get(&self.current_quorum_hash).unwrap()
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
    pub current_time_ms: u64,
}
