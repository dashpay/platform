use crate::error::WasmSdkError;
use crate::impl_wasm_serde_conversions;
use crate::queries::ProofMetadataResponseWasm;
use crate::sdk::WasmSdk;
use dash_sdk::dpp::address_funds::PlatformAddress;
use dash_sdk::platform::{Fetch, FetchMany};
use drive_proof_verifier::types::{AddressInfo, AddressInfos};
use js_sys::{BigInt, Map};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;
use wasm_dpp2::PlatformAddressWasm;

/// Information about a Platform address including its nonce and balance.
#[wasm_bindgen(js_name = "PlatformAddressInfo")]
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformAddressInfoWasm {
    address: PlatformAddressWasm,
    nonce: u32,
    balance: u64,
}

impl From<AddressInfo> for PlatformAddressInfoWasm {
    fn from(info: AddressInfo) -> Self {
        PlatformAddressInfoWasm {
            address: info.address.into(),
            nonce: info.nonce,
            balance: info.balance,
        }
    }
}

#[wasm_bindgen(js_class = PlatformAddressInfo)]
impl PlatformAddressInfoWasm {
    /// Returns the platform address.
    #[wasm_bindgen(getter = "address")]
    pub fn address(&self) -> PlatformAddressWasm {
        self.address
    }

    /// Returns the nonce associated with the address.
    #[wasm_bindgen(getter = "nonce")]
    pub fn nonce(&self) -> BigInt {
        BigInt::from(self.nonce)
    }

    /// Returns the balance stored for the address in credits.
    #[wasm_bindgen(getter = "balance")]
    pub fn balance(&self) -> BigInt {
        BigInt::from(self.balance)
    }
}

impl_wasm_serde_conversions!(PlatformAddressInfoWasm, PlatformAddressInfo);

#[wasm_bindgen]
impl WasmSdk {
    /// Fetches information about a Platform address including its nonce and balance.
    ///
    /// @param address - The platform address to query (PlatformAddress, Uint8Array, or bech32m string)
    /// @returns PlatformAddressInfo containing address, nonce, and balance
    #[wasm_bindgen(js_name = "getAddressInfo")]
    pub async fn get_address_info(
        &self,
        #[wasm_bindgen(unchecked_param_type = "PlatformAddressLike")] address: JsValue,
    ) -> Result<Option<PlatformAddressInfoWasm>, WasmSdkError> {
        let platform_address: PlatformAddress = PlatformAddressWasm::try_from(&address)
            .map_err(|err| {
                WasmSdkError::invalid_argument(format!("Invalid platform address: {}", err))
            })?
            .into();

        let address_info = AddressInfo::fetch(self.as_ref(), platform_address).await?;

        Ok(address_info.map(PlatformAddressInfoWasm::from))
    }

    /// Fetches information about a Platform address including its nonce and balance, with proof.
    ///
    /// @param address - The platform address to query (PlatformAddress, Uint8Array, or bech32m string)
    /// @returns ProofMetadataResponse containing PlatformAddressInfo with proof information
    #[wasm_bindgen(
        js_name = "getAddressInfoWithProofInfo",
        unchecked_return_type = "ProofMetadataResponseTyped<PlatformAddressInfo>"
    )]
    pub async fn get_address_info_with_proof_info(
        &self,
        #[wasm_bindgen(unchecked_param_type = "PlatformAddressLike")] address: JsValue,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        let platform_address: PlatformAddress = PlatformAddressWasm::try_from(&address)
            .map_err(|err| {
                WasmSdkError::invalid_argument(format!("Invalid platform address: {}", err))
            })?
            .into();

        let (address_info, metadata, proof) =
            AddressInfo::fetch_with_metadata_and_proof(self.as_ref(), platform_address, None)
                .await?;

        let data = match address_info {
            Some(info) => {
                let wrapper = PlatformAddressInfoWasm::from(info);
                wrapper.into()
            }
            None => JsValue::UNDEFINED,
        };

        Ok(ProofMetadataResponseWasm::from_sdk_parts(
            data, metadata, proof,
        ))
    }

    /// Fetches information about multiple Platform addresses.
    ///
    /// @param addresses - Array of platform addresses to query
    /// @returns Map of PlatformAddress to PlatformAddressInfo (or undefined for unfunded addresses)
    #[wasm_bindgen(
        js_name = "getAddressesInfos",
        unchecked_return_type = "Map<PlatformAddress, PlatformAddressInfo | undefined>"
    )]
    pub async fn get_addresses_infos(
        &self,
        #[wasm_bindgen(unchecked_param_type = "Array<PlatformAddressLike>")] addresses: Vec<
            JsValue,
        >,
    ) -> Result<Map, WasmSdkError> {
        let platform_addresses: BTreeSet<PlatformAddress> = addresses
            .into_iter()
            .map(|value| {
                PlatformAddressWasm::try_from(&value)
                    .map(PlatformAddress::from)
                    .map_err(|err| {
                        WasmSdkError::invalid_argument(format!("Invalid platform address: {}", err))
                    })
            })
            .collect::<Result<BTreeSet<_>, _>>()?;

        let address_infos: AddressInfos =
            AddressInfo::fetch_many(self.as_ref(), platform_addresses.clone()).await?;

        let results_map = Map::new();

        for address in platform_addresses {
            let key = JsValue::from(PlatformAddressWasm::from(address));
            let value = match address_infos.get(&address).and_then(|opt| opt.as_ref()) {
                Some(info) => {
                    let wrapper = PlatformAddressInfoWasm::from(info.clone());
                    wrapper.into()
                }
                None => JsValue::UNDEFINED,
            };
            results_map.set(&key, &value);
        }

        Ok(results_map)
    }

    /// Fetches information about multiple Platform addresses with proof.
    ///
    /// @param addresses - Array of platform addresses to query
    /// @returns ProofMetadataResponse containing Map of PlatformAddress to PlatformAddressInfo
    #[wasm_bindgen(
        js_name = "getAddressesInfosWithProofInfo",
        unchecked_return_type = "ProofMetadataResponseTyped<Map<PlatformAddress, PlatformAddressInfo | undefined>>"
    )]
    pub async fn get_addresses_infos_with_proof_info(
        &self,
        #[wasm_bindgen(unchecked_param_type = "Array<PlatformAddressLike>")] addresses: Vec<
            JsValue,
        >,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        let platform_addresses: BTreeSet<PlatformAddress> = addresses
            .into_iter()
            .map(|value| {
                PlatformAddressWasm::try_from(&value)
                    .map(PlatformAddress::from)
                    .map_err(|err| {
                        WasmSdkError::invalid_argument(format!("Invalid platform address: {}", err))
                    })
            })
            .collect::<Result<BTreeSet<_>, _>>()?;

        let (address_infos, metadata, proof): (AddressInfos, _, _) =
            AddressInfo::fetch_many_with_metadata_and_proof(
                self.as_ref(),
                platform_addresses.clone(),
                None,
            )
            .await?;

        let results_map = Map::new();

        for address in platform_addresses {
            let key = JsValue::from(PlatformAddressWasm::from(address));
            let value = match address_infos.get(&address).and_then(|opt| opt.as_ref()) {
                Some(info) => {
                    let wrapper = PlatformAddressInfoWasm::from(info.clone());
                    wrapper.into()
                }
                None => JsValue::UNDEFINED,
            };
            results_map.set(&key, &value);
        }

        Ok(ProofMetadataResponseWasm::from_sdk_parts(
            results_map,
            metadata,
            proof,
        ))
    }
}
