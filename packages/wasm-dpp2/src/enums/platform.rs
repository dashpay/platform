use crate::error::WasmDppError;
use dpp::version::PlatformVersion;
use dpp::version::v1::PLATFORM_V1;
use dpp::version::v2::PLATFORM_V2;
use dpp::version::v3::PLATFORM_V3;
use dpp::version::v4::PLATFORM_V4;
use dpp::version::v5::PLATFORM_V5;
use dpp::version::v6::PLATFORM_V6;
use dpp::version::v7::PLATFORM_V7;
use dpp::version::v8::PLATFORM_V8;
use dpp::version::v9::PLATFORM_V9;
use dpp::version::v10::PLATFORM_V10;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(js_name = "PlatformVersion")]
#[derive(Default)]
#[allow(non_camel_case_types)]
pub enum PlatformVersionWasm {
    #[default]
    PLATFORM_V1 = 1,
    PLATFORM_V2 = 2,
    PLATFORM_V3 = 3,
    PLATFORM_V4 = 4,
    PLATFORM_V5 = 5,
    PLATFORM_V6 = 6,
    PLATFORM_V7 = 7,
    PLATFORM_V8 = 8,
    PLATFORM_V9 = 9,
    PLATFORM_V10 = 10,
}

impl TryFrom<JsValue> for PlatformVersionWasm {
    type Error = WasmDppError;
    fn try_from(value: JsValue) -> Result<PlatformVersionWasm, Self::Error> {
        match value.is_string() {
            true => match value.as_string() {
                None => Err(WasmDppError::invalid_argument(
                    "cannot read value from enum",
                )),
                Some(enum_val) => match enum_val.to_lowercase().as_str() {
                    "platform_v1" => Ok(PlatformVersionWasm::PLATFORM_V1),
                    "platform_v2" => Ok(PlatformVersionWasm::PLATFORM_V2),
                    "platform_v3" => Ok(PlatformVersionWasm::PLATFORM_V3),
                    "platform_v4" => Ok(PlatformVersionWasm::PLATFORM_V4),
                    "platform_v5" => Ok(PlatformVersionWasm::PLATFORM_V5),
                    "platform_v6" => Ok(PlatformVersionWasm::PLATFORM_V6),
                    "platform_v7" => Ok(PlatformVersionWasm::PLATFORM_V7),
                    "platform_v8" => Ok(PlatformVersionWasm::PLATFORM_V8),
                    "platform_v9" => Ok(PlatformVersionWasm::PLATFORM_V9),
                    "platform_v10" => Ok(PlatformVersionWasm::PLATFORM_V10),
                    _ => Err(WasmDppError::invalid_argument(format!(
                        "unknown platform version value: {}",
                        enum_val
                    ))),
                },
            },
            false => match value.as_f64() {
                None => Err(WasmDppError::invalid_argument(
                    "cannot read value from enum",
                )),
                Some(enum_val) => match enum_val as u8 {
                    1 => Ok(PlatformVersionWasm::PLATFORM_V1),
                    2 => Ok(PlatformVersionWasm::PLATFORM_V2),
                    3 => Ok(PlatformVersionWasm::PLATFORM_V3),
                    4 => Ok(PlatformVersionWasm::PLATFORM_V4),
                    5 => Ok(PlatformVersionWasm::PLATFORM_V5),
                    6 => Ok(PlatformVersionWasm::PLATFORM_V6),
                    7 => Ok(PlatformVersionWasm::PLATFORM_V7),
                    8 => Ok(PlatformVersionWasm::PLATFORM_V8),
                    9 => Ok(PlatformVersionWasm::PLATFORM_V9),
                    10 => Ok(PlatformVersionWasm::PLATFORM_V10),
                    _ => Err(WasmDppError::invalid_argument(format!(
                        "unknown platform version value: {}",
                        enum_val
                    ))),
                },
            },
        }
    }
}

impl From<PlatformVersionWasm> for String {
    fn from(version: PlatformVersionWasm) -> String {
        match version {
            PlatformVersionWasm::PLATFORM_V1 => String::from("PLATFORM_V1"),
            PlatformVersionWasm::PLATFORM_V2 => String::from("PLATFORM_V2"),
            PlatformVersionWasm::PLATFORM_V3 => String::from("PLATFORM_V3"),
            PlatformVersionWasm::PLATFORM_V4 => String::from("PLATFORM_V4"),
            PlatformVersionWasm::PLATFORM_V5 => String::from("PLATFORM_V5"),
            PlatformVersionWasm::PLATFORM_V6 => String::from("PLATFORM_V6"),
            PlatformVersionWasm::PLATFORM_V7 => String::from("PLATFORM_V7"),
            PlatformVersionWasm::PLATFORM_V8 => String::from("PLATFORM_V8"),
            PlatformVersionWasm::PLATFORM_V9 => String::from("PLATFORM_V9"),
            PlatformVersionWasm::PLATFORM_V10 => String::from("PLATFORM_V10"),
        }
    }
}

impl From<PlatformVersionWasm> for PlatformVersion {
    fn from(value: PlatformVersionWasm) -> Self {
        match value {
            PlatformVersionWasm::PLATFORM_V1 => PLATFORM_V1,
            PlatformVersionWasm::PLATFORM_V2 => PLATFORM_V2,
            PlatformVersionWasm::PLATFORM_V3 => PLATFORM_V3,
            PlatformVersionWasm::PLATFORM_V4 => PLATFORM_V4,
            PlatformVersionWasm::PLATFORM_V5 => PLATFORM_V5,
            PlatformVersionWasm::PLATFORM_V6 => PLATFORM_V6,
            PlatformVersionWasm::PLATFORM_V7 => PLATFORM_V7,
            PlatformVersionWasm::PLATFORM_V8 => PLATFORM_V8,
            PlatformVersionWasm::PLATFORM_V9 => PLATFORM_V9,
            PlatformVersionWasm::PLATFORM_V10 => PLATFORM_V10,
        }
    }
}
