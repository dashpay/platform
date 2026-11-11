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
   * Starting timestamp (milliseconds) to filter polls. Accepts the
   * `timestampMs` bigint of a returned entry, for paging.
   * @default undefined
   */
  startTimeMs?: number | bigint;

  /**
   * Include the `startTimeMs` boundary when true.
   * @default true
   */
  startTimeIncluded?: boolean;

  /**
   * Ending timestamp (milliseconds) to filter polls.
   * @default undefined
   */
  endTimeMs?: number | bigint;

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

/// Timestamps are integers here, not `f64`: the query reaches serde through
/// `platform_value`, which turns a whole JS number (or a bigint) into an
/// integer `Value` and refuses to read one as `f64`. Negative and fractional
/// values are refused by the same conversion.
#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct VotePollsByEndDateQueryInput {
    #[serde(default)]
    start_time_ms: Option<TimestampMillis>,
    #[serde(default)]
    start_time_included: Option<bool>,
    #[serde(default)]
    end_time_ms: Option<TimestampMillis>,
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

    let start_time =
        start_time_ms.map(|timestamp| (timestamp, start_time_included.unwrap_or(true)));

    let end_time = end_time_ms.map(|timestamp| (timestamp, end_time_included.unwrap_or(true)));

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
    #[wasm_bindgen(
        js_name = "getVotePollsByEndDate",
        unchecked_return_type = "Array<VotePollsByEndDateEntry>"
    )]
    pub async fn get_vote_polls_by_end_date(
        &self,
        query: Option<VotePollsByEndDateQueryJs>,
    ) -> Result<Array, WasmSdkError> {
        let drive_query = parse_vote_polls_by_end_date_query(query)?;
        let polls = VotePoll::fetch_many(self.as_ref(), drive_query).await?;
        Ok(vote_polls_grouped_to_entries(polls))
    }

    #[wasm_bindgen(
        js_name = "getVotePollsByEndDateWithProofInfo",
        unchecked_return_type = "ProofMetadataResponseTyped<Array<VotePollsByEndDateEntry>>"
    )]
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

#[cfg(test)]
mod tests {
    use super::*;
    use dash_sdk::dpp::platform_value::{self, Value};

    /// The step `from_object` runs after `serde_wasm_bindgen` has turned the
    /// JS query into a `Value`: a whole JS number arrives as `Value::I64`, a
    /// bigint as `Value::I64` or `Value::U64`, a fractional number as
    /// `Value::Float`.
    fn parse(entries: Vec<(&str, Value)>) -> Result<VotePollsByEndDateDriveQuery, WasmSdkError> {
        let map = entries
            .into_iter()
            .map(|(key, value)| (Value::Text(key.to_string()), value))
            .collect();
        let input: VotePollsByEndDateQueryInput = platform_value::from_value(Value::Map(map))
            .map_err(|e| WasmSdkError::invalid_argument(e.to_string()))?;
        build_vote_polls_by_end_date_drive_query(input)
    }

    #[test]
    fn whole_number_and_bigint_timestamps_are_read() {
        let query = parse(vec![
            ("startTimeMs", Value::I64(1_727_500_000_000)),
            ("startTimeIncluded", Value::Bool(false)),
            ("endTimeMs", Value::U64(1_727_600_000_000)),
        ])
        .expect("integer timestamps should be accepted");

        assert_eq!(query.start_time, Some((1_727_500_000_000, false)));
        assert_eq!(query.end_time, Some((1_727_600_000_000, true)));
    }

    #[test]
    fn negative_and_fractional_timestamps_are_refused() {
        assert!(parse(vec![("startTimeMs", Value::I64(-1))]).is_err());
        assert!(parse(vec![("endTimeMs", Value::Float(1.5))]).is_err());
    }

    #[test]
    fn inclusion_flag_without_its_timestamp_is_refused() {
        assert!(parse(vec![("startTimeIncluded", Value::Bool(true))]).is_err());
        assert!(parse(vec![("endTimeIncluded", Value::Bool(false))]).is_err());
    }
}
