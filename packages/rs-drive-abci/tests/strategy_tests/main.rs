// MIT LICENSE
//
// Copyright (c) 2021 Dash Core Group
//
// Permission is hereby granted, free of charge, to any
// person obtaining a copy of this software and associated
// documentation files (the "Software"), to deal in the
// Software without restriction, including without
// limitation the rights to use, copy, modify, merge,
// publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software
// is furnished to do so, subject to the following
// conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions
// of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
// ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
// TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
// PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
// SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
// CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
// OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
// IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.
//

//! Execution Tests
//!

use crate::DocumentAction::{DocumentActionDelete, DocumentActionInsert};
use anyhow::anyhow;
use dashcore::signer;
use dpp::bls_signatures::Serialize;
use dpp::data_contract::state_transition::data_contract_create_transition::DataContractCreateTransition;
use dpp::document::document_transition::document_base_transition::DocumentBaseTransition;
use dpp::document::document_transition::{
    Action, DocumentCreateTransition, DocumentDeleteTransition,
};
use dpp::document::DocumentsBatchTransition;
use dpp::identity::signer::Signer;
use dpp::identity::state_transition::identity_create_transition::IdentityCreateTransition;
use dpp::identity::{IdentityPublicKey, KeyType, Purpose, SecurityLevel};
use dpp::platform_value::{BinaryData, Value};
use dpp::state_transition::errors::{
    InvalidIdentityPublicKeyTypeError, InvalidSignaturePublicKeyError,
};
use dpp::state_transition::{StateTransition, StateTransitionIdentitySigned};
use dpp::version::LATEST_VERSION;
use dpp::ProtocolError;
use drive::common::helpers::identities::create_test_masternode_identities_with_rng;
use drive::contract::{Contract, CreateRandomDocument, DocumentType};
use drive::dpp::document::Document;
use drive::dpp::identity::{Identity, KeyID};
use drive::dpp::util::deserializer::ProtocolVersion;
use drive::drive::block_info::BlockInfo;
use drive::drive::defaults::PROTOCOL_VERSION;
use drive::drive::flags::StorageFlags::SingleEpoch;
use drive::drive::Drive;
use drive::fee::credits::Credits;
use drive::fee_pools::epochs::Epoch;
use drive::query::DriveQuery;
use drive_abci::abci::AbciApplication;
use drive_abci::execution::fee_pools::epoch::{EpochInfo, EPOCH_CHANGE_TIME_MS};
use drive_abci::platform::Platform;
use drive_abci::rpc::core::MockCoreRPCLike;
use drive_abci::test::fixture::abci::static_init_chain_request;
use drive_abci::test::helpers::setup::TestPlatformBuilder;
use drive_abci::{config::PlatformConfig, test::helpers::setup::TempPlatform};
use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};
use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::ops::Range;

mod upgrade_fork_tests;

pub type QuorumHash = [u8; 32];

#[derive(Clone, Debug)]
pub struct Frequency {
    pub times_per_block_range: Range<u16>, //insertion count when block is chosen
    pub chance_per_block: Option<f64>,     //chance of insertion if set
}

impl Frequency {
    fn check_hit(&self, rng: &mut StdRng) -> bool {
        match self.chance_per_block {
            None => true,
            Some(chance) => rng.gen_bool(chance),
        }
    }

    fn events(&self, rng: &mut StdRng) -> u16 {
        if self.times_per_block_range.is_empty() {
            0
        } else {
            rng.gen_range(self.times_per_block_range.clone())
        }
    }
}

#[derive(Clone, Debug)]
pub enum DocumentAction {
    DocumentActionInsert,
    DocumentActionDelete,
}

#[derive(Clone, Debug)]
pub struct DocumentOp {
    pub contract: Contract,
    pub document_type: DocumentType,
    pub action: DocumentAction,
}

pub type ProTxHash = [u8; 32];

/// This simple signer is only to be used in tests
#[derive(Default, Debug)]
pub struct SimpleSigner {
    /// Private keys is a map from the public key to the Private key bytes
    private_keys: HashMap<IdentityPublicKey, Vec<u8>>,
}

impl SimpleSigner {
    fn add_key(&mut self, public_key: IdentityPublicKey, private_key: Vec<u8>) {
        self.private_keys.insert(public_key, private_key);
    }

    fn add_keys<I: IntoIterator<Item = (IdentityPublicKey, Vec<u8>)>>(&mut self, keys: I) {
        self.private_keys.extend(keys)
    }
}

impl Signer for SimpleSigner {
    fn sign(
        &self,
        identity_public_key: &IdentityPublicKey,
        data: &[u8],
    ) -> Result<BinaryData, ProtocolError> {
        let private_key = self.private_keys.get(identity_public_key).ok_or(
            ProtocolError::InvalidSignaturePublicKeyError(InvalidSignaturePublicKeyError::new(
                identity_public_key.data.to_vec(),
            )),
        )?;
        match identity_public_key.key_type {
            KeyType::ECDSA_SECP256K1 | KeyType::ECDSA_HASH160 => {
                let signature = signer::sign(data, private_key)?;
                Ok(signature.to_vec().into())
            }
            KeyType::BLS12_381 => {
                let pk = dpp::bls_signatures::PrivateKey::from_bytes(private_key).map_err(|e| {
                    ProtocolError::Error(anyhow!("bls private key from bytes isn't correct"))
                })?;
                Ok(pk.sign(data).as_bytes().into())
            }
            // the default behavior from
            // https://github.com/dashevo/platform/blob/6b02b26e5cd3a7c877c5fdfe40c4a4385a8dda15/packages/js-dpp/lib/stateTransition/AbstractStateTransition.js#L187
            // is to return the error for the BIP13_SCRIPT_HASH
            KeyType::BIP13_SCRIPT_HASH => {
                return Err(ProtocolError::InvalidIdentityPublicKeyTypeError(
                    InvalidIdentityPublicKeyTypeError::new(identity_public_key.key_type),
                ))
            }
        }
    }
}

pub type BlockHeight = u64;

#[derive(Clone, Debug)]
pub(crate) struct Strategy {
    contracts: Vec<Contract>,
    operations: Vec<(DocumentOp, Frequency)>,
    identities_inserts: Frequency,
    total_hpmns: u16,
    upgrading_info: Option<UpgradingInfo>,
}

#[derive(Clone, Debug)]
pub(crate) struct UpgradingInfo {
    current_protocol_version: ProtocolVersion,
    proposed_protocol_versions_with_weight: Vec<(ProtocolVersion, u16)>,
    /// The upgrade three quarters life is the expected amount of blocks in the window
    /// for three quarters of the network to upgrade
    /// if it is 1, there is a 50/50% chance that the network will upgrade in the first window
    /// if it lower than 1 there is a high chance it will upgrade in the first window
    /// the higher it is the lower the chance it will upgrade in the first window
    upgrade_three_quarters_life: f64,
}

#[derive(Clone, Debug)]
pub struct ValidatorVersionMigration {
    current_protocol_version: ProtocolVersion,
    next_protocol_version: ProtocolVersion,
    change_block_height: BlockHeight,
}

impl UpgradingInfo {
    fn apply_to_proposers(
        &self,
        proposers: Vec<[u8; 32]>,
        blocks_per_epoch: u64,
        rng: &mut StdRng,
    ) -> HashMap<[u8; 32], ValidatorVersionMigration> {
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
    fn add_strategy_contracts_into_drive(&mut self, drive: &Drive) {
        for (op, _) in &self.operations {
            let serialize = op.contract.to_cbor().expect("expected to serialize");
            drive
                .apply_contract(
                    &op.contract,
                    serialize,
                    BlockInfo::default(),
                    true,
                    Some(Cow::Owned(SingleEpoch(0))),
                    None,
                )
                .expect("expected to be able to add contract");
        }
    }
    fn identity_state_transitions_for_block(
        &self,
        rng: &mut StdRng,
    ) -> (
        Vec<(Identity, StateTransition)>,
        Vec<(IdentityPublicKey, Vec<u8>)>,
    ) {
        let frequency = &self.identities_inserts;
        if frequency.check_hit(rng) {
            let count = frequency.events(rng);
            create_identities_state_transitions(count, 5, rng)
        } else {
            (vec![], vec![])
        }
    }

    fn contract_state_transitions(
        &self,
        current_identities: &Vec<Identity>,
        signer: &SimpleSigner,
        rng: &mut StdRng,
    ) -> Vec<StateTransition> {
        self.contracts
            .iter()
            .map(|contract| {
                let identity_num = rng.gen_range(0..current_identities.len());
                let identity = current_identities
                    .get(identity_num)
                    .unwrap()
                    .clone()
                    .into_partial_identity_info();

                let state_transition = DataContractCreateTransition::new_from_data_contract(
                    contract.clone(),
                    &identity,
                    0,
                    signer,
                )
                .expect("expected to create a create state transition from a data contract");
                state_transition.into()
            })
            .collect()
    }

    fn document_state_transitions_for_block(
        &self,
        platform: &Platform<MockCoreRPCLike>,
        block_info: &BlockInfo,
        current_identities: &Vec<Identity>,
        signer: &mut SimpleSigner,
        rng: &mut StdRng,
    ) -> Vec<StateTransition> {
        let mut operations = vec![];
        for (op, frequency) in &self.operations {
            if frequency.check_hit(rng) {
                let count = rng.gen_range(frequency.times_per_block_range.clone());
                match op.action {
                    DocumentActionInsert => {
                        let documents = op
                            .document_type
                            .random_documents_with_rng(count as u32, rng);
                        documents.into_iter().for_each(|mut document| {
                            let identity_num = rng.gen_range(0..current_identities.len());
                            let identity = current_identities.get(identity_num).unwrap().clone();

                            let document_create_transition = DocumentCreateTransition {
                                base: DocumentBaseTransition {
                                    id: document.id,
                                    document_type_name: op.document_type.name.clone(),
                                    action: Action::Create,
                                    data_contract_id: op.contract.id,
                                    data_contract: op.contract.clone(),
                                },
                                entropy: [0; 32],
                                created_at: document.created_at,
                                updated_at: document.created_at,
                                data: document.properties.into(),
                            };

                            let mut document_batch_transition = DocumentsBatchTransition {
                                protocol_version: LATEST_VERSION,
                                transition_type: Default::default(),
                                owner_id: identity.id,
                                transitions: vec![document_create_transition.into()],
                                signature_public_key_id: None,
                                signature: None,
                            };

                            let identity_public_key = identity
                                .get_first_public_key_matching(
                                    Purpose::AUTHENTICATION,
                                    HashSet::from([SecurityLevel::HIGH, SecurityLevel::CRITICAL]),
                                    HashSet::from([KeyType::ECDSA_SECP256K1, KeyType::BLS12_381]),
                                )
                                .expect("expected to get a signing key");

                            document_batch_transition
                                .sign_external(identity_public_key, signer)
                                .expect("expected to sign");

                            operations.push(document_batch_transition.into());
                        });
                    }
                    DocumentActionDelete => {
                        let any_item_query =
                            DriveQuery::any_item_query(&op.contract, &op.document_type);
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
                                Document::from_bytes(first_item.as_slice(), &op.document_type)
                                    .expect("expected to deserialize document");
                            let identity = platform
                                .drive
                                .fetch_identity_with_balance(document.owner_id.to_buffer(), None)
                                .expect("expected to be able to get identity")
                                .expect("expected to get an identity");
                            let document_delete_transition = DocumentDeleteTransition {
                                base: DocumentBaseTransition {
                                    id: document.id,
                                    document_type_name: op.document_type.name.clone(),
                                    action: Action::Create,
                                    data_contract_id: op.contract.id,
                                    data_contract: op.contract.clone(),
                                },
                            };

                            let document_batch_transition = DocumentsBatchTransition {
                                protocol_version: LATEST_VERSION,
                                transition_type: Default::default(),
                                owner_id: identity.id,
                                transitions: vec![document_delete_transition.into()],
                                signature_public_key_id: None,
                                signature: None,
                            };

                            //todo: signing
                            //document_batch_transition.sign()

                            operations.push(document_batch_transition.into());
                        }
                    }
                }
            }
        }
        operations
    }

    fn state_transitions_for_block_with_new_identities(
        &self,
        platform: &Platform<MockCoreRPCLike>,
        block_info: &BlockInfo,
        current_identities: &mut Vec<Identity>,
        signer: &mut SimpleSigner,
        rng: &mut StdRng,
    ) -> Vec<StateTransition> {
        let (identity_state_transitions, new_keys) = self.identity_state_transitions_for_block(rng);
        let (mut identities, mut state_transitions): (Vec<Identity>, Vec<StateTransition>) =
            identity_state_transitions.into_iter().unzip();
        current_identities.append(&mut identities);
        signer.add_keys(new_keys);

        if block_info.height == 1 {
            // add contracts on block 1
            let mut contract_state_transitions =
                self.contract_state_transitions(current_identities, signer, rng);
            state_transitions.append(&mut contract_state_transitions);
        }

        let mut document_state_transitions: Vec<StateTransition> = self
            .document_state_transitions_for_block(
                platform,
                block_info,
                current_identities,
                signer,
                rng,
            );

        state_transitions.append(&mut document_state_transitions);
        state_transitions
    }
}

fn create_identities_state_transitions(
    count: u16,
    key_count: KeyID,
    rng: &mut StdRng,
) -> (
    Vec<(Identity, StateTransition)>,
    Vec<(IdentityPublicKey, Vec<u8>)>,
) {
    let (identities, keys) =
        Identity::random_identities_with_private_keys_with_rng(count, key_count, rng)
            .expect("expected to create identities");
    (
        identities
            .into_iter()
            .map(|identity| {
                let identity_create_transition: IdentityCreateTransition = identity
                    .clone()
                    .try_into()
                    .expect("expected to transform identity into identity create transition");
                (identity, identity_create_transition.into())
            })
            .collect(),
        keys,
    )
}

pub struct ChainExecutionOutcome<'a> {
    pub abci_app: AbciApplication<'a, MockCoreRPCLike>,
    pub masternode_identity_balances: BTreeMap<[u8; 32], Credits>,
    pub identities: Vec<Identity>,
    pub proposers: Vec<ProTxHash>,
    pub quorums: BTreeMap<QuorumHash, Vec<ProTxHash>>,
    pub current_quorum_hash: QuorumHash,
    pub current_proposer_versions: Option<HashMap<ProTxHash, ValidatorVersionMigration>>,
    pub end_epoch_index: u16,
    pub end_time_ms: u64,
}

pub struct ChainExecutionParameters {
    pub block_start: u64,
    pub block_count: u64,
    pub block_spacing_ms: u64,
    pub proposers: Vec<[u8; 32]>,
    pub quorums: BTreeMap<QuorumHash, Vec<ProTxHash>>,
    pub current_quorum_hash: QuorumHash,
    // the first option is if it is set
    // the second option is if we are even upgrading
    pub current_proposer_versions: Option<Option<HashMap<ProTxHash, ValidatorVersionMigration>>>,
    pub current_time_ms: u64,
}

pub enum StrategyRandomness {
    SeedEntropy(u64),
    RNGEntropy(StdRng),
}

pub(crate) fn run_chain_for_strategy(
    platform: &TempPlatform<MockCoreRPCLike>,
    block_count: u64,
    block_spacing_ms: u64,
    strategy: Strategy,
    config: PlatformConfig,
    seed: u64,
) -> ChainExecutionOutcome {
    let quorum_count = 24; // We assume 24 quorums
    let quorum_size = config.quorum_size;
    let abci_application = AbciApplication::new(&platform).expect("expected new abci application");
    let mut rng = StdRng::seed_from_u64(seed);
    // init chain
    let init_chain_request = static_init_chain_request();

    platform
        .init_chain(init_chain_request)
        .expect("should init chain");

    platform.create_mn_shares_contract(None);

    let proposers = create_test_masternode_identities_with_rng(
        &platform.drive,
        strategy.total_hpmns,
        &mut rng,
        None,
    );

    let quorums: BTreeMap<QuorumHash, Vec<[u8; 32]>> = (0..quorum_count)
        .into_iter()
        .map(|_| {
            let quorum_hash: [u8; 32] = rng.gen();
            (
                quorum_hash,
                proposers
                    .choose_multiple(&mut rng, quorum_size as usize)
                    .cloned()
                    .collect(),
            )
        })
        .collect();

    let quorum_hashes: Vec<&QuorumHash> = quorums.keys().collect();

    let current_quorum_hash = **quorum_hashes.choose(&mut rng).unwrap();

    continue_chain_for_strategy(
        abci_application,
        ChainExecutionParameters {
            block_start: 1,
            block_count,
            block_spacing_ms,
            proposers,
            quorums,
            current_quorum_hash,
            current_proposer_versions: None,
            current_time_ms: 0,
        },
        strategy,
        config,
        StrategyRandomness::RNGEntropy(rng),
    )
}

pub(crate) fn continue_chain_for_strategy(
    abci_app: AbciApplication<MockCoreRPCLike>,
    chain_execution_parameters: ChainExecutionParameters,
    strategy: Strategy,
    config: PlatformConfig,
    seed: StrategyRandomness,
) -> ChainExecutionOutcome {
    let platform = abci_app.platform;
    let ChainExecutionParameters {
        block_start,
        block_count,
        block_spacing_ms,
        proposers,
        quorums,
        mut current_quorum_hash,
        current_proposer_versions,
        mut current_time_ms,
    } = chain_execution_parameters;
    let mut rng = match seed {
        StrategyRandomness::SeedEntropy(seed) => StdRng::seed_from_u64(seed),
        StrategyRandomness::RNGEntropy(rng) => rng,
    };
    let quorum_size = config.quorum_size;
    let quorum_rotation_block_count = config.validator_set_quorum_rotation_block_count as u64;
    let first_block_time = 0;
    let mut current_identities = vec![];
    let mut signer = SimpleSigner::default();
    let mut i = 0;

    let blocks_per_epoch = EPOCH_CHANGE_TIME_MS / block_spacing_ms;

    let proposer_count = proposers.len() as u32;

    let proposer_versions = current_proposer_versions.unwrap_or(
        strategy.upgrading_info.as_ref().map(|upgrading_info| {
            upgrading_info.apply_to_proposers(proposers.clone(), blocks_per_epoch, &mut rng)
        }),
    );

    for block_height in block_start..(block_start + block_count) {
        let epoch_info = EpochInfo::calculate(
            first_block_time,
            current_time_ms,
            platform
                .state
                .read()
                .expect("lock is poisoned")
                .last_committed_block_info
                .as_ref()
                .map(|block_info| block_info.time_ms),
        )
        .expect("should calculate epoch info");

        let block_info = BlockInfo {
            time_ms: current_time_ms,
            height: block_height,
            core_height: 1,
            epoch: Epoch::new(epoch_info.current_epoch_index),
        };

        let proposer = quorums
            .get(current_quorum_hash.as_slice())
            .unwrap()
            .get(i as usize)
            .unwrap();
        let state_transitions = strategy.state_transitions_for_block_with_new_identities(
            &platform,
            &block_info,
            &mut current_identities,
            &mut signer,
            &mut rng,
        );

        let proposed_version = proposer_versions
            .as_ref()
            .map(|proposer_versions| {
                let ValidatorVersionMigration {
                    current_protocol_version,
                    next_protocol_version,
                    change_block_height,
                } = proposer_versions
                    .get(proposer)
                    .expect("expected to have version");
                if &block_height >= change_block_height {
                    *next_protocol_version
                } else {
                    *current_protocol_version
                }
            })
            .unwrap_or(1);

        abci_app
            .mimic_execute_block(
                *proposer,
                current_quorum_hash,
                proposed_version,
                proposer_count,
                block_info,
                state_transitions,
            )
            .expect("expected to execute a block");

        current_time_ms += block_spacing_ms;
        i += 1;
        i %= quorum_size;
        let needs_rotation = block_height % quorum_rotation_block_count == 0;
        if needs_rotation {
            let quorum_hashes: Vec<&QuorumHash> = quorums.keys().collect();

            current_quorum_hash = **quorum_hashes.choose(&mut rng).unwrap();
        }
    }

    let masternode_identity_balances = platform
        .drive
        .fetch_identities_balances(&proposers, None)
        .expect("expected to get balances");

    let end_epoch_index = platform
        .block_execution_context
        .read()
        .expect("lock is poisoned")
        .as_ref()
        .unwrap()
        .epoch_info
        .current_epoch_index;

    ChainExecutionOutcome {
        abci_app,
        masternode_identity_balances,
        identities: current_identities,
        proposers,
        quorums,
        current_quorum_hash,
        current_proposer_versions: proposer_versions,
        end_epoch_index,
        end_time_ms: current_time_ms,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use drive::dpp::data_contract::extra::common::json_document_to_cbor;
    use drive::dpp::data_contract::DriveContractExt;
    use tenderdash_abci::proto::types::CoreChainLock;
    #[test]
    fn run_chain_nothing_happening() {
        let strategy = Strategy {
            contracts: vec![],
            operations: vec![],
            identities_inserts: Frequency {
                times_per_block_range: Default::default(),
                chance_per_block: None,
            },
            total_hpmns: 100,
            upgrading_info: None,
        };
        let config = PlatformConfig {
            verify_sum_trees: true,
            quorum_size: 100,
            validator_set_quorum_rotation_block_count: 25,
            ..Default::default()
        };
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

        platform
            .core_rpc
            .expect_get_best_chain_lock()
            .returning(move || {
                Ok(CoreChainLock {
                    core_block_height: 10,
                    core_block_hash: [1; 32].to_vec(),
                    signature: [2; 96].to_vec(),
                })
            });
        run_chain_for_strategy(&platform, 1000, 3000, strategy, config, 15);
    }

    #[test]
    fn run_chain_insert_one_new_identity_per_block() {
        let strategy = Strategy {
            contracts: vec![],
            operations: vec![],
            identities_inserts: Frequency {
                times_per_block_range: 1..2,
                chance_per_block: None,
            },
            total_hpmns: 100,
            upgrading_info: None,
        };
        let config = PlatformConfig {
            verify_sum_trees: true,
            quorum_size: 100,
            validator_set_quorum_rotation_block_count: 25,
            ..Default::default()
        };
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();
        platform
            .core_rpc
            .expect_get_best_chain_lock()
            .returning(move || {
                Ok(CoreChainLock {
                    core_block_height: 10,
                    core_block_hash: [1; 32].to_vec(),
                    signature: [2; 96].to_vec(),
                })
            });
        let outcome = run_chain_for_strategy(&platform, 100, 3000, strategy, config, 15);

        assert_eq!(outcome.identities.len(), 100);
    }

    #[test]
    fn run_chain_insert_one_new_identity_per_block_with_epoch_change() {
        let strategy = Strategy {
            contracts: vec![],
            operations: vec![],
            identities_inserts: Frequency {
                times_per_block_range: 1..2,
                chance_per_block: None,
            },
            total_hpmns: 100,
            upgrading_info: None,
        };
        let config = PlatformConfig {
            verify_sum_trees: true,
            quorum_size: 100,
            validator_set_quorum_rotation_block_count: 100,
            ..Default::default()
        };
        let day_in_ms = 1000 * 60 * 60 * 24;
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();
        platform
            .core_rpc
            .expect_get_best_chain_lock()
            .returning(move || {
                Ok(CoreChainLock {
                    core_block_height: 10,
                    core_block_hash: [1; 32].to_vec(),
                    signature: [2; 96].to_vec(),
                })
            });
        let outcome = run_chain_for_strategy(&platform, 150, day_in_ms, strategy, config, 15);
        assert_eq!(outcome.identities.len(), 150);
        assert_eq!(outcome.masternode_identity_balances.len(), 100);
        let all_have_balances = outcome
            .masternode_identity_balances
            .iter()
            .all(|(_, balance)| *balance != 0);
        assert!(all_have_balances, "all masternodes should have a balance");
    }

    #[test]
    fn run_chain_insert_one_new_identity_and_a_contract() {
        let contract_cbor = json_document_to_cbor(
            "tests/supporting_files/contract/dashpay/dashpay-contract-all-mutable.json",
            Some(PROTOCOL_VERSION),
        )
        .expect("expected to get cbor from a json document");
        let contract = <Contract as DriveContractExt>::from_cbor(&contract_cbor, None)
            .expect("contract should be deserialized");

        let strategy = Strategy {
            contracts: vec![contract],
            operations: vec![],
            identities_inserts: Frequency {
                times_per_block_range: 1..2,
                chance_per_block: None,
            },
            total_hpmns: 100,
            upgrading_info: None,
        };
        let config = PlatformConfig {
            verify_sum_trees: true,
            quorum_size: 100,
            validator_set_quorum_rotation_block_count: 25,
            ..Default::default()
        };
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();
        platform
            .core_rpc
            .expect_get_best_chain_lock()
            .returning(move || {
                Ok(CoreChainLock {
                    core_block_height: 10,
                    core_block_hash: [1; 32].to_vec(),
                    signature: [2; 96].to_vec(),
                })
            });
        run_chain_for_strategy(&platform, 1, 3000, strategy, config, 15);
    }

    #[test]
    fn run_chain_insert_one_new_identity_per_block_and_one_new_document() {
        let contract_cbor = json_document_to_cbor(
            "tests/supporting_files/contract/dashpay/dashpay-contract-all-mutable.json",
            Some(PROTOCOL_VERSION),
        )
        .expect("expected to get cbor from a json document");
        let contract = <Contract as DriveContractExt>::from_cbor(&contract_cbor, None)
            .expect("contract should be deserialized");

        let document_op = DocumentOp {
            contract: contract.clone(),
            action: DocumentActionInsert,
            document_type: contract
                .document_type_for_name("contactRequest")
                .expect("expected a profile document type")
                .clone(),
        };

        let strategy = Strategy {
            contracts: vec![contract],
            operations: vec![(
                document_op,
                Frequency {
                    times_per_block_range: 1..2,
                    chance_per_block: None,
                },
            )],
            identities_inserts: Frequency {
                times_per_block_range: 1..2,
                chance_per_block: None,
            },
            total_hpmns: 100,
            upgrading_info: None,
        };
        let config = PlatformConfig {
            verify_sum_trees: true,
            quorum_size: 100,
            validator_set_quorum_rotation_block_count: 25,
            ..Default::default()
        };
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();
        platform
            .core_rpc
            .expect_get_best_chain_lock()
            .returning(move || {
                Ok(CoreChainLock {
                    core_block_height: 10,
                    core_block_hash: [1; 32].to_vec(),
                    signature: [2; 96].to_vec(),
                })
            });
        run_chain_for_strategy(&platform, 100, 3000, strategy, config, 15);
    }

    #[test]
    fn run_chain_insert_one_new_identity_per_block_and_a_document_with_epoch_change() {
        let contract_cbor = json_document_to_cbor(
            "tests/supporting_files/contract/dashpay/dashpay-contract-all-mutable.json",
            Some(PROTOCOL_VERSION),
        )
        .expect("expected to get cbor from a json document");
        let contract = <Contract as DriveContractExt>::from_cbor(&contract_cbor, None)
            .expect("contract should be deserialized");

        let document_op = DocumentOp {
            contract: contract.clone(),
            action: DocumentActionInsert,
            document_type: contract
                .document_type_for_name("contactRequest")
                .expect("expected a profile document type")
                .clone(),
        };

        let strategy = Strategy {
            contracts: vec![contract],
            operations: vec![(
                document_op,
                Frequency {
                    times_per_block_range: 1..2,
                    chance_per_block: None,
                },
            )],
            identities_inserts: Frequency {
                times_per_block_range: 1..2,
                chance_per_block: None,
            },
            total_hpmns: 100,
            upgrading_info: None,
        };
        let config = PlatformConfig {
            verify_sum_trees: true,
            quorum_size: 100,
            validator_set_quorum_rotation_block_count: 100,
            ..Default::default()
        };
        let day_in_ms = 1000 * 60 * 60 * 24;
        let block_count = 120;
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();
        platform
            .core_rpc
            .expect_get_best_chain_lock()
            .returning(move || {
                Ok(CoreChainLock {
                    core_block_height: 10,
                    core_block_hash: [1; 32].to_vec(),
                    signature: [2; 96].to_vec(),
                })
            });
        let outcome =
            run_chain_for_strategy(&platform, block_count, day_in_ms, strategy, config, 15);
        assert_eq!(outcome.identities.len() as u64, block_count);
        assert_eq!(outcome.masternode_identity_balances.len(), 100);
        let all_have_balances = outcome
            .masternode_identity_balances
            .iter()
            .all(|(_, balance)| *balance != 0);
        assert!(all_have_balances, "all masternodes should have a balance");
    }

    #[test]
    fn run_chain_insert_one_new_identity_per_block_document_insertions_and_deletions_with_epoch_change(
    ) {
        let contract_cbor = json_document_to_cbor(
            "tests/supporting_files/contract/dashpay/dashpay-contract-all-mutable.json",
            Some(PROTOCOL_VERSION),
        )
        .expect("expected to get cbor from a json document");
        let contract = <Contract as DriveContractExt>::from_cbor(&contract_cbor, None)
            .expect("contract should be deserialized");

        let document_insertion_op = DocumentOp {
            contract: contract.clone(),
            action: DocumentActionInsert,
            document_type: contract
                .document_type_for_name("contactRequest")
                .expect("expected a profile document type")
                .clone(),
        };

        let document_deletion_op = DocumentOp {
            contract: contract.clone(),
            action: DocumentActionDelete,
            document_type: contract
                .document_type_for_name("contactRequest")
                .expect("expected a profile document type")
                .clone(),
        };

        let strategy = Strategy {
            contracts: vec![contract],
            operations: vec![
                (
                    document_insertion_op,
                    Frequency {
                        times_per_block_range: 1..2,
                        chance_per_block: None,
                    },
                ),
                (
                    document_deletion_op,
                    Frequency {
                        times_per_block_range: 1..2,
                        chance_per_block: None,
                    },
                ),
            ],
            identities_inserts: Frequency {
                times_per_block_range: 1..2,
                chance_per_block: None,
            },
            total_hpmns: 100,
            upgrading_info: None,
        };
        let config = PlatformConfig {
            verify_sum_trees: true,
            quorum_size: 100,
            validator_set_quorum_rotation_block_count: 100,
            ..Default::default()
        };
        let day_in_ms = 1000 * 60 * 60 * 24;
        let block_count = 120;
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();
        platform
            .core_rpc
            .expect_get_best_chain_lock()
            .returning(move || {
                Ok(CoreChainLock {
                    core_block_height: 10,
                    core_block_hash: [1; 32].to_vec(),
                    signature: [2; 96].to_vec(),
                })
            });
        let outcome =
            run_chain_for_strategy(&platform, block_count, day_in_ms, strategy, config, 15);
        assert_eq!(outcome.identities.len() as u64, block_count);
        assert_eq!(outcome.masternode_identity_balances.len(), 100);
        let all_have_balances = outcome
            .masternode_identity_balances
            .iter()
            .all(|(_, balance)| *balance != 0);
        assert!(all_have_balances, "all masternodes should have a balance");
    }

    #[test]
    fn run_chain_insert_one_new_identity_per_block_many_document_insertions_and_deletions_with_epoch_change(
    ) {
        let contract_cbor = json_document_to_cbor(
            "tests/supporting_files/contract/dashpay/dashpay-contract-all-mutable.json",
            Some(PROTOCOL_VERSION),
        )
        .expect("expected to get cbor from a json document");
        let contract = <Contract as DriveContractExt>::from_cbor(&contract_cbor, None)
            .expect("contract should be deserialized");

        let document_insertion_op = DocumentOp {
            contract: contract.clone(),
            action: DocumentActionInsert,
            document_type: contract
                .document_type_for_name("contactRequest")
                .expect("expected a profile document type")
                .clone(),
        };

        let document_deletion_op = DocumentOp {
            contract: contract.clone(),
            action: DocumentActionDelete,
            document_type: contract
                .document_type_for_name("contactRequest")
                .expect("expected a profile document type")
                .clone(),
        };

        let strategy = Strategy {
            contracts: vec![contract],
            operations: vec![
                (
                    document_insertion_op,
                    Frequency {
                        times_per_block_range: 1..10,
                        chance_per_block: None,
                    },
                ),
                (
                    document_deletion_op,
                    Frequency {
                        times_per_block_range: 1..4,
                        chance_per_block: None,
                    },
                ),
            ],
            identities_inserts: Frequency {
                times_per_block_range: 1..2,
                chance_per_block: None,
            },
            total_hpmns: 100,
            upgrading_info: None,
        };
        let config = PlatformConfig {
            verify_sum_trees: true,
            quorum_size: 100,
            validator_set_quorum_rotation_block_count: 100,
            ..Default::default()
        };
        let day_in_ms = 1000 * 60 * 60 * 24;
        let block_count = 120;
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();
        platform
            .core_rpc
            .expect_get_best_chain_lock()
            .returning(move || {
                Ok(CoreChainLock {
                    core_block_height: 10,
                    core_block_hash: [1; 32].to_vec(),
                    signature: [2; 96].to_vec(),
                })
            });
        let outcome =
            run_chain_for_strategy(&platform, block_count, day_in_ms, strategy, config, 15);
        assert_eq!(outcome.identities.len() as u64, block_count);
        assert_eq!(outcome.masternode_identity_balances.len(), 100);
        let all_have_balances = outcome
            .masternode_identity_balances
            .iter()
            .all(|(_, balance)| *balance != 0);
        assert!(all_have_balances, "all masternodes should have a balance");
    }

    #[test]
    fn run_chain_insert_many_new_identity_per_block_many_document_insertions_and_deletions_with_epoch_change(
    ) {
        let contract_cbor = json_document_to_cbor(
            "tests/supporting_files/contract/dashpay/dashpay-contract-all-mutable.json",
            Some(PROTOCOL_VERSION),
        )
        .expect("expected to get cbor from a json document");
        let contract = <Contract as DriveContractExt>::from_cbor(&contract_cbor, None)
            .expect("contract should be deserialized");

        let document_insertion_op = DocumentOp {
            contract: contract.clone(),
            action: DocumentActionInsert,
            document_type: contract
                .document_type_for_name("contactRequest")
                .expect("expected a profile document type")
                .clone(),
        };

        let document_deletion_op = DocumentOp {
            contract: contract.clone(),
            action: DocumentActionDelete,
            document_type: contract
                .document_type_for_name("contactRequest")
                .expect("expected a profile document type")
                .clone(),
        };

        let strategy = Strategy {
            contracts: vec![contract],
            operations: vec![
                (
                    document_insertion_op,
                    Frequency {
                        times_per_block_range: 1..40,
                        chance_per_block: None,
                    },
                ),
                (
                    document_deletion_op,
                    Frequency {
                        times_per_block_range: 1..15,
                        chance_per_block: None,
                    },
                ),
            ],
            identities_inserts: Frequency {
                times_per_block_range: 1..30,
                chance_per_block: None,
            },
            total_hpmns: 100,
            upgrading_info: None,
        };
        let config = PlatformConfig {
            verify_sum_trees: true,
            quorum_size: 100,
            validator_set_quorum_rotation_block_count: 100,
            ..Default::default()
        };
        let day_in_ms = 1000 * 60 * 60 * 24;
        let block_count = 30;
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();
        platform
            .core_rpc
            .expect_get_best_chain_lock()
            .returning(move || {
                Ok(CoreChainLock {
                    core_block_height: 10,
                    core_block_hash: [1; 32].to_vec(),
                    signature: [2; 96].to_vec(),
                })
            });
        let outcome =
            run_chain_for_strategy(&platform, block_count, day_in_ms, strategy, config, 15);
        assert_eq!(outcome.identities.len() as u64, 398);
        assert_eq!(outcome.masternode_identity_balances.len(), 100);
        let balance_count = outcome
            .masternode_identity_balances
            .into_iter()
            .filter(|(_, balance)| *balance != 0)
            .count();
        assert_eq!(balance_count, 19); // 1 epoch worth of proposers
    }
}
