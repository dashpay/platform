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
use dpp::identity::{Identity, KeyType, PartialIdentity, Purpose, SecurityLevel};
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
use rand::Rng;
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

/// Represents a comprehensive strategy used for simulations or testing in a blockchain context.
///
/// The `Strategy` struct encapsulates various operations, state transitions, and data contracts to provide a structured plan or set of procedures for specific purposes such as simulations, automated tests, or other blockchain-related workflows.
///
/// # Fields
/// - `contracts_with_updates`: A list of tuples containing:
///   1. `CreatedDataContract`: A data contract that was created.
///   2. `Option<BTreeMap<u64, CreatedDataContract>>`: An optional mapping where the key is the block height (or other sequential integer identifier) and the value is a data contract that corresponds to an update. If `None`, it signifies that there are no updates.
///
/// - `operations`: A list of `Operation`s which define individual tasks or actions that are part of the strategy. Operations could encompass a range of blockchain-related actions like transfers, state changes, contract creations, etc.
///
/// - `start_identities`: An instance of StartIdentities, which specifies the number of identities to register on the first block of the strategy, along with number of keys per identity and starting balance of each identity.
///
/// - `identities_inserts`: Defines the frequency distribution of identity inserts. `Frequency` might encapsulate statistical data like mean, median, variance, etc., for understanding or predicting the frequency of identity insertions.
///
/// - `signer`: An optional instance of `SimpleSigner`. The `SimpleSigner` is responsible for generating and managing cryptographic signatures, and might be used to authenticate or validate various operations or state transitions.
///
/// # Usage
/// ```ignore
/// let strategy = Strategy {
///     contracts_with_updates: vec![...],
///     operations: vec![...],
///     start_identities: vec![...],
///     identities_inserts: Frequency::new(...),
///     signer: Some(SimpleSigner::new(...)),
/// };
/// ```
///
/// # Note
/// Ensure that when using or updating the `Strategy`, all associated operations, identities, and contracts are coherent with the intended workflow or simulation. Inconsistencies might lead to unexpected behaviors or simulation failures.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Strategy {
    pub contracts_with_updates: Vec<(
        CreatedDataContract,
        Option<BTreeMap<u64, CreatedDataContract>>,
    )>,
    pub operations: Vec<Operation>,
    pub start_identities: StartIdentities,
    pub identities_inserts: Frequency,
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
    pub starting_balances: Option<u64>,
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
    pub identities_inserts: Frequency,
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

    /// Creates state transitions based on the `identities_inserts` and `start_identities` fields.
    ///
    /// This method creates a list of state transitions associated with identities. If the block height
    /// is `1` and there are starting identities present in the strategy, these identities are directly
    /// added to the state transitions list.
    ///
    /// Additionally, based on a frequency criterion, this method can generate and append more state transitions
    /// related to the creation of identities.
    ///
    /// # Parameters
    /// - `block_info`: Information about the current block, used to decide on which state transitions should be generated.
    /// - `signer`: A mutable reference to a signer instance used during the creation of identities state transitions.
    /// - `rng`: A mutable reference to a random number generator.
    /// - `platform_version`: The current platform version.
    ///
    /// # Returns
    /// A vector of tuples containing `Identity` and its associated `StateTransition`.
    ///
    /// # Examples
    /// ```ignore
    /// // Assuming `strategy` is an instance of `Strategy`,
    /// // and `block_info`, `signer`, `rng`, and `platform_version` are appropriately initialized.
    /// let state_transitions = strategy.identity_state_transitions_for_block(&block_info, &mut signer, &mut rng, &platform_version);
    /// ```
    pub fn identity_state_transitions_for_block(
        &self,
        block_info: &BlockInfo,
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
            let frequency = &self.identities_inserts;
            if frequency.check_hit(rng) {
                let count = frequency.events(rng);
                let mut new_transitions = crate::transitions::create_identities_state_transitions(
                    count, // number of identities
                    3,     // number of keys per identity
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

    /// Generates state transitions for the initial contracts of the contracts_with_updates field.
    ///
    /// This method creates state transitions for data contracts by iterating over the contracts with updates
    /// present in the strategy. For each contract:
    /// 1. An identity is randomly selected from the provided list of current identities.
    /// 2. The owner ID of the contract is set to the selected identity's ID.
    /// 3. The ID of the contract is updated based on the selected identity's ID and entropy used during its creation.
    /// 4. Any contract updates associated with the main contract are adjusted to reflect these changes.
    /// 5. All operations in the strategy that match the old contract ID are updated with the new contract ID.
    ///
    /// Finally, a new data contract create state transition is generated using the modified contract.
    ///
    /// # Parameters
    /// - `current_identities`: A reference to a list of current identities.
    /// - `signer`: A reference to a signer instance used during the creation of state transitions.
    /// - `rng`: A mutable reference to a random number generator.
    /// - `platform_version`: The current platform version.
    ///
    /// # Returns
    /// A vector of `StateTransition` for data contracts.
    ///
    /// # Examples
    /// ```ignore
    /// // Assuming `strategy` is an instance of `Strategy`,
    /// // and `current_identities`, `signer`, `rng`, and `platform_version` are appropriately initialized.
    /// let contract_transitions = strategy.contract_state_transitions(&current_identities, &signer, &mut rng, &platform_version);
    /// ```
    pub fn contract_state_transitions(
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
                let identity_num = rng.gen_range(0..current_identities.len());
                let identity = current_identities
                    .get(identity_num)
                    .unwrap()
                    .clone()
                    .into_partial_identity_info();

                let contract = created_contract.data_contract_mut();

                let identity_nonce = identity_nonce_counter.entry(identity.id).or_default();
                *identity_nonce += 1;

                contract.set_owner_id(identity.id);
                let old_id = contract.id();
                let new_id =
                    DataContract::generate_data_contract_id_v0(identity.id, *identity_nonce);
                contract.set_id(new_id);

                id_mapping.insert(old_id, new_id); // Store the mapping

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
                        document_op.contract = contract.clone();
                        let document_type = contract.document_type_cloned_for_name(document_op.document_type.name())
                            .expect("Expected to get a document type for name while creating initial strategy contracts");
                        document_op.document_type = document_type;
                    }
                }

                DataContractCreateTransition::new_from_data_contract(
                    contract.clone(),
                    *identity_nonce,
                    &identity,
                    2, // key id 1 should always be a high or critical auth key in these tests
                    signer,
                    platform_version,
                    None,
                )
                .expect("expected to create a create state transition from a data contract")
            })
            .collect()
    }

    /// Generates state transitions for contract updates in contracts_with_updates based on the current set of identities and block height.
    ///
    /// This method creates update state transitions for data contracts by iterating over the contracts with updates
    /// present in the strategy. For each contract:
    /// 1. It checks for any contract updates associated with the provided block height.
    /// 2. For each matching update, it locates the corresponding identity based on the owner ID in the update.
    /// 3. A new data contract update state transition is then generated using the located identity and the updated contract.
    ///
    /// # Parameters
    /// - `current_identities`: A reference to a list of current identities.
    /// - `block_height`: The height of the current block.
    /// - `signer`: A reference to a signer instance used during the creation of state transitions.
    /// - `platform_version`: The current platform version.
    ///
    /// # Returns
    /// A vector of `StateTransition` for updating data contracts.
    ///
    /// # Panics
    /// The method will panic if it doesn't find an identity matching the owner ID from the data contract update.
    ///
    /// # Examples
    /// ```ignore
    /// // Assuming `strategy` is an instance of `Strategy`,
    /// // and `current_identities`, `block_height`, `signer`, and `platform_version` are appropriately initialized.
    /// let update_transitions = strategy.contract_update_state_transitions(&current_identities, block_height, &signer, &platform_version);
    /// ```
    pub fn contract_update_state_transitions(
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

    /// Generates state transitions for a given block based on Strategy operations.
    ///
    /// The `state_transitions_for_block` function processes state transitions based on the provided
    /// block information, platform, identities, and other input parameters. It facilitates
    /// the creation of state transitions for both new documents and updated documents in the system.
    /// Only deals with the operations field of Strategy. Not contracts_with_updates or identities fields.
    ///
    /// # Parameters
    /// - `platform`: A reference to the platform, which provides access to various blockchain
    ///   related functionalities and data.
    /// - `block_info`: Information about the block for which the state transitions are being generated.
    ///   This contains data such as its height and time.
    /// - `current_identities`: A mutable reference to the list of current identities in the system.
    ///   This list is used to facilitate state transitions related to the involved identities.
    /// - `signer`: A mutable reference to a signer, which aids in creating cryptographic signatures
    ///   for the state transitions.
    /// - `rng`: A mutable reference to a random number generator, used for generating random values
    ///   during state transition creation.
    /// - `platform_version`: The version of the platform being used. This information is crucial
    ///   to ensure compatibility and consistency in state transition generation.
    ///
    /// # Returns
    /// A tuple containing:
    /// 1. `Vec<StateTransition>`: A vector of state transitions generated for the given block.
    ///    These transitions encompass both new document state transitions and document update transitions.
    /// 2. `Vec<FinalizeBlockOperation>`: A vector of finalize block operations which may be necessary
    ///    to conclude the block's processing.
    ///
    /// # Examples
    /// ```ignore
    /// let (state_transitions, finalize_ops) = obj.state_transitions_for_block(
    ///     &platform,
    ///     &block_info,
    ///     &mut current_identities,
    ///     &mut signer,
    ///     &mut rng,
    ///     platform_version,
    /// );
    /// ```
    ///
    /// # Panics
    /// This function may panic under unexpected conditions, for example, when unable to generate state
    /// transitions for the given block.
    pub fn state_transitions_for_block(
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
                                block_info.time_ms,
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
                                    block_info.time_ms,
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
                                    block_info.time_ms,
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
                                (0..count).for_each(|_| {
                                    current_identities.iter_mut().enumerate().for_each(|(i, random_identity)| {
                                        if i >= count.into() { return; }

                                        let (state_transition, keys_to_add_at_end_block) =
                                            crate::transitions::create_identity_update_transition_add_keys(
                                                random_identity,
                                                *keys_count,
                                                identity_nonce_counter,
                                                signer,
                                                rng,
                                                platform_version,
                                            );
                                        operations.push(state_transition);
                                        finalize_block_operations.push(IdentityAddKeys(
                                            keys_to_add_at_end_block.0,
                                            keys_to_add_at_end_block.1,
                                        ));
                                    });
                                });
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

    /// Generates state transitions for a block by considering new identities.
    ///
    /// This function processes state transitions with respect to identities, contracts,
    /// and document operations. The state transitions are generated based on the
    /// given block's height and other parameters, with special handling for the initial block height.
    ///
    /// # Parameters
    /// - `platform`: A reference to the platform, which is parameterized with a mock core RPC type.
    /// - `block_info`: Information about the current block, like its height and time.
    /// - `current_identities`: A mutable reference to the current set of identities. This list
    ///   may be appended with new identities during processing.
    /// - `signer`: A mutable reference to a signer used for creating cryptographic signatures.
    /// - `rng`: A mutable reference to a random number generator.
    ///
    /// # Returns
    /// A tuple containing two vectors:
    /// 1. `Vec<StateTransition>`: A vector of state transitions generated during processing.
    /// 2. `Vec<FinalizeBlockOperation>`: A vector of finalize block operations derived during processing.
    ///
    /// # Examples
    /// ```ignore
    /// let (state_transitions, finalize_ops) = obj.state_transitions_for_block_with_new_identities(
    ///     &platform,
    ///     &block_info,
    ///     &mut current_identities,
    ///     &mut signer,
    ///     &mut rng,
    ///     platform_version
    /// );
    /// ```
    pub async fn state_transitions_for_block_with_new_identities(
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
    ) -> (Vec<StateTransition>, Vec<FinalizeBlockOperation>) {
        let mut finalize_block_operations = vec![];

        // Get identity state transitions
        let identity_state_transitions = match self.identity_state_transitions_for_block(
            block_info,
            signer,
            rng,
            create_asset_lock,
            config,
            platform_version,
        ) {
            Ok(transitions) => transitions,
            Err(e) => {
                error!("identity_state_transitions_for_block error: {}", e);
                return (vec![], finalize_block_operations);
            }
        };

        // Create state_transitions vec and identities vec based on identity_state_transitions outcome
        let (mut identities, mut state_transitions): (Vec<Identity>, Vec<StateTransition>) =
            identity_state_transitions.into_iter().unzip();

        // Append the new identities to current_identities
        current_identities.append(&mut identities);

        // Do we also need to add identities to the identity_nonce_counter?

        // Add initial contracts for contracts_with_updates on first block of strategy
        if block_info.height == config.start_block_height {
            let mut contract_state_transitions = self.contract_state_transitions(
                current_identities,
                identity_nonce_counter,
                signer,
                rng,
                platform_version,
            );
            state_transitions.append(&mut contract_state_transitions);
        } else {
            // Do operations and contract updates after the first block
            let (mut document_state_transitions, mut add_to_finalize_block_operations) = self
                .state_transitions_for_block(
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
            state_transitions.append(&mut document_state_transitions);

            // Contract updates for contracts_with_updates
            let mut contract_update_state_transitions = self.contract_update_state_transitions(
                current_identities,
                block_info.height,
                config.start_block_height,
                signer,
                contract_nonce_counter,
                platform_version,
            );
            state_transitions.append(&mut contract_update_state_transitions);
        }

        (state_transitions, finalize_block_operations)
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
                starting_balances: None,
            },
            identities_inserts: Frequency {
                times_per_block_range: Default::default(),
                chance_per_block: None,
            },
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
