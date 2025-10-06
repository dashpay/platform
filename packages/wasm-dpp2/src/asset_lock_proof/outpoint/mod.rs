use crate::utils::IntoWasm;
use dpp::dashcore::{OutPoint, Txid};
use dpp::platform_value::string_encoding::Encoding::{Base64, Hex};
use dpp::platform_value::string_encoding::{decode, encode};
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsError, JsValue};

#[wasm_bindgen(js_name = "OutPoint")]
#[derive(Clone)]
pub struct OutPointWasm(OutPoint);

impl From<OutPoint> for OutPointWasm {
    fn from(outpoint: OutPoint) -> Self {
        OutPointWasm(outpoint)
    }
}

impl From<OutPointWasm> for OutPoint {
    fn from(outpoint: OutPointWasm) -> Self {
        outpoint.0
    }
}

impl TryFrom<JsValue> for OutPointWasm {
    type Error = JsValue;
    fn try_from(value: JsValue) -> Result<Self, Self::Error> {
        let value = value.to_wasm::<OutPointWasm>("OutPoint")?;

        Ok(value.clone())
    }
}

#[wasm_bindgen(js_class = OutPoint)]
impl OutPointWasm {
    #[wasm_bindgen(getter = __type)]
    pub fn type_name(&self) -> String {
        "OutPoint".to_string()
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name() -> String {
        "OutPoint".to_string()
    }

    #[wasm_bindgen(constructor)]
    pub fn new(txid_hex: String, vout: u32) -> Result<OutPointWasm, JsValue> {
        let out_point = Txid::from_hex(&txid_hex).map_err(|err| JsValue::from(err.to_string()))?;

        Ok(OutPointWasm(OutPoint {
            txid: out_point,
            vout,
        }))
    }

    #[wasm_bindgen(js_name = "getVOUT")]
    pub fn get_vout(&self) -> u32 {
        self.0.vout
    }

    #[wasm_bindgen(js_name = "getTXID")]
    pub fn get_tx_id(&self) -> String {
        self.0.txid.to_hex()
    }

    #[wasm_bindgen(js_name = "toBytes")]
    pub fn to_bytes(&self) -> Vec<u8> {
        let slice: [u8; 36] = self.0.into();
        slice.to_vec()
    }

    #[wasm_bindgen(js_name = "toHex")]
    pub fn to_hex(&self) -> String {
        let slice: [u8; 36] = self.0.into();

        encode(slice.as_slice(), Hex)
    }

    #[wasm_bindgen(js_name = "base64")]
    pub fn to_base64(&self) -> String {
        let slice: [u8; 36] = self.0.into();

        encode(slice.as_slice(), Base64)
    }

    #[wasm_bindgen(js_name = "fromBytes")]
    pub fn from_bytes(js_buffer: Vec<u8>) -> OutPointWasm {
        let mut buffer = [0u8; 36];
        let bytes = js_buffer.as_slice();
        let len = bytes.len();
        buffer[..len].copy_from_slice(bytes);

        OutPointWasm(OutPoint::from(buffer))
    }

    #[wasm_bindgen(js_name = "fromHex")]
    pub fn from_hex(hex: String) -> Result<OutPointWasm, JsValue> {
        Ok(OutPointWasm::from_bytes(
            decode(hex.as_str(), Hex).map_err(JsError::from)?,
        ))
    }

    #[wasm_bindgen(js_name = "fromBase64")]
    pub fn from_base64(base64: String) -> Result<OutPointWasm, JsValue> {
        Ok(OutPointWasm::from_bytes(
            decode(base64.as_str(), Base64).map_err(JsError::from)?,
        ))
    }
}

impl OutPointWasm {
    pub fn vec_from_js_value(js_outpoints: &js_sys::Array) -> Result<Vec<OutPointWasm>, JsValue> {
        let outpoints: Vec<OutPointWasm> = js_outpoints
            .iter()
            .map(|key| OutPointWasm::try_from(key))
            .collect::<Result<Vec<OutPointWasm>, JsValue>>()?;

        Ok(outpoints)
    }
}
