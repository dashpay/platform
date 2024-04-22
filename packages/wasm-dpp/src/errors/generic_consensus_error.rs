#[macro_export]
macro_rules! generic_consensus_error {
    ($error_type:ident, $error_instance:expr) => {{
        use {
            dpp::{
                consensus::{codes::ErrorWithCode, ConsensusError},
                serialization::PlatformSerializableWithPlatformVersion,
                version::PlatformVersion,
            },
            paste::paste,
            wasm_bindgen::prelude::wasm_bindgen,
            $crate::buffer::Buffer,
        };

        paste! {
            #[derive(Debug)]
            #[wasm_bindgen(js_name=$error_type)]
            pub struct [<$error_type Wasm>] {
                inner: $error_type
            }

            impl From<&$error_type> for [<$error_type Wasm>] {
                fn from(e: &$error_type) -> Self {
                    Self {
                        inner: e.clone()
                    }
                }
            }

            #[wasm_bindgen(js_class=$error_type)]
            impl [<$error_type Wasm>] {
                #[wasm_bindgen(js_name=getCode)]
                pub fn get_code(&self) -> u32 {
                    ConsensusError::from(self.inner.clone()).code()
                }

                #[wasm_bindgen(getter)]
                pub fn message(&self) -> String {
                    self.inner.to_string()
                }

                pub fn serialize(&self) -> Result<Buffer, JsError> {
                    let bytes = ConsensusError::from(self.inner.clone())
                        .serialize_to_bytes_with_platform_version(PlatformVersion::first())
                        .map_err(JsError::from)?;

                    Ok(Buffer::from_bytes(bytes.as_slice()))
                }
            }

            [<$error_type Wasm>]::from($error_instance)
        }
    }};
}
