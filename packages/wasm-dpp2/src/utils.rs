use crate::error::{WasmDppError, WasmDppResult};
use anyhow::{anyhow, bail};
use dpp::identifier::Identifier;
use dpp::platform_value::Value;
use dpp::util::hash::hash_double_to_vec;
use js_sys::{Error as JsError, Object};
use serde_json::Value as JsonValue;
use std::collections::BTreeMap;
use std::convert::TryInto;
use wasm_bindgen::convert::RefFromWasmAbi;
use wasm_bindgen::{JsCast, JsValue};

/// Extension trait for extracting error messages from JsValue
pub trait JsValueExt {
    fn error_message(&self) -> String;
}

impl JsValueExt for JsValue {
    fn error_message(&self) -> String {
        if self.is_null() || self.is_undefined() {
            return "JavaScript error: value is null or undefined".to_string();
        }

        if let Some(js_error) = self.dyn_ref::<JsError>() {
            return js_error.message().into();
        }

        if let Some(message) = self.as_string() {
            return message;
        }

        js_sys::JSON::stringify(self)
            .ok()
            .and_then(|v| v.as_string())
            .unwrap_or_else(|| "Unknown JavaScript error".to_string())
    }
}

/// Extension trait for converting JsValue to serde/platform values
pub trait ToSerdeJSONExt {
    fn with_serde_to_json_value(&self) -> WasmDppResult<JsonValue>;
    fn with_serde_to_platform_value(&self) -> WasmDppResult<Value>;
    fn with_serde_to_platform_value_map(&self) -> WasmDppResult<BTreeMap<String, Value>>;
}

impl ToSerdeJSONExt for JsValue {
    fn with_serde_to_json_value(&self) -> WasmDppResult<JsonValue> {
        crate::serialization::js_value_to_json(self)
    }

    fn with_serde_to_platform_value(&self) -> WasmDppResult<Value> {
        Ok(self.with_serde_to_json_value()?.into())
    }

    fn with_serde_to_platform_value_map(&self) -> WasmDppResult<BTreeMap<String, Value>> {
        self.with_serde_to_platform_value()?
            .into_btree_string_map()
            .map_err(|err| WasmDppError::invalid_argument(err.to_string()))
    }
}

/// Trait for converting JsValue to a WASM type by reading its internal pointer
pub trait IntoWasm {
    fn to_wasm<T: RefFromWasmAbi<Abi = u32>>(&self, class_name: &str) -> WasmDppResult<T::Anchor>;
}

impl IntoWasm for JsValue {
    fn to_wasm<T: RefFromWasmAbi<Abi = u32>>(&self, class_name: &str) -> WasmDppResult<T::Anchor> {
        generic_of_js_val::<T>(self, class_name)
    }
}

/// Convert a JsValue to a WASM type by reading its internal pointer
pub fn generic_of_js_val<T: RefFromWasmAbi<Abi = u32>>(
    js_value: &JsValue,
    class_name: &str,
) -> WasmDppResult<T::Anchor> {
    if !js_value.is_object() {
        return Err(WasmDppError::invalid_argument(format!(
            "Value supplied as {} is not an object",
            class_name
        )));
    }

    let ctor_name = get_class_type(js_value)?;

    if ctor_name == class_name {
        let ptr =
            js_sys::Reflect::get(js_value, &JsValue::from_str("__wbg_ptr")).map_err(|err| {
                let message = err.error_message();
                WasmDppError::generic(format!(
                    "failed to read internal pointer from JS object '{}': {}",
                    class_name, message
                ))
            })?;
        let ptr_u32: u32 = ptr
            .as_f64()
            .ok_or_else(|| WasmDppError::invalid_argument("Invalid JS object pointer"))?
            as u32;
        let reference = unsafe { T::ref_from_abi(ptr_u32) };
        Ok(reference)
    } else {
        let error_string = format!(
            "JS object constructor name mismatch. Expected {}, provided {}.",
            class_name, ctor_name
        );
        Err(WasmDppError::invalid_argument(error_string))
    }
}

/// Get the `__type` property from a JsValue (used for WASM class identification)
pub fn get_class_type(value: &JsValue) -> WasmDppResult<String> {
    let class_type = js_sys::Reflect::get(value, &JsValue::from_str("__type")).map_err(|err| {
        let message = err.error_message();
        WasmDppError::generic(format!(
            "failed to read '__type' property from JS value: {}",
            message
        ))
    })?;

    Ok(class_type.as_string().unwrap_or_default())
}

/// Extract a required property from a JS object.
///
/// This function properly handles the case where `Reflect::get` returns `Ok(undefined)`
/// for missing properties (rather than `Err`), providing clear error messages.
pub fn get_required_property(
    object: &js_sys::Object,
    property_name: &str,
) -> WasmDppResult<JsValue> {
    let value = js_sys::Reflect::get(object, &JsValue::from_str(property_name))
        .map_err(|_| WasmDppError::invalid_argument(format!("Missing '{}' property", property_name)))?;

    if value.is_undefined() {
        return Err(WasmDppError::invalid_argument(format!(
            "Property '{}' is undefined",
            property_name
        )));
    }

    Ok(value)
}

/// Extract an optional property from a JS object.
///
/// Returns `JsValue::UNDEFINED` if the property doesn't exist or if `Reflect::get` fails.
/// This is useful for optional fields where absence should not cause an error.
pub fn get_optional_property(object: &js_sys::Object, property_name: &str) -> JsValue {
    js_sys::Reflect::get(object, &JsValue::from_str(property_name)).unwrap_or(JsValue::UNDEFINED)
}

/// Convert a JS Number or BigInt to u64
pub fn try_to_u64(value: JsValue) -> Result<u64, anyhow::Error> {
    if value.is_bigint() {
        js_sys::BigInt::new(&value)
            .map_err(|e| anyhow!("unable to create bigInt: {}", e.to_string()))?
            .try_into()
            .map_err(|e| anyhow!("conversion of BigInt to u64 failed: {:#}", e))
    } else if value.as_f64().is_some() {
        let number = js_sys::Number::from(value);
        convert_number_to_u64(number)
    } else {
        bail!("supported types are Number or BigInt")
    }
}

/// Convert a JS Number to u64 with validation
fn convert_number_to_u64(js_number: js_sys::Number) -> Result<u64, anyhow::Error> {
    if let Some(float_number) = js_number.as_f64() {
        if float_number.is_nan() || float_number.is_infinite() {
            bail!("received an invalid number: the number is either NaN or Inf")
        }
        if float_number < 0. {
            bail!("received an invalid number: the number is negative");
        }
        if float_number.fract() != 0. {
            bail!("received an invalid number: the number is fractional")
        }
        if float_number > u64::MAX as f64 {
            bail!("received an invalid number: the number is > u64::max")
        }

        return Ok(float_number as u64);
    }
    bail!("the value is not a number")
}

/// Convert a JS value to Object with validation.
///
/// Uses `dyn_into()` to safely convert, returning an error if the value is not an object.
pub fn try_to_object(value: JsValue, field_name: &str) -> WasmDppResult<Object> {
    value
        .dyn_into()
        .map_err(|_| WasmDppError::invalid_argument(format!("'{}' must be an object", field_name)))
}

/// Convert a JS value to bytes (Vec<u8>) with type validation.
///
/// Validates that the value is a Uint8Array using `dyn_into()`.
/// Returns an error if the value is not a Uint8Array.
pub fn try_to_bytes(value: JsValue, field_name: &str) -> WasmDppResult<Vec<u8>> {
    let array: js_sys::Uint8Array = value.dyn_into().map_err(|_| {
        WasmDppError::invalid_argument(format!("'{}' must be a Uint8Array", field_name))
    })?;
    Ok(array.to_vec())
}

/// Convert a JS value to a fixed-size byte array with type and length validation.
///
/// Validates that:
/// - The value is a Uint8Array
/// - The length is exactly N bytes
pub fn try_to_fixed_bytes<const N: usize>(
    value: JsValue,
    field_name: &str,
) -> WasmDppResult<[u8; N]> {
    let bytes = try_to_bytes(value, field_name)?;
    if bytes.len() != N {
        return Err(WasmDppError::invalid_argument(format!(
            "'{}' must be exactly {} bytes, got {}",
            field_name,
            N,
            bytes.len()
        )));
    }
    let mut arr = [0u8; N];
    arr.copy_from_slice(&bytes);
    Ok(arr)
}

/// Convert a JS value to u32 with validation.
///
/// Validates that the value is:
/// - A finite number (not NaN or Infinity)
/// - An integer (no fractional part)
/// - Non-negative
/// - Within u32 range (0..=4294967295)
pub fn try_to_u32(value: JsValue, field_name: &str) -> WasmDppResult<u32> {
    let num = value
        .as_f64()
        .ok_or_else(|| WasmDppError::invalid_argument(format!("'{}' must be a number", field_name)))?;

    if num.is_nan() || num.is_infinite() {
        return Err(WasmDppError::invalid_argument(format!(
            "'{}' must be a finite number, got {}",
            field_name,
            if num.is_nan() { "NaN" } else { "Infinity" }
        )));
    }

    if num.fract() != 0.0 {
        return Err(WasmDppError::invalid_argument(format!(
            "'{}' must be an integer, got {}",
            field_name, num
        )));
    }

    if num < 0.0 {
        return Err(WasmDppError::invalid_argument(format!(
            "'{}' must be non-negative, got {}",
            field_name, num
        )));
    }

    if num > u32::MAX as f64 {
        return Err(WasmDppError::invalid_argument(format!(
            "'{}' must be at most {}, got {}",
            field_name,
            u32::MAX,
            num
        )));
    }

    Ok(num as u32)
}

/// Convert a JS value to u16 with validation.
///
/// Validates that the value is:
/// - A finite number (not NaN or Infinity)
/// - An integer (no fractional part)
/// - Non-negative
/// - Within u16 range (0..=65535)
pub fn try_to_u16(value: JsValue, field_name: &str) -> WasmDppResult<u16> {
    let num = value
        .as_f64()
        .ok_or_else(|| WasmDppError::invalid_argument(format!("'{}' must be a number", field_name)))?;

    if num.is_nan() || num.is_infinite() {
        return Err(WasmDppError::invalid_argument(format!(
            "'{}' must be a finite number, got {}",
            field_name,
            if num.is_nan() { "NaN" } else { "Infinity" }
        )));
    }

    if num.fract() != 0.0 {
        return Err(WasmDppError::invalid_argument(format!(
            "'{}' must be an integer, got {}",
            field_name, num
        )));
    }

    if num < 0.0 {
        return Err(WasmDppError::invalid_argument(format!(
            "'{}' must be non-negative, got {}",
            field_name, num
        )));
    }

    if num > u16::MAX as f64 {
        return Err(WasmDppError::invalid_argument(format!(
            "'{}' must be at most {}, got {}",
            field_name,
            u16::MAX,
            num
        )));
    }

    Ok(num as u16)
}

/// Generate a document ID using the v0 algorithm
pub fn generate_document_id_v0(
    contract_id: &Identifier,
    owner_id: &Identifier,
    document_type_name: &str,
    entropy: &[u8],
) -> WasmDppResult<Identifier> {
    let mut buf: Vec<u8> = vec![];

    buf.extend_from_slice(&contract_id.to_buffer());
    buf.extend_from_slice(&owner_id.to_buffer());
    buf.extend_from_slice(document_type_name.as_bytes());
    buf.extend_from_slice(entropy);

    Identifier::from_bytes(&hash_double_to_vec(&buf))
        .map_err(|err| WasmDppError::invalid_argument(err.to_string()))
}

/// Macro to implement `try_from_options` helper method for extracting a WASM type from an options object.
///
/// This generates methods that read a named field from a JsValue options object and convert it
/// to the WASM wrapper type using the type's `TryFrom<&JsValue>` implementation.
///
/// The type must implement `TryFrom<&JsValue, Error = WasmDppError>`.
///
/// # Usage
///
/// ```ignore
/// // Basic form: requires field_name parameter
/// impl_try_from_options!(MyTypeWasm);
///
/// // With default field name: generates both try_from_options() and try_from_options_with_field()
/// impl_try_from_options!(MySignerWasm, "signer");
/// ```
///
/// The basic form generates:
/// ```ignore
/// impl MyTypeWasm {
///     pub fn try_from_options(options: &JsValue, field_name: &str) -> WasmDppResult<Self> { ... }
///     pub fn try_from_optional_options(options: &JsValue, field_name: &str) -> WasmDppResult<Option<Self>> { ... }
/// }
/// ```
///
/// The form with default field name generates:
/// ```ignore
/// impl MySignerWasm {
///     pub fn try_from_options(options: &JsValue) -> WasmDppResult<Self> { ... }
///     pub fn try_from_options_with_field(options: &JsValue, field_name: &str) -> WasmDppResult<Self> { ... }
///     pub fn try_from_optional_options(options: &JsValue) -> WasmDppResult<Option<Self>> { ... }
///     pub fn try_from_optional_options_with_field(options: &JsValue, field_name: &str) -> WasmDppResult<Option<Self>> { ... }
/// }
/// ```
#[macro_export]
macro_rules! impl_try_from_options {
    // Basic form: requires field_name parameter
    ($wrapper:ty) => {
        impl $wrapper {
            /// Try to extract this type from an options object field.
            ///
            /// This helper reads the specified field from an options object and converts it
            /// using the type's TryFrom implementation.
            pub fn try_from_options(
                options: &wasm_bindgen::JsValue,
                field_name: &str,
            ) -> $crate::error::WasmDppResult<Self> {
                let value_js =
                    js_sys::Reflect::get(options, &wasm_bindgen::JsValue::from_str(field_name))
                        .map_err(|_| {
                            $crate::error::WasmDppError::invalid_argument(format!(
                                "Missing '{}' field",
                                field_name
                            ))
                        })?;

                if value_js.is_undefined() || value_js.is_null() {
                    return Err($crate::error::WasmDppError::invalid_argument(format!(
                        "'{}' is required",
                        field_name
                    )));
                }

                Self::try_from(&value_js).map_err(Into::into)
            }

            /// Try to extract this type from an options object field, returning None if not present.
            ///
            /// Returns Ok(None) if the field is undefined or null.
            pub fn try_from_optional_options(
                options: &wasm_bindgen::JsValue,
                field_name: &str,
            ) -> $crate::error::WasmDppResult<Option<Self>> {
                let value_js =
                    js_sys::Reflect::get(options, &wasm_bindgen::JsValue::from_str(field_name))
                        .unwrap_or(wasm_bindgen::JsValue::UNDEFINED);

                if value_js.is_undefined() || value_js.is_null() {
                    return Ok(None);
                }

                Self::try_from(&value_js).map(Some).map_err(Into::into)
            }
        }
    };

    // Form with default field name: generates try_from_options() with default and try_from_options_with_field()
    ($wrapper:ty, $default_field:expr) => {
        impl $wrapper {
            /// Try to extract this type from an options object using the default field name.
            ///
            /// This helper reads the default field from an options object and converts it
            /// using the type's TryFrom implementation.
            pub fn try_from_options(
                options: &wasm_bindgen::JsValue,
            ) -> $crate::error::WasmDppResult<Self> {
                Self::try_from_options_with_field(options, $default_field)
            }

            /// Try to extract this type from an options object with a custom field name.
            ///
            /// This helper reads the specified field from an options object and converts it
            /// using the type's TryFrom implementation.
            pub fn try_from_options_with_field(
                options: &wasm_bindgen::JsValue,
                field_name: &str,
            ) -> $crate::error::WasmDppResult<Self> {
                let value_js =
                    js_sys::Reflect::get(options, &wasm_bindgen::JsValue::from_str(field_name))
                        .map_err(|_| {
                            $crate::error::WasmDppError::invalid_argument(format!(
                                "Missing '{}' field",
                                field_name
                            ))
                        })?;

                if value_js.is_undefined() || value_js.is_null() {
                    return Err($crate::error::WasmDppError::invalid_argument(format!(
                        "'{}' is required",
                        field_name
                    )));
                }

                Self::try_from(&value_js).map_err(Into::into)
            }

            /// Try to extract this type from an options object using the default field name,
            /// returning None if not present.
            ///
            /// Returns Ok(None) if the field is undefined or null.
            pub fn try_from_optional_options(
                options: &wasm_bindgen::JsValue,
            ) -> $crate::error::WasmDppResult<Option<Self>> {
                Self::try_from_optional_options_with_field(options, $default_field)
            }

            /// Try to extract this type from an options object with a custom field name,
            /// returning None if not present.
            ///
            /// Returns Ok(None) if the field is undefined or null.
            pub fn try_from_optional_options_with_field(
                options: &wasm_bindgen::JsValue,
                field_name: &str,
            ) -> $crate::error::WasmDppResult<Option<Self>> {
                let value_js =
                    js_sys::Reflect::get(options, &wasm_bindgen::JsValue::from_str(field_name))
                        .unwrap_or(wasm_bindgen::JsValue::UNDEFINED);

                if value_js.is_undefined() || value_js.is_null() {
                    return Ok(None);
                }

                Self::try_from(&value_js).map(Some).map_err(Into::into)
            }
        }
    };
}

/// Macro to implement `TryFrom<&JsValue>` for WASM wrapper types using `IntoWasm`.
///
/// This is for complex types that can only be instantiated from their WASM class objects
/// (not from strings, numbers, or bytes). The implementation reads the internal `__wbg_ptr`
/// from the JavaScript object.
///
/// # Usage
///
/// ```ignore
/// impl_try_from_js_value!(DocumentWasm, "Document");
/// impl_try_from_js_value!(DataContractWasm, "DataContract");
/// ```
#[macro_export]
macro_rules! impl_try_from_js_value {
    ($wrapper:ty, $type_name:expr) => {
        impl TryFrom<&wasm_bindgen::JsValue> for $wrapper {
            type Error = $crate::error::WasmDppError;

            fn try_from(value: &wasm_bindgen::JsValue) -> Result<Self, Self::Error> {
                $crate::utils::IntoWasm::to_wasm::<$wrapper>(value, $type_name)
                    .map(|boxed| (*boxed).clone())
            }
        }
    };
}

/// Macro to implement `__type` and `__struct` getters for WASM type identification.
///
/// These getters are used by `get_class_type()` and `to_wasm()` to verify that a JsValue
/// is the expected WASM class before extracting its internal pointer. This is necessary
/// because wasm-bindgen doesn't support `instanceof` or `JsCast` for exported structs.
///
/// Using explicit `__type` properties instead of `constructor.name` ensures the type
/// identification works correctly even when the code is bundled and minified by consumers.
///
/// # Usage
///
/// ```ignore
/// impl_wasm_type_info!(IdentifierWasm, Identifier);
/// impl_wasm_type_info!(DataContractWasm, DataContract);
/// ```
///
/// This generates:
/// ```ignore
/// #[wasm_bindgen(js_class = Identifier)]
/// impl IdentifierWasm {
///     #[wasm_bindgen(getter = __type)]
///     pub fn type_name(&self) -> String { "Identifier".to_string() }
///
///     #[wasm_bindgen(getter = __struct)]
///     pub fn struct_name() -> String { "Identifier".to_string() }
/// }
/// ```
#[macro_export]
macro_rules! impl_wasm_type_info {
    ($wrapper:ty, $js_class:ident) => {
        #[wasm_bindgen::prelude::wasm_bindgen(js_class = $js_class)]
        impl $wrapper {
            #[wasm_bindgen::prelude::wasm_bindgen(getter = __type)]
            pub fn type_name(&self) -> String {
                stringify!($js_class).to_string()
            }

            #[wasm_bindgen::prelude::wasm_bindgen(getter = __struct)]
            pub fn struct_name() -> String {
                stringify!($js_class).to_string()
            }
        }
    };
}

/// Macro to implement `From` traits for wasm-bindgen extern types.
///
/// This macro helps convert Rust/WASM types to JavaScript extern types,
/// which are typically used for union type returns in wasm-bindgen.
///
/// # Usage
///
/// Basic form - implements `From<JsValue>` for the extern type:
/// ```ignore
/// impl_from_for_extern_type!(MyExternTypeJs);
/// ```
///
/// With source types - implements `From` for each source type:
/// ```ignore
/// impl_from_for_extern_type!(MyExternTypeJs, MyWasmType1, MyWasmType2);
/// ```
///
/// Combined form - implements both `From<JsValue>` and `From` for source types:
/// ```ignore
/// impl_from_for_extern_type!(MyExternTypeJs; MyWasmType1, MyWasmType2);
/// ```
#[macro_export]
macro_rules! impl_from_for_extern_type {
    // Just the extern type - implements From<JsValue>
    ($extern_type:ty) => {
        impl From<wasm_bindgen::JsValue> for $extern_type {
            fn from(value: wasm_bindgen::JsValue) -> Self {
                wasm_bindgen::JsCast::unchecked_into(value)
            }
        }
    };

    // Extern type with source types (comma-separated) - implements From for each source only
    ($extern_type:ty, $($source_type:ty),+ $(,)?) => {
        $(
            impl From<$source_type> for $extern_type {
                fn from(value: $source_type) -> Self {
                    let js_value: wasm_bindgen::JsValue = value.into();
                    wasm_bindgen::JsCast::unchecked_into(js_value)
                }
            }
        )+
    };

    // Combined form (semicolon-separated) - implements From<JsValue> AND From for source types
    ($extern_type:ty; $($source_type:ty),+ $(,)?) => {
        impl From<wasm_bindgen::JsValue> for $extern_type {
            fn from(value: wasm_bindgen::JsValue) -> Self {
                wasm_bindgen::JsCast::unchecked_into(value)
            }
        }

        $(
            impl From<$source_type> for $extern_type {
                fn from(value: $source_type) -> Self {
                    let js_value: wasm_bindgen::JsValue = value.into();
                    wasm_bindgen::JsCast::unchecked_into(js_value)
                }
            }
        )+
    };
}
