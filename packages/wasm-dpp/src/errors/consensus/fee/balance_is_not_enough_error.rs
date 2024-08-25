use crate::buffer::Buffer;
use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::fee::balance_is_not_enough_error::BalanceIsNotEnoughError;
use dpp::consensus::ConsensusError;
use dpp::fee::Credits;

use dpp::serialization::PlatformSerializableWithPlatformVersion;
use dpp::version::PlatformVersion;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=BalanceIsNotEnoughError)]
pub struct BalanceIsNotEnoughErrorWasm {
    inner: BalanceIsNotEnoughError,
}

impl From<&BalanceIsNotEnoughError> for BalanceIsNotEnoughErrorWasm {
    fn from(e: &BalanceIsNotEnoughError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=BalanceIsNotEnoughError)]
impl BalanceIsNotEnoughErrorWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(balance: Credits, fee: Credits) -> Self {
        Self {
            inner: BalanceIsNotEnoughError::new(balance, fee),
        }
    }

    #[wasm_bindgen(js_name=getBalance)]
    pub fn get_balance(&self) -> f64 {
        self.inner.balance() as f64
    }

    #[wasm_bindgen(js_name=getFee)]
    pub fn get_fee(&self) -> f64 {
        self.inner.fee() as f64
    }

    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        ConsensusError::from(self.inner.clone()).code()
    }

    #[wasm_bindgen(getter)]
    pub fn message(&self) -> String {
        self.inner.to_string()
    }

    #[wasm_bindgen(js_name=serialize)]
    pub fn serialize(&self) -> Result<Buffer, JsError> {
        let bytes = ConsensusError::from(self.inner.clone())
            .serialize_to_bytes_with_platform_version(PlatformVersion::first())
            .map_err(JsError::from)?;

        Ok(Buffer::from_bytes(bytes.as_slice()))
    }
}
