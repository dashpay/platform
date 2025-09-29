mod from_document;
pub mod v0_methods;

use bincode::{Decode, Encode};

#[cfg(feature = "state-transition-value-conversion")]
use platform_value::btreemap_extensions::BTreeValueRemoveFromMapHelper;
use platform_value::{Identifier, Value};
#[cfg(feature = "state-transition-serde-conversion")]
use serde::{Deserialize, Serialize};

use std::collections::BTreeMap;

use std::string::ToString;

use crate::data_contract::DataContract;

use crate::{document, errors::ProtocolError};

use crate::block::block_info::BlockInfo;
use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::methods::DocumentTypeBasicMethods;
use crate::data_contract::document_type::DocumentTypeRef;
use crate::document::property_names::CREATOR_ID;
use crate::document::{Document, DocumentV0};
use crate::fee::Credits;
#[cfg(feature = "state-transition-value-conversion")]
use crate::state_transition::batch_transition;
use crate::state_transition::batch_transition::document_base_transition::v0::v0_methods::DocumentBaseTransitionV0Methods;
#[cfg(feature = "state-transition-value-conversion")]
use crate::state_transition::batch_transition::document_base_transition::v0::DocumentBaseTransitionV0;
#[cfg(feature = "state-transition-value-conversion")]
use crate::state_transition::batch_transition::document_base_transition::v0::DocumentTransitionObjectLike;
use crate::state_transition::batch_transition::document_base_transition::DocumentBaseTransition;
use derive_more::Display;
#[cfg(feature = "state-transition-value-conversion")]
use platform_value::btreemap_extensions::BTreeValueRemoveTupleFromMapHelper;
use platform_version::version::PlatformVersion;

mod property_names {
    pub const ENTROPY: &str = "$entropy";
    pub const PREFUNDED_VOTING_BALANCE: &str = "$prefundedVotingBalance";
}

/// The Binary fields in [`DocumentCreateTransition`]
pub const BINARY_FIELDS: [&str; 1] = ["$entropy"];
/// The Identifier fields in [`DocumentCreateTransition`]
pub use super::super::document_base_transition::IDENTIFIER_FIELDS;

#[derive(Debug, Clone, Default, Encode, Decode, PartialEq, Display)]
#[cfg_attr(
    feature = "state-transition-serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
#[display("Base: {}, Entropy: {:?}, Data: {:?}", "base", "entropy", "data")]
pub struct DocumentCreateTransitionV0 {
    /// Document Base Transition
    #[cfg_attr(feature = "state-transition-serde-conversion", serde(flatten))]
    pub base: DocumentBaseTransition,

    /// Entropy used to create a Document ID.
    #[cfg_attr(
        feature = "state-transition-serde-conversion",
        serde(rename = "$entropy")
    )]
    pub entropy: [u8; 32],

    #[cfg_attr(feature = "state-transition-serde-conversion", serde(flatten))]
    pub data: BTreeMap<String, Value>,

    #[cfg_attr(
        feature = "state-transition-serde-conversion",
        serde(rename = "$prefundedVotingBalance")
    )]
    /// Pre funded balance (for unique index conflict resolution voting - the identity will put money
    /// aside that will be used by voters to vote)
    /// This is a map of index names to the amount we want to prefund them for
    /// Since index conflict resolution is not a common feature most often nothing should be added here.
    pub prefunded_voting_balance: Option<(String, Credits)>,
}

impl DocumentCreateTransitionV0 {
    #[cfg(feature = "state-transition-value-conversion")]
    pub(crate) fn from_value_map(
        mut map: BTreeMap<String, Value>,
        data_contract: DataContract,
    ) -> Result<Self, ProtocolError> {
        let identity_contract_nonce = map
            .remove_integer(
                batch_transition::document_base_transition::property_names::IDENTITY_CONTRACT_NONCE,
            )
            .map_err(ProtocolError::ValueError)?;
        Ok(Self {
            base: DocumentBaseTransition::V0(DocumentBaseTransitionV0::from_value_map_consume(
                &mut map,
                data_contract,
                identity_contract_nonce,
            )?),
            entropy: map
                .remove_hash256_bytes(property_names::ENTROPY)
                .map_err(ProtocolError::ValueError)?,
            prefunded_voting_balance: map
                .remove_optional_tuple(property_names::PREFUNDED_VOTING_BALANCE)?,
            data: map,
        })
    }

    #[cfg(feature = "state-transition-value-conversion")]
    pub(crate) fn to_value_map(&self) -> Result<BTreeMap<String, Value>, ProtocolError> {
        let mut transition_base_map = self.base.to_value_map()?;
        transition_base_map.insert(
            property_names::ENTROPY.to_string(),
            Value::Bytes(self.entropy.to_vec()),
        );

        if let Some((index_name, prefunded_voting_balance)) = &self.prefunded_voting_balance {
            let index_name_value = Value::Text(index_name.clone());
            let prefunded_voting_balance_value = Value::U64(*prefunded_voting_balance);
            transition_base_map.insert(
                property_names::PREFUNDED_VOTING_BALANCE.to_string(),
                Value::Array(vec![index_name_value, prefunded_voting_balance_value]),
            );
        }

        transition_base_map.extend(self.data.clone());

        Ok(transition_base_map)
    }
}

/// documents from create transition v0
pub trait DocumentFromCreateTransitionV0 {
    /// Attempts to create a new `Document` from the given `DocumentCreateTransitionV0` instance, incorporating additional metadata such as ownership and block information.
    ///
    /// This function is responsible for taking an owned `DocumentCreateTransitionV0` instance, which encapsulates the initial data for a document, and augmenting this with metadata including the document owner's identifier, block information, and the requirement status of `created_at` and `updated_at` timestamps, as dictated by the associated data contract and the current platform version.
    ///
    /// # Arguments
    ///
    /// * `v0` - An owned `DocumentCreateTransitionV0` instance containing the initial data for the document being created.
    /// * `owner_id` - The `Identifier` of the document's owner, specifying who will own the newly created document.
    /// * `block_info` - A reference to the `BlockInfo`, which provides context about the block height and other block-related metadata at the time of document creation.
    /// * `document_type` - A reference to the `DocumentTypeRef` associated with this document, defining its structure and rules.
    /// * `platform_version` - A reference to the `PlatformVersion`, which may influence the creation process or validation logic based on the version-specific rules or features of the platform.
    ///
    /// # Returns
    ///
    /// * `Result<Self, ProtocolError>` - On successful creation, returns a new `Document` object populated with the provided data and augmented with necessary metadata. If the creation process encounters any validation failures or other issues, it returns a `ProtocolError`.
    ///
    fn try_from_owned_create_transition_v0(
        v0: DocumentCreateTransitionV0,
        owner_id: Identifier,
        block_info: &BlockInfo,
        contract: &DataContract,
        document_type: &DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized;
    /// Attempts to create a new `Document` from the given `DocumentCreateTransitionV0` reference, incorporating additional metadata like ownership and block information.
    ///
    /// This function takes a `DocumentCreateTransitionV0` reference, which contains the initial data for the document, and combines it with metadata such as the document owner's identifier, block information, and requirements for timestamp fields based on the associated data contract and platform version.
    ///
    /// # Arguments
    ///
    /// * `v0` - A reference to the `DocumentCreateTransitionV0` containing initial data for the document being created.
    /// * `owner_id` - The `Identifier` of the document's owner.
    /// * `block_info` - A reference to the `BlockInfo` containing the block height at which the document is being created.
    /// * `document_type` - A reference to the `DocumentTypeRef` associated with this document, defining its structure and rules.
    /// * `platform_version` - A reference to the `PlatformVersion` indicating the version of the platform under which the document is being created.
    ///
    /// # Returns
    ///
    /// * `Result<Self, ProtocolError>` - A new `Document` object if successful, populated with the provided data and metadata. Returns a `ProtocolError` if the creation fails due to issues like missing required fields, incorrect types, or other validation failures.
    ///
    fn try_from_create_transition_v0(
        v0: &DocumentCreateTransitionV0,
        owner_id: Identifier,
        block_info: &BlockInfo,
        contract: &DataContract,
        document_type: &DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized;
}

impl DocumentFromCreateTransitionV0 for Document {
    fn try_from_owned_create_transition_v0(
        v0: DocumentCreateTransitionV0,
        owner_id: Identifier,
        block_info: &BlockInfo,
        contract: &DataContract,
        document_type: &DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized,
    {
        let DocumentCreateTransitionV0 { base, mut data, .. } = v0;

        let requires_created_at = document_type
            .required_fields()
            .contains(document::property_names::CREATED_AT);

        let adds_creator_id = document_type.should_use_creator_id(
            contract.system_version_type(),
            contract.config().version(),
            platform_version,
        )?;

        if adds_creator_id {
            data.insert(CREATOR_ID.to_string(), owner_id.into());
        }

        let requires_updated_at = document_type
            .required_fields()
            .contains(document::property_names::UPDATED_AT);

        let requires_created_at_block_height = document_type
            .required_fields()
            .contains(document::property_names::CREATED_AT_BLOCK_HEIGHT);
        let requires_updated_at_block_height = document_type
            .required_fields()
            .contains(document::property_names::UPDATED_AT_BLOCK_HEIGHT);

        let requires_created_at_core_block_height = document_type
            .required_fields()
            .contains(document::property_names::CREATED_AT_CORE_BLOCK_HEIGHT);
        let requires_updated_at_core_block_height = document_type
            .required_fields()
            .contains(document::property_names::UPDATED_AT_CORE_BLOCK_HEIGHT);

        let created_at = if requires_created_at {
            Some(block_info.time_ms)
        } else {
            None
        };
        let updated_at = if requires_updated_at {
            Some(block_info.time_ms)
        } else {
            None
        };

        let created_at_block_height = if requires_created_at_block_height {
            Some(block_info.height)
        } else {
            None
        };
        let updated_at_block_height = if requires_updated_at_block_height {
            Some(block_info.height)
        } else {
            None
        };

        let created_at_core_block_height = if requires_created_at_core_block_height {
            Some(block_info.core_height)
        } else {
            None
        };
        let updated_at_core_block_height = if requires_updated_at_core_block_height {
            Some(block_info.core_height)
        } else {
            None
        };

        match platform_version
            .dpp
            .document_versions
            .document_structure_version
        {
            0 => Ok(DocumentV0 {
                id: base.id(),
                owner_id,
                properties: data,
                revision: document_type.initial_revision(),
                created_at,
                updated_at,
                transferred_at: None,
                created_at_block_height,
                updated_at_block_height,
                transferred_at_block_height: None,
                created_at_core_block_height,
                updated_at_core_block_height,
                transferred_at_core_block_height: None,
            }
            .into()),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "Document::try_from_create_transition_v0".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    fn try_from_create_transition_v0(
        v0: &DocumentCreateTransitionV0,
        owner_id: Identifier,
        block_info: &BlockInfo,
        contract: &DataContract,
        document_type: &DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized,
    {
        let DocumentCreateTransitionV0 { base, data, .. } = v0;

        let requires_created_at = document_type
            .required_fields()
            .contains(document::property_names::CREATED_AT);

        let mut properties = data.clone();
        let adds_creator_id = document_type.should_use_creator_id(
            contract.system_version_type(),
            contract.config().version(),
            platform_version,
        )?;

        if adds_creator_id {
            properties.insert(CREATOR_ID.to_string(), owner_id.into());
        }

        let requires_updated_at = document_type
            .required_fields()
            .contains(document::property_names::UPDATED_AT);

        let requires_created_at_block_height = document_type
            .required_fields()
            .contains(document::property_names::CREATED_AT_BLOCK_HEIGHT);
        let requires_updated_at_block_height = document_type
            .required_fields()
            .contains(document::property_names::UPDATED_AT_BLOCK_HEIGHT);

        let requires_created_at_core_block_height = document_type
            .required_fields()
            .contains(document::property_names::CREATED_AT_CORE_BLOCK_HEIGHT);
        let requires_updated_at_core_block_height = document_type
            .required_fields()
            .contains(document::property_names::UPDATED_AT_CORE_BLOCK_HEIGHT);

        let created_at = if requires_created_at {
            Some(block_info.time_ms)
        } else {
            None
        };
        let updated_at = if requires_updated_at {
            Some(block_info.time_ms)
        } else {
            None
        };

        let created_at_block_height = if requires_created_at_block_height {
            Some(block_info.height)
        } else {
            None
        };
        let updated_at_block_height = if requires_updated_at_block_height {
            Some(block_info.height)
        } else {
            None
        };

        let created_at_core_block_height = if requires_created_at_core_block_height {
            Some(block_info.core_height)
        } else {
            None
        };
        let updated_at_core_block_height = if requires_updated_at_core_block_height {
            Some(block_info.core_height)
        } else {
            None
        };

        match platform_version
            .dpp
            .document_versions
            .document_structure_version
        {
            0 => Ok(DocumentV0 {
                id: base.id(),
                owner_id,
                properties,
                revision: document_type.initial_revision(),
                created_at,
                updated_at,
                transferred_at: None,
                created_at_block_height,
                updated_at_block_height,
                transferred_at_block_height: None,
                created_at_core_block_height,
                updated_at_core_block_height,
                transferred_at_core_block_height: None,
            }
            .into()),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "Document::try_from_owned_create_transition_v0".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::data_contract::v0::DataContractV0;
    use crate::state_transition::batch_transition::document_create_transition::DocumentCreateTransition;
    use platform_value::btreemap_extensions::BTreeValueMapHelper;
    use platform_value::{platform_value, BinaryData, Bytes32, Identifier};
    use platform_version::version::LATEST_PLATFORM_VERSION;
    use serde_json::json;

    use super::*;
    use crate::data_contract::conversion::value::v0::DataContractValueConversionMethodsV0;
    use serde_json::Value as JsonValue;

    fn init() {
        let _ = env_logger::builder()
            .filter_level(log::LevelFilter::Debug)
            .try_init();
    }

    fn data_contract_with_dynamic_properties() -> DataContract {
        // The following is equivalent to the data contract
        // {
        //     "protocolVersion" :0,
        //     "$id" : vec![0_u8;32],
        //     "$schema" : "schema",
        //     "version" : 0,
        //     "ownerId" : vec![0_u8;32],
        //     "documents" : {
        //         "test" : {
        //             "properties" : {
        //                 "alphaIdentifier" :  {
        //                     "type": "array",
        //                     "byteArray": true,
        //                     "contentMediaType": "application/x.dash.dpp.identifier",
        //                 },
        //                 "alphaBinary" :  {
        //                     "type": "array",
        //                     "byteArray": true,
        //                 }
        //             }
        //         }
        //     }
        // }
        let test_document_properties_alpha_identifier = Value::from([
            ("type", Value::Text("array".to_string())),
            ("byteArray", Value::Bool(true)),
            ("position", Value::U64(0)),
        ]);
        let test_document_properties_alpha_binary = Value::from([
            ("type", Value::Text("array".to_string())),
            ("byteArray", Value::Bool(true)),
            ("minItems", Value::U64(32)),
            ("maxItems", Value::U64(32)),
            (
                "contentMediaType",
                Value::Text("application/x.dash.dpp.identifier".to_string()),
            ),
            ("position", Value::U64(1)),
        ]);
        let test_document_properties = Value::from([
            ("alphaIdentifier", test_document_properties_alpha_identifier),
            ("alphaBinary", test_document_properties_alpha_binary),
        ]);
        let test_document = Value::from([
            ("type", Value::Text("object".to_string())),
            ("properties", test_document_properties),
            ("additionalProperties", Value::Bool(false)),
        ]);
        let documents = Value::from([("test", test_document)]);
        DataContract::V0(
            DataContractV0::from_value(
                Value::from([
                    ("$id", Value::Identifier([0_u8; 32])),
                    ("id", Value::Identifier([0_u8; 32])),
                    ("$schema", Value::Text("schema".to_string())),
                    ("$format_version", Value::Text("0".to_string())),
                    ("version", Value::U32(0)),
                    ("documentSchemas", documents),
                    ("ownerId", Value::Identifier([0_u8; 32])),
                ]),
                true,
                LATEST_PLATFORM_VERSION,
            )
            .unwrap(),
        )
    }

    #[test]
    #[cfg(feature = "state-transition-json-conversion")]
    fn convert_to_json_with_dynamic_binary_paths() {
        let data_contract = data_contract_with_dynamic_properties();
        let alpha_binary = BinaryData::new(vec![10_u8; 32]);
        let alpha_identifier = Identifier::from([10_u8; 32]);
        let id = Identifier::from([11_u8; 32]);
        let data_contract_id = Identifier::from([13_u8; 32]);
        let entropy = Bytes32::new([14_u8; 32]);

        let raw_document = platform_value!({
            "$protocolVersion"  : 0u32,
            "$version"  : "0".to_string(),
            "$id" : id,
            "id" : id,
            "$type" : "test",
            "$dataContractId" : data_contract_id,
            "$identityContractNonce": 0u64,
            "revision" : 1u32,
            "alphaBinary" : alpha_binary,
            "alphaIdentifier" : alpha_identifier,
            "$entropy" : entropy,
            "$action": 0u8,
        });

        let transition: DocumentCreateTransition =
            DocumentCreateTransition::from_object(raw_document, data_contract).unwrap();

        let json_transition = transition.to_json().expect("no errors");
        assert_eq!(json_transition["V0"]["$id"], JsonValue::String(id.into()));
        assert_eq!(
            json_transition["V0"]["$dataContractId"],
            JsonValue::String(data_contract_id.into())
        );
        assert_eq!(
            json_transition["alphaBinary"],
            JsonValue::String(alpha_binary.into())
        );
        assert_eq!(
            json_transition["alphaIdentifier"],
            JsonValue::String(alpha_identifier.into())
        );
        assert_eq!(
            json_transition["$entropy"],
            JsonValue::String(entropy.into())
        );
    }

    #[test]
    fn convert_to_object_from_json_value_with_dynamic_binary_paths() {
        let data_contract = data_contract_with_dynamic_properties();
        let alpha_value = vec![10_u8; 32];
        let id = vec![11_u8; 32];
        let data_contract_id = vec![13_u8; 32];
        let entropy = vec![11_u8; 32];

        let raw_document = json!({
            "$protocolVersion"  : 0,
            "$version"  : "0",
            "$id" : id,
            "$type" : "test",
            "$dataContractId" : data_contract_id,
            "$identityContractNonce": 0u64,
            "revision" : 1,
            "alphaBinary" : alpha_value,
            "alphaIdentifier" : alpha_value,
            "$entropy" : entropy,
            "$action": 0 ,
        });

        let document: DocumentCreateTransition =
            DocumentCreateTransition::from_json_object(raw_document, data_contract).unwrap();

        let object_transition = document
            .to_object()
            .expect("no errors")
            .into_btree_string_map()
            .unwrap();

        let v0 = object_transition.get("V0").expect("to get V0");
        let right_id = Identifier::from_bytes(&id).unwrap();
        let right_data_contract_id = Identifier::from_bytes(&data_contract_id).unwrap();

        assert_eq!(v0["$id"], Value::Identifier(right_id.into_buffer()));
        assert_eq!(
            v0["$dataContractId"],
            Value::Identifier(right_data_contract_id.into_buffer())
        );

        assert_eq!(
            object_transition.get_bytes("alphaBinary").unwrap(),
            alpha_value
        );
        assert_eq!(
            object_transition
                .get_identifier_bytes("alphaIdentifier")
                .unwrap(),
            alpha_value
        );
    }
}
