use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::{IdentifierLikeJs, IdentifierWasm};
use crate::impl_try_from_js_value;
use crate::impl_try_from_options;
use crate::impl_wasm_type_info;
use crate::serialization;
use crate::tokens::configuration::TokenConfigurationWasm;
use crate::tokens::configuration::group::GroupWasm;
use crate::utils::{IntoWasm, JsValueExt};
use crate::version::{PlatformVersionLikeJs, PlatformVersionWasm};
use dpp::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
use dpp::data_contract::accessors::v1::{DataContractV1Getters, DataContractV1Setters};
use dpp::data_contract::config::DataContractConfig;
use dpp::data_contract::conversion::json::DataContractJsonConversionMethodsV0;
use dpp::data_contract::conversion::value::v0::DataContractValueConversionMethodsV0;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::data_contract::errors::DataContractError;
use dpp::data_contract::group::Group;
use dpp::data_contract::schema::DataContractSchemaMethodsV0;
use dpp::data_contract::{
    DataContract, GroupContractPosition, TokenConfiguration, TokenContractPosition,
};
use dpp::platform_value::string_encoding::Encoding::{Base64, Hex};
use dpp::platform_value::string_encoding::{decode, encode};
use dpp::platform_value::{Value, ValueMap};
use dpp::prelude::{Identifier, IdentityNonce};
use dpp::serialization::{
    PlatformDeserializableWithPotentialValidationFromVersionedStructure,
    PlatformSerializableWithPlatformVersion,
};
use dpp::version::PlatformVersion;
use js_sys::{Object, Reflect};
use serde::Deserialize;
use std::collections::BTreeMap;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsCast, JsValue};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DataContractOptions {
    identity_nonce: IdentityNonce,
    #[serde(default = "default_full_validation")]
    full_validation: bool,
}

fn default_full_validation() -> bool {
    true
}

#[wasm_bindgen(typescript_custom_section)]
const TS_TYPES: &'static str = r#"
export interface DataContractOptions {
    ownerId: IdentifierLike;
    identityNonce: bigint;
    schemas: object;
    definitions?: object;
    tokens?: Record<number, TokenConfiguration>;
    fullValidation?: boolean;
    platformVersion?: PlatformVersion | string | number;
}

/**
 * DataContract serialized as a plain object.
 */
export interface DataContractObject {
    $format_version: string;
    id: Identifier;
    ownerId: Identifier;
    version: number;
    documentSchemas: Record<string, object>;
    config?: DataContractConfig;
    groups?: Record<number, Group>;
    tokens?: Record<number, TokenConfiguration>;
    [key: string]: unknown;
}

/**
 * DataContract serialized as JSON (with string identifiers).
 */
export interface DataContractJSON {
    $format_version: string;
    id: string;
    ownerId: string;
    version: number;
    documentSchemas: Record<string, object>;
    config?: DataContractConfig;
    groups?: Record<number, object>;
    tokens?: Record<number, object>;
    [key: string]: unknown;
}

/**
 * DataContract configuration.
 */
export interface DataContractConfig {
    canBeDeleted: boolean;
    readonly: boolean;
    keepsHistory: boolean;
    documentsKeepHistoryContractDefault: boolean;
    documentsMutableContractDefault: boolean;
    documentsCanBeDeletedContractDefault: boolean;
    requiresIdentityEncryptionBoundedKey?: number;
    requiresIdentityDecryptionBoundedKey?: number;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "DataContractOptions")]
    pub type DataContractOptionsJs;

    #[wasm_bindgen(typescript_type = "DataContractObject")]
    pub type DataContractObjectJs;

    #[wasm_bindgen(typescript_type = "DataContractJSON")]
    pub type DataContractJSONJs;

    #[wasm_bindgen(typescript_type = "DataContractConfig")]
    pub type DataContractConfigJs;
}

#[wasm_bindgen(js_name = "DataContract")]
#[derive(Clone)]
pub struct DataContractWasm(DataContract);

impl From<DataContract> for DataContractWasm {
    fn from(v: DataContract) -> Self {
        DataContractWasm(v)
    }
}

impl From<DataContractWasm> for DataContract {
    fn from(v: DataContractWasm) -> Self {
        v.0
    }
}

pub fn tokens_configuration_from_js_value(
    configuration: &JsValue,
) -> WasmDppResult<BTreeMap<TokenContractPosition, TokenConfiguration>> {
    let configuration_object = Object::from(configuration.clone());
    let configuration_keys = Object::keys(&configuration_object);

    let mut configuration: BTreeMap<TokenContractPosition, TokenConfiguration> = BTreeMap::new();

    for key in configuration_keys.iter() {
        let contract_position = match key.as_string() {
            None => Err(WasmDppError::invalid_argument(
                "Cannot read timestamp in distribution rules",
            )),
            Some(contract_position) => Ok(contract_position
                .parse::<GroupContractPosition>()
                .map_err(|e| WasmDppError::serialization(e.to_string()))?),
        }?;

        let js_config = Reflect::get(&configuration_object, &key)
            .map_err(|err| {
                let message = err.error_message();
                WasmDppError::invalid_argument(format!(
                    "unable to read token configuration at contract position '{}': {}",
                    contract_position, message
                ))
            })?
            .to_wasm::<TokenConfigurationWasm>("TokenConfiguration")?
            .clone();

        configuration.insert(contract_position, js_config.into());
    }

    Ok(configuration)
}

#[wasm_bindgen(js_class = DataContract)]
impl DataContractWasm {
    #[wasm_bindgen(constructor)]
    pub fn constructor(options: DataContractOptionsJs) -> WasmDppResult<DataContractWasm> {
        let options: JsValue = options.into();
        let object = Object::from(options.clone());

        // Extract ownerId (required)
        let js_owner_id = Reflect::get(&object, &JsValue::from_str("ownerId"))
            .map_err(|e| WasmDppError::invalid_argument(format!("Missing ownerId: {:?}", e)))?;
        let owner_id: IdentifierWasm = js_owner_id.try_into()?;

        // Extract schemas (required)
        let js_schema = Reflect::get(&object, &JsValue::from_str("schemas"))
            .map_err(|e| WasmDppError::invalid_argument(format!("Missing schemas: {:?}", e)))?;
        let schema: Value = serialization::platform_value_from_object(js_schema)?;

        // Extract definitions (optional)
        let js_definitions =
            Reflect::get(&object, &JsValue::from_str("definitions")).unwrap_or(JsValue::UNDEFINED);
        let definitions: Option<Value> =
            match js_definitions.is_undefined() | js_definitions.is_null() {
                true => None,
                false => Some(serialization::platform_value_from_object(js_definitions)?),
            };

        // Extract tokens (optional)
        let js_tokens =
            Reflect::get(&object, &JsValue::from_str("tokens")).unwrap_or(JsValue::UNDEFINED);
        let tokens: BTreeMap<TokenContractPosition, TokenConfiguration> =
            match js_tokens.is_undefined() | js_tokens.is_null() {
                true => BTreeMap::new(),
                false => tokens_configuration_from_js_value(&js_tokens)?,
            };

        // Extract platformVersion (optional)
        let js_platform_version = Reflect::get(&object, &JsValue::from_str("platformVersion"))
            .unwrap_or(JsValue::UNDEFINED);
        let platform_version: PlatformVersion = match js_platform_version.is_undefined() {
            true => PlatformVersionWasm::default().into(),
            false => PlatformVersionWasm::try_from(js_platform_version)?.into(),
        };

        // Extract simple fields via serde
        let opts: DataContractOptions = serde_wasm_bindgen::from_value(options)
            .map_err(|e| WasmDppError::invalid_argument(e.to_string()))?;

        let data_contract_structure_version_value = Value::from(
            platform_version
                .dpp
                .contract_versions
                .contract_structure_version
                .to_string(),
        );

        let definitions_value = Value::from(definitions);

        let data_contract_id =
            DataContract::generate_data_contract_id_v0(owner_id.to_bytes(), opts.identity_nonce);

        let data_contract_id_value = Value::Identifier(data_contract_id.to_buffer());

        let config = DataContractConfig::default_for_version(&platform_version.clone())?;

        let config_value: Value = dpp::platform_value::to_value(config)
            .map_err(|e| WasmDppError::serialization(e.to_string()))?;

        let mut contract_value = Value::Map(ValueMap::new());

        contract_value
            .set_value("$format_version", data_contract_structure_version_value)
            .map_err(|err| WasmDppError::serialization(err.to_string()))?;

        contract_value
            .set_value("id", data_contract_id_value)
            .map_err(|err| WasmDppError::serialization(err.to_string()))?;

        contract_value
            .set_value("config", config_value)
            .map_err(|err| WasmDppError::serialization(err.to_string()))?;

        contract_value
            .set_value("version", Value::from(1u16))
            .map_err(|err| WasmDppError::serialization(err.to_string()))?;

        let owner_id_bytes: [u8; 32] = owner_id
            .to_bytes()
            .try_into()
            .map_err(|_| WasmDppError::invalid_argument("Invalid owner ID length"))?;

        contract_value
            .set_value("ownerId", Value::Identifier(owner_id_bytes))
            .map_err(|err| WasmDppError::serialization(err.to_string()))?;

        contract_value
            .set_value("schemaDefs", definitions_value)
            .map_err(|err| WasmDppError::serialization(err.to_string()))?;

        contract_value
            .set_value("documentSchemas", schema)
            .map_err(|err| WasmDppError::serialization(err.to_string()))?;

        let data_contract =
            DataContract::from_value(contract_value, opts.full_validation, &platform_version)?;

        let data_contract_with_tokens = match data_contract {
            DataContract::V0(v0) => DataContract::from(v0),
            DataContract::V1(mut v1) => {
                v1.set_tokens(tokens);

                DataContract::from(v1)
            }
        };

        Ok(DataContractWasm(data_contract_with_tokens))
    }

    #[wasm_bindgen(js_name = "fromJSON")]
    pub fn from_json(
        value: DataContractJSONJs,
        full_validation: bool,
        platform_version: PlatformVersionLikeJs,
    ) -> WasmDppResult<DataContractWasm> {
        let platform_version: JsValue = platform_version.into();
        let platform_version = match platform_version.is_undefined() {
            true => PlatformVersionWasm::default(),
            false => PlatformVersionWasm::try_from(platform_version)?,
        };

        let json_value = serialization::js_value_to_json(&value.into())?;

        let contract =
            DataContract::from_json(json_value, full_validation, &platform_version.into())?;

        Ok(DataContractWasm(contract))
    }

    #[wasm_bindgen(js_name = "fromObject")]
    pub fn from_object(
        value: DataContractObjectJs,
        full_validation: bool,
        platform_version: PlatformVersionLikeJs,
    ) -> WasmDppResult<DataContractWasm> {
        let platform_version: JsValue = platform_version.into();
        let platform_version = match platform_version.is_undefined() {
            true => PlatformVersionWasm::default(),
            false => PlatformVersionWasm::try_from(platform_version)?,
        };

        let platform_value: Value = serialization::platform_value_from_object(value.into())?;

        let contract =
            DataContract::from_value(platform_value, full_validation, &platform_version.into())
                .map_err(WasmDppError::from)?;

        Ok(DataContractWasm(contract))
    }

    #[wasm_bindgen(js_name = "fromBytes")]
    pub fn from_bytes(
        bytes: Vec<u8>,
        full_validation: bool,
        platform_version: PlatformVersionLikeJs,
    ) -> WasmDppResult<DataContractWasm> {
        Self::from_bytes_internal(bytes, full_validation, platform_version.into())
    }

    #[wasm_bindgen(js_name = "fromHex")]
    pub fn from_hex(
        hex: String,
        full_validation: bool,
        platform_version: PlatformVersionLikeJs,
    ) -> WasmDppResult<DataContractWasm> {
        let bytes =
            decode(hex.as_str(), Hex).map_err(|e| WasmDppError::serialization(e.to_string()))?;

        Self::from_bytes_internal(bytes, full_validation, platform_version.into())
    }

    #[wasm_bindgen(js_name = "fromBase64")]
    pub fn from_base64(
        base64: String,
        full_validation: bool,
        platform_version: PlatformVersionLikeJs,
    ) -> WasmDppResult<DataContractWasm> {
        let bytes = decode(base64.as_str(), Base64)
            .map_err(|e| WasmDppError::serialization(e.to_string()))?;

        Self::from_bytes_internal(bytes, full_validation, platform_version.into())
    }

    #[wasm_bindgen(js_name = "toBytes")]
    pub fn to_bytes(&self, platform_version: PlatformVersionLikeJs) -> WasmDppResult<Vec<u8>> {
        self.to_bytes_internal(platform_version.into())
    }

    #[wasm_bindgen(js_name = "toHex")]
    pub fn to_hex(&self, platform_version: PlatformVersionLikeJs) -> WasmDppResult<String> {
        Ok(encode(
            self.to_bytes_internal(platform_version.into())?.as_slice(),
            Hex,
        ))
    }

    #[wasm_bindgen(js_name = "toBase64")]
    pub fn to_base64(&self, platform_version: PlatformVersionLikeJs) -> WasmDppResult<String> {
        Ok(encode(
            self.to_bytes_internal(platform_version.into())?.as_slice(),
            Base64,
        ))
    }

    #[wasm_bindgen(js_name = "toObject")]
    pub fn to_object(
        &self,
        platform_version: PlatformVersionLikeJs,
    ) -> WasmDppResult<DataContractObjectJs> {
        let platform_version: JsValue = platform_version.into();
        let platform_version = match platform_version.is_undefined() {
            true => PlatformVersionWasm::default(),
            false => PlatformVersionWasm::try_from(platform_version)?,
        };

        let value = self.0.clone().to_value(&platform_version.into())?;
        let js_value = serialization::platform_value_to_object(&value)?;
        Ok(js_value.unchecked_into())
    }

    #[wasm_bindgen(getter = "schemas")]
    pub fn schemas(&self) -> WasmDppResult<JsValue> {
        serialization::to_object(&self.0.document_schemas())
    }

    #[wasm_bindgen(getter = "version")]
    pub fn get_version(&self) -> u32 {
        self.0.version()
    }

    #[wasm_bindgen(getter = "id")]
    pub fn get_id(&self) -> IdentifierWasm {
        self.0.id().into()
    }

    #[wasm_bindgen(getter = "ownerId")]
    pub fn get_owner_id(&self) -> IdentifierWasm {
        self.0.owner_id().into()
    }

    #[wasm_bindgen(getter = "config")]
    pub fn config(&self) -> WasmDppResult<JsValue> {
        serialization::to_object(self.0.config())
    }

    #[wasm_bindgen(getter = "tokens")]
    pub fn get_tokens(&self) -> WasmDppResult<Object> {
        let tokens_object = Object::new();

        for (key, value) in self.0.tokens().iter() {
            Reflect::set(
                &tokens_object,
                &JsValue::from(*key),
                &JsValue::from(TokenConfigurationWasm::from(value.clone())),
            )
            .map_err(|err| {
                let message = err.error_message();
                WasmDppError::generic(format!(
                    "unable to serialize token configuration at position '{}': {}",
                    key, message
                ))
            })?;
        }

        Ok(tokens_object)
    }

    #[wasm_bindgen(getter = "groups")]
    pub fn get_groups(&self) -> WasmDppResult<JsValue> {
        let groups_object = Object::new();

        for (key, value) in self.0.groups().iter() {
            Reflect::set(
                &groups_object,
                &JsValue::from(*key),
                &JsValue::from(GroupWasm::from(value.clone())),
            )
            .map_err(|err| {
                let message = err.error_message();
                WasmDppError::generic(format!(
                    "unable to serialize group at position '{}': {}",
                    key, message
                ))
            })?;
        }

        Ok(groups_object.into())
    }

    #[wasm_bindgen(setter = "id")]
    pub fn set_id(&mut self, id: IdentifierLikeJs) -> WasmDppResult<()> {
        self.0.set_id(id.try_into()?);
        Ok(())
    }

    #[wasm_bindgen(setter = "ownerId")]
    pub fn set_owner_id(&mut self, owner_id: IdentifierLikeJs) -> WasmDppResult<()> {
        self.0.set_owner_id(owner_id.try_into()?);
        Ok(())
    }

    #[wasm_bindgen(setter = "version")]
    pub fn set_version(&mut self, version: u32) {
        self.0.set_version(version)
    }

    #[wasm_bindgen(js_name = "setConfig")]
    pub fn set_config(
        &mut self,
        config: DataContractConfigJs,
        platform_version: PlatformVersionLikeJs,
    ) -> WasmDppResult<()> {
        let config: JsValue = config.into();
        let platform_version: JsValue = platform_version.into();
        let platform_version = match platform_version.is_undefined() {
            true => PlatformVersionWasm::default(),
            false => PlatformVersionWasm::try_from(platform_version)?,
        };

        let config_value: Value = serde_wasm_bindgen::from_value(config)
            .map_err(|err| WasmDppError::serialization(err.to_string()))?;

        let config = DataContractConfig::from_value(config_value, &platform_version.into())?;

        self.0.set_config(config);

        Ok(())
    }

    #[wasm_bindgen(js_name = "setSchemas")]
    pub fn set_schemas(
        &mut self,
        #[wasm_bindgen(unchecked_param_type = "Record<string, object>")] schemas: JsValue,
        definitions: Option<js_sys::Object>,
        full_validation: bool,
        platform_version: PlatformVersionLikeJs,
    ) -> WasmDppResult<()> {
        let platform_version: JsValue = platform_version.into();
        let platform_version = match platform_version.is_undefined() {
            true => PlatformVersionWasm::default(),
            false => PlatformVersionWasm::try_from(platform_version)?,
        };

        // Use platform_value_from_object to match getSchemas' to_object serialization
        // This preserves integer types properly (avoids JSON round-trip which converts to strings)
        let schema_value = serialization::platform_value_from_object(schemas)?;
        let schema = schema_value
            .into_btree_string_map()
            .map_err(|err| WasmDppError::invalid_argument(err.to_string()))?;

        let definitions: Option<BTreeMap<String, Value>> = definitions
            .map(|defs| serde_wasm_bindgen::from_value(defs.into()))
            .transpose()
            .map_err(|err| WasmDppError::serialization(err.to_string()))?;

        self.0.set_document_schemas(
            schema,
            definitions,
            full_validation,
            &mut Vec::new(),
            &platform_version.into(),
        )?;

        Ok(())
    }

    #[wasm_bindgen(setter = "tokens")]
    pub fn set_tokens(
        &mut self,
        #[wasm_bindgen(unchecked_param_type = "Record<number, TokenConfiguration>")]
        tokens: &JsValue,
    ) -> WasmDppResult<()> {
        self.0
            .set_tokens(tokens_configuration_from_js_value(tokens)?);
        Ok(())
    }

    #[wasm_bindgen(setter = "groups")]
    pub fn set_groups(
        &mut self,
        #[wasm_bindgen(unchecked_param_type = "Record<number, Group>")] groups: &JsValue,
    ) -> WasmDppResult<()> {
        let groups_object = Object::from(groups.clone());

        let mut groups: BTreeMap<GroupContractPosition, Group> = BTreeMap::new();

        for js_position in Object::keys(&groups_object) {
            let position_str = js_position.as_string().ok_or_else(|| {
                WasmDppError::invalid_argument(format!(
                    "Group position '{:?}' must be a stringified number.",
                    js_position
                ))
            })?;

            let position = position_str.parse::<u16>().map_err(|err| {
                WasmDppError::invalid_argument(format!(
                    "Invalid group position '{}': {}.",
                    position_str, err
                ))
            })?;

            let js_group = Reflect::get(&groups_object, &js_position).map_err(|err| {
                let message = err.error_message();
                WasmDppError::invalid_argument(format!(
                    "unable to read group at position '{}': {}",
                    position_str, message
                ))
            })?;

            let group = js_group.to_wasm::<GroupWasm>("Group")?.clone();

            groups.insert(position, group.into());
        }

        self.0.set_groups(groups);

        Ok(())
    }

    #[wasm_bindgen(js_name = "toJSON")]
    pub fn to_json(
        &self,
        platform_version: PlatformVersionLikeJs,
    ) -> WasmDppResult<DataContractJSONJs> {
        let platform_version: JsValue = platform_version.into();
        let platform_version = match platform_version.is_undefined() {
            true => PlatformVersionWasm::default(),
            false => PlatformVersionWasm::try_from(platform_version)?,
        };

        let json = self.0.to_json(&platform_version.into())?;
        let js_value = serialization::to_json(&json)?;
        Ok(js_value.unchecked_into())
    }

    #[wasm_bindgen(js_name = "generateId")]
    pub fn generate_id(
        owner_id: IdentifierLikeJs,
        identity_nonce: IdentityNonce,
    ) -> WasmDppResult<IdentifierWasm> {
        let owner_id: Identifier = owner_id.try_into()?;
        Ok(DataContract::generate_data_contract_id_v0(owner_id.to_buffer(), identity_nonce).into())
    }
}

impl DataContractWasm {
    pub fn get_document_type_ref_by_name(
        &self,
        name: String,
    ) -> Result<DocumentTypeRef<'_>, DataContractError> {
        self.0.document_type_for_name(name.as_str()).clone()
    }

    fn from_bytes_internal(
        bytes: Vec<u8>,
        full_validation: bool,
        platform_version: JsValue,
    ) -> WasmDppResult<DataContractWasm> {
        let platform_version = match platform_version.is_undefined() {
            true => PlatformVersionWasm::default(),
            false => PlatformVersionWasm::try_from(platform_version)?,
        };

        let rs_data_contract = DataContract::versioned_deserialize(
            bytes.as_slice(),
            full_validation,
            &platform_version.into(),
        )?;

        Ok(DataContractWasm(rs_data_contract))
    }

    fn to_bytes_internal(&self, platform_version: JsValue) -> WasmDppResult<Vec<u8>> {
        let platform_version = match platform_version.is_undefined() {
            true => PlatformVersionWasm::default(),
            false => PlatformVersionWasm::try_from(platform_version)?,
        };

        let rs_data_contract: DataContract = self.0.clone();

        Ok(rs_data_contract.serialize_to_bytes_with_platform_version(&platform_version.into())?)
    }
}

impl_try_from_js_value!(DataContractWasm, "DataContract");
impl_try_from_options!(DataContractWasm);
impl_wasm_type_info!(DataContractWasm, DataContract);
