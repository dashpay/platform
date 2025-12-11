use crate::data_contract::DataContractWasm;
use crate::data_contract::document::DocumentWasm;
use crate::enums::platform::PlatformVersionWasm;
use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::IdentifierWasm;
use crate::serde_format;
use crate::utils::ToSerdeJSONExt;
use dpp::data_contract::JsonValue;
use dpp::document::Document;
use dpp::document::serialization_traits::{
    DocumentJsonMethodsV0, DocumentPlatformConversionMethodsV0, DocumentPlatformValueMethodsV0,
};
use dpp::identifier::Identifier;
use dpp::platform_value::Value;
use dpp::platform_value::converter::serde_json::BTreeValueJsonConverter;
use dpp::platform_value::string_encoding::Encoding::{Base64, Hex};
use dpp::platform_value::string_encoding::{decode, encode};
use dpp::prelude::Revision;
use dpp::util::entropy_generator;
use dpp::util::entropy_generator::EntropyGenerator;
use std::collections::BTreeMap;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

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
    pub fn new(
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

        let document = js_raw_document.with_serde_to_platform_value_map()?;

        let revision = Revision::from(revision);

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

        Ok(DocumentWasm {
            owner_id: owner_id.into(),
            entropy: Some(entropy),
            id: document_id.into(),
            document_type_name: js_document_type_name.to_string(),
            data_contract_id: data_contract_id.into(),
            properties: document,
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
        })
    }

    #[wasm_bindgen(getter=id)]
    pub fn get_id(&self) -> IdentifierWasm {
        self.id
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
        self.owner_id
    }

    #[wasm_bindgen(getter=properties)]
    pub fn get_properties(&self) -> WasmDppResult<JsValue> {
        let json_value: JsonValue = self
            .properties
            .clone()
            .to_json_value()
            .map_err(|err| WasmDppError::serialization(err.to_string()))?;

        serde_format::to_json(&json_value)
    }

    #[wasm_bindgen(getter=revision)]
    pub fn get_revision(&self) -> Option<u64> {
        self.revision
    }

    #[wasm_bindgen(getter=createdAt)]
    pub fn get_created_at(&self) -> Option<u64> {
        self.created_at
    }

    #[wasm_bindgen(getter=updatedAt)]
    pub fn get_updated_at(&self) -> Option<u64> {
        self.updated_at
    }

    #[wasm_bindgen(getter=transferredAt)]
    pub fn get_transferred_at(&self) -> Option<u64> {
        self.transferred_at
    }

    #[wasm_bindgen(getter=createdAtBlockHeight)]
    pub fn get_created_at_block_height(&self) -> Option<u64> {
        self.created_at_block_height
    }

    #[wasm_bindgen(getter=updatedAtBlockHeight)]
    pub fn get_updated_at_block_height(&self) -> Option<u64> {
        self.updated_at_block_height
    }

    #[wasm_bindgen(getter=transferredAtBlockHeight)]
    pub fn get_transferred_at_block_height(&self) -> Option<u64> {
        self.transferred_at_block_height
    }

    #[wasm_bindgen(getter=createdAtCoreBlockHeight)]
    pub fn get_created_at_core_block_height(&self) -> Option<u32> {
        self.created_at_core_block_height
    }

    #[wasm_bindgen(getter=updatedAtCoreBlockHeight)]
    pub fn get_updated_at_core_block_height(&self) -> Option<u32> {
        self.updated_at_core_block_height
    }

    #[wasm_bindgen(getter=transferredAtCoreBlockHeight)]
    pub fn get_transferred_at_core_block_height(&self) -> Option<u32> {
        self.transferred_at_core_block_height
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
        self.id = IdentifierWasm::try_from(id)?;
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
        self.owner_id = IdentifierWasm::try_from(id)?;
        Ok(())
    }

    #[wasm_bindgen(setter=properties)]
    pub fn set_properties(&mut self, properties: JsValue) -> WasmDppResult<()> {
        self.properties = properties.with_serde_to_platform_value_map()?;
        Ok(())
    }

    #[wasm_bindgen(setter=revision)]
    pub fn set_revision(&mut self, revision: Option<u64>) {
        self.revision = revision
    }

    #[wasm_bindgen(setter=createdAt)]
    pub fn set_created_at(&mut self, created_at: Option<u64>) {
        self.created_at = created_at
    }

    #[wasm_bindgen(setter=updatedAt)]
    pub fn set_get_updated_at(&mut self, updated_at: Option<u64>) {
        self.updated_at = updated_at
    }

    #[wasm_bindgen(setter=transferredAt)]
    pub fn set_transferred_at(&mut self, transferred_at: Option<u64>) {
        self.transferred_at = transferred_at
    }

    #[wasm_bindgen(setter=createdAtBlockHeight)]
    pub fn set_created_at_block_height(&mut self, created_at_block_height: Option<u64>) {
        self.created_at_block_height = created_at_block_height
    }

    #[wasm_bindgen(setter=updatedAtBlockHeight)]
    pub fn set_updated_at_block_height(&mut self, updated_at_block_height: Option<u64>) {
        self.updated_at_block_height = updated_at_block_height
    }

    #[wasm_bindgen(setter=transferredAtBlockHeight)]
    pub fn set_transferred_at_block_height(&mut self, transferred_at_block_height: Option<u64>) {
        self.transferred_at_block_height = transferred_at_block_height
    }

    #[wasm_bindgen(setter=createdAtCoreBlockHeight)]
    pub fn set_created_at_core_block_height(&mut self, created_at_core_block_height: Option<u32>) {
        self.created_at_core_block_height = created_at_core_block_height
    }

    #[wasm_bindgen(setter=updatedAtCoreBlockHeight)]
    pub fn set_updated_at_core_block_height(&mut self, updated_at_core_block_height: Option<u32>) {
        self.updated_at_core_block_height = updated_at_core_block_height
    }

    #[wasm_bindgen(setter=transferredAtCoreBlockHeight)]
    pub fn set_transferred_at_core_block_height(
        &mut self,
        transferred_at_core_block_height: Option<u32>,
    ) {
        self.transferred_at_core_block_height = transferred_at_core_block_height
    }

    #[wasm_bindgen(setter=documentTypeName)]
    pub fn set_document_type_name(&mut self, document_type_name: &str) {
        self.document_type_name = document_type_name.to_string();
    }

    /// Convert to a JS object with binary fields as Uint8Array.
    #[wasm_bindgen(js_name = toObject)]
    pub fn to_object(&self, js_platform_version: JsValue) -> WasmDppResult<JsValue> {
        let _platform_version = match js_platform_version.is_undefined() {
            true => PlatformVersionWasm::default(),
            false => PlatformVersionWasm::try_from(js_platform_version)?,
        };

        let rs_document: Document = Document::from(self.clone());
        let value = rs_document.to_object()?;

        // Add document metadata that's stored in DocumentWasm but not in Document
        let mut map = value
            .into_btree_string_map()
            .map_err(|e| WasmDppError::serialization(e.to_string()))?;

        let contract_id: Identifier = self.data_contract_id.into();
        map.insert("$dataContractId".to_string(), Value::Identifier(contract_id.into_buffer()));
        map.insert(
            "$type".to_string(),
            Value::Text(self.document_type_name.clone()),
        );
        if let Some(entropy) = self.entropy {
            map.insert("$entropy".to_string(), Value::Bytes(entropy.to_vec()));
        }

        let final_value = Value::Map(
            map.into_iter()
                .map(|(k, v)| (Value::Text(k), v))
                .collect(),
        );
        serde_format::platform_value_to_object(&final_value)
    }

    /// Create a Document from a JS object.
    #[wasm_bindgen(js_name = fromObject)]
    pub fn from_object(
        js_value: JsValue,
        js_platform_version: JsValue,
    ) -> WasmDppResult<DocumentWasm> {
        let platform_version = match js_platform_version.is_undefined() {
            true => PlatformVersionWasm::default(),
            false => PlatformVersionWasm::try_from(js_platform_version)?,
        };

        let value: Value = serde_format::platform_value_from_object(js_value)?;
        let map = value
            .clone()
            .into_btree_string_map()
            .map_err(|e| WasmDppError::serialization(e.to_string()))?;

        // Extract document metadata
        let data_contract_id = map
            .get("$dataContractId")
            .and_then(|v| v.to_identifier().ok())
            .unwrap_or_default();
        let document_type_name = map
            .get("$type")
            .and_then(|v| v.as_text())
            .unwrap_or("")
            .to_string();
        let entropy = map.get("$entropy").and_then(|v| {
            v.as_bytes().and_then(|bytes| {
                let mut arr = [0u8; 32];
                if bytes.len() == 32 {
                    arr.copy_from_slice(bytes);
                    Some(arr)
                } else {
                    None
                }
            })
        });

        let rs_document =
            Document::from_platform_value(value, &platform_version.into())?;

        let mut doc = DocumentWasm::from(rs_document);
        doc.data_contract_id = data_contract_id.into();
        doc.document_type_name = document_type_name;
        doc.entropy = entropy;

        Ok(doc)
    }

    /// Convert to a JSON-compatible JS object with binary fields as strings.
    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self, js_platform_version: JsValue) -> WasmDppResult<JsValue> {
        let platform_version = match js_platform_version.is_undefined() {
            true => PlatformVersionWasm::default(),
            false => PlatformVersionWasm::try_from(js_platform_version)?,
        };

        let rs_document: Document = Document::from(self.clone());
        let mut json = rs_document.to_json(&platform_version.into())?;

        // Add document metadata that's stored in DocumentWasm but not in Document
        if let Some(obj) = json.as_object_mut() {
            obj.insert(
                "$dataContractId".to_string(),
                serde_json::Value::String(self.data_contract_id.to_base58()),
            );
            obj.insert(
                "$type".to_string(),
                serde_json::Value::String(self.document_type_name.clone()),
            );
            if let Some(entropy) = self.entropy {
                obj.insert(
                    "$entropy".to_string(),
                    serde_json::Value::String(encode(&entropy, Base64)),
                );
            }
        }

        serde_format::to_json(&json)
    }

    /// Create a Document from a JSON object.
    #[wasm_bindgen(js_name = fromJSON)]
    pub fn from_json(
        js_value: JsValue,
        js_platform_version: JsValue,
    ) -> WasmDppResult<DocumentWasm> {
        let platform_version = match js_platform_version.is_undefined() {
            true => PlatformVersionWasm::default(),
            false => PlatformVersionWasm::try_from(js_platform_version)?,
        };

        let mut json_value: JsonValue = js_value.with_serde_to_json_value()?;

        // Extract document metadata from JSON
        let data_contract_id: IdentifierWasm = json_value
            .get("$dataContractId")
            .and_then(|v| v.as_str())
            .and_then(|s| IdentifierWasm::try_from(s).ok())
            .unwrap_or_else(|| Identifier::default().into());
        let document_type_name = json_value
            .get("$type")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let entropy = json_value.get("$entropy").and_then(|v| {
            v.as_str().and_then(|s| {
                decode(s, Base64).ok().and_then(|bytes| {
                    if bytes.len() == 32 {
                        let mut arr = [0u8; 32];
                        arr.copy_from_slice(&bytes);
                        Some(arr)
                    } else {
                        None
                    }
                })
            })
        });

        // Convert identifier strings to byte arrays for from_platform_value
        if let Some(obj) = json_value.as_object_mut() {
            // Convert $id from Base58 string to byte array
            if let Some(id_val) = obj.get("$id").and_then(|v| v.as_str()) {
                if let Ok(id) = IdentifierWasm::try_from(id_val) {
                    obj.insert(
                        "$id".to_string(),
                        serde_json::Value::Array(
                            id.to_bytes().into_iter().map(|b| serde_json::Value::Number(b.into())).collect()
                        ),
                    );
                }
            }
            // Convert $ownerId from Base58 string to byte array
            if let Some(owner_val) = obj.get("$ownerId").and_then(|v| v.as_str()) {
                if let Ok(id) = IdentifierWasm::try_from(owner_val) {
                    obj.insert(
                        "$ownerId".to_string(),
                        serde_json::Value::Array(
                            id.to_bytes().into_iter().map(|b| serde_json::Value::Number(b.into())).collect()
                        ),
                    );
                }
            }
            // Convert $creatorId from Base58 string to byte array if present
            if let Some(creator_val) = obj.get("$creatorId").and_then(|v| v.as_str()) {
                if let Ok(id) = IdentifierWasm::try_from(creator_val) {
                    obj.insert(
                        "$creatorId".to_string(),
                        serde_json::Value::Array(
                            id.to_bytes().into_iter().map(|b| serde_json::Value::Number(b.into())).collect()
                        ),
                    );
                }
            }
        }

        // Convert JSON to platform value and use from_platform_value
        let platform_value: Value = json_value.into();
        let rs_document = Document::from_platform_value(platform_value, &platform_version.into())?;

        let mut doc = DocumentWasm::from(rs_document);
        doc.data_contract_id = data_contract_id;
        doc.document_type_name = document_type_name;
        doc.entropy = entropy;

        Ok(doc)
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

        let rs_document: Document = Document::from(self.clone());

        let document_type_ref = data_contract
            .get_document_type_ref_by_name(self.get_document_type_name())
            .map_err(|err| WasmDppError::invalid_argument(err.to_string()))?;

        DocumentPlatformConversionMethodsV0::serialize(
            &rs_document,
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

        let document_type_ref = match data_contract.get_document_type_ref_by_name(type_name.clone())
        {
            Ok(type_ref) => Ok(type_ref),
            Err(err) => Err(WasmDppError::invalid_argument(err.to_string())),
        }?;

        let rs_document = Document::from_bytes(
            bytes.as_slice(),
            document_type_ref,
            &platform_version.into(),
        )?;

        let mut js_document = DocumentWasm::from(rs_document);

        js_document.set_document_type_name(type_name.clone().as_str());
        js_document.set_data_contract_id(&data_contract.get_id());

        Ok(js_document)
    }

    #[wasm_bindgen(js_name=fromHex)]
    pub fn from_hex(
        hex: String,
        data_contract: &DataContractWasm,
        type_name: String,
        js_platform_version: JsValue,
    ) -> WasmDppResult<DocumentWasm> {
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

impl DocumentWasm {
    pub fn rs_get_owner_id(&self) -> Identifier {
        self.owner_id.into()
    }

    pub fn rs_get_id(&self) -> Identifier {
        self.id.into()
    }

    pub fn rs_get_data_contract_id(&self) -> Identifier {
        self.data_contract_id.into()
    }

    pub fn rs_get_entropy(&self) -> Option<[u8; 32]> {
        self.entropy
    }

    pub fn rs_get_properties(&self) -> BTreeMap<String, Value> {
        self.clone().properties
    }

    fn set_data_contract_id(&mut self, data_contract_id: &IdentifierWasm) {
        self.data_contract_id = *data_contract_id;
    }
}
