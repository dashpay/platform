use crate::data_contract::DataContractWasm;
use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::{IdentifierLikeJs, IdentifierWasm};
use crate::impl_try_from_js_value;
use crate::impl_try_from_options;
use crate::impl_wasm_type_info;
use crate::serialization;
use crate::utils::{
    ToSerdeJSONExt, try_from_options, try_from_options_optional, try_from_options_optional_with,
    try_from_options_with, try_to_u64, try_vec_to_fixed_bytes,
};
use crate::version::{PlatformVersionLikeJs, PlatformVersionWasm};
use dpp::document::serialization_traits::{
    DocumentPlatformConversionMethodsV0, DocumentPlatformValueMethodsV0,
};
// `DocumentPlatformValueMethodsV0` is brought in for `to_map_value`
// (the only method on it the wasm wrapper still uses, after Phase D
// step 8 slice B). `DocumentPlatformConversionMethodsV0` is the binary
// serialization trait. Canonical `JsonConvertible` / `ValueConvertible`
// are imported inline at the call sites.
use dpp::document::{Document, DocumentV0, DocumentV0Getters, DocumentV0Setters};
use dpp::identifier::Identifier;
use dpp::platform_value::string_encoding::Encoding::{Base58, Base64, Hex};
use dpp::platform_value::string_encoding::encode;
use dpp::platform_value::{Value, ValueMapHelper};
use dpp::prelude::IdentityNonce;
use dpp::util::entropy_generator;
use dpp::util::entropy_generator::EntropyGenerator;
use dpp::version::PlatformVersion;
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
  /** Document revision (default: 1n) */
  revision?: bigint;
  /**
   * Document ID. Derived when not provided (see `identityContractNonce`).
   * Together with `identityContractNonce` it must equal the derived id, or
   * the constructor throws: the nonce fixes the id, and a different explicit
   * one could only be referenced, never created. Whatever is given here is
   * replaced by `new DocumentCreateTransition(...)`, which can only carry the
   * id consensus recomputes.
   */
  id?: IdentifierLike;
  /** Entropy bytes (32 bytes, auto-generated if not provided) */
  entropy?: Uint8Array;
  /**
   * Identity contract nonce the create transition of this document is going
   * to use. From protocol version 14 the id of a new document commits to
   * that nonce, so without it the derived `id` is a placeholder that
   * `new DocumentCreateTransition(...)` replaces (and mirrors back onto this
   * document). Pass the nonce here, or call `setIdForCreation`, to have the
   * final id from the start.
   */
  identityContractNonce?: bigint;
  /** Platform version the id is derived for (default: latest) */
  platformVersion?: PlatformVersionLike;
}

/**
 * Document serialized as a plain object.
 * Note: u64 fields are serialized as BigInt when using toObject().
 */
export interface DocumentObject {
  $id: Identifier;
  $ownerId: Identifier;
  $revision?: bigint;
  $createdAt?: bigint;
  $updatedAt?: bigint;
  $transferredAt?: bigint;
  $createdAtBlockHeight?: bigint;
  $updatedAtBlockHeight?: bigint;
  $transferredAtBlockHeight?: bigint;
  $createdAtCoreBlockHeight?: number;
  $updatedAtCoreBlockHeight?: number;
  $transferredAtCoreBlockHeight?: number;
  $dataContractId: Identifier;
  $type: string;
  [key: string]: unknown;
}

/**
 * Document serialized as JSON (with string identifiers).
 * Note: u64 fields are stringified since JSON doesn't support BigInt.
 */
export interface DocumentJSON {
  $id: string;
  $ownerId: string;
  $revision?: string;
  $createdAt?: string;
  $updatedAt?: string;
  $transferredAt?: string;
  $createdAtBlockHeight?: string;
  $updatedAtBlockHeight?: string;
  $transferredAtBlockHeight?: string;
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
        with = "dpp::serialization::serde_bytes::option",
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

    /// Gives the document the id its create transition will carry: what
    /// `DocumentCreateTransitionV0::from_document` does in dpp, for a
    /// document that carries its entropy and knows its contract and type
    /// itself.
    ///
    /// Errors when the document has no entropy, which is the case for one
    /// deserialized from Platform: such a document already exists and can
    /// not be created.
    pub fn set_id_for_creation(
        &mut self,
        identity_contract_nonce: IdentityNonce,
        platform_version: &PlatformVersion,
    ) -> WasmDppResult<()> {
        let entropy = self.entropy.ok_or_else(|| {
            WasmDppError::invalid_argument(
                "document has no entropy: only a document built with `new Document(...)` can be \
                 created",
            )
        })?;

        let id = Document::generate_document_id(
            &self.data_contract_id.into(),
            &self.document.owner_id(),
            &self.document_type_name,
            &entropy,
            identity_contract_nonce,
            platform_version,
        )?;
        self.document.set_id(id);
        Ok(())
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

/// Serde struct for DocumentOptions (primitives only)
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DocumentOptionsInput {
    document_type_name: String,
    #[serde(default)]
    revision: Option<u64>,
    #[serde(default)]
    entropy: Option<[u8; 32]>,
}

#[wasm_bindgen(js_class = Document)]
impl DocumentWasm {
    #[wasm_bindgen(constructor)]
    pub fn constructor(options: DocumentOptionsJs) -> WasmDppResult<DocumentWasm> {
        // Extract complex types first (borrows &options)
        let data_contract_id: IdentifierWasm = try_from_options(&options, "dataContractId")?;
        let data_contract_id: Identifier = data_contract_id.into();

        let owner_id: IdentifierWasm = try_from_options(&options, "ownerId")?;
        let owner_id: Identifier = owner_id.into();

        let id: Option<IdentifierWasm> = try_from_options_optional(&options, "id")?;

        let identity_contract_nonce: Option<IdentityNonce> =
            try_from_options_optional_with(&options, "identityContractNonce", |v| {
                try_to_u64(v, "identityContractNonce")
            })?;

        let platform_version: PlatformVersion =
            try_from_options_optional::<PlatformVersionWasm>(&options, "platformVersion")?
                .unwrap_or_default()
                .into();

        let properties = try_from_options_with(&options, "properties", |v| {
            v.with_serde_to_platform_value_map()
        })?;

        // Deserialize primitive fields via serde last (consumes options)
        let input: DocumentOptionsInput = serde_wasm_bindgen::from_value(options.into())
            .map_err(|e| WasmDppError::invalid_argument(e.to_string()))?;

        let document_type_name = input.document_type_name;
        let revision = input.revision.unwrap_or(1);

        let entropy: [u8; 32] = input.entropy.map_or_else(
            || {
                entropy_generator::DefaultEntropyGenerator
                    .generate()
                    .map_err(|err| WasmDppError::serialization(err.to_string()))
            },
            Ok,
        )?;

        let doc_id: Identifier = match (id, identity_contract_nonce) {
            // The nonce fixes the id: it is the one the create transition
            // will carry. An explicit id may only restate it; one that
            // differs would let the caller reference (from another document
            // in the batch, say) an id no transition ever creates.
            (id, Some(identity_contract_nonce)) => {
                let derived = Document::generate_document_id(
                    &data_contract_id,
                    &owner_id,
                    &document_type_name,
                    &entropy,
                    identity_contract_nonce,
                    &platform_version,
                )?;

                if let Some(id) = id {
                    let id: Identifier = id.into();
                    if id != derived {
                        return Err(WasmDppError::invalid_argument(format!(
                            "id {} does not match the id {} derived from the document's entropy \
                             and identityContractNonce {}: pass one or the other",
                            id.to_string(Base58),
                            derived.to_string(Base58),
                            identity_contract_nonce,
                        )));
                    }
                }

                derived
            }
            (Some(id), None) => id.into(),
            // Without the nonce of the create transition the id can only be
            // the entropy-only one, which from protocol version 14 is a
            // placeholder: `DocumentCreateTransition` replaces it.
            (None, None) => Document::generate_document_id_v0(
                &data_contract_id,
                &owner_id,
                &document_type_name,
                &entropy,
            ),
        };

        let document = Document::V0(DocumentV0 {
            contract_version: None,
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
        // Identifier-typed properties surface as base58 strings — the
        // form where-clauses accept back, so a proven join value can be
        // used as a pagination cursor directly. Other binary properties
        // stay Uint8Array.
        let js_value =
            serialization::platform_value_to_object_with_base58_identifiers(&properties_value)?;
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

    /// Gives the document the id its create transition will carry, derived
    /// from its entropy and the identity contract nonce that transition is
    /// going to use.
    ///
    /// `new DocumentCreateTransition(...)` does this on its own; call it
    /// yourself to know the id before the transition exists (a document that
    /// another document in the same batch references). The transition must
    /// then be built with the same nonce.
    ///
    /// Throws when the document carries no entropy (one read back from
    /// Platform), since only a new document can be created.
    #[wasm_bindgen(js_name = "setIdForCreation")]
    pub fn set_id_for_creation_js(
        &mut self,
        #[wasm_bindgen(js_name = "identityContractNonce")] identity_contract_nonce: u64,
        #[wasm_bindgen(js_name = "platformVersion")] platform_version: Option<
            PlatformVersionLikeJs,
        >,
    ) -> WasmDppResult<()> {
        let platform_version: PlatformVersion = platform_version
            .map(PlatformVersionWasm::try_from)
            .transpose()?
            .unwrap_or_default()
            .into();

        self.set_id_for_creation(identity_contract_nonce, &platform_version)
    }

    #[wasm_bindgen(setter=entropy)]
    pub fn set_entropy(&mut self, entropy: Option<Vec<u8>>) -> WasmDppResult<()> {
        match entropy {
            None => {
                self.entropy = None;
            }
            Some(bytes) => {
                self.entropy = Some(try_vec_to_fixed_bytes(bytes, "entropy")?);
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
    pub fn set_created_at(
        &mut self,
        #[wasm_bindgen(js_name = "createdAt")] created_at: Option<u64>,
    ) {
        self.document.set_created_at(created_at);
    }

    #[wasm_bindgen(setter=updatedAt)]
    pub fn set_updated_at(
        &mut self,
        #[wasm_bindgen(js_name = "updatedAt")] updated_at: Option<u64>,
    ) {
        self.document.set_updated_at(updated_at);
    }

    #[wasm_bindgen(setter=transferredAt)]
    pub fn set_transferred_at(
        &mut self,
        #[wasm_bindgen(js_name = "transferredAt")] transferred_at: Option<u64>,
    ) {
        self.document.set_transferred_at(transferred_at);
    }

    #[wasm_bindgen(setter=createdAtBlockHeight)]
    pub fn set_created_at_block_height(
        &mut self,
        #[wasm_bindgen(js_name = "createdAtBlockHeight")] created_at_block_height: Option<u64>,
    ) {
        self.document
            .set_created_at_block_height(created_at_block_height);
    }

    #[wasm_bindgen(setter=updatedAtBlockHeight)]
    pub fn set_updated_at_block_height(
        &mut self,
        #[wasm_bindgen(js_name = "updatedAtBlockHeight")] updated_at_block_height: Option<u64>,
    ) {
        self.document
            .set_updated_at_block_height(updated_at_block_height);
    }

    #[wasm_bindgen(setter=transferredAtBlockHeight)]
    pub fn set_transferred_at_block_height(
        &mut self,
        #[wasm_bindgen(js_name = "transferredAtBlockHeight")] transferred_at_block_height: Option<
            u64,
        >,
    ) {
        self.document
            .set_transferred_at_block_height(transferred_at_block_height);
    }

    #[wasm_bindgen(setter=createdAtCoreBlockHeight)]
    pub fn set_created_at_core_block_height(
        &mut self,
        #[wasm_bindgen(js_name = "createdAtCoreBlockHeight")] created_at_core_block_height: Option<
            u32,
        >,
    ) {
        self.document
            .set_created_at_core_block_height(created_at_core_block_height);
    }

    #[wasm_bindgen(setter=updatedAtCoreBlockHeight)]
    pub fn set_updated_at_core_block_height(
        &mut self,
        #[wasm_bindgen(js_name = "updatedAtCoreBlockHeight")] updated_at_core_block_height: Option<
            u32,
        >,
    ) {
        self.document
            .set_updated_at_core_block_height(updated_at_core_block_height);
    }

    #[wasm_bindgen(setter=transferredAtCoreBlockHeight)]
    pub fn set_transferred_at_core_block_height(
        &mut self,
        #[wasm_bindgen(js_name = "transferredAtCoreBlockHeight")] transferred_at_core_block_height: Option<u32>,
    ) {
        self.document
            .set_transferred_at_core_block_height(transferred_at_core_block_height);
    }

    #[wasm_bindgen(setter=documentTypeName)]
    pub fn set_document_type_name(
        &mut self,
        #[wasm_bindgen(js_name = "documentTypeName")] document_type_name: &str,
    ) {
        self.document_type_name = document_type_name.to_string();
    }

    /// Convert to a JS object with binary fields as Uint8Array.
    ///
    /// Wire shape (Phase D step 8 slice B):
    /// - `$formatVersion: "0"` — canonical Document version tag
    /// - `$dataContractId`, `$type`, `$entropy` — wasm-side metadata
    /// - V0 Document fields (`$id`, `$ownerId`, `$revision`, …) flat
    ///   alongside user-defined properties
    #[wasm_bindgen(js_name = "toObject")]
    pub fn to_object(&self) -> WasmDppResult<DocumentObjectJs> {
        let mut map = self.document.to_map_value()?;
        // Canonical Document version tag — matches `IdentityWasm.toObject`
        // and every other rs-dpp type's canonical wire shape. Symmetric
        // with `fromObject` which now uses canonical
        // `Document::from_object` (requires the tag).
        map.insert("$formatVersion".to_string(), Value::Text("0".to_string()));
        // wasm-side metadata not in core Document
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

    /// Create a Document from a JS object (canonical-tagged shape).
    #[wasm_bindgen(js_name = "fromObject")]
    pub fn from_object(
        value: DocumentObjectJs,
        _platform_version: PlatformVersionLikeJs,
    ) -> WasmDppResult<DocumentWasm> {
        let platform_value = serialization::js_value_to_platform_value(&value.into())?;

        let Value::Map(mut map) = platform_value else {
            return Err(WasmDppError::invalid_argument("Expected an object"));
        };

        // Extract wasm-side metadata fields before passing to canonical
        // `Document::from_object`.
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

        // Canonical `ValueConvertible::from_object` after Phase D step 8
        // slice B. The legacy `from_platform_value` accepted un-tagged
        // shapes; with `toObject` now emitting `$formatVersion: "0"`,
        // canonical handles round-trip cleanly.
        use dpp::serialization::ValueConvertible;
        let document = Document::from_object(Value::Map(map))?;

        Ok(DocumentWasm::new(
            document,
            data_contract_id,
            document_type_name,
            entropy,
        ))
    }

    /// Convert to a JSON-compatible JS object with binary fields as strings.
    #[wasm_bindgen(js_name = "toJSON")]
    pub fn to_json(
        &self,
        _platform_version: PlatformVersionLikeJs,
    ) -> WasmDppResult<DocumentJSONJs> {
        // Canonical `JsonConvertible::to_json` after Phase D step 8 slice A.
        // The legacy `to_json(&self, &PlatformVersion)` was a 1:1 canonical
        // equivalent. `platform_version` stays in the JS API for SDK
        // consistency.
        use dpp::serialization::JsonConvertible;
        let mut json_value = self.document.to_json()?;

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

    /// Create a Document from a JSON object (canonical-tagged shape).
    /// JSON format has identifiers as base58 strings.
    #[wasm_bindgen(js_name = "fromJSON")]
    pub fn from_json(
        value: DocumentJSONJs,
        _platform_version: PlatformVersionLikeJs,
    ) -> WasmDppResult<DocumentWasm> {
        let mut json_value = serialization::js_value_to_json(&value.into())?;

        // Deserialize wrapper fields using serde
        let mut wrapper: DocumentWasm = serde_json::from_value(json_value.clone())
            .map_err(|e| WasmDppError::serialization(e.to_string()))?;

        // Remove wrapper fields from JSON before passing to Document::from_json
        if let serde_json::Value::Object(ref mut obj) = json_value {
            obj.remove("$dataContractId");
            obj.remove("$type");
            obj.remove("$entropy");
        }

        // Canonical `JsonConvertible::from_json` after Phase D step 8
        // slice B. The wasm-dpp2 `toJSON` already emits canonical-tagged
        // JSON (it routes through `Document::to_json` which carries
        // `$formatVersion`), so the round-trip works through canonical.
        use dpp::serialization::JsonConvertible;
        wrapper.document = Document::from_json(json_value)?;

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
        #[wasm_bindgen(js_name = "dataContract")] data_contract: &DataContractWasm,
        #[wasm_bindgen(js_name = "typeName")] type_name: String,
        #[wasm_bindgen(js_name = "platformVersion")] platform_version: PlatformVersionLikeJs,
    ) -> WasmDppResult<DocumentWasm> {
        Self::from_bytes_internal(bytes, data_contract, type_name, platform_version.into())
    }

    #[wasm_bindgen(js_name = "fromHex")]
    pub fn from_hex(
        hex: String,
        #[wasm_bindgen(js_name = "dataContract")] data_contract: &DataContractWasm,
        #[wasm_bindgen(js_name = "typeName")] type_name: String,
        #[wasm_bindgen(js_name = "platformVersion")] platform_version: PlatformVersionLikeJs,
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
        #[wasm_bindgen(js_name = "dataContract")] data_contract: &DataContractWasm,
        #[wasm_bindgen(js_name = "typeName")] type_name: String,
        #[wasm_bindgen(js_name = "platformVersion")] platform_version: PlatformVersionLikeJs,
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

    /// Derives the id of a document that is about to be created, the way
    /// consensus recomputes it for the create transition.
    ///
    /// From protocol version 14 the id commits to the identity contract
    /// nonce of the create transition, so `identityContractNonce` is
    /// required at that version (and ignored before it). The platform
    /// version defaults to the latest.
    #[wasm_bindgen(js_name = "generateId")]
    pub fn generate_id(
        #[wasm_bindgen(js_name = "documentTypeName")] document_type_name: &str,
        #[wasm_bindgen(js_name = "ownerId")] owner_id: IdentifierLikeJs,
        #[wasm_bindgen(js_name = "dataContractId")] data_contract_id: IdentifierLikeJs,
        entropy: Option<Vec<u8>>,
        #[wasm_bindgen(js_name = "identityContractNonce")] identity_contract_nonce: Option<u64>,
        #[wasm_bindgen(js_name = "platformVersion")] platform_version: Option<
            PlatformVersionLikeJs,
        >,
    ) -> WasmDppResult<Vec<u8>> {
        let owner_id: Identifier = owner_id.try_into()?;
        let data_contract_id: Identifier = data_contract_id.try_into()?;

        let entropy_bytes: [u8; 32] = match entropy {
            Some(entropy_vec) => try_vec_to_fixed_bytes(entropy_vec, "entropy")?,
            None => entropy_generator::DefaultEntropyGenerator
                .generate()
                .map_err(|err| WasmDppError::serialization(err.to_string()))?,
        };

        let platform_version: PlatformVersion = platform_version
            .map(PlatformVersionWasm::try_from)
            .transpose()?
            .unwrap_or_default()
            .into();

        let identity_contract_nonce = match identity_contract_nonce {
            Some(identity_contract_nonce) => identity_contract_nonce,
            None if Document::document_id_depends_on_nonce(&platform_version)? => {
                return Err(WasmDppError::invalid_argument(
                    "'identityContractNonce' is required: from protocol version 14 the id of a \
                     new document commits to the identity contract nonce of its create transition",
                ));
            }
            None => 0,
        };

        let identifier = Document::generate_document_id(
            &data_contract_id,
            &owner_id,
            document_type_name,
            &entropy_bytes,
            identity_contract_nonce,
            &platform_version,
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
        let platform_version = PlatformVersionWasm::try_from(platform_version)?;

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
            data_contract.id().into(),
            type_name,
            None,
        ))
    }
}

impl_try_from_js_value!(DocumentWasm, "Document");

impl_try_from_options!(DocumentWasm);
impl_wasm_type_info!(DocumentWasm, Document);

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::platform_value::string_encoding::Encoding;
    use std::collections::BTreeMap;

    fn note(entropy: Option<[u8; 32]>) -> DocumentWasm {
        let document = Document::V0(DocumentV0 {
            id: Identifier::from([9u8; 32]),
            owner_id: Identifier::from([2u8; 32]),
            properties: BTreeMap::new(),
            revision: Some(1),
            ..Default::default()
        });
        DocumentWasm::new(
            document,
            Identifier::from([1u8; 32]),
            "note".to_string(),
            entropy,
        )
    }

    #[test]
    fn set_id_for_creation_derives_the_id_dpp_pins() {
        // The vector `Document::generate_document_id_v1` pins in rs-dpp; the
        // wasm wrapper must feed its own contract id, type name and entropy
        // into the same derivation.
        let mut document = note(Some([7u8; 32]));

        document
            .set_id_for_creation(1, PlatformVersion::latest())
            .expect("expected an id");

        assert_eq!(
            document.document.id().to_string(Encoding::Hex),
            "e574ae73396611a517691d1f89275b6e99642cb9c176ce8cf879b1665c50f15f"
        );
    }

    #[test]
    fn set_id_for_creation_keeps_the_entropy_only_id_before_protocol_version_14() {
        let mut document = note(Some([7u8; 32]));
        let version_13 = PlatformVersion::get(13).expect("expected version 13");

        document
            .set_id_for_creation(1, version_13)
            .expect("expected an id");

        assert_eq!(
            document.document.id(),
            Document::generate_document_id_v0(
                &Identifier::from([1u8; 32]),
                &Identifier::from([2u8; 32]),
                "note",
                &[7u8; 32],
            )
        );
    }

    #[test]
    fn set_id_for_creation_refuses_a_document_without_entropy() {
        let mut document = note(None);

        let error = document
            .set_id_for_creation(1, PlatformVersion::latest())
            .expect_err("expected an error");

        assert!(error.to_string().contains("entropy"), "{error}");
    }
}
