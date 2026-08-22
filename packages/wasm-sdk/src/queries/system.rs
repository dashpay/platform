use crate::error::WasmSdkError;
use crate::impl_wasm_serde_conversions;
use crate::queries::ProofMetadataResponseWasm;
use crate::sdk::WasmSdk;
use dash_sdk::dpp::core_types::validator_set::v0::ValidatorSetV0Getters;
use dash_sdk::drive::grovedb::{element::reference_path::path_from_reference_path_type, Element};
use dash_sdk::platform::Identifier;
use js_sys::{Array, ArrayBuffer, BigInt, Object, Reflect, Uint8Array};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use wasm_dpp2::identifier::{IdentifierLikeJs, IdentifierWasm};
use wasm_dpp2::ProTxHashWasm;

#[wasm_bindgen(typescript_custom_section)]
const GROVE_PATH_ELEMENT_TS: &'static str = r#"
export type GrovePathSegment = string | Uint8Array;

export type GroveElementType =
  | "item"
  | "reference"
  | "tree"
  | "sumItem"
  | "sumTree"
  | "bigSumTree"
  | "countTree"
  | "countSumTree"
  | "provableCountTree"
  | "itemWithSumItem"
  | "referenceWithSumItem"
  | "provableCountSumTree"
  | "provableCountProvableSumTree"
  | "provableSumTree"
  | "provableSumIndexedTree"
  | "provableCountIndexedTree"
  | "provableCountProvableSumIndexedTree"
  | "commitmentTree"
  | "mmrTree"
  | "bulkAppendTree"
  | "denseAppendOnlyFixedSizeTree"
  | "privateDocumentStore"
  | "nonCountedItem"
  | "nonCountedReference"
  | "nonCountedTree"
  | "nonCountedSumItem"
  | "nonCountedSumTree"
  | "nonCountedBigSumTree"
  | "nonCountedCountTree"
  | "nonCountedCountSumTree"
  | "nonCountedProvableCountTree"
  | "nonCountedItemWithSumItem"
  | "nonCountedReferenceWithSumItem"
  | "nonCountedProvableCountSumTree"
  | "nonCountedProvableCountProvableSumTree"
  | "nonCountedProvableSumTree"
  | "nonCountedProvableSumIndexedTree"
  | "nonCountedProvableCountIndexedTree"
  | "nonCountedProvableCountProvableSumIndexedTree"
  | "nonCountedCommitmentTree"
  | "nonCountedMmrTree"
  | "nonCountedBulkAppendTree"
  | "nonCountedDenseAppendOnlyFixedSizeTree"
  | "nonCountedPrivateDocumentStore"
  | "notSummedSumTree"
  | "notSummedBigSumTree"
  | "notSummedCountSumTree"
  | "notSummedProvableCountSumTree"
  | "notSummedProvableCountProvableSumTree"
  | "notSummedProvableSumTree"
  | "notCountedOrSummedSumTree"
  | "notCountedOrSummedBigSumTree"
  | "notCountedOrSummedCountSumTree"
  | "notCountedOrSummedProvableCountSumTree"
  | "notCountedOrSummedProvableCountProvableSumTree"
  | "notCountedOrSummedProvableSumTree";
"#;

#[wasm_bindgen(js_name = "StatusSoftware")]
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusSoftwareWasm {
    #[wasm_bindgen(getter_with_clone)]
    pub dapi: String,
    #[wasm_bindgen(getter_with_clone)]
    pub drive: Option<String>,
    #[wasm_bindgen(getter_with_clone)]
    pub tenderdash: Option<String>,
}

impl StatusSoftwareWasm {
    fn new(dapi: String, drive: Option<String>, tenderdash: Option<String>) -> Self {
        Self {
            dapi,
            drive,
            tenderdash,
        }
    }
}

#[wasm_bindgen(js_name = "StatusTenderdashProtocol")]
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusTenderdashProtocolWasm {
    #[wasm_bindgen(getter_with_clone)]
    pub p2p: u32,
    #[wasm_bindgen(getter_with_clone)]
    pub block: u32,
}

impl StatusTenderdashProtocolWasm {
    fn new(p2p: u32, block: u32) -> Self {
        Self { p2p, block }
    }
}

#[wasm_bindgen(js_name = "StatusDriveProtocol")]
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusDriveProtocolWasm {
    #[wasm_bindgen(getter_with_clone)]
    pub latest: u32,
    #[wasm_bindgen(getter_with_clone)]
    pub current: u32,
}

impl StatusDriveProtocolWasm {
    fn new(latest: u32, current: u32) -> Self {
        Self { latest, current }
    }
}

#[wasm_bindgen(js_name = "StatusProtocol")]
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusProtocolWasm {
    #[wasm_bindgen(getter_with_clone)]
    pub tenderdash: StatusTenderdashProtocolWasm,
    #[wasm_bindgen(getter_with_clone)]
    pub drive: StatusDriveProtocolWasm,
}

impl StatusProtocolWasm {
    fn new(tenderdash: StatusTenderdashProtocolWasm, drive: StatusDriveProtocolWasm) -> Self {
        Self { tenderdash, drive }
    }
}

#[wasm_bindgen(js_name = "StatusVersion")]
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusVersionWasm {
    #[wasm_bindgen(getter_with_clone)]
    pub software: StatusSoftwareWasm,
    #[wasm_bindgen(getter_with_clone)]
    pub protocol: StatusProtocolWasm,
}

impl StatusVersionWasm {
    fn new(software: StatusSoftwareWasm, protocol: StatusProtocolWasm) -> Self {
        Self { software, protocol }
    }
}

#[wasm_bindgen(js_name = "StatusNode")]
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusNodeWasm {
    pub(crate) id: String,
    pub(crate) pro_tx_hash: Option<String>,
}

impl StatusNodeWasm {
    fn new(id: String, pro_tx_hash: Option<String>) -> Self {
        Self { id, pro_tx_hash }
    }
}

#[wasm_bindgen(js_class = StatusNode)]
impl StatusNodeWasm {
    #[wasm_bindgen(getter)]
    pub fn id(&self) -> String {
        self.id.clone()
    }

    #[wasm_bindgen(getter = "proTxHash")]
    pub fn pro_tx_hash(&self) -> Option<ProTxHashWasm> {
        self.pro_tx_hash
            .as_ref()
            .and_then(|hex| ProTxHashWasm::from_hex(hex).ok())
    }
}

#[wasm_bindgen(js_name = "StatusChain")]
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusChainWasm {
    #[wasm_bindgen(getter_with_clone, js_name = "isCatchingUp")]
    pub is_catching_up: bool,
    #[wasm_bindgen(getter_with_clone)]
    pub latest_block_hash: String,
    #[wasm_bindgen(getter_with_clone)]
    pub latest_app_hash: String,
    #[wasm_bindgen(getter_with_clone)]
    pub latest_block_height: String,
    #[wasm_bindgen(getter_with_clone)]
    pub earliest_block_hash: String,
    #[wasm_bindgen(getter_with_clone)]
    pub earliest_app_hash: String,
    #[wasm_bindgen(getter_with_clone)]
    pub earliest_block_height: String,
    #[wasm_bindgen(getter_with_clone)]
    pub max_peer_block_height: String,
    #[wasm_bindgen(getter_with_clone)]
    pub core_chain_locked_height: Option<u32>,
}

impl StatusChainWasm {
    #[allow(clippy::too_many_arguments)]
    fn new(
        is_catching_up: bool,
        latest_block_hash: String,
        latest_app_hash: String,
        latest_block_height: String,
        earliest_block_hash: String,
        earliest_app_hash: String,
        earliest_block_height: String,
        max_peer_block_height: String,
        core_chain_locked_height: Option<u32>,
    ) -> Self {
        Self {
            is_catching_up,
            latest_block_hash,
            latest_app_hash,
            latest_block_height,
            earliest_block_hash,
            earliest_app_hash,
            earliest_block_height,
            max_peer_block_height,
            core_chain_locked_height,
        }
    }
}

#[wasm_bindgen(js_name = "StatusNetwork")]
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusNetworkWasm {
    #[wasm_bindgen(getter_with_clone)]
    pub chain_id: String,
    #[wasm_bindgen(getter_with_clone)]
    pub peers_count: u32,
    #[wasm_bindgen(getter_with_clone, js_name = "isListening")]
    pub is_listening: bool,
}

impl StatusNetworkWasm {
    fn new(chain_id: String, peers_count: u32, is_listening: bool) -> Self {
        Self {
            chain_id,
            peers_count,
            is_listening,
        }
    }
}

#[wasm_bindgen(js_name = "StatusStateSync")]
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusStateSyncWasm {
    #[wasm_bindgen(getter_with_clone)]
    pub total_synced_time: String,
    #[wasm_bindgen(getter_with_clone)]
    pub remaining_time: String,
    #[wasm_bindgen(getter_with_clone)]
    pub total_snapshots: u32,
    #[wasm_bindgen(getter_with_clone)]
    pub chunk_process_avg_time: String,
    #[wasm_bindgen(getter_with_clone)]
    pub snapshot_height: String,
    #[wasm_bindgen(getter_with_clone)]
    pub snapshot_chunks_count: String,
    #[wasm_bindgen(getter_with_clone)]
    pub backfilled_blocks: String,
    #[wasm_bindgen(getter_with_clone)]
    pub backfill_blocks_total: String,
}

#[wasm_bindgen(js_name = "StatusTime")]
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusTimeWasm {
    #[wasm_bindgen(getter_with_clone)]
    pub local: String,
    #[wasm_bindgen(getter_with_clone)]
    pub block: Option<String>,
    #[wasm_bindgen(getter_with_clone)]
    pub genesis: Option<String>,
    #[wasm_bindgen(getter_with_clone)]
    pub epoch: Option<u32>,
}

impl StatusTimeWasm {
    fn new(
        local: String,
        block: Option<String>,
        genesis: Option<String>,
        epoch: Option<u32>,
    ) -> Self {
        Self {
            local,
            block,
            genesis,
            epoch,
        }
    }
}

#[wasm_bindgen(js_name = "StatusResponse")]
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusResponseWasm {
    #[wasm_bindgen(getter_with_clone)]
    pub version: StatusVersionWasm,
    #[wasm_bindgen(getter_with_clone)]
    pub node: StatusNodeWasm,
    #[wasm_bindgen(getter_with_clone)]
    pub chain: StatusChainWasm,
    #[wasm_bindgen(getter_with_clone)]
    pub network: StatusNetworkWasm,
    #[wasm_bindgen(getter_with_clone)]
    pub state_sync: StatusStateSyncWasm,
    #[wasm_bindgen(getter_with_clone)]
    pub time: StatusTimeWasm,
}

impl StatusResponseWasm {
    fn new(
        version: StatusVersionWasm,
        node: StatusNodeWasm,
        chain: StatusChainWasm,
        network: StatusNetworkWasm,
        state_sync: StatusStateSyncWasm,
        time: StatusTimeWasm,
    ) -> Self {
        Self {
            version,
            node,
            chain,
            network,
            state_sync,
            time,
        }
    }
}

impl_wasm_serde_conversions!(StatusSoftwareWasm, StatusSoftware);
impl_wasm_serde_conversions!(StatusTenderdashProtocolWasm, StatusTenderdashProtocol);
impl_wasm_serde_conversions!(StatusDriveProtocolWasm, StatusDriveProtocol);
impl_wasm_serde_conversions!(StatusProtocolWasm, StatusProtocol);
impl_wasm_serde_conversions!(StatusVersionWasm, StatusVersion);
impl_wasm_serde_conversions!(StatusNodeWasm, StatusNode);
impl_wasm_serde_conversions!(StatusChainWasm, StatusChain);
impl_wasm_serde_conversions!(StatusNetworkWasm, StatusNetwork);
impl_wasm_serde_conversions!(StatusStateSyncWasm, StatusStateSync);
impl_wasm_serde_conversions!(StatusTimeWasm, StatusTime);
impl_wasm_serde_conversions!(StatusResponseWasm, StatusResponse);
impl_wasm_serde_conversions!(QuorumInfoWasm, QuorumInfo);
impl_wasm_serde_conversions!(CurrentQuorumsInfoWasm, CurrentQuorumsInfo);
impl_wasm_serde_conversions!(PrefundedSpecializedBalanceWasm, PrefundedSpecializedBalance);
impl_wasm_serde_conversions!(StateTransitionResultWasm, StateTransitionResult);

#[wasm_bindgen(js_name = "QuorumInfo")]
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuorumInfoWasm {
    #[wasm_bindgen(getter_with_clone, js_name = "quorumHash")]
    pub quorum_hash: String,
    #[wasm_bindgen(getter_with_clone, js_name = "quorumType")]
    pub quorum_type: String,
    #[wasm_bindgen(getter_with_clone, js_name = "memberCount")]
    pub member_count: u32,
    #[wasm_bindgen(getter_with_clone)]
    pub threshold: u32,
    #[wasm_bindgen(getter_with_clone, js_name = "isVerified")]
    pub is_verified: bool,
}

impl QuorumInfoWasm {
    pub(crate) fn new(
        quorum_hash: String,
        quorum_type: String,
        member_count: u32,
        threshold: u32,
        is_verified: bool,
    ) -> Self {
        Self {
            quorum_hash,
            quorum_type,
            member_count,
            threshold,
            is_verified,
        }
    }
}

#[dpp_json_convertible_derive::json_safe_fields(crate = "dash_sdk::dpp")]
#[wasm_bindgen(js_name = "CurrentQuorumsInfo")]
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurrentQuorumsInfoWasm {
    quorums: Vec<QuorumInfoWasm>,
    height: u64,
}

impl CurrentQuorumsInfoWasm {
    fn new(quorums: Vec<QuorumInfoWasm>, height: u64) -> Self {
        Self { quorums, height }
    }
}

#[wasm_bindgen(js_class = CurrentQuorumsInfo)]
impl CurrentQuorumsInfoWasm {
    #[wasm_bindgen(getter = "quorums")]
    pub fn quorums(&self) -> Array {
        let array = Array::new();
        for quorum in &self.quorums {
            array.push(&JsValue::from(quorum.clone()));
        }
        array
    }

    #[wasm_bindgen(getter = "height")]
    pub fn height(&self) -> u64 {
        self.height
    }
}

#[dpp_json_convertible_derive::json_safe_fields(crate = "dash_sdk::dpp")]
#[wasm_bindgen(js_name = "PrefundedSpecializedBalance")]
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrefundedSpecializedBalanceWasm {
    identity_id: IdentifierWasm,
    balance: u64,
}

impl PrefundedSpecializedBalanceWasm {
    fn new(identity_id: IdentifierWasm, balance: u64) -> Self {
        Self {
            identity_id,
            balance,
        }
    }
}

#[wasm_bindgen(js_class = PrefundedSpecializedBalance)]
impl PrefundedSpecializedBalanceWasm {
    #[wasm_bindgen(getter = "identityId")]
    pub fn identity_id(&self) -> IdentifierWasm {
        self.identity_id
    }

    #[wasm_bindgen(getter = "balance")]
    pub fn balance(&self) -> BigInt {
        BigInt::from(self.balance)
    }
}

#[wasm_bindgen(js_name = "PathElement")]
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PathElementWasm {
    path: Vec<String>,
    #[serde(with = "path_element_bytes")]
    key: Vec<u8>,
    #[serde(with = "path_element_bytes_vec")]
    path_bytes: Vec<Vec<u8>>,
    #[wasm_bindgen(getter_with_clone)]
    pub value: Option<String>,
    #[serde(with = "path_element_optional_bytes")]
    value_bytes: Option<Vec<u8>>,
    element_type: Option<String>,
    #[serde(with = "path_element_optional_i128")]
    sum: Option<i128>,
    #[serde(with = "path_element_optional_bytes_vec")]
    reference_target: Option<Vec<Vec<u8>>>,
    reference_target_error: Option<String>,
}

impl PathElementWasm {
    fn missing(parent_path: &[Vec<u8>], key: &DecodedPathInput) -> Self {
        let mut path_bytes = parent_path.to_vec();
        path_bytes.push(key.bytes.clone());

        Self {
            path: key.legacy_path_segment.clone().into_iter().collect(),
            key: key.bytes.clone(),
            path_bytes,
            value: None,
            value_bytes: None,
            element_type: None,
            sum: None,
            reference_target: None,
            reference_target_error: None,
        }
    }

    fn from_element(parent_path: &[Vec<u8>], key: &DecodedPathInput, element: &Element) -> Self {
        let value_bytes = element_value_bytes(element);
        let value = value_bytes.as_ref().map(|bytes| {
            use base64::Engine;
            base64::engine::general_purpose::STANDARD.encode(bytes)
        });
        let mut path_bytes = parent_path.to_vec();
        path_bytes.push(key.bytes.clone());
        let (reference_target, reference_target_error) =
            element_reference_target(element, parent_path, &key.bytes);

        Self {
            path: key.legacy_path_segment.clone().into_iter().collect(),
            key: key.bytes.clone(),
            path_bytes,
            value,
            value_bytes,
            element_type: Some(element_type_name(element).to_string()),
            sum: element_sum(element),
            reference_target,
            reference_target_error,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct DecodedPathInput {
    bytes: Vec<u8>,
    legacy_path_segment: Option<String>,
}

enum PathInputValue<'a> {
    Bytes(&'a [u8]),
    String(&'a str),
}

mod path_element_bytes {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    #[derive(Serialize, Deserialize)]
    pub(super) struct BytesField(
        #[serde(with = "dash_sdk::dpp::serialization::serde_bytes_var")] pub(super) Vec<u8>,
    );

    pub fn serialize<S>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        BytesField(bytes.to_owned()).serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        BytesField::deserialize(deserializer).map(|field| field.0)
    }
}

mod path_element_optional_bytes {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    use super::path_element_bytes::BytesField;

    pub fn serialize<S>(bytes: &Option<Vec<u8>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        bytes
            .as_ref()
            .map(|bytes| BytesField(bytes.clone()))
            .serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Vec<u8>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        Option::<BytesField>::deserialize(deserializer).map(|field| field.map(|bytes| bytes.0))
    }
}

mod path_element_bytes_vec {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    use super::path_element_bytes::BytesField;

    pub fn serialize<S>(path: &[Vec<u8>], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        path.iter()
            .cloned()
            .map(BytesField)
            .collect::<Vec<_>>()
            .serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<Vec<u8>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        Vec::<BytesField>::deserialize(deserializer)
            .map(|path| path.into_iter().map(|bytes| bytes.0).collect())
    }
}

mod path_element_optional_bytes_vec {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    use super::path_element_bytes::BytesField;

    pub fn serialize<S>(path: &Option<Vec<Vec<u8>>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        path.as_ref()
            .map(|path| path.iter().cloned().map(BytesField).collect::<Vec<_>>())
            .serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Vec<Vec<u8>>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        Option::<Vec<BytesField>>::deserialize(deserializer)
            .map(|path| path.map(|path| path.into_iter().map(|bytes| bytes.0).collect()))
    }
}

mod path_element_optional_i128 {
    use serde::de::{self, Deserializer, Visitor};
    use serde::ser::Serializer;

    pub fn serialize<S>(value: &Option<i128>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match value {
            Some(value) if serializer.is_human_readable() => {
                serializer.serialize_some(&value.to_string())
            }
            Some(value) => serializer.serialize_some(value),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<i128>, D::Error>
    where
        D: Deserializer<'de>,
    {
        if deserializer.is_human_readable() {
            deserializer.deserialize_option(OptionI128Visitor)
        } else {
            serde::Deserialize::deserialize(deserializer)
        }
    }

    struct OptionI128Visitor;

    impl<'de> Visitor<'de> for OptionI128Visitor {
        type Value = Option<i128>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("null, an integer, or a string containing an i128")
        }

        fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
            Ok(None)
        }

        fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
            Ok(None)
        }

        fn visit_some<D: Deserializer<'de>>(
            self,
            deserializer: D,
        ) -> Result<Self::Value, D::Error> {
            deserializer.deserialize_any(I128Visitor).map(Some)
        }
    }

    struct I128Visitor;

    impl<'de> Visitor<'de> for I128Visitor {
        type Value = i128;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("an integer or a string containing an i128")
        }

        fn visit_i64<E: de::Error>(self, value: i64) -> Result<Self::Value, E> {
            Ok(value as i128)
        }

        fn visit_i128<E: de::Error>(self, value: i128) -> Result<Self::Value, E> {
            Ok(value)
        }

        fn visit_u64<E: de::Error>(self, value: u64) -> Result<Self::Value, E> {
            Ok(value as i128)
        }

        fn visit_u128<E: de::Error>(self, value: u128) -> Result<Self::Value, E> {
            i128::try_from(value)
                .map_err(|_| de::Error::custom(format!("u128 value {value} out of i128 range")))
        }

        fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
            value
                .parse::<i128>()
                .map_err(|_| de::Error::custom(format!("invalid i128 string: {value}")))
        }
    }
}

/// Decode a public Grove path segment.
///
/// String path segments preserve a legacy compatibility rule: decimal strings
/// in the `u8` range are decoded as a single byte, while other strings are
/// decoded as UTF-8. Use `Uint8Array` for unambiguous binary path segments.
fn decode_path_input(value: PathInputValue<'_>) -> DecodedPathInput {
    match value {
        PathInputValue::Bytes(bytes) => DecodedPathInput {
            bytes: bytes.to_vec(),
            legacy_path_segment: bytes_to_round_trippable_path_display(bytes),
        },
        PathInputValue::String(value) => DecodedPathInput {
            bytes: decode_path_string(value),
            legacy_path_segment: Some(value.to_string()),
        },
    }
}

fn decode_key_input(value: PathInputValue<'_>) -> DecodedPathInput {
    match value {
        PathInputValue::Bytes(bytes) => DecodedPathInput {
            bytes: bytes.to_vec(),
            legacy_path_segment: bytes_to_round_trippable_key_display(bytes),
        },
        PathInputValue::String(value) => DecodedPathInput {
            bytes: value.as_bytes().to_vec(),
            legacy_path_segment: Some(value.to_string()),
        },
    }
}

fn decode_path_string(value: &str) -> Vec<u8> {
    if let Ok(number) = value.parse::<u8>() {
        tracing::warn!(
            "decoding Grove path string segment as legacy decimal u8; use Uint8Array for unambiguous binary paths"
        );
        vec![number]
    } else {
        value.as_bytes().to_vec()
    }
}

fn decode_path_string_silent(value: &str) -> Vec<u8> {
    value
        .parse::<u8>()
        .map(|number| vec![number])
        .unwrap_or_else(|_| value.as_bytes().to_vec())
}

fn decode_js_path_inputs(
    array: &Array,
    field: &str,
) -> Result<Vec<DecodedPathInput>, WasmSdkError> {
    decode_js_inputs(array, field, decode_path_input)
}

fn decode_js_key_inputs(array: &Array, field: &str) -> Result<Vec<DecodedPathInput>, WasmSdkError> {
    decode_js_inputs(array, field, decode_key_input)
}

fn decode_js_inputs(
    array: &Array,
    field: &str,
    decode: fn(PathInputValue<'_>) -> DecodedPathInput,
) -> Result<Vec<DecodedPathInput>, WasmSdkError> {
    array
        .iter()
        .enumerate()
        .map(|(index, value)| {
            if is_uint8_array_value(&value) {
                let bytes = value.unchecked_ref::<Uint8Array>().to_vec();
                Ok(decode(PathInputValue::Bytes(&bytes)))
            } else if let Some(string) = value.as_string() {
                Ok(decode(PathInputValue::String(&string)))
            } else {
                Err(WasmSdkError::invalid_argument(format!(
                    "{}[{}] must be a string or Uint8Array",
                    field, index
                )))
            }
        })
        .collect()
}

fn is_uint8_array_value(value: &JsValue) -> bool {
    if value.is_instance_of::<Uint8Array>() {
        return true;
    }

    if !ArrayBuffer::is_view(value) {
        return false;
    }

    let constructor_name = Reflect::get(value, &JsValue::from_str("constructor"))
        .ok()
        .and_then(|constructor| Reflect::get(&constructor, &JsValue::from_str("name")).ok())
        .and_then(|name| name.as_string());

    matches!(
        constructor_name.as_deref(),
        Some("Uint8Array") | Some("Buffer")
    )
}

fn decoded_bytes(inputs: &[DecodedPathInput]) -> Vec<Vec<u8>> {
    inputs.iter().map(|input| input.bytes.clone()).collect()
}

fn bytes_to_round_trippable_path_display(bytes: &[u8]) -> Option<String> {
    let value = std::str::from_utf8(bytes).ok()?;
    (decode_path_string_silent(value) == bytes).then(|| value.to_string())
}

fn bytes_to_round_trippable_key_display(bytes: &[u8]) -> Option<String> {
    std::str::from_utf8(bytes).ok().map(str::to_string)
}

fn bytes_path_to_js_array(path: &[Vec<u8>]) -> Array {
    let array = Array::new();
    for segment in path {
        array.push(&Uint8Array::from(segment.as_slice()));
    }
    array
}

fn set_js_property(object: &Object, property: &str, value: &JsValue) -> Result<(), WasmSdkError> {
    Reflect::set(object, &JsValue::from_str(property), value).map_err(|_| {
        WasmSdkError::generic(format!("failed to set PathElement.{} property", property))
    })?;
    Ok(())
}

fn element_value_bytes(element: &Element) -> Option<Vec<u8>> {
    match element {
        Element::Item(bytes, _) => Some(bytes.clone()),
        Element::ItemWithSumItem(bytes, _, _) => Some(bytes.clone()),
        Element::NonCounted(inner)
        | Element::NotSummed(inner)
        | Element::NotCountedOrSummed(inner) => element_value_bytes(inner),
        _ => None,
    }
}

fn element_sum(element: &Element) -> Option<i128> {
    match element {
        Element::SumItem(sum, _) => Some(*sum as i128),
        Element::SumTree(_, sum, _) => Some(*sum as i128),
        Element::BigSumTree(_, sum, _) => Some(*sum),
        Element::CountSumTree(_, _, sum, _) => Some(*sum as i128),
        Element::ItemWithSumItem(_, sum, _) => Some(*sum as i128),
        Element::ProvableCountSumTree(_, _, sum, _) => Some(*sum as i128),
        Element::ProvableCountProvableSumTree(_, _, sum, _) => Some(*sum as i128),
        Element::ReferenceWithSumItem(_, _, sum, _) => Some(*sum as i128),
        Element::ProvableSumTree(_, sum, _) => Some(*sum as i128),
        // Indexed trees carry the same aggregate as their non-indexed
        // counterparts — the secondary Merks are an ordering over it, not a
        // second copy of it. `ProvableSumIndexedTree` is
        // `(primary_root_key, secondary_root_key, sum, flags)` and
        // `ProvableCountProvableSumIndexedTree` is
        // `(primary_root_key, count, sum, axes, flags)`.
        // `ProvableCountIndexedTree` holds a count and no sum, so it is
        // deliberately absent here and falls through to `None`.
        Element::ProvableSumIndexedTree(_, _, sum, _) => Some(*sum as i128),
        Element::ProvableCountProvableSumIndexedTree(_, _, sum, _, _) => Some(*sum as i128),
        Element::NonCounted(inner)
        | Element::NotSummed(inner)
        | Element::NotCountedOrSummed(inner) => element_sum(inner),
        _ => None,
    }
}

fn element_reference_target(
    element: &Element,
    parent_path: &[Vec<u8>],
    key: &[u8],
) -> (Option<Vec<Vec<u8>>>, Option<String>) {
    match element {
        Element::Reference(reference_path, _, _)
        | Element::ReferenceWithSumItem(reference_path, _, _, _) => {
            match path_from_reference_path_type(reference_path.clone(), parent_path, Some(key)) {
                Ok(target) => (Some(target), None),
                Err(error) => {
                    let message = error.to_string();
                    tracing::warn!("failed to resolve GroveDB reference target: {}", message);
                    (None, Some(message))
                }
            }
        }
        Element::NonCounted(inner)
        | Element::NotSummed(inner)
        | Element::NotCountedOrSummed(inner) => element_reference_target(inner, parent_path, key),
        _ => (None, None),
    }
}

fn element_type_name(element: &Element) -> &'static str {
    match element {
        Element::Item(_, _) => "item",
        Element::Reference(_, _, _) => "reference",
        Element::Tree(_, _) => "tree",
        Element::SumItem(_, _) => "sumItem",
        Element::SumTree(_, _, _) => "sumTree",
        Element::BigSumTree(_, _, _) => "bigSumTree",
        Element::CountTree(_, _, _) => "countTree",
        Element::CountSumTree(_, _, _, _) => "countSumTree",
        Element::ProvableCountTree(_, _, _) => "provableCountTree",
        Element::ItemWithSumItem(_, _, _) => "itemWithSumItem",
        Element::ReferenceWithSumItem(_, _, _, _) => "referenceWithSumItem",
        Element::ProvableCountSumTree(_, _, _, _) => "provableCountSumTree",
        Element::ProvableCountProvableSumTree(_, _, _, _) => "provableCountProvableSumTree",
        Element::ProvableSumTree(_, _, _) => "provableSumTree",
        Element::ProvableSumIndexedTree(_, _, _, _) => "provableSumIndexedTree",
        Element::ProvableCountIndexedTree(_, _, _, _) => "provableCountIndexedTree",
        Element::ProvableCountProvableSumIndexedTree(_, _, _, _, _) => {
            "provableCountProvableSumIndexedTree"
        }
        Element::CommitmentTree(_, _, _) => "commitmentTree",
        Element::MmrTree(_, _) => "mmrTree",
        Element::BulkAppendTree(_, _, _) => "bulkAppendTree",
        Element::DenseAppendOnlyFixedSizeTree(_, _, _) => "denseAppendOnlyFixedSizeTree",

        Element::PrivateDocumentStore(_, _, _, _) => "privateDocumentStore",
        Element::NonCounted(inner) => non_counted_element_type_name(inner),
        Element::NotSummed(inner) => not_summed_element_type_name(inner),
        Element::NotCountedOrSummed(inner) => not_counted_or_summed_element_type_name(inner),
    }
}

fn non_counted_element_type_name(element: &Element) -> &'static str {
    match element {
        Element::Item(_, _) => "nonCountedItem",
        Element::Reference(_, _, _) => "nonCountedReference",
        Element::Tree(_, _) => "nonCountedTree",
        Element::SumItem(_, _) => "nonCountedSumItem",
        Element::SumTree(_, _, _) => "nonCountedSumTree",
        Element::BigSumTree(_, _, _) => "nonCountedBigSumTree",
        Element::CountTree(_, _, _) => "nonCountedCountTree",
        Element::CountSumTree(_, _, _, _) => "nonCountedCountSumTree",
        Element::ProvableCountTree(_, _, _) => "nonCountedProvableCountTree",
        Element::ItemWithSumItem(_, _, _) => "nonCountedItemWithSumItem",
        Element::ReferenceWithSumItem(_, _, _, _) => "nonCountedReferenceWithSumItem",
        Element::ProvableCountSumTree(_, _, _, _) => "nonCountedProvableCountSumTree",
        Element::ProvableCountProvableSumTree(_, _, _, _) => {
            "nonCountedProvableCountProvableSumTree"
        }
        Element::ProvableSumTree(_, _, _) => "nonCountedProvableSumTree",
        Element::ProvableSumIndexedTree(_, _, _, _) => "nonCountedProvableSumIndexedTree",
        Element::ProvableCountIndexedTree(_, _, _, _) => "nonCountedProvableCountIndexedTree",
        Element::ProvableCountProvableSumIndexedTree(_, _, _, _, _) => {
            "nonCountedProvableCountProvableSumIndexedTree"
        }
        Element::CommitmentTree(_, _, _) => "nonCountedCommitmentTree",
        Element::MmrTree(_, _) => "nonCountedMmrTree",
        Element::BulkAppendTree(_, _, _) => "nonCountedBulkAppendTree",
        Element::DenseAppendOnlyFixedSizeTree(_, _, _) => "nonCountedDenseAppendOnlyFixedSizeTree",

        Element::PrivateDocumentStore(_, _, _, _) => "nonCountedPrivateDocumentStore",
        Element::NonCounted(_) | Element::NotSummed(_) | Element::NotCountedOrSummed(_) => {
            element_type_name(element)
        }
    }
}

fn not_summed_element_type_name(element: &Element) -> &'static str {
    match element {
        Element::SumTree(_, _, _) => "notSummedSumTree",
        Element::BigSumTree(_, _, _) => "notSummedBigSumTree",
        Element::CountSumTree(_, _, _, _) => "notSummedCountSumTree",
        Element::ProvableCountSumTree(_, _, _, _) => "notSummedProvableCountSumTree",
        Element::ProvableCountProvableSumTree(_, _, _, _) => {
            "notSummedProvableCountProvableSumTree"
        }
        Element::ProvableSumTree(_, _, _) => "notSummedProvableSumTree",
        _ => element_type_name(element),
    }
}

fn not_counted_or_summed_element_type_name(element: &Element) -> &'static str {
    match element {
        Element::SumTree(_, _, _) => "notCountedOrSummedSumTree",
        Element::BigSumTree(_, _, _) => "notCountedOrSummedBigSumTree",
        Element::CountSumTree(_, _, _, _) => "notCountedOrSummedCountSumTree",
        Element::ProvableCountSumTree(_, _, _, _) => "notCountedOrSummedProvableCountSumTree",
        Element::ProvableCountProvableSumTree(_, _, _, _) => {
            "notCountedOrSummedProvableCountProvableSumTree"
        }
        Element::ProvableSumTree(_, _, _) => "notCountedOrSummedProvableSumTree",
        _ => element_type_name(element),
    }
}

#[wasm_bindgen(js_class = PathElement)]
impl PathElementWasm {
    #[wasm_bindgen(js_name = toObject)]
    pub fn to_object(&self) -> Result<JsValue, WasmSdkError> {
        let object = Object::new();

        set_js_property(&object, "path", &self.path().into())?;
        set_js_property(&object, "key", &self.key().into())?;
        set_js_property(&object, "pathBytes", &self.path_bytes().into())?;

        let value = self
            .value
            .as_ref()
            .map(|value| JsValue::from_str(value))
            .unwrap_or(JsValue::UNDEFINED);
        set_js_property(&object, "value", &value)?;

        let value_bytes = self
            .value_bytes()
            .map(JsValue::from)
            .unwrap_or(JsValue::UNDEFINED);
        set_js_property(&object, "valueBytes", &value_bytes)?;

        let element_type = self
            .element_type()
            .map(|element_type| JsValue::from_str(&element_type))
            .unwrap_or(JsValue::UNDEFINED);
        set_js_property(&object, "elementType", &element_type)?;

        let sum = self.sum().map(JsValue::from).unwrap_or(JsValue::UNDEFINED);
        set_js_property(&object, "sum", &sum)?;

        let reference_target = self
            .reference_target()
            .map(JsValue::from)
            .unwrap_or(JsValue::UNDEFINED);
        set_js_property(&object, "referenceTarget", &reference_target)?;

        let reference_target_error = self
            .reference_target_error()
            .map(|error| JsValue::from_str(&error))
            .unwrap_or(JsValue::UNDEFINED);
        set_js_property(&object, "referenceTargetError", &reference_target_error)?;

        Ok(object.into())
    }

    #[wasm_bindgen(js_name = fromObject)]
    pub fn from_object(obj: Object) -> Result<PathElementWasm, WasmSdkError> {
        wasm_dpp2::serialization::from_object(obj.into()).map_err(WasmSdkError::from)
    }

    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> Result<JsValue, WasmSdkError> {
        wasm_dpp2::serialization::to_json(self).map_err(WasmSdkError::from)
    }

    #[wasm_bindgen(js_name = fromJSON)]
    pub fn from_json(js: Object) -> Result<PathElementWasm, WasmSdkError> {
        wasm_dpp2::serialization::from_json(js.into()).map_err(WasmSdkError::from)
    }

    #[wasm_bindgen(getter)]
    pub fn path(&self) -> Array {
        let array = Array::new();
        for segment in &self.path {
            array.push(&JsValue::from_str(segment));
        }
        array
    }

    #[wasm_bindgen(getter)]
    pub fn key(&self) -> Uint8Array {
        Uint8Array::from(self.key.as_slice())
    }

    #[wasm_bindgen(getter = "pathBytes", unchecked_return_type = "Uint8Array[]")]
    pub fn path_bytes(&self) -> Array {
        bytes_path_to_js_array(&self.path_bytes)
    }

    #[wasm_bindgen(getter = "valueBytes")]
    pub fn value_bytes(&self) -> Option<Uint8Array> {
        self.value_bytes
            .as_ref()
            .map(|bytes| Uint8Array::from(bytes.as_slice()))
    }

    #[wasm_bindgen(
        getter = "elementType",
        unchecked_return_type = "GroveElementType | undefined"
    )]
    pub fn element_type(&self) -> Option<String> {
        self.element_type.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn sum(&self) -> Option<BigInt> {
        self.sum.as_ref().map(|sum| {
            BigInt::new(&JsValue::from_str(&sum.to_string()))
                .expect("i128 decimal string always parses as BigInt")
        })
    }

    #[wasm_bindgen(
        getter = "referenceTarget",
        unchecked_return_type = "Uint8Array[] | undefined"
    )]
    pub fn reference_target(&self) -> Option<Array> {
        self.reference_target
            .as_ref()
            .map(|path| bytes_path_to_js_array(path))
    }

    #[wasm_bindgen(getter = "referenceTargetError")]
    pub fn reference_target_error(&self) -> Option<String> {
        self.reference_target_error.clone()
    }
}

#[wasm_bindgen(js_name = "StateTransitionResult")]
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StateTransitionResultWasm {
    #[wasm_bindgen(getter_with_clone)]
    pub state_transition_hash: String,
    #[wasm_bindgen(getter_with_clone)]
    pub status: String,
    #[wasm_bindgen(getter_with_clone)]
    pub error: Option<String>,
}

impl StateTransitionResultWasm {
    fn new(state_transition_hash: String, status: String, error: Option<String>) -> Self {
        Self {
            state_transition_hash,
            status,
            error,
        }
    }
}

#[wasm_bindgen]
impl WasmSdk {
    #[wasm_bindgen(js_name = "getStatus")]
    pub async fn get_status(&self) -> Result<StatusResponseWasm, WasmSdkError> {
        use dapi_grpc::platform::v0::get_status_request::{GetStatusRequestV0, Version};
        use dapi_grpc::platform::v0::GetStatusRequest;
        use dash_sdk::RequestSettings;
        use rs_dapi_client::DapiRequestExecutor;

        // Create the gRPC request
        let request = GetStatusRequest {
            version: Some(Version::V0(GetStatusRequestV0 {})),
        };

        // Execute the request
        let response = self
            .as_ref()
            .execute(request, RequestSettings::default())
            .await
            .map_err(|e| WasmSdkError::generic(format!("Failed to get status: {}", e)))?;

        // Parse the response
        use dapi_grpc::platform::v0::get_status_response::Version as ResponseVersion;

        let v0_response = match response.inner.version {
            Some(ResponseVersion::V0(v0)) => v0,
            None => return Err(WasmSdkError::generic("No version in GetStatus response")),
        };

        // Map the response to our StatusResponse structure
        let software = StatusSoftwareWasm::new(
            v0_response
                .version
                .as_ref()
                .and_then(|v| v.software.as_ref())
                .map(|s| s.dapi.clone())
                .unwrap_or_else(|| "unknown".to_string()),
            v0_response
                .version
                .as_ref()
                .and_then(|v| v.software.as_ref())
                .and_then(|s| s.drive.clone()),
            v0_response
                .version
                .as_ref()
                .and_then(|v| v.software.as_ref())
                .and_then(|s| s.tenderdash.clone()),
        );

        let tenderdash_protocol = StatusTenderdashProtocolWasm::new(
            v0_response
                .version
                .as_ref()
                .and_then(|v| v.protocol.as_ref())
                .and_then(|p| p.tenderdash.as_ref())
                .map(|t| t.p2p)
                .unwrap_or(0),
            v0_response
                .version
                .as_ref()
                .and_then(|v| v.protocol.as_ref())
                .and_then(|p| p.tenderdash.as_ref())
                .map(|t| t.block)
                .unwrap_or(0),
        );

        let drive_protocol = StatusDriveProtocolWasm::new(
            v0_response
                .version
                .as_ref()
                .and_then(|v| v.protocol.as_ref())
                .and_then(|p| p.drive.as_ref())
                .map(|d| d.latest)
                .unwrap_or(0),
            v0_response
                .version
                .as_ref()
                .and_then(|v| v.protocol.as_ref())
                .and_then(|p| p.drive.as_ref())
                .map(|d| d.current)
                .unwrap_or(0),
        );

        let protocol = StatusProtocolWasm::new(tenderdash_protocol, drive_protocol);
        let version = StatusVersionWasm::new(software, protocol);

        let node = StatusNodeWasm::new(
            v0_response
                .node
                .as_ref()
                .map(|n| hex::encode(&n.id))
                .unwrap_or_else(|| "unknown".to_string()),
            v0_response
                .node
                .as_ref()
                .and_then(|n| n.pro_tx_hash.as_ref())
                .map(hex::encode),
        );

        let chain = StatusChainWasm::new(
            v0_response
                .chain
                .as_ref()
                .map(|c| c.catching_up)
                .unwrap_or(false),
            v0_response
                .chain
                .as_ref()
                .map(|c| hex::encode(&c.latest_block_hash))
                .unwrap_or_else(|| "unknown".to_string()),
            v0_response
                .chain
                .as_ref()
                .map(|c| hex::encode(&c.latest_app_hash))
                .unwrap_or_else(|| "unknown".to_string()),
            v0_response
                .chain
                .as_ref()
                .map(|c| c.latest_block_height.to_string())
                .unwrap_or_else(|| "0".to_string()),
            v0_response
                .chain
                .as_ref()
                .map(|c| hex::encode(&c.earliest_block_hash))
                .unwrap_or_else(|| "unknown".to_string()),
            v0_response
                .chain
                .as_ref()
                .map(|c| hex::encode(&c.earliest_app_hash))
                .unwrap_or_else(|| "unknown".to_string()),
            v0_response
                .chain
                .as_ref()
                .map(|c| c.earliest_block_height.to_string())
                .unwrap_or_else(|| "0".to_string()),
            v0_response
                .chain
                .as_ref()
                .map(|c| c.max_peer_block_height.to_string())
                .unwrap_or_else(|| "0".to_string()),
            v0_response
                .chain
                .as_ref()
                .and_then(|c| c.core_chain_locked_height),
        );

        let network = StatusNetworkWasm::new(
            v0_response
                .network
                .as_ref()
                .map(|n| n.chain_id.clone())
                .unwrap_or_else(|| "unknown".to_string()),
            v0_response
                .network
                .as_ref()
                .map(|n| n.peers_count)
                .unwrap_or(0),
            v0_response
                .network
                .as_ref()
                .map(|n| n.listening)
                .unwrap_or(false),
        );

        let state_sync = StatusStateSyncWasm {
            total_synced_time: v0_response
                .state_sync
                .as_ref()
                .map(|s| s.total_synced_time.to_string())
                .unwrap_or_else(|| "0".to_string()),
            remaining_time: v0_response
                .state_sync
                .as_ref()
                .map(|s| s.remaining_time.to_string())
                .unwrap_or_else(|| "0".to_string()),
            total_snapshots: v0_response
                .state_sync
                .as_ref()
                .map(|s| s.total_snapshots)
                .unwrap_or(0),
            chunk_process_avg_time: v0_response
                .state_sync
                .as_ref()
                .map(|s| s.chunk_process_avg_time.to_string())
                .unwrap_or_else(|| "0".to_string()),
            snapshot_height: v0_response
                .state_sync
                .as_ref()
                .map(|s| s.snapshot_height.to_string())
                .unwrap_or_else(|| "0".to_string()),
            snapshot_chunks_count: v0_response
                .state_sync
                .as_ref()
                .map(|s| s.snapshot_chunks_count.to_string())
                .unwrap_or_else(|| "0".to_string()),
            backfilled_blocks: v0_response
                .state_sync
                .as_ref()
                .map(|s| s.backfilled_blocks.to_string())
                .unwrap_or_else(|| "0".to_string()),
            backfill_blocks_total: v0_response
                .state_sync
                .as_ref()
                .map(|s| s.backfill_blocks_total.to_string())
                .unwrap_or_else(|| "0".to_string()),
        };

        let time = StatusTimeWasm::new(
            v0_response
                .time
                .as_ref()
                .map(|t| t.local.to_string())
                .unwrap_or_else(|| "0".to_string()),
            v0_response
                .time
                .as_ref()
                .and_then(|t| t.block)
                .map(|b| b.to_string()),
            v0_response
                .time
                .as_ref()
                .and_then(|t| t.genesis)
                .map(|g| g.to_string()),
            v0_response.time.as_ref().and_then(|t| t.epoch),
        );

        Ok(StatusResponseWasm::new(
            version, node, chain, network, state_sync, time,
        ))
    }

    #[wasm_bindgen(js_name = "getCurrentQuorumsInfo")]
    pub async fn get_current_quorums_info(&self) -> Result<CurrentQuorumsInfoWasm, WasmSdkError> {
        use dash_sdk::platform::FetchUnproved;
        use drive_proof_verifier::types::{
            CurrentQuorumsInfo as SdkCurrentQuorumsInfo, NoParamQuery,
        };

        let quorums_result =
            SdkCurrentQuorumsInfo::fetch_unproved(self.as_ref(), NoParamQuery {}).await?;

        // The result is Option<CurrentQuorumsInfo>
        if let Some(quorum_info) = quorums_result {
            // Convert the SDK response to our structure
            // Match quorum hashes with validator sets to get detailed information
            let quorums: Vec<QuorumInfoWasm> = quorum_info
                .quorum_hashes
                .into_iter()
                .map(|quorum_hash| {
                    // Try to find the corresponding validator set
                    let validator_set = quorum_info.validator_sets.iter().find(|vs| {
                        // Compare the quorum hash bytes directly

                        let vs_hash_bytes: &[u8] = vs.quorum_hash().as_ref();
                        vs_hash_bytes == &quorum_hash[..]
                    });

                    if let Some(vs) = validator_set {
                        let member_count = vs.members().len() as u32;

                        // Determine quorum type based on member count and quorum index
                        // This is an approximation based on common quorum sizes
                        // TODO: Get actual quorum type from the platform when available
                        let (quorum_type, threshold) = match member_count {
                            50..=70 => ("LLMQ_60_75".to_string(), (member_count * 75 / 100).max(1)),
                            90..=110 => {
                                ("LLMQ_100_67".to_string(), (member_count * 67 / 100).max(1))
                            }
                            350..=450 => {
                                ("LLMQ_400_60".to_string(), (member_count * 60 / 100).max(1))
                            }
                            _ => (
                                "LLMQ_TYPE_UNKNOWN".to_string(),
                                (member_count * 2 / 3).max(1),
                            ),
                        };

                        QuorumInfoWasm::new(
                            hex::encode(quorum_hash),
                            quorum_type,
                            member_count,
                            threshold,
                            true,
                        )
                    } else {
                        // No validator set found for this quorum hash
                        // TODO: This should not happen in normal circumstances. When the SDK
                        // provides complete quorum information, this fallback can be removed.
                        QuorumInfoWasm::new(
                            hex::encode(quorum_hash),
                            "LLMQ_TYPE_UNKNOWN".to_string(),
                            0,
                            0,
                            false,
                        )
                    }
                })
                .collect();

            Ok(CurrentQuorumsInfoWasm::new(
                quorums,
                quorum_info.last_platform_block_height,
            ))
        } else {
            // No quorum info available
            Ok(CurrentQuorumsInfoWasm::new(vec![], 0))
        }
    }

    #[wasm_bindgen(js_name = "getTotalCreditsInPlatform")]
    pub async fn get_total_credits_in_platform(&self) -> Result<BigInt, WasmSdkError> {
        use dash_sdk::platform::Fetch;
        use drive_proof_verifier::types::{
            NoParamQuery, TotalCreditsInPlatform as TotalCreditsQuery,
        };

        let total_credits_result = TotalCreditsQuery::fetch(self.as_ref(), NoParamQuery {}).await?;

        // TotalCreditsInPlatform is likely a newtype wrapper around u64
        let credits_value = if let Some(credits) = total_credits_result {
            // Extract the inner value - assuming it has a field or can be dereferenced
            // We'll try to access it as a tuple struct
            credits.0
        } else {
            0
        };

        Ok(BigInt::from(credits_value))
    }

    #[wasm_bindgen(js_name = "getPrefundedSpecializedBalance")]
    pub async fn get_prefunded_specialized_balance(
        &self,
        #[wasm_bindgen(js_name = "identityId")] identity_id: IdentifierLikeJs,
    ) -> Result<PrefundedSpecializedBalanceWasm, WasmSdkError> {
        use dash_sdk::platform::Fetch;
        use drive_proof_verifier::types::PrefundedSpecializedBalance as PrefundedBalance;

        let identity_identifier: Identifier = identity_id.try_into().map_err(|err| {
            WasmSdkError::invalid_argument(format!("Invalid identity ID: {}", err))
        })?;

        // Fetch prefunded specialized balance
        let balance_result = PrefundedBalance::fetch(self.as_ref(), identity_identifier).await?;

        let balance_value = balance_result.map(|b| b.0).unwrap_or(0);

        Ok(PrefundedSpecializedBalanceWasm::new(
            IdentifierWasm::from(identity_identifier),
            balance_value,
        ))
    }

    #[wasm_bindgen(js_name = "waitForStateTransitionResult")]
    pub async fn wait_for_state_transition_result(
        &self,
        #[wasm_bindgen(js_name = "stateTransitionHash")] state_transition_hash: &str,
    ) -> Result<StateTransitionResultWasm, WasmSdkError> {
        use dapi_grpc::platform::v0::wait_for_state_transition_result_request::{
            Version, WaitForStateTransitionResultRequestV0,
        };
        use dapi_grpc::platform::v0::WaitForStateTransitionResultRequest;

        use dash_sdk::RequestSettings;
        use rs_dapi_client::DapiRequestExecutor;

        // Parse the hash from hex string to bytes
        let hash_bytes = hex::decode(state_transition_hash).map_err(|e| {
            WasmSdkError::invalid_argument(format!("Invalid state transition hash: {}", e))
        })?;

        // Create the gRPC request
        let request = WaitForStateTransitionResultRequest {
            version: Some(Version::V0(WaitForStateTransitionResultRequestV0 {
                state_transition_hash: hash_bytes,
                prove: self.prove(),
            })),
        };

        // Execute the request
        let response = self
            .as_ref()
            .execute(request, RequestSettings::default())
            .await
            .map_err(|e| {
                WasmSdkError::generic(format!("Failed to wait for state transition result: {}", e))
            })?;

        // Parse the response
        use dapi_grpc::platform::v0::wait_for_state_transition_result_response::{
            wait_for_state_transition_result_response_v0::Result as V0Result,
            Version as ResponseVersion,
        };

        let (status, error) = match response.inner.version {
            Some(ResponseVersion::V0(v0)) => match v0.result {
                Some(V0Result::Error(e)) => {
                    let error_message = format!("Code: {}, Message: {}", e.code, e.message);
                    ("ERROR".to_string(), Some(error_message))
                }
                Some(V0Result::Proof(_)) => {
                    // State transition was successful
                    ("SUCCESS".to_string(), None)
                }
                None => (
                    "UNKNOWN".to_string(),
                    Some("No result returned".to_string()),
                ),
            },
            None => (
                "UNKNOWN".to_string(),
                Some("No version in response".to_string()),
            ),
        };

        Ok(StateTransitionResultWasm::new(
            state_transition_hash.to_string(),
            status,
            error,
        ))
    }

    #[wasm_bindgen(
        js_name = "getPathElements",
        unchecked_return_type = "Array<PathElement>"
    )]
    pub async fn get_path_elements(
        &self,
        #[wasm_bindgen(unchecked_param_type = "GrovePathSegment[]")] path: Array,
        #[wasm_bindgen(unchecked_param_type = "GrovePathSegment[]")] keys: Array,
    ) -> Result<Array, WasmSdkError> {
        use dash_sdk::platform::FetchMany;
        use drive_proof_verifier::types::{Elements, KeysInPath};

        let decoded_path = decode_js_path_inputs(&path, "path")?;
        let decoded_keys = decode_js_key_inputs(&keys, "keys")?;
        let path_bytes = decoded_bytes(&decoded_path);
        let key_bytes = decoded_bytes(&decoded_keys);

        // Create the query
        let query = KeysInPath {
            path: path_bytes.clone(),
            keys: key_bytes,
        };

        // Fetch path elements
        let path_elements_result: Elements = Element::fetch_many(self.as_ref(), query).await?;

        // Convert the result to our response format
        let elements_array = Array::new();
        for key in &decoded_keys {
            let path_element = path_elements_result
                .get(key.bytes.as_slice())
                .and_then(|element_opt| element_opt.as_ref())
                .map(|element| PathElementWasm::from_element(&path_bytes, key, element))
                .unwrap_or_else(|| PathElementWasm::missing(&path_bytes, key));

            elements_array.push(&JsValue::from(path_element));
        }

        Ok(elements_array)
    }

    // Proof versions for system queries

    #[wasm_bindgen(
        js_name = "getTotalCreditsInPlatformWithProofInfo",
        unchecked_return_type = "ProofMetadataResponseTyped<bigint | undefined>"
    )]
    pub async fn get_total_credits_in_platform_with_proof_info(
        &self,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        use dash_sdk::platform::Fetch;
        use drive_proof_verifier::types::{
            NoParamQuery, TotalCreditsInPlatform as TotalCreditsQuery,
        };

        let (total_credits_result, metadata, proof) =
            TotalCreditsQuery::fetch_with_metadata_and_proof(self.as_ref(), NoParamQuery {}, None)
                .await?;

        let data = total_credits_result
            .map(|credits| JsValue::from(BigInt::from(credits.0)))
            .unwrap_or(JsValue::UNDEFINED);

        Ok(ProofMetadataResponseWasm::from_sdk_parts(
            data, metadata, proof,
        ))
    }

    #[wasm_bindgen(
        js_name = "getPrefundedSpecializedBalanceWithProofInfo",
        unchecked_return_type = "ProofMetadataResponseTyped<PrefundedSpecializedBalance | undefined>"
    )]
    pub async fn get_prefunded_specialized_balance_with_proof_info(
        &self,
        #[wasm_bindgen(js_name = "identityId")] identity_id: IdentifierLikeJs,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        use dash_sdk::platform::Fetch;
        use drive_proof_verifier::types::PrefundedSpecializedBalance as PrefundedBalance;

        let identity_identifier: Identifier = identity_id.try_into().map_err(|err| {
            WasmSdkError::invalid_argument(format!("Invalid identity ID: {}", err))
        })?;

        // Fetch prefunded specialized balance with proof
        let (balance_result, metadata, proof) = PrefundedBalance::fetch_with_metadata_and_proof(
            self.as_ref(),
            identity_identifier,
            None,
        )
        .await?;

        let data = balance_result
            .map(|balance| {
                JsValue::from(PrefundedSpecializedBalanceWasm::new(
                    IdentifierWasm::from(identity_identifier),
                    balance.0,
                ))
            })
            .unwrap_or(JsValue::UNDEFINED);

        Ok(ProofMetadataResponseWasm::from_sdk_parts(
            data, metadata, proof,
        ))
    }

    #[wasm_bindgen(
        js_name = "getPathElementsWithProofInfo",
        unchecked_return_type = "ProofMetadataResponseTyped<Array<PathElement>>"
    )]
    pub async fn get_path_elements_with_proof_info(
        &self,
        #[wasm_bindgen(unchecked_param_type = "GrovePathSegment[]")] path: Array,
        #[wasm_bindgen(unchecked_param_type = "GrovePathSegment[]")] keys: Array,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        use dash_sdk::platform::FetchMany;
        use drive_proof_verifier::types::KeysInPath;

        let decoded_path = decode_js_path_inputs(&path, "path")?;
        let decoded_keys = decode_js_key_inputs(&keys, "keys")?;
        let path_bytes = decoded_bytes(&decoded_path);
        let key_bytes = decoded_bytes(&decoded_keys);

        // Create the query
        let query = KeysInPath {
            path: path_bytes.clone(),
            keys: key_bytes,
        };

        // Fetch path elements with proof
        let (path_elements_result, metadata, proof) =
            Element::fetch_many_with_metadata_and_proof(self.as_ref(), query, None).await?;

        let elements_array = Array::new();
        for key in &decoded_keys {
            let path_element = path_elements_result
                .get(key.bytes.as_slice())
                .and_then(|element_opt| element_opt.as_ref())
                .map(|element| PathElementWasm::from_element(&path_bytes, key, element))
                .unwrap_or_else(|| PathElementWasm::missing(&path_bytes, key));

            elements_array.push(&JsValue::from(path_element));
        }

        Ok(ProofMetadataResponseWasm::from_sdk_parts(
            elements_array,
            metadata,
            proof,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dash_sdk::drive::grovedb::element::reference_path::ReferencePathType;

    #[test]
    fn should_decode_raw_bytes_without_utf8_expansion() {
        let input = [0x80, 0xff];

        let path = decode_path_input(PathInputValue::Bytes(&input));
        let key = decode_key_input(PathInputValue::Bytes(&input));

        assert_eq!(path.bytes, input);
        assert_eq!(key.bytes, input);
        assert_eq!(path.legacy_path_segment, None);
        assert_eq!(key.legacy_path_segment, None);
    }

    #[test]
    fn should_decode_path_decimal_string_as_single_byte() {
        let path = decode_path_input(PathInputValue::String("96"));

        assert_eq!(path.bytes, vec![96]);
    }

    #[test]
    fn should_decode_key_string_as_utf8() {
        let key = decode_key_input(PathInputValue::String("96"));

        assert_eq!(key.bytes, vec![0x39, 0x36]);
        assert_eq!(key.legacy_path_segment.as_deref(), Some("96"));
    }

    #[test]
    fn should_preserve_utf8_numeric_key_bytes_in_legacy_path_segment() {
        let key = decode_key_input(PathInputValue::Bytes(b"96"));

        assert_eq!(key.bytes, vec![0x39, 0x36]);
        assert_eq!(key.legacy_path_segment.as_deref(), Some("96"));
    }

    #[test]
    fn should_preserve_non_ascii_utf8_key_bytes_in_legacy_path_segment() {
        let bytes = "café".as_bytes();
        let key = decode_key_input(PathInputValue::Bytes(bytes));

        assert_eq!(key.bytes, bytes);
        assert_eq!(key.legacy_path_segment.as_deref(), Some("café"));
    }

    #[test]
    fn should_preserve_non_ascii_utf8_path_bytes_in_legacy_path_segment() {
        let bytes = "café".as_bytes();
        let path = decode_path_input(PathInputValue::Bytes(bytes));

        assert_eq!(path.bytes, bytes);
        assert_eq!(path.legacy_path_segment.as_deref(), Some("café"));
    }

    #[test]
    fn should_drop_legacy_path_segment_for_ambiguous_numeric_path_bytes() {
        let path = decode_path_input(PathInputValue::Bytes(b"96"));

        assert_eq!(path.bytes, b"96".to_vec());
        assert_eq!(path.legacy_path_segment, None);
    }

    #[test]
    fn should_convert_item_element_with_compatible_value_and_bytes() {
        let parent_path = vec![vec![1]];
        let key = decode_key_input(PathInputValue::String("key"));
        let element = Element::Item(b"value".to_vec(), None);

        let path_element = PathElementWasm::from_element(&parent_path, &key, &element);

        assert_eq!(path_element.value.as_deref(), Some("dmFsdWU="));
        assert_eq!(path_element.value_bytes, Some(b"value".to_vec()));
        assert_eq!(path_element.element_type.as_deref(), Some("item"));
        assert_eq!(path_element.key, b"key".to_vec());
        assert_eq!(path_element.path_bytes, vec![vec![1], b"key".to_vec()]);
    }

    #[test]
    fn should_convert_tree_elements_without_value_bytes() {
        let parent_path = vec![vec![1]];
        let key = decode_key_input(PathInputValue::String("subtree"));
        let tree = Element::Tree(None, None);
        let sum_tree = Element::SumTree(None, 42, None);

        let tree_element = PathElementWasm::from_element(&parent_path, &key, &tree);
        let sum_tree_element = PathElementWasm::from_element(&parent_path, &key, &sum_tree);

        assert_eq!(tree_element.value, None);
        assert_eq!(tree_element.value_bytes, None);
        assert_eq!(tree_element.element_type.as_deref(), Some("tree"));
        assert_eq!(sum_tree_element.value, None);
        assert_eq!(sum_tree_element.value_bytes, None);
        assert_eq!(sum_tree_element.element_type.as_deref(), Some("sumTree"));
        assert_eq!(sum_tree_element.sum, Some(42));
    }

    #[test]
    fn should_convert_sum_item_and_reference_metadata() {
        let parent_path = vec![b"parent".to_vec()];
        let key = decode_key_input(PathInputValue::String("key"));
        let sum_item = Element::SumItem(-5, None);
        let reference = Element::Reference(
            ReferencePathType::SiblingReference(b"other".to_vec()),
            None,
            None,
        );
        let reference_with_sum = Element::ReferenceWithSumItem(
            ReferencePathType::SiblingReference(b"weighted".to_vec()),
            None,
            7,
            None,
        );
        let provable_sum_tree = Element::ProvableSumTree(None, 9, None);
        let not_counted_or_summed =
            Element::NotCountedOrSummed(Box::new(Element::ProvableSumTree(None, 11, None)));

        let sum_element = PathElementWasm::from_element(&parent_path, &key, &sum_item);
        let reference_element = PathElementWasm::from_element(&parent_path, &key, &reference);
        let reference_with_sum_element =
            PathElementWasm::from_element(&parent_path, &key, &reference_with_sum);
        let provable_sum_tree_element =
            PathElementWasm::from_element(&parent_path, &key, &provable_sum_tree);
        let not_counted_or_summed_element =
            PathElementWasm::from_element(&parent_path, &key, &not_counted_or_summed);

        assert_eq!(sum_element.element_type.as_deref(), Some("sumItem"));
        assert_eq!(sum_element.sum, Some(-5));
        assert_eq!(reference_element.element_type.as_deref(), Some("reference"));
        assert_eq!(
            reference_element.reference_target,
            Some(vec![b"parent".to_vec(), b"other".to_vec()])
        );
        assert_eq!(reference_element.reference_target_error, None);
        assert_eq!(
            reference_with_sum_element.element_type.as_deref(),
            Some("referenceWithSumItem")
        );
        assert_eq!(reference_with_sum_element.sum, Some(7));
        assert_eq!(
            reference_with_sum_element.reference_target,
            Some(vec![b"parent".to_vec(), b"weighted".to_vec()])
        );
        assert_eq!(
            provable_sum_tree_element.element_type.as_deref(),
            Some("provableSumTree")
        );
        assert_eq!(provable_sum_tree_element.sum, Some(9));
        assert_eq!(
            not_counted_or_summed_element.element_type.as_deref(),
            Some("notCountedOrSummedProvableSumTree")
        );
        assert_eq!(not_counted_or_summed_element.sum, Some(11));
    }

    #[test]
    fn should_report_reference_resolution_errors() {
        let parent_path = vec![b"parent".to_vec()];
        let key = decode_key_input(PathInputValue::String("key"));
        let invalid_reference = Element::Reference(
            ReferencePathType::UpstreamRootHeightReference(2, vec![b"target".to_vec()]),
            None,
            None,
        );

        let reference_element =
            PathElementWasm::from_element(&parent_path, &key, &invalid_reference);

        assert_eq!(reference_element.element_type.as_deref(), Some("reference"));
        assert_eq!(reference_element.reference_target, None);
        assert!(reference_element.reference_target_error.is_some());
    }

    #[test]
    fn should_map_supported_grovedb_element_types() {
        let reference_path = || ReferencePathType::SiblingReference(b"target".to_vec());
        let cases = vec![
            (Element::Item(vec![1], None), "item"),
            (
                Element::Reference(reference_path(), None, None),
                "reference",
            ),
            (Element::Tree(None, None), "tree"),
            (Element::SumItem(1, None), "sumItem"),
            (Element::SumTree(None, 1, None), "sumTree"),
            (Element::BigSumTree(None, 1, None), "bigSumTree"),
            (Element::CountTree(None, 1, None), "countTree"),
            (Element::CountSumTree(None, 1, 2, None), "countSumTree"),
            (
                Element::ProvableCountTree(None, 1, None),
                "provableCountTree",
            ),
            (
                Element::ItemWithSumItem(vec![1], 2, None),
                "itemWithSumItem",
            ),
            (
                Element::ReferenceWithSumItem(reference_path(), None, 2, None),
                "referenceWithSumItem",
            ),
            (
                Element::ProvableCountSumTree(None, 1, 2, None),
                "provableCountSumTree",
            ),
            (Element::ProvableSumTree(None, 2, None), "provableSumTree"),
            (Element::CommitmentTree(1, 2, None), "commitmentTree"),
            (Element::MmrTree(1, None), "mmrTree"),
            (Element::BulkAppendTree(1, 2, None), "bulkAppendTree"),
            (
                Element::DenseAppendOnlyFixedSizeTree(1, 2, None),
                "denseAppendOnlyFixedSizeTree",
            ),
            (
                Element::PrivateDocumentStore(0, 32, 2, None),
                "privateDocumentStore",
            ),
            (
                Element::NonCounted(Box::new(Element::PrivateDocumentStore(0, 32, 2, None))),
                "nonCountedPrivateDocumentStore",
            ),
            (
                Element::NonCounted(Box::new(Element::ReferenceWithSumItem(
                    reference_path(),
                    None,
                    2,
                    None,
                ))),
                "nonCountedReferenceWithSumItem",
            ),
            (
                Element::NonCounted(Box::new(Element::ProvableSumTree(None, 2, None))),
                "nonCountedProvableSumTree",
            ),
            (
                Element::NotSummed(Box::new(Element::ProvableSumTree(None, 2, None))),
                "notSummedProvableSumTree",
            ),
            (
                Element::NotCountedOrSummed(Box::new(Element::ProvableSumTree(None, 2, None))),
                "notCountedOrSummedProvableSumTree",
            ),
            (
                Element::ProvableSumIndexedTree(None, None, 2, None),
                "provableSumIndexedTree",
            ),
            (
                Element::ProvableCountIndexedTree(None, None, 1, None),
                "provableCountIndexedTree",
            ),
            (
                Element::ProvableCountProvableSumIndexedTree(None, 1, 2, vec![(0, None)], None),
                "provableCountProvableSumIndexedTree",
            ),
            (
                Element::NonCounted(Box::new(Element::ProvableSumIndexedTree(
                    None, None, 2, None,
                ))),
                "nonCountedProvableSumIndexedTree",
            ),
            (
                Element::NonCounted(Box::new(Element::ProvableCountIndexedTree(
                    None, None, 1, None,
                ))),
                "nonCountedProvableCountIndexedTree",
            ),
            (
                Element::NonCounted(Box::new(Element::ProvableCountProvableSumIndexedTree(
                    None,
                    1,
                    2,
                    vec![(0, None)],
                    None,
                ))),
                "nonCountedProvableCountProvableSumIndexedTree",
            ),
        ];

        let union = typescript_element_type_union();
        for (element, expected_type) in cases {
            assert_eq!(element_type_name(&element), expected_type);
            // The hand-written `GroveElementType` union is what TypeScript
            // callers narrow on, so a name the getter can emit but the union
            // does not list is a silently un-narrowable value.
            assert!(
                union.contains(&format!("\"{expected_type}\"")),
                "`{expected_type}` is emitted by the elementType getter but missing from \
                 the GroveElementType TypeScript union"
            );
        }
    }

    /// The `GroveElementType` union, read back out of this file's own
    /// source. `#[wasm_bindgen(typescript_custom_section)]` consumes the
    /// const it is attached to on non-wasm targets, so there is no binding
    /// left for a host test to name — the declaration text is the only
    /// thing available, and it is the thing under test anyway.
    fn typescript_element_type_union() -> String {
        const DECLARATION: &str = "export type GroveElementType =";
        let source = include_str!("system.rs");
        let start = source
            .find(DECLARATION)
            .expect("the TypeScript custom section declares GroveElementType");
        let rest = &source[start..];
        let end = rest
            .find(';')
            .expect("the GroveElementType declaration is `;`-terminated");
        rest[..end].to_string()
    }

    /// Indexed trees carry the same aggregate their non-indexed counterparts
    /// do; the secondaries only order it. Projecting `sum` off them is what
    /// keeps `pathElement.sum` meaningful once a contract declares a ranked
    /// index. `ProvableCountIndexedTree` has no sum to project.
    #[test]
    fn should_project_the_sum_of_indexed_trees() {
        assert_eq!(
            element_sum(&Element::ProvableSumIndexedTree(None, None, -7, None)),
            Some(-7)
        );
        assert_eq!(
            element_sum(&Element::ProvableCountProvableSumIndexedTree(
                None,
                3,
                42,
                vec![(0, None)],
                None
            )),
            Some(42)
        );
        assert_eq!(
            element_sum(&Element::ProvableCountIndexedTree(None, None, 9, None)),
            None,
            "a count-indexed tree has a count, not a sum"
        );
        assert_eq!(
            element_sum(&Element::NonCounted(Box::new(
                Element::ProvableSumIndexedTree(None, None, 5, None)
            ))),
            Some(5),
            "the wrappers look through to the inner element"
        );
    }
}
