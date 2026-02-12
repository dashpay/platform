use crate::masternodes::MasternodeListItemWithUpdates;
use crate::query::QueryStrategy;
use dpp::block::block_info::BlockInfo;
use dpp::dashcore::{Network, PrivateKey};
use dpp::dashcore::{ProTxHash, QuorumHash};
use dpp::shielded::{compute_platform_sighash, SerializedAction};
use dpp::state_transition::identity_topup_transition::methods::IdentityTopUpTransitionMethodsV0;
use dpp::state_transition::shield_from_asset_lock_transition::methods::ShieldFromAssetLockTransitionMethodsV0;
use dpp::state_transition::shield_from_asset_lock_transition::ShieldFromAssetLockTransition;
use dpp::state_transition::shield_transition::methods::ShieldTransitionMethodsV0;
use dpp::state_transition::shield_transition::ShieldTransition;
use dpp::state_transition::shielded_transfer_transition::methods::ShieldedTransferTransitionMethodsV0;
use dpp::state_transition::shielded_transfer_transition::ShieldedTransferTransition;
use dpp::state_transition::shielded_withdrawal_transition::methods::ShieldedWithdrawalTransitionMethodsV0;
use dpp::state_transition::shielded_withdrawal_transition::ShieldedWithdrawalTransition;
use dpp::state_transition::unshield_transition::methods::UnshieldTransitionMethodsV0;
use dpp::state_transition::unshield_transition::UnshieldTransition;
use dpp::ProtocolError;
use grovedb_commitment_tree::{
    new_memory_store, Anchor, Authorized as OrchardAuthorized, Builder, Bundle, BundleType,
    CommitmentTree, ExtractedNoteCommitment, Flags as OrchardFlags, FullViewingKey,
    MemoryCommitmentStore, MerklePath, Note, NoteValue, Position, ProvingKey, Retention, Scope,
    SpendAuthorizingKey, SpendingKey,
};
use orchard::note::RandomSeed;

use dpp::dashcore::secp256k1::SecretKey;
use dpp::data_contract::document_type::random_document::CreateRandomDocument;
use dpp::data_contract::{DataContract, DataContractFactory};
use dpp::state_transition::identity_topup_transition::IdentityTopUpTransition;
use strategy_tests::frequency::Frequency;
use strategy_tests::operations::FinalizeBlockOperation::IdentityAddKeys;
use strategy_tests::operations::{
    AmountRange, DocumentAction, DocumentOp, ExtraKeys, FinalizeBlockOperation, IdentityUpdateOp,
    MaybeOutputAmount, OperationType, OutputCountRange, TokenOp,
    UseExistingAddressesAsOutputChance,
};
use strategy_tests::KeyMaps;

use dpp::address_funds::fee_strategy::AddressFundsFeeStrategyStep;
use dpp::address_funds::{AddressFundsFeeStrategy, PlatformAddress};
use dpp::document::DocumentV0Getters;
use dpp::fee::Credits;
use dpp::identity::{Identity, IdentityPublicKey, KeyID, KeyType, Purpose, SecurityLevel};
use dpp::serialization::PlatformSerializableWithPlatformVersion;
use dpp::state_transition::data_contract_create_transition::DataContractCreateTransition;
use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use dpp::state_transition::StateTransition;
use dpp::util::deserializer::ProtocolVersion;
use dpp::version::PlatformVersion;
use drive::drive::identity::key::fetch::{IdentityKeysRequest, KeyRequestType};
use drive::drive::Drive;
use drive::util::storage_flags::StorageFlags::SingleEpoch;

use crate::addresses_with_balance::AddressesWithBalance;
use crate::strategy::CoreHeightIncrease::NoCoreHeightIncrease;
use dpp::dashcore::hashes::Hash;
use dpp::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::v0::DocumentTypeV0;
use dpp::identifier::MasternodeIdentifiers;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::core_script::CoreScript;
use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
use dpp::identity::signer::Signer;
use dpp::identity::state_transition::asset_lock_proof::InstantAssetLockProof;
use dpp::identity::KeyCount;
use dpp::identity::KeyType::ECDSA_SECP256K1;
use dpp::platform_value::{BinaryData, Value};
use dpp::prelude::{AssetLockProof, BlockHeight, Identifier, IdentityNonce};
use dpp::state_transition::address_credit_withdrawal_transition::methods::AddressCreditWithdrawalTransitionMethodsV0;
use dpp::state_transition::address_credit_withdrawal_transition::AddressCreditWithdrawalTransition;
use dpp::state_transition::address_funding_from_asset_lock_transition::methods::AddressFundingFromAssetLockTransitionMethodsV0;
use dpp::state_transition::address_funding_from_asset_lock_transition::v0::AddressFundingFromAssetLockTransitionV0;
use dpp::state_transition::address_funds_transfer_transition::methods::AddressFundsTransferTransitionMethodsV0;
use dpp::state_transition::address_funds_transfer_transition::AddressFundsTransferTransition;
use dpp::state_transition::batch_transition::batched_transition::document_delete_transition::DocumentDeleteTransitionV0;
use dpp::state_transition::batch_transition::batched_transition::document_replace_transition::DocumentReplaceTransitionV0;
use dpp::state_transition::batch_transition::batched_transition::document_transfer_transition::DocumentTransferTransitionV0;
use dpp::state_transition::batch_transition::batched_transition::{
    BatchedTransition, DocumentDeleteTransition, DocumentReplaceTransition,
    DocumentTransferTransition,
};
use dpp::state_transition::batch_transition::document_base_transition::v0::DocumentBaseTransitionV0;
use dpp::state_transition::batch_transition::document_create_transition::{
    DocumentCreateTransition, DocumentCreateTransitionV0,
};
use dpp::state_transition::batch_transition::token_base_transition::v0::TokenBaseTransitionV0;
use dpp::state_transition::batch_transition::token_mint_transition::TokenMintTransitionV0;
use dpp::state_transition::batch_transition::token_transfer_transition::TokenTransferTransitionV0;
use dpp::state_transition::batch_transition::{
    BatchTransition, BatchTransitionV0, BatchTransitionV1, TokenMintTransition,
    TokenTransferTransition,
};
use dpp::state_transition::data_contract_create_transition::methods::v0::DataContractCreateTransitionMethodsV0;
use dpp::state_transition::data_contract_update_transition::methods::DataContractUpdateTransitionMethodsV0;
use dpp::state_transition::identity_create_from_addresses_transition::methods::IdentityCreateFromAddressesTransitionMethodsV0;
use dpp::state_transition::identity_create_from_addresses_transition::v0::IdentityCreateFromAddressesTransitionV0;
use dpp::state_transition::identity_topup_from_addresses_transition::methods::IdentityTopUpFromAddressesTransitionMethodsV0;
use dpp::state_transition::identity_topup_from_addresses_transition::v0::IdentityTopUpFromAddressesTransitionV0;
use dpp::state_transition::masternode_vote_transition::methods::MasternodeVoteTransitionMethodsV0;
use dpp::state_transition::masternode_vote_transition::MasternodeVoteTransition;
use dpp::tokens::calculate_token_id;
use dpp::tokens::token_event::TokenEvent;
use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use dpp::voting::vote_polls::VotePoll;
use dpp::voting::votes::resource_vote::v0::ResourceVoteV0;
use dpp::voting::votes::resource_vote::ResourceVote;
use dpp::voting::votes::Vote;
use dpp::withdrawal::Pooling;
use drive::drive::document::query::QueryDocumentsOutcomeV0Methods;
use drive::query::DriveDocumentQuery;
use drive_abci::abci::app::FullAbciApplication;
use drive_abci::config::PlatformConfig;
use drive_abci::mimic::test_quorum::TestQuorumInfo;
use drive_abci::platform_types::platform::Platform;
use drive_abci::platform_types::platform_state::PlatformStateV0Methods;
use drive_abci::platform_types::signature_verification_quorum_set::{
    QuorumConfig, Quorums, SigningQuorum,
};
use drive_abci::platform_types::withdrawal::unsigned_withdrawal_txs::v0::UnsignedWithdrawalTxs;
use drive_abci::rpc::core::MockCoreRPCLike;
use rand::prelude::{IteratorRandom, SliceRandom, StdRng};
use rand::Rng;
use simple_signer::signer::SimpleSigner;
use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::ops::RangeInclusive;
use std::str::FromStr;
use std::sync::OnceLock;
use strategy_tests::transitions::{
    create_identity_credit_transfer_to_addresses_transition,
    create_identity_credit_transfer_to_addresses_transition_with_outputs,
    create_identity_credit_transfer_transition, create_state_transitions_for_identities,
    create_state_transitions_for_identities_and_proofs,
    instant_asset_lock_proof_fixture_with_dynamic_range,
};
use strategy_tests::Strategy;
use tenderdash_abci::proto::abci::{ExecTxResult, ValidatorSetUpdate};

/// Cached Orchard proving key for strategy tests (~30s to build, reused across tests).
static TEST_PROVING_KEY: OnceLock<ProvingKey> = OnceLock::new();

fn get_proving_key() -> &'static ProvingKey {
    TEST_PROVING_KEY.get_or_init(ProvingKey::build)
}

/// Deterministic Orchard spending key seed used throughout all shielded strategy tests.
const TEST_SK_BYTES: [u8; 32] = [0u8; 32];

/// Tracks shielded pool state locally for strategy tests.
///
/// After each block, successful Shield/ShieldFromAssetLock transitions append their
/// output note commitments to this tree. Spend-based transitions (ShieldedTransfer,
/// Unshield, ShieldedWithdrawal) then pick notes from here to build spend bundles
/// with valid Merkle witnesses.
pub struct ShieldedState {
    /// Local commitment tree mirroring the on-chain tree.
    pub tree: CommitmentTree<MemoryCommitmentStore>,
    /// Spendable notes: (Note, Position in commitment tree).
    /// Notes are removed once spent.
    pub spendable_notes: Vec<(Note, Position)>,
    /// Monotonically increasing checkpoint ID.
    pub checkpoint_counter: u32,
    /// Cached spending key derived from TEST_SK_BYTES.
    #[allow(dead_code)]
    pub sk: SpendingKey,
    /// Cached full viewing key derived from sk.
    pub fvk: FullViewingKey,
    /// Cached spend authorizing key for signing spend bundles.
    pub ask: SpendAuthorizingKey,
    /// Counter for generating unique rho values for notes.
    pub rho_counter: u64,
}

impl ShieldedState {
    pub fn new() -> Self {
        let sk = SpendingKey::from_bytes(TEST_SK_BYTES).unwrap();
        let fvk = FullViewingKey::from(&sk);
        let ask = SpendAuthorizingKey::from(&sk);
        Self {
            tree: CommitmentTree::new(new_memory_store(), 1000),
            spendable_notes: Vec::new(),
            checkpoint_counter: 0,
            sk,
            fvk,
            ask,
            rho_counter: 1, // Start at 1 to avoid zero rho
        }
    }

    /// Record a note that was output by a successful shield transition.
    ///
    /// `value` is the shielded amount in credits.
    /// The note is reconstructed deterministically using the test spending key
    /// and a unique rho derived from `rho_counter`.
    pub fn record_shielded_note(&mut self, value: u64) {
        let recipient = self.fvk.address_at(0u32, Scope::External);

        // Create a deterministic rho from the counter
        let mut rho_bytes = [0u8; 32];
        rho_bytes[..8].copy_from_slice(&self.rho_counter.to_le_bytes());
        self.rho_counter += 1;

        let rho = orchard::note::Rho::from_bytes(&rho_bytes).unwrap();
        let rseed = RandomSeed::from_bytes([42u8; 32], &rho).unwrap();
        let note = Note::from_parts(recipient, NoteValue::from_raw(value), rho, rseed).unwrap();

        // Append to commitment tree
        let cmx = ExtractedNoteCommitment::from(note.commitment());
        self.tree.append(cmx, Retention::Marked).unwrap();

        let position = self.tree.max_leaf_position().unwrap().unwrap();
        self.spendable_notes.push((note, position));

        tracing::debug!(
            value,
            position = u64::from(position),
            "Recorded spendable shielded note"
        );
    }

    /// Create a checkpoint after processing a block.
    pub fn checkpoint(&mut self) {
        self.tree.checkpoint(self.checkpoint_counter).unwrap();
        self.checkpoint_counter += 1;
    }

    /// Take a spendable note (removes it from the pool).
    /// Returns (Note, MerklePath, Anchor) if a note is available.
    pub fn take_spendable_note(&mut self) -> Option<(Note, MerklePath, Anchor)> {
        if self.spendable_notes.is_empty() {
            return None;
        }
        let (note, position) = self.spendable_notes.remove(0);
        let merkle_path = self.tree.orchard_witness(position).ok()??;
        let anchor = self.tree.anchor().ok()?;
        Some((note, merkle_path, anchor))
    }

    /// Check if any spendable notes exist.
    pub fn has_spendable_notes(&self) -> bool {
        !self.spendable_notes.is_empty()
    }
}

/// Decompose an authorized Orchard bundle into platform serialization fields.
fn serialize_authorized_bundle(
    bundle: &Bundle<OrchardAuthorized, i64>,
) -> (Vec<SerializedAction>, u8, i64, [u8; 32], Vec<u8>, [u8; 64]) {
    let actions: Vec<SerializedAction> = bundle
        .actions()
        .iter()
        .map(|action| {
            let enc = action.encrypted_note();
            let mut encrypted_note = Vec::with_capacity(692);
            encrypted_note.extend_from_slice(&enc.epk_bytes);
            encrypted_note.extend_from_slice(&enc.enc_ciphertext);
            encrypted_note.extend_from_slice(&enc.out_ciphertext);
            SerializedAction {
                nullifier: action.nullifier().to_bytes(),
                rk: <[u8; 32]>::from(action.rk()),
                cmx: action.cmx().to_bytes(),
                encrypted_note,
                cv_net: action.cv_net().to_bytes(),
                spend_auth_sig: <[u8; 64]>::from(action.authorization()),
            }
        })
        .collect();
    let flags = bundle.flags().to_byte();
    let value_balance = *bundle.value_balance();
    let anchor = bundle.anchor().to_bytes();
    let proof = bundle.authorization().proof().as_ref().to_vec();
    let binding_sig = <[u8; 64]>::from(bundle.authorization().binding_signature());
    (actions, flags, value_balance, anchor, proof, binding_sig)
}

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
    pub max_addresses_to_choose_from_in_cache: Option<u32>,
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
            max_addresses_to_choose_from_in_cache: Some(50),
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
        let operations_to_execute = self.strategy.operations.clone();
        for op in operations_to_execute.iter() {
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
                    self.strategy.start_identities.keys_per_identity as KeyID,
                    &self.strategy.start_identities.extra_keys,
                    &(self.strategy.start_identities.starting_balances
                        ..=self.strategy.start_identities.starting_balances),
                    signer,
                    rng,
                    instant_lock_quorums,
                    platform_config,
                    platform_version,
                );
                state_transitions.append(&mut new_transitions);
            }
            // Extend the state transitions with the strategy's hard coded start identities
            // Filtering out the ones that have no create transition
            if !self.strategy.start_identities.hard_coded.is_empty() {
                state_transitions.extend(
                    self.strategy.start_identities.hard_coded.iter().filter_map(
                        |(identity, transition)| {
                            transition.as_ref().map(|create_transition| {
                                (identity.clone(), create_transition.clone())
                            })
                        },
                    ),
                );
            }
        }
        let frequency = &self.strategy.identity_inserts.frequency;
        if frequency.check_hit(rng) {
            let count = frequency.events(rng);
            let mut new_transitions = self.create_identities_state_transitions(
                count,
                self.strategy.identity_inserts.start_keys as KeyID,
                &self.strategy.identity_inserts.extra_keys,
                &self.strategy.identity_inserts.start_balance_range,
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
                    } else if let OperationType::Token(token_op) = &mut operation.op_type {
                        if token_op.contract.id() == old_id {
                            token_op.contract.set_id(contract.id());
                            token_op.token_id = calculate_token_id(
                                contract.id_ref().as_bytes(),
                                token_op.token_pos,
                            )
                            .into();
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
        &mut self,
        platform: &Platform<MockCoreRPCLike>,
        block_info: &BlockInfo,
        current_identities: &mut Vec<Identity>,
        current_addresses_with_balance: &mut AddressesWithBalance,
        signer: &mut SimpleSigner,
        identity_nonce_counter: &mut BTreeMap<Identifier, u64>,
        contract_nonce_counter: &mut BTreeMap<(Identifier, Identifier), u64>,
        // first identifier is the vote poll id
        // second identifier is the identifier
        current_votes: &mut BTreeMap<Identifier, BTreeMap<Identifier, ResourceVoteChoice>>,
        instant_lock_quorums: &Quorums<SigningQuorum>,
        rng: &mut StdRng,
        platform_version: &PlatformVersion,
        shielded_state: &mut Option<ShieldedState>,
    ) -> (Vec<StateTransition>, Vec<FinalizeBlockOperation>) {
        let mut maybe_state = None;
        let mut operations = vec![];
        let mut finalize_block_operations = vec![];
        let mut replaced = vec![];
        let mut transferred = vec![];
        let mut deleted = vec![];
        let max_document_operation_count_without_inserts =
            self.strategy.max_document_operation_count_without_inserts();
        let operations_to_execute = self.strategy.operations.clone();
        for op in operations_to_execute.iter() {
            if op.frequency.check_hit(rng) {
                let mut count = rng.gen_range(op.frequency.times_per_block_range.clone());
                match &op.op_type {
                    OperationType::Document(DocumentOp {
                        action: DocumentAction::DocumentActionInsertRandom(fill_type, fill_size),
                        document_type,
                        contract,
                    }) => {
                        if current_identities.len() < count as usize {
                            count = current_identities.len() as u16;

                            tracing::warn!(
                                "Not enough identities to insert documents, reducing count to {}",
                                count
                            );
                        }

                        let current_identities_as_refs: Vec<&dpp::identity::Identity> =
                            current_identities.iter().collect();

                        let documents = document_type
                            .random_documents_with_params(
                                count as u32,
                                current_identities_as_refs.as_ref(),
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

                                let document_batch_transition: BatchTransition =
                                    BatchTransitionV0 {
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
                                        false,
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
                            let held_identity = current_identities
                                .iter()
                                .find(|identity| identity.id() == identifier)
                                .expect("expected to find identifier, review strategy params");

                            let mut eligible_identities = Vec::with_capacity(count as usize);
                            for _ in 0..count {
                                eligible_identities.push(held_identity);
                            }

                            document_type
                                .random_documents_with_params(
                                    count as u32,
                                    &eligible_identities,
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
                            let current_identities_as_refs: Vec<&dpp::identity::Identity> =
                                current_identities.iter().collect();

                            document_type
                                .random_documents_with_params(
                                    count as u32,
                                    current_identities_as_refs.as_ref(),
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

                                let document_batch_transition: BatchTransition =
                                    BatchTransitionV0 {
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
                                        false,
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
                        let any_item_query = DriveDocumentQuery::all_items_query(
                            contract,
                            document_type.as_ref(),
                            Some(max_document_operation_count_without_inserts),
                        );
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

                            let document_batch_transition: BatchTransition = BatchTransitionV0 {
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

                            let document_batch_transition: BatchTransition = BatchTransitionV0 {
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

                            let document_batch_transition: BatchTransition = BatchTransitionV0 {
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
                    OperationType::IdentityTopUp(amount) if !current_identities.is_empty() => {
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
                                amount,
                                instant_lock_quorums,
                                &platform.config,
                                platform_version,
                            ));
                        }
                    }
                    OperationType::IdentityTopUpFromAddresses(amount_range)
                        if !current_identities.is_empty() =>
                    {
                        let indices: Vec<usize> =
                            (0..current_identities.len()).choose_multiple(rng, count as usize);
                        let random_identities: Vec<&Identity> = indices
                            .into_iter()
                            .map(|index| &current_identities[index])
                            .collect();

                        for random_identity in random_identities {
                            let Some(state_transition) = self
                                .create_identity_top_up_from_addresses_transitions(
                                    current_addresses_with_balance,
                                    random_identity,
                                    amount_range,
                                    signer,
                                    rng,
                                    platform_version,
                                )
                            else {
                                // no funds left
                                break;
                            };
                            operations.push(state_transition);
                        }
                    }
                    OperationType::IdentityCreateFromAddresses(
                        amount_range,
                        maybe_output_amount,
                        fee_strategy,
                        key_count,
                        extra_keys,
                    ) => {
                        for _i in 0..count {
                            let Some((identity, state_transition)) = self
                                .create_identity_from_addresses_transition(
                                    current_addresses_with_balance,
                                    amount_range,
                                    maybe_output_amount,
                                    fee_strategy,
                                    *key_count,
                                    extra_keys,
                                    signer,
                                    rng,
                                    platform_version,
                                )
                            else {
                                // no funds left
                                break;
                            };
                            operations.push(state_transition);
                            // Add the newly created identity to the pool
                            current_identities.push(identity);
                        }
                    }
                    OperationType::AddressFundingFromCoreAssetLock(amount_range) => {
                        for _i in 0..count {
                            let Some(state_transition) = self
                                .create_address_funding_from_asset_lock_transitions(
                                    current_addresses_with_balance,
                                    amount_range,
                                    rng,
                                    signer,
                                    instant_lock_quorums,
                                    &platform.config,
                                    platform_version,
                                )
                            else {
                                // no funds left
                                break;
                            };
                            operations.push(state_transition);
                        }
                    }
                    OperationType::AddressTransfer(
                        amount_range,
                        output_count_range,
                        use_existing_outputs_chance,
                        fee_strategy,
                    ) => {
                        for _i in 0..count {
                            let Some(state_transition) = self.create_address_transfer_transition(
                                current_addresses_with_balance,
                                amount_range,
                                output_count_range,
                                *use_existing_outputs_chance,
                                fee_strategy,
                                signer,
                                rng,
                                platform_version,
                            ) else {
                                tracing::debug!(
                                    block_height = block_info.height,
                                    ?amount_range,
                                    available_to_spend = current_addresses_with_balance
                                        .available_for_spending_count(),
                                    max_available_balance =
                                        current_addresses_with_balance.max_available_balance(),
                                    committed = current_addresses_with_balance.committed_count(),
                                    staged = current_addresses_with_balance.staged_count(),
                                    "no funds for transfer"
                                );
                                // no funds left
                                break;
                            };
                            operations.push(state_transition);
                        }
                    }
                    OperationType::AddressWithdrawal(
                        amount_range,
                        maybe_output_range,
                        fee_strategy,
                    ) => {
                        for _i in 0..count {
                            let Some(state_transition) = self.create_address_withdrawal_transition(
                                current_addresses_with_balance,
                                amount_range,
                                maybe_output_range,
                                fee_strategy,
                                signer,
                                rng,
                                platform_version,
                            ) else {
                                // no funds left
                                break;
                            };
                            operations.push(state_transition);
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
                    OperationType::IdentityWithdrawal(amount) if !current_identities.is_empty() => {
                        let indices: Vec<usize> =
                            (0..current_identities.len()).choose_multiple(rng, count as usize);
                        for index in indices {
                            let random_identity = current_identities.get_mut(index).unwrap();
                            let state_transition =
                                strategy_tests::transitions::create_identity_withdrawal_transition(
                                    random_identity,
                                    amount.clone(),
                                    identity_nonce_counter,
                                    signer,
                                    rng,
                                );
                            operations.push(state_transition);
                        }
                    }
                    OperationType::IdentityTransfer(identity_transfer_info)
                        if current_identities.len() > 1 =>
                    {
                        for _ in 0..count {
                            // Handle the case where specific sender, recipient, and amount are provided
                            if let Some(transfer_info) = identity_transfer_info {
                                let sender = current_identities
                                    .iter()
                                    .find(|identity| identity.id() == transfer_info.from)
                                    .expect(
                                        "Expected to find sender identity in hardcoded start identities",
                                    );
                                let recipient = current_identities
                                    .iter()
                                    .find(|identity| identity.id() == transfer_info.to)
                                    .expect(
                                        "Expected to find recipient identity in hardcoded start identities",
                                    );

                                let state_transition = create_identity_credit_transfer_transition(
                                    sender,
                                    recipient,
                                    identity_nonce_counter,
                                    signer, // This means in the TUI, the loaded identity must always be the sender since we're always signing with it for now
                                    transfer_info.amount,
                                );
                                operations.push(state_transition);
                            } else if current_identities.len() > 1 {
                                // Handle the case where no sender, recipient, and amount are provided

                                let identities_count = current_identities.len();
                                if identities_count == 0 {
                                    break;
                                }

                                // Select a random identity from the current_identities for the sender
                                let random_index_sender = rng.gen_range(0..identities_count);

                                // Clone current_identities to a Vec for manipulation
                                let mut unused_identities: Vec<_> = current_identities.to_vec();
                                unused_identities.remove(random_index_sender); // Remove the sender
                                let unused_identities_count = unused_identities.len();

                                // Select a random identity from the remaining ones for the recipient
                                let random_index_recipient =
                                    rng.gen_range(0..unused_identities_count);
                                let recipient = &unused_identities[random_index_recipient];

                                // Use the sender index on the original slice
                                let sender = &mut current_identities[random_index_sender];

                                let state_transition = create_identity_credit_transfer_transition(
                                    sender,
                                    recipient,
                                    identity_nonce_counter,
                                    signer,
                                    300000,
                                );
                                operations.push(state_transition);
                            }
                        }
                    }
                    OperationType::IdentityTransferToAddresses(
                        amount_range,
                        output_count_range,
                        _use_existing,
                        identity_transfer_info,
                    ) if !current_identities.is_empty() => {
                        for _ in 0..count {
                            // Handle the case where specific sender and outputs are provided
                            if let Some(transfer_info) = identity_transfer_info {
                                let sender = current_identities
                                    .iter()
                                    .find(|identity| identity.id() == transfer_info.from)
                                    .expect(
                                        "Expected to find sender identity in hardcoded start identities",
                                    );

                                // Use the pre-specified outputs from transfer_info
                                let state_transition = create_identity_credit_transfer_to_addresses_transition_with_outputs(
                                    sender,
                                    identity_nonce_counter,
                                    signer,
                                    transfer_info.outputs.clone(),
                                    platform_version,
                                );
                                operations.push(state_transition);
                            } else {
                                // Handle the case where no sender/outputs are provided - generate random ones
                                let identities_count = current_identities.len();
                                if identities_count == 0 {
                                    break;
                                }

                                // Select a random identity from the current_identities for the sender
                                let random_index_sender = rng.gen_range(0..identities_count);
                                let sender = &current_identities[random_index_sender];

                                // Generate random number of outputs from the provided range
                                let output_count =
                                    rng.gen_range(output_count_range.clone()) as usize;
                                let total_amount = rng.gen_range(amount_range.clone());

                                let (state_transition, _recipient_addresses) =
                                    create_identity_credit_transfer_to_addresses_transition(
                                        sender,
                                        identity_nonce_counter,
                                        current_addresses_with_balance,
                                        signer,
                                        total_amount,
                                        output_count,
                                        rng,
                                        platform_version,
                                    );
                                operations.push(state_transition);
                            }
                        }
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
                                    false,
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
                    OperationType::Token(TokenOp {
                        contract,
                        token_id,
                        token_pos,
                        use_identity_with_id,
                        action: TokenEvent::Mint(amount, recipient, note),
                    }) if current_identities.len() > 1 => {
                        let operation_owner_id = if let Some(identity_id) = use_identity_with_id {
                            *identity_id
                        } else {
                            let random_index = rng.gen_range(0..current_identities.len());
                            current_identities[random_index].id()
                        };

                        let request = IdentityKeysRequest {
                            identity_id: operation_owner_id.to_buffer(),
                            request_type: KeyRequestType::SpecificKeys(vec![1]),
                            limit: Some(1),
                            offset: None,
                        };
                        let identity = platform
                            .drive
                            .fetch_identity_balance_with_keys(request, None, platform_version)
                            .expect("expected to be able to get identity")
                            .expect("expected to get an identity for token mint operation");
                        let identity_contract_nonce = contract_nonce_counter
                            .entry((operation_owner_id, contract.id()))
                            .or_default();
                        *identity_contract_nonce += 1;
                        let token_mint_transition: TokenMintTransition = TokenMintTransitionV0 {
                            base: TokenBaseTransitionV0 {
                                identity_contract_nonce: *identity_contract_nonce,
                                token_contract_position: *token_pos,
                                data_contract_id: contract.id(),
                                token_id: *token_id,
                                using_group_info: None,
                            }
                            .into(),
                            issued_to_identity_id: Some(*recipient),
                            amount: *amount,
                            public_note: note.clone(),
                        }
                        .into();

                        let batch_transition: BatchTransition = BatchTransitionV1 {
                            owner_id: identity.id,
                            transitions: vec![BatchedTransition::Token(
                                token_mint_transition.into(),
                            )],
                            user_fee_increase: 0,
                            signature_public_key_id: 0,
                            signature: BinaryData::default(),
                        }
                        .into();

                        let mut batch_transition: StateTransition = batch_transition.into();

                        let identity_public_key = identity
                            .loaded_public_keys
                            .values()
                            .next()
                            .expect("expected a key");

                        batch_transition
                            .sign_external(
                                identity_public_key,
                                signer,
                                Some(|_data_contract_id, _document_type_name| {
                                    Ok(SecurityLevel::HIGH)
                                }),
                            )
                            .expect("expected to sign");

                        operations.push(batch_transition);
                    }
                    OperationType::Token(TokenOp {
                        contract,
                        token_id,
                        token_pos,
                        use_identity_with_id,
                        action:
                            TokenEvent::Transfer(
                                recipient,
                                public_note,
                                shared_encrypted_note,
                                private_encrypted_note,
                                amount,
                            ),
                    }) if current_identities.len() > 1 => {
                        let operation_owner_id = if let Some(identity_id) = use_identity_with_id {
                            *identity_id
                        } else {
                            let random_index = rng.gen_range(0..current_identities.len());
                            current_identities[random_index].id()
                        };

                        let request = IdentityKeysRequest {
                            identity_id: operation_owner_id.to_buffer(),
                            request_type: KeyRequestType::SpecificKeys(vec![1]),
                            limit: Some(1),
                            offset: None,
                        };
                        let identity = platform
                            .drive
                            .fetch_identity_balance_with_keys(request, None, platform_version)
                            .expect("expected to be able to get identity")
                            .expect("expected to get an identity for token mint operation");
                        let identity_contract_nonce = contract_nonce_counter
                            .entry((operation_owner_id, contract.id()))
                            .or_default();
                        *identity_contract_nonce += 1;
                        let token_transfer_transition: TokenTransferTransition =
                            TokenTransferTransitionV0 {
                                base: TokenBaseTransitionV0 {
                                    identity_contract_nonce: *identity_contract_nonce,
                                    token_contract_position: *token_pos,
                                    data_contract_id: contract.id(),
                                    token_id: *token_id,
                                    using_group_info: None,
                                }
                                .into(),
                                amount: *amount,
                                recipient_id: *recipient,
                                public_note: public_note.clone(),
                                shared_encrypted_note: shared_encrypted_note.clone(),
                                private_encrypted_note: private_encrypted_note.clone(),
                            }
                            .into();

                        let batch_transition: BatchTransition = BatchTransitionV1 {
                            owner_id: identity.id,
                            transitions: vec![BatchedTransition::Token(
                                token_transfer_transition.into(),
                            )],
                            user_fee_increase: 0,
                            signature_public_key_id: 0,
                            signature: BinaryData::default(),
                        }
                        .into();

                        let mut batch_transition: StateTransition = batch_transition.into();

                        let identity_public_key = identity
                            .loaded_public_keys
                            .values()
                            .next()
                            .expect("expected a key");

                        batch_transition
                            .sign_external(
                                identity_public_key,
                                signer,
                                Some(|_data_contract_id, _document_type_name| {
                                    Ok(SecurityLevel::HIGH)
                                }),
                            )
                            .expect("expected to sign");

                        operations.push(batch_transition);
                    }
                    OperationType::Shield(amount_range) => {
                        for _i in 0..count {
                            let Some(state_transition) = self.create_shield_transition(
                                current_addresses_with_balance,
                                amount_range,
                                signer,
                                rng,
                                platform_version,
                            ) else {
                                break;
                            };
                            // Record the shielded note for potential future spends.
                            // The value is |-value_balance| since value_balance is negative
                            // for shield transitions (money flowing into the pool).
                            if let StateTransition::Shield(ref shield) = state_transition {
                                let shielded_value = match shield {
                                    ShieldTransition::V0(v0) => (-v0.value_balance) as u64,
                                };
                                let state = shielded_state.get_or_insert_with(ShieldedState::new);
                                state.record_shielded_note(shielded_value);
                                state.checkpoint();
                            }
                            operations.push(state_transition);
                        }
                    }
                    OperationType::ShieldFromAssetLock(amount_range) => {
                        for _i in 0..count {
                            let Some(state_transition) = self
                                .create_shield_from_asset_lock_transition(
                                    amount_range,
                                    rng,
                                    instant_lock_quorums,
                                    &platform.config,
                                    platform_version,
                                )
                            else {
                                break;
                            };
                            // Record the shielded note for potential future spends
                            if let StateTransition::ShieldFromAssetLock(ref shield) =
                                state_transition
                            {
                                let shielded_value = match shield {
                                    ShieldFromAssetLockTransition::V0(v0) => {
                                        (-v0.value_balance) as u64
                                    }
                                };
                                let state = shielded_state.get_or_insert_with(ShieldedState::new);
                                state.record_shielded_note(shielded_value);
                                state.checkpoint();
                            }
                            operations.push(state_transition);
                        }
                    }
                    OperationType::ShieldedTransfer(amount_range) => {
                        for _i in 0..count {
                            let Some(state_transition) = self.create_shielded_transfer_transition(
                                amount_range,
                                rng,
                                shielded_state,
                                platform_version,
                            ) else {
                                break;
                            };
                            operations.push(state_transition);
                        }
                    }
                    OperationType::Unshield(amount_range) => {
                        for _i in 0..count {
                            let Some(state_transition) = self.create_unshield_transition(
                                current_addresses_with_balance,
                                amount_range,
                                rng,
                                shielded_state,
                                platform_version,
                            ) else {
                                break;
                            };
                            operations.push(state_transition);
                        }
                    }
                    OperationType::ShieldedWithdrawal(amount_range) => {
                        for _i in 0..count {
                            let Some(state_transition) = self
                                .create_shielded_withdrawal_transition(
                                    amount_range,
                                    rng,
                                    shielded_state,
                                    platform_version,
                                )
                            else {
                                break;
                            };
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
        start_block_height: BlockHeight,
        block_info: &BlockInfo,
        current_identities: &mut Vec<Identity>,
        current_addresses_with_balance: &mut AddressesWithBalance,
        identity_nonce_counter: &mut BTreeMap<Identifier, u64>,
        contract_nonce_counter: &mut BTreeMap<(Identifier, Identifier), u64>,
        current_votes: &mut BTreeMap<Identifier, BTreeMap<Identifier, ResourceVoteChoice>>,
        signer: &mut SimpleSigner,
        rng: &mut StdRng,
        instant_lock_quorums: &Quorums<SigningQuorum>,
        shielded_state: &mut Option<ShieldedState>,
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
            Err(_) => (vec![], vec![]),
        };

        current_identities.append(&mut identities);

        let should_do_operation_transitions =
            if block_info.height == start_block_height && !current_identities.is_empty() {
                // add contracts on block 1
                let mut contract_state_transitions = self.initial_contract_state_transitions(
                    current_identities,
                    signer,
                    contract_nonce_counter,
                    rng,
                    platform_version,
                );
                state_transitions.append(&mut contract_state_transitions);
                block_info.height != 1
            } else {
                true
            };
        if should_do_operation_transitions {
            // Don't do any state transitions on block 1
            let (mut operation_based_state_transitions, mut add_to_finalize_block_operations) =
                self.operations_based_transitions(
                    platform,
                    block_info,
                    current_identities,
                    current_addresses_with_balance,
                    signer,
                    identity_nonce_counter,
                    contract_nonce_counter,
                    current_votes,
                    instant_lock_quorums,
                    rng,
                    platform_version,
                    shielded_state,
                );
            finalize_block_operations.append(&mut add_to_finalize_block_operations);
            state_transitions.append(&mut operation_based_state_transitions);

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
        key_count: KeyID,
        extra_keys: &KeyMaps,
        balance_range: &RangeInclusive<Credits>,
        signer: &mut SimpleSigner,
        rng: &mut StdRng,
        instant_lock_quorums: &Quorums<SigningQuorum>,
        platform_config: &PlatformConfig,
        platform_version: &PlatformVersion,
    ) -> Vec<(Identity, StateTransition)> {
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

        signer.add_identity_public_keys(keys);

        if self.sign_instant_locks {
            let identities_with_proofs = create_signed_instant_asset_lock_proofs_for_identities(
                identities,
                balance_range,
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
            create_state_transitions_for_identities(
                &mut identities,
                balance_range,
                signer,
                rng,
                platform_version,
            )
        }
    }

    // add this because strategy tests library now requires a callback and uses the actual chain.
    fn create_identity_top_up_transition(
        &self,
        rng: &mut StdRng,
        identity: &Identity,
        amount_range: &AmountRange,
        instant_lock_quorums: &Quorums<SigningQuorum>,
        platform_config: &PlatformConfig,
        platform_version: &PlatformVersion,
    ) -> StateTransition {
        let (_, pk) = ECDSA_SECP256K1
            .random_public_and_private_key_data(rng, platform_version)
            .unwrap();
        let sk: [u8; 32] = pk.try_into().unwrap();
        let secret_key = SecretKey::from_str(hex::encode(sk).as_str()).unwrap();
        let mut asset_lock_proof = instant_asset_lock_proof_fixture_with_dynamic_range(
            PrivateKey::new(secret_key, Network::Dash),
            amount_range,
            rng,
        );

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

    fn create_asset_lock_proof_with_amount(
        &self,
        rng: &mut StdRng,
        amount_range: &AmountRange,
        instant_lock_quorums: &Quorums<SigningQuorum>,
        platform_config: &PlatformConfig,
        platform_version: &PlatformVersion,
    ) -> (AssetLockProof, Vec<u8>, Credits) {
        let (_, pk) = ECDSA_SECP256K1
            .random_public_and_private_key_data(rng, platform_version)
            .unwrap();
        let sk_bytes: [u8; 32] = pk.try_into().unwrap();
        let secret_key = SecretKey::from_str(hex::encode(sk_bytes).as_str()).unwrap();
        let mut asset_lock_proof = instant_asset_lock_proof_fixture_with_dynamic_range(
            PrivateKey::new(secret_key, Network::Dash),
            amount_range,
            rng,
        );

        if self.sign_instant_locks {
            let quorum_config = QuorumConfig {
                quorum_type: platform_config.instant_lock.quorum_type,
                active_signers: platform_config.instant_lock.quorum_active_signers,
                rotation: platform_config.instant_lock.quorum_rotation,
                window: platform_config.instant_lock.quorum_window,
            };

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

        let funded_amount = match &asset_lock_proof {
            AssetLockProof::Instant(proof) => {
                let output_index = proof.output_index() as usize;
                proof
                    .transaction()
                    .output
                    .get(output_index)
                    .map(|output| output.value)
                    .unwrap_or_default()
            }
            AssetLockProof::Chain(_chain) => 0,
        };

        (
            asset_lock_proof,
            secret_key.secret_bytes().to_vec(),
            funded_amount,
        )
    }

    fn create_identity_top_up_from_addresses_transitions<S: Signer<PlatformAddress>>(
        &mut self,
        current_addresses_with_balance: &mut AddressesWithBalance,
        recipient: &Identity,
        amount_range: &AmountRange,
        signer: &S,
        rng: &mut StdRng,
        platform_version: &PlatformVersion,
    ) -> Option<StateTransition> {
        let inputs =
            current_addresses_with_balance.take_random_amounts_with_range(amount_range, rng)?;
        tracing::trace!(
            ?inputs,
            "Preparing identity top-up transition with addresses"
        );

        let top_up_transition =
            IdentityTopUpFromAddressesTransitionV0::try_from_inputs_with_signer(
                recipient,
                inputs,
                signer,
                0,
                platform_version,
                None,
            )
            .expect("expected to create top up from addresses transition"); // if you need to upcast to StateTransition

        tracing::debug!(
            ?top_up_transition,
            "Top up from addresses transition successfully signed"
        );

        Some(top_up_transition)
    }

    fn create_identity_from_addresses_transition(
        &mut self,
        current_addresses_with_balance: &mut AddressesWithBalance,
        amount_range: &AmountRange,
        maybe_output_amount: &MaybeOutputAmount,
        fee_strategy: &Option<AddressFundsFeeStrategy>,
        key_count: KeyCount,
        extra_keys: &ExtraKeys,
        signer: &mut SimpleSigner,
        rng: &mut StdRng,
        platform_version: &PlatformVersion,
    ) -> Option<(Identity, StateTransition)> {
        let inputs =
            current_addresses_with_balance.take_random_amounts_with_range(amount_range, rng)?;
        tracing::debug!(
            ?inputs,
            "Preparing identity create from addresses transition"
        );

        // Create a new identity with random keys
        let (mut identity, keys) = Identity::random_identity_with_main_keys_with_private_key::<
            Vec<_>,
        >(key_count, rng, platform_version)
        .expect("Expected to create identity with keys");

        // Add extra keys to the identity
        for (purpose, security_to_key_type_map) in extra_keys.iter() {
            for (security_level, key_types) in security_to_key_type_map {
                for key_type in key_types {
                    let (key, private_key) = IdentityPublicKey::random_key_with_known_attributes(
                        (identity.public_keys().len() + 1) as KeyID,
                        rng,
                        *purpose,
                        *security_level,
                        *key_type,
                        None,
                        platform_version,
                    )
                    .expect("expected to create random key");
                    identity.add_public_key(key.clone());
                    signer.add_identity_public_key(key, private_key);
                }
            }
        }

        // Add all keys to the signer
        signer.add_identity_public_keys(keys);

        // Determine fee strategy
        let fee_strategy = fee_strategy
            .clone()
            .unwrap_or(vec![AddressFundsFeeStrategyStep::DeductFromInput(0)]);

        // Create output if maybe_output_amount is provided
        let output = maybe_output_amount.as_ref().map(|output_range| {
            let output_amount = rng.gen_range(output_range.clone());
            let output_address = signer.add_random_address_key(rng);
            // Register the output address with balance
            current_addresses_with_balance.register_new_address_keep_only_highest(
                output_address.clone(),
                output_amount,
                self.max_addresses_to_choose_from_in_cache,
            );
            (output_address, output_amount)
        });

        let transition = IdentityCreateFromAddressesTransitionV0::try_from_inputs_with_signer(
            &identity,
            inputs,
            output,
            fee_strategy,
            signer, // identity public key signer
            signer, // address signer
            0,      // user_fee_increase
            platform_version,
        )
        .expect("expected to create identity from addresses transition");

        tracing::debug!(
            ?transition,
            "Identity create from addresses transition successfully signed"
        );

        Some((identity, transition))
    }

    fn create_address_transfer_transition(
        &mut self,
        current_addresses_with_balance: &mut AddressesWithBalance,
        amount_range: &AmountRange,
        output_count_range: &OutputCountRange,
        use_existing_outputs_chance: UseExistingAddressesAsOutputChance,
        fee_strategy: &Option<AddressFundsFeeStrategy>,
        signer: &mut SimpleSigner,
        rng: &mut StdRng,
        platform_version: &PlatformVersion,
    ) -> Option<StateTransition> {
        let inputs =
            current_addresses_with_balance.take_random_amounts_with_range(amount_range, rng)?;

        tracing::debug!(?inputs, "Preparing address funds transfer transition");

        // Calculate total input amount (we'll distribute this among outputs)
        let total_input: Credits = inputs.values().map(|(_, credits)| credits).sum();

        // Generate random number of outputs within the specified range
        let output_count = rng.gen_range(output_count_range.clone()).max(1) as usize;

        // Generate fee strategy: if not provided, reduce from outputs sequentially
        // Limited to 4 steps due to max_address_fee_strategies platform constraint
        let fee_strategy = fee_strategy.clone().unwrap_or_else(|| {
            let max_steps = output_count.min(4);
            (0..max_steps as u16)
                .map(AddressFundsFeeStrategyStep::ReduceOutput)
                .collect()
        });

        // Create output addresses and distribute funds evenly
        let amount_per_output = total_input / output_count as Credits;
        let mut outputs = BTreeMap::new();

        // Collect existing addresses that are not used as inputs (for potential reuse as outputs)
        let input_addresses: std::collections::HashSet<_> = inputs.keys().cloned().collect();
        let mut available_existing_addresses: Vec<_> = current_addresses_with_balance
            .addresses_with_balance
            .keys()
            .filter(|addr| !input_addresses.contains(*addr))
            .cloned()
            .collect();

        for _ in 0..output_count {
            // Check if we should use an existing address as output
            let use_existing = use_existing_outputs_chance
                .map(|chance| rng.gen::<f64>() < chance && !available_existing_addresses.is_empty())
                .unwrap_or(false);

            let address = if use_existing {
                // Pick a random existing address and remove it from available pool
                let idx = rng.gen_range(0..available_existing_addresses.len());
                let existing_address = available_existing_addresses.swap_remove(idx);
                // Update the balance for this existing address
                if let Some((nonce, balance)) = current_addresses_with_balance
                    .addresses_with_balance
                    .get(&existing_address)
                {
                    current_addresses_with_balance
                        .addresses_in_block_with_new_balance
                        .insert(
                            existing_address.clone(),
                            (*nonce, balance + amount_per_output),
                        );
                }
                existing_address
            } else {
                // Create a new address
                let new_address = signer.add_random_address_key(rng);
                current_addresses_with_balance
                    .addresses_in_block_with_new_balance
                    .insert(new_address.clone(), (0, amount_per_output));
                new_address
            };

            outputs.insert(address, amount_per_output);
        }

        let transfer_transition = AddressFundsTransferTransition::try_from_inputs_with_signer(
            inputs,
            outputs,
            fee_strategy,
            signer,
            0,
            platform_version,
        )
        .expect("expected to create address funds transfer transition");

        tracing::debug!(
            ?transfer_transition,
            "Address funds transfer transition successfully signed"
        );

        Some(transfer_transition)
    }

    fn create_address_withdrawal_transition(
        &mut self,
        current_addresses_with_balance: &mut AddressesWithBalance,
        amount_range: &AmountRange,
        maybe_output_amount: &MaybeOutputAmount,
        fee_strategy: &Option<AddressFundsFeeStrategy>,
        signer: &mut SimpleSigner,
        rng: &mut StdRng,
        platform_version: &PlatformVersion,
    ) -> Option<StateTransition> {
        let inputs =
            current_addresses_with_balance.take_random_amounts_with_range(amount_range, rng)?;

        let fee_strategy = fee_strategy
            .clone()
            .unwrap_or(vec![AddressFundsFeeStrategyStep::DeductFromInput(0)]);
        tracing::debug!(?inputs, "Preparing address credit withdrawal transition");

        // Determine if we have an output (change address) and its amount
        let output = if let Some(output_amount_range) = maybe_output_amount {
            let output_amount = rng.gen_range(output_amount_range.clone());
            let output_address = signer.add_random_address_key(rng);
            current_addresses_with_balance
                .addresses_in_block_with_new_balance
                .insert(output_address.clone(), (0, output_amount));
            Some((output_address, output_amount))
        } else {
            None
        };

        // Generate a random output script for the withdrawal
        let output_script = if rng.gen_bool(0.5) {
            CoreScript::random_p2pkh(rng)
        } else {
            CoreScript::random_p2sh(rng)
        };

        let withdrawal_transition = AddressCreditWithdrawalTransition::try_from_inputs_with_signer(
            inputs,
            output,
            fee_strategy,
            1, // core_fee_per_byte
            Pooling::Never,
            output_script,
            signer,
            0,
            platform_version,
        )
        .expect("expected to create address credit withdrawal transition");

        tracing::debug!(
            ?withdrawal_transition,
            "Address credit withdrawal transition successfully signed"
        );

        Some(withdrawal_transition)
    }

    fn create_address_funding_from_asset_lock_transitions(
        &mut self,
        current_addresses_with_balance: &mut AddressesWithBalance,
        amount_range: &AmountRange,
        rng: &mut StdRng,
        signer: &mut SimpleSigner,
        instant_lock_quorums: &Quorums<SigningQuorum>,
        platform_config: &PlatformConfig,
        platform_version: &PlatformVersion,
    ) -> Option<StateTransition> {
        let (asset_lock_proof, asset_lock_private_key, funded_amount) = self
            .create_asset_lock_proof_with_amount(
                rng,
                amount_range,
                instant_lock_quorums,
                platform_config,
                platform_version,
            );

        let address = signer.add_random_address_key(rng);
        current_addresses_with_balance.register_new_address_keep_only_highest(
            address,
            funded_amount,
            self.max_addresses_to_choose_from_in_cache,
        );
        let mut outputs = BTreeMap::new();
        outputs.insert(address.clone(), None);

        tracing::debug!(?outputs, "Preparing funding transition");
        let funding_transition =
            AddressFundingFromAssetLockTransitionV0::try_from_asset_lock_with_signer(
                asset_lock_proof,
                asset_lock_private_key.as_slice(),
                BTreeMap::new(),
                outputs,
                vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
                signer,
                0,
                platform_version,
            )
            .ok()?;

        Some(funding_transition)
    }

    /// Build a Shield state transition (transparent addresses → shielded pool).
    ///
    /// Creates an output-only Orchard bundle (no spends) with a real Halo 2 proof,
    /// signs the address input witnesses, and returns the transition.
    fn create_shield_transition(
        &mut self,
        current_addresses_with_balance: &mut AddressesWithBalance,
        amount_range: &AmountRange,
        signer: &mut SimpleSigner,
        rng: &mut StdRng,
        platform_version: &PlatformVersion,
    ) -> Option<StateTransition> {
        // 1. Pick input addresses with sufficient balances
        let inputs =
            current_addresses_with_balance.take_random_amounts_with_range(amount_range, rng)?;

        let total_input: Credits = inputs.values().map(|(_, credits)| credits).sum();

        tracing::debug!(?inputs, total_input, "Preparing shield transition");

        // 2. Create deterministic Orchard recipient (same key each time is fine for testing)
        let sk = SpendingKey::from_bytes([0u8; 32]).unwrap();
        let fvk = FullViewingKey::from(&sk);
        let recipient = fvk.address_at(0u32, Scope::External);

        // 3. Build output-only Orchard bundle (shield = outputs only, no spends)
        let anchor = Anchor::empty_tree();
        let mut builder = Builder::new(
            BundleType::Transactional {
                flags: OrchardFlags::SPENDS_DISABLED,
                bundle_required: false,
            },
            anchor,
        );

        // Use total_input as the shielded value (fee will be deducted from inputs)
        // value_balance will be negative (money flowing into the pool)
        let shield_value = total_input;
        builder
            .add_output(
                None,
                recipient,
                NoteValue::from_raw(shield_value),
                [0u8; 512],
            )
            .expect("expected to add output");

        // 4. Build → prove → sign
        let pk = get_proving_key();
        let mut bundle_rng = rand::rngs::OsRng;
        let (unauthorized, _) = builder
            .build::<i64>(&mut bundle_rng)
            .expect("expected to build bundle")
            .expect("expected bundle to be present");

        let bundle_commitment: [u8; 32] = unauthorized.commitment().into();
        let sighash = compute_platform_sighash(&bundle_commitment, &[]);
        let proven = unauthorized
            .create_proof(pk, &mut bundle_rng)
            .expect("expected to create proof");
        let bundle = proven
            .apply_signatures(bundle_rng, sighash, &[])
            .expect("expected to apply signatures");

        // 5. Decompose bundle into platform serialization fields
        let (actions, flags, value_balance, anchor_bytes, proof_bytes, binding_sig) =
            serialize_authorized_bundle(&bundle);

        // 6. Build ShieldTransition with signed address witnesses
        let fee_strategy: AddressFundsFeeStrategy =
            vec![AddressFundsFeeStrategyStep::DeductFromInput(0)].into();

        let shield_transition = ShieldTransition::try_from_bundle_with_signer(
            inputs,
            actions,
            flags,
            value_balance,
            anchor_bytes,
            proof_bytes,
            binding_sig,
            fee_strategy,
            signer,
            0,
            platform_version,
        )
        .expect("expected to create shield transition");

        tracing::debug!("Shield transition successfully built and signed");

        Some(shield_transition)
    }

    /// Build a ShieldFromAssetLock state transition (core asset lock -> shielded pool).
    ///
    /// Like Shield, this is output-only (no spends). The funds come from a core
    /// asset lock proof rather than platform address inputs.
    fn create_shield_from_asset_lock_transition(
        &mut self,
        amount_range: &AmountRange,
        rng: &mut StdRng,
        instant_lock_quorums: &Quorums<SigningQuorum>,
        platform_config: &PlatformConfig,
        platform_version: &PlatformVersion,
    ) -> Option<StateTransition> {
        // 1. Create asset lock proof
        let (asset_lock_proof, asset_lock_private_key, funded_amount) = self
            .create_asset_lock_proof_with_amount(
                rng,
                amount_range,
                instant_lock_quorums,
                platform_config,
                platform_version,
            );

        tracing::debug!(funded_amount, "Preparing shield from asset lock transition");

        // 2. Create deterministic Orchard recipient
        let sk = SpendingKey::from_bytes(TEST_SK_BYTES).unwrap();
        let fvk = FullViewingKey::from(&sk);
        let recipient = fvk.address_at(0u32, Scope::External);

        // 3. Build output-only Orchard bundle (same as Shield)
        let anchor = Anchor::empty_tree();
        let mut builder = Builder::new(
            BundleType::Transactional {
                flags: OrchardFlags::SPENDS_DISABLED,
                bundle_required: false,
            },
            anchor,
        );

        builder
            .add_output(
                None,
                recipient,
                NoteValue::from_raw(funded_amount),
                [0u8; 512],
            )
            .expect("expected to add output");

        // 4. Build -> prove -> sign
        let pk = get_proving_key();
        let mut bundle_rng = rand::rngs::OsRng;
        let (unauthorized, _) = builder
            .build::<i64>(&mut bundle_rng)
            .expect("expected to build bundle")
            .expect("expected bundle to be present");

        let bundle_commitment: [u8; 32] = unauthorized.commitment().into();
        let sighash = compute_platform_sighash(&bundle_commitment, &[]);
        let proven = unauthorized
            .create_proof(pk, &mut bundle_rng)
            .expect("expected to create proof");
        let bundle = proven
            .apply_signatures(bundle_rng, sighash, &[])
            .expect("expected to apply signatures");

        // 5. Decompose bundle
        let (actions, flags, value_balance, anchor_bytes, proof_bytes, binding_sig) =
            serialize_authorized_bundle(&bundle);

        // 6. Build ShieldFromAssetLockTransition
        let transition = ShieldFromAssetLockTransition::try_from_asset_lock_with_bundle(
            asset_lock_proof,
            asset_lock_private_key.as_slice(),
            actions,
            flags,
            value_balance,
            anchor_bytes,
            proof_bytes,
            binding_sig,
            0,
            platform_version,
        )
        .expect("expected to create shield from asset lock transition");

        tracing::debug!("ShieldFromAssetLock transition successfully built and signed");

        Some(transition)
    }

    /// Build a ShieldedTransfer state transition (shielded pool -> shielded pool).
    ///
    /// Spends an existing note and creates a new note with the same value.
    /// Requires notes from prior Shield or ShieldFromAssetLock transitions.
    fn create_shielded_transfer_transition(
        &mut self,
        _amount_range: &AmountRange,
        _rng: &mut StdRng,
        shielded_state: &mut Option<ShieldedState>,
        platform_version: &PlatformVersion,
    ) -> Option<StateTransition> {
        let state = shielded_state.as_mut()?;
        if !state.has_spendable_notes() {
            tracing::debug!("No spendable notes available for shielded transfer");
            return None;
        }

        let (note, merkle_path, anchor) = state.take_spendable_note()?;
        let note_value = note.value().inner();

        tracing::debug!(note_value, "Building shielded transfer bundle");

        let fvk = state.fvk.clone();
        let ask = state.ask.clone();
        let recipient = fvk.address_at(0u32, Scope::External);

        // Build bundle: spend note -> output same value (value_balance = 0)
        let mut builder = Builder::new(BundleType::DEFAULT, anchor);
        builder
            .add_spend(fvk, note, merkle_path)
            .expect("expected to add spend");
        builder
            .add_output(None, recipient, NoteValue::from_raw(note_value), [0u8; 512])
            .expect("expected to add output");

        let pk = get_proving_key();
        let mut bundle_rng = rand::rngs::OsRng;
        let (unauthorized, _) = builder
            .build::<i64>(&mut bundle_rng)
            .expect("expected to build bundle")
            .expect("expected bundle to be present");

        // Shielded transfer has no extra_data in sighash
        let bundle_commitment: [u8; 32] = unauthorized.commitment().into();
        let sighash = compute_platform_sighash(&bundle_commitment, &[]);
        let proven = unauthorized
            .create_proof(pk, &mut bundle_rng)
            .expect("expected to create proof");
        let bundle = proven
            .apply_signatures(bundle_rng, sighash, &[ask])
            .expect("expected to apply signatures");

        let (actions, flags, value_balance, anchor_bytes, proof_bytes, binding_sig) =
            serialize_authorized_bundle(&bundle);

        // value_balance should be 0 (all value stays in pool)
        // Cast i64 to u64 for the ShieldedTransferTransition API
        let transition = ShieldedTransferTransition::try_from_bundle(
            actions,
            flags,
            value_balance as u64,
            anchor_bytes,
            proof_bytes,
            binding_sig,
            0,
            platform_version,
        )
        .expect("expected to create shielded transfer transition");

        tracing::debug!("ShieldedTransfer transition successfully built");

        Some(transition)
    }

    /// Build an Unshield state transition (shielded pool -> platform address).
    ///
    /// Spends an existing note and sends the value to a platform address.
    /// Requires notes from prior Shield or ShieldFromAssetLock transitions.
    fn create_unshield_transition(
        &mut self,
        _current_addresses_with_balance: &mut AddressesWithBalance,
        _amount_range: &AmountRange,
        _rng: &mut StdRng,
        shielded_state: &mut Option<ShieldedState>,
        platform_version: &PlatformVersion,
    ) -> Option<StateTransition> {
        let state = shielded_state.as_mut()?;
        if !state.has_spendable_notes() {
            tracing::debug!("No spendable notes available for unshield");
            return None;
        }

        let (note, merkle_path, anchor) = state.take_spendable_note()?;
        let note_value = note.value().inner();

        tracing::debug!(note_value, "Building unshield bundle");

        let fvk = state.fvk.clone();
        let ask = state.ask.clone();
        let recipient = fvk.address_at(0u32, Scope::External);

        // Spend full note, output half back to pool, unshield the other half
        let unshield_amount = note_value / 2;
        let change_amount = note_value - unshield_amount;

        // Build bundle: spend note -> output change (value_balance = unshield_amount)
        let mut builder = Builder::new(BundleType::DEFAULT, anchor);
        builder
            .add_spend(fvk, note, merkle_path)
            .expect("expected to add spend");
        builder
            .add_output(
                None,
                recipient,
                NoteValue::from_raw(change_amount),
                [0u8; 512],
            )
            .expect("expected to add output");

        let pk = get_proving_key();
        let mut bundle_rng = rand::rngs::OsRng;
        let (unauthorized, _) = builder
            .build::<i64>(&mut bundle_rng)
            .expect("expected to build bundle")
            .expect("expected bundle to be present");

        // Unshield extra_data = output_address.to_bytes() || amount.to_le_bytes()
        let output_address = PlatformAddress::P2pkh([42u8; 20]);
        let amount = unshield_amount;
        let mut extra_sighash_data = output_address.to_bytes();
        extra_sighash_data.extend_from_slice(&amount.to_le_bytes());

        let bundle_commitment: [u8; 32] = unauthorized.commitment().into();
        let sighash = compute_platform_sighash(&bundle_commitment, &extra_sighash_data);
        let proven = unauthorized
            .create_proof(pk, &mut bundle_rng)
            .expect("expected to create proof");
        let bundle = proven
            .apply_signatures(bundle_rng, sighash, &[ask])
            .expect("expected to apply signatures");

        let (actions, flags, value_balance, anchor_bytes, proof_bytes, binding_sig) =
            serialize_authorized_bundle(&bundle);

        let transition = UnshieldTransition::try_from_bundle(
            output_address,
            amount,
            actions,
            flags,
            value_balance,
            anchor_bytes,
            proof_bytes,
            binding_sig,
            0,
            platform_version,
        )
        .expect("expected to create unshield transition");

        tracing::debug!(amount, "Unshield transition successfully built");

        Some(transition)
    }

    /// Build a ShieldedWithdrawal state transition (shielded pool -> core L1 address).
    ///
    /// Spends an existing note and withdraws the value to a core script.
    /// Requires notes from prior Shield or ShieldFromAssetLock transitions.
    fn create_shielded_withdrawal_transition(
        &mut self,
        _amount_range: &AmountRange,
        _rng: &mut StdRng,
        shielded_state: &mut Option<ShieldedState>,
        platform_version: &PlatformVersion,
    ) -> Option<StateTransition> {
        let state = shielded_state.as_mut()?;
        if !state.has_spendable_notes() {
            tracing::debug!("No spendable notes available for shielded withdrawal");
            return None;
        }

        let (note, merkle_path, anchor) = state.take_spendable_note()?;
        let note_value = note.value().inner();

        tracing::debug!(note_value, "Building shielded withdrawal bundle");

        let fvk = state.fvk.clone();
        let ask = state.ask.clone();
        let recipient = fvk.address_at(0u32, Scope::External);

        // Spend full note, output half back to pool, withdraw the other half
        let withdrawal_amount = note_value / 2;
        let change_amount = note_value - withdrawal_amount;

        // Build bundle: spend note -> output change (value_balance = withdrawal_amount)
        let mut builder = Builder::new(BundleType::DEFAULT, anchor);
        builder
            .add_spend(fvk, note, merkle_path)
            .expect("expected to add spend");
        builder
            .add_output(
                None,
                recipient,
                NoteValue::from_raw(change_amount),
                [0u8; 512],
            )
            .expect("expected to add output");

        let pk = get_proving_key();
        let mut bundle_rng = rand::rngs::OsRng;
        let (unauthorized, _) = builder
            .build::<i64>(&mut bundle_rng)
            .expect("expected to build bundle")
            .expect("expected bundle to be present");

        // ShieldedWithdrawal extra_data = output_script.as_bytes() || amount.to_le_bytes()
        let output_script = CoreScript::new_p2pkh([7u8; 20]);
        let amount = withdrawal_amount;
        let mut extra_sighash_data = output_script.as_bytes().to_vec();
        extra_sighash_data.extend_from_slice(&amount.to_le_bytes());

        let bundle_commitment: [u8; 32] = unauthorized.commitment().into();
        let sighash = compute_platform_sighash(&bundle_commitment, &extra_sighash_data);
        let proven = unauthorized
            .create_proof(pk, &mut bundle_rng)
            .expect("expected to create proof");
        let bundle = proven
            .apply_signatures(bundle_rng, sighash, &[ask])
            .expect("expected to apply signatures");

        let (actions, flags, value_balance, anchor_bytes, proof_bytes, binding_sig) =
            serialize_authorized_bundle(&bundle);

        let transition = ShieldedWithdrawalTransition::try_from_bundle(
            amount,
            actions,
            flags,
            value_balance,
            anchor_bytes,
            proof_bytes,
            binding_sig,
            1, // core_fee_per_byte
            Pooling::Never,
            output_script,
            0,
            platform_version,
        )
        .expect("expected to create shielded withdrawal transition");

        tracing::debug!(amount, "ShieldedWithdrawal transition successfully built");

        Some(transition)
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
    pub addresses_with_balance: AddressesWithBalance,
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
    pub signer: SimpleSigner,
}

impl ChainExecutionOutcome<'_> {
    pub fn current_quorum(&self) -> &TestQuorumInfo {
        self.validator_quorums
            .get::<QuorumHash>(&self.current_validator_quorum_hash)
            .unwrap()
    }
}

pub struct ChainExecutionParameters {
    pub block_start: u64,
    #[allow(dead_code)]
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
    pub current_identities: Vec<Identity>,
    pub current_addresses_with_balance: AddressesWithBalance,
}

fn create_signed_instant_asset_lock_proofs_for_identities(
    identities: Vec<Identity>,
    balance_range: &RangeInclusive<Credits>,
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

            let mut asset_lock_proof = instant_asset_lock_proof_fixture_with_dynamic_range(
                private_key,
                balance_range,
                rng,
            );

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
