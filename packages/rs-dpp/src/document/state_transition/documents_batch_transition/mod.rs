use std::collections::HashMap;
use std::convert::TryInto;

use anyhow::{anyhow, Context};
use ciborium::value::Value as CborValue;
use integer_encoding::VarInt;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::data_contract::DataContract;
use crate::document::document_transition::DocumentTransitionObjectLike;
use crate::prelude::{DocumentTransition, Identifier};
use crate::state_transition::state_transition_execution_context::StateTransitionExecutionContext;
use crate::util::cbor_value::{CborCanonicalMap, FieldType, ReplacePaths, ValuesCollection};
use crate::util::json_value::{JsonValueExt, ReplaceWith};
use crate::util::string_encoding::Encoding;
use crate::version::LATEST_VERSION;
use crate::ProtocolError;
use crate::{
    identity::{KeyID, SecurityLevel},
    state_transition::{
        StateTransitionConvert, StateTransitionIdentitySigned, StateTransitionLike,
        StateTransitionType,
    },
};

use self::document_transition::{
    document_base_transition, document_create_transition, DocumentTransitionExt,
};

pub mod apply_documents_batch_transition_factory;
pub mod document_transition;
pub mod validation;

pub mod property_names {
    pub const DATA_CONTRACT_ID: &str = "$dataContractId";
    pub const TRANSITIONS: &str = "transitions";
    pub const OWNER_ID: &str = "ownerId";
    pub const SIGNATURE_PUBLIC_KEY_ID: &str = "signaturePublicKeyId";
    pub const SIGNATURE: &str = "signature";
    pub const PROTOCOL_VERSION: &str = "protocolVersion";
    pub const SECURITY_LEVEL_REQUIREMENT: &str = "signatureSecurityLevelRequirement";
}

const DEFAULT_SECURITY_LEVEL: SecurityLevel = SecurityLevel::HIGH;
const EMPTY_VEC: Vec<u8> = vec![];

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DocumentsBatchTransition {
    pub protocol_version: u32,
    #[serde(rename = "type")]
    pub transition_type: StateTransitionType,
    pub owner_id: Identifier,
    // we want to skip serialization of transitions, as we does it manually in `to_object()`  and `to_json()`
    #[serde(skip_serializing)]
    pub transitions: Vec<DocumentTransition>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signature_public_key_id: Option<KeyID>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signature: Option<Vec<u8>>,

    #[serde(skip)]
    pub execution_context: StateTransitionExecutionContext,
}

impl std::default::Default for DocumentsBatchTransition {
    fn default() -> Self {
        DocumentsBatchTransition {
            protocol_version: Default::default(),
            transition_type: StateTransitionType::DocumentsBatch,
            owner_id: Identifier::default(),
            transitions: vec![],
            signature_public_key_id: None,
            signature: None,
            execution_context: Default::default(),
        }
    }
}

impl DocumentsBatchTransition {
    pub fn from_json_object(
        json_value: JsonValue,
        data_contracts: Vec<DataContract>,
    ) -> Result<Self, ProtocolError> {
        let mut json_value = json_value;

        let maybe_signature = json_value.get_string(property_names::SIGNATURE).ok();
        let signature = if let Some(signature) = maybe_signature {
            Some(base64::decode(signature).context("signature exists but isn't valid base64")?)
        } else {
            None
        };

        let mut batch_transitions = DocumentsBatchTransition {
            protocol_version: json_value
                .get_u64(property_names::PROTOCOL_VERSION)
                // js-dpp allows `protocolVersion` to be undefined
                .unwrap_or(LATEST_VERSION as u64) as u32,
            signature,
            signature_public_key_id: json_value
                .get_u64(property_names::SIGNATURE_PUBLIC_KEY_ID)
                .ok()
                .map(|v| v as KeyID),
            owner_id: Identifier::from_string(
                json_value.get_string(property_names::OWNER_ID)?,
                Encoding::Base58,
            )?,
            ..Default::default()
        };

        let mut document_transitions: Vec<DocumentTransition> = vec![];
        let maybe_transitions = json_value.remove(property_names::TRANSITIONS);
        if let Ok(JsonValue::Array(json_transitions)) = maybe_transitions {
            let data_contracts_map: HashMap<Vec<u8>, DataContract> = data_contracts
                .into_iter()
                .map(|dc| (dc.id.as_bytes().to_vec(), dc))
                .collect();

            for json_transition in json_transitions {
                let id = Identifier::from_string(
                    json_transition.get_string(property_names::DATA_CONTRACT_ID)?,
                    Encoding::Base58,
                )?;
                let data_contract =
                    data_contracts_map
                        .get(&id.as_bytes().to_vec())
                        .ok_or_else(|| {
                            anyhow!(
                                "Data Contract doesn't exists for Transition: {:?}",
                                json_transition
                            )
                        })?;
                let document_transition =
                    DocumentTransition::from_json_object(json_transition, data_contract.clone())?;
                document_transitions.push(document_transition);
            }
        }

        batch_transitions.transitions = document_transitions;
        Ok(batch_transitions)
    }

    /// creates the instance of [`DocumentsBatchTransition`] from raw object
    pub fn from_raw_object(
        mut raw_object: JsonValue,
        data_contracts: Vec<DataContract>,
    ) -> Result<Self, ProtocolError> {
        let mut batch_transitions = DocumentsBatchTransition {
            protocol_version: raw_object
                .get_u64(property_names::PROTOCOL_VERSION)
                // js-dpp allows `protocolVersion` to be undefined
                .unwrap_or(LATEST_VERSION as u64) as u32,
            signature: raw_object.get_bytes(property_names::SIGNATURE).ok(),
            signature_public_key_id: raw_object
                .get_u64(property_names::SIGNATURE_PUBLIC_KEY_ID)
                .ok()
                .map(|v| v as KeyID),
            owner_id: Identifier::from_bytes(&raw_object.get_bytes(property_names::OWNER_ID)?)?,
            ..Default::default()
        };

        let mut document_transitions: Vec<DocumentTransition> = vec![];
        let maybe_transitions = raw_object.remove(property_names::TRANSITIONS);
        if let Ok(JsonValue::Array(raw_transitions)) = maybe_transitions {
            let data_contracts_map: HashMap<Vec<u8>, DataContract> = data_contracts
                .into_iter()
                .map(|dc| (dc.id.as_bytes().to_vec(), dc))
                .collect();

            for raw_transition in raw_transitions {
                let id = raw_transition.get_bytes(property_names::DATA_CONTRACT_ID)?;
                let data_contract = data_contracts_map.get(&id).ok_or_else(|| {
                    anyhow!(
                        "Data Contract doesn't exists for Transition: {:?}",
                        raw_transition
                    )
                })?;
                let document_transition =
                    DocumentTransition::from_raw_object(raw_transition, data_contract.clone())?;
                document_transitions.push(document_transition);
            }
        }

        batch_transitions.transitions = document_transitions;
        Ok(batch_transitions)
    }

    pub fn get_transitions(&self) -> &Vec<DocumentTransition> {
        &self.transitions
    }

    // TODO to decide if this should be a lazy iterator or a vector
    pub fn get_modified_data_ids(&self) -> impl Iterator<Item = &Identifier> {
        self.transitions.iter().map(|t| &t.base().id)
    }
}

impl StateTransitionIdentitySigned for DocumentsBatchTransition {
    fn get_owner_id(&self) -> &Identifier {
        &self.owner_id
    }

    fn get_security_level_requirement(&self) -> crate::identity::SecurityLevel {
        // Step 1: Get all document types for the ST
        // Step 2: Get document schema for every type
        // If schema has security level, use that, if not, use the default security level
        // Find the highest level (lowest int value) of all documents - the ST's signature
        // requirement is the highest level across all documents affected by the ST./
        let mut highest_security_level = SecurityLevel::lowest_level();

        for transition in self.transitions.iter() {
            let document_type = &transition.base().document_type;
            let data_contract = &transition.base().data_contract;
            let maybe_document_schema = data_contract.get_document_schema(document_type);

            if let Ok(document_schema) = maybe_document_schema {
                let document_security_level =
                    get_security_level_requirement(document_schema, DEFAULT_SECURITY_LEVEL);

                // lower enum enum representation means higher in security
                if document_security_level < highest_security_level {
                    highest_security_level = document_security_level
                }
            }
        }
        highest_security_level
    }

    fn get_signature_public_key_id(&self) -> Option<KeyID> {
        self.signature_public_key_id
    }

    fn set_signature_public_key_id(&mut self, key_id: KeyID) {
        self.signature_public_key_id = Some(key_id);
    }
}

impl StateTransitionConvert for DocumentsBatchTransition {
    fn binary_property_paths() -> Vec<&'static str> {
        vec![property_names::SIGNATURE]
    }

    fn identifiers_property_paths() -> Vec<&'static str> {
        vec![property_names::OWNER_ID]
    }

    fn signature_property_paths() -> Vec<&'static str> {
        vec![property_names::SIGNATURE]
    }

    fn to_json(&self, skip_signature: bool) -> Result<JsonValue, ProtocolError> {
        let mut json_value: JsonValue = serde_json::to_value(self)?;

        if skip_signature {
            if let JsonValue::Object(ref mut o) = json_value {
                for path in Self::signature_property_paths() {
                    o.remove(path);
                }
            }
        }

        json_value.replace_binary_paths(Self::binary_property_paths(), ReplaceWith::Base64)?;

        let mut transitions = vec![];
        for transition in self.transitions.iter() {
            transitions.push(transition.to_json()?)
        }
        json_value.insert(
            String::from(property_names::TRANSITIONS),
            JsonValue::Array(transitions),
        )?;

        Ok(json_value)
    }

    fn to_object(&self, skip_signature: bool) -> Result<JsonValue, ProtocolError> {
        let mut json_object: JsonValue = serde_json::to_value(self)?;
        json_object
            .replace_identifier_paths(Self::identifiers_property_paths(), ReplaceWith::Bytes)?;

        if skip_signature {
            for path in Self::signature_property_paths() {
                let _ = json_object.remove(path);
            }
        }
        let mut transitions = vec![];
        for transition in self.transitions.iter() {
            transitions.push(transition.to_object()?)
        }
        json_object.insert(
            String::from(property_names::TRANSITIONS),
            JsonValue::Array(transitions),
        )?;

        Ok(json_object)
    }

    fn to_buffer(&self, skip_signature: bool) -> Result<Vec<u8>, ProtocolError> {
        let mut result_buf = self.protocol_version.encode_var_vec();
        let value = self.to_object(skip_signature)?;

        let map = CborValue::serialized(&value)
            .map_err(|e| ProtocolError::EncodingError(e.to_string()))?;

        let mut canonical_map: CborCanonicalMap = map.try_into()?;
        canonical_map.remove(property_names::PROTOCOL_VERSION);

        // Replace binary fields individually for every transition using respective data contract
        if let Some(CborValue::Array(ref mut transitions)) =
            canonical_map.get_mut(&CborValue::Text(property_names::TRANSITIONS.to_string()))
        {
            for (i, cbor_transition) in transitions.iter_mut().enumerate() {
                let transition = self
                    .transitions
                    .get(i)
                    .context(format!("transition with index {} doesn't exist", i))?;

                let (identifier_properties, binary_properties) = transition
                    .base()
                    .data_contract
                    .get_identifiers_and_binary_paths(&self.transitions[i].base().document_type)?;

                if transition.get_updated_at().is_none() {
                    cbor_transition.remove("$updatedAt");
                }

                cbor_transition.replace_paths(
                    identifier_properties
                        .into_iter()
                        .chain(binary_properties)
                        .chain(document_base_transition::IDENTIFIER_FIELDS)
                        .chain(document_create_transition::BINARY_FIELDS),
                    FieldType::ArrayInt,
                    FieldType::Bytes,
                );
            }
        }

        canonical_map.replace_paths(
            Self::binary_property_paths()
                .into_iter()
                .chain(Self::identifiers_property_paths()),
            FieldType::ArrayInt,
            FieldType::Bytes,
        );

        if !skip_signature {
            if self.signature.is_none() {
                canonical_map.insert(property_names::SIGNATURE, CborValue::Null)
            }
            if self.signature_public_key_id.is_none() {
                canonical_map.insert(property_names::SIGNATURE_PUBLIC_KEY_ID, CborValue::Null)
            }
        }

        canonical_map.sort_canonical();

        let mut buffer = canonical_map
            .to_bytes()
            .map_err(|e| ProtocolError::EncodingError(e.to_string()))?;
        result_buf.append(&mut buffer);

        Ok(result_buf)
    }
}

impl StateTransitionLike for DocumentsBatchTransition {
    fn get_protocol_version(&self) -> u32 {
        self.protocol_version
    }

    fn get_signature(&self) -> &Vec<u8> {
        if let Some(ref signature) = self.signature {
            signature
        } else {
            // TODO This is temporary solution to not break the `get_signature()` method
            // TODO for other transitions
            todo!()
        }
    }

    fn get_type(&self) -> StateTransitionType {
        self.transition_type
    }

    fn set_signature(&mut self, signature: Vec<u8>) {
        self.signature = Some(signature);
    }
    fn get_execution_context(&self) -> &StateTransitionExecutionContext {
        &self.execution_context
    }

    fn get_execution_context_mut(&mut self) -> &mut StateTransitionExecutionContext {
        &mut self.execution_context
    }

    fn set_execution_context(&mut self, execution_context: StateTransitionExecutionContext) {
        self.execution_context = execution_context
    }
}

pub fn get_security_level_requirement(v: &JsonValue, default: SecurityLevel) -> SecurityLevel {
    let maybe_security_level = v.get_u64(property_names::SECURITY_LEVEL_REQUIREMENT);
    match maybe_security_level {
        Ok(some_level) => (some_level as u8).try_into().unwrap_or(default),
        Err(_) => default,
    }
}

#[cfg(test)]
mod test {
    use std::sync::Arc;

    use serde_json::json;

    use crate::{
        document::{
            document_factory::DocumentFactory,
            fetch_and_validate_data_contract::DataContractFetcherAndValidator,
        },
        state_repository::MockStateRepositoryLike,
        tests::fixtures::{
            get_data_contract_fixture, get_document_transitions_fixture,
            get_document_validator_fixture, get_documents_fixture,
        },
    };

    use super::{document_transition::Action, *};

    #[test]
    fn should_return_highest_sec_level_for_all_transitions() {
        let mut data_contract = get_data_contract_fixture(None);
        data_contract
            .documents
            .get_mut("niceDocument")
            .unwrap()
            .insert(
                property_names::SECURITY_LEVEL_REQUIREMENT.to_string(),
                json!(SecurityLevel::MEDIUM),
            )
            .unwrap();
        data_contract
            .documents
            .get_mut("prettyDocument")
            .unwrap()
            .insert(
                property_names::SECURITY_LEVEL_REQUIREMENT.to_string(),
                json!(SecurityLevel::MASTER),
            )
            .unwrap();

        // 0 is niceDocument,
        // 1 and 2 are pretty documents,
        // 3 and 4 are indexed documents that do not have security level specified
        let documents = get_documents_fixture(data_contract).unwrap();
        let medium_security_document = documents.get(0).unwrap();
        let master_security_document = documents.get(1).unwrap();
        let no_security_level_document = documents.get(3).unwrap();

        let document_factory = DocumentFactory::new(
            1,
            get_document_validator_fixture(),
            DataContractFetcherAndValidator::new(Arc::new(MockStateRepositoryLike::new())),
        );

        let batch_transition = document_factory
            .create_state_transition(vec![(
                Action::Create,
                vec![medium_security_document.to_owned()],
            )])
            .expect("batch transition should be created");

        assert_eq!(
            SecurityLevel::MEDIUM,
            batch_transition.get_security_level_requirement()
        );

        let batch_transition = document_factory
            .create_state_transition(vec![(
                Action::Create,
                vec![
                    medium_security_document.to_owned(),
                    master_security_document.to_owned(),
                ],
            )])
            .expect("batch transition should be created");

        assert_eq!(
            SecurityLevel::MASTER,
            batch_transition.get_security_level_requirement()
        );

        let batch_transition = document_factory
            .create_state_transition(vec![(
                Action::Create,
                vec![no_security_level_document.to_owned()],
            )])
            .expect("batch transition should be created");

        assert_eq!(
            SecurityLevel::HIGH,
            batch_transition.get_security_level_requirement()
        );
    }

    #[test]
    fn should_convert_to_batch_transition_to_the_buffer() {
        let transition_id_base58 = "6o8UfoeE2s7dTkxxyPCixuxe8TM5DtCGHTMummUN6t5M";
        let expected_bytes_hex ="01a5647479706501676f776e657249645820a858bdc49c968148cd12648ee048d34003e9da3fbf2cbc62c31bb4c717bf690d697369676e6174757265f76b7472616e736974696f6e7381a7632469645820561b9b2e90b7c0ca355f729777b45bc646a18f5426a9462f0333c766135a3120646e616d656543757469656524747970656c6e696365446f63756d656e746724616374696f6e006824656e74726f707958202cdbaeda81c14765ba48432ff5cc900a7cacd4538b817fc71f38907aaa7023746a246372656174656441741b000001853a3602876f2464617461436f6e74726163744964582049aea5df2124a51d5d8dcf466e238fbc77fd72601be69daeb6dba75e8d26b30c747369676e61747572655075626c69634b65794964f7" ;
        let data_contract_id_base58 = "5xdDqypFMPfvF6UdWxefCGvRFyxgkPZCAK6TS4pvvw6T";
        let owner_id_base58 = "CL9ydpdxP4kQniGx6z5JUL8K72gnwcemKT2aJmh7sdwJ";
        let entropy_base64 = "LNuu2oHBR2W6SEMv9cyQCnys1FOLgX/HHziQeqpwI3Q=";

        let transition_id =
            Identifier::from_string(transition_id_base58, Encoding::Base58).unwrap();
        let expected_bytes = hex::decode(expected_bytes_hex).unwrap();
        let data_contract_id =
            Identifier::from_string(data_contract_id_base58, Encoding::Base58).unwrap();
        let owner_id = Identifier::from_string(owner_id_base58, Encoding::Base58).unwrap();
        let entropy_bytes: [u8; 32] = base64::decode(entropy_base64).unwrap().try_into().unwrap();

        let mut data_contract = get_data_contract_fixture(Some(owner_id.clone()));
        data_contract.id = data_contract_id;

        let documents = get_documents_fixture(data_contract.clone()).unwrap();
        let mut document = documents.first().unwrap().to_owned();
        document.entropy = entropy_bytes;

        let transitions = get_document_transitions_fixture([(Action::Create, vec![document])]);
        let mut transition = transitions.first().unwrap().to_owned();
        if let DocumentTransition::Create(ref mut t) = transition {
            t.created_at = Some(1671718896263);
            t.base.id = transition_id;
        }

        let state_transition = DocumentsBatchTransition::from_raw_object(
            json!({
                "ownerId" : owner_id.as_bytes(),
                "transitions" : [transition.to_object().unwrap()],
            }),
            vec![data_contract],
        )
        .expect("transition should be created");

        let bytes = state_transition.to_buffer(false).unwrap();

        pretty_assertions::assert_eq!(expected_bytes.len(), bytes.len());
        pretty_assertions::assert_eq!(expected_bytes, bytes);
    }
}
