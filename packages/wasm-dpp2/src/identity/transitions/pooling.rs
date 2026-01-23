use crate::error::WasmDppError;
use dpp::withdrawal::Pooling;
use serde::{Deserialize, Deserializer};
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(typescript_custom_section)]
const TS_TYPES: &str = r#"
/**
 * Flexible input type for Pooling - accepts the enum, string name, or numeric value.
 */
export type CreditWithdrawalTransitionPoolingLike = Pooling | "never" | "ifavailable" | "standard" | 0 | 1 | 2;
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "CreditWithdrawalTransitionPoolingLike")]
    pub type PoolingLikeJs;
}

impl TryFrom<PoolingLikeJs> for PoolingWasm {
    type Error = WasmDppError;

    fn try_from(value: PoolingLikeJs) -> Result<Self, Self::Error> {
        let js_value: JsValue = value.into();
        PoolingWasm::try_from(js_value)
    }
}

impl TryFrom<PoolingLikeJs> for Pooling {
    type Error = WasmDppError;

    fn try_from(value: PoolingLikeJs) -> Result<Self, Self::Error> {
        let wasm: PoolingWasm = value.try_into()?;
        Ok(Pooling::from(wasm))
    }
}

#[wasm_bindgen]
#[derive(Clone, Copy)]
pub enum PoolingWasm {
    Never = 0,
    IfAvailable = 1,
    Standard = 2,
}

impl<'de> Deserialize<'de> for PoolingWasm {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value: serde_json::Value = Deserialize::deserialize(deserializer)?;

        // Try as string first
        if let Some(s) = value.as_str() {
            return match s.to_lowercase().as_str() {
                "never" => Ok(PoolingWasm::Never),
                "ifavailable" => Ok(PoolingWasm::IfAvailable),
                "standard" => Ok(PoolingWasm::Standard),
                _ => Err(serde::de::Error::custom(format!(
                    "unsupported pooling value ({})",
                    s
                ))),
            };
        }

        // Try as number
        if let Some(n) = value.as_u64() {
            return match n {
                0 => Ok(PoolingWasm::Never),
                1 => Ok(PoolingWasm::IfAvailable),
                2 => Ok(PoolingWasm::Standard),
                _ => Err(serde::de::Error::custom(format!(
                    "unsupported pooling value ({})",
                    n
                ))),
            };
        }

        Err(serde::de::Error::custom(
            "pooling must be a string or number",
        ))
    }
}

impl From<PoolingWasm> for Pooling {
    fn from(pooling: PoolingWasm) -> Self {
        match pooling {
            PoolingWasm::Never => Pooling::Never,
            PoolingWasm::IfAvailable => Pooling::IfAvailable,
            PoolingWasm::Standard => Pooling::Standard,
        }
    }
}

impl From<Pooling> for PoolingWasm {
    fn from(pooling: Pooling) -> Self {
        match pooling {
            Pooling::Never => PoolingWasm::Never,
            Pooling::IfAvailable => PoolingWasm::IfAvailable,
            Pooling::Standard => PoolingWasm::Standard,
        }
    }
}

impl TryFrom<&JsValue> for PoolingWasm {
    type Error = WasmDppError;

    fn try_from(value: &JsValue) -> Result<Self, Self::Error> {
        if value.is_string() {
            match value.as_string() {
                None => Err(WasmDppError::invalid_argument(
                    "cannot read value from enum",
                )),
                Some(enum_val) => match enum_val.to_lowercase().as_str() {
                    "never" => Ok(PoolingWasm::Never),
                    "ifavailable" => Ok(PoolingWasm::IfAvailable),
                    "standard" => Ok(PoolingWasm::Standard),
                    _ => Err(WasmDppError::invalid_argument(format!(
                        "unsupported pooling value ({})",
                        enum_val
                    ))),
                },
            }
        } else {
            match value.as_f64() {
                None => Err(WasmDppError::invalid_argument(
                    "cannot read value from enum",
                )),
                Some(enum_val) => match enum_val as u8 {
                    0 => Ok(PoolingWasm::Never),
                    1 => Ok(PoolingWasm::IfAvailable),
                    2 => Ok(PoolingWasm::Standard),
                    _ => Err(WasmDppError::invalid_argument(format!(
                        "unsupported pooling value ({})",
                        enum_val
                    ))),
                },
            }
        }
    }
}

impl TryFrom<JsValue> for PoolingWasm {
    type Error = WasmDppError;

    fn try_from(value: JsValue) -> Result<Self, Self::Error> {
        Self::try_from(&value)
    }
}

impl From<PoolingWasm> for String {
    fn from(pooling_wasm: PoolingWasm) -> String {
        match pooling_wasm {
            PoolingWasm::Never => String::from("Never"),
            PoolingWasm::IfAvailable => String::from("IfAvailable"),
            PoolingWasm::Standard => String::from("Standard"),
        }
    }
}
