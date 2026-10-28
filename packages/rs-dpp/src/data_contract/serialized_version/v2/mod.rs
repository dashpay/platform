//! Format version 2 of a data contract: the stored format, written only to state, never
//! carried by a state transition. It holds format version 1, raw document schemas included, and
//! beside it each document type compiled ([`DocumentTypeInSerializationFormat`]), so a node
//! reading its state, or a client verifying a proof of it, reads the document types back instead
//! of parsing their schemas. A reader that decodes bytes nothing vouches for parses the schemas
//! instead, and one that asks for full validation also checks that the compiled types are what
//! they compile to.

use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::accessors::v1::DataContractV1Getters;
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::serialized_version::DocumentTypeInSerializationFormat;
use crate::data_contract::schema::DataContractSchemaMethodsV0;
use crate::data_contract::serialized_version::v1::DataContractInSerializationFormatV1;
use crate::data_contract::{DataContract, DocumentName};
#[cfg(feature = "json-conversion")]
use crate::serialization::json_safe_fields;
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_version::version::PlatformVersion;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// On the serde wire (JSON, platform values) format version 1's keys with `documentTypes`
/// beside them; on the bincode wire format version 1's bytes followed by the compiled types.
#[cfg_attr(feature = "json-conversion", json_safe_fields)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Encode, Decode, DecodeUntrusted)]
#[serde(rename_all = "camelCase")]
pub struct DataContractInSerializationFormatV2 {
    /// The contract in format version 1: everything but the compiled document types.
    #[serde(flatten)]
    pub base: DataContractInSerializationFormatV1,

    /// Each document type of the base's document schemas, compiled, under the same name
    pub document_types: BTreeMap<DocumentName, DocumentTypeInSerializationFormat>,
}

impl DataContractInSerializationFormatV2 {
    /// `contract` in format version 2: its format version 1 with each document type compiled.
    pub(in crate::data_contract) fn try_from_data_contract(
        contract: &DataContract,
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
        Ok(Self {
            base: format_version_1_of(contract),
            document_types,
        })
    }
}

/// `contract` in format version 1, as `DataContractInSerializationFormatV1::from` gives it,
/// copying only the fields and the raw schemas, not the parsed document types.
fn format_version_1_of(contract: &DataContract) -> DataContractInSerializationFormatV1 {
    DataContractInSerializationFormatV1 {
        id: contract.id(),
        config: contract.config().clone(),
        version: contract.version(),
        owner_id: contract.owner_id(),
        schema_defs: contract.schema_defs().cloned(),
        document_schemas: contract
            .document_types()
            .iter()
            .map(|(name, document_type)| (name.clone(), document_type.schema().clone()))
            .collect(),
        created_at: contract.created_at(),
        updated_at: contract.updated_at(),
        created_at_block_height: contract.created_at_block_height(),
        updated_at_block_height: contract.updated_at_block_height(),
        created_at_epoch: contract.created_at_epoch(),
        updated_at_epoch: contract.updated_at_epoch(),
        groups: contract.groups().clone(),
        tokens: contract.tokens().clone(),
        keywords: contract.keywords().clone(),
        description: contract.description().cloned(),
    }
}

/// Format version 1 of the same contract: the raw schemas without the compiled document types,
/// which were compiled from them.
impl From<DataContractInSerializationFormatV2> for DataContractInSerializationFormatV1 {
    fn from(value: DataContractInSerializationFormatV2) -> Self {
        value.base
    }
}

#[cfg(test)]
mod tests;
