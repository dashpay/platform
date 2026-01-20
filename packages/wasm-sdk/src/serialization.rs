// Re-export the macro from wasm-dpp2 for newtype wrappers
pub use wasm_dpp2::impl_wasm_conversions;

/// Macro to implement `toObject`, `fromObject`, `toJSON`, and `fromJSON` methods
/// for a wasm_bindgen type using serde_format.
///
/// This macro is for types that directly implement Serialize/Deserialize.
/// For newtype wrappers (e.g., `struct Foo(Inner)`), use `impl_wasm_conversions!` instead.
///
/// # Usage
///
/// ```ignore
/// // Single-argument: uses Rust type name as JS class
/// impl_wasm_serde_conversions!(MyTypeWasm);
///
/// // Two-argument: specify JS class name
/// impl_wasm_serde_conversions!(MyTypeWasm, MyType);
/// ```
#[macro_export]
macro_rules! impl_wasm_serde_conversions {
    // Single-argument form: Rust type name equals JS class name
    ($ty:ty) => {
        #[wasm_bindgen::prelude::wasm_bindgen]
        impl $ty {
            #[wasm_bindgen::prelude::wasm_bindgen(js_name = toObject)]
            pub fn to_object(&self) -> Result<wasm_bindgen::JsValue, $crate::WasmSdkError> {
                wasm_dpp2::serialization::to_object(self).map_err($crate::WasmSdkError::from)
            }

            #[wasm_bindgen::prelude::wasm_bindgen(js_name = fromObject)]
            pub fn from_object(obj: js_sys::Object) -> Result<$ty, $crate::WasmSdkError> {
                wasm_dpp2::serialization::from_object(obj.into()).map_err($crate::WasmSdkError::from)
            }

            #[wasm_bindgen::prelude::wasm_bindgen(js_name = toJSON)]
            pub fn to_json(&self) -> Result<wasm_bindgen::JsValue, $crate::WasmSdkError> {
                wasm_dpp2::serialization::to_json(self).map_err($crate::WasmSdkError::from)
            }

            #[wasm_bindgen::prelude::wasm_bindgen(js_name = fromJSON)]
            pub fn from_json(js: js_sys::Object) -> Result<$ty, $crate::WasmSdkError> {
                wasm_dpp2::serialization::from_json(js.into()).map_err($crate::WasmSdkError::from)
            }
        }
    };
    // Two-argument form: Rust type and JS class name
    ($ty:ty, $js_class:ident) => {
        #[wasm_bindgen::prelude::wasm_bindgen(js_class = $js_class)]
        impl $ty {
            #[wasm_bindgen::prelude::wasm_bindgen(js_name = toObject)]
            pub fn to_object(&self) -> Result<wasm_bindgen::JsValue, $crate::WasmSdkError> {
                wasm_dpp2::serialization::to_object(self).map_err($crate::WasmSdkError::from)
            }

            #[wasm_bindgen::prelude::wasm_bindgen(js_name = fromObject)]
            pub fn from_object(obj: js_sys::Object) -> Result<$ty, $crate::WasmSdkError> {
                wasm_dpp2::serialization::from_object(obj.into()).map_err($crate::WasmSdkError::from)
            }

            #[wasm_bindgen::prelude::wasm_bindgen(js_name = toJSON)]
            pub fn to_json(&self) -> Result<wasm_bindgen::JsValue, $crate::WasmSdkError> {
                wasm_dpp2::serialization::to_json(self).map_err($crate::WasmSdkError::from)
            }

            #[wasm_bindgen::prelude::wasm_bindgen(js_name = fromJSON)]
            pub fn from_json(js: js_sys::Object) -> Result<$ty, $crate::WasmSdkError> {
                wasm_dpp2::serialization::from_json(js.into()).map_err($crate::WasmSdkError::from)
            }
        }
    };
}
