#![allow(clippy::from_over_into)]

use std::collections::BTreeMap;
use std::convert::TryFrom;

pub use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

use dpp::data_contract::schema::DataContractSchemaMethodsV0;
use dpp::data_contract::DataContract;
use dpp::platform_value::{platform_value, Value};

use dpp::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
use dpp::data_contract::config::DataContractConfig;
use dpp::data_contract::conversion::json::DataContractJsonConversionMethodsV0;
use dpp::data_contract::conversion::value::v0::DataContractValueConversionMethodsV0;
use dpp::data_contract::created_data_contract::CreatedDataContract;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::serialized_version::DataContractInSerializationFormat;
use dpp::prelude::IdentityNonce;
use dpp::serialization::PlatformSerializableWithPlatformVersion;
use dpp::version::PlatformVersion;
use dpp::ProtocolError;

use crate::identifier::identifier_from_js_value;
use crate::metadata::MetadataWasm;
use crate::utils::get_bool_from_options;
use crate::utils::SKIP_VALIDATION_PROPERTY_NAME;
use crate::utils::{Inner, IntoWasm, ToSerdeJSONExt, WithJsError};
use crate::with_js_error;
use crate::{buffer::Buffer, identifier::IdentifierWrapper};

#[wasm_bindgen(js_name=DataContract)]
#[derive(Debug, Clone)]
pub struct DataContractWasm {
    inner: DataContract,
    identity_nonce: Option<IdentityNonce>,
}

/// CreatedDataContract contains entropy and is used to create
/// DataContractCreateTransition
impl From<CreatedDataContract> for DataContractWasm {
    fn from(v: CreatedDataContract) -> Self {
        DataContractWasm {
            inner: v.data_contract().clone(),
            identity_nonce: Some(v.identity_nonce()),
        }
    }
}

/// Regular DataContract does not contain entropy and is used
/// in DataContractUpdateTransition
impl From<DataContract> for DataContractWasm {
    fn from(v: DataContract) -> Self {
        DataContractWasm {
            inner: v,
            identity_nonce: None,
        }
    }
}

impl From<&DataContractWasm> for DataContract {
    fn from(v: &DataContractWasm) -> Self {
        v.inner.clone()
    }
}

impl TryFrom<&DataContractWasm> for CreatedDataContract {
    type Error = ProtocolError;
    fn try_from(v: &DataContractWasm) -> Result<Self, Self::Error> {
        let identity_nonce = v.identity_nonce.unwrap_or_default();

        let platform_version = PlatformVersion::first();

        CreatedDataContract::from_contract_and_identity_nonce(
            v.to_owned().into(),
            identity_nonce,
            platform_version,
        )
    }
}

impl Into<DataContract> for DataContractWasm {
    fn into(self) -> DataContract {
        self.inner
    }
}

#[wasm_bindgen(js_class=DataContract)]
impl DataContractWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(
        raw_parameters: JsValue,
        options: Option<js_sys::Object>,
    ) -> Result<DataContractWasm, JsValue> {
        let skip_validation = if let Some(opts) = options {
            get_bool_from_options(opts.into(), SKIP_VALIDATION_PROPERTY_NAME, false)?
        } else {
            false
        };

        let platform_version = PlatformVersion::first();

        DataContract::from_value(
            raw_parameters.with_serde_to_platform_value()?,
            !skip_validation,
            platform_version,
        )
        .with_js_error()
        .map(Into::into)
    }

    #[wasm_bindgen(js_name=getId)]
    pub fn id(&self) -> IdentifierWrapper {
        self.inner.id().into()
    }

    #[wasm_bindgen(js_name=setId)]
    pub fn set_id(&mut self, id: &JsValue) -> Result<(), JsValue> {
        let id = identifier_from_js_value(id)?;

        self.inner.set_id(id);

        Ok(())
    }

    #[wasm_bindgen(js_name=getOwnerId)]
    pub fn owner_id(&self) -> IdentifierWrapper {
        self.inner.owner_id().into()
    }

    #[wasm_bindgen(js_name=getVersion)]
    pub fn version(&self) -> u32 {
        self.inner.version()
    }

    #[wasm_bindgen(js_name=setVersion)]
    pub fn set_version(&mut self, v: u32) {
        self.inner.set_version(v);
    }

    #[wasm_bindgen(js_name=incrementVersion)]
    pub fn increment_version(&mut self) {
        self.inner.increment_version()
    }

    #[wasm_bindgen(js_name=getBinaryProperties)]
    pub fn get_binary_properties(&self, doc_type: &str) -> Result<JsValue, JsValue> {
        let serializer = serde_wasm_bindgen::Serializer::json_compatible();

        let document_type = self
            .inner
            .document_type_for_name(doc_type)
            .map_err(ProtocolError::DataContractError)
            .with_js_error()?;

        let mut binary_paths = BTreeMap::new();

        document_type.binary_paths().iter().for_each(|path| {
            binary_paths.insert(path.to_owned(), platform_value!({}));
        });

        document_type.identifier_paths().iter().for_each(|path| {
            binary_paths.insert(
                path.to_owned(),
                platform_value!({
                    "contentMediaType": "application/x.dash.dpp.identifier"
                }),
            );
        });

        with_js_error!(binary_paths.serialize(&serializer))
    }

    #[wasm_bindgen(js_name=setDocumentSchemas)]
    pub fn set_document_schemas(
        &mut self,
        schemas: JsValue,
        options: Option<js_sys::Object>,
    ) -> Result<(), JsValue> {
        let (skip_validation, defs) = if let Some(opts) = options {
            let opts_value = opts.with_serde_to_platform_value()?;

            (
                get_bool_from_options(opts.into(), SKIP_VALIDATION_PROPERTY_NAME, false)?,
                opts_value
                    .get_optional_value("defs")
                    .map_err(JsError::from)?
                    .map(|defs| defs.clone().into_btree_string_map())
                    .transpose()
                    .map_err(JsError::from)?,
            )
        } else {
            (false, None)
        };

        let document_schemas: Value = schemas.with_serde_to_platform_value()?;
        let document_schemas_map = document_schemas
            .into_btree_string_map()
            .map_err(JsError::from)?;

        let platform_version = PlatformVersion::first();

        self.inner
            .set_document_schemas(
                document_schemas_map,
                defs,
                !skip_validation,
                &mut vec![],
                platform_version,
            )
            .with_js_error()
    }

    #[wasm_bindgen(js_name=setDocumentSchema)]
    pub fn set_document_schema(
        &mut self,
        name: &str,
        schema: JsValue,
        options: Option<js_sys::Object>,
    ) -> Result<(), JsValue> {
        let skip_validation = if let Some(options) = options {
            get_bool_from_options(options.into(), SKIP_VALIDATION_PROPERTY_NAME, false)?
        } else {
            false
        };

        let schema_value: Value = schema.with_serde_to_platform_value()?;

        let platform_version = PlatformVersion::first();

        self.inner
            .set_document_schema(
                name,
                schema_value,
                !skip_validation,
                &mut vec![],
                platform_version,
            )
            .with_js_error()
    }

    #[wasm_bindgen(js_name=getDocumentSchema)]
    pub fn document_schema(&mut self, name: &str) -> Result<JsValue, JsValue> {
        let document_type = self
            .inner
            .document_type_for_name(name)
            .map_err(ProtocolError::DataContractError)
            .with_js_error()?;

        let serializer = serde_wasm_bindgen::Serializer::json_compatible();

        document_type
            .schema()
            .serialize(&serializer)
            .map_err(JsError::from)
            .map_err(Into::into)
    }

    #[wasm_bindgen(js_name=getDocumentSchemas)]
    pub fn document_schemas(&self) -> JsValue {
        let serializer = serde_wasm_bindgen::Serializer::json_compatible();

        self.inner
            .document_schemas()
            .serialize(&serializer)
            .expect("document schemas defs to js value")
    }

    #[wasm_bindgen(js_name=getSchemaDefs)]
    pub fn schema_defs(&self) -> JsValue {
        let serializer = serde_wasm_bindgen::Serializer::json_compatible();

        self.inner
            .schema_defs()
            .serialize(&serializer)
            .expect("document schemas defs to js value")
    }

    #[wasm_bindgen(js_name=setSchemaDefs)]
    pub fn set_schema_defs(
        &mut self,
        defs: Option<js_sys::Object>,
        options: Option<js_sys::Object>,
    ) -> Result<(), JsValue> {
        let skip_validation = if let Some(options) = options {
            get_bool_from_options(options.into(), SKIP_VALIDATION_PROPERTY_NAME, false)?
        } else {
            false
        };

        let platform_version = PlatformVersion::first();

        let maybe_schema_defs = defs
            .map(|js_value| serde_wasm_bindgen::from_value(js_value.into()))
            .transpose()?;

        self.inner
            .set_schema_defs(
                maybe_schema_defs,
                !skip_validation,
                &mut vec![],
                platform_version,
            )
            .with_js_error()?;

        Ok(())
    }

    #[wasm_bindgen(js_name=hasDocumentType)]
    pub fn has_document_type(&self, doc_type: String) -> bool {
        self.inner.has_document_type_for_name(&doc_type)
    }

    #[wasm_bindgen(js_name=setIdentityNonce)]
    pub fn set_identity_nonce(&mut self, e: u64) -> Result<(), JsValue> {
        self.identity_nonce = Some(e);
        Ok(())
    }

    #[wasm_bindgen(js_name=getIdentityNonce)]
    pub fn identity_nonce(&mut self) -> u64 {
        self.identity_nonce.unwrap_or_default()
    }

    #[wasm_bindgen(js_name=getMetadata)]
    pub fn metadata(&self) -> Option<MetadataWasm> {
        self.inner.metadata().cloned().map(Into::into)
    }

    #[wasm_bindgen(js_name=setMetadata)]
    pub fn set_metadata(&mut self, metadata: JsValue) -> Result<(), JsValue> {
        let metadata = if !metadata.is_falsy() {
            let metadata = metadata.to_wasm::<MetadataWasm>("Metadata")?;
            Some(metadata.to_owned().into())
        } else {
            None
        };

        self.inner.set_metadata(metadata);

        Ok(())
    }

    #[wasm_bindgen(js_name=toObject)]
    pub fn to_object(&self) -> Result<JsValue, JsValue> {
        let platform_version = PlatformVersion::first();

        let value = self.inner.to_value(platform_version).with_js_error()?;

        let serializer = serde_wasm_bindgen::Serializer::json_compatible();

        let object = with_js_error!(value.serialize(&serializer))?;

        js_sys::Reflect::set(
            &object,
            &Into::<JsValue>::into("id".to_owned()),
            &Into::<JsValue>::into(Buffer::from_bytes_owned(self.inner.id().to_vec())),
        )
        .expect("target is an object");
        js_sys::Reflect::set(
            &object,
            &Into::<JsValue>::into("ownerId".to_owned()),
            &Into::<JsValue>::into(Buffer::from_bytes_owned(self.inner.owner_id().to_vec())),
        )
        .expect("target is an object");
        Ok(object)
    }

    #[wasm_bindgen(js_name=getConfig)]
    pub fn config(&self) -> Result<JsValue, JsValue> {
        Ok(serde_wasm_bindgen::to_value(&self.inner.config())?)
    }

    #[wasm_bindgen(js_name=setConfig)]
    pub fn set_config(&mut self, config: JsValue) -> Result<(), JsValue> {
        let value = config.with_serde_to_platform_value()?;

        let platform_version = &PlatformVersion::first();

        let data_contract_config =
            DataContractConfig::from_value(value, platform_version).with_js_error()?;

        self.inner.set_config(data_contract_config);

        Ok(())
    }

    #[wasm_bindgen(js_name=toJSON)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        let platform_version = PlatformVersion::first();

        let json = self.inner.to_json(platform_version).with_js_error()?;
        let serializer = serde_wasm_bindgen::Serializer::json_compatible();
        with_js_error!(json.serialize(&serializer))
    }

    #[wasm_bindgen(js_name=toBuffer)]
    pub fn to_buffer(&self) -> Result<Buffer, JsValue> {
        let platform_version = PlatformVersion::first();

        let bytes = self
            .inner
            .serialize_to_bytes_with_platform_version(platform_version)
            .with_js_error()?;

        Ok(Buffer::from_bytes(&bytes))
    }

    #[wasm_bindgen(js_name=hash)]
    pub fn hash(&self) -> Result<Vec<u8>, JsValue> {
        let platform_version = PlatformVersion::first();

        self.inner.hash(platform_version).with_js_error()
    }

    // #[wasm_bindgen(js_name=from)]
    // pub fn from_js_value(v: JsValue) -> Result<DataContractWasm, JsValue> {
    //     let json_contract: JsonValue = with_js_error!(serde_wasm_bindgen::from_value(v))?;
    //     Ok(DataContract::try_from(json_contract)
    //         .with_js_error()?
    //         .into())
    // }

    // #[wasm_bindgen(js_name=fromBuffer)]
    // pub fn from_buffer(b: &[u8]) -> Result<DataContractWasm, JsValue> {
    //     let data_contract = DataContract::from_cbor(b).with_js_error()?;
    //     Ok(data_contract.into())
    // }

    #[wasm_bindgen(js_name=clone)]
    pub fn deep_clone(&self) -> Self {
        self.clone()
    }

    pub(crate) fn try_from_serialization_format(
        value: DataContractInSerializationFormat,
        full_validation: bool,
    ) -> Result<Self, JsValue> {
        let platform_version = PlatformVersion::first();

        DataContract::try_from_platform_versioned(
            value,
            full_validation,
            &mut vec![],
            platform_version,
        )
        .with_js_error()
        .map(Into::into)
    }
}

impl Inner for DataContractWasm {
    type InnerItem = DataContract;

    fn into_inner(self) -> DataContract {
        self.inner
    }

    fn inner(&self) -> &DataContract {
        &self.inner
    }

    fn inner_mut(&mut self) -> &mut DataContract {
        &mut self.inner
    }
}

impl DataContractWasm {
    pub fn inner(&self) -> &DataContract {
        &self.inner
    }
}
