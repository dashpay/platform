//! Comprehensive testing framework for Dash Platform using configurable strategies.
//!
//! This library provides the [`Strategy`] struct and supporting types for simulating
//! realistic blockchain activity on Dash Platform. It enables automated testing of
//! state transitions, contract interactions, identity management, and fund operations
//! across multiple blocks.
//!
//! # Overview
//!
//! Strategy tests simulate real-world platform usage by executing randomized sequences
//! of operations over configurable block ranges. A strategy defines:
//!
//! - **Initial state**: Identities, addresses, and contracts created at the start
//! - **Operations**: Actions to perform each block (documents, transfers, votes, etc.)
//! - **Frequency**: How often each operation type occurs
//!
//! # Block Execution Model
//!
//! Strategies execute over a sequence of blocks with specific timing:
//!
//! | Block | Activity |
//! |-------|----------|
//! | 1 (start) | Create start identities, fund start addresses |
//! | 2 | Create initial contracts from `start_contracts` |
//! | 3+ | Execute operations, insert new identities, apply contract updates |
//!
//! # Key Types
//!
//! - [`Strategy`]: The main configuration struct defining what to test
//! - [`StrategyConfig`]: Runtime parameters (start block, number of blocks)
//! - [`StartIdentities`]: Initial identities to create
//! - [`StartAddresses`]: Initial platform addresses to fund
//! - [`IdentityInsertInfo`]: Configuration for ongoing identity creation
//! - [`Operation`](operations::Operation): Individual operations with frequency settings
//!
//! # Usage
//!
//! Strategies are used in two main contexts:
//!
//! 1. **Platform TUI** (`dashpay/platform-tui`): Interactive terminal UI for
//!    defining, managing, and executing strategies
//! 2. **Drive ABCI tests** (`rs-drive-abci/tests/strategy_tests`): Automated
//!    integration tests that verify platform behavior under various conditions
//!
//! Programmatic usage via the [`Strategy`] struct:
//!
//! ```ignore
//! let strategy = Strategy {
//!     start_identities: StartIdentities { number_of_identities: 10, .. },
//!     start_contracts: vec![(my_contract, None)],
//!     operations: vec![document_insert_op, transfer_op],
//!     identity_inserts: IdentityInsertInfo { frequency: ..., ... },
//!     ..Default::default()
//! };
//!
//! // Execute for each block
//! let (transitions, finalize_ops, new_identities) = strategy.state_transitions_for_block(...);
//! ```
//!
//! # Submodules
//!
//! - [`addresses_with_balance`]: Two-phase address balance tracking
//! - [`frequency`]: Randomized event frequency configuration
//! - [`operations`]: Operation type definitions
//! - [`transitions`]: State transition factory functions

#![allow(clippy::result_large_err)]

use crate::addresses_with_balance::AddressesWithBalance;
use crate::frequency::Frequency;
use crate::operations::FinalizeBlockOperation::IdentityAddKeys;
use crate::operations::{
    DocumentAction, DocumentOp, FinalizeBlockOperation, IdentityUpdateOp, Operation, OperationType,
};
use bincode::{Decode, Encode};
use dpp::address_funds::fee_strategy::AddressFundsFeeStrategyStep;
use dpp::address_funds::PlatformAddress;
use dpp::block::block_info::BlockInfo;
use dpp::dashcore::PrivateKey;
use dpp::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
use dpp::data_contract::created_data_contract::CreatedDataContract;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::random_document::CreateRandomDocument;
use dpp::data_contract::document_type::v0::DocumentTypeV0;
use dpp::data_contract::document_type::DocumentType;
use dpp::data_contract::{DataContract, DataContractFactory};
use dpp::document::{Document, DocumentV0Getters};
use dpp::fee::Credits;
use dpp::identifier::Identifier;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::core_script::CoreScript;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::state_transition::asset_lock_proof::AssetLockProof;
use dpp::identity::IdentityPublicKey;
use dpp::identity::{Identity, KeyID, KeyType, PartialIdentity, Purpose, SecurityLevel};
use dpp::platform_value::string_encoding::Encoding;
use dpp::platform_value::{BinaryData, Bytes32, Value};
use dpp::serialization::{
    PlatformDeserializableWithPotentialValidationFromVersionedStructure,
    PlatformSerializableWithPlatformVersion,
};
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
    DocumentDeleteTransition, DocumentReplaceTransition, DocumentTransferTransition,
};
use dpp::state_transition::batch_transition::document_base_transition::v0::DocumentBaseTransitionV0;
use dpp::state_transition::batch_transition::document_create_transition::{
    DocumentCreateTransition, DocumentCreateTransitionV0,
};
use dpp::state_transition::batch_transition::{BatchTransition, BatchTransitionV0};
use dpp::state_transition::data_contract_create_transition::methods::v0::DataContractCreateTransitionMethodsV0;
use dpp::state_transition::data_contract_create_transition::DataContractCreateTransition;
use dpp::state_transition::data_contract_update_transition::methods::DataContractUpdateTransitionMethodsV0;
use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use dpp::state_transition::identity_create_from_addresses_transition::methods::IdentityCreateFromAddressesTransitionMethodsV0;
use dpp::state_transition::identity_create_from_addresses_transition::v0::IdentityCreateFromAddressesTransitionV0;
use dpp::state_transition::identity_topup_from_addresses_transition::methods::IdentityTopUpFromAddressesTransitionMethodsV0;
use dpp::state_transition::identity_topup_from_addresses_transition::v0::IdentityTopUpFromAddressesTransitionV0;
use dpp::state_transition::StateTransition;
use dpp::version::PlatformVersion;
use dpp::withdrawal::Pooling;
use dpp::ProtocolError::{PlatformDeserializationError, PlatformSerializationError};
use dpp::{dash_to_duffs, ProtocolError};
use drive::drive::identity::key::fetch::{IdentityKeysRequest, KeyRequestType};
use operations::{DataContractUpdateAction, DataContractUpdateOp};
use platform_version::TryFromPlatformVersioned;
use rand::prelude::StdRng;
use rand::seq::{IteratorRandom, SliceRandom};
use rand::Rng;
use simple_signer::signer::SimpleSigner;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::ops::RangeInclusive;
use transitions::create_identity_credit_transfer_transition;

pub mod addresses_with_balance;
pub mod frequency;
pub mod operations;
pub mod transitions;

/// The core configuration for a Dash Platform testing strategy.
///
/// A `Strategy` defines everything needed to simulate realistic platform activity
/// over a sequence of blocks. It specifies initial conditions (identities, addresses,
/// contracts) and ongoing operations (document actions, transfers, votes, etc.).
///
/// # Execution Timing
///
/// - **Block 1** (`start_block_height`): Creates start identities and funds start addresses
/// - **Block 2**: Deploys initial contracts from `start_contracts`
/// - **Block 3+**: Executes operations, inserts new identities, applies contract updates
///
/// # Fields
///
/// - `start_identities`: Identities created on the first block
/// - `start_addresses`: Platform addresses funded on the first block
/// - `start_contracts`: Contracts deployed on the second block, with optional scheduled updates
/// - `operations`: Actions to perform each block based on their frequency settings
/// - `identity_inserts`: Configuration for ongoing identity creation (block 3+)
/// - `identity_contract_nonce_gaps`: Optional nonce gaps for testing edge cases
/// - `signer`: Key manager for signing state transitions
///
/// # Example
///
/// ```ignore
/// let strategy = Strategy {
///     start_identities: StartIdentities {
///         number_of_identities: 10,
///         keys_per_identity: 3,
///         starting_balances: dash_to_duffs!(1),
///         ..Default::default()
///     },
///     start_addresses: StartAddresses {
///         number_of_addresses: 5,
///         starting_balance: dash_to_credits!(0.5),
///         ..Default::default()
///     },
///     operations: vec![document_op, transfer_op],
///     ..Default::default()
/// };
/// ```
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Strategy {
    /// Identities to create on the first block.
    pub start_identities: StartIdentities,

    /// Platform addresses to fund on the first block.
    pub start_addresses: StartAddresses,

    /// Contracts to deploy on the second block, with optional scheduled updates.
    /// Each tuple contains the initial contract and an optional map of
    /// `block_offset -> updated_contract` for time-based contract evolution.
    pub start_contracts: Vec<(
        CreatedDataContract,
        Option<BTreeMap<u64, CreatedDataContract>>,
    )>,

    /// Operations to execute each block based on their frequency settings.
    pub operations: Vec<Operation>,

    /// Configuration for ongoing identity creation (block 3+).
    pub identity_inserts: IdentityInsertInfo,

    /// Optional nonce gaps to inject for testing edge cases.
    /// When set, randomly increments nonces to simulate real-world entropy.
    pub identity_contract_nonce_gaps: Option<Frequency>,

    /// Key manager for signing all state transitions.
    /// Required for most operations; initialized with identity keys.
    pub signer: Option<SimpleSigner>,
}

impl Strategy {
    /// Returns the maximum number of document operations that don't create new documents.
    ///
    /// Counts operations like delete, replace, and transfer (excluding insert operations).
    /// Useful for capacity planning when existing documents are required.
    pub fn max_document_operation_count_without_inserts(&self) -> u16 {
        self.operations
            .iter()
            .filter(|operation| match &operation.op_type {
                OperationType::Document(document_op) => !matches!(
                    &document_op.action,
                    DocumentAction::DocumentActionInsertRandom(_, _)
                        | DocumentAction::DocumentActionInsertSpecific(_, _, _, _)
                ),
                _ => false,
            })
            .count() as u16
    }
}

/// Runtime configuration for strategy execution.
///
/// Specifies the block range over which the strategy runs. These values
/// are provided at execution time, separate from the strategy definition.
#[derive(Clone, Debug, PartialEq)]
pub struct StrategyConfig {
    /// The block height at which to begin executing the strategy.
    pub start_block_height: u64,
    /// Total number of blocks to simulate.
    pub number_of_blocks: u64,
}

/// Maps key purposes to security levels to key types for identity key generation.
///
/// Used to specify additional keys when creating identities. The nested structure
/// allows fine-grained control over which key types are created for each
/// purpose/security-level combination.
///
/// Example: Create a HIGH-security ECDSA key for AUTHENTICATION:
/// ```ignore
/// let extra_keys: KeyMaps = BTreeMap::from([(
///     Purpose::AUTHENTICATION,
///     BTreeMap::from([(SecurityLevel::HIGH, vec![KeyType::ECDSA_SECP256K1])])
/// )]);
/// ```
pub type KeyMaps = BTreeMap<Purpose, BTreeMap<SecurityLevel, Vec<KeyType>>>;

/// Configuration for identities created at strategy start (block 1).
///
/// These identities are created via asset lock proofs and are available
/// for all subsequent operations. They form the initial participant pool.
#[derive(Clone, Debug, PartialEq, Default, Encode, Decode)]
pub struct StartIdentities {
    /// Number of random identities to generate and register.
    pub number_of_identities: u16,
    /// Number of main keys per identity (authentication, etc.).
    pub keys_per_identity: u8,
    /// Initial credit balance for each identity (in duffs, 1 Dash = 100,000,000 duffs).
    pub starting_balances: u64,
    /// Additional keys to create beyond the main keys.
    /// Allows specifying keys for specific purposes/security levels.
    pub extra_keys: KeyMaps,
    /// Pre-defined identities with optional pre-signed state transitions.
    /// Useful for testing with known identity IDs or specific key configurations.
    pub hard_coded: Vec<(Identity, Option<StateTransition>)>,
}

/// Configuration for platform addresses funded at strategy start (block 1).
///
/// Platform addresses can hold credits and participate in address-based
/// operations like transfers and withdrawals. Unlike identities, addresses
/// don't have keys or the ability to sign arbitrary state transitions.
#[derive(Clone, Debug, PartialEq, Default, Encode, Decode)]
pub struct StartAddresses {
    /// Number of random addresses to create and fund.
    pub number_of_addresses: u16,
    /// Initial credit balance for each random address.
    pub starting_balance: Credits,
    /// Pre-defined addresses with specific balances.
    /// The addresses are registered with their specified balances.
    pub extra_addresses: BTreeMap<PlatformAddress, Credits>,
}

/// Configuration for ongoing identity creation during strategy execution.
///
/// Controls how new identities are introduced during blocks 3+, enabling
/// simulation of growing participant pools and testing identity creation
/// under various conditions.
#[derive(Clone, Debug, PartialEq, Encode, Decode)]
pub struct IdentityInsertInfo {
    /// How often to create new identities and how many per block.
    pub frequency: Frequency,
    /// Number of main keys for each new identity.
    pub start_keys: u8,
    /// Additional keys beyond the main keys.
    pub extra_keys: KeyMaps,
    /// Credit balance range for new identities (randomly selected).
    pub start_balance_range: RangeInclusive<Credits>,
}

impl Default for IdentityInsertInfo {
    fn default() -> Self {
        Self {
            frequency: Default::default(),
            start_keys: 5,
            extra_keys: Default::default(),
            start_balance_range: dash_to_duffs!(1)..=dash_to_duffs!(1),
        }
    }
}

/// Query specification for retrieving random documents from the platform.
///
/// Used with document query callbacks to fetch existing documents for
/// operations like replace, delete, or transfer.
#[derive(Clone, Debug, PartialEq)]
pub struct RandomDocumentQuery<'a> {
    /// The contract containing the document type.
    pub data_contract: &'a DataContract,
    /// The document type to query.
    pub document_type: &'a DocumentType,
}

/// Query types supported by the document query callback.
///
/// Currently supports random document queries; may be extended with
/// additional query types in the future.
#[derive(Clone, Debug, PartialEq)]
pub enum LocalDocumentQuery<'a> {
    /// Query for random documents matching a contract/type.
    RandomDocumentQuery(RandomDocumentQuery<'a>),
}

#[derive(Clone, Debug, Encode, Decode)]
struct StrategyInSerializationFormat {
    // TODO: Use type or struct
    #[allow(clippy::type_complexity)]
    pub contracts_with_updates: Vec<(Vec<u8>, Option<BTreeMap<u64, Vec<u8>>>)>,
    pub operations: Vec<Vec<u8>>,
    pub start_identities: StartIdentities,
    pub start_addresses: StartAddresses,
    pub identities_inserts: IdentityInsertInfo,
    pub identity_contract_nonce_gaps: Option<Frequency>,
    pub signer: Option<SimpleSigner>,
}

impl PlatformSerializableWithPlatformVersion for Strategy {
    type Error = ProtocolError;

    fn serialize_to_bytes_with_platform_version(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, ProtocolError> {
        self.clone()
            .serialize_consume_to_bytes_with_platform_version(platform_version)
    }

    fn serialize_consume_to_bytes_with_platform_version(
        self,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, ProtocolError> {
        let Strategy {
            start_contracts: contracts_with_updates,
            operations,
            start_identities,
            start_addresses,
            identity_inserts: identities_inserts,
            identity_contract_nonce_gaps,
            signer,
        } = self;

        let contract_with_updates_in_serialization_format = contracts_with_updates
            .into_iter()
            .map(|(created_data_contract, maybe_updates)| {
                let serialized_created_data_contract = created_data_contract
                    .serialize_consume_to_bytes_with_platform_version(platform_version)?;
                let maybe_updates = maybe_updates
                    .map(|updates| {
                        updates
                            .into_iter()
                            .map(|(key, value)| {
                                let serialized_created_data_contract_update = value
                                    .serialize_consume_to_bytes_with_platform_version(
                                        platform_version,
                                    )?;
                                Ok((key, serialized_created_data_contract_update))
                            })
                            .collect::<Result<BTreeMap<u64, Vec<u8>>, ProtocolError>>()
                    })
                    .transpose()?;
                Ok((serialized_created_data_contract, maybe_updates))
            })
            .collect::<Result<Vec<(Vec<u8>, Option<BTreeMap<u64, Vec<u8>>>)>, ProtocolError>>()?;

        let operations_in_serialization_format = operations
            .into_iter()
            .map(|operation| {
                operation.serialize_consume_to_bytes_with_platform_version(platform_version)
            })
            .collect::<Result<Vec<Vec<u8>>, ProtocolError>>()?;

        let strategy_in_serialization_format = StrategyInSerializationFormat {
            contracts_with_updates: contract_with_updates_in_serialization_format,
            operations: operations_in_serialization_format,
            start_identities,
            start_addresses,
            identities_inserts,
            identity_contract_nonce_gaps,
            signer,
        };

        let config = bincode::config::standard()
            .with_big_endian()
            .with_no_limit();
        bincode::encode_to_vec(strategy_in_serialization_format, config)
            .map_err(|e| PlatformSerializationError(format!("unable to serialize Strategy: {}", e)))
    }
}

impl PlatformDeserializableWithPotentialValidationFromVersionedStructure for Strategy {
    fn versioned_deserialize(
        data: &[u8],
        full_validation: bool,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized,
    {
        let config = bincode::config::standard()
            .with_big_endian()
            .with_no_limit();
        let strategy: StrategyInSerializationFormat =
            bincode::borrow_decode_from_slice(data, config)
                .map_err(|e| {
                    PlatformDeserializationError(format!("unable to deserialize Strategy: {}", e))
                })?
                .0;

        let StrategyInSerializationFormat {
            contracts_with_updates,
            operations,
            start_identities,
            start_addresses,
            identities_inserts,
            identity_contract_nonce_gaps,
            signer,
        } = strategy;

        let contracts_with_updates = contracts_with_updates
            .into_iter()
            .map(|(serialized_contract, maybe_updates)| {
                let contract = CreatedDataContract::versioned_deserialize(
                    serialized_contract.as_slice(),
                    full_validation,
                    platform_version,
                )?;
                let maybe_updates = maybe_updates
                    .map(|updates| {
                        updates
                            .into_iter()
                            .map(|(key, serialized_contract_update)| {
                                let update = CreatedDataContract::versioned_deserialize(
                                    serialized_contract_update.as_slice(),
                                    full_validation,
                                    platform_version,
                                )?;
                                Ok((key, update))
                            })
                            .collect::<Result<BTreeMap<u64, CreatedDataContract>, ProtocolError>>()
                    })
                    .transpose()?;
                Ok((contract, maybe_updates))
            })
            .collect::<Result<
                Vec<(
                    CreatedDataContract,
                    Option<BTreeMap<u64, CreatedDataContract>>,
                )>,
                ProtocolError,
            >>()?;

        let operations = operations
            .into_iter()
            .map(|operation| {
                Operation::versioned_deserialize(
                    operation.as_slice(),
                    full_validation,
                    platform_version,
                )
            })
            .collect::<Result<Vec<Operation>, ProtocolError>>()?;

        Ok(Strategy {
            start_contracts: contracts_with_updates,
            operations,
            start_identities,
            start_addresses,
            identity_inserts: identities_inserts,
            identity_contract_nonce_gaps,
            signer,
        })
    }
}

/// Tracks document counts in the mempool per (identity, contract) pair.
///
/// The platform limits each identity to 24 pending documents per contract
/// in the mempool at any time. This counter tracks current counts to avoid
/// exceeding that limit when generating new document operations.
///
/// Key: `(identity_id, contract_id)` → Value: number of pending documents
pub type MempoolDocumentCounter<'c> = BTreeMap<(Identifier, Identifier), u64>;

/// Selects identities capable of submitting documents for a given contract.
///
/// Filters out identities that have reached the mempool limit (24 documents)
/// for the specified contract. If more documents are needed than available
/// identities, will reuse identities up to their remaining capacity.
///
/// # Parameters
/// - `identities`: Pool of available identities
/// - `contract`: The target contract for document submission
/// - `mempool_document_counter`: Current pending document counts
/// - `count`: Number of identities needed
/// - `rng`: Random number generator for selection
///
/// # Returns
/// A vector of identity references, potentially with duplicates if reuse
/// is required to meet the requested count.
fn choose_capable_identities<'i>(
    identities: &'i [Identity],
    contract: &DataContract,
    mempool_document_counter: &MempoolDocumentCounter,
    count: usize,
    rng: &mut StdRng,
) -> Vec<&'i Identity> {
    // Filter out those that already have 24 documents in the mempool for this contract
    let mut all_capable_identities: Vec<&'i Identity> = identities
        .iter()
        .filter(|identity| {
            mempool_document_counter
                .get(&(identity.id(), contract.id()))
                .unwrap_or(&0)
                < &24u64
        })
        .collect();

    // If we need more documents than we have eligible identities, we will need to use some identities more than once
    if all_capable_identities.len() < count {
        let mut additional_identities_count = count - all_capable_identities.len();

        let mut additional_identities = Vec::with_capacity(additional_identities_count);
        let mut index = 0;
        while additional_identities_count > 0 && index < all_capable_identities.len() {
            // Iterate through the identities only one time, using as many documents as each identity can submit
            // We should see if this often causes identities to max out their mempool count, which could potentially
            // lead to issues, even though this function should really only be covering an edge case
            let identity = all_capable_identities[index];

            // How many documents are in the mempool for this (identity, contract) plus the one that we are already planning to use
            let mempool_documents_count = mempool_document_counter
                .get(&(identity.id(), contract.id()))
                .unwrap_or(&0)
                + 1;

            let identity_capacity = 24 - mempool_documents_count;

            if identity_capacity > 0 {
                for _ in 0..identity_capacity {
                    additional_identities.push(identity);
                    additional_identities_count -= 1;
                    if additional_identities_count == 0 {
                        break;
                    }
                }
            }

            index += 1;
        }

        all_capable_identities.extend(additional_identities);
        all_capable_identities
    } else {
        all_capable_identities
            .iter()
            .choose_multiple(rng, count)
            .into_iter()
            .cloned()
            .collect()
    }
}

impl Strategy {
    /// Generates comprehensive state transitions for a given block, including handling new identities and contracts.
    ///
    /// This primary function orchestrates the generation of state transitions for a block, accounting for
    /// new identities, document operations, contract creations, and updates. It serves as the main entry point
    /// for simulating activities on the Dash Platform based on the defined strategy. The function integrates
    /// the creation and management of identities, their related transactions, and the dynamics of contracts and
    /// document operations to provide a holistic view of block activities.
    ///
    /// Internally, it calls `operations_based_transitions` to process specific operations and generates
    /// additional transitions related to identities and contracts. It's designed to simulate a realistic
    /// blockchain environment, enabling the testing of complex scenarios and strategies.
    ///
    /// # Parameters
    /// - `document_query_callback`: Callback for querying documents based on specified criteria.
    /// - `identity_fetch_callback`: Callback for fetching identity details, including public keys.
    /// - `asset_lock_proofs`: A vector of asset lock proofs and associated private keys.
    /// - `block_info`: Information about the current block, such as its height and timestamp.
    /// - `current_identities`: A mutable list of identities present in the simulation, potentially expanded with new identities.
    /// - `known_contracts`: A mutable map of contracts known in the simulation, including any updates.
    /// - `signer`: A mutable reference to a signer instance for signing transactions.
    /// - `identity_nonce_counter`: Tracks nonce values for identities, crucial for transaction uniqueness.
    /// - `contract_nonce_counter`: Tracks nonce values for contract interactions.
    /// - `rng`: A mutable random number generator for creating randomized elements in transactions.
    /// - `config`: Configuration details for the strategy, including block start height and number of blocks.
    /// - `platform_version`: Specifies the platform version for compatibility with Dash Platform features.
    ///
    /// # Returns
    /// A tuple containing:
    /// 1. `Vec<StateTransition>`: A collection of state transitions generated for the block.
    /// 2. `Vec<FinalizeBlockOperation>`: Operations that need to be finalized at the block's end, often related to identity updates.
    ///
    /// # Usage
    /// This function is typically called once per block during simulation, with its output used to apply transactions
    /// and operations within the simulated Dash Platform environment.
    ///
    /// ```ignore
    /// let (state_transitions, finalize_ops) = strategy.state_transitions_for_block(
    ///     &mut document_query_callback,
    ///     &mut identity_fetch_callback,
    ///     &mut asset_lock_proofs,
    ///     &block_info,
    ///     &mut current_identities,
    ///     &mut known_contracts,
    ///     &mut signer,
    ///     &mut identity_nonce_counter,
    ///     &mut contract_nonce_counter,
    ///     &mut rng,
    ///     &config,
    ///     &platform_version,
    /// );
    /// ```
    ///
    /// # Note
    /// This function is central to simulating the lifecycle of block processing and strategy execution
    /// on the Dash Platform. It encapsulates the complexity of transaction generation, identity management,
    /// and contract dynamics within a block's context.
    #[allow(clippy::too_many_arguments)]
    pub fn state_transitions_for_block(
        &mut self,
        document_query_callback: &mut impl FnMut(LocalDocumentQuery) -> Vec<Document>,
        identity_fetch_callback: &mut impl FnMut(
            Identifier,
            Option<IdentityKeysRequest>,
        ) -> PartialIdentity,
        asset_lock_proofs: &mut Vec<(AssetLockProof, PrivateKey)>,
        block_info: &BlockInfo,
        current_identities: &mut Vec<Identity>,
        current_addresses_with_balance: &mut AddressesWithBalance,
        known_contracts: &mut BTreeMap<String, DataContract>,
        signer: &mut SimpleSigner,
        identity_nonce_counter: &mut BTreeMap<Identifier, u64>,
        contract_nonce_counter: &mut BTreeMap<(Identifier, Identifier), u64>,
        mempool_document_counter: &MempoolDocumentCounter,
        rng: &mut StdRng,
        config: &StrategyConfig,
        platform_version: &PlatformVersion,
    ) -> (
        Vec<StateTransition>,
        Vec<FinalizeBlockOperation>,
        Vec<Identity>,
    ) {
        let mut finalize_block_operations = vec![];

        // Get identity state transitions
        // Start identities are done on the 1st block, identity inserts are done on 3rd+ blocks
        let identity_state_transitions = match self.identity_state_transitions_for_block(
            block_info,
            self.start_identities.starting_balances,
            signer,
            rng,
            asset_lock_proofs,
            config,
            platform_version,
        ) {
            Ok(transitions) => transitions,
            Err(e) => {
                tracing::error!("identity_state_transitions_for_block error: {}", e);
                return (vec![], finalize_block_operations, vec![]);
            }
        };

        // Create state_transitions vec and identities vec based on identity_state_transitions outcome
        let (identities, mut state_transitions): (Vec<Identity>, Vec<StateTransition>) =
            identity_state_transitions.into_iter().unzip();

        // Initialize start addresses on the first block by funding them from asset lock proofs
        if block_info.height == config.start_block_height {
            tracing::info!(
                "Creating {} AddressFundingFromAssetLock transitions (block {})",
                self.start_addresses.number_of_addresses,
                block_info.height
            );
            // Fund random addresses from asset lock proofs
            for i in 0..self.start_addresses.number_of_addresses {
                // Get an asset lock proof from the pool
                let Some((asset_lock_proof, private_key)) = asset_lock_proofs.pop() else {
                    tracing::warn!(
                        "No asset lock proofs available for start_addresses (funded {} of {})",
                        i,
                        self.start_addresses.number_of_addresses
                    );
                    break;
                };

                // Get the funded amount from the asset lock proof
                let funded_amount = match &asset_lock_proof {
                    AssetLockProof::Instant(proof) => {
                        let output_index = proof.output_index() as usize;
                        proof
                            .transaction()
                            .output
                            .get(output_index)
                            .map(|output| output.value * 1000) // Convert sats to credits
                            .unwrap_or(self.start_addresses.starting_balance)
                    }
                    AssetLockProof::Chain(_) => self.start_addresses.starting_balance,
                };

                // Create a new address for the funding
                let address = signer.add_random_address_key(rng);
                current_addresses_with_balance.register_new_address(address, funded_amount);

                let mut outputs = BTreeMap::new();
                outputs.insert(address, None);

                let funding_transition =
                    AddressFundingFromAssetLockTransitionV0::try_from_asset_lock_with_signer(
                        asset_lock_proof,
                        private_key.inner.secret_bytes().as_slice(),
                        BTreeMap::new(), // no additional inputs
                        outputs,
                        vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
                        signer,
                        0,
                        platform_version,
                    );

                match funding_transition {
                    Ok(transition) => state_transitions.push(transition),
                    Err(e) => {
                        tracing::error!(
                            "Error creating address funding transition for start_address: {:?}",
                            e
                        );
                    }
                }
            }

            // Add extra hard-coded addresses (these are assumed to already exist on platform)
            for (address, balance) in &self.start_addresses.extra_addresses {
                current_addresses_with_balance.register_new_address(*address, *balance);
            }

            // Commit the initial addresses immediately so they're available for operations
            current_addresses_with_balance.commit();
        }

        // Add initial contracts for contracts_with_updates on 2nd block of strategy
        #[allow(clippy::comparison_chain)]
        if block_info.height == config.start_block_height + 1 {
            let mut contract_state_transitions = self.initial_contract_state_transitions(
                current_identities,
                identity_nonce_counter,
                signer,
                rng,
                platform_version,
            );
            state_transitions.append(&mut contract_state_transitions);
        } else if block_info.height > config.start_block_height + 1 {
            let (mut operations_state_transitions, mut add_to_finalize_block_operations) = self
                .operations_based_transitions(
                    document_query_callback,
                    identity_fetch_callback,
                    asset_lock_proofs,
                    block_info,
                    current_identities,
                    current_addresses_with_balance,
                    known_contracts,
                    signer,
                    identity_nonce_counter,
                    contract_nonce_counter,
                    mempool_document_counter,
                    rng,
                    platform_version,
                );
            finalize_block_operations.append(&mut add_to_finalize_block_operations);
            state_transitions.append(&mut operations_state_transitions);

            // Contract updates for contracts_with_updates
            let mut initial_contract_update_state_transitions = self
                .initial_contract_update_state_transitions(
                    current_identities,
                    block_info.height,
                    config.start_block_height,
                    signer,
                    contract_nonce_counter,
                    platform_version,
                );
            state_transitions.append(&mut initial_contract_update_state_transitions);
        }

        (state_transitions, finalize_block_operations, identities)
    }

    /// Processes strategy operations to generate state transitions specific to operations for a given block.
    ///
    /// This function is responsible for generating state transitions based on the operations defined within
    /// the strategy. It evaluates each operation's conditions and frequency to determine if a transition should
    /// be created for the current block. The function supports a variety of operations, including document
    /// creation, updates, deletions, identity-related transactions, and more, each tailored to the specifics
    /// of the Dash Platform.
    ///
    /// `operations_based_transitions` is called internally by `state_transitions_for_block`
    /// to handle the operational aspects of the strategy. While it focuses on the execution of operations,
    /// it is part of a larger workflow that includes managing new identities, contracts, and their updates
    /// across blocks.
    ///
    /// # Parameters
    /// - `document_query_callback`: A callback function for querying existing documents based on specified criteria.
    /// - `identity_fetch_callback`: A callback function for fetching identity information, including public keys.
    /// - `asset_lock_proofs`: A vector of asset lock proofs and associated private keys.
    /// - `block_info`: Information about the current block, including height and time.
    /// - `current_identities`: A mutable reference to the list of current identities involved in the operations.
    /// - `known_contracts`: A mutable reference to a map of known contracts and their updates.
    /// - `signer`: A mutable reference to a signer instance used for transaction signatures.
    /// - `identity_nonce_counter`: A mutable reference to a map tracking nonce values for identities.
    /// - `contract_nonce_counter`: A mutable reference to a map tracking nonce values for contract interactions.
    /// - `rng`: A mutable reference to a random number generator for random value generation in operations.
    /// - `platform_version`: The platform version, ensuring compatibility with Dash Platform features.
    ///
    /// # Returns
    /// A tuple containing:
    /// 1. `Vec<StateTransition>`: A vector of state transitions generated based on the operations for the current block.
    /// 2. `Vec<FinalizeBlockOperation>`: A vector of operations that need to be finalized by the end of the block processing.
    ///
    /// # Usage
    /// This function is a critical component of the strategy execution process, providing detailed control over
    /// the generation and management of operations-specific state transitions within a block's context. It is not
    /// typically called directly by external code but is an essential part of the internal mechanics of generating
    /// state transitions for simulation or testing purposes.
    ///
    /// ```ignore
    /// let (state_transitions, finalize_ops) = strategy.operations_based_transitions(
    ///     &mut document_query_callback,
    ///     &mut identity_fetch_callback,
    ///     &mut asset_lock_proofs,
    ///     &block_info,
    ///     &mut current_identities,
    ///     &mut known_contracts,
    ///     &mut signer,
    ///     &mut identity_nonce_counter,
    ///     &mut contract_nonce_counter,
    ///     &mut rng,
    ///     &platform_version,
    /// );
    /// ```
    ///
    /// # Note
    /// This function plays a pivotal role in simulating realistic blockchain operations, allowing for the
    /// detailed and nuanced execution of a wide range of actions on the Dash Platform as defined by the strategy.
    #[allow(clippy::too_many_arguments)]
    pub fn operations_based_transitions(
        &self,
        document_query_callback: &mut impl FnMut(LocalDocumentQuery) -> Vec<Document>,
        identity_fetch_callback: &mut impl FnMut(
            Identifier,
            Option<IdentityKeysRequest>,
        ) -> PartialIdentity,
        asset_lock_proofs: &mut Vec<(AssetLockProof, PrivateKey)>,
        block_info: &BlockInfo,
        current_identities: &mut Vec<Identity>,
        current_addresses_with_balance: &mut AddressesWithBalance,
        known_contracts: &mut BTreeMap<String, DataContract>,
        signer: &mut SimpleSigner,
        identity_nonce_counter: &mut BTreeMap<Identifier, u64>,
        contract_nonce_counter: &mut BTreeMap<(Identifier, Identifier), u64>,
        mempool_document_counter: &MempoolDocumentCounter,
        rng: &mut StdRng,
        platform_version: &PlatformVersion,
    ) -> (Vec<StateTransition>, Vec<FinalizeBlockOperation>) {
        // Lists to store generated operations and block finalization operations
        let mut operations = vec![];
        let mut finalize_block_operations = vec![];

        // Lists to keep track of replaced and deleted documents
        let mut replaced = vec![];
        let mut transferred = vec![];
        let mut deleted = vec![];

        // Loop through the operations and generate state transitions based on frequency and type
        for op in &self.operations {
            // Check if the op frequency hits for this block
            if op.frequency.check_hit(rng) {
                // Get times_per_block
                let count = rng.gen_range(op.frequency.times_per_block_range.clone());
                match &op.op_type {
                    // Generate state transition for document insert operation with random data
                    OperationType::Document(DocumentOp {
                        action: DocumentAction::DocumentActionInsertRandom(fill_type, fill_size),
                        document_type,
                        contract,
                    }) => {
                        let eligible_identities = choose_capable_identities(
                            current_identities,
                            contract,
                            mempool_document_counter,
                            count as usize,
                            rng,
                        );

                        // TO-DO: these documents should be created according to the data contract's validation rules
                        let mut documents_with_identity_and_entropy: Vec<(
                            Document,
                            &Identity,
                            Bytes32,
                        )> = Vec::new();
                        match document_type.random_documents_with_params(
                            count as u32,
                            &eligible_identities,
                            Some(block_info.time_ms),
                            Some(block_info.height),
                            Some(block_info.core_height),
                            *fill_type,
                            *fill_size,
                            rng,
                            platform_version,
                        ) {
                            Ok(documents) => documents_with_identity_and_entropy = documents,
                            Err(e) => tracing::warn!("Failed to create random documents: {}", e),
                        }

                        documents_with_identity_and_entropy.into_iter().for_each(
                            |(document, identity, entropy)| {
                                let identity_contract_nonce =
                                    if contract.owner_id() == identity.id() {
                                        contract_nonce_counter
                                            .entry((identity.id(), contract.id()))
                                            .or_insert(1)
                                    } else {
                                        contract_nonce_counter
                                            .entry((identity.id(), contract.id()))
                                            .or_default()
                                    };

                                let gap = self
                                    .identity_contract_nonce_gaps
                                    .as_ref()
                                    .map_or(0, |gap_amount| gap_amount.events_if_hit(rng))
                                    as u64;
                                *identity_contract_nonce += 1 + gap;

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
                                        prefunded_voting_balance: Default::default(),
                                    }
                                    .into();

                                let document_batch_transition: BatchTransition =
                                    BatchTransitionV0 {
                                        owner_id: identity.id(),
                                        transitions: vec![document_create_transition.into()],
                                        user_fee_increase: 0,
                                        signature_public_key_id: 2,
                                        signature: BinaryData::default(),
                                    }
                                    .into();
                                let mut document_batch_transition: StateTransition =
                                    document_batch_transition.into();

                                let identity_public_key = identity
                                    .get_first_public_key_matching(
                                        Purpose::AUTHENTICATION,
                                        HashSet::from([SecurityLevel::CRITICAL]),
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
                                            Ok(SecurityLevel::CRITICAL)
                                        }),
                                    )
                                    .expect("expected to sign");

                                operations.push(document_batch_transition);
                            },
                        );
                    }

                    // Generate state transition for specific document insert operation
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
                            let eligible_identities = choose_capable_identities(
                                current_identities,
                                contract,
                                mempool_document_counter,
                                count as usize,
                                rng,
                            );

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
                                        prefunded_voting_balance: Default::default(),
                                    }
                                    .into();

                                let document_batch_transition: BatchTransition =
                                    BatchTransitionV0 {
                                        owner_id: identity.id(),
                                        transitions: vec![document_create_transition.into()],
                                        user_fee_increase: 0,
                                        signature_public_key_id: 1,
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

                    // Generate state transition for document delete operation
                    OperationType::Document(DocumentOp {
                        action: DocumentAction::DocumentActionDelete,
                        document_type,
                        contract,
                    }) => {
                        let mut items = document_query_callback(
                            LocalDocumentQuery::RandomDocumentQuery(RandomDocumentQuery {
                                data_contract: contract,
                                document_type,
                            }),
                        );

                        items.retain(|item| !deleted.contains(&item.id()));

                        items.retain(|item| !replaced.contains(&item.id()));

                        items.retain(|item| !transferred.contains(&item.id()));

                        if !items.is_empty() {
                            let document = items.remove(0);

                            deleted.push(document.id());

                            //todo: fix this into a search key request for the following
                            //let search_key_request = BTreeMap::from([(Purpose::AUTHENTICATION as u8, BTreeMap::from([(SecurityLevel::HIGH as u8, AllKeysOfKindRequest)]))]);

                            let identity_id = document.owner_id();
                            let identity = current_identities
                                .iter()
                                .find(|&identity| identity.id() == identity_id)
                                .expect("Expected to find the identity in current_identities");
                            let identity_contract_nonce = contract_nonce_counter
                                .entry((identity.id(), contract.id()))
                                .or_default();
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
                                owner_id: identity.id(),
                                transitions: vec![document_delete_transition.into()],
                                user_fee_increase: 0,
                                signature_public_key_id: 1,
                                signature: BinaryData::default(),
                            }
                            .into();

                            let mut document_batch_transition: StateTransition =
                                document_batch_transition.into();

                            let identity_public_key = identity
                                .get_first_public_key_matching(
                                    Purpose::AUTHENTICATION,
                                    HashSet::from([SecurityLevel::CRITICAL]),
                                    HashSet::from([KeyType::ECDSA_SECP256K1, KeyType::BLS12_381]),
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
                        }
                    }

                    // Generate state transition for document replace operation
                    OperationType::Document(DocumentOp {
                        action: DocumentAction::DocumentActionReplaceRandom,
                        document_type,
                        contract,
                    }) => {
                        let mut items = document_query_callback(
                            LocalDocumentQuery::RandomDocumentQuery(RandomDocumentQuery {
                                data_contract: contract,
                                document_type,
                            }),
                        );

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
                            let identity =
                                identity_fetch_callback(request.identity_id.into(), Some(request));
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
                                signature_public_key_id: 1,
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

                    // Generate state transition for document transfer operation
                    OperationType::Document(DocumentOp {
                        action: DocumentAction::DocumentActionTransferRandom,
                        document_type,
                        contract,
                    }) => {
                        let mut items = document_query_callback(
                            LocalDocumentQuery::RandomDocumentQuery(RandomDocumentQuery {
                                data_contract: contract,
                                document_type,
                            }),
                        );

                        items.retain(|item| !deleted.contains(&item.id()));

                        items.retain(|item| !replaced.contains(&item.id()));

                        items.retain(|item| !transferred.contains(&item.id()));

                        if !items.is_empty() {
                            let document = items.remove(0);

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

                            transferred.push(document.id());

                            let request = IdentityKeysRequest {
                                identity_id: document.owner_id().to_buffer(),
                                request_type: KeyRequestType::SpecificKeys(vec![1]),
                                limit: Some(1),
                                offset: None,
                            };
                            let identity =
                                identity_fetch_callback(request.identity_id.into(), Some(request));
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
                                signature_public_key_id: 1,
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

                    // Generate state transition for identity top-up operation
                    OperationType::IdentityTopUp(_amount_range)
                        if !current_identities.is_empty() =>
                    {
                        // todo: use amount ranges
                        // Use a cyclic iterator over the identities to ensure we can create 'count' transitions
                        let cyclic_identities = current_identities.iter().cycle();

                        // Iterate 'count' times to create the required number of state transitions.
                        for random_identity in cyclic_identities.take(count.into()) {
                            match crate::transitions::create_identity_top_up_transition(
                                random_identity,
                                asset_lock_proofs,
                                platform_version,
                            ) {
                                Ok(transition) => operations.push(transition),
                                Err(_) => {
                                    tracing::error!("Error creating asset lock proof for identity top up transition");
                                    continue;
                                }
                            }
                        }
                    }
                    // Generate state transition for identity top-up from addresses operation
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
                            let Some(inputs) = current_addresses_with_balance
                                .take_random_amounts_with_range(amount_range, rng)
                            else {
                                // no funds left
                                break;
                            };

                            let state_transition = IdentityTopUpFromAddressesTransitionV0::try_from_inputs_with_signer(
                                random_identity,
                                inputs,
                                signer,
                                0, // user_fee_increase
                                platform_version,
                                None,
                            )
                            .expect("expected to create top up from addresses transition");
                            operations.push(state_transition);
                        }
                    }

                    OperationType::IdentityUpdate(update_op) if !current_identities.is_empty() => {
                        match update_op {
                            IdentityUpdateOp::IdentityUpdateAddKeys(keys_count) => {
                                // Map the count of keys already added this block to each identity
                                // This prevents adding duplicate KeyIDs in the same block
                                let mut keys_already_added_count_map = HashMap::new();
                                for id in &*current_identities {
                                    keys_already_added_count_map.insert(id.id(), 0);
                                }

                                // Create `count` state transitions
                                for _ in 0..count {
                                    let identities_count = current_identities.len();
                                    if identities_count == 0 {
                                        break;
                                    }

                                    // Select a random identity from the current_identities
                                    let random_index = rng.gen_range(0..identities_count);
                                    let random_identity = &mut current_identities[random_index];

                                    // Get keys already added
                                    let keys_already_added = keys_already_added_count_map.get(&random_identity.id())
                                        .expect("Expected to get keys_already_added in IdentityAddKeys ST creation");

                                    // Create transition
                                    let (state_transition, keys_to_add_at_end_block) = crate::transitions::create_identity_update_transition_add_keys(
                                            random_identity,
                                            *keys_count,
                                            *keys_already_added,
                                            identity_nonce_counter,
                                            signer,
                                            rng,
                                            platform_version,
                                        );

                                    // Push to operations vectors
                                    operations.push(state_transition);
                                    finalize_block_operations.push(IdentityAddKeys(
                                        keys_to_add_at_end_block.0,
                                        keys_to_add_at_end_block.1,
                                    ));

                                    // Increment keys_already_added count
                                    keys_already_added_count_map.insert(
                                        random_identity.id(),
                                        keys_already_added + *keys_count as u32,
                                    );
                                }
                            }
                            IdentityUpdateOp::IdentityUpdateDisableKey(keys_count) => {
                                (0..count).for_each(|_| {
                                    current_identities.iter_mut().enumerate().for_each(|(i, random_identity)| {
                                        if i >= count.into() { return; }

                                        if let Some(state_transition) =
                                            crate::transitions::create_identity_update_transition_disable_keys(
                                                random_identity,
                                                *keys_count,
                                                identity_nonce_counter,
                                                block_info.time_ms,
                                                signer,
                                                rng,
                                                platform_version,
                                            ) {
                                                operations.push(state_transition);
                                        }
                                    });
                                });
                            }
                        }
                    }

                    // Generate state transition for identity withdrawal operation
                    OperationType::IdentityWithdrawal(amount_range)
                        if !current_identities.is_empty() =>
                    {
                        for i in 0..count {
                            let index = (i as usize) % current_identities.len();
                            let random_identity = &mut current_identities[index];
                            let state_transition =
                                crate::transitions::create_identity_withdrawal_transition(
                                    random_identity,
                                    amount_range.clone(),
                                    identity_nonce_counter,
                                    signer,
                                    rng,
                                );
                            operations.push(state_transition);
                        }
                    }

                    // Generate state transition for identity transfer operation
                    OperationType::IdentityTransfer(identity_transfer_info) => {
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

                    OperationType::ContractCreate(params, doc_type_range)
                        if !current_identities.is_empty() =>
                    {
                        let contract_factory = match DataContractFactory::new(
                            platform_version.protocol_version,
                        ) {
                            Ok(contract_factory) => contract_factory,
                            Err(e) => {
                                tracing::error!("Failed to get DataContractFactory while creating random contract: {e}");
                                continue;
                            }
                        };

                        // Create `count` ContractCreate transitions and push to operations vec
                        for _ in 0..count {
                            // Get the contract owner_id from a random current_identity and identity nonce
                            let identity = current_identities
                                .choose(rng)
                                .expect("Expected to get an identity for ContractCreate");
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
                            let doc_types =
                                Value::Map(
                                    doc_type_range
                                        .clone()
                                        .filter_map(|_| match DocumentTypeV0::random_document_type(
                                            params.clone(),
                                            contract_id,
                                            rng,
                                            platform_version,
                                        ) {
                                            Ok(new_document_type) => {
                                                let doc_type_clone =
                                                    new_document_type.schema().clone();
                                                Some((
                                                    Value::Text(new_document_type.name().clone()),
                                                    doc_type_clone,
                                                ))
                                            }
                                            Err(e) => {
                                                tracing::error!(
                                                    "Error generating random document type: {:?}",
                                                    e
                                                );
                                                None
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
                                    tracing::error!("Failed to create random data contract: {e}");
                                    continue;
                                }
                            };

                            let transition = match contract_factory
                                .create_data_contract_create_transition(created_data_contract)
                            {
                                Ok(transition) => transition,
                                Err(e) => {
                                    tracing::error!(
                                        "Failed to create ContractCreate transition: {e}"
                                    );
                                    continue;
                                }
                            };

                            // Sign transition
                            let public_key = identity
                                .get_first_public_key_matching(
                                    Purpose::AUTHENTICATION,
                                    HashSet::from([SecurityLevel::HIGH, SecurityLevel::CRITICAL]),
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
                                tracing::error!("Error signing state transition: {:?}", e);
                            }

                            operations.push(state_transition);
                        }
                    }
                    OperationType::ContractUpdate(DataContractUpdateOp {
                        action: DataContractUpdateAction::DataContractNewDocumentTypes(params),
                        contract,
                        document_type: None,
                    }) if !current_identities.is_empty() => {
                        let contract_key = contract.id().to_string(Encoding::Base58);

                        for _ in 0..count {
                            if let Some(DataContract::V0(contract_ref)) =
                                known_contracts.get_mut(&contract_key)
                            {
                                match DocumentTypeV0::random_document_type(
                                    params.clone(),
                                    contract_ref.id(),
                                    rng,
                                    platform_version,
                                ) {
                                    Ok(new_document_type) => {
                                        let document_type_name =
                                            format!("doc_type_{}", rng.gen::<u16>());

                                        // Update the document types and increment the version
                                        contract_ref.document_types.insert(
                                            document_type_name,
                                            DocumentType::V0(new_document_type),
                                        );
                                        contract_ref.increment_version();

                                        let identity = current_identities
                                            .iter()
                                            .find(|identity| {
                                                identity.id() == contract_ref.owner_id()
                                            })
                                            .or_else(|| current_identities.choose(rng)).expect("Expected to get an identity for DataContractUpdate");

                                        let identity_contract_nonce = contract_nonce_counter
                                            .entry((identity.id(), contract_ref.id()))
                                            .or_default();
                                        *identity_contract_nonce += 1;

                                        // Prepare the DataContractUpdateTransition with the updated contract_ref
                                        match DataContractUpdateTransition::try_from_platform_versioned((DataContract::V0(contract_ref.clone()), *identity_contract_nonce), platform_version) {
                                            Ok(data_contract_update_transition) => {
                                                let identity_public_key = identity
                                                    .get_first_public_key_matching(
                                                        Purpose::AUTHENTICATION,
                                                        HashSet::from([SecurityLevel::CRITICAL]),
                                                        HashSet::from([KeyType::ECDSA_SECP256K1, KeyType::BLS12_381]),
                                                        false
                                                    )
                                                    .expect("expected to get a signing key with CRITICAL security level");

                                                let mut state_transition = StateTransition::DataContractUpdate(data_contract_update_transition);
                                                state_transition.sign_external(
                                                    identity_public_key,
                                                    signer,
                                                    Some(|_data_contract_id, _document_type_name| {
                                                        Ok(SecurityLevel::CRITICAL)
                                                    }),
                                                )
                                                .expect("expected to sign the contract update transition with a CRITICAL level key");

                                                operations.push(state_transition);
                                            },
                                            Err(e) => tracing::error!("Error converting data contract to update transition: {:?}", e),
                                        }
                                    }
                                    Err(e) => {
                                        tracing::error!(
                                            "Error generating random document type: {:?}",
                                            e
                                        )
                                    }
                                }
                            } else {
                                tracing::error!("Contract not found in known_contracts");
                            }
                        }
                    }
                    // Generate state transition for identity transfer to addresses operation
                    OperationType::IdentityTransferToAddresses(
                        amount_range,
                        output_count_range,
                        use_existing_outputs_chance,
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

                                let (state_transition, _recipient_addresses) =
                                    crate::transitions::create_identity_credit_transfer_to_addresses_transition(
                                        sender,
                                        identity_nonce_counter,
                                        current_addresses_with_balance,
                                        signer,
                                        transfer_info.amount,
                                        transfer_info.outputs.len(),
                                        rng,
                                        platform_version,
                                    );
                                operations.push(state_transition);
                            } else {
                                // Handle the case where no specific sender is provided
                                let identities_count = current_identities.len();
                                if identities_count == 0 {
                                    break;
                                }

                                // Select a random identity from the current_identities for the sender
                                let random_index = rng.gen_range(0..identities_count);
                                let sender = &current_identities[random_index];

                                // Get random amount within range
                                let amount = rng.gen_range(amount_range.clone());

                                // Get random output count within range
                                let output_count =
                                    rng.gen_range(output_count_range.clone()).max(1) as usize;

                                // Calculate amount per output
                                let amount_per_output = amount / output_count as u64;
                                let mut recipient_addresses = BTreeMap::new();

                                // Collect existing addresses that could be reused
                                let mut available_existing_addresses: Vec<_> =
                                    current_addresses_with_balance
                                        .addresses_with_balance
                                        .keys()
                                        .cloned()
                                        .collect();

                                for _ in 0..output_count {
                                    // Check if we should use an existing address as output
                                    let use_existing = use_existing_outputs_chance
                                        .map(|chance| {
                                            rng.gen::<f64>() < chance
                                                && !available_existing_addresses.is_empty()
                                        })
                                        .unwrap_or(false);

                                    let address = if use_existing {
                                        // Pick a random existing address and remove it from available pool
                                        let idx =
                                            rng.gen_range(0..available_existing_addresses.len());
                                        let existing_address =
                                            available_existing_addresses.swap_remove(idx);
                                        // Update the balance for this existing address
                                        if let Some((nonce, balance)) =
                                            current_addresses_with_balance
                                                .addresses_with_balance
                                                .get(&existing_address)
                                        {
                                            current_addresses_with_balance
                                                .addresses_in_block_with_new_balance
                                                .insert(
                                                    existing_address,
                                                    (*nonce, balance + amount_per_output),
                                                );
                                        }
                                        existing_address
                                    } else {
                                        // Create a new address
                                        let new_address = signer.add_random_address_key(rng);
                                        current_addresses_with_balance
                                            .register_new_address(new_address, amount_per_output);
                                        new_address
                                    };

                                    recipient_addresses.insert(address, amount_per_output);
                                }

                                let state_transition =
                                    crate::transitions::create_identity_credit_transfer_to_addresses_transition_with_outputs(
                                        sender,
                                        identity_nonce_counter,
                                        signer,
                                        recipient_addresses,
                                        platform_version,
                                    );
                                operations.push(state_transition);
                            }
                        }
                    }

                    // Generate state transition for identity create from addresses operation
                    OperationType::IdentityCreateFromAddresses(
                        amount_range,
                        maybe_output_amount,
                        fee_strategy,
                        key_count,
                        extra_keys,
                    ) => {
                        for _i in 0..count {
                            let Some(inputs) = current_addresses_with_balance
                                .take_random_amounts_with_range(amount_range, rng)
                            else {
                                // no funds left
                                break;
                            };

                            // Create a new identity with random keys
                            let (mut identity, keys) = Identity::random_identity_with_main_keys_with_private_key::<
                                Vec<_>,
                            >(*key_count, rng, platform_version)
                            .expect("Expected to create identity with keys");

                            // Add extra keys to the identity
                            for (purpose, security_to_key_type_map) in extra_keys.iter() {
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

                            // Calculate estimated fee for local balance tracking
                            // Fee = identity_create_base_cost + identity_key_in_creation_cost * num_keys
                            //     + address_funds_transfer_input_cost * num_inputs
                            //     + address_funds_transfer_output_cost * num_outputs
                            let total_key_count = identity.public_keys().len() as u64;
                            let num_inputs = inputs.len() as u64;
                            let has_output = maybe_output_amount.is_some();
                            let num_outputs: u64 = if has_output { 1 } else { 0 };

                            let min_fees = &platform_version.fee_version.state_transition_min_fees;
                            let estimated_fee = min_fees
                                .identity_create_base_cost
                                .saturating_add(
                                    min_fees
                                        .identity_key_in_creation_cost
                                        .saturating_mul(total_key_count),
                                )
                                .saturating_add(
                                    min_fees
                                        .address_funds_transfer_input_cost
                                        .saturating_mul(num_inputs),
                                )
                                .saturating_add(
                                    min_fees
                                        .address_funds_transfer_output_cost
                                        .saturating_mul(num_outputs),
                                );

                            // Track fee deductions for balance tracking
                            let mut output_fee_deduction: Credits = 0;
                            let mut input_fee_deductions: BTreeMap<usize, Credits> =
                                BTreeMap::new();

                            let input_addresses_in_order: Vec<PlatformAddress> =
                                inputs.keys().cloned().collect();

                            let mut remaining_fee = estimated_fee;
                            for step in &fee_strategy {
                                if remaining_fee == 0 {
                                    break;
                                }
                                match step {
                                    AddressFundsFeeStrategyStep::ReduceOutput(index) => {
                                        if *index == 0 && has_output {
                                            output_fee_deduction =
                                                output_fee_deduction.saturating_add(remaining_fee);
                                            remaining_fee = 0;
                                        }
                                    }
                                    AddressFundsFeeStrategyStep::DeductFromInput(index) => {
                                        if (*index as usize) < inputs.len() {
                                            let entry = input_fee_deductions
                                                .entry(*index as usize)
                                                .or_insert(0);
                                            *entry = entry.saturating_add(remaining_fee);
                                            remaining_fee = 0;
                                        }
                                    }
                                }
                            }

                            // Apply fee deductions to input balances (beyond the amount already staged)
                            for (idx, fee_deduction) in input_fee_deductions {
                                if let Some(address) = input_addresses_in_order.get(idx) {
                                    // Get the current staged balance and reduce it further by the fee
                                    if let Some((_, staged_balance)) =
                                        current_addresses_with_balance
                                            .addresses_in_block_with_new_balance
                                            .get_mut(address)
                                    {
                                        *staged_balance =
                                            staged_balance.saturating_sub(fee_deduction);
                                    }
                                }
                            }

                            // Create output if maybe_output_amount is provided
                            let output = maybe_output_amount.as_ref().map(|output_range| {
                                let output_amount = rng.gen_range(output_range.clone());
                                let output_address = signer.add_random_address_key(rng);
                                // Register the output address with balance minus fee deduction
                                let actual_output_amount =
                                    output_amount.saturating_sub(output_fee_deduction);
                                current_addresses_with_balance
                                    .register_new_address(output_address, actual_output_amount);
                                (output_address, output_amount)
                            });

                            let state_transition = IdentityCreateFromAddressesTransitionV0::try_from_inputs_with_signer(
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

                            operations.push(state_transition);
                            // Add the newly created identity to the pool
                            current_identities.push(identity);
                        }
                    }

                    // Generate state transition for address funding from core asset lock operation
                    // Note: amount_range is not used here because the actual funded amount comes from the asset lock proof
                    OperationType::AddressFundingFromCoreAssetLock(_amount_range) => {
                        for _i in 0..count {
                            // Get an asset lock proof from the pool
                            let Some((asset_lock_proof, private_key)) = asset_lock_proofs.pop()
                            else {
                                tracing::warn!(
                                    "No asset lock proofs available for AddressFundingFromCoreAssetLock"
                                );
                                break;
                            };

                            // Get the funded amount from the asset lock proof
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

                            // Calculate estimated fee for local balance tracking
                            // For address funding from asset lock with no platform inputs and one output,
                            // the fee is just the output cost. With ReduceOutput(0) strategy,
                            // the fee is deducted from the output.
                            let min_fees = &platform_version.fee_version.state_transition_min_fees;
                            let estimated_fee = min_fees.address_funds_transfer_output_cost; // 1 output

                            // Check if we have enough funds to cover the fee
                            if funded_amount <= estimated_fee {
                                tracing::warn!(
                                    "Asset lock amount {} is too small to cover fee {}",
                                    funded_amount,
                                    estimated_fee
                                );
                                continue;
                            }

                            // Create a new address for the funding
                            // Register with the actual amount after fee deduction (ReduceOutput(0))
                            let address = signer.add_random_address_key(rng);
                            let actual_funded_amount = funded_amount.saturating_sub(estimated_fee);
                            current_addresses_with_balance
                                .register_new_address(address, actual_funded_amount);

                            let mut outputs = BTreeMap::new();
                            outputs.insert(address, None);

                            let funding_transition =
                                AddressFundingFromAssetLockTransitionV0::try_from_asset_lock_with_signer(
                                    asset_lock_proof,
                                    private_key.inner.secret_bytes().as_slice(),
                                    BTreeMap::new(), // no additional inputs
                                    outputs,
                                    vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
                                    signer,
                                    0,
                                    platform_version,
                                );

                            match funding_transition {
                                Ok(state_transition) => operations.push(state_transition),
                                Err(e) => {
                                    tracing::error!(
                                        "Error creating address funding transition: {:?}",
                                        e
                                    );
                                    continue;
                                }
                            }
                        }
                    }

                    // Generate state transition for address transfer operation
                    OperationType::AddressTransfer(
                        amount_range,
                        output_count_range,
                        use_existing_outputs_chance,
                        fee_strategy,
                    ) => {
                        for _i in 0..count {
                            let Some(inputs) = current_addresses_with_balance
                                .take_random_amounts_with_range(amount_range, rng)
                            else {
                                eprintln!("no funds left on block {}", block_info.height);
                                // no funds left
                                break;
                            };

                            let fee_strategy = fee_strategy
                                .clone()
                                .unwrap_or(vec![AddressFundsFeeStrategyStep::ReduceOutput(0)]);

                            // Calculate total input amount (we'll distribute this among outputs)
                            let total_input: Credits =
                                inputs.values().map(|(_, credits)| credits).sum();

                            // Generate random number of outputs within the specified range
                            let output_count =
                                rng.gen_range(output_count_range.clone()).max(1) as usize;

                            // Calculate estimated fee for local balance tracking
                            // The transition must have inputs == outputs (balanced), but the chain
                            // will deduct fees according to fee_strategy. We need to track the
                            // actual credited amounts locally.
                            let min_fees = &platform_version.fee_version.state_transition_min_fees;
                            let estimated_fee = min_fees
                                .address_funds_transfer_input_cost
                                .saturating_mul(inputs.len() as u64)
                                .saturating_add(
                                    min_fees
                                        .address_funds_transfer_output_cost
                                        .saturating_mul(output_count as u64),
                                );

                            // Check if we have enough funds to cover the fee
                            // The check depends on the fee_strategy:
                            // - DeductFromInput(i): input[i]'s REMAINING balance must cover the fee
                            // - ReduceOutput(i): output[i]'s amount must cover the fee
                            //
                            // At this point, inputs have been staged with their remaining balances
                            // (original - transfer_amount). We need to verify fee can be covered.

                            let input_addresses_for_check: Vec<PlatformAddress> =
                                inputs.keys().cloned().collect();

                            let mut fee_can_be_covered = false;
                            let mut remaining_fee_to_check = estimated_fee;

                            for step in &fee_strategy {
                                if remaining_fee_to_check == 0 {
                                    fee_can_be_covered = true;
                                    break;
                                }
                                match step {
                                    AddressFundsFeeStrategyStep::ReduceOutput(_index) => {
                                        // For ReduceOutput, the output amount must cover the fee
                                        // Since we're distributing total_input evenly, check if
                                        // the output amount is enough
                                        let output_amount =
                                            total_input / output_count.max(1) as Credits;
                                        if output_amount >= remaining_fee_to_check {
                                            remaining_fee_to_check = 0;
                                            fee_can_be_covered = true;
                                        } else {
                                            remaining_fee_to_check -= output_amount;
                                        }
                                    }
                                    AddressFundsFeeStrategyStep::DeductFromInput(index) => {
                                        // For DeductFromInput, the input's REMAINING balance
                                        // (after transfer deduction) must cover the fee
                                        if let Some(input_addr) =
                                            input_addresses_for_check.get(*index as usize)
                                        {
                                            // Get the staged remaining balance
                                            if let Some((_, remaining_balance)) =
                                                current_addresses_with_balance
                                                    .addresses_in_block_with_new_balance
                                                    .get(input_addr)
                                            {
                                                if *remaining_balance >= remaining_fee_to_check {
                                                    remaining_fee_to_check = 0;
                                                    fee_can_be_covered = true;
                                                } else {
                                                    remaining_fee_to_check -= *remaining_balance;
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            if !fee_can_be_covered {
                                tracing::warn!(
                                    total_input = total_input,
                                    estimated_fee = estimated_fee,
                                    "AddressTransfer: insufficient remaining balance for fees, skipping"
                                );
                                // Rollback the staged changes from take_random_amounts_with_range
                                for addr in input_addresses_for_check {
                                    current_addresses_with_balance
                                        .addresses_in_block_with_new_balance
                                        .remove(&addr);
                                }
                                continue;
                            }

                            // Create output addresses and distribute funds evenly
                            // Account for remainder from integer division
                            let amount_per_output = total_input / output_count as Credits;
                            let remainder = total_input % output_count as Credits;
                            let mut outputs = BTreeMap::new();

                            // Collect existing addresses that are not used as inputs (for potential reuse as outputs)
                            let input_addresses: HashSet<_> = inputs.keys().cloned().collect();
                            let mut available_existing_addresses: Vec<_> =
                                current_addresses_with_balance
                                    .addresses_with_balance
                                    .keys()
                                    .filter(|addr| !input_addresses.contains(*addr))
                                    .cloned()
                                    .collect();

                            // Track output addresses in order for fee deduction
                            let mut output_addresses_in_order: Vec<PlatformAddress> = Vec::new();
                            // Track which addresses are existing vs new for balance updates
                            let mut existing_output_addresses: HashSet<PlatformAddress> =
                                HashSet::new();

                            for (outputs_created, _) in (0..output_count).enumerate() {
                                // First output gets the remainder to ensure exact balance
                                let this_output_amount = if outputs_created == 0 {
                                    amount_per_output + remainder
                                } else {
                                    amount_per_output
                                };

                                // Check if we should use an existing address as output
                                let use_existing = use_existing_outputs_chance
                                    .map(|chance| {
                                        rng.gen::<f64>() < chance
                                            && !available_existing_addresses.is_empty()
                                    })
                                    .unwrap_or(false);

                                let address = if use_existing {
                                    // Pick a random existing address and remove it from available pool
                                    let idx = rng.gen_range(0..available_existing_addresses.len());
                                    let existing_address =
                                        available_existing_addresses.swap_remove(idx);
                                    existing_output_addresses.insert(existing_address);
                                    existing_address
                                } else {
                                    // Create a new address
                                    signer.add_random_address_key(rng)
                                };

                                outputs.insert(address, this_output_amount);
                                output_addresses_in_order.push(address);
                            }

                            // Calculate fee deductions based on fee_strategy
                            // Track deductions for both outputs (ReduceOutput) and inputs (DeductFromInput)
                            let mut output_fee_deductions: BTreeMap<usize, Credits> =
                                BTreeMap::new();
                            let mut input_fee_deductions: BTreeMap<usize, Credits> =
                                BTreeMap::new();
                            let mut remaining_fee = estimated_fee;

                            // Get input addresses in order for DeductFromInput matching
                            let input_addresses_in_order: Vec<PlatformAddress> =
                                inputs.keys().cloned().collect();

                            for step in &fee_strategy {
                                if remaining_fee == 0 {
                                    break;
                                }
                                match step {
                                    AddressFundsFeeStrategyStep::ReduceOutput(index) => {
                                        let idx = *index as usize;
                                        if idx < output_addresses_in_order.len() {
                                            let output_addr = &output_addresses_in_order[idx];
                                            let output_amount =
                                                outputs.get(output_addr).copied().unwrap_or(0);
                                            let deduction = remaining_fee.min(output_amount);
                                            *output_fee_deductions.entry(idx).or_insert(0) +=
                                                deduction;
                                            remaining_fee = remaining_fee.saturating_sub(deduction);
                                        }
                                    }
                                    AddressFundsFeeStrategyStep::DeductFromInput(index) => {
                                        let idx = *index as usize;
                                        if idx < input_addresses_in_order.len() {
                                            // The fee is deducted from the input on-chain
                                            // We need to track this additional deduction
                                            let deduction = remaining_fee;
                                            *input_fee_deductions.entry(idx).or_insert(0) +=
                                                deduction;
                                            remaining_fee = 0;
                                        }
                                    }
                                }
                            }

                            // Apply fee deductions to INPUT balance tracking
                            // The transfer amount was already deducted by take_random_amounts_with_range,
                            // but the fee is ALSO deducted from inputs when using DeductFromInput
                            for (idx, fee_deduction) in input_fee_deductions {
                                if let Some(input_addr) = input_addresses_in_order.get(idx) {
                                    if let Some((nonce, current_balance)) =
                                        current_addresses_with_balance
                                            .addresses_in_block_with_new_balance
                                            .get(input_addr)
                                    {
                                        // Further reduce the input's balance by the fee
                                        let adjusted_balance =
                                            current_balance.saturating_sub(fee_deduction);
                                        current_addresses_with_balance
                                            .addresses_in_block_with_new_balance
                                            .insert(*input_addr, (*nonce, adjusted_balance));
                                    }
                                }
                            }

                            // Apply fee deductions to OUTPUT balance tracking
                            for (idx, address) in output_addresses_in_order.iter().enumerate() {
                                let transition_amount = outputs.get(address).copied().unwrap_or(0);
                                let fee_deduction =
                                    output_fee_deductions.get(&idx).copied().unwrap_or(0);
                                let actual_credited_amount =
                                    transition_amount.saturating_sub(fee_deduction);

                                if existing_output_addresses.contains(address) {
                                    // Update balance for existing address
                                    if let Some((nonce, balance)) = current_addresses_with_balance
                                        .addresses_with_balance
                                        .get(address)
                                    {
                                        current_addresses_with_balance
                                            .addresses_in_block_with_new_balance
                                            .insert(
                                                *address,
                                                (*nonce, balance + actual_credited_amount),
                                            );
                                    }
                                } else {
                                    // New address - track with fee-adjusted amount
                                    current_addresses_with_balance
                                        .addresses_in_block_with_new_balance
                                        .insert(*address, (0, actual_credited_amount));
                                }
                            }

                            // Verify input/output balance before creating transition
                            let actual_input_sum: Credits = inputs.values().map(|(_, c)| c).sum();
                            let actual_output_sum: Credits = outputs.values().sum();
                            if actual_input_sum != actual_output_sum {
                                tracing::error!(
                                    "Balance mismatch BEFORE transition creation: input_sum={}, output_sum={}, diff={}, output_count={}, outputs.len()={}",
                                    actual_input_sum,
                                    actual_output_sum,
                                    actual_input_sum as i128 - actual_output_sum as i128,
                                    output_count,
                                    outputs.len()
                                );
                            }

                            // Log the inputs for debugging nonce issues
                            for (addr, (nonce, amount)) in &inputs {
                                tracing::debug!(
                                    address = %addr,
                                    nonce = nonce,
                                    amount = amount,
                                    "AddressTransfer: creating transition with input"
                                );
                            }

                            let transfer_transition =
                                AddressFundsTransferTransition::try_from_inputs_with_signer(
                                    inputs,
                                    outputs,
                                    fee_strategy,
                                    signer,
                                    0,
                                    platform_version,
                                )
                                .expect("expected to create address funds transfer transition");

                            operations.push(transfer_transition);
                        }
                    }

                    // Generate state transition for address withdrawal operation
                    OperationType::AddressWithdrawal(
                        amount_range,
                        maybe_output_range,
                        fee_strategy,
                    ) => {
                        for _i in 0..count {
                            let Some(inputs) = current_addresses_with_balance
                                .take_random_amounts_with_range(amount_range, rng)
                            else {
                                // no funds left
                                break;
                            };

                            let fee_strategy = fee_strategy
                                .clone()
                                .unwrap_or(vec![AddressFundsFeeStrategyStep::DeductFromInput(0)]);

                            // Calculate estimated fee for local balance tracking
                            // Fee = address_credit_withdrawal + input_cost * num_inputs + output_cost * num_outputs
                            let num_inputs = inputs.len() as u64;
                            let has_output = maybe_output_range.is_some();
                            let num_outputs: u64 = if has_output { 1 } else { 0 };

                            let min_fees = &platform_version.fee_version.state_transition_min_fees;
                            let estimated_fee = min_fees
                                .address_credit_withdrawal
                                .saturating_add(
                                    min_fees
                                        .address_funds_transfer_input_cost
                                        .saturating_mul(num_inputs),
                                )
                                .saturating_add(
                                    min_fees
                                        .address_funds_transfer_output_cost
                                        .saturating_mul(num_outputs),
                                );

                            // Track fee deductions for balance tracking
                            let mut output_fee_deduction: Credits = 0;
                            let mut input_fee_deductions: BTreeMap<usize, Credits> =
                                BTreeMap::new();

                            let input_addresses_in_order: Vec<PlatformAddress> =
                                inputs.keys().cloned().collect();

                            let mut remaining_fee = estimated_fee;
                            for step in &fee_strategy {
                                if remaining_fee == 0 {
                                    break;
                                }
                                match step {
                                    AddressFundsFeeStrategyStep::ReduceOutput(index) => {
                                        if *index == 0 && has_output {
                                            output_fee_deduction =
                                                output_fee_deduction.saturating_add(remaining_fee);
                                            remaining_fee = 0;
                                        }
                                    }
                                    AddressFundsFeeStrategyStep::DeductFromInput(index) => {
                                        if (*index as usize) < inputs.len() {
                                            let entry = input_fee_deductions
                                                .entry(*index as usize)
                                                .or_insert(0);
                                            *entry = entry.saturating_add(remaining_fee);
                                            remaining_fee = 0;
                                        }
                                    }
                                }
                            }

                            // Apply fee deductions to input balances (beyond the amount already staged)
                            for (idx, fee_deduction) in input_fee_deductions {
                                if let Some(address) = input_addresses_in_order.get(idx) {
                                    // Get the current staged balance and reduce it further by the fee
                                    if let Some((_, staged_balance)) =
                                        current_addresses_with_balance
                                            .addresses_in_block_with_new_balance
                                            .get_mut(address)
                                    {
                                        *staged_balance =
                                            staged_balance.saturating_sub(fee_deduction);
                                    }
                                }
                            }

                            // Determine if we have an output (change address) and its amount
                            let output = if let Some(output_amount_range) = maybe_output_range {
                                let output_amount = rng.gen_range(output_amount_range.clone());
                                let output_address = signer.add_random_address_key(rng);
                                // Register with actual amount after fee deduction
                                let actual_output_amount =
                                    output_amount.saturating_sub(output_fee_deduction);
                                current_addresses_with_balance
                                    .addresses_in_block_with_new_balance
                                    .insert(output_address, (0, actual_output_amount));
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

                            let withdrawal_transition =
                                AddressCreditWithdrawalTransition::try_from_inputs_with_signer(
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

                            operations.push(withdrawal_transition);
                        }
                    }

                    _ => {}
                }
            }
        }
        (operations, finalize_block_operations)
    }

    /// Generates identity-related state transitions for a specified block, considering new and existing identities.
    ///
    /// This function orchestrates the creation of state transitions associated with identities, leveraging
    /// the `start_identities` field to initialize identities at the strategy's start block, and the `identities_inserts`
    /// field to dynamically insert new identities based on a defined frequency. It is essential for simulating
    /// identity actions within the blockchain, such as identity creation, throughout the lifecycle of the strategy.
    ///
    /// The function intelligently handles the initial setup of identities at the beginning of the strategy and
    /// supports the continuous introduction of new identities into the simulation, reflecting a more realistic
    /// blockchain environment.
    ///
    /// # Parameters
    /// - `block_info`: Provides details about the current block, such as height, to guide the generation of state transitions.
    /// - `signer`: A mutable reference to a signer instance, used for signing the state transitions of identities.
    /// - `rng`: A mutable reference to a random number generator, for creating randomized elements where necessary.
    /// - `asset_lock_proofs`: A vector of asset lock proofs and associated private keys.
    /// - `config`: Configuration details of the strategy, including the start block height.
    /// - `platform_version`: Specifies the version of the Dash Platform, ensuring compatibility with its features and behaviors.
    ///
    /// # Returns
    /// A vector of tuples, each containing an `Identity` and its associated `StateTransition`, representing the actions taken by or on behalf of that identity within the block.
    ///
    /// # Examples
    /// ```ignore
    /// // Assuming `strategy` is an instance of `Strategy`, with `block_info`, `signer`, `rng`,
    /// // `asset_lock_proofs`, `config`, and `platform_version` properly initialized:
    /// let identity_transitions = strategy.identity_state_transitions_for_block(
    ///     &block_info,
    ///     &mut signer,
    ///     &mut rng,
    ///     &mut create_asset_lock,
    ///     &config,
    ///     &platform_version,
    /// ).expect("Expected to generate identity state transitions without error");
    /// ```
    ///
    /// # Notes
    /// This function plays a crucial role in simulating the dynamic nature of identity management on the Dash Platform,
    /// allowing for a nuanced and detailed representation of identity-related activities within a blockchain simulation environment.
    #[allow(clippy::too_many_arguments)]
    pub fn identity_state_transitions_for_block(
        &self,
        block_info: &BlockInfo,
        amount: u64,
        signer: &mut SimpleSigner,
        rng: &mut StdRng,
        asset_lock_proofs: &mut Vec<(AssetLockProof, PrivateKey)>,
        config: &StrategyConfig,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<(Identity, StateTransition)>, ProtocolError> {
        let mut state_transitions = vec![];

        // Add start_identities
        if block_info.height == config.start_block_height
            && self.start_identities.number_of_identities > 0
        {
            let mut new_transitions = crate::transitions::create_identities_state_transitions(
                self.start_identities.number_of_identities,
                self.start_identities.keys_per_identity.into(),
                &self.start_identities.extra_keys,
                amount,
                signer,
                rng,
                asset_lock_proofs,
                platform_version,
            )?;
            state_transitions.append(&mut new_transitions);
        }

        // Add identities_inserts
        // Don't do this on first two blocks (per design but also we need to skip utxo refresh)
        if block_info.height > config.start_block_height + 1 {
            let frequency = &self.identity_inserts.frequency;
            if frequency.check_hit(rng) {
                let count = frequency.events(rng); // number of identities to create

                let mut new_transitions = crate::transitions::create_identities_state_transitions(
                    count,
                    self.identity_inserts.start_keys as KeyID,
                    &self.identity_inserts.extra_keys,
                    200000, // 0.002 dash
                    signer,
                    rng,
                    asset_lock_proofs,
                    platform_version,
                )?;
                state_transitions.append(&mut new_transitions);
            }
        }

        Ok(state_transitions)
    }

    /// Initializes contracts and generates their creation state transitions based on the `contracts_with_updates` field.
    ///
    /// This function orchestrates the setup of initial data contracts specified in the strategy, applying any predefined updates
    /// based on the simulation's block height. It assigns ownership of these contracts to identities randomly selected from the
    /// current identities list, ensuring dynamic interaction within the simulated environment. Additionally, the function
    /// updates the ID of each contract to reflect the ownership and creation details, maintaining the integrity of contract
    /// relationships throughout the simulation.
    ///
    /// For contracts designated with updates, this process also prepares the contracts by adjusting their details to match
    /// the simulated block height, ensuring that updates are accurately reflected in the simulation. Operations related to
    /// these contracts are updated accordingly to maintain consistency.
    ///
    /// # Parameters
    /// - `current_identities`: A list of current identities available in the simulation.
    /// - `identity_nonce_counter`: Tracks nonce values for each identity to ensure unique contract identifiers.
    /// - `signer`: A reference to a signer instance for signing the contract creation transactions.
    /// - `rng`: A random number generator for selecting identities and generating contract details.
    /// - `platform_version`: Indicates the platform version to ensure compatibility with Dash Platform features.
    ///
    /// # Returns
    /// A vector of `StateTransition`, each representing the creation of a data contract within the simulated environment.
    ///
    /// # Examples
    /// ```ignore
    /// let initial_contract_transitions = strategy.initial_contract_state_transitions(
    ///     &current_identities,
    ///     &mut identity_nonce_counter,
    ///     &signer,
    ///     &mut rng,
    ///     &platform_version,
    /// );
    /// ```
    ///
    /// This function is pivotal for setting up the simulated environment's initial state, providing a foundation for
    /// subsequent operations and updates within the strategy.
    pub fn initial_contract_state_transitions(
        &mut self,
        current_identities: &[Identity],
        identity_nonce_counter: &mut BTreeMap<Identifier, u64>,
        signer: &SimpleSigner,
        rng: &mut StdRng,
        platform_version: &PlatformVersion,
    ) -> Vec<StateTransition> {
        let mut id_mapping = HashMap::new(); // Maps old IDs to new IDs

        self.start_contracts
            .iter_mut()
            .map(|(created_contract, contract_updates)| {
                // Select a random identity from current_identities to be the contract owner
                let identity_num = rng.gen_range(0..current_identities.len());

                let identity = current_identities
                    .get(identity_num)
                    .expect("Expected to find the identity in the current_identities");

                let identity_public_key = identity.get_first_public_key_matching(
                    Purpose::AUTHENTICATION,
                    HashSet::from([SecurityLevel::HIGH, SecurityLevel::CRITICAL]),
                    HashSet::from([KeyType::ECDSA_SECP256K1]),
                    false
                )
                .expect("Expected to get identity public key in initial_contract_state_transitions");
                let key_id = identity_public_key.id();

                let partial_identity = identity.clone().into_partial_identity_info();
                let partial_identity_public_key = partial_identity.loaded_public_keys.values()
                    .find(|&public_key| public_key.id() == key_id)
                    .expect("No public key with matching id found");

                let contract = created_contract.data_contract_mut();

                // Get and bump the identity nonce
                let identity_nonce = identity_nonce_counter.entry(partial_identity.id).or_default();
                *identity_nonce += 1;

                // Set the contract ID and owner ID with the random identity
                contract.set_owner_id(partial_identity.id);
                let old_id = contract.id();
                let new_id =
                    DataContract::generate_data_contract_id_v0(partial_identity.id, *identity_nonce);
                contract.set_id(new_id);

                id_mapping.insert(old_id, new_id); // Store the mapping

                // If there are contract updates, use the mapping to update their ID and owner ID too
                if let Some(contract_updates) = contract_updates {
                    for (_, updated_contract) in contract_updates.iter_mut() {
                        let updated_contract_data = updated_contract.data_contract_mut();
                        // Use the new ID from the mapping
                        if let Some(new_updated_id) = id_mapping.get(&updated_contract_data.id()) {
                            updated_contract_data.set_id(*new_updated_id);
                        }
                        updated_contract_data.set_owner_id(contract.owner_id());
                    }
                }

                // Update any document transitions that registered to the old contract id
                for op in self.operations.iter_mut() {
                    if let OperationType::Document(document_op) = &mut op.op_type {
                        if document_op.contract.id() == old_id {
                            document_op.contract = contract.clone();
                            let document_type = contract.document_type_cloned_for_name(document_op.document_type.name())
                                .expect("Expected to get a document type for name while creating initial strategy contracts");
                            document_op.document_type = document_type;
                        }
                    }
                }

                match DataContractCreateTransition::new_from_data_contract(
                    contract.clone(),
                    *identity_nonce,
                    &partial_identity,
                    partial_identity_public_key.id(),
                    signer,
                    platform_version,
                    None,
                ) {
                    Ok(transition) => transition,
                    Err(e) => {
                        tracing::error!("Error creating DataContractCreateTransition: {e}");
                        panic!();
                    }
                }
            })
            .collect()
    }

    /// Generates state transitions for updating contracts based on the current set of identities and the block height.
    ///
    /// This function identifies and processes updates for data contracts as specified in the strategy, taking into account
    /// the current block height to determine which updates to apply. Each eligible update is matched with its corresponding
    /// identity based on ownership, ensuring that contract state transitions reflect the intended changes within the simulation.
    ///
    /// The function dynamically adjusts contract versions and ownership details, generating update state transitions that
    /// are applied to the simulated blockchain environment. This process enables the simulation of contract evolution over
    /// time, reflecting real-world scenarios where contracts may be updated in response to changing requirements or conditions.
    ///
    /// # Parameters
    /// - `current_identities`: The list of identities involved in the simulation, used to match contract ownership.
    /// - `block_height`: The current block height, used to determine eligibility for contract updates.
    /// - `initial_block_height`: The block height at which the simulation or strategy begins, for calculating update timing.
    /// - `signer`: A reference to a signer instance for signing contract update transactions.
    /// - `contract_nonce_counter`: Tracks nonce values for contract interactions, ensuring uniqueness.
    /// - `platform_version`: The platform version, for compatibility with Dash Platform features.
    ///
    /// # Returns
    /// A vector of `StateTransition`, each representing an update to a data contract within the simulation.
    ///
    /// # Examples
    /// ```ignore
    /// let contract_update_transitions = strategy.initial_contract_update_state_transitions(
    ///     &current_identities,
    ///     block_height,
    ///     initial_block_height,
    ///     &signer,
    ///     &mut contract_nonce_counter,
    ///     &platform_version,
    /// );
    /// ```
    ///
    /// Through these updates, the simulation accurately mirrors the lifecycle of contracts on the Dash Platform, incorporating
    /// changes that may occur over time.
    pub fn initial_contract_update_state_transitions(
        &mut self,
        current_identities: &[Identity],
        block_height: u64,
        initial_block_height: u64,
        signer: &SimpleSigner,
        contract_nonce_counter: &mut BTreeMap<(Identifier, Identifier), u64>,
        platform_version: &PlatformVersion,
    ) -> Vec<StateTransition> {
        // Collect updates
        let updates: Vec<_> = self
            .start_contracts
            .iter()
            .flat_map(|(_, contract_updates_option)| {
                contract_updates_option
                    .as_ref()
                    .map_or_else(Vec::new, |contract_updates| {
                        contract_updates
                            .iter()
                            .filter_map(move |(update_height, contract_update)| {
                                let adjusted_update_height =
                                    initial_block_height + update_height * 3;
                                if adjusted_update_height != block_height {
                                    return None;
                                }
                                current_identities
                                    .iter()
                                    .find(|identity| {
                                        identity.id() == contract_update.data_contract().owner_id()
                                    })
                                    .map(|identity| {
                                        (identity.clone(), *update_height, contract_update)
                                    })
                            })
                            .collect::<Vec<_>>()
                    })
                    .into_iter()
            })
            .collect();

        // Increment nonce counter, update data contract version, and create state transitions
        updates
            .into_iter()
            .map(|(identity, update_height, contract_update)| {
                let identity_info = identity.into_partial_identity_info();
                let contract_id = contract_update.data_contract().id();
                let nonce = contract_nonce_counter
                    .entry((identity_info.id, contract_id))
                    .and_modify(|e| *e += 1)
                    .or_insert(1);

                // Set the version number on the data contract
                let mut contract_update_clone = contract_update.clone();
                let data_contract = contract_update_clone.data_contract_mut();
                data_contract.set_version(update_height as u32);

                // Create the state transition
                DataContractUpdateTransition::new_from_data_contract(
                    data_contract.clone(),
                    &identity_info,
                    2, // Assuming key id 2 is a high or critical auth key
                    *nonce,
                    0,
                    signer,
                    platform_version,
                    None,
                )
                .expect("expected to create a state transition from a data contract")
            })
            .collect()
    }

    /// Returns all contract IDs referenced by operations in this strategy.
    ///
    /// Collects IDs from document operations and contract update operations.
    /// Useful for preloading contracts or validating strategy configuration.
    pub fn used_contract_ids(&self) -> BTreeSet<Identifier> {
        self.operations
            .iter()
            .filter_map(|operation| match &operation.op_type {
                OperationType::Document(document) => Some(document.contract.id()),
                OperationType::ContractUpdate(op) => Some(op.contract.id()),
                _ => None,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::frequency::Frequency;
    use crate::operations::{DocumentAction, DocumentOp, Operation, OperationType};
    use crate::transitions::create_state_transitions_for_identities;
    use crate::StartAddresses;
    use crate::{StartIdentities, Strategy};
    use dpp::dash_to_duffs;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::document_type::random_document::{
        DocumentFieldFillSize, DocumentFieldFillType,
    };
    use dpp::data_contracts::SystemDataContract;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::Identity;
    use dpp::platform_value::Value;
    use dpp::serialization::{
        PlatformDeserializableWithPotentialValidationFromVersionedStructure,
        PlatformSerializableWithPlatformVersion,
    };
    use dpp::system_data_contracts::load_system_data_contract;
    use platform_version::version::PlatformVersion;
    use rand::prelude::StdRng;
    use rand::SeedableRng;
    use simple_signer::signer::SimpleSigner;
    use std::collections::BTreeMap;

    #[test]
    fn serialize_deserialize_strategy() {
        let platform_version = PlatformVersion::latest();

        let mut rng = StdRng::seed_from_u64(567);

        let mut simple_signer = SimpleSigner::default();

        let (mut identity1, keys) = Identity::random_identity_with_main_keys_with_private_key::<
            Vec<_>,
        >(2, &mut rng, platform_version)
        .unwrap();

        simple_signer.add_identity_public_keys(keys);

        let (mut identity2, keys) = Identity::random_identity_with_main_keys_with_private_key::<
            Vec<_>,
        >(2, &mut rng, platform_version)
        .unwrap();

        simple_signer.add_identity_public_keys(keys);

        let start_identities = create_state_transitions_for_identities(
            vec![&mut identity1, &mut identity2],
            &(dash_to_duffs!(1)..=dash_to_duffs!(1)),
            &mut simple_signer,
            &mut rng,
            platform_version,
        );

        let dpns_contract = load_system_data_contract(SystemDataContract::DPNS, platform_version)
            .expect("data contract");

        let document_op_1 = DocumentOp {
            contract: dpns_contract.clone(),
            action: DocumentAction::DocumentActionInsertSpecific(
                BTreeMap::from([
                    ("label".into(), "simon1".into()),
                    ("normalizedLabel".into(), "s1m0n1".into()),
                    ("normalizedParentDomainName".into(), "dash".into()),
                    (
                        "records".into(),
                        BTreeMap::from([(
                            "dashUniqueIdentityId",
                            Value::from(start_identities.first().unwrap().0.id()),
                        )])
                        .into(),
                    ),
                ]),
                Some(start_identities.first().unwrap().0.id()),
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
            ),
            document_type: dpns_contract
                .document_type_cloned_for_name("domain")
                .expect("expected a domain document type"),
        };

        let document_op_2 = DocumentOp {
            contract: dpns_contract.clone(),
            action: DocumentAction::DocumentActionInsertSpecific(
                BTreeMap::from([
                    ("label".into(), "simon1".into()),
                    ("normalizedLabel".into(), "s1m0n1".into()),
                    ("normalizedParentDomainName".into(), "dash".into()),
                    (
                        "records".into(),
                        BTreeMap::from([(
                            "dashUniqueIdentityId",
                            Value::from(start_identities.last().unwrap().0.id()),
                        )])
                        .into(),
                    ),
                ]),
                Some(start_identities.last().unwrap().0.id()),
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
            ),
            document_type: dpns_contract
                .document_type_cloned_for_name("domain")
                .expect("expected a profile document type"),
        };

        let strategy = Strategy {
            start_contracts: vec![],
            operations: vec![
                Operation {
                    op_type: OperationType::Document(document_op_1),
                    frequency: Frequency {
                        times_per_block_range: 1..2,
                        chance_per_block: None,
                    },
                },
                Operation {
                    op_type: OperationType::Document(document_op_2),
                    frequency: Frequency {
                        times_per_block_range: 1..2,
                        chance_per_block: None,
                    },
                },
            ],
            start_identities: StartIdentities {
                number_of_identities: 2,
                keys_per_identity: 3,
                starting_balances: 100_000_000,
                extra_keys: BTreeMap::new(),
                hard_coded: vec![],
            },
            start_addresses: StartAddresses::default(),
            identity_inserts: Default::default(),
            identity_contract_nonce_gaps: None,
            signer: Some(simple_signer),
        };

        let serialized = strategy
            .serialize_to_bytes_with_platform_version(platform_version)
            .expect("expected to serialize");

        let deserialized =
            Strategy::versioned_deserialize(serialized.as_slice(), true, platform_version)
                .expect("expected to deserialize");

        assert_eq!(strategy, deserialized);
    }
}
