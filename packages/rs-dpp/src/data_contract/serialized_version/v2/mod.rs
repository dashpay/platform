//! Format version 2 of a data contract: the stored format, written only to state, never
//! carried by a state transition. It holds format version 1's fields, raw document schemas
//! included, and beside them each document type compiled
//! ([`DocumentTypeInSerializationFormat`]), so a node reading its state, or a client verifying a
//! proof of it, reads the document types back instead of parsing their schemas. A reader that
//! asks for full validation, or decodes bytes nothing vouches for with the untrusted decoders,
//! parses the schemas instead.

use crate::block::epoch::EpochIndex;
use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::associated_token::token_configuration::TokenConfiguration;
use crate::data_contract::config::v1::DataContractConfigV1;
use crate::data_contract::config::DataContractConfig;
use crate::data_contract::document_type::serialized_version::DocumentTypeInSerializationFormat;
use crate::data_contract::group::Group;
use crate::data_contract::schema::DataContractSchemaMethodsV0;
use crate::data_contract::serialized_version::v1::{
    deserialize_u16_group_map, deserialize_u16_token_configuration_map,
    DataContractInSerializationFormatV1,
};
use crate::data_contract::{
    DataContract, DefinitionName, DocumentName, GroupContractPosition, TokenContractPosition,
};
use crate::identity::TimestampMillis;
use crate::prelude::BlockHeight;
#[cfg(feature = "json-conversion")]
use crate::serialization::json_safe_fields;
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_value::{Identifier, Value};
use platform_version::version::PlatformVersion;
use platform_version::IntoPlatformVersioned;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[cfg_attr(feature = "json-conversion", json_safe_fields)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Encode, Decode, DecodeUntrusted)]
#[serde(rename_all = "camelCase")]
pub struct DataContractInSerializationFormatV2 {
    /// A unique identifier for the data contract.
    pub id: Identifier,

    /// Internal configuration for the contract.
    #[serde(default = "DataContractConfigV1::default_with_version")]
    pub config: DataContractConfig,

    /// The version of this data contract.
    pub version: u32,

    /// The identifier of the contract owner.
    pub owner_id: Identifier,

    /// Shared subschemas to reuse across documents as $defs object
    pub schema_defs: Option<BTreeMap<DefinitionName, Value>>,

    /// Document JSON Schemas per type, as the state transition that registered or last updated
    /// the contract carried them
    pub document_schemas: BTreeMap<DocumentName, Value>,

    /// Each document type of [`Self::document_schemas`], compiled, under the same name
    pub document_types: BTreeMap<DocumentName, DocumentTypeInSerializationFormat>,

    /// The time in milliseconds that the contract was created.
    pub created_at: Option<TimestampMillis>,
    /// The time in milliseconds that the contract was last updated.
    pub updated_at: Option<TimestampMillis>,
    /// The block that the document was created.
    pub created_at_block_height: Option<BlockHeight>,
    /// The block that the contract was last updated
    pub updated_at_block_height: Option<BlockHeight>,
    /// The epoch at which the contract was created.
    pub created_at_epoch: Option<EpochIndex>,
    /// The epoch at which the contract was last updated.
    pub updated_at_epoch: Option<EpochIndex>,

    /// Groups that allow for specific multiparty actions on the contract
    #[serde(default, deserialize_with = "deserialize_u16_group_map")]
    pub groups: BTreeMap<GroupContractPosition, Group>,

    /// The tokens on the contract.
    #[serde(default, deserialize_with = "deserialize_u16_token_configuration_map")]
    pub tokens: BTreeMap<TokenContractPosition, TokenConfiguration>,

    /// The contract's keywords for searching
    #[serde(default)]
    pub keywords: Vec<String>,

    /// The contract's description
    #[serde(default)]
    pub description: Option<String>,
}

impl DataContractInSerializationFormatV2 {
    /// `contract` in format version 2: its format version 1 with each document type compiled.
    pub(in crate::data_contract) fn try_from_data_contract(
        contract: DataContract,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        let document_types = contract
            .document_types()
            .iter()
            .map(|(name, document_type)| {
                Ok((
                    name.clone(),
                    DocumentTypeInSerializationFormat::try_from_document_type(
                        document_type.as_ref(),
                        contract.schema_defs(),
                        platform_version,
                    )?,
                ))
            })
            .collect::<Result<BTreeMap<_, _>, ProtocolError>>()?;
        let DataContractInSerializationFormatV1 {
            id,
            config,
            version,
            owner_id,
            schema_defs,
            document_schemas,
            created_at,
            updated_at,
            created_at_block_height,
            updated_at_block_height,
            created_at_epoch,
            updated_at_epoch,
            groups,
            tokens,
            keywords,
            description,
        } = contract.into_platform_versioned(platform_version);
        Ok(Self {
            id,
            config,
            version,
            owner_id,
            schema_defs,
            document_schemas,
            document_types,
            created_at,
            updated_at,
            created_at_block_height,
            updated_at_block_height,
            created_at_epoch,
            updated_at_epoch,
            groups,
            tokens,
            keywords,
            description,
        })
    }
}

/// Format version 1 of the same contract: the raw schemas without the compiled document types,
/// which were compiled from them.
impl From<DataContractInSerializationFormatV2> for DataContractInSerializationFormatV1 {
    fn from(value: DataContractInSerializationFormatV2) -> Self {
        let DataContractInSerializationFormatV2 {
            id,
            config,
            version,
            owner_id,
            schema_defs,
            document_schemas,
            document_types: _,
            created_at,
            updated_at,
            created_at_block_height,
            updated_at_block_height,
            created_at_epoch,
            updated_at_epoch,
            groups,
            tokens,
            keywords,
            description,
        } = value;
        DataContractInSerializationFormatV1 {
            id,
            config,
            version,
            owner_id,
            schema_defs,
            document_schemas,
            created_at,
            updated_at,
            created_at_block_height,
            updated_at_block_height,
            created_at_epoch,
            updated_at_epoch,
            groups,
            tokens,
            keywords,
            description,
        }
    }
}

#[cfg(test)]
mod tests;
