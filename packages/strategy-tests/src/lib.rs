//! This library facilitates the creation and execution of comprehensive testing strategies for Dash Platform, leveraging the `Strategy` struct as its core.
//! It is designed to simulate a wide range of blockchain activities, offering detailed control over the generation of state transitions, contract interactions, and identity management across blocks.
//!
//! Utilizing this library, users can craft scenarios that encompass every conceivable state transition on Dash Platform, with precise timing control on a block-by-block basis.
//! Strategies can be as simple or complex as needed, from initializing contracts and identities at the start of a simulation to conducting intricate operations like document submissions, credit transfers, and more throughout the lifespan of the blockchain.
//!
//! This tool does not require any preliminary setup for the entities involved in the strategies; identities, contracts, and documents can be introduced at any point in the simulation.
//! This flexibility ensures users can test against both new and existing blockchain states, adapting the scenarios as Dash Platform evolves.
//!
//! As of March 2024, the recommended approach to leverage this library's capabilities is through the `Strategies` module within Dash Platform's terminal user interface, located at `dashpay/rs-platform-explorer`.
//! This interface provides an accessible and streamlined way to define, manage, and execute your testing strategies against Dash Platform.

use crate::frequency::Frequency;
use crate::operations::FinalizeBlockOperation::IdentityAddKeys;
use crate::operations::{
    DocumentAction, DocumentOp, FinalizeBlockOperation, IdentityUpdateOp, Operation, OperationType,
};
use dpp::block::block_info::BlockInfo;
use dpp::dashcore::PrivateKey;
use dpp::data_contract::created_data_contract::CreatedDataContract;
use dpp::data_contract::document_type::random_document::CreateRandomDocument;
use dpp::data_contract::document_type::v0::DocumentTypeV0;
use dpp::data_contract::{DataContract, DataContractFactory};

use dpp::document::{Document, DocumentV0Getters};
use dpp::identity::state_transition::asset_lock_proof::AssetLockProof;
use dpp::identity::{Identity, KeyID, KeyType, PartialIdentity, Purpose, SecurityLevel};
use dpp::platform_value::string_encoding::Encoding;
use dpp::serialization::{
    PlatformDeserializableWithPotentialValidationFromVersionedStructure,
    PlatformSerializableWithPlatformVersion,
};
use dpp::state_transition::data_contract_create_transition::DataContractCreateTransition;
use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use dpp::state_transition::StateTransition;
use dpp::version::PlatformVersion;
use drive::drive::identity::key::fetch::{IdentityKeysRequest, KeyRequestType};

use dpp::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
use dpp::state_transition::data_contract_update_transition::methods::DataContractUpdateTransitionMethodsV0;
use operations::{DataContractUpdateAction, DataContractUpdateOp};
use platform_version::TryFromPlatformVersioned;
use rand::prelude::StdRng;
use rand::{thread_rng, Rng};
use tracing::error;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use bincode::{Decode, Encode};
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::identifier::Identifier;
use dpp::data_contract::document_type::DocumentType;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::platform_value::{BinaryData, Value};
use dpp::ProtocolError;
use dpp::ProtocolError::{PlatformDeserializationError, PlatformSerializationError};
use dpp::state_transition::documents_batch_transition::document_base_transition::v0::DocumentBaseTransitionV0;
use dpp::state_transition::documents_batch_transition::document_create_transition::{DocumentCreateTransition, DocumentCreateTransitionV0};
use dpp::state_transition::documents_batch_transition::document_transition::document_delete_transition::DocumentDeleteTransitionV0;
use dpp::state_transition::documents_batch_transition::document_transition::document_replace_transition::DocumentReplaceTransitionV0;
use dpp::state_transition::documents_batch_transition::{DocumentsBatchTransition, DocumentsBatchTransitionV0};
use dpp::state_transition::documents_batch_transition::document_transition::{DocumentDeleteTransition, DocumentReplaceTransition};
use dpp::state_transition::data_contract_create_transition::methods::v0::DataContractCreateTransitionMethodsV0;
use simple_signer::signer::SimpleSigner;

pub mod frequency;
pub mod operations;
pub mod transitions;
pub type KeyMaps = BTreeMap<Purpose, BTreeMap<SecurityLevel, Vec<KeyType>>>;

/// Defines a detailed strategy for conducting simulations or tests on Dash Platform.
///
/// This struct serves as the core framework for designing and executing comprehensive simulations or automated testing scenarios on Dash Platform. It encompasses a wide array of operations, state transitions, and data contract manipulations, enabling users to craft intricate strategies that mimic real-world blockchain dynamics or test specific functionalities.
///
/// The strategy allows for the specification of initial conditions, such as contracts to be created and identities to be registered, as well as dynamic actions that unfold over the simulation's lifespan, including contract updates and identity transactions. This versatile structure supports a broad spectrum of blockchain-related activities, from simple transfer operations to complex contract lifecycle management.
///
/// # Fields
/// - `contracts_with_updates`: Maps each created data contract to potential updates, enabling the simulation of contract evolution. Each tuple consists of a `CreatedDataContract` and an optional mapping of block heights to subsequent contract versions, facilitating time-sensitive contract transformations.
///
/// - `operations`: Enumerates discrete operations to be executed within the strategy. These operations represent individual actions or sequences of actions, such as document manipulations, identity updates, or contract interactions, each contributing to the overarching simulation narrative.
///
/// - `start_identities`: Specifies identities to be established at the simulation's outset, including their initial attributes and balances. This setup allows for immediate participation of these identities in the blockchain's simulated activities.
///
/// - `identities_inserts`: Controls the stochastic introduction of new identities into the simulation, based on a defined frequency distribution. This field allows the strategy to dynamically expand the set of participants, reflecting organic growth or specific testing requirements.
///
/// - `identity_contract_nonce_gaps`: Optionally defines intervals at which nonce values for identities and contracts may be artificially incremented, introducing realistic entropy or testing specific edge cases.
///
/// - `signer`: Provides an optional `SimpleSigner` instance responsible for generating cryptographic signatures for various transactions within the strategy. While optional, a signer is critical for authenticating state transitions and operations that require verification.
///
/// # Usage Example
/// ```ignore
/// let strategy = Strategy {
///     contracts_with_updates: vec![...], // Initial contracts and their planned updates
///     operations: vec![...],             // Defined operations to simulate blockchain interactions
///     start_identities: StartIdentities::new(...), // Identities to initialize
///     identities_inserts: Frequency::new(...),     // Frequency of new identity introduction
///     identity_contract_nonce_gaps: Some(Frequency::new(...)), // Optional nonce gaps
///     signer: Some(SimpleSigner::new(...)),        // Optional signer for authenticating transactions
/// };
/// ```
///
/// # Implementation Note
/// It's imperative to maintain coherence among the specified operations, identities, and contracts within the `Strategy` to ensure the simulated scenarios accurately reflect intended behaviors or test conditions. Discrepancies or inconsistencies may result in unexpected outcomes or hinder the simulation's effectiveness in achieving its objectives.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Strategy {
    pub contracts_with_updates: Vec<(
        CreatedDataContract,
        Option<BTreeMap<u64, CreatedDataContract>>,
    )>,
    pub operations: Vec<Operation>,
    pub start_identities: StartIdentities,
    pub identities_inserts: IdentityInsertInfo,
    pub identity_contract_nonce_gaps: Option<Frequency>,
    pub signer: Option<SimpleSigner>,
}

/// Config stuff for a Strategy
#[derive(Clone, Debug, PartialEq)]
pub struct StrategyConfig {
    pub start_block_height: u64,
    pub number_of_blocks: u64,
}

/// Identities to register on the first block of the strategy
#[derive(Clone, Debug, PartialEq, Default, Encode, Decode)]
pub struct StartIdentities {
    pub number_of_identities: u8,
    pub keys_per_identity: u8,
    pub starting_balances: u64, // starting balance in duffs
}

/// Identities to register on the first block of the strategy
#[derive(Clone, Debug, PartialEq, Encode, Decode)]
pub struct IdentityInsertInfo {
    pub frequency: Frequency,
    pub start_keys: u8,
    pub extra_keys: KeyMaps,
}

impl Default for IdentityInsertInfo {
    fn default() -> Self {
        Self {
            frequency: Default::default(),
            start_keys: 5,
            extra_keys: Default::default(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RandomDocumentQuery<'a> {
    pub data_contract: &'a DataContract,
    pub document_type: &'a DocumentType,
}

#[derive(Clone, Debug, PartialEq)]
pub enum LocalDocumentQuery<'a> {
    RandomDocumentQuery(RandomDocumentQuery<'a>),
}

#[derive(Clone, Debug, Encode, Decode)]
struct StrategyInSerializationFormat {
    pub contracts_with_updates: Vec<(Vec<u8>, Option<BTreeMap<u64, Vec<u8>>>)>,
    pub operations: Vec<Vec<u8>>,
    pub start_identities: StartIdentities,
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
            contracts_with_updates,
            operations,
            start_identities,
            identities_inserts,
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
        validate: bool,
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
            identities_inserts,
            identity_contract_nonce_gaps,
            signer,
        } = strategy;

        let contracts_with_updates = contracts_with_updates
            .into_iter()
            .map(|(serialized_contract, maybe_updates)| {
                let contract = CreatedDataContract::versioned_deserialize(
                    serialized_contract.as_slice(),
                    validate,
                    platform_version,
                )?;
                let maybe_updates = maybe_updates
                    .map(|updates| {
                        updates
                            .into_iter()
                            .map(|(key, serialized_contract_update)| {
                                let update = CreatedDataContract::versioned_deserialize(
                                    serialized_contract_update.as_slice(),
                                    validate,
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
                Operation::versioned_deserialize(operation.as_slice(), validate, platform_version)
            })
            .collect::<Result<Vec<Operation>, ProtocolError>>()?;

        Ok(Strategy {
            contracts_with_updates,
            operations,
            start_identities,
            identities_inserts,
            identity_contract_nonce_gaps,
            signer,
        })
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
    /// - `create_asset_lock`: Callback for creating asset lock proofs, primarily for identity transactions.
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
    ///     &mut create_asset_lock,
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
    pub async fn state_transitions_for_block(
        &mut self,
        document_query_callback: &mut impl FnMut(LocalDocumentQuery) -> Vec<Document>,
        identity_fetch_callback: &mut impl FnMut(
            Identifier,
            Option<IdentityKeysRequest>,
        ) -> PartialIdentity,
        create_asset_lock: &mut impl FnMut(u64) -> Option<(AssetLockProof, PrivateKey)>,
        block_info: &BlockInfo,
        current_identities: &mut Vec<Identity>,
        known_contracts: &mut BTreeMap<String, DataContract>,
        signer: &mut SimpleSigner,
        identity_nonce_counter: &mut BTreeMap<Identifier, u64>,
        contract_nonce_counter: &mut BTreeMap<(Identifier, Identifier), u64>,
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
        let identity_state_transitions = match self.identity_state_transitions_for_block(
            block_info,
            self.start_identities.starting_balances,
            signer,
            rng,
            create_asset_lock,
            config,
            platform_version,
        ) {
            Ok(transitions) => transitions,
            Err(e) => {
                error!("identity_state_transitions_for_block error: {}", e);
                return (vec![], finalize_block_operations, vec![]);
            }
        };

        // Create state_transitions vec and identities vec based on identity_state_transitions outcome
        let (identities, mut state_transitions): (Vec<Identity>, Vec<StateTransition>) =
            identity_state_transitions.into_iter().unzip();

        // Do we also need to add identities to the identity_nonce_counter?

        // Add initial contracts for contracts_with_updates on first block of strategy
        if block_info.height == config.start_block_height {
            let mut contract_state_transitions = self.initial_contract_state_transitions(
                current_identities,
                identity_nonce_counter,
                signer,
                rng,
                platform_version,
            );
            state_transitions.append(&mut contract_state_transitions);
        } else {
            // Do operations and contract updates after the first block
            let (mut operations_state_transitions, mut add_to_finalize_block_operations) = self
                .operations_based_transitions(
                    document_query_callback,
                    identity_fetch_callback,
                    create_asset_lock,
                    block_info,
                    current_identities,
                    known_contracts,
                    signer,
                    identity_nonce_counter,
                    contract_nonce_counter,
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
    /// - `create_asset_lock`: A callback function for creating asset lock proofs for identity transactions.
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
    ///     &mut create_asset_lock,
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
    pub fn operations_based_transitions(
        &self,
        document_query_callback: &mut impl FnMut(LocalDocumentQuery) -> Vec<Document>,
        identity_fetch_callback: &mut impl FnMut(
            Identifier,
            Option<IdentityKeysRequest>,
        ) -> PartialIdentity,
        create_asset_lock: &mut impl FnMut(u64) -> Option<(AssetLockProof, PrivateKey)>,
        block_info: &BlockInfo,
        current_identities: &mut [Identity],
        known_contracts: &mut BTreeMap<String, DataContract>,
        signer: &mut SimpleSigner,
        identity_nonce_counter: &mut BTreeMap<Identifier, u64>,
        contract_nonce_counter: &mut BTreeMap<(Identifier, Identifier), u64>,
        rng: &mut StdRng,
        platform_version: &PlatformVersion,
    ) -> (Vec<StateTransition>, Vec<FinalizeBlockOperation>) {
        // Lists to store generated operations and block finalization operations
        let mut operations = vec![];
        let mut finalize_block_operations = vec![];

        // Lists to keep track of replaced and deleted documents
        let mut replaced = vec![];
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
                        // TO-DO: these documents should be created according to the data contract's validation rules
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
                                    }
                                    .into();

                                let document_batch_transition: DocumentsBatchTransition =
                                    DocumentsBatchTransitionV0 {
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
                            });
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
                                    }
                                    .into();

                                let document_batch_transition: DocumentsBatchTransition =
                                    DocumentsBatchTransitionV0 {
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
                        //// the following is removed in favor of the local document query callback above
                        // let mut items = drive
                        //     .query_documents(
                        //         any_item_query,
                        //         Some(&block_info.epoch),
                        //         false,
                        //         None,
                        //         Some(platform_version.protocol_version),
                        //     )
                        //     .expect("expect to execute query")
                        //     .documents_owned();

                        items.retain(|item| !deleted.contains(&item.id()));

                        items.retain(|item| !replaced.contains(&item.id()));

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
                            let identity =
                                identity_fetch_callback(request.identity_id.into(), Some(request));
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

                    // Generate state transition for document replace operation
                    OperationType::Document(DocumentOp {
                        action: DocumentAction::DocumentActionReplace,
                        document_type,
                        contract,
                    }) => {
                        let mut items = document_query_callback(
                            LocalDocumentQuery::RandomDocumentQuery(RandomDocumentQuery {
                                data_contract: contract,
                                document_type,
                            }),
                        );
                        // let any_item_query =
                        //     DriveQuery::any_item_query(contract, document_type.as_ref());
                        // let mut items = drive
                        //     .query_documents(
                        //         any_item_query,
                        //         Some(&block_info.epoch),
                        //         false,
                        //         None,
                        //         Some(platform_version.protocol_version),
                        //     )
                        //     .expect("expect to execute query")
                        //     .documents_owned();

                        items.retain(|item| !deleted.contains(&item.id()));

                        items.retain(|item| !replaced.contains(&item.id()));

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

                            let document_batch_transition: DocumentsBatchTransition =
                                DocumentsBatchTransitionV0 {
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

                    // Generate state transition for identity top-up operation
                    OperationType::IdentityTopUp if !current_identities.is_empty() => {
                        // Use a cyclic iterator over the identities to ensure we can create 'count' transitions
                        let cyclic_identities = current_identities.iter().cycle();

                        // Iterate 'count' times to create the required number of state transitions.
                        for random_identity in cyclic_identities.take(count.into()) {
                            match crate::transitions::create_identity_top_up_transition(
                                random_identity,
                                create_asset_lock,
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
                                    let random_index = thread_rng().gen_range(0..identities_count);
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
                    OperationType::IdentityWithdrawal if !current_identities.is_empty() => {
                        for i in 0..count {
                            let index = (i as usize) % current_identities.len();
                            let random_identity = &mut current_identities[index];
                            let state_transition =
                                crate::transitions::create_identity_withdrawal_transition(
                                    random_identity,
                                    identity_nonce_counter,
                                    signer,
                                    rng,
                                );
                            operations.push(state_transition);
                        }
                    }

                    // Generate state transition for identity transfer operation
                    OperationType::IdentityTransfer if current_identities.len() > 1 => {
                        let identities_clone = current_identities.to_owned();
                        // Sender is the first in the list, which should be loaded_identity
                        let owner = &mut current_identities[0];
                        // Recipient is the second in the list
                        let recipient = &identities_clone[1];
                        for _ in 0..count {
                            let state_transition =
                                crate::transitions::create_identity_credit_transfer_transition(
                                    owner,
                                    recipient,
                                    identity_nonce_counter,
                                    signer,
                                    1000,
                                );
                            operations.push(state_transition);
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
                                error!("Failed to get DataContractFactory while creating random contract: {e}");
                                continue;
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
                                                let mut doc_type_clone =
                                                    new_document_type.schema().clone();
                                                let name = doc_type_clone.remove("title").expect(
                                            "Expected to get a doc type title in ContractCreate",
                                        );
                                                Some((
                                                    Value::Text(name.to_string()),
                                                    doc_type_clone,
                                                ))
                                            }
                                            Err(e) => {
                                                error!(
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
                                    error!("Failed to create random data contract: {e}");
                                    continue;
                                }
                            };

                            let transition = match contract_factory
                                .create_data_contract_create_transition(created_data_contract)
                            {
                                Ok(transition) => transition,
                                Err(e) => {
                                    error!("Failed to create ContractCreate transition: {e}");
                                    continue;
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
                                error!("Error signing state transition: {:?}", e);
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

                                        let identity = &current_identities[0];
                                        let identity_contract_nonce = contract_nonce_counter
                                            .entry((identity.id(), contract_ref.id()))
                                            .or_default();
                                        *identity_contract_nonce += 1;

                                        // Prepare the DataContractUpdateTransition with the updated contract_ref
                                        match DataContractUpdateTransition::try_from_platform_versioned((DataContract::V0(contract_ref.clone()), *identity_contract_nonce), platform_version) {
                                            Ok(data_contract_update_transition) => {
                                                let identity_public_key = current_identities[0]
                                                    .get_first_public_key_matching(
                                                        Purpose::AUTHENTICATION,
                                                        HashSet::from([SecurityLevel::CRITICAL]),
                                                        HashSet::from([KeyType::ECDSA_SECP256K1, KeyType::BLS12_381]),
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
                                            Err(e) => error!("Error converting data contract to update transition: {:?}", e),
                                        }
                                    }
                                    Err(e) => {
                                        error!("Error generating random document type: {:?}", e)
                                    }
                                }
                            } else {
                                // Handle the case where the contract is not found in known_contracts
                            }
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
    /// - `create_asset_lock`: A mutable reference to a callback function that generates an asset lock proof and associated private key, used in identity creation transactions.
    /// - `config`: Configuration details of the strategy, including the start block height.
    /// - `platform_version`: Specifies the version of the Dash Platform, ensuring compatibility with its features and behaviors.
    ///
    /// # Returns
    /// A vector of tuples, each containing an `Identity` and its associated `StateTransition`, representing the actions taken by or on behalf of that identity within the block.
    ///
    /// # Examples
    /// ```ignore
    /// // Assuming `strategy` is an instance of `Strategy`, with `block_info`, `signer`, `rng`,
    /// // `create_asset_lock`, `config`, and `platform_version` properly initialized:
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
    pub fn identity_state_transitions_for_block(
        &self,
        block_info: &BlockInfo,
        amount: u64,
        signer: &mut SimpleSigner,
        rng: &mut StdRng,
        create_asset_lock: &mut impl FnMut(u64) -> Option<(AssetLockProof, PrivateKey)>,
        config: &StrategyConfig,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<(Identity, StateTransition)>, ProtocolError> {
        let mut state_transitions = vec![];

        // Add start_identities
        if block_info.height == config.start_block_height
            && self.start_identities.number_of_identities > 0
        {
            let mut new_transitions = crate::transitions::create_identities_state_transitions(
                self.start_identities.number_of_identities.into(), // number of identities
                self.start_identities.keys_per_identity.into(),    // number of keys per identity
                &self.identities_inserts.extra_keys,
                amount,
                signer,
                rng,
                create_asset_lock,
                platform_version,
            )?;
            state_transitions.append(&mut new_transitions);
        }

        // Add identities_inserts
        // Don't do this on first block because we need to skip utxo refresh
        if block_info.height > config.start_block_height {
            let frequency = &self.identities_inserts.frequency;
            if frequency.check_hit(rng) {
                let count = frequency.events(rng);
                let mut new_transitions = crate::transitions::create_identities_state_transitions(
                    count,                                       // number of identities
                    self.identities_inserts.start_keys as KeyID, // number of keys per identity
                    &self.identities_inserts.extra_keys,
                    200000, // 0.002 dash
                    signer,
                    rng,
                    create_asset_lock,
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

        self.contracts_with_updates
            .iter_mut()
            .map(|(created_contract, contract_updates)| {
                // Select a random identity from current_identities to be the contract owner
                let identity_num = rng.gen_range(0..current_identities.len());
                let identity = current_identities
                    .get(identity_num)
                    .unwrap()
                    .clone()
                    .into_partial_identity_info();

                let contract = created_contract.data_contract_mut();

                // Get and bump the identity nonce
                let identity_nonce = identity_nonce_counter.entry(identity.id).or_default();
                *identity_nonce += 1;

                // Set the contract ID and owner ID with the random identity
                contract.set_owner_id(identity.id);
                let old_id = contract.id();
                let new_id =
                    DataContract::generate_data_contract_id_v0(identity.id, *identity_nonce);
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

                DataContractCreateTransition::new_from_data_contract(
                    contract.clone(),
                    *identity_nonce,
                    &identity,
                    2,
                    signer,
                    platform_version,
                    None,
                )
                .expect("expected to create a create state transition from a data contract")
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
            .contracts_with_updates
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

    /// Convenience method to get all contract ids that are in operations
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
    use crate::{StartIdentities, Strategy};
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

        let (identity1, keys) =
            Identity::random_identity_with_main_keys_with_private_key::<Vec<_>>(
                2,
                &mut rng,
                platform_version,
            )
            .unwrap();

        simple_signer.add_keys(keys);

        let (identity2, keys) =
            Identity::random_identity_with_main_keys_with_private_key::<Vec<_>>(
                2,
                &mut rng,
                platform_version,
            )
            .unwrap();

        simple_signer.add_keys(keys);

        let start_identities = create_state_transitions_for_identities(
            vec![identity1, identity2],
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
            contracts_with_updates: vec![],
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
            },
            identities_inserts: Default::default(),
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
