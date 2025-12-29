use crate::error::{WasmDppError, WasmDppResult};
use crate::utils::IntoWasm;
use dpp::address_funds::PlatformAddress;
use dpp::dashcore::Network;
use js_sys::Uint8Array;
use serde::de::{self, Error, Visitor};
use serde::{Deserialize, Deserializer};
use std::fmt;
use wasm_bindgen::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
#[wasm_bindgen(js_name = "PlatformAddress")]
pub struct PlatformAddressWasm(PlatformAddress);

#[wasm_bindgen(typescript_custom_section)]
const PLATFORM_ADDRESS_TS_HELPERS: &'static str = r#"
/**
 * A Platform address can be provided as:
 * - A PlatformAddress object
 * - A Uint8Array (21 bytes: type byte + 20-byte hash)
 * - A bech32m string (e.g., "dashevo1..." or "tdashevo1...")
 */
export type PlatformAddressLike = PlatformAddress | Uint8Array | string;
"#;

impl From<PlatformAddressWasm> for PlatformAddress {
    fn from(address: PlatformAddressWasm) -> Self {
        address.0
    }
}

impl From<PlatformAddress> for PlatformAddressWasm {
    fn from(address: PlatformAddress) -> Self {
        PlatformAddressWasm(address)
    }
}

impl From<&PlatformAddressWasm> for PlatformAddress {
    fn from(address: &PlatformAddressWasm) -> Self {
        address.0
    }
}

impl TryFrom<&[u8]> for PlatformAddressWasm {
    type Error = WasmDppError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        PlatformAddress::from_bytes(value)
            .map(PlatformAddressWasm)
            .map_err(|e| WasmDppError::invalid_argument(e.to_string()))
    }
}

impl TryFrom<JsValue> for PlatformAddressWasm {
    type Error = WasmDppError;

    fn try_from(value: JsValue) -> Result<Self, Self::Error> {
        if value.is_undefined() || value.is_null() {
            return Err(WasmDppError::invalid_argument(
                "the platform address cannot be null or undefined",
            ));
        }

        // Check if it's already a PlatformAddressWasm
        if let Ok(existing) = value
            .clone()
            .to_wasm::<PlatformAddressWasm>("PlatformAddress")
        {
            return Ok(*existing);
        }

        // Try parsing as string (bech32m)
        if let Some(string) = value.as_string() {
            return PlatformAddressWasm::try_from(string.as_str());
        }

        // Try parsing as bytes
        if value.is_instance_of::<js_sys::Uint8Array>() || value.is_array() {
            let uint8_array = Uint8Array::from(value.clone());
            let bytes = uint8_array.to_vec();

            return PlatformAddress::from_bytes(&bytes)
                .map(PlatformAddressWasm)
                .map_err(|e| WasmDppError::invalid_argument(e.to_string()));
        }

        Err(WasmDppError::invalid_argument(
            "Invalid platform address. Expected PlatformAddress, Uint8Array, or bech32m string",
        ))
    }
}

impl TryFrom<&JsValue> for PlatformAddressWasm {
    type Error = WasmDppError;

    fn try_from(value: &JsValue) -> Result<Self, Self::Error> {
        PlatformAddressWasm::try_from(value.clone())
    }
}

impl TryFrom<&str> for PlatformAddressWasm {
    type Error = WasmDppError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        // Try parsing as bech32m string
        PlatformAddress::from_bech32m_string(value)
            .map(|(addr, _network)| PlatformAddressWasm(addr))
            .map_err(|e| WasmDppError::invalid_argument(e.to_string()))
    }
}

struct PlatformAddressWasmVisitor;

impl<'de> Visitor<'de> for PlatformAddressWasmVisitor {
    type Value = PlatformAddressWasm;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("PlatformAddress or compatible representation")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        PlatformAddressWasm::try_from(value).map_err(|err| E::custom(err.to_string()))
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visit_str(&value)
    }

    fn visit_bytes<E>(self, value: &[u8]) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        PlatformAddress::from_bytes(value)
            .map(PlatformAddressWasm)
            .map_err(|e| E::custom(e.to_string()))
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: de::SeqAccess<'de>,
    {
        let mut bytes: Vec<u8> = Vec::new();
        while let Some(byte) = seq.next_element::<u8>()? {
            bytes.push(byte);
        }
        PlatformAddress::from_bytes(&bytes)
            .map(PlatformAddressWasm)
            .map_err(|e| A::Error::custom(e.to_string()))
    }
}

impl<'de> Deserialize<'de> for PlatformAddressWasm {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(PlatformAddressWasmVisitor)
    }
}

fn parse_network(network: &str) -> Result<Network, WasmDppError> {
    match network.to_lowercase().as_str() {
        "mainnet" | "dash" => Ok(Network::Dash),
        "testnet" => Ok(Network::Testnet),
        "devnet" => Ok(Network::Devnet),
        "regtest" => Ok(Network::Regtest),
        _ => Err(WasmDppError::invalid_argument(format!(
            "Invalid network: {}. Expected 'mainnet', 'testnet', 'devnet', or 'regtest'",
            network
        ))),
    }
}

#[wasm_bindgen(js_class = PlatformAddress)]
impl PlatformAddressWasm {
    #[wasm_bindgen(getter = __type)]
    pub fn type_name(&self) -> String {
        "PlatformAddress".to_string()
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name() -> String {
        "PlatformAddress".to_string()
    }

    /// Creates a new PlatformAddress from various input types.
    ///
    /// Accepts:
    /// - A bech32m string (e.g., "dashevo1..." or "tdashevo1...")
    /// - A Uint8Array (21 bytes: type byte + 20-byte hash)
    /// - An existing PlatformAddress object
    #[wasm_bindgen(constructor)]
    pub fn new(
        #[wasm_bindgen(unchecked_param_type = "PlatformAddress | Uint8Array | string")]
        js_address: &JsValue,
    ) -> WasmDppResult<PlatformAddressWasm> {
        PlatformAddressWasm::try_from(js_address)
    }

    /// Returns the bech32m-encoded address string for the specified network.
    ///
    /// @param network - "mainnet", "testnet", "devnet", or "regtest"
    #[wasm_bindgen(js_name = "toBech32m")]
    pub fn to_bech32m(&self, network: &str) -> WasmDppResult<String> {
        let net = parse_network(network)?;
        Ok(self.0.to_bech32m_string(net))
    }

    /// Returns the raw bytes of the address (21 bytes: type byte + 20-byte hash).
    #[wasm_bindgen(js_name = "toBytes")]
    pub fn to_bytes(&self) -> Vec<u8> {
        self.0.to_bytes()
    }

    /// Returns the hex-encoded address bytes.
    #[wasm_bindgen(js_name = "toHex")]
    pub fn to_hex(&self) -> String {
        hex::encode(self.0.to_bytes())
    }

    /// Returns the address type: "P2PKH" or "P2SH".
    #[wasm_bindgen(js_name = "addressType", getter)]
    pub fn address_type(&self) -> String {
        match self.0 {
            PlatformAddress::P2pkh(_) => "P2PKH".to_string(),
            PlatformAddress::P2sh(_) => "P2SH".to_string(),
        }
    }

    /// Returns true if this is a P2PKH address.
    #[wasm_bindgen(js_name = "isP2pkh", getter)]
    pub fn is_p2pkh(&self) -> bool {
        self.0.is_p2pkh()
    }

    /// Returns true if this is a P2SH address.
    #[wasm_bindgen(js_name = "isP2sh", getter)]
    pub fn is_p2sh(&self) -> bool {
        self.0.is_p2sh()
    }

    /// Returns the 20-byte hash portion of the address.
    #[wasm_bindgen(js_name = "hash")]
    pub fn hash(&self) -> Vec<u8> {
        self.0.hash().to_vec()
    }

    /// Returns the hash as a hex string.
    #[wasm_bindgen(js_name = "hashToHex")]
    pub fn hash_to_hex(&self) -> String {
        hex::encode(self.0.hash())
    }

    /// Creates a PlatformAddress from a bech32m-encoded string.
    ///
    /// Accepts addresses with either mainnet ("dashevo") or testnet ("tdashevo") HRP.
    #[wasm_bindgen(js_name = "fromBech32m")]
    pub fn from_bech32m(address: &str) -> WasmDppResult<PlatformAddressWasm> {
        PlatformAddress::from_bech32m_string(address)
            .map(|(addr, _)| PlatformAddressWasm(addr))
            .map_err(|e| WasmDppError::invalid_argument(e.to_string()))
    }

    /// Creates a PlatformAddress from raw bytes (21 bytes: type byte + 20-byte hash).
    #[wasm_bindgen(js_name = "fromBytes")]
    pub fn from_bytes(bytes: Vec<u8>) -> WasmDppResult<PlatformAddressWasm> {
        PlatformAddress::from_bytes(&bytes)
            .map(PlatformAddressWasm)
            .map_err(|e| WasmDppError::invalid_argument(e.to_string()))
    }

    /// Creates a PlatformAddress from a hex-encoded string.
    #[wasm_bindgen(js_name = "fromHex")]
    pub fn from_hex(hex_string: &str) -> WasmDppResult<PlatformAddressWasm> {
        let bytes = hex::decode(hex_string)
            .map_err(|e| WasmDppError::invalid_argument(format!("Invalid hex: {}", e)))?;
        PlatformAddress::from_bytes(&bytes)
            .map(PlatformAddressWasm)
            .map_err(|e| WasmDppError::invalid_argument(e.to_string()))
    }

    /// Creates a P2PKH address from a 20-byte public key hash.
    #[wasm_bindgen(js_name = "fromP2pkhHash")]
    pub fn from_p2pkh_hash(hash: Vec<u8>) -> WasmDppResult<PlatformAddressWasm> {
        if hash.len() != 20 {
            return Err(WasmDppError::invalid_argument(format!(
                "P2PKH hash must be 20 bytes, got {}",
                hash.len()
            )));
        }
        let mut arr = [0u8; 20];
        arr.copy_from_slice(&hash);
        Ok(PlatformAddressWasm(PlatformAddress::P2pkh(arr)))
    }

    /// Creates a P2SH address from a 20-byte script hash.
    #[wasm_bindgen(js_name = "fromP2shHash")]
    pub fn from_p2sh_hash(hash: Vec<u8>) -> WasmDppResult<PlatformAddressWasm> {
        if hash.len() != 20 {
            return Err(WasmDppError::invalid_argument(format!(
                "P2SH hash must be 20 bytes, got {}",
                hash.len()
            )));
        }
        let mut arr = [0u8; 20];
        arr.copy_from_slice(&hash);
        Ok(PlatformAddressWasm(PlatformAddress::P2sh(arr)))
    }
}

impl PlatformAddressWasm {
    /// Returns the inner PlatformAddress.
    pub fn inner(&self) -> &PlatformAddress {
        &self.0
    }

    /// Consumes self and returns the inner PlatformAddress.
    pub fn into_inner(self) -> PlatformAddress {
        self.0
    }
}
