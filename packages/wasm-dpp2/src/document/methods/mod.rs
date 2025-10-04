use crate::data_contract::DataContractWasm;
use crate::document::DocumentWasm;
use crate::enums::platform::PlatformVersionWasm;
use crate::identifier::IdentifierWasm;
use crate::utils::{ToSerdeJSONExt, WithJsError};
use dpp::ProtocolError;
use dpp::dashcore::hashes::serde::Serialize;
use dpp::data_contract::JsonValue;
use dpp::document::Document;
use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
use dpp::identifier::Identifier;
use dpp::platform_value::Value;
use dpp::platform_value::converter::serde_json::BTreeValueJsonConverter;
use dpp::platform_value::string_encoding::Encoding::{Base64, Hex};
use dpp::platform_value::string_encoding::{decode, encode};
use dpp::prelude::Revision;
use dpp::util::entropy_generator;
use dpp::util::entropy_generator::EntropyGenerator;
use std::collections::BTreeMap;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsError, JsValue};

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
        js_data_contract_id: &JsValue,
        js_owner_id: &JsValue,
        js_document_id: &JsValue,
    ) -> Result<DocumentWasm, JsValue> {
        let data_contract_id = IdentifierWasm::try_from(js_data_contract_id)?;
        let owner_id = IdentifierWasm::try_from(js_owner_id)?;

        let revision = Revision::from(js_revision);

        let document = js_raw_document
            .with_serde_to_platform_value_map()
            .expect("cannot convert document to platform value map");

        let revision = Revision::from(revision);

        let entropy = entropy_generator::DefaultEntropyGenerator
            .generate()
            .map_err(|err| JsValue::from(err.to_string()))?;

        let document_id: IdentifierWasm = match js_document_id.is_undefined() {
            true => crate::utils::generate_document_id_v0(
                &data_contract_id.into(),
                &owner_id.into(),
                js_document_type_name,
                &entropy,
            )?
            .into(),
            false => js_document_id.try_into()?,
        };

        Ok(DocumentWasm {
            owner_id,
            entropy: Some(entropy),
            id: document_id,
            document_type_name: js_document_type_name.to_string(),
            data_contract_id,
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
        })
    }

    #[wasm_bindgen(getter=id)]
    pub fn get_id(&self) -> IdentifierWasm {
        self.id
    }

    #[wasm_bindgen(getter=entropy)]
    pub fn get_entropy(&self) -> Option<Vec<u8>> {
        match self.entropy {
            Some(entropy) => Some(entropy.to_vec()),
            None => None,
        }
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
    pub fn get_properties(&self) -> Result<JsValue, JsValue> {
        let json_value: JsonValue = self
            .properties
            .clone()
            .to_json_value()
            .map_err(ProtocolError::ValueError)
            .with_js_error()?;

        let js_value = json_value.serialize(&serde_wasm_bindgen::Serializer::json_compatible())?;
        Ok(js_value)
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
    pub fn set_id(&mut self, id: &JsValue) -> Result<(), JsValue> {
        self.id = IdentifierWasm::try_from(id)?.clone();
        Ok(())
    }

    #[wasm_bindgen(setter=entropy)]
    pub fn set_entropy(&mut self, entropy: JsValue) {
        match entropy.is_undefined() {
            false => {
                let value = entropy.with_serde_to_platform_value().unwrap();

                let mut entropy = [0u8; 32];
                let bytes = value.as_bytes().unwrap();
                let len = bytes.len().min(32);
                entropy[..len].copy_from_slice(&bytes[..len]);
                self.entropy = Some(entropy);
            }
            true => self.entropy = None,
        }
    }

    #[wasm_bindgen(setter=dataContractId)]
    pub fn set_js_data_contract_id(&mut self, js_contract_id: &JsValue) -> Result<(), JsValue> {
        self.data_contract_id = IdentifierWasm::try_from(js_contract_id.clone())?;

        Ok(())
    }

    #[wasm_bindgen(setter=ownerId)]
    pub fn set_owner_id(&mut self, id: &JsValue) -> Result<(), JsValue> {
        self.owner_id = IdentifierWasm::try_from(id)?.clone();
        Ok(())
    }

    #[wasm_bindgen(setter=properties)]
    pub fn set_properties(&mut self, properties: JsValue) {
        self.properties = properties.with_serde_to_platform_value_map().unwrap()
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

    #[wasm_bindgen(js_name=bytes)]
    pub fn to_bytes(
        &self,
        data_contract: &DataContractWasm,
        js_platform_version: JsValue,
    ) -> Result<Vec<u8>, JsValue> {
        let platform_version = match js_platform_version.is_undefined() {
            true => PlatformVersionWasm::default(),
            false => PlatformVersionWasm::try_from(js_platform_version)?,
        };

        let rs_document: Document = Document::from(self.clone());

        let document_type_ref = data_contract
            .get_document_type_ref_by_name(self.get_document_type_name())
            .map_err(|err| JsValue::from_str(err.to_string().as_str()))?;

        DocumentPlatformConversionMethodsV0::serialize(
            &rs_document,
            document_type_ref,
            &data_contract.clone().into(),
            &platform_version.into(),
        )
        .with_js_error()
    }

    #[wasm_bindgen(js_name=hex)]
    pub fn to_hex(
        &self,
        data_contract: &DataContractWasm,
        js_platform_version: JsValue,
    ) -> Result<String, JsValue> {
        Ok(encode(
            self.to_bytes(data_contract, js_platform_version)?
                .as_slice(),
            Hex,
        ))
    }

    #[wasm_bindgen(js_name=base64)]
    pub fn to_base64(
        &self,
        data_contract: &DataContractWasm,
        js_platform_version: JsValue,
    ) -> Result<String, JsValue> {
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
    ) -> Result<DocumentWasm, JsValue> {
        let platform_version = match js_platform_version.is_undefined() {
            true => PlatformVersionWasm::default(),
            false => PlatformVersionWasm::try_from(js_platform_version)?,
        };

        let document_type_ref = match data_contract.get_document_type_ref_by_name(type_name.clone())
        {
            Ok(type_ref) => Ok(type_ref),
            Err(err) => Err(JsValue::from_str(err.to_string().as_str())),
        }?;

        let rs_document = Document::from_bytes(
            bytes.as_slice(),
            document_type_ref,
            &platform_version.into(),
        )
        .with_js_error()?;

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
    ) -> Result<DocumentWasm, JsValue> {
        DocumentWasm::from_bytes(
            decode(hex.as_str(), Hex).map_err(JsError::from)?,
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
    ) -> Result<DocumentWasm, JsValue> {
        DocumentWasm::from_bytes(
            decode(base64.as_str(), Base64).map_err(JsError::from)?,
            data_contract,
            type_name,
            js_platform_version,
        )
    }

    #[wasm_bindgen(js_name=generateId)]
    pub fn generate_id(
        js_document_type_name: &str,
        js_owner_id: &JsValue,
        js_data_contract_id: &JsValue,
        opt_entropy: Option<Vec<u8>>,
    ) -> Result<Vec<u8>, JsValue> {
        let owner_id = IdentifierWasm::try_from(js_owner_id)?;
        let data_contract_id = IdentifierWasm::try_from(js_data_contract_id)?;

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
                .with_js_error()?,
        };

        let identifier = crate::utils::generate_document_id_v0(
            &data_contract_id.into(),
            &owner_id.into(),
            js_document_type_name,
            &entropy,
        );

        match identifier {
            Ok(identifier) => Ok(identifier.to_vec()),
            Err(err) => Err(err),
        }
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
        self.data_contract_id = data_contract_id.clone();
    }
}
