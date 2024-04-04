#[macro_export]
macro_rules! generic_consensus_error {
    ($error_type:ident, $error_instance:expr) => {
        {
            use wasm_bindgen::prelude::wasm_bindgen;
            use dpp::consensus::ConsensusError;
            use dpp::consensus::codes::ErrorWithCode;
            use paste::paste;

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
                }

                [<$error_type Wasm>]::from($error_instance)
            }
        }
    }
}
