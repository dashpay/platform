use crate::error::WasmDppError;
use crate::impl_wasm_type_info;
use crate::utils::{IntoWasm, try_to_u32};
use dpp::version::PlatformVersion;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(typescript_custom_section)]
const PLATFORM_VERSION_LIKE_TS: &str = r#"
export type PlatformVersionLike = PlatformVersion | number;
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "PlatformVersionLike")]
    pub type PlatformVersionLikeJs;
}

impl TryFrom<PlatformVersionLikeJs> for PlatformVersionWasm {
    type Error = WasmDppError;
    fn try_from(value: PlatformVersionLikeJs) -> Result<Self, Self::Error> {
        let js_value: JsValue = value.into();
        PlatformVersionWasm::try_from(js_value)
    }
}

impl TryFrom<PlatformVersionLikeJs> for PlatformVersion {
    type Error = WasmDppError;
    fn try_from(value: PlatformVersionLikeJs) -> Result<Self, Self::Error> {
        let wasm: PlatformVersionWasm = value.try_into()?;
        Ok(PlatformVersion::from(wasm))
    }
}

#[wasm_bindgen(js_name = "PlatformVersion")]
#[derive(Clone)]
pub struct PlatformVersionWasm {
    version: u32,
}

impl_wasm_type_info!(PlatformVersionWasm, PlatformVersion);

#[wasm_bindgen(js_class = "PlatformVersion")]
impl PlatformVersionWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(version: u32) -> Result<PlatformVersionWasm, WasmDppError> {
        PlatformVersion::get(version).map_err(|_| {
            WasmDppError::invalid_argument(format!("unknown platform version: {}", version))
        })?;
        Ok(PlatformVersionWasm { version })
    }

    #[wasm_bindgen(js_name = "latest")]
    pub fn latest() -> PlatformVersionWasm {
        PlatformVersionWasm {
            version: PlatformVersion::latest().protocol_version,
        }
    }

    #[wasm_bindgen(js_name = "first")]
    pub fn first() -> PlatformVersionWasm {
        PlatformVersionWasm {
            version: PlatformVersion::first().protocol_version,
        }
    }

    #[wasm_bindgen(js_name = "current")]
    pub fn current() -> PlatformVersionWasm {
        PlatformVersionWasm {
            version: PlatformVersion::desired().protocol_version,
        }
    }

    #[wasm_bindgen(getter)]
    pub fn version(&self) -> u32 {
        self.version
    }
}

impl Default for PlatformVersionWasm {
    fn default() -> Self {
        PlatformVersionWasm {
            version: PlatformVersion::desired().protocol_version,
        }
    }
}

impl TryFrom<&JsValue> for PlatformVersionWasm {
    type Error = WasmDppError;

    fn try_from(value: &JsValue) -> Result<Self, Self::Error> {
        // Return default for undefined/null
        if value.is_undefined() || value.is_null() {
            return Ok(Self::default());
        }

        // Detect PlatformVersionWasm instance via to_wasm
        if let Ok(wasm_ref) = value.to_wasm::<PlatformVersionWasm>("PlatformVersion") {
            return Ok(wasm_ref.clone());
        }

        // Otherwise treat as raw number
        let version = try_to_u32(value, "platformVersion")?;

        PlatformVersion::get(version).map_err(|_| {
            WasmDppError::invalid_argument(format!("unknown platform version: {}", version))
        })?;

        Ok(PlatformVersionWasm { version })
    }
}

impl TryFrom<JsValue> for PlatformVersionWasm {
    type Error = WasmDppError;

    fn try_from(value: JsValue) -> Result<Self, Self::Error> {
        Self::try_from(&value)
    }
}

impl From<PlatformVersionWasm> for PlatformVersion {
    fn from(value: PlatformVersionWasm) -> Self {
        PlatformVersion::get(value.version)
            .expect("PlatformVersionWasm should always contain a valid version")
            .clone()
    }
}
