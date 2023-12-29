use dpp::prelude::Revision;

use dpp::state_transition::documents_batch_transition::document_create_transition::DocumentCreateTransition;

use dpp::document::INITIAL_REVISION;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=DocumentCreateTransition)]
#[derive(Debug, Clone)]
pub struct DocumentCreateTransitionWasm {
    inner: DocumentCreateTransition,
}

impl From<DocumentCreateTransition> for DocumentCreateTransitionWasm {
    fn from(v: DocumentCreateTransition) -> Self {
        Self { inner: v }
    }
}

impl From<DocumentCreateTransitionWasm> for DocumentCreateTransition {
    fn from(v: DocumentCreateTransitionWasm) -> Self {
        v.inner
    }
}

#[wasm_bindgen(js_class=DocumentCreateTransition)]
impl DocumentCreateTransitionWasm {
    // #[wasm_bindgen(constructor)]
    // pub fn from_object(
    //     raw_object: JsValue,
    //     data_contract: &DataContractWasm,
    // ) -> Result<DocumentCreateTransitionWasm, JsValue> {
    //     let data_contract: DataContract = data_contract.clone().into();
    //     let mut value = raw_object.with_serde_to_platform_value_map()?;
    //     // let document_type = value
    //     //     .get_string(dpp::document::extended_document::property_names::DOCUMENT_TYPE_NAME)
    //     //     .map_err(ProtocolError::ValueError)
    //     //     .with_js_error()?;
    //     //
    //     // let (identifier_paths, _): (Vec<_>, Vec<_>) = data_contract
    //     //     .get_identifiers_and_binary_paths_owned(document_type.as_str())
    //     //     .with_js_error()?;
    //     // value
    //     //     .replace_at_paths(identifier_paths, ReplacementType::Identifier)
    //     //     .map_err(ProtocolError::ValueError)
    //     //     .with_js_error()?;
    //     let transition =
    //         DocumentCreateTransition::from_value_map(value, data_contract).with_js_error()?;
    //
    //     Ok(transition.into())
    // }
    //
    // // DocumentCreateTransition
    // #[wasm_bindgen(js_name=getEntropy)]
    // pub fn entropy(&self) -> Vec<u8> {
    //     self.inner.entropy().to_vec()
    // }
    //
    // #[wasm_bindgen(js_name=getCreatedAt)]
    // pub fn created_at(&self) -> Option<js_sys::Date> {
    //     self.inner
    //         .created_at()
    //         .map(|timestamp| js_sys::Date::new(&JsValue::from_f64(timestamp as f64)))
    // }
    //
    // #[wasm_bindgen(js_name=getUpdatedAt)]
    // pub fn updated_at(&self) -> Option<js_sys::Date> {
    //     self.inner
    //         .updated_at()
    //         .map(|timestamp| js_sys::Date::new(&JsValue::from_f64(timestamp as f64)))
    // }

    #[wasm_bindgen(js_name=getRevision)]
    pub fn revision(&self) -> Revision {
        INITIAL_REVISION
    }

    #[wasm_bindgen(getter, js_name=INITIAL_REVISION)]
    pub fn initial_revision() -> u32 {
        INITIAL_REVISION as u32
    }

    //     // AbstractDocumentTransitionMethods
    //     #[wasm_bindgen(js_name=getId)]
    //     pub fn id(&self) -> IdentifierWrapper {
    //         self.inner.base().id().into()
    //     }
    //
    //     #[wasm_bindgen(js_name=getType)]
    //     pub fn document_type(&self) -> String {
    //         self.inner.base().document_type_name().clone()
    //     }
    //
    //     #[wasm_bindgen(js_name=getAction)]
    //     pub fn action(&self) -> u8 {
    //         DocumentTransitionActionType::Create as u8
    //     }
    //
    //     #[wasm_bindgen(js_name=getDataContractId)]
    //     pub fn data_contract_id(&self) -> IdentifierWrapper {
    //         self.inner.base().data_contract_id().into()
    //     }
    //
    //     #[wasm_bindgen]
    //     pub fn get(&self, path: String, data_contract: &DataContractWasm) -> Result<JsValue, JsValue> {
    //         let document_data = self.inner.data();
    //
    //         let value = if let Ok(value) = document_data.get_at_path(&path) {
    //             value.clone()
    //         } else {
    //             return Ok(JsValue::undefined());
    //         };
    //
    //         match self.get_binary_type_of_path(&path) {
    //             BinaryType::Buffer => {
    //                 let bytes = value
    //                     .to_bytes()
    //                     .map_err(ProtocolError::ValueError)
    //                     .with_js_error()?;
    //                 let buffer = Buffer::from_bytes(&bytes);
    //                 return Ok(buffer.into());
    //             }
    //             BinaryType::Identifier => {
    //                 let buffer = value
    //                     .to_hash256()
    //                     .map_err(ProtocolError::ValueError)
    //                     .with_js_error()?;
    //                 let id = <IdentifierWrapper as convert::From<Identifier>>::from(Identifier::from(
    //                     buffer,
    //                 ));
    //                 return Ok(id.into());
    //             }
    //             BinaryType::None => {
    //                 // Do nothing. If is 'None' it means that binary may contain binary data
    //                 // or may not captain it at all
    //             }
    //         }
    //
    //         let json_value: JsonValue = value
    //             .clone()
    //             .try_into()
    //             .map_err(ProtocolError::ValueError)
    //             .with_js_error()?;
    //         let map = value
    //             .to_btree_ref_string_map()
    //             .map_err(ProtocolError::ValueError)
    //             .with_js_error()?;
    //         let js_value = json_value.serialize(&serde_wasm_bindgen::Serializer::json_compatible())?;
    //
    //         let document_type = data_contract
    //             .inner()
    //             .document_type_for_name(self.inner.base().document_type_name())
    //             .with_js_error()?;
    //         let identifier_paths = document_type.identifier_paths();
    //         let binary_paths = document_type.binary_paths();
    //
    //         for property_path in identifier_paths {
    //             if property_path.starts_with(&path) {
    //                 let (_, suffix) = property_path.split_at(path.len() + 1);
    //
    //                 if let Some(bytes) = map
    //                     .get_optional_bytes_at_path(suffix)
    //                     .map_err(ProtocolError::ValueError)
    //                     .with_js_error()?
    //                 {
    //                     let id = <IdentifierWrapper as convert::From<Identifier>>::from(
    //                         Identifier::from_bytes(&bytes).unwrap(),
    //                     );
    //                     lodash_set(&js_value, suffix, id.into());
    //                 }
    //             }
    //         }
    //
    //         for property_path in binary_paths {
    //             if property_path.starts_with(&path) {
    //                 let (_, suffix) = property_path.split_at(path.len() + 1);
    //
    //                 if let Some(bytes) = map
    //                     .get_optional_bytes_at_path(suffix)
    //                     .map_err(ProtocolError::ValueError)
    //                     .with_js_error()?
    //                 {
    //                     let buffer = Buffer::from_bytes(&bytes);
    //                     lodash_set(&js_value, suffix, buffer.into());
    //                 }
    //             }
    //         }
    //         Ok(js_value)
    //     }
    //
    //     // DocumentTransitionObjectLike
    //     #[wasm_bindgen(js_name=toObject)]
    //     pub fn to_object(&self, options: &JsValue) -> Result<JsValue, JsValue> {
    //         let (identifiers_paths, binary_paths) = self
    //             .inner
    //             .base()
    //             .data_contract
    //             .get_identifiers_and_binary_paths(&self.inner.base().document_type_name)
    //             .with_js_error()?;
    //
    //         to_object(
    //             self.inner.to_object().with_js_error()?,
    //             options,
    //             identifiers_paths
    //                 .into_iter()
    //                 .chain(document_create_transition::v0::IDENTIFIER_FIELDS),
    //             binary_paths
    //                 .into_iter()
    //                 .chain(document_create_transition::v0::BINARY_FIELDS),
    //         )
    //     }
    //
    //     #[wasm_bindgen(js_name=toJSON)]
    //     pub fn to_json(&self) -> Result<JsValue, JsValue> {
    //         let value = self.inner.to_json().with_js_error()?;
    //         let serializer = serde_wasm_bindgen::Serializer::json_compatible();
    //         let js_value = value.serialize(&serializer)?;
    //         Ok(js_value)
    //     }
    //
    //     // AbstractDataDocumentTransition
    //     #[wasm_bindgen(js_name=getData)]
    //     pub fn get_data(&self) -> Result<JsValue, JsValue> {
    //         let json_data = if let Some(ref data) = self.inner.data() {
    //             data.to_json_value()
    //                 .map_err(ProtocolError::ValueError)
    //                 .with_js_error()?
    //         } else {
    //             return Ok(JsValue::undefined());
    //         };
    //
    //         let js_value = json_data.serialize(&serde_wasm_bindgen::Serializer::json_compatible())?;
    //         Ok(js_value)
    //     }
    // }
    //
    // impl DocumentCreateTransitionWasm {
    //     fn get_binary_type_of_path(&self, path: &String) -> BinaryType {
    //         let maybe_binary_properties = self
    //             .inner
    //             .base
    //             .data_contract
    //             .get_binary_properties(&self.inner.base.document_type_name);
    //
    //         if let Ok(binary_properties) = maybe_binary_properties {
    //             if let Some(data) = binary_properties.get(path) {
    //                 if data.is_type_of_identifier() {
    //                     return BinaryType::Identifier;
    //                 }
    //                 return BinaryType::Buffer;
    //             }
    //         }
    //         BinaryType::None
    //     }
}
