use crate::data_contract::DataContractWasm;
use crate::enums::platform::PlatformVersionWasm;
use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::IdentifierWasm;
use crate::serialization;
use crate::utils::ToSerdeJSONExt;
use dpp::document::serialization_traits::{
    DocumentJsonMethodsV0, DocumentPlatformConversionMethodsV0, DocumentPlatformValueMethodsV0,
};
use dpp::document::{Document, DocumentV0, DocumentV0Getters, DocumentV0Setters};
use dpp::identifier::Identifier;
use dpp::platform_value::string_encoding::Encoding::{Base58, Base64, Hex};
use dpp::platform_value::string_encoding::encode;
use dpp::platform_value::Value;
use dpp::prelude::Revision;
use dpp::util::entropy_generator;
use dpp::util::entropy_generator::EntropyGenerator;
use dpp::version::PlatformVersion;
use serde_json::json;
use std::collections::BTreeMap;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

/// DocumentWasm wraps a Document and adds metadata fields that are not part of the core Document.
#[derive(Clone)]
#[wasm_bindgen(js_name = Document)]
pub struct DocumentWasm {
    pub(crate) document: Document,
    pub(crate) data_contract_id: IdentifierWasm,
    pub(crate) document_type_name: String,
    pub(crate) entropy: Option<[u8; 32]>,
}

impl From<&DocumentWasm> for Document {
    fn from(wasm_doc: &DocumentWasm) -> Self {
        wasm_doc.document.clone()
    }
}

impl From<DocumentWasm> for Document {
    fn from(wasm_doc: DocumentWasm) -> Self {
        wasm_doc.document
    }
}

impl DocumentWasm {
    /// Create a new DocumentWasm with metadata
    pub fn new(
        document: Document,
        data_contract_id: Identifier,
        document_type_name: String,
        entropy: Option<[u8; 32]>,
    ) -> Self {
        DocumentWasm {
            document,
            data_contract_id: data_contract_id.into(),
            document_type_name,
            entropy,
        }
    }

    /// Access the inner document
    pub fn inner(&self) -> &Document {
        &self.document
    }

    /// Mutable access to the inner document
    pub fn inner_mut(&mut self) -> &mut Document {
        &mut self.document
    }

    pub fn rs_get_owner_id(&self) -> Identifier {
        self.document.owner_id()
    }

    pub fn rs_get_id(&self) -> Identifier {
        self.document.id()
    }

    pub fn rs_get_data_contract_id(&self) -> Identifier {
        self.data_contract_id.into()
    }

    pub fn rs_get_entropy(&self) -> Option<[u8; 32]> {
        self.entropy
    }

    pub fn rs_get_properties(&self) -> BTreeMap<String, Value> {
        self.document.properties().clone()
    }

    pub fn set_data_contract_id(&mut self, data_contract_id: &IdentifierWasm) {
        self.data_contract_id = *data_contract_id;
    }
}

#[wasm_bindgen(js_class = Document)]
impl DocumentWasm {
    #[wasm_bindgen(getter = __type)]
    pub fn type_name(&self) -> String {
        "Document".to_string()
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name() -> String {
        "Document".to_string()
    }

    #[wasm_bindgen(constructor)]
    pub fn constructor(
        js_raw_document: JsValue,
        js_document_type_name: &str,
        js_revision: u64,
        #[wasm_bindgen(unchecked_param_type = "Identifier | Uint8Array | string")]
        js_data_contract_id: &JsValue,
        #[wasm_bindgen(unchecked_param_type = "Identifier | Uint8Array | string")]
        js_owner_id: &JsValue,
        #[wasm_bindgen(unchecked_param_type = "Identifier | Uint8Array | string")]
        js_document_id: &JsValue,
    ) -> WasmDppResult<DocumentWasm> {
        let data_contract_id: Identifier = IdentifierWasm::try_from(js_data_contract_id)?.into();
        let owner_id: Identifier = IdentifierWasm::try_from(js_owner_id)?.into();
        let revision = Revision::from(js_revision);
        let properties = js_raw_document.with_serde_to_platform_value_map()?;

        let entropy = entropy_generator::DefaultEntropyGenerator
            .generate()
            .map_err(|err| WasmDppError::serialization(err.to_string()))?;

        let document_id: Identifier = match js_document_id.is_undefined() {
            true => crate::utils::generate_document_id_v0(
                &data_contract_id,
                &owner_id,
                js_document_type_name,
                &entropy,
            )?,
            false => IdentifierWasm::try_from(js_document_id)?.into(),
        };

        let document = Document::V0(DocumentV0 {
            id: document_id,
            owner_id,
            properties,
            revision: Some(revision),
            created_at: None,
            updated_at: None,
            transferred_at: None,
            created_at_block_height: None,
            updated_at_block_height: None,
            transferred_at_block_height: None,
            created_at_core_block_height: None,
            updated_at_core_block_height: None,
            transferred_at_core_block_height: None,
            creator_id: None,
        });

        Ok(DocumentWasm::new(
            document,
            data_contract_id,
            js_document_type_name.to_string(),
            Some(entropy),
        ))
    }

    #[wasm_bindgen(getter=id)]
    pub fn get_id(&self) -> IdentifierWasm {
        self.document.id().into()
    }

    #[wasm_bindgen(getter=entropy)]
    pub fn get_entropy(&self) -> Option<Vec<u8>> {
        self.entropy.map(|entropy| entropy.to_vec())
    }

    #[wasm_bindgen(getter=dataContractId)]
    pub fn get_data_contract_id(&self) -> IdentifierWasm {
        self.data_contract_id
    }

    #[wasm_bindgen(getter=ownerId)]
    pub fn get_owner_id(&self) -> IdentifierWasm {
        self.document.owner_id().into()
    }

    #[wasm_bindgen(getter=properties)]
    pub fn get_properties(&self) -> WasmDppResult<JsValue> {
        let properties_value = Value::Map(
            self.document
                .properties()
                .iter()
                .map(|(k, v)| (Value::Text(k.clone()), v.clone()))
                .collect(),
        );
        serialization::platform_value_to_object(&properties_value)
    }

    #[wasm_bindgen(getter=revision)]
    pub fn get_revision(&self) -> Option<u64> {
        self.document.revision()
    }

    #[wasm_bindgen(getter=createdAt)]
    pub fn get_created_at(&self) -> Option<u64> {
        self.document.created_at()
    }

    #[wasm_bindgen(getter=updatedAt)]
    pub fn get_updated_at(&self) -> Option<u64> {
        self.document.updated_at()
    }

    #[wasm_bindgen(getter=transferredAt)]
    pub fn get_transferred_at(&self) -> Option<u64> {
        self.document.transferred_at()
    }

    #[wasm_bindgen(getter=createdAtBlockHeight)]
    pub fn get_created_at_block_height(&self) -> Option<u64> {
        self.document.created_at_block_height()
    }

    #[wasm_bindgen(getter=updatedAtBlockHeight)]
    pub fn get_updated_at_block_height(&self) -> Option<u64> {
        self.document.updated_at_block_height()
    }

    #[wasm_bindgen(getter=transferredAtBlockHeight)]
    pub fn get_transferred_at_block_height(&self) -> Option<u64> {
        self.document.transferred_at_block_height()
    }

    #[wasm_bindgen(getter=createdAtCoreBlockHeight)]
    pub fn get_created_at_core_block_height(&self) -> Option<u32> {
        self.document.created_at_core_block_height()
    }

    #[wasm_bindgen(getter=updatedAtCoreBlockHeight)]
    pub fn get_updated_at_core_block_height(&self) -> Option<u32> {
        self.document.updated_at_core_block_height()
    }

    #[wasm_bindgen(getter=transferredAtCoreBlockHeight)]
    pub fn get_transferred_at_core_block_height(&self) -> Option<u32> {
        self.document.transferred_at_core_block_height()
    }

    #[wasm_bindgen(getter=documentTypeName)]
    pub fn get_document_type_name(&self) -> String {
        self.document_type_name.clone()
    }

    #[wasm_bindgen(setter=id)]
    pub fn set_id(
        &mut self,
        #[wasm_bindgen(unchecked_param_type = "Identifier | Uint8Array | string")] id: &JsValue,
    ) -> WasmDppResult<()> {
        let identifier: Identifier = IdentifierWasm::try_from(id)?.into();
        self.document.set_id(identifier);
        Ok(())
    }

    #[wasm_bindgen(setter=entropy)]
    pub fn set_entropy(&mut self, entropy: JsValue) -> WasmDppResult<()> {
        if entropy.is_undefined() {
            self.entropy = None;
            return Ok(());
        }

        let value = entropy.with_serde_to_platform_value()?;
        let bytes = value.as_bytes().ok_or_else(|| {
            WasmDppError::invalid_argument("Entropy must be provided as a byte array")
        })?;

        let mut entropy_bytes = [0u8; 32];
        let len = bytes.len().min(32);
        entropy_bytes[..len].copy_from_slice(&bytes[..len]);
        self.entropy = Some(entropy_bytes);

        Ok(())
    }

    #[wasm_bindgen(setter=dataContractId)]
    pub fn set_js_data_contract_id(
        &mut self,
        #[wasm_bindgen(unchecked_param_type = "Identifier | Uint8Array | string")]
        js_contract_id: &JsValue,
    ) -> WasmDppResult<()> {
        self.data_contract_id = IdentifierWasm::try_from(js_contract_id)?;
        Ok(())
    }

    #[wasm_bindgen(setter=ownerId)]
    pub fn set_owner_id(
        &mut self,
        #[wasm_bindgen(unchecked_param_type = "Identifier | Uint8Array | string")] id: &JsValue,
    ) -> WasmDppResult<()> {
        let identifier: Identifier = IdentifierWasm::try_from(id)?.into();
        self.document.set_owner_id(identifier);
        Ok(())
    }

    #[wasm_bindgen(setter=properties)]
    pub fn set_properties(&mut self, properties: JsValue) -> WasmDppResult<()> {
        let props = properties.with_serde_to_platform_value_map()?;
        *self.document.properties_mut() = props;
        Ok(())
    }

    #[wasm_bindgen(setter=revision)]
    pub fn set_revision(&mut self, revision: Option<u64>) {
        self.document.set_revision(revision);
    }

    #[wasm_bindgen(setter=createdAt)]
    pub fn set_created_at(&mut self, created_at: Option<u64>) {
        self.document.set_created_at(created_at);
    }

    #[wasm_bindgen(setter=updatedAt)]
    pub fn set_updated_at(&mut self, updated_at: Option<u64>) {
        self.document.set_updated_at(updated_at);
    }

    #[wasm_bindgen(setter=transferredAt)]
    pub fn set_transferred_at(&mut self, transferred_at: Option<u64>) {
        self.document.set_transferred_at(transferred_at);
    }

    #[wasm_bindgen(setter=createdAtBlockHeight)]
    pub fn set_created_at_block_height(&mut self, created_at_block_height: Option<u64>) {
        self.document
            .set_created_at_block_height(created_at_block_height);
    }

    #[wasm_bindgen(setter=updatedAtBlockHeight)]
    pub fn set_updated_at_block_height(&mut self, updated_at_block_height: Option<u64>) {
        self.document
            .set_updated_at_block_height(updated_at_block_height);
    }

    #[wasm_bindgen(setter=transferredAtBlockHeight)]
    pub fn set_transferred_at_block_height(&mut self, transferred_at_block_height: Option<u64>) {
        self.document
            .set_transferred_at_block_height(transferred_at_block_height);
    }

    #[wasm_bindgen(setter=createdAtCoreBlockHeight)]
    pub fn set_created_at_core_block_height(&mut self, created_at_core_block_height: Option<u32>) {
        self.document
            .set_created_at_core_block_height(created_at_core_block_height);
    }

    #[wasm_bindgen(setter=updatedAtCoreBlockHeight)]
    pub fn set_updated_at_core_block_height(&mut self, updated_at_core_block_height: Option<u32>) {
        self.document
            .set_updated_at_core_block_height(updated_at_core_block_height);
    }

    #[wasm_bindgen(setter=transferredAtCoreBlockHeight)]
    pub fn set_transferred_at_core_block_height(
        &mut self,
        transferred_at_core_block_height: Option<u32>,
    ) {
        self.document
            .set_transferred_at_core_block_height(transferred_at_core_block_height);
    }

    #[wasm_bindgen(setter=documentTypeName)]
    pub fn set_document_type_name(&mut self, document_type_name: &str) {
        self.document_type_name = document_type_name.to_string();
    }

    /// Convert to a JS object with binary fields as Uint8Array.
    #[wasm_bindgen(js_name = toObject)]
    pub fn to_object(&self) -> WasmDppResult<JsValue> {
        let mut map = self.document.to_map_value()?;
        // Add metadata fields not in core Document
        let data_contract_id: Identifier = self.data_contract_id.into();
        map.insert(
            "$dataContractId".to_string(),
            Value::Identifier(data_contract_id.into_buffer()),
        );
        map.insert(
            "$type".to_string(),
            Value::Text(self.document_type_name.clone()),
        );
        if let Some(entropy) = self.entropy {
            map.insert("$entropy".to_string(), Value::Bytes(entropy.to_vec()));
        }
        serialization::platform_value_to_object(&Value::Map(
            map.into_iter()
                .map(|(k, v)| (Value::Text(k), v))
                .collect(),
        ))
    }

    /// Create a Document from a JS object.
    #[wasm_bindgen(js_name = fromObject)]
    pub fn from_object(js_value: JsValue) -> WasmDppResult<DocumentWasm> {
        let mut map = js_value.with_serde_to_platform_value_map()?;

        // Extract metadata fields
        let data_contract_id = map
            .remove("$dataContractId")
            .ok_or_else(|| WasmDppError::invalid_argument("Missing $dataContractId"))?
            .into_identifier()
            .map_err(|e| WasmDppError::invalid_argument(e.to_string()))?;

        let document_type_name = map
            .remove("$type")
            .ok_or_else(|| WasmDppError::invalid_argument("Missing $type"))?
            .into_text()
            .map_err(|e| WasmDppError::invalid_argument(e.to_string()))?;

        let entropy = map.remove("$entropy").and_then(|v| {
            v.into_bytes().ok().and_then(|bytes| {
                if bytes.len() == 32 {
                    let mut arr = [0u8; 32];
                    arr.copy_from_slice(&bytes);
                    Some(arr)
                } else {
                    None
                }
            })
        });

        // Create Document from remaining fields
        let document = Document::from_platform_value(
            Value::Map(map.into_iter().map(|(k, v)| (Value::Text(k), v)).collect()),
            PlatformVersion::latest(),
        )?;

        Ok(DocumentWasm::new(
            document,
            data_contract_id,
            document_type_name,
            entropy,
        ))
    }

    /// Convert to a JSON-compatible JS object with binary fields as strings.
    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> WasmDppResult<JsValue> {
        // Use Document's to_json which handles proper JSON serialization
        let mut json_value = self.document.to_json(PlatformVersion::latest())?;

        // Add metadata fields not in core Document
        let obj = json_value.as_object_mut().ok_or_else(|| {
            WasmDppError::serialization("Expected JSON object from Document::to_json")
        })?;

        let data_contract_id: Identifier = self.data_contract_id.into();
        obj.insert("$dataContractId".to_string(), json!(data_contract_id));
        obj.insert("$type".to_string(), json!(self.document_type_name));

        if let Some(entropy) = self.entropy {
            // Use base58 encoding for entropy (consistent with Identifier encoding)
            obj.insert("$entropy".to_string(), json!(encode(&entropy, Base58)));
        }

        serialization::json_to_js_value(&json_value)
    }

    /// Create a Document from a JSON object.
    #[wasm_bindgen(js_name = fromJSON)]
    pub fn from_json(js_value: JsValue) -> WasmDppResult<DocumentWasm> {
        let mut map = js_value.with_serde_to_platform_value_map()?;

        // Extract metadata fields
        let data_contract_id = map
            .remove("$dataContractId")
            .ok_or_else(|| WasmDppError::invalid_argument("Missing $dataContractId"))?
            .into_identifier()
            .map_err(|e| WasmDppError::invalid_argument(e.to_string()))?;

        let document_type_name = map
            .remove("$type")
            .ok_or_else(|| WasmDppError::invalid_argument("Missing $type"))?
            .into_text()
            .map_err(|e| WasmDppError::invalid_argument(e.to_string()))?;

        let entropy = map.remove("$entropy").and_then(|v| {
            v.into_bytes().ok().and_then(|bytes| {
                if bytes.len() == 32 {
                    let mut arr = [0u8; 32];
                    arr.copy_from_slice(&bytes);
                    Some(arr)
                } else {
                    None
                }
            })
        });

        // Create Document from remaining fields
        let document = Document::from_platform_value(
            Value::Map(map.into_iter().map(|(k, v)| (Value::Text(k), v)).collect()),
            PlatformVersion::latest(),
        )?;

        Ok(DocumentWasm::new(
            document,
            data_contract_id,
            document_type_name,
            entropy,
        ))
    }

    #[wasm_bindgen(js_name=toBytes)]
    pub fn to_bytes(
        &self,
        data_contract: &DataContractWasm,
        js_platform_version: JsValue,
    ) -> WasmDppResult<Vec<u8>> {
        let platform_version = match js_platform_version.is_undefined() {
            true => PlatformVersionWasm::default(),
            false => PlatformVersionWasm::try_from(js_platform_version)?,
        };

        let document_type_ref = data_contract
            .get_document_type_ref_by_name(self.get_document_type_name())
            .map_err(|err| WasmDppError::invalid_argument(err.to_string()))?;

        self.document
            .serialize(
                document_type_ref,
                &data_contract.clone().into(),
                &platform_version.into(),
            )
            .map_err(Into::into)
    }

    #[wasm_bindgen(js_name=toHex)]
    pub fn to_hex(
        &self,
        data_contract: &DataContractWasm,
        js_platform_version: JsValue,
    ) -> WasmDppResult<String> {
        Ok(encode(
            self.to_bytes(data_contract, js_platform_version)?
                .as_slice(),
            Hex,
        ))
    }

    #[wasm_bindgen(js_name=toBase64)]
    pub fn to_base64(
        &self,
        data_contract: &DataContractWasm,
        js_platform_version: JsValue,
    ) -> WasmDppResult<String> {
        Ok(encode(
            self.to_bytes(data_contract, js_platform_version)?
                .as_slice(),
            Base64,
        ))
    }

    #[wasm_bindgen(js_name=fromBytes)]
    pub fn from_bytes(
        bytes: Vec<u8>,
        data_contract: &DataContractWasm,
        type_name: String,
        js_platform_version: JsValue,
    ) -> WasmDppResult<DocumentWasm> {
        let platform_version = match js_platform_version.is_undefined() {
            true => PlatformVersionWasm::default(),
            false => PlatformVersionWasm::try_from(js_platform_version)?,
        };

        let document_type_ref = data_contract
            .get_document_type_ref_by_name(type_name.clone())
            .map_err(|err| WasmDppError::invalid_argument(err.to_string()))?;

        let document = Document::from_bytes(
            bytes.as_slice(),
            document_type_ref,
            &platform_version.into(),
        )?;

        Ok(DocumentWasm::new(
            document,
            data_contract.get_id().into(),
            type_name,
            None,
        ))
    }

    #[wasm_bindgen(js_name=fromHex)]
    pub fn from_hex(
        hex: String,
        data_contract: &DataContractWasm,
        type_name: String,
        js_platform_version: JsValue,
    ) -> WasmDppResult<DocumentWasm> {
        use dpp::platform_value::string_encoding::decode;
        DocumentWasm::from_bytes(
            decode(hex.as_str(), Hex)
                .map_err(|err| WasmDppError::serialization(err.to_string()))?,
            data_contract,
            type_name,
            js_platform_version,
        )
    }

    #[wasm_bindgen(js_name=fromBase64)]
    pub fn from_base64(
        base64: String,
        data_contract: &DataContractWasm,
        type_name: String,
        js_platform_version: JsValue,
    ) -> WasmDppResult<DocumentWasm> {
        use dpp::platform_value::string_encoding::decode;
        DocumentWasm::from_bytes(
            decode(base64.as_str(), Base64)
                .map_err(|err| WasmDppError::serialization(err.to_string()))?,
            data_contract,
            type_name,
            js_platform_version,
        )
    }

    #[wasm_bindgen(js_name=generateId)]
    pub fn generate_id(
        js_document_type_name: &str,
        #[wasm_bindgen(unchecked_param_type = "Identifier | Uint8Array | string")]
        js_owner_id: &JsValue,
        #[wasm_bindgen(unchecked_param_type = "Identifier | Uint8Array | string")]
        js_data_contract_id: &JsValue,
        opt_entropy: Option<Vec<u8>>,
    ) -> WasmDppResult<Vec<u8>> {
        let owner_id: Identifier = IdentifierWasm::try_from(js_owner_id)?.into();
        let data_contract_id: Identifier = IdentifierWasm::try_from(js_data_contract_id)?.into();

        let entropy: [u8; 32] = match opt_entropy {
            Some(entropy_vec) => {
                let mut entropy = [0u8; 32];
                let bytes = entropy_vec.as_slice();
                let len = bytes.len().min(32);
                entropy[..len].copy_from_slice(&bytes[..len]);
                entropy
            }
            None => entropy_generator::DefaultEntropyGenerator
                .generate()
                .map_err(|err| WasmDppError::serialization(err.to_string()))?,
        };

        let identifier = crate::utils::generate_document_id_v0(
            &data_contract_id,
            &owner_id,
            js_document_type_name,
            &entropy,
        )?;

        Ok(identifier.to_vec())
    }
}
