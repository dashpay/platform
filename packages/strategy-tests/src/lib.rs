use crate::frequency::Frequency;
use crate::operations::FinalizeBlockOperation::IdentityAddKeys;
use crate::operations::{
    DocumentAction, DocumentOp, FinalizeBlockOperation, IdentityUpdateOp, Operation, OperationType,
};
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::created_data_contract::CreatedDataContract;
use dpp::data_contract::document_type::random_document::CreateRandomDocument;
use dpp::data_contract::DataContract;

use dpp::document::DocumentV0Getters;
use dpp::identity::{Identity, KeyType, Purpose, SecurityLevel};
use dpp::serialization::PlatformSerializableWithPlatformVersion;
use dpp::state_transition::data_contract_create_transition::DataContractCreateTransition;
use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use dpp::state_transition::StateTransition;
use dpp::version::PlatformVersion;
use drive::drive::flags::StorageFlags::SingleEpoch;
use drive::drive::identity::key::fetch::{IdentityKeysRequest, KeyRequestType};
use drive::drive::Drive;

use dpp::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
use dpp::state_transition::data_contract_update_transition::methods::DataContractUpdateTransitionMethodsV0;
use drive::query::DriveQuery;
use rand::prelude::{IteratorRandom, StdRng};
use rand::Rng;
use std::borrow::Cow;
use std::collections::{BTreeMap, HashSet};
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::platform_value::BinaryData;
use dpp::state_transition::documents_batch_transition::document_base_transition::v0::DocumentBaseTransitionV0;
use dpp::state_transition::documents_batch_transition::document_create_transition::{DocumentCreateTransition, DocumentCreateTransitionV0};
use dpp::state_transition::documents_batch_transition::document_transition::document_delete_transition::DocumentDeleteTransitionV0;
use dpp::state_transition::documents_batch_transition::document_transition::document_replace_transition::DocumentReplaceTransitionV0;
use dpp::state_transition::documents_batch_transition::{DocumentsBatchTransition, DocumentsBatchTransitionV0};
use dpp::state_transition::documents_batch_transition::document_transition::{DocumentDeleteTransition, DocumentReplaceTransition};
use drive::drive::document::query::QueryDocumentsOutcomeV0Methods;
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
/// - `start_identities`: A list of tuples representing the starting state of identities. Each tuple contains:
///   1. `Identity`: The initial identity state.
///   2. `StateTransition`: The state transition that led to the current state of the identity.
///
/// - `identities_inserts`: Defines the frequency distribution of identity inserts. `Frequency` might encapsulate statistical data like mean, median, variance, etc., for understanding or predicting the frequency of identity insertions.
///
/// - `signer`: An optional instance of `SimpleSigner`. The `SimpleSigner` is responsible for generating and managing cryptographic signatures, and might be used to authenticate or validate various operations or state transitions.
///
/// # Usage
/// ```rust
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
#[derive(Clone, Debug)]
pub struct Strategy {
    pub contracts_with_updates: Vec<(
        CreatedDataContract,
        Option<BTreeMap<u64, CreatedDataContract>>,
    )>,
    pub operations: Vec<Operation>,
    pub start_identities: Vec<(Identity, StateTransition)>,
    pub identities_inserts: Frequency,
    pub signer: Option<SimpleSigner>,
}


impl Strategy {

    /// Adds strategy contracts from the current operations into a specified Drive.
    ///
    /// This method iterates over the operations present in the current strategy. For each operation
    /// of type `Document`, it serializes the associated contract and applies it to the provided drive.
    ///
    /// # Parameters
    /// - `drive`: The Drive where contracts should be added.
    /// - `platform_version`: The current Platform version used for serializing the contract.
    ///
    /// # Panics
    /// This method may panic in the following situations:
    /// - If serialization of a contract fails.
    /// - If applying a contract with serialization to the drive fails.
    ///
    /// # Examples
    /// ```
    /// // Assuming `strategy` is an instance of `Strategy`
    /// // and `drive` and `platform_version` are appropriately initialized.
    /// strategy.add_strategy_contracts_into_drive(&drive, &platform_version);
    /// ```
    // TODO: This belongs to `DocumentOp`
    pub fn add_strategy_contracts_into_drive(
        &mut self,
        drive: &Drive,
        platform_version: &PlatformVersion,
    ) {
        for op in &self.operations {
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

    /// Generates state transitions for identities based on the block information provided.
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
    /// ```
    /// // Assuming `strategy` is an instance of `Strategy`,
    /// // and `block_info`, `signer`, `rng`, and `platform_version` are appropriately initialized.
    /// let state_transitions = strategy.identity_state_transitions_for_block(&block_info, &mut signer, &mut rng, &platform_version);
    /// ```
    pub fn identity_state_transitions_for_block(
        &self,
        block_info: &BlockInfo,
        signer: &mut SimpleSigner,
        rng: &mut StdRng,
        platform_version: &PlatformVersion,
    ) -> Vec<(Identity, StateTransition)> {
        let mut state_transitions = vec![];
        if block_info.height == 1 && !self.start_identities.is_empty() {
            state_transitions.append(&mut self.start_identities.clone());
        }
        let frequency = &self.identities_inserts;
        if frequency.check_hit(rng) {
            let count = frequency.events(rng);
            state_transitions.append(
                &mut crate::transitions::create_identities_state_transitions(
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

    /// Generates state transitions for data contracts based on the current set of identities.
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
    /// ```
    /// // Assuming `strategy` is an instance of `Strategy`,
    /// // and `current_identities`, `signer`, `rng`, and `platform_version` are appropriately initialized.
    /// let contract_transitions = strategy.contract_state_transitions(&current_identities, &signer, &mut rng, &platform_version);
    /// ```
    pub fn contract_state_transitions(
        &mut self,
        current_identities: &Vec<Identity>,
        signer: &SimpleSigner,
        rng: &mut StdRng,
        platform_version: &PlatformVersion,
    ) -> Vec<StateTransition> {
        self.contracts_with_updates
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
                self.operations.iter_mut().for_each(|operation| {
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

    /// Generates state transitions for updating data contracts based on the current set of identities and block height.
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
    /// ```
    /// // Assuming `strategy` is an instance of `Strategy`,
    /// // and `current_identities`, `block_height`, `signer`, and `platform_version` are appropriately initialized.
    /// let update_transitions = strategy.contract_update_state_transitions(&current_identities, block_height, &signer, &platform_version);
    /// ```
    pub fn contract_update_state_transitions(
        &mut self,
        current_identities: &Vec<Identity>,
        block_height: u64,
        signer: &SimpleSigner,
        platform_version: &PlatformVersion,
    ) -> Vec<StateTransition> {
        self.contracts_with_updates
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

                let state_transition = DataContractUpdateTransition::new_from_data_contract(
                    contract_update.data_contract().clone(),
                    &identity,
                    1, //key id 1 should always be a high or critical auth key in these tests
                    signer,
                    platform_version,
                    None,
                )
                .expect("expected to create a create state transition from a data contract");
                Some(state_transition)
            })
            .collect()
    }

    /// Generates state transitions for a given block.
    ///
    /// The `state_transitions_for_block` function processes state transitions based on the provided
    /// block information, platform, identities, and other input parameters. It facilitates
    /// the creation of state transitions for both new documents and updated documents in the system.
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
    /// ```rust
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
        drive: &Drive,
        block_info: &BlockInfo,
        current_identities: &mut Vec<Identity>,
        signer: &mut SimpleSigner,
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
            if op.frequency.check_hit(rng) {
                let count = rng.gen_range(op.frequency.times_per_block_range.clone());
                match &op.op_type {
                    // Generate state transition for document insert operation with random data
                    OperationType::Document(DocumentOp {
                        action: DocumentAction::DocumentActionInsertRandom(fill_type, fill_size),
                        document_type,
                        contract,
                    }) => {
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
                                let updated_at =
                                    if document_type.required_fields().contains("$updatedAt") {
                                        document.created_at()
                                    } else {
                                        None
                                    };
                                let document_create_transition: DocumentCreateTransition =
                                    DocumentCreateTransitionV0 {
                                        base: DocumentBaseTransitionV0 {
                                            id: document.id(),
                                            document_type_name: document_type.name().clone(),
                                            data_contract_id: contract.id(),
                                        }
                                        .into(),
                                        entropy: entropy.to_buffer(),
                                        created_at: document.created_at(),
                                        updated_at,
                                        data: document.properties_consumed(),
                                    }
                                    .into();

                                let document_batch_transition: DocumentsBatchTransition =
                                    DocumentsBatchTransitionV0 {
                                        owner_id: identity.id(),
                                        transitions: vec![document_create_transition.into()],
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
                                let updated_at =
                                    if document_type.required_fields().contains("$updatedAt") {
                                        document.created_at()
                                    } else {
                                        None
                                    };
                                let document_create_transition: DocumentCreateTransition =
                                    DocumentCreateTransitionV0 {
                                        base: DocumentBaseTransitionV0 {
                                            id: document.id(),
                                            document_type_name: document_type.name().clone(),
                                            data_contract_id: contract.id(),
                                        }
                                        .into(),
                                        entropy: entropy.to_buffer(),
                                        created_at: document.created_at(),
                                        updated_at,
                                        data: document.properties_consumed(),
                                    }
                                    .into();

                                let document_batch_transition: DocumentsBatchTransition =
                                    DocumentsBatchTransitionV0 {
                                        owner_id: identity.id(),
                                        transitions: vec![document_create_transition.into()],
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

                    // Generate state transition for document delete operation
                    OperationType::Document(DocumentOp {
                        action: DocumentAction::DocumentActionDelete,
                        document_type,
                        contract,
                    }) => {
                        let any_item_query =
                            DriveQuery::any_item_query(contract, document_type.as_ref());
                        let mut items = drive
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
                            let identity = drive
                                .fetch_identity_balance_with_keys(request, None, platform_version)
                                .expect("expected to be able to get identity")
                                .expect("expected to get an identity");
                            let document_delete_transition: DocumentDeleteTransition =
                                DocumentDeleteTransitionV0 {
                                    base: DocumentBaseTransitionV0 {
                                        id: document.id(),
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

                    // Generate state transition for document replace operation
                    OperationType::Document(DocumentOp {
                        action: DocumentAction::DocumentActionReplace,
                        document_type,
                        contract,
                    }) => {
                        let any_item_query =
                            DriveQuery::any_item_query(contract, document_type.as_ref());
                        let mut items = drive
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
                            let identity = drive
                                .fetch_identity_balance_with_keys(request, None, platform_version)
                                .expect("expected to be able to get identity")
                                .expect("expected to get an identity");
                            let document_replace_transition: DocumentReplaceTransition =
                                DocumentReplaceTransitionV0 {
                                    base: DocumentBaseTransitionV0 {
                                        id: document.id(),
                                        document_type_name: document_type.name().clone(),
                                        data_contract_id: contract.id(),
                                    }
                                    .into(),
                                    revision: document
                                        .revision()
                                        .expect("expected to unwrap revision")
                                        + 1,
                                    updated_at: Some(block_info.time_ms),
                                    data: random_new_document.properties_consumed(),
                                }
                                .into();

                            let document_batch_transition: DocumentsBatchTransition =
                                DocumentsBatchTransitionV0 {
                                    owner_id: identity.id,
                                    transitions: vec![document_replace_transition.into()],
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

                    // Generate state transition for identity top-up operation
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
                                platform_version,
                            ));
                        }
                    }

                    // Generate state transition for identity update operation
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
                                        crate::transitions::create_identity_update_transition_disable_keys(
                                            random_identity,
                                            *count,
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

                    // Generate state transition for identity withdrawal operation
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

                    // Generate state transition for identity transfer operation
                    OperationType::IdentityTransfer if current_identities.len() > 1 => {
                        // chose 2 last identities
                        let indices: Vec<usize> =
                            vec![current_identities.len() - 2, current_identities.len() - 1];

                        let owner = current_identities.get(indices[0]).unwrap();
                        let recipient = current_identities.get(indices[1]).unwrap();

                        let fetched_owner_balance = drive
                            .fetch_identity_balance(owner.id().to_buffer(), None, platform_version)
                            .expect("expected to be able to get identity")
                            .expect("expected to get an identity");

                        let state_transition =
                            crate::transitions::create_identity_credit_transfer_transition(
                                owner,
                                recipient,
                                signer,
                                fetched_owner_balance - 100,
                            );
                        operations.push(state_transition);
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

    /// Generates state transitions for a block by considering new identities.
    ///
    /// This function processes state transitions with respect to identities, contracts,
    /// and document operations. The state transitions are generated based on the 
    /// given block's height and other parameters, with special handling for block height `1`.
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
    /// ```rust
    /// let (state_transitions, finalize_ops) = obj.state_transitions_for_block_with_new_identities(
    ///     &platform,
    ///     &block_info,
    ///     &mut current_identities,
    ///     &mut signer,
    ///     &mut rng,
    ///     platform_version
    /// );
    /// ```
    pub fn state_transitions_for_block_with_new_identities(
        &mut self,
        drive: &Drive,
        block_info: &BlockInfo,
        current_identities: &mut Vec<Identity>,
        signer: &mut SimpleSigner,
        rng: &mut StdRng,
        platform_version: &PlatformVersion,
    ) -> (Vec<StateTransition>, Vec<FinalizeBlockOperation>) {
        let mut finalize_block_operations = vec![];
        let identity_state_transitions =
            self.identity_state_transitions_for_block(block_info, signer, rng, platform_version);
        let (mut identities, mut state_transitions): (Vec<Identity>, Vec<StateTransition>) =
            identity_state_transitions.into_iter().unzip();
        current_identities.append(&mut identities);

        if block_info.height == 1 {
            // add contracts on block 1
            let mut contract_state_transitions =
                self.contract_state_transitions(current_identities, signer, rng, platform_version);
            state_transitions.append(&mut contract_state_transitions);
        } else {
            // Don't do any state transitions on block 1
            let (mut document_state_transitions, mut add_to_finalize_block_operations) = self
                .state_transitions_for_block(
                    drive,
                    block_info,
                    current_identities,
                    signer,
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
                platform_version,
            );
            state_transitions.append(&mut contract_update_state_transitions);
        }

        (state_transitions, finalize_block_operations)
    }
}