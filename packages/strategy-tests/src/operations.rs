//! Operation types for strategy-based platform testing.
//!
//! This module defines the various operations that can be performed during strategy tests,
//! including document operations, identity management, contract updates, voting, token
//! operations, and address-based fund transfers.
//!
//! # Overview
//!
//! Strategy tests simulate realistic platform usage by executing randomized sequences
//! of operations. Each operation type represents a different kind of platform interaction:
//!
//! - **Document operations**: Create, update, delete, and transfer documents
//! - **Identity operations**: Top-up, withdrawal, key management, transfers
//! - **Contract operations**: Create and update data contracts
//! - **Token operations**: Token-related events (mint, burn, transfer, etc.)
//! - **Voting operations**: Resource voting with weighted choices
//! - **Address operations**: Fund addresses, transfer between addresses, withdraw
//!
//! # Operation Structure
//!
//! Each operation is wrapped in an [`Operation`] struct that pairs the operation type
//! with a [`Frequency`] configuration, controlling how often the operation occurs
//! during test execution.
//!
//! # Serialization
//!
//! All operation types implement platform serialization traits for persistence and
//! transmission. Internal `*InSerializationFormat` variants handle the conversion
//! between runtime types (which may contain non-serializable references) and
//! serializable representations.

use crate::frequency::Frequency;
use crate::KeyMaps;
use bincode::{Decode, Encode};
use dpp::address_funds::{AddressFundsFeeStrategy, PlatformAddress};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::accessors::v1::DataContractV1Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::random_document::{
    DocumentFieldFillSize, DocumentFieldFillType,
};
use dpp::data_contract::document_type::v0::random_document_type::RandomDocumentTypeParameters;
use dpp::data_contract::document_type::DocumentType;
use dpp::data_contract::serialized_version::DataContractInSerializationFormat;
use dpp::data_contract::{DataContract as Contract, DataContract, TokenContractPosition};
use dpp::fee::Credits;
use dpp::identifier::Identifier;
use dpp::identity::{IdentityPublicKey, KeyCount};
use dpp::platform_value::Value;
use dpp::serialization::{
    PlatformDeserializableWithPotentialValidationFromVersionedStructure,
    PlatformSerializableWithPlatformVersion,
};
use dpp::tokens::token_event::TokenEvent;
use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use dpp::ProtocolError;
use dpp::ProtocolError::{PlatformDeserializationError, PlatformSerializationError};
use drive::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfo;
use drive::util::object_size_info::DataContractOwnedResolvedInfo;
use platform_version::version::PlatformVersion;
use platform_version::{TryFromPlatformVersioned, TryIntoPlatformVersioned};
use rand::distributions::{Distribution, WeightedIndex};
use rand::prelude::StdRng;
use std::collections::BTreeMap;
use std::ops::{Range, RangeInclusive};

/// A token operation to be executed during strategy tests.
///
/// Represents actions on platform tokens such as minting, burning, transferring,
/// or other token events defined by the contract.
#[derive(Clone, Debug, PartialEq)]
pub struct TokenOp {
    /// The data contract that defines this token.
    pub contract: Contract,
    /// The unique identifier of the token.
    pub token_id: Identifier,
    /// The position of this token within the contract's token definitions.
    pub token_pos: TokenContractPosition,
    /// Optional specific identity to use for this operation.
    /// If `None`, a random identity may be selected.
    pub use_identity_with_id: Option<Identifier>,
    /// The token event to execute (mint, burn, transfer, etc.).
    pub action: TokenEvent,
}

/// Serialization format for [`TokenOp`].
///
/// Converts the contract to its serializable representation while preserving
/// all other fields directly.
#[derive(Clone, Debug, Encode, Decode)]
pub struct TokenOpInSerializationFormat {
    pub contract: DataContractInSerializationFormat,
    pub token_id: Identifier,
    pub token_pos: TokenContractPosition,
    pub use_identity_with_id: Option<Identifier>,
    pub action: TokenEvent,
}

impl PlatformSerializableWithPlatformVersion for TokenOp {
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
        let TokenOp {
            contract,
            token_id,
            token_pos,
            use_identity_with_id,
            action,
        } = self;
        let data_contract_serialization_format: DataContractInSerializationFormat =
            contract.try_into_platform_versioned(platform_version)?;

        let document_op = TokenOpInSerializationFormat {
            contract: data_contract_serialization_format,
            token_id,
            token_pos,
            use_identity_with_id,
            action,
        };
        let config = bincode::config::standard()
            .with_big_endian()
            .with_no_limit();
        bincode::encode_to_vec(document_op, config).map_err(|e| {
            PlatformSerializationError(format!("unable to serialize DocumentOp: {}", e))
        })
    }
}

impl PlatformDeserializableWithPotentialValidationFromVersionedStructure for TokenOp {
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
        let token_op_in_serialization_format: TokenOpInSerializationFormat =
            bincode::borrow_decode_from_slice(data, config)
                .map_err(|e| {
                    PlatformDeserializationError(format!("unable to deserialize DocumentOp: {}", e))
                })?
                .0;
        let TokenOpInSerializationFormat {
            contract,
            token_id,
            token_pos,
            use_identity_with_id,
            action,
        } = token_op_in_serialization_format;
        let data_contract = DataContract::try_from_platform_versioned(
            contract,
            full_validation,
            &mut vec![],
            platform_version,
        )?;
        Ok(TokenOp {
            contract: data_contract,
            token_id,
            token_pos,
            use_identity_with_id,
            action,
        })
    }
}

/// Actions that can be performed on documents during strategy tests.
///
/// Each variant represents a different document state transition that will
/// be submitted to the platform.
#[derive(Clone, Debug, PartialEq, Encode, Decode)]
pub enum DocumentAction {
    /// Insert a new document with randomly generated field values.
    ///
    /// Parameters control how fields are filled (type and size constraints).
    DocumentActionInsertRandom(DocumentFieldFillType, DocumentFieldFillSize),

    /// Insert a document with specific field values.
    ///
    /// - First parameter: Map of field names to their values
    /// - Second parameter: Optional owner identity ID (random if `None`)
    /// - Third/Fourth parameters: Fill type and size for any unspecified required fields
    DocumentActionInsertSpecific(
        BTreeMap<String, Value>,
        Option<Identifier>,
        DocumentFieldFillType,
        DocumentFieldFillSize,
    ),

    /// Delete an existing document.
    DocumentActionDelete,

    /// Replace an existing document with new random field values.
    DocumentActionReplaceRandom,

    /// Transfer document ownership to a random identity.
    DocumentActionTransferRandom,
}

/// A document operation to be executed during strategy tests.
///
/// Combines a target contract, document type, and action to form a complete
/// document state transition that can be submitted to the platform.
#[derive(Clone, Debug, PartialEq)]
pub struct DocumentOp {
    /// The data contract containing the document type definition.
    pub contract: Contract,
    /// The specific document type within the contract.
    pub document_type: DocumentType,
    /// The action to perform (insert, delete, replace, transfer).
    pub action: DocumentAction,
}

/// Serialization format for [`DocumentOp`].
///
/// Stores the document type by name rather than the full type definition,
/// which is reconstructed from the contract during deserialization.
#[derive(Clone, Debug, Encode, Decode)]
pub struct DocumentOpInSerializationFormat {
    pub contract: DataContractInSerializationFormat,
    pub document_type_name: String,
    pub action: DocumentAction,
}
impl PlatformSerializableWithPlatformVersion for DocumentOp {
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
        let DocumentOp {
            contract,
            document_type,
            action,
        } = self;
        let data_contract_serialization_format: DataContractInSerializationFormat =
            contract.try_into_platform_versioned(platform_version)?;

        let document_op = DocumentOpInSerializationFormat {
            contract: data_contract_serialization_format,
            document_type_name: document_type.name().clone(),
            action,
        };
        let config = bincode::config::standard()
            .with_big_endian()
            .with_no_limit();
        bincode::encode_to_vec(document_op, config).map_err(|e| {
            PlatformSerializationError(format!("unable to serialize DocumentOp: {}", e))
        })
    }
}

impl PlatformDeserializableWithPotentialValidationFromVersionedStructure for DocumentOp {
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
        let document_op_in_serialization_format: DocumentOpInSerializationFormat =
            bincode::borrow_decode_from_slice(data, config)
                .map_err(|e| {
                    PlatformDeserializationError(format!("unable to deserialize DocumentOp: {}", e))
                })?
                .0;
        let DocumentOpInSerializationFormat {
            contract,
            document_type_name,
            action,
        } = document_op_in_serialization_format;
        let data_contract = DataContract::try_from_platform_versioned(
            contract,
            full_validation,
            &mut vec![],
            platform_version,
        )?;
        let document_type =
            data_contract.document_type_cloned_for_name(document_type_name.as_str())?;
        Ok(DocumentOp {
            contract: data_contract,
            document_type,
            action,
        })
    }
}

/// A complete operation definition combining type and frequency.
///
/// This is the primary unit of work in strategy tests. Each operation
/// specifies what action to perform ([`OperationType`]) and how often
/// to perform it ([`Frequency`]).
///
/// During test execution, operations are evaluated each block according
/// to their frequency configuration to determine if and how many times
/// they should be executed.
#[derive(Clone, Debug, PartialEq)]
pub struct Operation {
    /// The type of operation to perform.
    pub op_type: OperationType,
    /// Configuration controlling how often this operation occurs.
    pub frequency: Frequency,
}

/// Serialization format for [`Operation`].
#[derive(Clone, Debug, Encode, Decode)]
pub struct OperationInSerializationFormat {
    /// Serialized operation type bytes.
    pub op_type: Vec<u8>,
    /// Frequency configuration (directly serializable).
    pub frequency: Frequency,
}

impl PlatformSerializableWithPlatformVersion for Operation {
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
        let Operation { op_type, frequency } = self;
        let op_type_serialized =
            op_type.serialize_consume_to_bytes_with_platform_version(platform_version)?;

        let operation = OperationInSerializationFormat {
            op_type: op_type_serialized,
            frequency,
        };
        let config = bincode::config::standard()
            .with_big_endian()
            .with_no_limit();
        bincode::encode_to_vec(operation, config).map_err(|e| {
            PlatformSerializationError(format!("unable to serialize Operation: {}", e))
        })
    }
}

impl PlatformDeserializableWithPotentialValidationFromVersionedStructure for Operation {
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
        let operation_in_serialization_format: OperationInSerializationFormat =
            bincode::borrow_decode_from_slice(data, config)
                .map_err(|e| {
                    PlatformDeserializationError(format!("unable to deserialize DocumentOp: {}", e))
                })?
                .0;
        let OperationInSerializationFormat { op_type, frequency } =
            operation_in_serialization_format;
        let op_type = OperationType::versioned_deserialize(
            op_type.as_slice(),
            full_validation,
            platform_version,
        )?;
        Ok(Operation { op_type, frequency })
    }
}

/// Identity update operations for modifying identity keys.
#[derive(Clone, Debug, PartialEq, Encode, Decode)]
pub enum IdentityUpdateOp {
    /// Add new keys to an identity. Parameter is the number of keys to add.
    IdentityUpdateAddKeys(u16),
    /// Disable an existing key. Parameter is the key index to disable.
    IdentityUpdateDisableKey(u16),
}

/// Range for the number of optional fields to add to a document type.
pub type DocumentTypeNewFieldsOptionalCountRange = Range<u16>;

/// Range for how many document types to affect in an update operation.
pub type DocumentTypeCount = Range<u16>;

/// A data contract update operation for strategy tests.
///
/// Represents modifications to an existing data contract, such as adding
/// new document types or extending existing ones with new fields.
#[derive(Clone, Debug, PartialEq)]
pub struct DataContractUpdateOp {
    /// The type of update to perform.
    pub action: DataContractUpdateAction,
    /// The contract to update.
    pub contract: DataContract,
    /// Optional document type context for field-level updates.
    pub document_type: Option<DocumentType>,
}

/// Serialization format for [`DataContractUpdateOp`].
#[derive(Clone, Debug, PartialEq, Encode, Decode)]
pub struct DataContractUpdateOpInSerializationFormat {
    action: DataContractUpdateAction,
    contract: DataContractInSerializationFormat,
    document_type: Option<Value>,
}

/// Types of updates that can be performed on a data contract.
#[derive(Clone, Debug, PartialEq, Encode, Decode)]
pub enum DataContractUpdateAction {
    /// Add new document types to the contract.
    /// Parameter specifies how many fields the new types should have.
    DataContractNewDocumentTypes(RandomDocumentTypeParameters),

    /// Add new optional fields to existing document types.
    /// First parameter: range for number of fields to add per type.
    /// Second parameter: range for how many document types to modify.
    DataContractNewOptionalFields(DocumentTypeNewFieldsOptionalCountRange, DocumentTypeCount),
}

impl PlatformSerializableWithPlatformVersion for DataContractUpdateOp {
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
        let DataContractUpdateOp {
            action,
            contract,
            document_type,
        } = self;

        // Serialize contract and optionally document type
        let contract_in_serialization_format: DataContractInSerializationFormat =
            contract.try_into_platform_versioned(platform_version)?;

        // Convert DocumentType to its serializable schema representation
        let document_type_in_serialization_format = document_type.map(|dt| {
            // Assuming `schema_owned` or a similar method returns a serializable representation
            dt.schema_owned()
        });

        let update_op_in_serialization_format = DataContractUpdateOpInSerializationFormat {
            action,
            contract: contract_in_serialization_format,
            document_type: document_type_in_serialization_format,
        };

        let config = bincode::config::standard()
            .with_big_endian()
            .with_no_limit();
        bincode::encode_to_vec(update_op_in_serialization_format, config).map_err(|e| {
            PlatformSerializationError(format!("Unable to serialize DataContractUpdateOp: {}", e))
        })
    }
}

impl PlatformDeserializableWithPotentialValidationFromVersionedStructure for DataContractUpdateOp {
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
        let deserialized: DataContractUpdateOpInSerializationFormat =
            bincode::borrow_decode_from_slice(data, config)
                .map_err(|e| {
                    PlatformDeserializationError(format!(
                        "Unable to deserialize DataContractUpdateOp: {}",
                        e
                    ))
                })?
                .0;

        let contract = DataContract::try_from_platform_versioned(
            deserialized.contract,
            full_validation,
            &mut vec![],
            platform_version,
        )?;

        let action = deserialized.action;

        let document_type = deserialized.document_type.and_then(|value| {
            match value {
                Value::Map(map) => {
                    map.into_iter()
                        .map(|(name, schema_json)| {
                            let name_str = name.to_str().expect(
                                "Couldn't convert document type name to str in deserialization",
                            );
                            let owner_id = contract.owner_id(); // Assuming you have a method to get the owner_id from the contract
                            DocumentType::try_from_schema(
                                owner_id,
                                contract.system_version_type(),
                                contract.config().version(),
                                name_str,
                                schema_json,
                                None,
                                contract.tokens(),
                                contract.config(),
                                full_validation,
                                &mut vec![],
                                platform_version,
                            )
                            .expect("Failed to reconstruct DocumentType from schema")
                        })
                        .next() // Assumes only one document type is being deserialized
                }
                _ => None,
            }
        });

        Ok(DataContractUpdateOp {
            action,
            contract,
            document_type,
        })
    }
}

/// Serializable version of a contested document resource vote poll.
///
/// Used for persisting vote poll information that references a specific
/// document resource being contested in a vote.
#[derive(Debug, PartialEq, Clone, Encode, Decode)]
pub struct ContestedDocumentResourceVotePollWithSerializableContract {
    /// The contract information associated with the document.
    pub contract: DataContractInSerializationFormat,
    /// The name of the document type.
    pub document_type_name: String,
    /// The name of the index.
    pub index_name: String,
    /// The values used in the index for the poll.
    pub index_values: Vec<Value>,
}

impl TryFromPlatformVersioned<ContestedDocumentResourceVotePollWithContractInfo>
    for ContestedDocumentResourceVotePollWithSerializableContract
{
    type Error = ProtocolError;
    fn try_from_platform_versioned(
        value: ContestedDocumentResourceVotePollWithContractInfo,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Self::Error> {
        let ContestedDocumentResourceVotePollWithContractInfo {
            contract,
            document_type_name,
            index_name,
            index_values,
        } = value;
        Ok(ContestedDocumentResourceVotePollWithSerializableContract {
            contract: contract
                .into_owned()
                .try_into_platform_versioned(platform_version)?,
            document_type_name,
            index_name,
            index_values,
        })
    }
}

impl TryFromPlatformVersioned<ContestedDocumentResourceVotePollWithSerializableContract>
    for ContestedDocumentResourceVotePollWithContractInfo
{
    type Error = ProtocolError;
    fn try_from_platform_versioned(
        value: ContestedDocumentResourceVotePollWithSerializableContract,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Self::Error> {
        let ContestedDocumentResourceVotePollWithSerializableContract {
            contract,
            document_type_name,
            index_name,
            index_values,
        } = value;
        Ok(ContestedDocumentResourceVotePollWithContractInfo {
            contract: DataContractOwnedResolvedInfo::OwnedDataContract(
                DataContract::try_from_platform_versioned(
                    contract,
                    false,
                    &mut vec![],
                    platform_version,
                )?,
            ),
            document_type_name,
            index_name,
            index_values,
        })
    }
}

/// A resource voting operation for strategy tests.
///
/// Represents a vote on a contested document resource, combining the
/// vote poll context with the voting action to perform.
#[derive(Clone, Debug, PartialEq)]
pub struct ResourceVoteOp {
    /// The vote poll being voted on, with full contract information.
    pub resolved_vote_poll: ContestedDocumentResourceVotePollWithContractInfo,
    /// The voting action with weighted choices.
    pub action: VoteAction,
}

/// Serialization format for [`ResourceVoteOp`].
#[derive(Clone, Debug, PartialEq, Encode, Decode)]
pub struct ResourceVoteOpSerializable {
    pub resolved_vote_poll: ContestedDocumentResourceVotePollWithSerializableContract,
    pub action: VoteAction,
}

impl TryFromPlatformVersioned<ResourceVoteOpSerializable> for ResourceVoteOp {
    type Error = ProtocolError;

    fn try_from_platform_versioned(
        value: ResourceVoteOpSerializable,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Self::Error> {
        let ResourceVoteOpSerializable {
            resolved_vote_poll,
            action,
        } = value;

        Ok(ResourceVoteOp {
            resolved_vote_poll: resolved_vote_poll.try_into_platform_versioned(platform_version)?,
            action,
        })
    }
}

impl TryFromPlatformVersioned<ResourceVoteOp> for ResourceVoteOpSerializable {
    type Error = ProtocolError;

    fn try_from_platform_versioned(
        value: ResourceVoteOp,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Self::Error> {
        let ResourceVoteOp {
            resolved_vote_poll,
            action,
        } = value;

        Ok(ResourceVoteOpSerializable {
            resolved_vote_poll: resolved_vote_poll.try_into_platform_versioned(platform_version)?,
            action,
        })
    }
}

/// A voting action with weighted choice probabilities.
///
/// Allows configuring a distribution of vote choices where each choice
/// has an associated weight determining its likelihood of being selected.
#[derive(Clone, Debug, PartialEq, Encode, Decode)]
pub struct VoteAction {
    /// Pairs of (vote choice, weight) defining the probability distribution.
    /// Higher weights increase the likelihood of that choice being selected.
    pub vote_choices_with_weights: Vec<(ResourceVoteChoice, u8)>,
}

impl VoteAction {
    /// Selects a vote choice randomly based on the configured weights.
    ///
    /// Uses weighted random selection where choices with higher weights
    /// are proportionally more likely to be chosen. Returns `Abstain`
    /// if no choices are configured.
    pub fn choose_weighted_choice(&self, rng: &mut StdRng) -> ResourceVoteChoice {
        if self.vote_choices_with_weights.is_empty() {
            ResourceVoteChoice::Abstain
        } else if self.vote_choices_with_weights.len() == 1 {
            self.vote_choices_with_weights[0].0
        } else {
            let weights: Vec<u8> = self
                .vote_choices_with_weights
                .iter()
                .map(|(_, weight)| *weight)
                .collect();
            let dist = WeightedIndex::new(weights).unwrap();
            let index = dist.sample(rng);
            self.vote_choices_with_weights[index].0
        }
    }
}

/// Inclusive range for credit amounts in operations.
///
/// Used to specify minimum and maximum amounts for transfers, top-ups,
/// withdrawals, and other credit-based operations.
pub type AmountRange = RangeInclusive<Credits>;

/// Inclusive range for the number of outputs in a transaction.
pub type OutputCountRange = RangeInclusive<u8>;

/// Optional amount range for operation outputs.
///
/// When `Some`, specifies the range for output amounts.
/// When `None`, the operation may use default behavior.
pub type MaybeOutputAmount = Option<AmountRange>;

/// Probability (0.0 to 1.0) of reusing existing addresses as outputs.
///
/// When `Some(p)`, there's a `p` probability that existing addresses
/// will be used as transaction outputs instead of generating new ones.
/// When `None`, new addresses are always generated.
pub type UseExistingAddressesAsOutputChance = Option<f64>;

/// Additional keys to create alongside new identities.
pub type ExtraKeys = KeyMaps;

/// Information for a direct identity-to-identity credit transfer.
#[derive(Clone, Debug, PartialEq, Encode, Decode)]
pub struct IdentityTransferInfo {
    /// Source identity identifier.
    pub from: Identifier,
    /// Destination identity identifier.
    pub to: Identifier,
    /// Amount of credits to transfer.
    pub amount: Credits,
}

/// Information for transferring credits from an identity to platform addresses.
#[derive(Clone, Debug, PartialEq, Encode, Decode)]
pub struct IdentityTransferToAddresses {
    /// Source identity identifier.
    pub from: Identifier,
    /// Map of destination addresses to their respective credit amounts.
    pub outputs: BTreeMap<PlatformAddress, Credits>,
    /// Total amount being transferred.
    pub amount: Credits,
}

/// The type of operation to perform during strategy tests.
///
/// Each variant represents a different platform interaction, from document
/// management to identity operations, contract updates, and fund transfers.
#[derive(Clone, Debug, PartialEq)]
pub enum OperationType {
    /// Perform a document operation (insert, delete, replace, transfer).
    Document(DocumentOp),

    /// Top up an identity's credit balance from core chain funds.
    /// Parameter specifies the range of credits to add.
    IdentityTopUp(AmountRange),

    /// Update an identity (add/disable keys).
    IdentityUpdate(IdentityUpdateOp),

    /// Withdraw credits from an identity back to core chain.
    /// Parameter specifies the range of credits to withdraw.
    IdentityWithdrawal(AmountRange),

    /// Create a new data contract with random document types.
    /// First parameter: configuration for document type generation.
    /// Second parameter: range for number of document types to create.
    ContractCreate(RandomDocumentTypeParameters, DocumentTypeCount),

    /// Update an existing data contract.
    ContractUpdate(DataContractUpdateOp),

    /// Transfer credits between identities.
    /// If `None`, random source/destination identities are selected.
    IdentityTransfer(Option<IdentityTransferInfo>),

    /// Cast a vote on a contested resource.
    ResourceVote(ResourceVoteOp),

    /// Perform a token operation (mint, burn, transfer, etc.).
    Token(TokenOp),

    /// Top up an identity using funds from platform addresses.
    IdentityTopUpFromAddresses(AmountRange),

    /// Fund a platform address via core chain asset lock.
    AddressFundingFromCoreAssetLock(AmountRange),

    /// Transfer credits between platform addresses.
    /// Parameters: amount range, output count, reuse chance, fee strategy.
    AddressTransfer(
        AmountRange,
        OutputCountRange,
        UseExistingAddressesAsOutputChance,
        Option<AddressFundsFeeStrategy>,
    ),

    /// Withdraw credits from a platform address to core chain.
    /// Parameters: amount range, optional output amount, fee strategy.
    AddressWithdrawal(
        AmountRange,
        MaybeOutputAmount,
        Option<AddressFundsFeeStrategy>,
    ),

    /// Transfer credits from an identity to platform addresses.
    /// Parameters: amount range, output count, reuse chance, optional specific transfer info.
    IdentityTransferToAddresses(
        AmountRange,
        OutputCountRange,
        UseExistingAddressesAsOutputChance,
        Option<IdentityTransferToAddresses>,
    ),

    /// Create a new identity funded from platform addresses.
    /// Parameters: amount range, output amount, fee strategy, key count, extra keys.
    IdentityCreateFromAddresses(
        AmountRange,
        MaybeOutputAmount,
        Option<AddressFundsFeeStrategy>,
        KeyCount,
        ExtraKeys,
    ),
}

#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, Encode, Decode)]
enum OperationTypeInSerializationFormat {
    Document(Vec<u8>),
    IdentityTopUp(AmountRange),
    IdentityUpdate(IdentityUpdateOp),
    IdentityWithdrawal(AmountRange),
    ContractCreate(RandomDocumentTypeParameters, DocumentTypeCount),
    ContractUpdate(Vec<u8>),
    IdentityTransfer(Option<IdentityTransferInfo>),
    ResourceVote(ResourceVoteOpSerializable),
    Token(Vec<u8>),
    IdentityTopUpFromAddresses(AmountRange),
    AddressFundingFromCoreAssetLock(AmountRange),
    AddressTransfer(
        AmountRange,
        OutputCountRange,
        UseExistingAddressesAsOutputChance,
        Option<AddressFundsFeeStrategy>,
    ),
    AddressWithdrawal(
        AmountRange,
        MaybeOutputAmount,
        Option<AddressFundsFeeStrategy>,
    ),
    IdentityTransferToAddresses(
        AmountRange,
        OutputCountRange,
        UseExistingAddressesAsOutputChance,
        Option<IdentityTransferToAddresses>,
    ),
    IdentityCreateFromAddresses(
        AmountRange,
        MaybeOutputAmount,
        Option<AddressFundsFeeStrategy>,
        KeyCount,
        ExtraKeys,
    ),
}

impl PlatformSerializableWithPlatformVersion for OperationType {
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
        let op = match self {
            OperationType::Document(document_op) => {
                // let's just serialize it to make things easier
                let document_op_in_serialization_format = document_op
                    .serialize_consume_to_bytes_with_platform_version(platform_version)?;
                OperationTypeInSerializationFormat::Document(document_op_in_serialization_format)
            }
            OperationType::IdentityTopUp(amount_range) => {
                OperationTypeInSerializationFormat::IdentityTopUp(amount_range)
            }
            OperationType::IdentityTopUpFromAddresses(amount_range) => {
                OperationTypeInSerializationFormat::IdentityTopUpFromAddresses(amount_range)
            }
            OperationType::IdentityUpdate(identity_update_op) => {
                OperationTypeInSerializationFormat::IdentityUpdate(identity_update_op)
            }
            OperationType::IdentityWithdrawal(amount_range) => {
                OperationTypeInSerializationFormat::IdentityWithdrawal(amount_range)
            }
            OperationType::ContractCreate(p, c) => {
                OperationTypeInSerializationFormat::ContractCreate(p, c)
            }
            OperationType::ContractUpdate(update_op) => {
                // let's just serialize it to make things easier
                let contract_op_in_serialization_format =
                    update_op.serialize_consume_to_bytes_with_platform_version(platform_version)?;
                OperationTypeInSerializationFormat::ContractUpdate(
                    contract_op_in_serialization_format,
                )
            }
            OperationType::IdentityTransfer(identity_transfer_info) => {
                OperationTypeInSerializationFormat::IdentityTransfer(identity_transfer_info)
            }
            OperationType::ResourceVote(resource_vote_op) => {
                let vote_op_in_serialization_format =
                    resource_vote_op.try_into_platform_versioned(platform_version)?;
                OperationTypeInSerializationFormat::ResourceVote(vote_op_in_serialization_format)
            }
            OperationType::Token(token_op) => {
                let token_op_in_serialization_format =
                    token_op.serialize_consume_to_bytes_with_platform_version(platform_version)?;
                OperationTypeInSerializationFormat::Token(token_op_in_serialization_format)
            }
            OperationType::AddressFundingFromCoreAssetLock(amount_range) => {
                OperationTypeInSerializationFormat::AddressFundingFromCoreAssetLock(amount_range)
            }
            OperationType::AddressTransfer(
                amount_range,
                output_count_range,
                use_existing,
                fee_strategy,
            ) => OperationTypeInSerializationFormat::AddressTransfer(
                amount_range,
                output_count_range,
                use_existing,
                fee_strategy,
            ),
            OperationType::AddressWithdrawal(amount_range, maybe_output_amount, fee_strategy) => {
                OperationTypeInSerializationFormat::AddressWithdrawal(
                    amount_range,
                    maybe_output_amount,
                    fee_strategy,
                )
            }
            OperationType::IdentityTransferToAddresses(
                amount_range,
                output_count_range,
                use_existing,
                transfer_to_address_op,
            ) => OperationTypeInSerializationFormat::IdentityTransferToAddresses(
                amount_range,
                output_count_range,
                use_existing,
                transfer_to_address_op,
            ),
            OperationType::IdentityCreateFromAddresses(
                amount_range,
                maybe_output_amount,
                fee_strategy,
                key_count,
                extra_keys,
            ) => OperationTypeInSerializationFormat::IdentityCreateFromAddresses(
                amount_range,
                maybe_output_amount,
                fee_strategy,
                key_count,
                extra_keys,
            ),
        };
        let config = bincode::config::standard()
            .with_big_endian()
            .with_no_limit();
        bincode::encode_to_vec(op, config).map_err(|e| {
            PlatformSerializationError(format!("unable to serialize OperationType: {}", e))
        })
    }
}

impl PlatformDeserializableWithPotentialValidationFromVersionedStructure for OperationType {
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
        let operation_type: OperationTypeInSerializationFormat =
            bincode::borrow_decode_from_slice(data, config)
                .map_err(|e| {
                    PlatformDeserializationError(format!("unable to deserialize DocumentOp: {}", e))
                })?
                .0;
        Ok(match operation_type {
            OperationTypeInSerializationFormat::Document(serialized_op) => {
                let document_op = DocumentOp::versioned_deserialize(
                    serialized_op.as_slice(),
                    full_validation,
                    platform_version,
                )?;
                OperationType::Document(document_op)
            }
            OperationTypeInSerializationFormat::IdentityTopUp(amount_range) => {
                OperationType::IdentityTopUp(amount_range)
            }
            OperationTypeInSerializationFormat::IdentityTopUpFromAddresses(amount_range) => {
                OperationType::IdentityTopUpFromAddresses(amount_range)
            }
            OperationTypeInSerializationFormat::IdentityUpdate(identity_update_op) => {
                OperationType::IdentityUpdate(identity_update_op)
            }
            OperationTypeInSerializationFormat::IdentityWithdrawal(amount_range) => {
                OperationType::IdentityWithdrawal(amount_range)
            }
            OperationTypeInSerializationFormat::ContractCreate(p, c) => {
                OperationType::ContractCreate(p, c)
            }
            OperationTypeInSerializationFormat::ContractUpdate(serialized_op) => {
                let update_op = DataContractUpdateOp::versioned_deserialize(
                    serialized_op.as_slice(),
                    full_validation,
                    platform_version,
                )?;
                OperationType::ContractUpdate(update_op)
            }
            OperationTypeInSerializationFormat::IdentityTransfer(identity_transfer_info) => {
                OperationType::IdentityTransfer(identity_transfer_info)
            }
            OperationTypeInSerializationFormat::ResourceVote(resource_vote_op) => {
                let vote_op = resource_vote_op.try_into_platform_versioned(platform_version)?;
                OperationType::ResourceVote(vote_op)
            }
            OperationTypeInSerializationFormat::Token(serialized_token_op) => {
                let token_op = TokenOp::versioned_deserialize(
                    serialized_token_op.as_slice(),
                    full_validation,
                    platform_version,
                )?;
                OperationType::Token(token_op)
            }
            OperationTypeInSerializationFormat::AddressFundingFromCoreAssetLock(amount_range) => {
                OperationType::AddressFundingFromCoreAssetLock(amount_range)
            }
            OperationTypeInSerializationFormat::AddressTransfer(
                amount_range,
                output_count_range,
                use_existing,
                fee_strategy,
            ) => OperationType::AddressTransfer(
                amount_range,
                output_count_range,
                use_existing,
                fee_strategy,
            ),
            OperationTypeInSerializationFormat::AddressWithdrawal(
                amount_range,
                maybe_output_amount,
                fee_strategy,
            ) => OperationType::AddressWithdrawal(amount_range, maybe_output_amount, fee_strategy),
            OperationTypeInSerializationFormat::IdentityTransferToAddresses(
                amount_range,
                output_count_range,
                use_existing,
                transfer_to_address_op,
            ) => OperationType::IdentityTransferToAddresses(
                amount_range,
                output_count_range,
                use_existing,
                transfer_to_address_op,
            ),
            OperationTypeInSerializationFormat::IdentityCreateFromAddresses(
                amount_range,
                maybe_output_amount,
                fee_strategy,
                key_count,
                extra_keys,
            ) => OperationType::IdentityCreateFromAddresses(
                amount_range,
                maybe_output_amount,
                fee_strategy,
                key_count,
                extra_keys,
            ),
        })
    }
}

/// Operations that execute during block finalization.
///
/// These operations are deferred until the block is being finalized,
/// typically for actions that depend on the block's final state.
#[derive(Clone, Debug, Encode, Decode)]
pub enum FinalizeBlockOperation {
    /// Add keys to an identity during block finalization.
    /// Parameters: identity ID, list of public keys to add.
    IdentityAddKeys(Identifier, Vec<IdentityPublicKey>),
}
