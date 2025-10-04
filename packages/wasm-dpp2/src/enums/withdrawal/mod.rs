use dpp::withdrawal::Pooling;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen]
pub enum PoolingWasm {
    Never = 0,
    IfAvailable = 1,
    Standard = 2,
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

impl TryFrom<JsValue> for PoolingWasm {
    type Error = JsValue;
    fn try_from(value: JsValue) -> Result<Self, Self::Error> {
        match value.is_string() {
            true => match value.as_string() {
                None => Err(JsValue::from("cannot read value from enum")),
                Some(enum_val) => match enum_val.to_lowercase().as_str() {
                    "never" => Ok(PoolingWasm::Never),
                    "ifavailable" => Ok(PoolingWasm::IfAvailable),
                    "standard" => Ok(PoolingWasm::Standard),
                    _ => Err(JsValue::from(format!(
                        "unsupported pooling value ({})",
                        enum_val
                    ))),
                },
            },
            false => match value.as_f64() {
                None => Err(JsValue::from("cannot read value from enum")),
                Some(enum_val) => match enum_val as u8 {
                    0 => Ok(PoolingWasm::Never),
                    1 => Ok(PoolingWasm::IfAvailable),
                    2 => Ok(PoolingWasm::Standard),
                    _ => Err(JsValue::from(format!(
                        "unsupported pooling value ({})",
                        enum_val
                    ))),
                },
            },
        }
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
