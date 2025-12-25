use crate::error::{WasmDppError, WasmDppResult};
use super::PlatformAddressWasm;
use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use dpp::prelude::AddressNonce;
use js_sys::BigInt;
use serde::de::{self, Deserializer, MapAccess, Visitor};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fmt;
use wasm_bindgen::prelude::*;

/// Represents an input address for address-based state transitions.
///
/// An input specifies a Platform address that will spend credits,
/// along with its current nonce and the amount to spend.
#[wasm_bindgen(js_name = "PlatformAddressInput")]
#[derive(Clone, Debug)]
pub struct PlatformAddressInputWasm {
    address: PlatformAddressWasm,
    nonce: AddressNonce,
    amount: Credits,
}

/// Helper to convert BigInt to u32
fn bigint_to_u32(value: BigInt) -> Result<u32, WasmDppError> {
    let value_u64: u64 = value
        .try_into()
        .map_err(|_| WasmDppError::invalid_argument("value must be a valid u64"))?;

    if value_u64 > u32::MAX as u64 {
        return Err(WasmDppError::invalid_argument("value exceeds u32 max"));
    }

    Ok(value_u64 as u32)
}

/// Helper to convert BigInt to u64
fn bigint_to_u64(value: BigInt) -> Result<u64, WasmDppError> {
    value
        .try_into()
        .map_err(|_| WasmDppError::invalid_argument("value must be a valid u64"))
}

#[wasm_bindgen(js_class = PlatformAddressInput)]
impl PlatformAddressInputWasm {
    /// Creates a new PlatformAddressInput.
    ///
    /// @param address - The Platform address (PlatformAddress, Uint8Array, or bech32m string)
    /// @param nonce - The current nonce of the address (will be incremented for the transaction)
    /// @param amount - The amount of credits to spend from this address
    #[wasm_bindgen(constructor)]
    pub fn new(
        #[wasm_bindgen(unchecked_param_type = "PlatformAddressLike")] address: &JsValue,
        nonce: BigInt,
        amount: BigInt,
    ) -> WasmDppResult<PlatformAddressInputWasm> {
        let platform_address = PlatformAddressWasm::try_from(address)?;
        let nonce_u32 = bigint_to_u32(nonce)?;
        let amount_u64 = bigint_to_u64(amount)?;

        Ok(PlatformAddressInputWasm {
            address: platform_address,
            nonce: nonce_u32,
            amount: amount_u64,
        })
    }

    /// Returns the Platform address.
    #[wasm_bindgen(getter)]
    pub fn address(&self) -> PlatformAddressWasm {
        self.address
    }

    /// Returns the nonce.
    #[wasm_bindgen(getter)]
    pub fn nonce(&self) -> BigInt {
        BigInt::from(self.nonce)
    }

    /// Returns the amount.
    #[wasm_bindgen(getter)]
    pub fn amount(&self) -> BigInt {
        BigInt::from(self.amount)
    }
}

impl PlatformAddressInputWasm {
    /// Returns the inner values as a tuple suitable for BTreeMap insertion.
    pub fn into_inner(self) -> (PlatformAddress, (AddressNonce, Credits)) {
        (self.address.into(), (self.nonce, self.amount))
    }

    /// Returns a reference to the inner PlatformAddress.
    pub fn platform_address(&self) -> PlatformAddress {
        self.address.into()
    }

    /// Returns the nonce value.
    pub fn nonce_value(&self) -> AddressNonce {
        self.nonce
    }

    /// Returns the amount value.
    pub fn amount_value(&self) -> Credits {
        self.amount
    }
}

/// Custom deserializer for Credits that handles number, string, or bigint.
fn deserialize_credits<'de, D>(deserializer: D) -> Result<Credits, D::Error>
where
    D: Deserializer<'de>,
{
    struct CreditsVisitor;

    impl<'de> Visitor<'de> for CreditsVisitor {
        type Value = Credits;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a number, string, or bigint representing credits")
        }

        fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(value)
        }

        fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            if value < 0 {
                Err(E::custom("credits cannot be negative"))
            } else {
                Ok(value as Credits)
            }
        }

        fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            if value < 0.0 {
                Err(E::custom("credits cannot be negative"))
            } else {
                Ok(value as Credits)
            }
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            // Handle bigint string format (e.g., "1000000n" or just "1000000")
            let clean = value.trim_end_matches('n');
            clean
                .parse::<Credits>()
                .map_err(|_| E::custom(format!("invalid credits value: {}", value)))
        }

        fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            self.visit_str(&value)
        }
    }

    deserializer.deserialize_any(CreditsVisitor)
}

/// Custom deserializer for AddressNonce that handles number or bigint.
fn deserialize_nonce<'de, D>(deserializer: D) -> Result<AddressNonce, D::Error>
where
    D: Deserializer<'de>,
{
    struct NonceVisitor;

    impl<'de> Visitor<'de> for NonceVisitor {
        type Value = AddressNonce;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a number or bigint representing a nonce")
        }

        fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            if value > u32::MAX as u64 {
                Err(E::custom("nonce exceeds u32 max"))
            } else {
                Ok(value as AddressNonce)
            }
        }

        fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            if value < 0 {
                Err(E::custom("nonce cannot be negative"))
            } else if value > u32::MAX as i64 {
                Err(E::custom("nonce exceeds u32 max"))
            } else {
                Ok(value as AddressNonce)
            }
        }

        fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            if value < 0.0 {
                Err(E::custom("nonce cannot be negative"))
            } else if value > u32::MAX as f64 {
                Err(E::custom("nonce exceeds u32 max"))
            } else {
                Ok(value as AddressNonce)
            }
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            let clean = value.trim_end_matches('n');
            clean
                .parse::<AddressNonce>()
                .map_err(|_| E::custom(format!("invalid nonce value: {}", value)))
        }

        fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            self.visit_str(&value)
        }
    }

    deserializer.deserialize_any(NonceVisitor)
}

impl<'de> Deserialize<'de> for PlatformAddressInputWasm {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "camelCase")]
        enum Field {
            Address,
            Nonce,
            Amount,
        }

        struct PlatformAddressInputVisitor;

        impl<'de> Visitor<'de> for PlatformAddressInputVisitor {
            type Value = PlatformAddressInputWasm;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct PlatformAddressInput")
            }

            fn visit_map<V>(self, mut map: V) -> Result<PlatformAddressInputWasm, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut address: Option<PlatformAddressWasm> = None;
                let mut nonce: Option<AddressNonce> = None;
                let mut amount: Option<Credits> = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Address => {
                            if address.is_some() {
                                return Err(de::Error::duplicate_field("address"));
                            }
                            address = Some(map.next_value()?);
                        }
                        Field::Nonce => {
                            if nonce.is_some() {
                                return Err(de::Error::duplicate_field("nonce"));
                            }
                            // Use a helper struct to deserialize nonce
                            #[derive(Deserialize)]
                            struct NonceHelper(#[serde(deserialize_with = "deserialize_nonce")] AddressNonce);
                            let helper: NonceHelper = map.next_value()?;
                            nonce = Some(helper.0);
                        }
                        Field::Amount => {
                            if amount.is_some() {
                                return Err(de::Error::duplicate_field("amount"));
                            }
                            // Use a helper struct to deserialize amount
                            #[derive(Deserialize)]
                            struct AmountHelper(#[serde(deserialize_with = "deserialize_credits")] Credits);
                            let helper: AmountHelper = map.next_value()?;
                            amount = Some(helper.0);
                        }
                    }
                }

                let address = address.ok_or_else(|| de::Error::missing_field("address"))?;
                let nonce = nonce.ok_or_else(|| de::Error::missing_field("nonce"))?;
                let amount = amount.ok_or_else(|| de::Error::missing_field("amount"))?;

                Ok(PlatformAddressInputWasm {
                    address,
                    nonce,
                    amount,
                })
            }
        }

        const FIELDS: &[&str] = &["address", "nonce", "amount"];
        deserializer.deserialize_struct("PlatformAddressInput", FIELDS, PlatformAddressInputVisitor)
    }
}

impl<'de> Deserialize<'de> for PlatformAddressOutputWasm {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "camelCase")]
        enum Field {
            Address,
            Amount,
        }

        struct PlatformAddressOutputVisitor;

        impl<'de> Visitor<'de> for PlatformAddressOutputVisitor {
            type Value = PlatformAddressOutputWasm;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct PlatformAddressOutput")
            }

            fn visit_map<V>(self, mut map: V) -> Result<PlatformAddressOutputWasm, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut address: Option<PlatformAddressWasm> = None;
                let mut amount: Option<Credits> = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Address => {
                            if address.is_some() {
                                return Err(de::Error::duplicate_field("address"));
                            }
                            address = Some(map.next_value()?);
                        }
                        Field::Amount => {
                            if amount.is_some() {
                                return Err(de::Error::duplicate_field("amount"));
                            }
                            // Use a helper struct to deserialize amount
                            #[derive(Deserialize)]
                            struct AmountHelper(#[serde(deserialize_with = "deserialize_credits")] Credits);
                            let helper: AmountHelper = map.next_value()?;
                            amount = Some(helper.0);
                        }
                    }
                }

                let address = address.ok_or_else(|| de::Error::missing_field("address"))?;
                let amount = amount.ok_or_else(|| de::Error::missing_field("amount"))?;

                Ok(PlatformAddressOutputWasm { address, amount })
            }
        }

        const FIELDS: &[&str] = &["address", "amount"];
        deserializer.deserialize_struct("PlatformAddressOutput", FIELDS, PlatformAddressOutputVisitor)
    }
}

/// Represents an output address for address-based state transitions.
///
/// An output specifies a Platform address that will receive credits,
/// along with the amount to receive.
#[wasm_bindgen(js_name = "PlatformAddressOutput")]
#[derive(Clone, Debug)]
pub struct PlatformAddressOutputWasm {
    address: PlatformAddressWasm,
    amount: Credits,
}

#[wasm_bindgen(js_class = PlatformAddressOutput)]
impl PlatformAddressOutputWasm {
    /// Creates a new PlatformAddressOutput.
    ///
    /// @param address - The Platform address (PlatformAddress, Uint8Array, or bech32m string)
    /// @param amount - The amount of credits to send to this address
    #[wasm_bindgen(constructor)]
    pub fn new(
        #[wasm_bindgen(unchecked_param_type = "PlatformAddressLike")] address: &JsValue,
        amount: BigInt,
    ) -> WasmDppResult<PlatformAddressOutputWasm> {
        let platform_address = PlatformAddressWasm::try_from(address)?;
        let amount_u64 = bigint_to_u64(amount)?;

        Ok(PlatformAddressOutputWasm {
            address: platform_address,
            amount: amount_u64,
        })
    }

    /// Returns the Platform address.
    #[wasm_bindgen(getter)]
    pub fn address(&self) -> PlatformAddressWasm {
        self.address
    }

    /// Returns the amount.
    #[wasm_bindgen(getter)]
    pub fn amount(&self) -> BigInt {
        BigInt::from(self.amount)
    }
}

impl PlatformAddressOutputWasm {
    /// Returns the inner values as a tuple suitable for BTreeMap insertion.
    pub fn into_inner(self) -> (PlatformAddress, Credits) {
        (self.address.into(), self.amount)
    }

    /// Returns a reference to the inner PlatformAddress.
    pub fn platform_address(&self) -> PlatformAddress {
        self.address.into()
    }

    /// Returns the amount value.
    pub fn amount_value(&self) -> Credits {
        self.amount
    }
}

/// Converts a vector of PlatformAddressInput into a BTreeMap.
pub fn inputs_to_btree_map(
    inputs: Vec<PlatformAddressInputWasm>,
) -> BTreeMap<PlatformAddress, (AddressNonce, Credits)> {
    inputs.into_iter().map(|i| i.into_inner()).collect()
}

/// Converts a vector of PlatformAddressOutput into a BTreeMap.
pub fn outputs_to_btree_map(
    outputs: Vec<PlatformAddressOutputWasm>,
) -> BTreeMap<PlatformAddress, Credits> {
    outputs.into_iter().map(|o| o.into_inner()).collect()
}

/// Extracts addresses from a slice of PlatformAddressOutput.
pub fn extract_addresses(outputs: &[PlatformAddressOutputWasm]) -> Vec<PlatformAddress> {
    outputs.iter().map(|o| o.platform_address()).collect()
}

/// Extracts amounts from a slice of PlatformAddressOutput.
pub fn extract_amounts(outputs: &[PlatformAddressOutputWasm]) -> Vec<Credits> {
    outputs.iter().map(|o| o.amount_value()).collect()
}
