use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsError, JsValue};

#[wasm_bindgen(js_name = "AssetLockProofTypeWASM")]
pub enum AssetLockProofTypeWASM {
    Instant = 0,
    Chain = 1,
}

impl From<AssetLockProofTypeWASM> for String {
    fn from(value: AssetLockProofTypeWASM) -> Self {
        match value {
            AssetLockProofTypeWASM::Instant => String::from("Instant"),
            AssetLockProofTypeWASM::Chain => String::from("Chain"),
        }
    }
}

impl TryFrom<JsValue> for AssetLockProofTypeWASM {
    type Error = JsValue;

    fn try_from(value: JsValue) -> Result<Self, Self::Error> {
        match value.is_string() {
            true => match value.as_string() {
                None => Err(JsValue::from("cannot read value from enum")),
                Some(enum_val) => match enum_val.to_lowercase().as_str() {
                    "instant" => Ok(AssetLockProofTypeWASM::Instant),
                    "chain" => Ok(AssetLockProofTypeWASM::Chain),
                    _ => Err(JsValue::from(format!("unsupported lock type {}", enum_val))),
                },
            },
            false => match value.as_f64() {
                None => Err(JsValue::from("cannot read value from enum")),
                Some(enum_val) => match enum_val as u8 {
                    0 => Ok(AssetLockProofTypeWASM::Instant),
                    1 => Ok(AssetLockProofTypeWASM::Chain),
                    _ => Err(JsValue::from(format!("unsupported lock type {}", enum_val))),
                },
            },
        }
    }
}

impl TryFrom<u8> for AssetLockProofTypeWASM {
    type Error = JsError;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Instant),
            1 => Ok(Self::Chain),
            _ => Err(JsError::new("Unexpected asset lock proof type")),
        }
    }
}

impl TryFrom<u64> for AssetLockProofTypeWASM {
    type Error = JsError;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Instant),
            1 => Ok(Self::Chain),
            _ => Err(JsError::new("Unexpected asset lock proof type")),
        }
    }
}
