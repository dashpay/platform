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

extern crate core;

use anyhow::anyhow;
use dashcore_rpc::dashcore::{signer, Network, PrivateKey, ProTxHash, QuorumHash};
use dashcore_rpc::dashcore_rpc_json::{
    DMNState, ExtendedQuorumDetails, MasternodeListDiffWithMasternodes, MasternodeListItem,
    MasternodeType, QuorumInfoResult, QuorumType,
};
use dpp::bls_signatures::PrivateKey as BlsPrivateKey;
use dpp::data_contract::state_transition::data_contract_create_transition::DataContractCreateTransition;
use dpp::document::document_transition::document_base_transition::DocumentBaseTransition;
use dpp::document::document_transition::{
    Action, DocumentCreateTransition, DocumentDeleteTransition, DocumentReplaceTransition,
};
use dpp::document::DocumentsBatchTransition;
use dpp::identity::signer::Signer;

use dpp::block::block_info::BlockInfo;
use dpp::identity::state_transition::identity_create_transition::IdentityCreateTransition;
use dpp::identity::state_transition::identity_topup_transition::IdentityTopUpTransition;
use dpp::identity::KeyType::ECDSA_SECP256K1;
use dpp::identity::{IdentityPublicKey, KeyType, Purpose, SecurityLevel};
use dpp::platform_value::BinaryData;
use dpp::state_transition::errors::{
    InvalidIdentityPublicKeyTypeError, InvalidSignaturePublicKeyError,
};
use dpp::state_transition::{StateTransition, StateTransitionIdentitySigned, StateTransitionType};
use dpp::tests::fixtures::instant_asset_lock_proof_fixture;
use dpp::version::LATEST_VERSION;
use dpp::{bls_signatures, ed25519_dalek, NativeBlsModule, ProtocolError};

use drive::contract::{Contract, CreateRandomDocument, DocumentType};
use drive::dpp::document::Document;
use drive::dpp::identity::{Identity, KeyID};
use drive::dpp::util::deserializer::ProtocolVersion;
use drive::drive::defaults::PROTOCOL_VERSION;
use drive::drive::flags::StorageFlags::SingleEpoch;

use crate::FinalizeBlockOperation::IdentityAddKeys;
use dashcore_rpc::dashcore::hashes::hex::ToHex;
use dashcore_rpc::dashcore::hashes::Hash;
use dashcore_rpc::dashcore::secp256k1::SecretKey;

use dpp::block::epoch::Epoch;
use dpp::data_contract::document_type::random_document_type::RandomDocumentTypeParameters;
use dpp::data_contract::generate_data_contract_id;
use dpp::data_contract::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use dpp::ed25519_dalek::Signer as EddsaSigner;
use dpp::identity::core_script::CoreScript;
use dpp::identity::state_transition::identity_credit_withdrawal_transition::{
    IdentityCreditWithdrawalTransition, Pooling,
};
use dpp::identity::state_transition::identity_update_transition::identity_update_transition::IdentityUpdateTransition;
use dpp::identity::Purpose::AUTHENTICATION;
use dpp::identity::SecurityLevel::{CRITICAL, MASTER};
use dpp::prelude::Identifier;
use drive::drive::identity::key::fetch::{IdentityKeysRequest, KeyRequestType};
use drive::drive::Drive;
use drive::fee::credits::Credits;
use drive::query::DriveQuery;
use drive_abci::abci::AbciApplication;
use drive_abci::execution::fee_pools::epoch::{EpochInfo, EPOCH_CHANGE_TIME_MS};

use dashcore_rpc::json::RemovedMasternodeItem;
use drive_abci::execution::test_quorum::TestQuorumInfo;
use drive_abci::platform::Platform;
use drive_abci::rpc::core::MockCoreRPCLike;
use drive_abci::test::fixture::abci::static_init_chain_request;
use drive_abci::test::helpers::setup::TestPlatformBuilder;
use drive_abci::{config::PlatformConfig, test::helpers::setup::TempPlatform};
use rand::prelude::IteratorRandom;
use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};
use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::net::SocketAddr;
use std::ops::Range;
use std::str::FromStr;
use tenderdash_abci::proto::abci::ValidatorSetUpdate;
use tenderdash_abci::proto::crypto::public_key::Sum::Bls12381;

mod upgrade_fork_tests;

#[derive(Clone, Debug, Default)]
pub struct Frequency {
    pub times_per_block_range: Range<u16>, //insertion count when block is chosen
    pub chance_per_block: Option<f64>,     //chance of insertion if set
}

impl Frequency {
    fn is_set(&self) -> bool {
        self.chance_per_block.is_some() || !self.times_per_block_range.is_empty()
    }

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

    fn events_if_hit(&self, rng: &mut StdRng) -> u16 {
        if self.check_hit(rng) {
            self.events(rng)
        } else {
            0
        }
    }

    fn average_event_count(&self) -> f64 {
        if let Some(chance_per_block) = self.chance_per_block {
            let avg_times_per_block_range =
                (self.times_per_block_range.start + self.times_per_block_range.end) as f64 / 2.0;
            avg_times_per_block_range * chance_per_block
        } else {
            (self.times_per_block_range.start + self.times_per_block_range.end) as f64 / 2.0
        }
    }
}

#[derive(Clone, Debug)]
pub enum DocumentAction {
    DocumentActionInsert,
    DocumentActionDelete,
    DocumentActionReplace,
}

#[derive(Clone, Debug)]
pub struct DocumentOp {
    pub contract: Contract,
    pub document_type: DocumentType,
    pub action: DocumentAction,
}

#[derive(Clone, Debug)]
pub struct Operation {
    op_type: OperationType,
    frequency: Frequency,
}

#[derive(Clone, Debug)]
pub enum IdentityUpdateOp {
    IdentityUpdateAddKeys(u16),
    IdentityUpdateDisableKey(u16),
}

pub type DocumentTypeNewFieldsOptionalCountRange = Range<u16>;
pub type DocumentTypeCount = Range<u16>;

#[derive(Clone, Debug)]
pub enum DataContractUpdateOp {
    DataContractNewDocumentTypes(RandomDocumentTypeParameters), // How many fields should it have
    DataContractNewOptionalFields(DocumentTypeNewFieldsOptionalCountRange, DocumentTypeCount), // How many new fields on how many document types
}

#[derive(Clone, Debug)]
pub enum OperationType {
    Document(DocumentOp),
    IdentityTopUp,
    IdentityUpdate(IdentityUpdateOp),
    IdentityWithdrawal,
    ContractCreate(RandomDocumentTypeParameters, DocumentTypeCount),
    ContractUpdate(DataContractUpdateOp),
}

#[derive(Clone, Debug)]
pub enum FinalizeBlockOperation {
    IdentityAddKeys(Identifier, Vec<IdentityPublicKey>),
}

/// This simple signer is only to be used in tests
#[derive(Default, Debug)]
pub struct SimpleSigner {
    /// Private keys is a map from the public key to the Private key bytes
    private_keys: HashMap<IdentityPublicKey, Vec<u8>>,
    /// Private keys to be added at the end of a block
    private_keys_in_creation: HashMap<IdentityPublicKey, Vec<u8>>,
}

impl SimpleSigner {
    fn add_key(&mut self, public_key: IdentityPublicKey, private_key: Vec<u8>) {
        self.private_keys.insert(public_key, private_key);
    }

    fn add_keys<I: IntoIterator<Item = (IdentityPublicKey, Vec<u8>)>>(&mut self, keys: I) {
        self.private_keys.extend(keys)
    }

    fn commit_block_keys(&mut self) {
        self.private_keys
            .extend(self.private_keys_in_creation.drain())
    }
}

impl Signer for SimpleSigner {
    fn sign(
        &self,
        identity_public_key: &IdentityPublicKey,
        data: &[u8],
    ) -> Result<BinaryData, ProtocolError> {
        let private_key = self
            .private_keys
            .get(identity_public_key)
            .or_else(|| self.private_keys_in_creation.get(identity_public_key))
            .ok_or(ProtocolError::InvalidSignaturePublicKeyError(
                InvalidSignaturePublicKeyError::new(identity_public_key.data.to_vec()),
            ))?;
        match identity_public_key.key_type {
            KeyType::ECDSA_SECP256K1 | KeyType::ECDSA_HASH160 => {
                let signature = signer::sign(data, private_key)?;
                Ok(signature.to_vec().into())
            }
            KeyType::BLS12_381 => {
                let pk =
                    bls_signatures::PrivateKey::from_bytes(private_key, false).map_err(|_e| {
                        ProtocolError::Error(anyhow!("bls private key from bytes isn't correct"))
                    })?;
                Ok(pk.sign(data).to_bytes().to_vec().into())
            }
            KeyType::EDDSA_25519_HASH160 => {
                let key: [u8; 32] = private_key.clone().try_into().expect("expected 32 bytes");
                let pk = ed25519_dalek::SigningKey::try_from(&key).map_err(|_e| {
                    ProtocolError::Error(anyhow!(
                        "eddsa 25519 private key from bytes isn't correct"
                    ))
                })?;
                Ok(pk.sign(data).to_vec().into())
            }
            // the default behavior from
            // https://github.com/dashevo/platform/blob/6b02b26e5cd3a7c877c5fdfe40c4a4385a8dda15/packages/js-dpp/lib/stateTransition/AbstractStateTransition.js#L187
            // is to return the error for the BIP13_SCRIPT_HASH
            KeyType::BIP13_SCRIPT_HASH => Err(ProtocolError::InvalidIdentityPublicKeyTypeError(
                InvalidIdentityPublicKeyTypeError::new(identity_public_key.key_type),
            )),
        }
    }
}

pub type BlockHeight = u64;

#[derive(Clone, Debug)]
pub struct Strategy {
    contracts_with_updates: Vec<(Contract, Option<BTreeMap<u64, Contract>>)>,
    operations: Vec<Operation>,
    identities_inserts: Frequency,
    total_hpmns: u16,
    extra_normal_mns: u16,
    quorum_count: u16,
    upgrading_info: Option<UpgradingInfo>,
    core_height_increase: Frequency,
    /// how many new proposers on average per core chain lock increase
    new_proposers: Frequency,
    /// how many proposers leave the system
    removed_proposers: Frequency,
    rotate_quorums: bool,
}

#[derive(Clone, Debug)]
pub struct UpgradingInfo {
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
    // TODO: This belongs to `DocumentOp`
    fn add_strategy_contracts_into_drive(&mut self, drive: &Drive) {
        for op in &self.operations {
            match &op.op_type {
                OperationType::Document(doc_op) => {
                    let serialize = doc_op.contract.to_cbor().expect("expected to serialize");
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

    fn identity_state_transitions_for_block(
        &self,
        signer: &mut SimpleSigner,
        rng: &mut StdRng,
    ) -> Vec<(Identity, StateTransition)> {
        let frequency = &self.identities_inserts;
        if frequency.check_hit(rng) {
            let count = frequency.events(rng);
            create_identities_state_transitions(count, 5, signer, rng)
        } else {
            vec![]
        }
    }

    fn contract_state_transitions(
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

    fn contract_update_state_transitions(
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
    fn state_transitions_for_block(
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
                                    updated_at: document.created_at,
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
                            operations
                                .push(create_identity_top_up_transition(rng, random_identity));
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
                                        create_identity_update_transition_add_keys(
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
                                        create_identity_update_transition_disable_keys(
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
                                create_identity_withdrawal_transition(random_identity, signer, rng);
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

    fn state_transitions_for_block_with_new_identities(
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

fn create_identity_top_up_transition(rng: &mut StdRng, identity: &Identity) -> StateTransition {
    let (_, pk) = ECDSA_SECP256K1.random_public_and_private_key_data(rng);
    let sk: [u8; 32] = pk.try_into().unwrap();
    let secret_key = SecretKey::from_str(hex::encode(sk).as_str()).unwrap();
    let asset_lock_proof =
        instant_asset_lock_proof_fixture(Some(PrivateKey::new(secret_key, Network::Dash)));

    StateTransition::IdentityTopUp(
        IdentityTopUpTransition::try_from_identity(
            identity.clone(),
            asset_lock_proof,
            secret_key.as_ref(),
            &NativeBlsModule::default(),
        )
        .expect("expected to create top up transition"),
    )
}

fn create_identity_update_transition_add_keys(
    identity: &mut Identity,
    count: u16,
    signer: &mut SimpleSigner,
    rng: &mut StdRng,
) -> (StateTransition, (Identifier, Vec<IdentityPublicKey>)) {
    identity.revision += 1;
    let keys = IdentityPublicKey::random_authentication_keys_with_private_keys_with_rng(
        identity.public_keys.len() as KeyID,
        count as u32,
        rng,
    );

    let add_public_keys: Vec<IdentityPublicKey> = keys.iter().map(|(key, _)| key.clone()).collect();
    signer.private_keys_in_creation.extend(keys);
    let (key_id, _) = identity
        .public_keys
        .iter()
        .find(|(_, key)| key.security_level == MASTER)
        .expect("expected to have a master key");

    let state_transition = StateTransition::IdentityUpdate(
        IdentityUpdateTransition::try_from_identity_with_signer(
            identity,
            key_id,
            add_public_keys.clone(),
            vec![],
            None,
            signer,
        )
        .expect("expected to create top up transition"),
    );

    (state_transition, (identity.id, add_public_keys))
}

fn create_identity_update_transition_disable_keys(
    identity: &mut Identity,
    count: u16,
    block_time: u64,
    signer: &mut SimpleSigner,
    rng: &mut StdRng,
) -> Option<StateTransition> {
    identity.revision += 1;
    // we want to find keys that are not disabled
    let key_ids_we_could_disable = identity
        .public_keys
        .iter()
        .filter(|(_, key)| {
            key.disabled_at.is_none()
                && (key.security_level != MASTER
                    && !(key.security_level == CRITICAL
                        && key.purpose == AUTHENTICATION
                        && key.key_type == ECDSA_SECP256K1))
        })
        .map(|(key_id, _)| *key_id)
        .collect::<Vec<_>>();

    if key_ids_we_could_disable.is_empty() {
        identity.revision -= 1; //since we added 1 before
        return None;
    }
    let indices: Vec<_> = (0..key_ids_we_could_disable.len()).choose_multiple(rng, count as usize);

    let key_ids_to_disable: Vec<_> = indices
        .into_iter()
        .map(|index| key_ids_we_could_disable[index])
        .collect();

    identity.public_keys.iter_mut().for_each(|(key_id, key)| {
        if key_ids_to_disable.contains(key_id) {
            key.disabled_at = Some(block_time);
        }
    });

    let (key_id, _) = identity
        .public_keys
        .iter()
        .find(|(_, key)| key.security_level == MASTER)
        .expect("expected to have a master key");

    let state_transition = StateTransition::IdentityUpdate(
        IdentityUpdateTransition::try_from_identity_with_signer(
            identity,
            key_id,
            vec![],
            key_ids_to_disable,
            Some(block_time),
            signer,
        )
        .expect("expected to create top up transition"),
    );

    Some(state_transition)
}

fn create_identity_withdrawal_transition(
    identity: &mut Identity,
    signer: &mut SimpleSigner,
    rng: &mut StdRng,
) -> StateTransition {
    identity.revision += 1;
    let mut withdrawal = IdentityCreditWithdrawalTransition {
        protocol_version: LATEST_VERSION,
        transition_type: StateTransitionType::IdentityCreditWithdrawal,
        identity_id: identity.id,
        amount: 100000000, // 0.001 Dash
        core_fee_per_byte: 1,
        pooling: Pooling::Never,
        output_script: CoreScript::random_p2sh(rng),
        revision: identity.revision,
        signature_public_key_id: 0,
        signature: Default::default(),
    };

    let identity_public_key = identity
        .get_first_public_key_matching(
            Purpose::AUTHENTICATION,
            HashSet::from([SecurityLevel::HIGH, SecurityLevel::CRITICAL]),
            HashSet::from([KeyType::ECDSA_SECP256K1, KeyType::BLS12_381]),
        )
        .expect("expected to get a signing key");

    withdrawal
        .sign_external(identity_public_key, signer)
        .expect("expected to sign withdrawal");

    withdrawal.into()
}

fn create_identities_state_transitions(
    count: u16,
    key_count: KeyID,
    signer: &mut SimpleSigner,
    rng: &mut StdRng,
) -> Vec<(Identity, StateTransition)> {
    let (identities, keys) =
        Identity::random_identities_with_private_keys_with_rng::<Vec<_>>(count, key_count, rng)
            .expect("expected to create identities");
    signer.add_keys(keys);
    identities
        .into_iter()
        .map(|mut identity| {
            let (_, pk) = ECDSA_SECP256K1.random_public_and_private_key_data(rng);
            let sk: [u8; 32] = pk.clone().try_into().unwrap();
            let secret_key = SecretKey::from_str(hex::encode(sk).as_str()).unwrap();
            let asset_lock_proof =
                instant_asset_lock_proof_fixture(Some(PrivateKey::new(secret_key, Network::Dash)));
            let identity_create_transition =
                IdentityCreateTransition::try_from_identity_with_signer(
                    identity.clone(),
                    asset_lock_proof,
                    pk.as_slice(),
                    signer,
                    &NativeBlsModule::default(),
                )
                .expect("expected to transform identity into identity create transition");
            identity.id = *identity_create_transition.get_identity_id();
            (identity, identity_create_transition.into())
        })
        .collect()
}

pub struct ChainExecutionOutcome<'a> {
    pub abci_app: AbciApplication<'a, MockCoreRPCLike>,
    pub masternode_identity_balances: BTreeMap<[u8; 32], Credits>,
    pub identities: Vec<Identity>,
    pub proposers: Vec<MasternodeListItem>,
    pub quorums: BTreeMap<QuorumHash, TestQuorumInfo>,
    pub current_quorum_hash: QuorumHash,
    pub current_proposer_versions: Option<HashMap<ProTxHash, ValidatorVersionMigration>>,
    pub end_epoch_index: u16,
    pub end_time_ms: u64,
    pub strategy: Strategy,
    pub withdrawals: Vec<dashcore::Transaction>,
}

pub struct ChainExecutionParameters {
    pub block_start: u64,
    pub core_height_start: u32,
    pub block_count: u64,
    pub proposers: Vec<MasternodeListItem>,
    pub quorums: BTreeMap<QuorumHash, TestQuorumInfo>,
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

/// Creates a list of test Masternode identities of size `count` with random data
pub fn generate_test_masternodes(
    masternode_count: u16,
    hpmn_count: u16,
    rng: &mut StdRng,
) -> Vec<MasternodeListItem> {
    let mut masternodes: Vec<MasternodeListItem> =
        Vec::with_capacity((masternode_count + hpmn_count) as usize);

    for i in 0..masternode_count {
        let private_key_operator =
            BlsPrivateKey::generate_dash(rng).expect("expected to generate a private key");
        let pub_key_operator = private_key_operator
            .g1_element()
            .expect("expected to get public key")
            .to_bytes()
            .to_vec();
        let masternode_list_item = MasternodeListItem {
            node_type: MasternodeType::Regular,
            protx_hash: ProTxHash::from_inner(rng.gen::<[u8; 32]>()),
            collateral_hash: rng.gen::<[u8; 32]>(),
            collateral_index: 0,
            operator_reward: 0,
            state: DMNState {
                service: SocketAddr::from_str(format!("1.0.{}.{}:1234", i / 256, i % 256).as_str())
                    .unwrap(),
                registered_height: 0,
                pose_revived_height: 0,
                pose_ban_height: 0,
                revocation_reason: 0,
                owner_address: rng.gen::<[u8; 20]>(),
                voting_address: rng.gen::<[u8; 20]>(),
                payout_address: rng.gen::<[u8; 20]>(),
                pub_key_operator,
                operator_payout_address: None,
                platform_node_id: None,
            },
        };
        masternodes.push(masternode_list_item);
    }

    for i in 0..hpmn_count {
        let private_key_operator =
            BlsPrivateKey::generate_dash(rng).expect("expected to generate a private key");
        let pub_key_operator = private_key_operator
            .g1_element()
            .expect("expected to get public key")
            .to_bytes()
            .to_vec();
        let masternode_list_item = MasternodeListItem {
            node_type: MasternodeType::HighPerformance,
            protx_hash: ProTxHash::from_inner(rng.gen::<[u8; 32]>()),
            collateral_hash: rng.gen::<[u8; 32]>(),
            collateral_index: 0,
            operator_reward: 0,
            state: DMNState {
                service: SocketAddr::from_str(format!("1.1.{}.{}:1234", i / 256, i % 256).as_str())
                    .unwrap(),
                registered_height: 0,
                pose_revived_height: 0,
                pose_ban_height: 0,
                revocation_reason: 0,
                owner_address: rng.gen::<[u8; 20]>(),
                voting_address: rng.gen::<[u8; 20]>(),
                payout_address: rng.gen::<[u8; 20]>(),
                pub_key_operator,
                operator_payout_address: None,
                platform_node_id: Some(rng.gen::<[u8; 20]>()),
            },
        };
        masternodes.push(masternode_list_item);
    }

    masternodes
}

pub fn generate_test_quorums(
    count: usize,
    proposers: &Vec<MasternodeListItem>,
    quorum_size: usize,
    rng: &mut StdRng,
) -> BTreeMap<QuorumHash, TestQuorumInfo> {
    (0..count)
        .into_iter()
        .map(|_| {
            let quorum_hash: QuorumHash = QuorumHash::from_inner(rng.gen());
            let validator_pro_tx_hashes = proposers
                .iter()
                .filter(|m| m.node_type == MasternodeType::HighPerformance)
                .choose_multiple(rng, quorum_size)
                .iter()
                .map(|masternode| masternode.protx_hash)
                .collect(); //choose multiple chooses out of order (as we would like)
            (
                quorum_hash,
                TestQuorumInfo::from_quorum_hash_and_pro_tx_hashes(
                    quorum_hash,
                    validator_pro_tx_hashes,
                    rng,
                ),
            )
        })
        .collect()
}

pub(crate) fn run_chain_for_strategy(
    platform: &mut Platform<MockCoreRPCLike>,
    block_count: u64,
    strategy: Strategy,
    config: PlatformConfig,
    seed: u64,
) -> ChainExecutionOutcome {
    let quorum_count = strategy.quorum_count; // We assume 24 quorums
    let quorum_size = config.quorum_size;

    let mut rng = StdRng::seed_from_u64(seed);

    let new_proposers_in_strategy = strategy.new_proposers.is_set();
    let removed_proposers_in_strategy = strategy.removed_proposers.is_set();
    let (initial_proposers, with_extra_proposers) = if new_proposers_in_strategy
        || removed_proposers_in_strategy
    {
        let initial_proposers =
            generate_test_masternodes(strategy.extra_normal_mns, strategy.total_hpmns, &mut rng);
        let mut all_proposers = initial_proposers.clone();
        let approximate_end_core_height =
            ((block_count as f64) * strategy.core_height_increase.average_event_count()) as u32;
        let end_core_height = approximate_end_core_height * 2; //let's be safe
        let extra_proposers_by_block = (config.abci.genesis_core_height..end_core_height)
            .map(|height| {
                let new_proposers = strategy.new_proposers.events_if_hit(&mut rng);
                let removed_proposers_count = strategy.removed_proposers.events_if_hit(&mut rng);
                let extra_proposers_by_block =
                    generate_test_masternodes(0, new_proposers, &mut rng);

                if removed_proposers_in_strategy {
                    let removed_count =
                        std::cmp::min(removed_proposers_count as usize, all_proposers.len());
                    all_proposers.drain(0..removed_count);
                }

                all_proposers.extend(extra_proposers_by_block.clone());
                (height, all_proposers.clone())
            })
            .collect::<HashMap<u32, Vec<MasternodeListItem>>>();
        (initial_proposers, extra_proposers_by_block)
    } else {
        (
            generate_test_masternodes(strategy.extra_normal_mns, strategy.total_hpmns, &mut rng),
            HashMap::new(),
        )
    };

    let mut all_proposers = with_extra_proposers
        .iter()
        .max_by_key(|(key, _)| *key)
        .map(|(_, v)| v.clone())
        .unwrap_or(initial_proposers.clone());

    let total_quorums = if strategy.rotate_quorums {
        quorum_count * 10
    } else {
        quorum_count
    };

    let quorums = generate_test_quorums(
        total_quorums as usize,
        &initial_proposers,
        quorum_size as usize,
        &mut rng,
    );

    let quorums_clone: HashMap<QuorumHash, ExtendedQuorumDetails> = quorums
        .keys()
        .map(|quorum_hash| {
            (
                *quorum_hash,
                ExtendedQuorumDetails {
                    creation_height: 0,
                    quorum_index: None,
                    mined_block_hash: Default::default(),
                    num_valid_members: 0,
                    health_ratio: 0.0,
                },
            )
        })
        .collect();

    platform
        .core_rpc
        .expect_get_quorum_listextended()
        .returning(move |core_height: Option<u32>| {
            if !strategy.rotate_quorums {
                Ok(dashcore_rpc::dashcore_rpc_json::QuorumListResult {
                    quorums_by_type: HashMap::from([(
                        QuorumType::Llmq100_67,
                        quorums_clone.clone(),
                    )]),
                })
            } else {
                let core_height = core_height.expect("expected a core height");
                // if we rotate quorums we shouldn't give back the same ones every time
                let start_range = core_height / 24;
                let end_range = start_range + quorum_count as u32;
                let start_range = start_range % total_quorums as u32;
                let end_range = end_range % total_quorums as u32;

                let quorums = if end_range > start_range {
                    quorums_clone
                        .iter()
                        .skip(start_range as usize)
                        .take((end_range - start_range) as usize)
                        .map(|(quorum_hash, quorum)| (quorum_hash.clone(), quorum.clone()))
                        .collect()
                } else {
                    let first_range = quorums_clone
                        .iter()
                        .skip(start_range as usize)
                        .take((total_quorums as u32 - start_range) as usize);
                    let second_range = quorums_clone.iter().take(end_range as usize);
                    first_range
                        .chain(second_range)
                        .map(|(quorum_hash, quorum)| (quorum_hash.clone(), quorum.clone()))
                        .collect()
                };

                Ok(dashcore_rpc::dashcore_rpc_json::QuorumListResult {
                    quorums_by_type: HashMap::from([(QuorumType::Llmq100_67, quorums)]),
                })
            }
        });

    let quorums_info: HashMap<QuorumHash, QuorumInfoResult> = quorums
        .iter()
        .map(|(quorum_hash, test_quorum_info)| (*quorum_hash, test_quorum_info.into()))
        .collect();

    platform
        .core_rpc
        .expect_get_quorum_info()
        .returning(move |_, quorum_hash: &QuorumHash, _| {
            Ok(quorums_info
                .get(quorum_hash)
                .unwrap_or_else(|| panic!("expected to get quorum {}", quorum_hash.to_hex()))
                .clone())
        });

    platform
        .core_rpc
        .expect_get_protx_diff_with_masternodes()
        .returning(move |base_block, block| {
            let diff = if base_block == 0 {
                MasternodeListDiffWithMasternodes {
                    base_height: base_block,
                    block_height: block,
                    added_mns: initial_proposers.clone(),
                    removed_mns: vec![],
                    updated_mns: vec![],
                }
            } else {
                if !new_proposers_in_strategy {
                    // no changes
                    MasternodeListDiffWithMasternodes {
                        base_height: base_block,
                        block_height: block,
                        added_mns: vec![],
                        removed_mns: vec![],
                        updated_mns: vec![],
                    }
                } else {
                    // we need to figure out the difference of proposers between two heights
                    let start_proposers = with_extra_proposers
                        .get(&base_block)
                        .expect("expected start proposers");
                    let end_proposers = with_extra_proposers
                        .get(&block)
                        .expect("expected end proposers");
                    let added_mns: Vec<_> = end_proposers
                        .iter()
                        .filter(|item| !start_proposers.contains(item))
                        .map(|a| a.clone())
                        .collect();

                    let removed_mns: Vec<_> = start_proposers
                        .iter()
                        .filter(|item| !end_proposers.contains(item))
                        .map(|masternode_list_item| RemovedMasternodeItem {
                            protx_hash: masternode_list_item.protx_hash,
                        })
                        .collect();

                    MasternodeListDiffWithMasternodes {
                        base_height: base_block,
                        block_height: block,
                        added_mns,
                        removed_mns,
                        updated_mns: vec![],
                    }
                }
            };

            Ok(diff)
        });

    start_chain_for_strategy(
        platform,
        block_count,
        all_proposers,
        quorums,
        strategy,
        config,
        rng,
    )
}

pub(crate) fn start_chain_for_strategy(
    platform: &Platform<MockCoreRPCLike>,
    block_count: u64,
    proposers: Vec<MasternodeListItem>,
    quorums: BTreeMap<QuorumHash, TestQuorumInfo>,
    strategy: Strategy,
    config: PlatformConfig,
    mut rng: StdRng,
) -> ChainExecutionOutcome {
    let abci_application = AbciApplication::new(platform).expect("expected new abci application");

    let quorum_hashes: Vec<&QuorumHash> = quorums.keys().collect();

    let current_quorum_hash = **quorum_hashes
        .choose(&mut rng)
        .expect("expected quorums to be initialized");

    let current_quorum_with_test_info = quorums
        .get(&current_quorum_hash)
        .expect("expected a quorum to be found");

    // init chain
    let mut init_chain_request = static_init_chain_request();

    init_chain_request.initial_core_height = config.abci.genesis_core_height;
    init_chain_request.validator_set = Some(ValidatorSetUpdate {
        validator_updates: current_quorum_with_test_info
            .validator_set
            .iter()
            .map(
                |validator_in_quorum| tenderdash_abci::proto::abci::ValidatorUpdate {
                    pub_key: Some(tenderdash_abci::proto::crypto::PublicKey {
                        sum: Some(Bls12381(validator_in_quorum.public_key.to_bytes().to_vec())),
                    }),
                    power: 100,
                    pro_tx_hash: validator_in_quorum.pro_tx_hash.to_vec(),
                    node_address: "".to_string(),
                },
            )
            .collect(),
        threshold_public_key: Some(tenderdash_abci::proto::crypto::PublicKey {
            sum: Some(Bls12381(
                current_quorum_with_test_info.public_key.to_bytes().to_vec(),
            )),
        }),
        quorum_hash: current_quorum_hash.to_vec(),
    });

    abci_application.start_transaction();

    let binding = abci_application.transaction.read().unwrap();

    let transaction = binding.as_ref().expect("expected a transaction");

    platform
        .init_chain(init_chain_request, transaction)
        .expect("should init chain");

    platform.create_mn_shares_contract(Some(transaction));

    drop(binding);

    continue_chain_for_strategy(
        abci_application,
        ChainExecutionParameters {
            block_start: 1,
            core_height_start: config.abci.genesis_core_height,
            block_count,
            proposers,
            quorums,
            current_quorum_hash,
            current_proposer_versions: None,
            current_time_ms: 1681094380000,
        },
        strategy,
        config,
        StrategyRandomness::RNGEntropy(rng),
    )
}

pub(crate) fn continue_chain_for_strategy(
    abci_app: AbciApplication<MockCoreRPCLike>,
    chain_execution_parameters: ChainExecutionParameters,
    mut strategy: Strategy,
    config: PlatformConfig,
    seed: StrategyRandomness,
) -> ChainExecutionOutcome {
    let platform = abci_app.platform;
    let ChainExecutionParameters {
        block_start,
        core_height_start,
        block_count,
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

    let blocks_per_epoch = EPOCH_CHANGE_TIME_MS / config.block_spacing_ms;

    let proposer_count = proposers.len() as u32;

    let proposer_versions = current_proposer_versions.unwrap_or(
        strategy.upgrading_info.as_ref().map(|upgrading_info| {
            upgrading_info.apply_to_proposers(
                proposers
                    .iter()
                    .map(|masternode_list_item| masternode_list_item.protx_hash)
                    .collect(),
                blocks_per_epoch,
                &mut rng,
            )
        }),
    );

    let mut current_core_height = core_height_start;

    let mut total_withdrawals = vec![];

    let mut current_quorum_with_test_info = quorums.get(&current_quorum_hash).unwrap();

    let mut next_quorum_hash = current_quorum_hash;

    let mut next_quorum_with_test_info = quorums.get(&next_quorum_hash).unwrap();

    for block_height in block_start..(block_start + block_count) {
        let needs_rotation_on_next_block = block_height % quorum_rotation_block_count == 0;
        if needs_rotation_on_next_block {
            let quorum_hashes: Vec<&QuorumHash> = quorums.keys().collect();

            next_quorum_hash = **quorum_hashes.choose(&mut rng).unwrap();
        }
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

        current_core_height += strategy.core_height_increase.events_if_hit(&mut rng) as u32;

        let block_info = BlockInfo {
            time_ms: current_time_ms,
            height: block_height,
            core_height: current_core_height,
            epoch: Epoch::new(epoch_info.current_epoch_index).unwrap(),
        };
        if current_quorum_with_test_info.quorum_hash != current_quorum_hash {
            current_quorum_with_test_info = quorums.get(&current_quorum_hash).unwrap();
        }

        if next_quorum_with_test_info.quorum_hash != next_quorum_hash {
            next_quorum_with_test_info = quorums.get(&next_quorum_hash).unwrap();
        }

        let proposer = current_quorum_with_test_info
            .validator_set
            .get(i as usize)
            .unwrap();
        let (state_transitions, finalize_block_operations) = strategy
            .state_transitions_for_block_with_new_identities(
                platform,
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
                    .get(&proposer.pro_tx_hash)
                    .expect("expected to have version");
                if &block_height >= change_block_height {
                    *next_protocol_version
                } else {
                    *current_protocol_version
                }
            })
            .unwrap_or(1);

        let mut withdrawals_this_block = abci_app
            .mimic_execute_block(
                proposer.pro_tx_hash.into_inner(),
                current_quorum_with_test_info,
                next_quorum_with_test_info,
                proposed_version,
                proposer_count,
                block_info,
                false,
                state_transitions,
            )
            .expect("expected to execute a block");

        total_withdrawals.append(&mut withdrawals_this_block);

        for finalize_block_operation in finalize_block_operations {
            match finalize_block_operation {
                IdentityAddKeys(identifier, keys) => {
                    let identity = current_identities
                        .iter_mut()
                        .find(|identity| identity.id == identifier)
                        .expect("expected to find an identity");
                    identity
                        .public_keys
                        .extend(keys.into_iter().map(|key| (key.id, key)));
                }
            }
        }
        signer.commit_block_keys();

        current_time_ms += config.block_spacing_ms;
        i += 1;
        i %= quorum_size;
        if needs_rotation_on_next_block {
            current_quorum_hash = next_quorum_hash;
        }
    }

    let masternode_identity_balances = platform
        .drive
        .fetch_identities_balances(
            &proposers
                .iter()
                .map(|proposer| proposer.protx_hash.into_inner())
                .collect(),
            None,
        )
        .expect("expected to get balances");

    let end_epoch_index = platform
        .state
        .read()
        .expect("lock is poisoned")
        .last_committed_block_info
        .as_ref()
        .unwrap()
        .epoch
        .index;

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
        strategy,
        withdrawals: total_withdrawals,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DocumentAction::DocumentActionReplace;
    use dashcore_rpc::dashcore::hashes::Hash;
    use dashcore_rpc::dashcore::BlockHash;
    use dashcore_rpc::dashcore_rpc_json::ExtendedQuorumDetails;
    use drive::dpp::data_contract::extra::common::json_document_to_cbor;
    use drive::dpp::data_contract::DriveContractExt;
    use drive_abci::config::PlatformTestConfig;
    use drive_abci::rpc::core::QuorumListExtendedInfo;
    use tenderdash_abci::proto::types::CoreChainLock;

    pub fn generate_quorums_extended_info(n: u32) -> QuorumListExtendedInfo {
        let mut quorums = QuorumListExtendedInfo::new();

        for i in 0..n {
            let i_bytes = [i as u8; 32];

            let hash = QuorumHash::from_inner(i_bytes);

            let details = ExtendedQuorumDetails {
                creation_height: i,
                health_ratio: (i as f32) / (n as f32),
                mined_block_hash: BlockHash::from_slice(&i_bytes).unwrap(),
                num_valid_members: i,
                quorum_index: Some(i),
            };

            if let Some(v) = quorums.insert(hash, details) {
                panic!("duplicate record {:?}={:?}", hash, v)
            }
        }
        quorums
    }

    #[test]
    fn run_chain_nothing_happening() {
        let strategy = Strategy {
            contracts_with_updates: vec![],
            operations: vec![],
            identities_inserts: Frequency {
                times_per_block_range: Default::default(),
                chance_per_block: None,
            },
            total_hpmns: 100,
            extra_normal_mns: 0,
            quorum_count: 24,
            upgrading_info: None,
            core_height_increase: Frequency {
                times_per_block_range: Default::default(),
                chance_per_block: None,
            },
            new_proposers: Default::default(),
            removed_proposers: Default::default(),
            rotate_quorums: false,
        };
        let config = PlatformConfig {
            verify_sum_trees: true,
            quorum_size: 100,
            validator_set_quorum_rotation_block_count: 25,
            block_spacing_ms: 3000,
            testing_configs: PlatformTestConfig::default_with_no_block_signing(),
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
        run_chain_for_strategy(&mut platform, 100, strategy, config, 15);
    }

    #[test]
    fn run_chain_block_signing() {
        let strategy = Strategy {
            contracts_with_updates: vec![],
            operations: vec![],
            identities_inserts: Frequency {
                times_per_block_range: Default::default(),
                chance_per_block: None,
            },
            total_hpmns: 100,
            extra_normal_mns: 0,
            quorum_count: 24,
            upgrading_info: None,
            core_height_increase: Frequency {
                times_per_block_range: Default::default(),
                chance_per_block: None,
            },
            new_proposers: Default::default(),
            removed_proposers: Default::default(),
            rotate_quorums: false,
        };
        let config = PlatformConfig {
            verify_sum_trees: true,
            quorum_size: 100,
            validator_set_quorum_rotation_block_count: 25,
            block_spacing_ms: 3000,
            testing_configs: PlatformTestConfig::default(),
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
        run_chain_for_strategy(&mut platform, 50, strategy, config, 13);
    }

    #[test]
    fn run_chain_stop_and_restart() {
        let strategy = Strategy {
            contracts_with_updates: vec![],
            operations: vec![],
            identities_inserts: Frequency {
                times_per_block_range: Default::default(),
                chance_per_block: None,
            },
            total_hpmns: 100,
            extra_normal_mns: 0,
            quorum_count: 24,
            upgrading_info: None,
            core_height_increase: Frequency {
                times_per_block_range: Default::default(),
                chance_per_block: None,
            },
            new_proposers: Default::default(),
            removed_proposers: Default::default(),
            rotate_quorums: false,
        };
        let config = PlatformConfig {
            verify_sum_trees: true,
            quorum_size: 100,
            validator_set_quorum_rotation_block_count: 25,
            block_spacing_ms: 3000,
            testing_configs: PlatformTestConfig::default(),
            ..Default::default()
        };
        let TempPlatform {
            mut platform,
            tempdir: _,
        } = TestPlatformBuilder::new()
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

        let ChainExecutionOutcome {
            abci_app,
            proposers,
            quorums,
            current_quorum_hash,
            current_proposer_versions,
            end_time_ms,
            ..
        } = run_chain_for_strategy(&mut platform, 15, strategy.clone(), config.clone(), 40);

        abci_app
            .platform
            .recreate_state()
            .expect("expected to recreate state");

        let block_start = abci_app
            .platform
            .state
            .read()
            .unwrap()
            .last_committed_block_info
            .as_ref()
            .unwrap()
            .height
            + 1;

        continue_chain_for_strategy(
            abci_app,
            ChainExecutionParameters {
                block_start,
                core_height_start: 1,
                block_count: 30,
                proposers,
                quorums,
                current_quorum_hash,
                current_proposer_versions: Some(current_proposer_versions),
                current_time_ms: end_time_ms,
            },
            strategy,
            config,
            StrategyRandomness::SeedEntropy(7),
        );
    }

    #[test]
    fn run_chain_one_identity_in_solitude() {
        let strategy = Strategy {
            contracts_with_updates: vec![],
            operations: vec![],
            identities_inserts: Frequency {
                times_per_block_range: 1..2,
                chance_per_block: None,
            },
            total_hpmns: 100,
            extra_normal_mns: 0,
            quorum_count: 24,
            upgrading_info: None,
            core_height_increase: Frequency {
                times_per_block_range: Default::default(),
                chance_per_block: None,
            },
            new_proposers: Default::default(),
            removed_proposers: Default::default(),
            rotate_quorums: false,
        };
        let config = PlatformConfig {
            verify_sum_trees: true,
            quorum_size: 100,
            validator_set_quorum_rotation_block_count: 25,
            block_spacing_ms: 3000,
            testing_configs: PlatformTestConfig::default_with_no_block_signing(),
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
        let outcome = run_chain_for_strategy(&mut platform, 100, strategy, config, 15);

        let balance = outcome
            .abci_app
            .platform
            .drive
            .fetch_identity_balance(outcome.identities.first().unwrap().id.to_buffer(), None)
            .expect("expected to fetch balances")
            .expect("expected to have an identity to get balance from");

        assert_eq!(balance, 99874449360)
    }

    #[test]
    fn run_chain_core_height_randomly_increasing() {
        let strategy = Strategy {
            contracts_with_updates: vec![],
            operations: vec![],
            identities_inserts: Frequency {
                times_per_block_range: Default::default(),
                chance_per_block: None,
            },
            total_hpmns: 100,
            extra_normal_mns: 0,
            quorum_count: 24,
            upgrading_info: None,
            core_height_increase: Frequency {
                times_per_block_range: 1..3,
                chance_per_block: Some(0.01),
            },
            new_proposers: Default::default(),
            removed_proposers: Default::default(),
            rotate_quorums: false,
        };
        let config = PlatformConfig {
            verify_sum_trees: true,
            quorum_size: 100,
            validator_set_quorum_rotation_block_count: 25,
            block_spacing_ms: 3000,
            testing_configs: PlatformTestConfig::default_with_no_block_signing(),
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
        run_chain_for_strategy(&mut platform, 1000, strategy, config, 15);
    }

    #[test]
    fn run_chain_core_height_randomly_increasing_with_quorum_updates() {
        let strategy = Strategy {
            contracts_with_updates: vec![],
            operations: vec![],
            identities_inserts: Frequency {
                times_per_block_range: Default::default(),
                chance_per_block: None,
            },
            total_hpmns: 500,
            extra_normal_mns: 0,
            quorum_count: 24,
            upgrading_info: None,
            core_height_increase: Frequency {
                times_per_block_range: 5..6,
                chance_per_block: Some(0.5),
            },
            new_proposers: Default::default(),
            removed_proposers: Default::default(),
            rotate_quorums: true,
        };
        let config = PlatformConfig {
            verify_sum_trees: true,
            quorum_size: 10,
            validator_set_quorum_rotation_block_count: 25,
            block_spacing_ms: 300,
            testing_configs: PlatformTestConfig::default_with_no_block_signing(),
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
        let ChainExecutionOutcome {
            abci_app,
            proposers,
            quorums,
            current_quorum_hash,
            current_proposer_versions,
            end_time_ms,
            ..
        } = run_chain_for_strategy(&mut platform, 2000, strategy, config, 40);

        // With these params if we didn't rotate we would have at most 240
        // of the 500 hpmns that could get paid, however we are expecting that most
        // will be able to propose a block (and then get paid later on).

        let platform = abci_app.platform;
        let drive_cache = platform.drive.cache.read().unwrap();
        let counter = drive_cache
            .protocol_versions_counter
            .as_ref()
            .expect("expected a version counter");
        platform
            .drive
            .fetch_versions_with_counter(None)
            .expect("expected to get versions");

        assert_eq!(
            platform
                .state
                .read()
                .unwrap()
                .last_committed_block_info
                .as_ref()
                .unwrap()
                .epoch
                .index,
            0
        );
        assert!(counter.get(&1).unwrap() > &240);
    }

    #[test]
    fn run_chain_core_height_randomly_increasing_with_quorum_updates_new_proposers() {
        let strategy = Strategy {
            contracts_with_updates: vec![],
            operations: vec![],
            identities_inserts: Frequency {
                times_per_block_range: Default::default(),
                chance_per_block: None,
            },
            total_hpmns: 100,
            extra_normal_mns: 0,
            quorum_count: 24,
            upgrading_info: None,
            core_height_increase: Frequency {
                times_per_block_range: 1..2,
                chance_per_block: Some(0.2),
            },
            new_proposers: Frequency {
                times_per_block_range: 1..3,
                chance_per_block: Some(0.5),
            },
            removed_proposers: Default::default(),
            rotate_quorums: true,
        };
        let config = PlatformConfig {
            verify_sum_trees: true,
            quorum_size: 10,
            validator_set_quorum_rotation_block_count: 25,
            block_spacing_ms: 300,
            testing_configs: PlatformTestConfig::default_with_no_block_signing(),
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
        let ChainExecutionOutcome {
            abci_app,
            proposers,
            quorums,
            current_quorum_hash,
            current_proposer_versions,
            end_time_ms,
            ..
        } = run_chain_for_strategy(&mut platform, 300, strategy, config, 43);

        // With these params if we add new mns the hpmn masternode list would be 100, but we
        // can expect it to be much higher.

        let platform = abci_app.platform;
        let platform_state = platform.state.read().unwrap();

        assert!(platform_state.hpmn_masternode_list.len() > 100);
    }

    #[test]
    fn run_chain_core_height_randomly_increasing_with_quorum_updates_changing_proposers() {
        let strategy = Strategy {
            contracts_with_updates: vec![],
            operations: vec![],
            identities_inserts: Frequency {
                times_per_block_range: Default::default(),
                chance_per_block: None,
            },
            total_hpmns: 100,
            extra_normal_mns: 0,
            quorum_count: 24,
            upgrading_info: None,
            core_height_increase: Frequency {
                times_per_block_range: 1..2,
                chance_per_block: Some(0.2),
            },
            new_proposers: Frequency {
                times_per_block_range: 1..3,
                chance_per_block: Some(0.5),
            },
            removed_proposers: Frequency {
                times_per_block_range: 1..3,
                chance_per_block: Some(0.5),
            },
            rotate_quorums: true,
        };
        let config = PlatformConfig {
            verify_sum_trees: true,
            quorum_size: 10,
            validator_set_quorum_rotation_block_count: 25,
            block_spacing_ms: 300,
            testing_configs: PlatformTestConfig::default_with_no_block_signing(),
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
        let ChainExecutionOutcome {
            abci_app,
            proposers,
            quorums,
            current_quorum_hash,
            current_proposer_versions,
            end_time_ms,
            ..
        } = run_chain_for_strategy(&mut platform, 300, strategy, config, 43);

        // With these params if we add new mns the hpmn masternode list would be randomly different than 100.

        let platform = abci_app.platform;
        let platform_state = platform.state.read().unwrap();

        assert_ne!(platform_state.hpmn_masternode_list.len(), 100);
    }

    #[test]
    fn run_chain_insert_one_new_identity_per_block() {
        let strategy = Strategy {
            contracts_with_updates: vec![],
            operations: vec![],
            identities_inserts: Frequency {
                times_per_block_range: 1..2,
                chance_per_block: None,
            },
            total_hpmns: 100,
            extra_normal_mns: 0,
            quorum_count: 24,
            upgrading_info: None,
            core_height_increase: Frequency {
                times_per_block_range: Default::default(),
                chance_per_block: None,
            },
            new_proposers: Default::default(),
            removed_proposers: Default::default(),
            rotate_quorums: false,
        };
        let config = PlatformConfig {
            verify_sum_trees: true,
            quorum_size: 100,
            validator_set_quorum_rotation_block_count: 25,
            block_spacing_ms: 3000,
            testing_configs: PlatformTestConfig::default_with_no_block_signing(),
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
        let outcome = run_chain_for_strategy(&mut platform, 100, strategy, config, 15);

        assert_eq!(outcome.identities.len(), 100);
    }

    #[test]
    fn run_chain_insert_one_new_identity_per_block_with_epoch_change() {
        let strategy = Strategy {
            contracts_with_updates: vec![],
            operations: vec![],
            identities_inserts: Frequency {
                times_per_block_range: 1..2,
                chance_per_block: None,
            },
            total_hpmns: 100,
            extra_normal_mns: 0,
            quorum_count: 24,
            upgrading_info: None,
            core_height_increase: Frequency {
                times_per_block_range: Default::default(),
                chance_per_block: None,
            },
            new_proposers: Default::default(),
            removed_proposers: Default::default(),
            rotate_quorums: false,
        };
        let day_in_ms = 1000 * 60 * 60 * 24;
        let config = PlatformConfig {
            verify_sum_trees: true,
            quorum_size: 100,
            validator_set_quorum_rotation_block_count: 100,
            block_spacing_ms: day_in_ms,
            testing_configs: PlatformTestConfig::default_with_no_block_signing(),
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
        let outcome = run_chain_for_strategy(&mut platform, 150, strategy, config, 15);
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
            contracts_with_updates: vec![(contract, None)],
            operations: vec![],
            identities_inserts: Frequency {
                times_per_block_range: 1..2,
                chance_per_block: None,
            },
            total_hpmns: 100,
            extra_normal_mns: 0,
            quorum_count: 24,
            upgrading_info: None,
            core_height_increase: Frequency {
                times_per_block_range: Default::default(),
                chance_per_block: None,
            },
            new_proposers: Default::default(),
            removed_proposers: Default::default(),
            rotate_quorums: false,
        };
        let config = PlatformConfig {
            verify_sum_trees: true,
            quorum_size: 100,
            validator_set_quorum_rotation_block_count: 25,
            block_spacing_ms: 3000,
            testing_configs: PlatformTestConfig::default_with_no_block_signing(),
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
        let outcome = run_chain_for_strategy(&mut platform, 1, strategy, config, 15);

        outcome
            .abci_app
            .platform
            .drive
            .fetch_contract(
                outcome
                    .strategy
                    .contracts_with_updates
                    .first()
                    .unwrap()
                    .0
                    .id
                    .to_buffer(),
                None,
                None,
            )
            .unwrap()
            .expect("expected to execute the fetch of a contract")
            .expect("expected to get a contract");
    }

    #[test]
    fn run_chain_insert_one_new_identity_and_a_contract_with_updates() {
        let contract_cbor = json_document_to_cbor(
            "tests/supporting_files/contract/dashpay/dashpay-contract-all-mutable.json",
            Some(PROTOCOL_VERSION),
        )
        .expect("expected to get cbor from a json document");
        let contract = <Contract as DriveContractExt>::from_cbor(&contract_cbor, None)
            .expect("contract should be deserialized");

        let contract_cbor_update_1 = json_document_to_cbor(
            "tests/supporting_files/contract/dashpay/dashpay-contract-all-mutable-update-1.json",
            Some(PROTOCOL_VERSION),
        )
        .expect("expected to get cbor from a json document");
        let mut contract_update_1 =
            <Contract as DriveContractExt>::from_cbor(&contract_cbor_update_1, None)
                .expect("contract should be deserialized");

        //todo: versions should start at 0 (so this should be 1)
        contract_update_1.version = 2;

        let contract_cbor_update_2 = json_document_to_cbor(
            "tests/supporting_files/contract/dashpay/dashpay-contract-all-mutable-update-2.json",
            Some(PROTOCOL_VERSION),
        )
        .expect("expected to get cbor from a json document");
        let mut contract_update_2 =
            <Contract as DriveContractExt>::from_cbor(&contract_cbor_update_2, None)
                .expect("contract should be deserialized");

        contract_update_2.version = 3;

        let strategy = Strategy {
            contracts_with_updates: vec![(
                contract,
                Some(BTreeMap::from([
                    (3, contract_update_1),
                    (8, contract_update_2),
                ])),
            )],
            operations: vec![],
            identities_inserts: Frequency {
                times_per_block_range: 1..2,
                chance_per_block: None,
            },
            total_hpmns: 100,
            extra_normal_mns: 0,
            quorum_count: 24,
            upgrading_info: None,
            core_height_increase: Frequency {
                times_per_block_range: Default::default(),
                chance_per_block: None,
            },
            new_proposers: Default::default(),
            removed_proposers: Default::default(),
            rotate_quorums: false,
        };
        let config = PlatformConfig {
            verify_sum_trees: true,
            quorum_size: 100,
            validator_set_quorum_rotation_block_count: 25,
            block_spacing_ms: 3000,
            testing_configs: PlatformTestConfig::default_with_no_block_signing(),
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
        let outcome = run_chain_for_strategy(&mut platform, 10, strategy, config, 15);

        outcome
            .abci_app
            .platform
            .drive
            .fetch_contract(
                outcome
                    .strategy
                    .contracts_with_updates
                    .first()
                    .unwrap()
                    .0
                    .id
                    .to_buffer(),
                None,
                None,
            )
            .unwrap()
            .expect("expected to execute the fetch of a contract")
            .expect("expected to get a contract");
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
            action: DocumentAction::DocumentActionInsert,
            document_type: contract
                .document_type_for_name("contactRequest")
                .expect("expected a profile document type")
                .clone(),
        };

        let strategy = Strategy {
            contracts_with_updates: vec![(contract, None)],
            operations: vec![Operation {
                op_type: OperationType::Document(document_op),
                frequency: Frequency {
                    times_per_block_range: 1..2,
                    chance_per_block: None,
                },
            }],
            identities_inserts: Frequency {
                times_per_block_range: 1..2,
                chance_per_block: None,
            },
            total_hpmns: 100,
            extra_normal_mns: 0,
            quorum_count: 24,
            upgrading_info: None,
            core_height_increase: Frequency {
                times_per_block_range: Default::default(),
                chance_per_block: None,
            },
            new_proposers: Default::default(),
            removed_proposers: Default::default(),
            rotate_quorums: false,
        };
        let config = PlatformConfig {
            verify_sum_trees: true,
            quorum_size: 100,
            validator_set_quorum_rotation_block_count: 25,
            block_spacing_ms: 3000,
            testing_configs: PlatformTestConfig::default_with_no_block_signing(),
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
        run_chain_for_strategy(&mut platform, 100, strategy, config, 15);
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
            action: DocumentAction::DocumentActionInsert,
            document_type: contract
                .document_type_for_name("contactRequest")
                .expect("expected a profile document type")
                .clone(),
        };

        let strategy = Strategy {
            contracts_with_updates: vec![(contract, None)],
            operations: vec![Operation {
                op_type: OperationType::Document(document_op),
                frequency: Frequency {
                    times_per_block_range: 1..2,
                    chance_per_block: None,
                },
            }],
            identities_inserts: Frequency {
                times_per_block_range: 1..2,
                chance_per_block: None,
            },
            total_hpmns: 100,
            extra_normal_mns: 0,
            quorum_count: 24,
            upgrading_info: None,
            core_height_increase: Frequency {
                times_per_block_range: Default::default(),
                chance_per_block: None,
            },
            new_proposers: Default::default(),
            removed_proposers: Default::default(),
            rotate_quorums: false,
        };
        let day_in_ms = 1000 * 60 * 60 * 24;
        let config = PlatformConfig {
            verify_sum_trees: true,
            quorum_size: 100,
            validator_set_quorum_rotation_block_count: 100,
            block_spacing_ms: day_in_ms,
            testing_configs: PlatformTestConfig::default_with_no_block_signing(),
            ..Default::default()
        };
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
        let outcome = run_chain_for_strategy(&mut platform, block_count, strategy, config, 15);
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
            action: DocumentAction::DocumentActionInsert,
            document_type: contract
                .document_type_for_name("contactRequest")
                .expect("expected a profile document type")
                .clone(),
        };

        let document_deletion_op = DocumentOp {
            contract: contract.clone(),
            action: DocumentAction::DocumentActionDelete,
            document_type: contract
                .document_type_for_name("contactRequest")
                .expect("expected a profile document type")
                .clone(),
        };

        let strategy = Strategy {
            contracts_with_updates: vec![(contract, None)],
            operations: vec![
                Operation {
                    op_type: OperationType::Document(document_insertion_op),
                    frequency: Frequency {
                        times_per_block_range: 1..2,
                        chance_per_block: None,
                    },
                },
                Operation {
                    op_type: OperationType::Document(document_deletion_op),
                    frequency: Frequency {
                        times_per_block_range: 1..2,
                        chance_per_block: None,
                    },
                },
            ],
            identities_inserts: Frequency {
                times_per_block_range: 1..2,
                chance_per_block: None,
            },
            total_hpmns: 100,
            extra_normal_mns: 0,
            quorum_count: 24,
            upgrading_info: None,
            core_height_increase: Frequency {
                times_per_block_range: Default::default(),
                chance_per_block: None,
            },
            new_proposers: Default::default(),
            removed_proposers: Default::default(),
            rotate_quorums: false,
        };
        let day_in_ms = 1000 * 60 * 60 * 24;
        let config = PlatformConfig {
            verify_sum_trees: true,
            quorum_size: 100,
            validator_set_quorum_rotation_block_count: 100,
            block_spacing_ms: day_in_ms,
            testing_configs: PlatformTestConfig::default_with_no_block_signing(),
            ..Default::default()
        };
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
        let outcome = run_chain_for_strategy(&mut platform, block_count, strategy, config, 15);
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
            action: DocumentAction::DocumentActionInsert,
            document_type: contract
                .document_type_for_name("contactRequest")
                .expect("expected a profile document type")
                .clone(),
        };

        let document_deletion_op = DocumentOp {
            contract: contract.clone(),
            action: DocumentAction::DocumentActionDelete,
            document_type: contract
                .document_type_for_name("contactRequest")
                .expect("expected a profile document type")
                .clone(),
        };

        let strategy = Strategy {
            contracts_with_updates: vec![(contract, None)],
            operations: vec![
                Operation {
                    op_type: OperationType::Document(document_insertion_op),
                    frequency: Frequency {
                        times_per_block_range: 1..10,
                        chance_per_block: None,
                    },
                },
                Operation {
                    op_type: OperationType::Document(document_deletion_op),
                    frequency: Frequency {
                        times_per_block_range: 1..10,
                        chance_per_block: None,
                    },
                },
            ],
            identities_inserts: Frequency {
                times_per_block_range: 1..2,
                chance_per_block: None,
            },
            total_hpmns: 100,
            extra_normal_mns: 0,
            quorum_count: 24,
            upgrading_info: None,
            core_height_increase: Frequency {
                times_per_block_range: Default::default(),
                chance_per_block: None,
            },
            new_proposers: Default::default(),
            removed_proposers: Default::default(),
            rotate_quorums: false,
        };
        let day_in_ms = 1000 * 60 * 60 * 24;
        let config = PlatformConfig {
            verify_sum_trees: true,
            quorum_size: 100,
            validator_set_quorum_rotation_block_count: 100,
            block_spacing_ms: day_in_ms,
            testing_configs: PlatformTestConfig::default_with_no_block_signing(),
            ..Default::default()
        };

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
        let outcome = run_chain_for_strategy(&mut platform, block_count, strategy, config, 15);
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
            action: DocumentAction::DocumentActionInsert,
            document_type: contract
                .document_type_for_name("contactRequest")
                .expect("expected a profile document type")
                .clone(),
        };

        let document_deletion_op = DocumentOp {
            contract: contract.clone(),
            action: DocumentAction::DocumentActionDelete,
            document_type: contract
                .document_type_for_name("contactRequest")
                .expect("expected a profile document type")
                .clone(),
        };

        let strategy = Strategy {
            contracts_with_updates: vec![(contract, None)],
            operations: vec![
                Operation {
                    op_type: OperationType::Document(document_insertion_op),
                    frequency: Frequency {
                        times_per_block_range: 1..40,
                        chance_per_block: None,
                    },
                },
                Operation {
                    op_type: OperationType::Document(document_deletion_op),
                    frequency: Frequency {
                        times_per_block_range: 1..15,
                        chance_per_block: None,
                    },
                },
            ],
            identities_inserts: Frequency {
                times_per_block_range: 1..30,
                chance_per_block: None,
            },
            total_hpmns: 100,
            extra_normal_mns: 0,
            quorum_count: 24,
            upgrading_info: None,
            core_height_increase: Frequency {
                times_per_block_range: Default::default(),
                chance_per_block: None,
            },
            new_proposers: Default::default(),
            removed_proposers: Default::default(),
            rotate_quorums: false,
        };

        let day_in_ms = 1000 * 60 * 60 * 24;

        let config = PlatformConfig {
            verify_sum_trees: true,
            quorum_size: 100,
            validator_set_quorum_rotation_block_count: 100,
            block_spacing_ms: day_in_ms,
            testing_configs: PlatformTestConfig::default_with_no_block_signing(),
            ..Default::default()
        };
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
        let outcome = run_chain_for_strategy(&mut platform, block_count, strategy, config, 15);
        assert_eq!(outcome.identities.len() as u64, 371);
        assert_eq!(outcome.masternode_identity_balances.len(), 100);
        let balance_count = outcome
            .masternode_identity_balances
            .into_iter()
            .filter(|(_, balance)| *balance != 0)
            .count();
        assert_eq!(balance_count, 19); // 1 epoch worth of proposers
    }

    #[test]
    fn run_chain_insert_many_new_identity_per_block_many_document_insertions_updates_and_deletions_with_epoch_change(
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
            action: DocumentAction::DocumentActionInsert,
            document_type: contract
                .document_type_for_name("contactRequest")
                .expect("expected a profile document type")
                .clone(),
        };

        let document_replace_op = DocumentOp {
            contract: contract.clone(),
            action: DocumentActionReplace,
            document_type: contract
                .document_type_for_name("contactRequest")
                .expect("expected a profile document type")
                .clone(),
        };

        let document_deletion_op = DocumentOp {
            contract: contract.clone(),
            action: DocumentAction::DocumentActionDelete,
            document_type: contract
                .document_type_for_name("contactRequest")
                .expect("expected a profile document type")
                .clone(),
        };

        let strategy = Strategy {
            contracts_with_updates: vec![(contract, None)],
            operations: vec![
                Operation {
                    op_type: OperationType::Document(document_insertion_op),
                    frequency: Frequency {
                        times_per_block_range: 1..40,
                        chance_per_block: None,
                    },
                },
                Operation {
                    op_type: OperationType::Document(document_replace_op),
                    frequency: Frequency {
                        times_per_block_range: 1..5,
                        chance_per_block: None,
                    },
                },
                Operation {
                    op_type: OperationType::Document(document_deletion_op),
                    frequency: Frequency {
                        times_per_block_range: 1..5,
                        chance_per_block: None,
                    },
                },
            ],
            identities_inserts: Frequency {
                times_per_block_range: 1..6,
                chance_per_block: None,
            },
            total_hpmns: 100,
            extra_normal_mns: 0,
            quorum_count: 24,
            upgrading_info: None,
            core_height_increase: Frequency {
                times_per_block_range: Default::default(),
                chance_per_block: None,
            },
            new_proposers: Default::default(),
            removed_proposers: Default::default(),
            rotate_quorums: false,
        };

        let day_in_ms = 1000 * 60 * 60 * 24;

        let config = PlatformConfig {
            verify_sum_trees: true,
            quorum_size: 100,
            validator_set_quorum_rotation_block_count: 100,
            block_spacing_ms: day_in_ms,
            testing_configs: PlatformTestConfig::default_with_no_block_signing(),
            ..Default::default()
        };
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
        let outcome = run_chain_for_strategy(&mut platform, block_count, strategy, config, 15);
        assert_eq!(outcome.identities.len() as u64, 87);
        assert_eq!(outcome.masternode_identity_balances.len(), 100);
        let balance_count = outcome
            .masternode_identity_balances
            .into_iter()
            .filter(|(_, balance)| *balance != 0)
            .count();
        assert_eq!(balance_count, 19); // 1 epoch worth of proposers
    }

    #[test]
    fn run_chain_top_up_identities() {
        let strategy = Strategy {
            contracts_with_updates: vec![],
            operations: vec![Operation {
                op_type: OperationType::IdentityTopUp,
                frequency: Frequency {
                    times_per_block_range: 1..3,
                    chance_per_block: None,
                },
            }],
            identities_inserts: Frequency {
                times_per_block_range: 1..2,
                chance_per_block: None,
            },
            total_hpmns: 100,
            extra_normal_mns: 0,
            quorum_count: 24,
            upgrading_info: None,
            core_height_increase: Frequency {
                times_per_block_range: Default::default(),
                chance_per_block: None,
            },
            new_proposers: Default::default(),
            removed_proposers: Default::default(),
            rotate_quorums: false,
        };
        let config = PlatformConfig {
            verify_sum_trees: true,
            quorum_size: 100,
            validator_set_quorum_rotation_block_count: 25,
            block_spacing_ms: 3000,
            testing_configs: PlatformTestConfig::default_with_no_block_signing(),
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
        let outcome = run_chain_for_strategy(&mut platform, 10, strategy, config, 15);

        let max_initial_balance = 100000000000u64; // TODO: some centralized way for random test data (`arbitrary` maybe?)
        let balances = outcome
            .abci_app
            .platform
            .drive
            .fetch_identities_balances(
                &outcome
                    .identities
                    .into_iter()
                    .map(|identity| identity.id.to_buffer())
                    .collect(),
                None,
            )
            .expect("expected to fetch balances");

        assert!(balances
            .into_iter()
            .any(|(_, balance)| balance > max_initial_balance));
    }

    #[test]
    fn run_chain_update_identities_add_keys() {
        let strategy = Strategy {
            contracts_with_updates: vec![],
            operations: vec![Operation {
                op_type: OperationType::IdentityUpdate(IdentityUpdateOp::IdentityUpdateAddKeys(3)),
                frequency: Frequency {
                    times_per_block_range: 1..2,
                    chance_per_block: None,
                },
            }],
            identities_inserts: Frequency {
                times_per_block_range: 1..2,
                chance_per_block: None,
            },
            total_hpmns: 100,
            extra_normal_mns: 0,
            quorum_count: 24,
            upgrading_info: None,
            core_height_increase: Frequency {
                times_per_block_range: Default::default(),
                chance_per_block: None,
            },
            new_proposers: Default::default(),
            removed_proposers: Default::default(),
            rotate_quorums: false,
        };
        let config = PlatformConfig {
            verify_sum_trees: true,
            quorum_size: 100,
            validator_set_quorum_rotation_block_count: 25,
            block_spacing_ms: 3000,
            testing_configs: PlatformTestConfig::default_with_no_block_signing(),
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
        let outcome = run_chain_for_strategy(&mut platform, 10, strategy, config, 15);

        let identities = outcome
            .abci_app
            .platform
            .drive
            .fetch_full_identities(
                outcome
                    .identities
                    .into_iter()
                    .map(|identity| identity.id.to_buffer())
                    .collect(),
                None,
            )
            .expect("expected to fetch balances");

        assert!(identities
            .into_iter()
            .any(|(_, identity)| { identity.expect("expected identity").public_keys.len() > 7 }));
    }

    #[test]
    fn run_chain_update_identities_remove_keys() {
        let strategy = Strategy {
            contracts_with_updates: vec![],
            operations: vec![Operation {
                op_type: OperationType::IdentityUpdate(IdentityUpdateOp::IdentityUpdateDisableKey(
                    3,
                )),
                frequency: Frequency {
                    times_per_block_range: 1..2,
                    chance_per_block: None,
                },
            }],
            identities_inserts: Frequency {
                times_per_block_range: 1..2,
                chance_per_block: None,
            },
            total_hpmns: 100,
            extra_normal_mns: 0,
            quorum_count: 24,
            upgrading_info: None,
            core_height_increase: Frequency {
                times_per_block_range: Default::default(),
                chance_per_block: None,
            },
            new_proposers: Default::default(),
            removed_proposers: Default::default(),
            rotate_quorums: false,
        };
        let config = PlatformConfig {
            verify_sum_trees: true,
            quorum_size: 100,
            validator_set_quorum_rotation_block_count: 25,
            block_spacing_ms: 3000,
            testing_configs: PlatformTestConfig::default_with_no_block_signing(),
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
        let outcome = run_chain_for_strategy(&mut platform, 10, strategy, config, 15);

        let identities = outcome
            .abci_app
            .platform
            .drive
            .fetch_full_identities(
                outcome
                    .identities
                    .into_iter()
                    .map(|identity| identity.id.to_buffer())
                    .collect(),
                None,
            )
            .expect("expected to fetch balances");

        assert!(identities.into_iter().any(|(_, identity)| {
            identity
                .expect("expected identity")
                .public_keys
                .into_iter()
                .any(|(_, public_key)| public_key.is_disabled())
        }));
    }

    #[test]
    fn run_chain_top_up_and_withdraw_from_identities() {
        let strategy = Strategy {
            contracts_with_updates: vec![],
            operations: vec![
                Operation {
                    op_type: OperationType::IdentityTopUp,
                    frequency: Frequency {
                        times_per_block_range: 1..4,
                        chance_per_block: None,
                    },
                },
                Operation {
                    op_type: OperationType::IdentityWithdrawal,
                    frequency: Frequency {
                        times_per_block_range: 1..4,
                        chance_per_block: None,
                    },
                },
            ],
            identities_inserts: Frequency {
                times_per_block_range: 1..2,
                chance_per_block: None,
            },
            total_hpmns: 100,
            extra_normal_mns: 0,
            quorum_count: 24,
            upgrading_info: None,
            core_height_increase: Frequency {
                times_per_block_range: Default::default(),
                chance_per_block: None,
            },
            new_proposers: Default::default(),
            removed_proposers: Default::default(),
            rotate_quorums: false,
        };
        let config = PlatformConfig {
            verify_sum_trees: true,
            quorum_size: 100,
            validator_set_quorum_rotation_block_count: 25,
            block_spacing_ms: 3000,
            testing_configs: PlatformTestConfig::default_with_no_block_signing(),
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
        let outcome = run_chain_for_strategy(&mut platform, 10, strategy, config, 15);

        assert_eq!(outcome.identities.len(), 10);
        assert_eq!(outcome.withdrawals.len(), 18);
    }
}
