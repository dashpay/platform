use dpp::withdrawal::Pooling;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen]
pub enum PoolingWASM {
    Never = 0,
    IfAvailable = 1,
    Standard = 2,
}

impl From<PoolingWASM> for Pooling {
    fn from(pooling: PoolingWASM) -> Self {
        match pooling {
            PoolingWASM::Never => Pooling::Never,
            PoolingWASM::IfAvailable => Pooling::IfAvailable,
            PoolingWASM::Standard => Pooling::Standard,
        }
    }
}

impl From<Pooling> for PoolingWASM {
    fn from(pooling: Pooling) -> Self {
        match pooling {
            Pooling::Never => PoolingWASM::Never,
            Pooling::IfAvailable => PoolingWASM::IfAvailable,
            Pooling::Standard => PoolingWASM::Standard,
        }
    }
}

impl TryFrom<JsValue> for PoolingWASM {
    type Error = JsValue;
    fn try_from(value: JsValue) -> Result<Self, Self::Error> {
        match value.is_string() {
            true => match value.as_string() {
                None => Err(JsValue::from("cannot read value from enum")),
                Some(enum_val) => match enum_val.to_lowercase().as_str() {
                    "never" => Ok(PoolingWASM::Never),
                    "ifavailable" => Ok(PoolingWASM::IfAvailable),
                    "standard" => Ok(PoolingWASM::Standard),
                    _ => Err(JsValue::from(format!(
                        "unsupported pooling value ({})",
                        enum_val
                    ))),
                },
            },
            false => match value.as_f64() {
                None => Err(JsValue::from("cannot read value from enum")),
                Some(enum_val) => match enum_val as u8 {
                    0 => Ok(PoolingWASM::Never),
                    1 => Ok(PoolingWASM::IfAvailable),
                    2 => Ok(PoolingWASM::Standard),
                    _ => Err(JsValue::from(format!(
                        "unsupported pooling value ({})",
                        enum_val
                    ))),
                },
            },
        }
    }
}

impl From<PoolingWASM> for String {
    fn from(pooling_wasm: PoolingWASM) -> String {
        match pooling_wasm {
            PoolingWASM::Never => String::from("Never"),
            PoolingWASM::IfAvailable => String::from("IfAvailable"),
            PoolingWASM::Standard => String::from("Standard"),
        }
    }
}
