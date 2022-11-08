use std::collections::HashMap;
use std::convert::TryInto;

use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::data_contract::DataContract;
use crate::state_transition::state_transition_execution_context::StateTransitionExecutionContext;
use crate::{
    identity::{KeyID, SecurityLevel},
    state_transition::{
        StateTransitionConvert, StateTransitionIdentitySigned, StateTransitionLike,
        StateTransitionType,
    },
};
// TODO simplify imports
use crate::document::document_transition::DocumentTransitionObjectLike;
use crate::prelude::{DocumentTransition, Identifier};
use crate::util::json_value::{JsonValueExt, ReplaceWith};
use crate::version::LATEST_VERSION;
use crate::ProtocolError;

pub mod document_transition;
pub mod validation;

const PROPERTY_DATA_CONTRACT_ID: &str = "$dataContractId";
const PROPERTY_TRANSITIONS: &str = "transitions";
const PROPERTY_OWNER_ID: &str = "ownerId";
const PROPERTY_SIGNATURE_PUBLIC_KEY_ID: &str = "signaturePublicKeyId";
const PROPERTY_SIGNATURE: &str = "signature";
const PROPERTY_PROTOCOL_VERSION: &str = "protocolVersion";
const PROPERTY_SECURITY_LEVEL_REQUIREMENT: &str = "signatureSecurityLevelRequirement";
const DEFAULT_SECURITY_LEVEL: SecurityLevel = SecurityLevel::HIGH;

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
    pub signature_public_key_id: KeyID,
    pub signature: Vec<u8>,
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
            signature_public_key_id: 0,
            signature: vec![],
            execution_context: Default::default(),
        }
    }
}

impl DocumentsBatchTransition {
    // TODO (rs-dpp-feature): do not use [`JsonValue`] with constructors

    /// creates the instance of [`DocumentsBatchTransition`] from raw object
    pub fn from_raw_object(
        mut raw_object: JsonValue,
        data_contracts: Vec<DataContract>,
    ) -> Result<Self, ProtocolError> {
        let mut batch_transitions = DocumentsBatchTransition {
            protocol_version: raw_object
                .get_u64(PROPERTY_PROTOCOL_VERSION)
                // js-dpp allows `protocolVersion` to be undefined
                .unwrap_or(LATEST_VERSION as u64) as u32,
            signature: raw_object.get_bytes(PROPERTY_SIGNATURE).unwrap_or_default(),
            signature_public_key_id: raw_object
                .get_u64(PROPERTY_SIGNATURE_PUBLIC_KEY_ID)
                .unwrap_or_default(),
            owner_id: Identifier::from_bytes(&raw_object.get_bytes(PROPERTY_OWNER_ID)?)?,
            ..Default::default()
        };

        let mut document_transitions: Vec<DocumentTransition> = vec![];
        let maybe_transitions = raw_object.remove(PROPERTY_TRANSITIONS);
        if let Ok(JsonValue::Array(raw_transitions)) = maybe_transitions {
            //? what if we have to data contracts with the same id?
            let data_contracts_map: HashMap<Vec<u8>, DataContract> = data_contracts
                .into_iter()
                .map(|dc| (dc.id.to_buffer().to_vec(), dc))
                .collect();

            for raw_transition in raw_transitions {
                let id = raw_transition.get_bytes(PROPERTY_DATA_CONTRACT_ID)?;
                let data_contract = data_contracts_map.get(&id).ok_or_else(|| {
                    anyhow!(
                        "Data Contract doesn't exists for Transition: {:?}",
                        raw_transition
                    )
                })?;
                let document_transition =
                    DocumentTransition::from_raw_document(raw_transition, data_contract.clone())?;
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

    fn get_signature_public_key_id(&self) -> KeyID {
        self.signature_public_key_id
    }

    fn set_signature_public_key_id(&mut self, key_id: KeyID) {
        self.signature_public_key_id = key_id;
    }
}

impl StateTransitionConvert for DocumentsBatchTransition {
    fn binary_property_paths() -> Vec<&'static str> {
        vec![PROPERTY_SIGNATURE]
    }

    fn identifiers_property_paths() -> Vec<&'static str> {
        vec![PROPERTY_OWNER_ID]
    }

    fn signature_property_paths() -> Vec<&'static str> {
        vec![PROPERTY_SIGNATURE]
    }

    fn to_json(&self) -> Result<JsonValue, ProtocolError> {
        let mut json_value: JsonValue = serde_json::to_value(self)?;
        json_value.replace_binary_paths(Self::binary_property_paths(), ReplaceWith::Base64)?;
        json_value
            .replace_identifier_paths(Self::identifiers_property_paths(), ReplaceWith::Base58)?;

        let mut transitions = vec![];
        for transition in self.transitions.iter() {
            transitions.push(transition.to_json()?)
        }
        json_value.insert(
            String::from(PROPERTY_TRANSITIONS),
            JsonValue::Array(transitions),
        )?;

        Ok(json_value)
    }

    fn to_object(&self, skip_signature: bool) -> Result<JsonValue, ProtocolError> {
        let mut json_object: JsonValue = serde_json::to_value(self)?;
        json_object
            .replace_identifier_paths(Self::identifiers_property_paths(), ReplaceWith::Bytes)?;

        if skip_signature {
            if let JsonValue::Object(ref mut o) = json_object {
                for path in Self::signature_property_paths() {
                    o.remove(path);
                }
            }
        }
        let mut transitions = vec![];
        for transition in self.transitions.iter() {
            transitions.push(transition.to_object()?)
        }
        json_object.insert(
            String::from(PROPERTY_TRANSITIONS),
            JsonValue::Array(transitions),
        )?;

        Ok(json_object)
    }
}

impl StateTransitionLike for DocumentsBatchTransition {
    fn get_protocol_version(&self) -> u32 {
        self.protocol_version
    }

    fn get_signature(&self) -> &Vec<u8> {
        &self.signature
    }

    fn get_type(&self) -> StateTransitionType {
        self.transition_type
    }

    fn set_signature(&mut self, signature: Vec<u8>) {
        self.signature = signature;
    }
    fn get_execution_context(&self) -> &StateTransitionExecutionContext {
        &self.execution_context
    }

    fn set_execution_context(&mut self, execution_context: StateTransitionExecutionContext) {
        self.execution_context = execution_context
    }
}

pub fn get_security_level_requirement(v: &JsonValue, default: SecurityLevel) -> SecurityLevel {
    let maybe_security_level = v.get_u64(PROPERTY_SECURITY_LEVEL_REQUIREMENT);
    match maybe_security_level {
        Ok(some_level) => (some_level as usize).try_into().unwrap_or(default),
        Err(_) => default,
    }
}

#[cfg(test)]
mod test {
    use serde_json::json;

    use crate::{
        document::document_factory::DocumentFactory,
        mocks,
        tests::fixtures::{
            get_data_contract_fixture, get_document_validator_fixture, get_documents_fixture,
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
                PROPERTY_SECURITY_LEVEL_REQUIREMENT.to_string(),
                json!(SecurityLevel::MEDIUM),
            )
            .unwrap();
        data_contract
            .documents
            .get_mut("prettyDocument")
            .unwrap()
            .insert(
                PROPERTY_SECURITY_LEVEL_REQUIREMENT.to_string(),
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
            mocks::FetchAndValidateDataContract {},
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
}
