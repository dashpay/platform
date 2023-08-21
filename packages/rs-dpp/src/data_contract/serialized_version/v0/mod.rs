use crate::data_contract::config::v0::DataContractConfigV0;
use crate::data_contract::config::DataContractConfig;
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::storage_requirements::keys_for_document_type::StorageKeyRequirements;
use crate::data_contract::v0::DataContractV0;
use crate::data_contract::{DataContract, DefinitionName, DocumentName};
use crate::identity::state_transition::asset_lock_proof::{Decode, Encode};
use platform_value::{Identifier, Value};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub mod property_names {
    pub const ID: &str = "id";
    pub const OWNER_ID: &str = "ownerId";
    pub const VERSION: &str = "version";
    pub const DOCUMENTS: &str = "documents";
    pub const DEFINITIONS: &str = "$defs";
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Encode, Decode)]
#[serde(rename_all = "camelCase")]
pub struct DataContractInSerializationFormatV0 {
    /// A unique identifier for the data contract.
    pub id: Identifier,

    /// Internal configuration for the contract.
    #[serde(default = "DataContractConfigV0::default_with_version")]
    pub config: DataContractConfig,

    /// The version of this data contract.
    pub version: u32,

    /// The identifier of the contract owner.
    pub owner_id: Identifier,

    /// Shared subschemas to reuse across documents as $defs object
    pub schema_defs: Option<BTreeMap<DefinitionName, Value>>,

    /// Document JSON Schemas per type
    pub document_schemas: BTreeMap<DocumentName, Value>,

    /// Encryption key storage requirements
    pub encryption_key_storage_requirements: Option<StorageKeyRequirements>,

    /// Decryption key storage requirements
    pub decryption_key_storage_requirements: Option<StorageKeyRequirements>,
}

impl From<DataContract> for DataContractInSerializationFormatV0 {
    fn from(value: DataContract) -> Self {
        match value {
            DataContract::V0(v0) => {
                let DataContractV0 {
                    id,
                    config,
                    version,
                    owner_id,
                    schema_defs,
                    document_types,
                    ..
                } = v0;

                DataContractInSerializationFormatV0 {
                    id,
                    config,
                    version,
                    owner_id,
                    document_schemas: document_types
                        .into_iter()
                        .map(|(key, document_type)| (key, document_type.schema_owned()))
                        .collect(),
                    schema_defs,
                }
            }
        }
    }
}
