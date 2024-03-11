use anyhow::anyhow;

use dpp::document::document_factory::DocumentFactory;
use std::collections::{BTreeMap, HashMap, HashSet};

use wasm_bindgen::prelude::*;

use crate::document::errors::InvalidActionNameError;
use crate::document::platform_value::Bytes32;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::document::Document;

use dpp::prelude::ExtendedDocument;

use dpp::identifier::Identifier;
use dpp::state_transition::documents_batch_transition::document_transition::action_type::DocumentTransitionActionType;
use dpp::version::PlatformVersion;
use std::convert::TryFrom;

use crate::document_batch_transition::DocumentsBatchTransitionWasm;
use crate::entropy_generator::ExternalEntropyGenerator;
use crate::{
    identifier::identifier_from_js_value,
    utils::{IntoWasm, ToSerdeJSONExt, WithJsError},
    DataContractWasm, ExtendedDocumentWasm,
};

#[wasm_bindgen(js_name=DocumentTransitions)]
#[derive(Debug, Default)]
pub struct DocumentTransitions {
    create: Vec<ExtendedDocumentWasm>,
    replace: Vec<ExtendedDocumentWasm>,
    delete: Vec<ExtendedDocumentWasm>,
}

#[wasm_bindgen(js_class=DocumentTransitions)]
impl DocumentTransitions {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Default::default()
    }

    #[wasm_bindgen(js_name = "addTransitionCreate")]
    pub fn add_transition_create(&mut self, transition: ExtendedDocumentWasm) {
        self.create.push(transition)
    }

    #[wasm_bindgen(js_name = "addTransitionReplace")]
    pub fn add_transition_replace(&mut self, transition: ExtendedDocumentWasm) {
        self.replace.push(transition)
    }

    #[wasm_bindgen(js_name = "addTransitionDelete")]
    pub fn add_transition_delete(&mut self, transition: ExtendedDocumentWasm) {
        self.delete.push(transition)
    }
}

#[wasm_bindgen(js_name = DocumentFactory)]
pub struct DocumentFactoryWASM(DocumentFactory);

#[wasm_bindgen(js_class=DocumentFactory)]
impl DocumentFactoryWASM {
    #[wasm_bindgen(constructor)]
    pub fn new(
        protocol_version: u32,
        external_entropy_generator_arg: Option<ExternalEntropyGenerator>,
    ) -> Result<DocumentFactoryWASM, JsValue> {
        let factory = if let Some(external_entropy_generator) = external_entropy_generator_arg {
            DocumentFactory::new_with_entropy_generator(
                protocol_version,
                Box::new(external_entropy_generator),
            )
            .with_js_error()?
        } else {
            DocumentFactory::new(protocol_version).with_js_error()?
        };

        Ok(DocumentFactoryWASM(factory))
    }

    #[wasm_bindgen]
    pub fn create(
        &self,
        data_contract: &DataContractWasm,
        js_owner_id: &JsValue,
        document_type: &str,
        data: &JsValue,
    ) -> Result<ExtendedDocumentWasm, JsValue> {
        let owner_id = identifier_from_js_value(js_owner_id)?;
        let dynamic_data = data.with_serde_to_platform_value()?;

        let extended_document = self
            .0
            .create_extended_document(
                data_contract.inner(),
                owner_id,
                document_type.to_string(),
                dynamic_data,
            )
            .with_js_error()?;

        Ok(extended_document.into())
    }

    #[wasm_bindgen(js_name=createStateTransition)]
    pub fn create_state_transition(
        &self,
        documents: &JsValue,
        nonce_counter_value: &js_sys::Object, //IdentityID/ContractID -> nonce
    ) -> Result<DocumentsBatchTransitionWasm, JsValue> {
        let mut nonce_counter = BTreeMap::new();
        let mut contract_ids_to_check = HashSet::<&Identifier>::new();

        // TODO: move to a function and handle errors instead of doing unwraps
        {
            js_sys::Object::entries(nonce_counter_value)
                .iter()
                .for_each(|entry| {
                    let key_value = js_sys::Array::from(&entry);
                    let identity_id = identifier_from_js_value(&key_value.get(0)).unwrap();
                    let contract_ids = key_value.get(1);
                    let contract_ids = js_sys::Object::try_from(&contract_ids).unwrap();

                    js_sys::Object::entries(contract_ids)
                        .iter()
                        .for_each(|entry| {
                            let key_value = js_sys::Array::from(&entry);
                            let contract_id = identifier_from_js_value(&key_value.get(0)).unwrap();
                            let nonce = key_value.get(1).as_f64().unwrap() as u64;
                            nonce_counter.insert((identity_id, contract_id), nonce);
                        });
                });
        }

        nonce_counter.iter().for_each(|((_, contract_id), _)| {
            contract_ids_to_check.insert(contract_id);
        });

        let documents_by_action = extract_documents_by_action(documents)?;

        for (_, documents) in documents_by_action.iter() {
            for document in documents.iter() {
                if !contract_ids_to_check.contains(&document.data_contract().id()) {
                    return Err(JsValue::from_str(
                        "Document's data contract is not in the nonce counter",
                    ));
                }
            }
        }

        let documents: Vec<(
            DocumentTransitionActionType,
            Vec<(Document, DocumentTypeRef, Bytes32)>,
        )> = documents_by_action
            .iter()
            .map(|(action_type, documents)| {
                let documents_with_refs: Vec<(Document, DocumentTypeRef, Bytes32)> = documents
                    .iter()
                    .map(|extended_document| {
                        (
                            extended_document.document().clone(),
                            extended_document
                                .data_contract()
                                .document_type_for_name(extended_document.document_type_name())
                                .expect("should be able to get document type"),
                            extended_document.entropy().to_owned(),
                        )
                    })
                    .collect();

                (*action_type, documents_with_refs)
            })
            .collect();

        let batch_transition = self
            .0
            .create_state_transition(documents, &mut nonce_counter)
            .with_js_error()?;

        Ok(batch_transition.into())
    }
    //
    // #[wasm_bindgen(js_name=createFromObject)]
    // pub async fn create_from_object(
    //     &self,
    //     raw_document_js: JsValue,
    //     options: JsValue,
    // ) -> Result<ExtendedDocumentWasm, JsValue> {
    //     let mut raw_document = raw_document_js.with_serde_to_platform_value()?;
    //     let options: FactoryOptions = if !options.is_undefined() && options.is_object() {
    //         let raw_options = options.with_serde_to_json_value()?;
    //         serde_json::from_value(raw_options).with_js_error()?
    //     } else {
    //         Default::default()
    //     };
    //     raw_document
    //         .replace_at_paths(
    //             extended_document::IDENTIFIER_FIELDS,
    //             ReplacementType::Identifier,
    //         )
    //         .map_err(ProtocolError::ValueError)
    //         .with_js_error()?;
    //
    //     let mut document = self
    //         .0
    //         .create_from_object(raw_document, options)
    //         .await
    //         .with_js_error()?;
    //     let (identifier_paths, binary_paths): (Vec<_>, Vec<_>) = document
    //         .get_identifiers_and_binary_paths_owned()
    //         .with_js_error()?;
    //     // When data contract is available, replace remaining dynamic paths
    //     let document_data = document.properties_as_mut();
    //     document_data
    //         .replace_at_paths(identifier_paths, ReplacementType::Identifier)
    //         .map_err(ProtocolError::ValueError)
    //         .with_js_error()?;
    //     document_data
    //         .replace_at_paths(binary_paths, ReplacementType::BinaryBytes)
    //         .map_err(ProtocolError::ValueError)
    //         .with_js_error()?;
    //     Ok(document.into())
    // }
    //
    // #[wasm_bindgen(js_name=createFromBuffer)]
    // pub async fn create_from_buffer(
    //     &self,
    //     buffer: Vec<u8>,
    //     options: &JsValue,
    // ) -> Result<ExtendedDocumentWasm, JsValue> {
    //     // let options: FactoryOptions = if !options.is_undefined() && options.is_object() {
    //     //     let raw_options = options.with_serde_to_json_value()?;
    //     //     serde_json::from_value(raw_options).with_js_error()?
    //     // } else {
    //     //     Default::default()
    //     // };
    //
    //     let document = self
    //         .0
    //         .create_from_buffer(buffer, options)
    //         .await
    //         .with_js_error()?;
    //
    //     Ok(document.into())
    // }

    #[wasm_bindgen(js_name=createExtendedDocumentFromDocumentBuffer)]
    pub fn create_extended_from_document_buffer(
        &self,
        buffer: Vec<u8>,
        document_type: &str,
        data_contract: &DataContractWasm,
    ) -> Result<ExtendedDocumentWasm, JsValue> {
        let platform_version = PlatformVersion::first();

        self.0
            .create_extended_from_document_buffer(
                buffer.as_slice(),
                document_type,
                &data_contract.to_owned().into(),
                platform_version,
            )
            .map(|document| document.into())
            .with_js_error()
    }
}
//
fn extract_documents_by_action(
    documents: &JsValue,
) -> Result<HashMap<DocumentTransitionActionType, Vec<ExtendedDocument>>, JsValue> {
    check_actions(documents)?;

    let mut documents_by_action: HashMap<DocumentTransitionActionType, Vec<ExtendedDocument>> =
        Default::default();

    let documents_create = extract_documents_of_action(documents, "create").with_js_error()?;
    let documents_replace = extract_documents_of_action(documents, "replace").with_js_error()?;
    let documents_delete = extract_documents_of_action(documents, "delete").with_js_error()?;

    documents_by_action.insert(DocumentTransitionActionType::Create, documents_create);
    documents_by_action.insert(DocumentTransitionActionType::Replace, documents_replace);
    documents_by_action.insert(DocumentTransitionActionType::Delete, documents_delete);

    Ok(documents_by_action)
}

fn check_actions(documents: &JsValue) -> Result<(), JsValue> {
    if !documents.is_object() {
        return Err(anyhow!("Expected documents to be an object")).with_js_error();
    }

    let documents_object = js_sys::Object::from(documents.clone());

    let actions: js_sys::Array = js_sys::Object::keys(&documents_object);

    for action in actions.iter() {
        let action_string: String = action
            .as_string()
            .ok_or_else(|| anyhow!("Expected all keys to be strings"))
            .with_js_error()?;

        DocumentTransitionActionType::try_from(action_string.as_str())
            .map_err(|_| InvalidActionNameError::new(vec![action.clone()]))?;
    }

    Ok(())
}

fn extract_documents_of_action(
    documents: &JsValue,
    action: &str,
) -> Result<Vec<ExtendedDocument>, anyhow::Error> {
    let documents_with_action =
        js_sys::Reflect::get(documents, &action.to_string().into()).unwrap_or(JsValue::NULL);

    if documents_with_action.is_null() || documents_with_action.is_undefined() {
        return Ok(vec![]);
    }

    let documents_array = js_sys::Array::try_from(documents_with_action)
        .map_err(|e| anyhow!("property '{}' isn't an array: {}", action, e))?;

    documents_array
        .iter()
        .map(|js_document| {
            js_document
                .to_wasm::<ExtendedDocumentWasm>("ExtendedDocument")
                .map_err(|e| {
                    anyhow!(
                        "Element in '{}' isn't an Extended Document instance: {:#?}",
                        action,
                        e
                    )
                })
                .map(|wasm_doc| wasm_doc.clone().into())
        })
        .collect()
}
