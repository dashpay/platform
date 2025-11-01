use crate::queries::utils::{convert_optional_limit, deserialize_query_with_default};
use crate::sdk::WasmSdk;
use crate::{ProofMetadataResponseWasm, WasmSdkError};
use dash_sdk::dpp::prelude::TimestampMillis;
use dash_sdk::dpp::voting::vote_polls::VotePoll;
use dash_sdk::platform::FetchMany;
use drive::query::VotePollsByEndDateDriveQuery;
use drive_proof_verifier::types::VotePollsGroupedByTimestamp;
use js_sys::{Array, BigInt};
use serde::Deserialize;
use std::rc::Rc;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;
use wasm_dpp2::VotePollWasm;

#[wasm_bindgen(typescript_custom_section)]
const VOTE_POLLS_BY_END_DATE_QUERY_TS: &'static str = r#"
/**
 * Query parameters for retrieving vote polls grouped by end date.
 */
export interface VotePollsByEndDateQuery {
  /**
   * Starting timestamp (milliseconds) to filter polls.
   * @default undefined
   */
  startTimeMs?: number;

  /**
   * Include the `startTimeMs` boundary when true.
   * @default true
   */
  startTimeIncluded?: boolean;

  /**
   * Ending timestamp (milliseconds) to filter polls.
   * @default undefined
   */
  endTimeMs?: number;

  /**
   * Include the `endTimeMs` boundary when true.
   * @default true
   */
  endTimeIncluded?: boolean;

  /**
   * Maximum number of buckets to return.
   * @default undefined (no explicit limit)
   */
  limit?: number;

  /**
   * Offset into the paginated result set.
   * @default undefined
   */
  offset?: number;

  /**
   * Sort order for timestamps; ascending by default.
   * @default true
   */
  orderAscending?: boolean;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "VotePollsByEndDateQuery")]
    pub type VotePollsByEndDateQueryJs;
}

fn timestamp_from_option(
    value: Option<f64>,
    field: &str,
) -> Result<Option<TimestampMillis>, WasmSdkError> {
    match value {
        Some(raw) => {
            if !raw.is_finite() || raw < 0.0 {
                return Err(WasmSdkError::invalid_argument(format!(
                    "{} must be a non-negative finite number",
                    field
                )));
            }

            if raw.fract() != 0.0 {
                return Err(WasmSdkError::invalid_argument(format!(
                    "{} must be an integer value",
                    field
                )));
            }

            let timestamp = raw as u64;
            Ok(Some(timestamp))
        }
        None => Ok(None),
    }
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct VotePollsByEndDateQueryInput {
    #[serde(default)]
    start_time_ms: Option<f64>,
    #[serde(default)]
    start_time_included: Option<bool>,
    #[serde(default)]
    end_time_ms: Option<f64>,
    #[serde(default)]
    end_time_included: Option<bool>,
    #[serde(default)]
    limit: Option<u32>,
    #[serde(default)]
    offset: Option<u32>,
    #[serde(default)]
    order_ascending: Option<bool>,
}

fn build_vote_polls_by_end_date_drive_query(
    input: VotePollsByEndDateQueryInput,
) -> Result<VotePollsByEndDateDriveQuery, WasmSdkError> {
    let VotePollsByEndDateQueryInput {
        start_time_ms,
        start_time_included,
        end_time_ms,
        end_time_included,
        limit,
        offset,
        order_ascending,
    } = input;

    if start_time_ms.is_none() && start_time_included.is_some() {
        return Err(WasmSdkError::invalid_argument(
            "startTimeIncluded provided without startTimeMs",
        ));
    }

    if end_time_ms.is_none() && end_time_included.is_some() {
        return Err(WasmSdkError::invalid_argument(
            "endTimeIncluded provided without endTimeMs",
        ));
    }

    let start_time = timestamp_from_option(start_time_ms, "startTimeMs")?
        .map(|timestamp| (timestamp, start_time_included.unwrap_or(true)));

    let end_time = timestamp_from_option(end_time_ms, "endTimeMs")?
        .map(|timestamp| (timestamp, end_time_included.unwrap_or(true)));

    let limit = convert_optional_limit(limit, "limit")?;
    let offset = convert_optional_limit(offset, "offset")?;

    Ok(VotePollsByEndDateDriveQuery {
        start_time,
        end_time,
        limit,
        offset,
        order_ascending: order_ascending.unwrap_or(true),
    })
}

fn parse_vote_polls_by_end_date_query(
    query: Option<VotePollsByEndDateQueryJs>,
) -> Result<VotePollsByEndDateDriveQuery, WasmSdkError> {
    let input: VotePollsByEndDateQueryInput =
        deserialize_query_with_default(query, "vote polls by end date query")?;

    build_vote_polls_by_end_date_drive_query(input)
}

#[derive(Clone)]
#[wasm_bindgen(js_name = "VotePollsByEndDateEntry")]
pub struct VotePollsByEndDateEntryWasm {
    timestamp_ms: TimestampMillis,
    polls: Rc<Array>,
}

impl VotePollsByEndDateEntryWasm {
    fn new(timestamp_ms: TimestampMillis, polls: Vec<VotePollWasm>) -> Self {
        let array = Array::new();
        for poll in polls {
            array.push(&JsValue::from(poll));
        }

        Self {
            timestamp_ms,
            polls: Rc::new(array),
        }
    }
}

#[wasm_bindgen(js_class = VotePollsByEndDateEntry)]
impl VotePollsByEndDateEntryWasm {
    #[wasm_bindgen(getter = timestampMs)]
    pub fn timestamp_ms(&self) -> BigInt {
        BigInt::from(self.timestamp_ms)
    }

    #[wasm_bindgen(getter = votePolls)]
    pub fn vote_polls(&self) -> Array {
        self.polls.as_ref().clone()
    }
}

fn vote_polls_grouped_to_entries(grouped: VotePollsGroupedByTimestamp) -> Array {
    let entries = Array::new();
    for (timestamp, polls) in grouped {
        let poll_wrappers = polls.into_iter().map(VotePollWasm::from).collect();
        let entry = VotePollsByEndDateEntryWasm::new(timestamp, poll_wrappers);
        entries.push(&JsValue::from(entry));
    }
    entries
}

#[wasm_bindgen]
impl WasmSdk {
    #[wasm_bindgen(js_name = "getVotePollsByEndDate")]
    pub async fn get_vote_polls_by_end_date(
        &self,
        query: Option<VotePollsByEndDateQueryJs>,
    ) -> Result<Array, WasmSdkError> {
        let drive_query = parse_vote_polls_by_end_date_query(query)?;
        let polls = VotePoll::fetch_many(self.as_ref(), drive_query).await?;
        Ok(vote_polls_grouped_to_entries(polls))
    }

    #[wasm_bindgen(js_name = "getVotePollsByEndDateWithProofInfo")]
    pub async fn get_vote_polls_by_end_date_with_proof_info(
        &self,
        query: Option<VotePollsByEndDateQueryJs>,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        let drive_query = parse_vote_polls_by_end_date_query(query)?;
        let (polls, metadata, proof) =
            VotePoll::fetch_many_with_metadata_and_proof(self.as_ref(), drive_query, None).await?;

        let entries = vote_polls_grouped_to_entries(polls);

        Ok(ProofMetadataResponseWasm::from_sdk_parts(
            entries, metadata, proof,
        ))
    }
}
