use crate::data_contract::DataContractWasm;
use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::{IdentifierLikeJs, IdentifierWasm};
use crate::impl_try_from_js_value;
use crate::impl_wasm_type_info;
use crate::serialization;
use crate::utils::{
    ToSerdeJSONExt, try_from_options_optional, try_from_options_optional_with, try_from_options,
    try_from_options_with, try_to_fixed_bytes, try_to_u64,
};
use crate::version::{PlatformVersionLikeJs, PlatformVersionWasm};
use dpp::document::serialization_traits::{
    DocumentJsonMethodsV0, DocumentPlatformConversionMethodsV0, DocumentPlatformValueMethodsV0,
};
use dpp::document::{Document, DocumentV0, DocumentV0Getters, DocumentV0Setters};
use dpp::identifier::Identifier;
use dpp::platform_value::string_encoding::Encoding::{Base64, Hex};
use dpp::platform_value::string_encoding::encode;
use dpp::platform_value::{Value, ValueMapHelper};
use dpp::prelude::Revision;
use dpp::util::entropy_generator;
use dpp::util::entropy_generator::EntropyGenerator;
use dpp::version::PlatformVersion;
use js_sys::Object;
use serde::Deserialize;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

/// TypeScript interface for Document constructor options
#[wasm_bindgen(typescript_custom_section)]
const DOCUMENT_OPTIONS_TS: &str = r#"
/**
 * Options for creating a new Document.
 */
export interface DocumentOptions {
  /** Document properties/data */
  properties: Record<string, unknown>;
  /** Document type name from the data contract */
  documentTypeName: string;
  /** Data contract ID this document belongs to */
  dataContractId: IdentifierLike;
  /** Owner identity ID */
  ownerId: IdentifierLike;
  /** Document revision (default: 1) */
  revision?: number;
  /** Document ID (auto-generated if not provided) */
  id?: IdentifierLike;
  /** Entropy bytes (32 bytes, auto-generated if not provided) */
  entropy?: Uint8Array;
}

/**
 * Document serialized as a plain object.
 */
export interface DocumentObject {
  $id: Identifier;
  $ownerId: Identifier;
  $revision?: number;
  $createdAt?: number;
  $updatedAt?: number;
  $transferredAt?: number;
  $createdAtBlockHeight?: number;
  $updatedAtBlockHeight?: number;
  $transferredAtBlockHeight?: number;
  $createdAtCoreBlockHeight?: number;
  $updatedAtCoreBlockHeight?: number;
  $transferredAtCoreBlockHeight?: number;
  $dataContractId: Identifier;
  $type: string;
  [key: string]: unknown;
}

/**
 * Document serialized as JSON (with string identifiers).
 */
export interface DocumentJSON {
  $id: string;
  $ownerId: string;
  $revision?: number;
  $createdAt?: number;
  $updatedAt?: number;
  $transferredAt?: number;
  $createdAtBlockHeight?: number;
  $updatedAtBlockHeight?: number;
  $transferredAtBlockHeight?: number;
  $createdAtCoreBlockHeight?: number;
  $updatedAtCoreBlockHeight?: number;
  $transferredAtCoreBlockHeight?: number;
  $dataContractId: string;
  $type: string;
  [key: string]: unknown;
}
"#;

/// DocumentWasm wraps a Document and adds metadata fields that are not part of the core Document.
#[derive(Clone, serde::Serialize, Deserialize)]
#[wasm_bindgen(js_name = "Document")]
pub struct DocumentWasm {
    #[serde(skip_serializing, skip_deserializing, default = "default_document")]
    pub(crate) document: Document,
    #[serde(rename = "$dataContractId")]
    pub(crate) data_contract_id: IdentifierWasm,
    #[serde(rename = "$type")]
    pub(crate) document_type_name: String,
    #[serde(
        rename = "$entropy",
        with = "serialization::bytes_b64::option",
        skip_serializing_if = "Option::is_none",
        default
    )]
    pub(crate) entropy: Option<[u8; 32]>,
}

fn default_document() -> Document {
    Document::V0(DocumentV0::default())
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

    pub fn set_data_contract_id(&mut self, data_contract_id: &IdentifierWasm) {
        self.data_contract_id = *data_contract_id;
    }
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "DocumentOptions")]
    pub type DocumentOptionsJs;

    #[wasm_bindgen(typescript_type = "DocumentObject")]
    pub type DocumentObjectJs;

    #[wasm_bindgen(typescript_type = "DocumentJSON")]
    pub type DocumentJSONJs;

    #[wasm_bindgen(typescript_type = "Record<string, unknown>")]
    pub type DocumentPropertiesJs;
}

#[wasm_bindgen(js_class = Document)]
impl DocumentWasm {
    #[wasm_bindgen(constructor)]
    pub fn constructor(options: DocumentOptionsJs) -> WasmDppResult<DocumentWasm> {
        let options_value: JsValue = options.into();
        let options_obj = Object::from(options_value.clone());

        // Extract required properties
        let document_type_name = try_from_options_with(&options_obj, "documentTypeName", |v| {
            v.as_string()
                .ok_or_else(|| WasmDppError::invalid_argument("'documentTypeName' must be a string"))
        })?;

        let data_contract_id: Identifier =
            try_from_options::<IdentifierWasm>(&options_obj, "dataContractId")?.into();

        let owner_id: Identifier =
            try_from_options::<IdentifierWasm>(&options_obj, "ownerId")?.into();

        let properties = try_from_options_with(&options_obj, "properties", |v| {
            v.with_serde_to_platform_value_map()
        })?;

        // Extract optional properties
        let revision = try_from_options_optional_with(&options_obj, "revision", |v| {
            try_to_u64(v, "revision").map(Revision::from)
        })?
        .unwrap_or(Revision::from(1u64));

        let id: Option<IdentifierWasm> = try_from_options_optional(&options_obj, "id")?;

        let entropy: Option<[u8; 32]> = try_from_options_optional_with(&options_obj, "entropy", |v| {
            try_to_fixed_bytes::<32>(v, "entropy")
        })?;

        let entropy: [u8; 32] = entropy.map_or_else(
            || {
                entropy_generator::DefaultEntropyGenerator
                    .generate()
                    .map_err(|err| WasmDppError::serialization(err.to_string()))
            },
            Ok,
        )?;

        let doc_id: Identifier = id.map_or_else(
            || {
                crate::utils::generate_document_id_v0(
                    &data_contract_id,
                    &owner_id,
                    &document_type_name,
                    &entropy,
                )
            },
            |id| Ok(id.into()),
        )?;

        let document = Document::V0(DocumentV0 {
            id: doc_id,
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
            document_type_name,
            Some(entropy),
        ))
    }

    #[wasm_bindgen(getter = id)]
    pub fn id(&self) -> IdentifierWasm {
        self.document.id().into()
    }

    #[wasm_bindgen(getter = entropy)]
    pub fn entropy(&self) -> Option<Vec<u8>> {
        self.entropy.map(|entropy| entropy.to_vec())
    }

    #[wasm_bindgen(getter = dataContractId)]
    pub fn data_contract_id(&self) -> IdentifierWasm {
        self.data_contract_id
    }

    #[wasm_bindgen(getter = ownerId)]
    pub fn owner_id(&self) -> IdentifierWasm {
        self.document.owner_id().into()
    }

    #[wasm_bindgen(getter = properties)]
    pub fn properties(&self) -> WasmDppResult<DocumentPropertiesJs> {
        let properties_value = Value::Map(
            self.document
                .properties()
                .iter()
                .map(|(k, v)| (Value::Text(k.clone()), v.clone()))
                .collect(),
        );
        let js_value = serialization::platform_value_to_object(&properties_value)?;
        Ok(js_value.into())
    }

    #[wasm_bindgen(getter = revision)]
    pub fn revision(&self) -> Option<u64> {
        self.document.revision()
    }

    #[wasm_bindgen(getter = createdAt)]
    pub fn created_at(&self) -> Option<u64> {
        self.document.created_at()
    }

    #[wasm_bindgen(getter = updatedAt)]
    pub fn updated_at(&self) -> Option<u64> {
        self.document.updated_at()
    }

    #[wasm_bindgen(getter = transferredAt)]
    pub fn transferred_at(&self) -> Option<u64> {
        self.document.transferred_at()
    }

    #[wasm_bindgen(getter = createdAtBlockHeight)]
    pub fn created_at_block_height(&self) -> Option<u64> {
        self.document.created_at_block_height()
    }

    #[wasm_bindgen(getter = updatedAtBlockHeight)]
    pub fn updated_at_block_height(&self) -> Option<u64> {
        self.document.updated_at_block_height()
    }

    #[wasm_bindgen(getter = transferredAtBlockHeight)]
    pub fn transferred_at_block_height(&self) -> Option<u64> {
        self.document.transferred_at_block_height()
    }

    #[wasm_bindgen(getter = createdAtCoreBlockHeight)]
    pub fn created_at_core_block_height(&self) -> Option<u32> {
        self.document.created_at_core_block_height()
    }

    #[wasm_bindgen(getter = updatedAtCoreBlockHeight)]
    pub fn updated_at_core_block_height(&self) -> Option<u32> {
        self.document.updated_at_core_block_height()
    }

    #[wasm_bindgen(getter = transferredAtCoreBlockHeight)]
    pub fn transferred_at_core_block_height(&self) -> Option<u32> {
        self.document.transferred_at_core_block_height()
    }

    #[wasm_bindgen(getter = documentTypeName)]
    pub fn document_type_name(&self) -> String {
        self.document_type_name.clone()
    }

    #[wasm_bindgen(setter=id)]
    pub fn set_id(&mut self, id: IdentifierLikeJs) -> WasmDppResult<()> {
        self.document.set_id(id.try_into()?);
        Ok(())
    }

    #[wasm_bindgen(setter=entropy)]
    pub fn set_entropy(&mut self, entropy: Option<Vec<u8>>) -> WasmDppResult<()> {
        match entropy {
            None => {
                self.entropy = None;
            }
            Some(bytes) => {
                if bytes.len() != 32 {
                    return Err(WasmDppError::invalid_argument(format!(
                        "Entropy must be exactly 32 bytes, got {}",
                        bytes.len()
                    )));
                }
                let mut entropy_bytes = [0u8; 32];
                entropy_bytes.copy_from_slice(&bytes);
                self.entropy = Some(entropy_bytes);
            }
        }
        Ok(())
    }

    #[wasm_bindgen(setter=dataContractId)]
    pub fn set_data_contract_id_js(
        &mut self,
        data_contract_id: IdentifierLikeJs,
    ) -> WasmDppResult<()> {
        self.data_contract_id = data_contract_id.try_into()?;
        Ok(())
    }

    #[wasm_bindgen(setter=ownerId)]
    pub fn set_owner_id(&mut self, id: IdentifierLikeJs) -> WasmDppResult<()> {
        self.document.set_owner_id(id.try_into()?);
        Ok(())
    }

    #[wasm_bindgen(setter=properties)]
    pub fn set_properties(
        &mut self,
        #[wasm_bindgen(unchecked_param_type = "Record<string, unknown>")] properties: JsValue,
    ) -> WasmDppResult<()> {
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
    #[wasm_bindgen(js_name = "toObject")]
    pub fn to_object(&self) -> WasmDppResult<DocumentObjectJs> {
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
        let js_value = serialization::platform_value_to_object(&Value::Map(
            map.into_iter().map(|(k, v)| (Value::Text(k), v)).collect(),
        ))?;
        Ok(js_value.into())
    }

    /// Create a Document from a JS object.
    #[wasm_bindgen(js_name = "fromObject")]
    pub fn from_object(value: DocumentObjectJs) -> WasmDppResult<DocumentWasm> {
        let platform_value = serialization::js_value_to_platform_value(&value.into())?;

        let Value::Map(mut map) = platform_value else {
            return Err(WasmDppError::invalid_argument("Expected an object"));
        };

        // Extract metadata fields using ValueMapHelper trait methods
        let data_contract_id = map
            .remove_optional_key("$dataContractId")
            .ok_or_else(|| WasmDppError::invalid_argument("Missing $dataContractId"))?
            .into_identifier()
            .map_err(|e| WasmDppError::invalid_argument(e.to_string()))?;

        let document_type_name = map
            .remove_optional_key("$type")
            .ok_or_else(|| WasmDppError::invalid_argument("Missing $type"))?
            .into_text()
            .map_err(|e| WasmDppError::invalid_argument(e.to_string()))?;

        let entropy = map.remove_optional_key("$entropy").and_then(|v| {
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
        let document = Document::from_platform_value(Value::Map(map), PlatformVersion::latest())?;

        Ok(DocumentWasm::new(
            document,
            data_contract_id,
            document_type_name,
            entropy,
        ))
    }

    /// Convert to a JSON-compatible JS object with binary fields as strings.
    #[wasm_bindgen(js_name = "toJSON")]
    pub fn to_json(&self) -> WasmDppResult<DocumentJSONJs> {
        // Get document fields as JSON
        let mut json_value = self.document.to_json(PlatformVersion::latest())?;

        // Serialize wrapper fields using serde and merge into document JSON
        let wrapper_json =
            serde_json::to_value(self).map_err(|e| WasmDppError::serialization(e.to_string()))?;

        let obj = json_value.as_object_mut().ok_or_else(|| {
            WasmDppError::serialization("Expected JSON object from Document::to_json")
        })?;

        if let serde_json::Value::Object(wrapper_obj) = wrapper_json {
            for (key, value) in wrapper_obj {
                obj.insert(key, value);
            }
        }

        let js_value = serialization::json_to_js_value(&json_value)?;
        Ok(js_value.into())
    }

    /// Create a Document from a JSON object.
    /// JSON format has identifiers as base58 strings.
    #[wasm_bindgen(js_name = "fromJSON")]
    pub fn from_json(value: DocumentJSONJs) -> WasmDppResult<DocumentWasm> {
        let mut json_value = serialization::js_value_to_json(&value.into())?;

        // Deserialize wrapper fields using serde
        let mut wrapper: DocumentWasm = serde_json::from_value(json_value.clone())
            .map_err(|e| WasmDppError::serialization(e.to_string()))?;

        // Remove wrapper fields from JSON before passing to Document::from_json_value
        if let serde_json::Value::Object(ref mut obj) = json_value {
            obj.remove("$dataContractId");
            obj.remove("$type");
            obj.remove("$entropy");
        }

        // Create Document from remaining fields
        wrapper.document =
            Document::from_json_value::<String, _>(json_value, PlatformVersion::latest())?;

        Ok(wrapper)
    }

    #[wasm_bindgen(js_name = "toBytes")]
    pub fn to_bytes(
        &self,
        data_contract: &DataContractWasm,
        platform_version: PlatformVersionLikeJs,
    ) -> WasmDppResult<Vec<u8>> {
        self.to_bytes_internal(data_contract, platform_version)
    }

    #[wasm_bindgen(js_name = "toHex")]
    pub fn to_hex(
        &self,
        data_contract: &DataContractWasm,
        platform_version: PlatformVersionLikeJs,
    ) -> WasmDppResult<String> {
        Ok(encode(
            self.to_bytes_internal(data_contract, platform_version)?
                .as_slice(),
            Hex,
        ))
    }

    #[wasm_bindgen(js_name = "toBase64")]
    pub fn to_base64(
        &self,
        data_contract: &DataContractWasm,
        platform_version: PlatformVersionLikeJs,
    ) -> WasmDppResult<String> {
        Ok(encode(
            self.to_bytes_internal(data_contract, platform_version)?
                .as_slice(),
            Base64,
        ))
    }

    #[wasm_bindgen(js_name = "fromBytes")]
    pub fn from_bytes(
        bytes: Vec<u8>,
        data_contract: &DataContractWasm,
        type_name: String,
        platform_version: PlatformVersionLikeJs,
    ) -> WasmDppResult<DocumentWasm> {
        Self::from_bytes_internal(bytes, data_contract, type_name, platform_version.into())
    }

    #[wasm_bindgen(js_name = "fromHex")]
    pub fn from_hex(
        hex: String,
        data_contract: &DataContractWasm,
        type_name: String,
        platform_version: PlatformVersionLikeJs,
    ) -> WasmDppResult<DocumentWasm> {
        use dpp::platform_value::string_encoding::decode;
        Self::from_bytes_internal(
            decode(hex.as_str(), Hex)
                .map_err(|err| WasmDppError::serialization(err.to_string()))?,
            data_contract,
            type_name,
            platform_version.into(),
        )
    }

    #[wasm_bindgen(js_name = "fromBase64")]
    pub fn from_base64(
        base64: String,
        data_contract: &DataContractWasm,
        type_name: String,
        platform_version: PlatformVersionLikeJs,
    ) -> WasmDppResult<DocumentWasm> {
        use dpp::platform_value::string_encoding::decode;
        Self::from_bytes_internal(
            decode(base64.as_str(), Base64)
                .map_err(|err| WasmDppError::serialization(err.to_string()))?,
            data_contract,
            type_name,
            platform_version.into(),
        )
    }

    #[wasm_bindgen(js_name = "generateId")]
    pub fn generate_id(
        document_type_name: &str,
        owner_id: IdentifierLikeJs,
        data_contract_id: IdentifierLikeJs,
        entropy: Option<Vec<u8>>,
    ) -> WasmDppResult<Vec<u8>> {
        let owner_id: Identifier = owner_id.try_into()?;
        let data_contract_id: Identifier = data_contract_id.try_into()?;

        let entropy_bytes: [u8; 32] = match entropy {
            Some(entropy_vec) => {
                if entropy_vec.len() != 32 {
                    return Err(WasmDppError::invalid_argument(format!(
                        "Entropy must be exactly 32 bytes, got {}",
                        entropy_vec.len()
                    )));
                }
                let mut arr = [0u8; 32];
                arr.copy_from_slice(&entropy_vec);
                arr
            }
            None => entropy_generator::DefaultEntropyGenerator
                .generate()
                .map_err(|err| WasmDppError::serialization(err.to_string()))?,
        };

        let identifier = crate::utils::generate_document_id_v0(
            &data_contract_id,
            &owner_id,
            document_type_name,
            &entropy_bytes,
        )?;

        Ok(identifier.to_vec())
    }
}

impl DocumentWasm {
    fn to_bytes_internal(
        &self,
        data_contract: &DataContractWasm,
        platform_version: PlatformVersionLikeJs,
    ) -> WasmDppResult<Vec<u8>> {
        let platform_version: PlatformVersionWasm = platform_version.try_into()?;

        let document_type_ref = data_contract
            .get_document_type_ref_by_name(self.document_type_name())
            .map_err(|err| WasmDppError::invalid_argument(err.to_string()))?;

        self.document
            .serialize(
                document_type_ref,
                &data_contract.clone().into(),
                &platform_version.into(),
            )
            .map_err(Into::into)
    }

    fn from_bytes_internal(
        bytes: Vec<u8>,
        data_contract: &DataContractWasm,
        type_name: String,
        platform_version: JsValue,
    ) -> WasmDppResult<DocumentWasm> {
        let platform_version = if platform_version.is_undefined() {
            PlatformVersionWasm::default()
        } else {
            PlatformVersionWasm::try_from(platform_version)?
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
}

impl_try_from_js_value!(DocumentWasm, "Document");
impl_wasm_type_info!(DocumentWasm, Document);
