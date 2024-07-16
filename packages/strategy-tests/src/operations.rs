use crate::frequency::Frequency;
use bincode::{Decode, Encode};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::random_document::{
    DocumentFieldFillSize, DocumentFieldFillType,
};
use dpp::data_contract::document_type::v0::random_document_type::RandomDocumentTypeParameters;
use dpp::data_contract::document_type::DocumentType;
use dpp::data_contract::serialized_version::DataContractInSerializationFormat;
use dpp::data_contract::{DataContract as Contract, DataContract};
use dpp::identifier::Identifier;
use dpp::identity::IdentityPublicKey;
use dpp::platform_value::Value;
use dpp::serialization::{
    PlatformDeserializableWithPotentialValidationFromVersionedStructure,
    PlatformSerializableWithPlatformVersion,
};
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
use std::ops::Range;

#[derive(Clone, Debug, PartialEq, Encode, Decode)]
pub enum DocumentAction {
    DocumentActionInsertRandom(DocumentFieldFillType, DocumentFieldFillSize),
    /// Insert a document with specific values
    /// If a required value is not set, it will use random ones
    /// The second parameter is the owner id of the document
    /// If none then it should be random
    DocumentActionInsertSpecific(
        BTreeMap<String, Value>,
        Option<Identifier>,
        DocumentFieldFillType,
        DocumentFieldFillSize,
    ),
    DocumentActionDelete,
    DocumentActionReplaceRandom,
    DocumentActionTransferRandom,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DocumentOp {
    pub contract: Contract,
    pub document_type: DocumentType,
    pub action: DocumentAction,
}

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

#[derive(Clone, Debug, PartialEq)]
pub struct Operation {
    pub op_type: OperationType,
    pub frequency: Frequency,
}

#[derive(Clone, Debug, Encode, Decode)]
pub struct OperationInSerializationFormat {
    pub op_type: Vec<u8>,
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

#[derive(Clone, Debug, PartialEq, Encode, Decode)]
pub enum IdentityUpdateOp {
    IdentityUpdateAddKeys(u16),
    IdentityUpdateDisableKey(u16),
}

pub type DocumentTypeNewFieldsOptionalCountRange = Range<u16>;
pub type DocumentTypeCount = Range<u16>;

#[derive(Clone, Debug, PartialEq)]
pub struct DataContractUpdateOp {
    pub action: DataContractUpdateAction,
    pub contract: DataContract,
    pub document_type: Option<DocumentType>,
}

#[derive(Clone, Debug, PartialEq, Encode, Decode)]
pub struct DataContractUpdateOpInSerializationFormat {
    action: DataContractUpdateAction,
    contract: DataContractInSerializationFormat,
    document_type: Option<Value>,
}

#[derive(Clone, Debug, PartialEq, Encode, Decode)]
pub enum DataContractUpdateAction {
    DataContractNewDocumentTypes(RandomDocumentTypeParameters), // How many fields should it have
    DataContractNewOptionalFields(DocumentTypeNewFieldsOptionalCountRange, DocumentTypeCount), // How many new fields on how many document types
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
                            let schema = Value::try_from(schema_json).unwrap();
                            let owner_id = contract.owner_id(); // Assuming you have a method to get the owner_id from the contract
                            DocumentType::try_from_schema(
                                owner_id,
                                name_str,
                                schema,
                                None,
                                true,
                                true,
                                true,
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

#[derive(Clone, Debug, PartialEq)]
pub struct ResourceVoteOp {
    pub resolved_vote_poll: ContestedDocumentResourceVotePollWithContractInfo,
    pub action: VoteAction,
}

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

#[derive(Clone, Debug, PartialEq, Encode, Decode)]
pub struct VoteAction {
    pub vote_choices_with_weights: Vec<(ResourceVoteChoice, u8)>,
}

impl VoteAction {
    // Function to choose a ResourceVoteChoice based on weights
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

#[derive(Clone, Debug, PartialEq)]
pub enum OperationType {
    Document(DocumentOp),
    IdentityTopUp,
    IdentityUpdate(IdentityUpdateOp),
    IdentityWithdrawal,
    ContractCreate(RandomDocumentTypeParameters, DocumentTypeCount),
    ContractUpdate(DataContractUpdateOp),
    IdentityTransfer,
    ResourceVote(ResourceVoteOp),
}

#[derive(Clone, Debug, Encode, Decode)]
enum OperationTypeInSerializationFormat {
    Document(Vec<u8>),
    IdentityTopUp,
    IdentityUpdate(IdentityUpdateOp),
    IdentityWithdrawal,
    ContractCreate(RandomDocumentTypeParameters, DocumentTypeCount),
    ContractUpdate(Vec<u8>),
    IdentityTransfer,
    ResourceVote(ResourceVoteOpSerializable),
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
            OperationType::IdentityTopUp => OperationTypeInSerializationFormat::IdentityTopUp,
            OperationType::IdentityUpdate(identity_update_op) => {
                OperationTypeInSerializationFormat::IdentityUpdate(identity_update_op)
            }
            OperationType::IdentityWithdrawal => {
                OperationTypeInSerializationFormat::IdentityWithdrawal
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
            OperationType::IdentityTransfer => OperationTypeInSerializationFormat::IdentityTransfer,
            OperationType::ResourceVote(resource_vote_op) => {
                let vote_op_in_serialization_format =
                    resource_vote_op.try_into_platform_versioned(platform_version)?;
                OperationTypeInSerializationFormat::ResourceVote(vote_op_in_serialization_format)
            }
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
            OperationTypeInSerializationFormat::IdentityTopUp => OperationType::IdentityTopUp,
            OperationTypeInSerializationFormat::IdentityUpdate(identity_update_op) => {
                OperationType::IdentityUpdate(identity_update_op)
            }
            OperationTypeInSerializationFormat::IdentityWithdrawal => {
                OperationType::IdentityWithdrawal
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
            OperationTypeInSerializationFormat::IdentityTransfer => OperationType::IdentityTransfer,
            OperationTypeInSerializationFormat::ResourceVote(resource_vote_op) => {
                let vote_op = resource_vote_op.try_into_platform_versioned(platform_version)?;
                OperationType::ResourceVote(vote_op)
            }
        })
    }
}

#[derive(Clone, Debug, Encode, Decode)]
pub enum FinalizeBlockOperation {
    IdentityAddKeys(Identifier, Vec<IdentityPublicKey>),
}
