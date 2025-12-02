# WASM return structs

Rust structs that the wasm SDK returns to JavaScript (directly or inside `Result`/`Option`). Use this list to scope TypeScript `toObject` / `toJSON` support in the generated bindings.

## Query response wrappers

- `ProofMetadataResponseWasm` (queries/mod.rs) – proof+metadata envelope used by proof-enabled queries in `dpns`, `data_contract`, `document`, `epoch`, `group`, `identity`, `protocol`, `system`, `token`, `voting`.
- `ResponseMetadataWasm` (queries/mod.rs) – metadata accessor on query responses.
- `ProofInfoWasm` (queries/mod.rs) – proof accessor on query responses.
- `StatusResponseWasm` (queries/system.rs) – `get_status`.
- `CurrentQuorumsInfoWasm` (queries/system.rs) – `get_current_quorums_info`.
- `PrefundedSpecializedBalanceWasm` (queries/system.rs) – `get_prefunded_specialized_balance`.
- `StateTransitionResultWasm` (queries/system.rs) – `get_state_transition_result`.
- `ProtocolVersionUpgradeStateWasm` (queries/protocol.rs) – `get_protocol_version_upgrade_state`.

## Domain-specific query structs

- `RegisterDpnsNameResult`, `DpnsUsernameInfo` (dpns.rs) – DPNS registration/resolution results.
- `IdentityNonceWasm`, `IdentityBalanceWasm`, `IdentityBalanceAndRevisionWasm` (queries/identity.rs) – identity nonce/balance helpers.
- `TokenPriceInfoWasm`, `TokenLastClaimWasm` (Option), `TokenTotalSupplyWasm` (Option) (queries/token.rs) – token info endpoints.
- `ContestedResourceVoteStateWasm` (queries/voting/state.rs) – contested resource vote state queries.

## Wallet / keys

- `KeyPair`, `KeyPairWasm` (wallet/key_generation.rs) – key generation helpers (single and batch).
- `DerivationPath`, `DerivationPathWasm`, `Dip13DerivationPathWasm` (wallet/key_derivation.rs) – derivation path helpers.
- `SeedPhraseKeyInfoWasm`, `PathDerivedKeyInfoWasm`, `DerivedKeyInfoWasm`, `DashpayContactKeyInfoWasm` (wallet/key_derivation.rs, wallet/extended_derivation.rs) – derived key material/introspection.
- `Dip14ExtendedPubKey` (wallet/dip14.rs) – conversion from DIP-14 extended private key.

## SDK construction

- `WasmSdk` (sdk.rs) – returned from `WasmSdkBuilder::build`.
- `WasmSdkError` (error.rs) – error type carried by all exported `Result` signatures.

toObject/fromObject and toJSON/fromJSON methods should be implemented with serde (look example) bellow.

## example

```rust
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MyType {
    pub id: u64,
    pub name: String,

    // The magic: one attribute, two behaviors
    #[serde(with = "bytes_b64")]
    pub data: Vec<u8>,
}

use wasm_bindgen::prelude::*;
use serde_wasm_bindgen as swb;

#[wasm_bindgen]
impl MyType {
    /// JS object path: binary → Uint8Array
    #[wasm_bindgen(js_name = toObject)]
    pub fn to_object(&self) -> Result<JsValue, WasmSdkError> {
        swb::to_value(self).map_err(|e| WasmSdkError::serialization(&e.to_string()))
    }

    #[wasm_bindgen(js_name = fromObject)]
    pub fn from_object(obj: JsValue) -> Result<Self, WasmSdkError> {
        swb::from_value(obj).map_err(|e| WasmSdkError::serialization(&e.to_string()))
    }
}

#[wasm_bindgen]
impl MyType {
    /// JS `toJSON` – returns a JSON **value** (object), not a string.
    /// Binary field `data` is a Base64 string in that object.
    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> Result<JsValue, WasmSdkError> {
        let v = serde_json::to_value(self)
            .map_err(|e| WasmSdkError::serialization(&e.to_string()))?;

        // JsValueSerdeExt::from_serde serializes to JSON and parses into a JS value.
        JsValue::from_serde(&v)
            .map_err(|e| WasmSdkError::serialization(&e.to_string()))
    }

    /// JS `fromJSON` – accepts a JSON value (plain JS object).
    #[wasm_bindgen(js_name = fromJSON)]
    pub fn from_json(js: JsValue) -> Result<Self, WasmSdkError> {
        // turn JS value -> serde_json::Value
        let v: serde_json::Value = js
            .into_serde()
            .map_err(|e| WasmSdkError::serialization(&e.to_string()))?;

        serde_json::from_value(v)
            .map_err(|e| WasmSdkError::serialization(&e.to_string()))
    }
}
```
