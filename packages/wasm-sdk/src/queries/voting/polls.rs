use crate::sdk::WasmSdk;
use crate::{ProofMetadataResponseWasm, WasmSdkError};
use dash_sdk::dpp::prelude::{TimestampIncluded, TimestampMillis};
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

fn convert_limit(limit: Option<u32>, field: &str) -> Result<Option<u16>, WasmSdkError> {
    match limit {
        Some(0) => Ok(None),
        Some(value) => {
            if value > u16::MAX as u32 {
                return Err(WasmSdkError::invalid_argument(format!(
                    "{} {} exceeds maximum of {}",
                    field,
                    value,
                    u16::MAX
                )));
            }

            Ok(Some(value as u16))
        }
        None => Ok(None),
    }
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct VotePollsByEndDateQueryOptions {
    start_time_ms: Option<f64>,
    start_time_included: Option<bool>,
    end_time_ms: Option<f64>,
    end_time_included: Option<bool>,
    limit: Option<u32>,
    offset: Option<u32>,
    order_ascending: Option<bool>,
}

#[wasm_bindgen(js_name = "VotePollsByEndDateQuery")]
pub struct VotePollsByEndDateQueryWasm(VotePollsByEndDateDriveQuery);

impl VotePollsByEndDateQueryWasm {
    pub(crate) fn into_inner(self) -> VotePollsByEndDateDriveQuery {
        self.0
    }
}

#[wasm_bindgen(js_name = "VotePollsByEndDateQueryBuilder")]
pub struct VotePollsByEndDateQueryBuilder {
    start_time: Option<(TimestampMillis, TimestampIncluded)>,
    end_time: Option<(TimestampMillis, TimestampIncluded)>,
    limit: Option<u16>,
    offset: Option<u16>,
    order_ascending: bool,
}

#[wasm_bindgen(js_class = VotePollsByEndDateQueryBuilder)]
impl VotePollsByEndDateQueryBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new() -> VotePollsByEndDateQueryBuilder {
        Self {
            start_time: None,
            end_time: None,
            limit: None,
            offset: None,
            order_ascending: true,
        }
    }

    #[wasm_bindgen(js_name = "withStartTime")]
    pub fn with_start_time(
        mut self,
        timestamp_ms: Option<f64>,
        included: bool,
    ) -> Result<VotePollsByEndDateQueryBuilder, WasmSdkError> {
        self.start_time = timestamp_from_option(timestamp_ms, "startTimeMs")?
            .map(|timestamp| (timestamp, included));
        Ok(self)
    }

    #[wasm_bindgen(js_name = "withEndTime")]
    pub fn with_end_time(
        mut self,
        timestamp_ms: Option<f64>,
        included: bool,
    ) -> Result<VotePollsByEndDateQueryBuilder, WasmSdkError> {
        self.end_time = timestamp_from_option(timestamp_ms, "endTimeMs")?
            .map(|timestamp| (timestamp, included));
        Ok(self)
    }

    #[wasm_bindgen(js_name = "withLimit")]
    pub fn with_limit(
        mut self,
        limit: Option<u32>,
    ) -> Result<VotePollsByEndDateQueryBuilder, WasmSdkError> {
        self.limit = convert_limit(limit, "limit")?;
        Ok(self)
    }

    #[wasm_bindgen(js_name = "withOffset")]
    pub fn with_offset(
        mut self,
        offset: Option<u32>,
    ) -> Result<VotePollsByEndDateQueryBuilder, WasmSdkError> {
        self.offset = convert_limit(offset, "offset")?;
        Ok(self)
    }

    #[wasm_bindgen(js_name = "withOrderAscending")]
    pub fn with_order_ascending(mut self, ascending: bool) -> VotePollsByEndDateQueryBuilder {
        self.order_ascending = ascending;
        self
    }

    #[wasm_bindgen(js_name = "build")]
    pub fn build(self) -> VotePollsByEndDateQueryWasm {
        let VotePollsByEndDateQueryBuilder {
            start_time,
            end_time,
            limit,
            offset,
            order_ascending,
        } = self;

        VotePollsByEndDateQueryWasm(VotePollsByEndDateDriveQuery {
            start_time,
            end_time,
            limit,
            offset,
            order_ascending,
        })
    }
}

fn build_query_from_options(
    opts: VotePollsByEndDateQueryOptions,
) -> Result<VotePollsByEndDateQueryWasm, WasmSdkError> {
    let VotePollsByEndDateQueryOptions {
        start_time_ms,
        start_time_included,
        end_time_ms,
        end_time_included,
        limit,
        offset,
        order_ascending,
    } = opts;

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

    let mut builder = VotePollsByEndDateQueryBuilder::new();

    if start_time_ms.is_some() || start_time_included.is_some() {
        builder = builder.with_start_time(start_time_ms, start_time_included.unwrap_or(true))?;
    }

    if end_time_ms.is_some() || end_time_included.is_some() {
        builder = builder.with_end_time(end_time_ms, end_time_included.unwrap_or(true))?;
    }

    if limit.is_some() {
        builder = builder.with_limit(limit)?;
    }

    if offset.is_some() {
        builder = builder.with_offset(offset)?;
    }

    if let Some(order) = order_ascending {
        builder = builder.with_order_ascending(order);
    }

    Ok(builder.build())
}

#[wasm_bindgen(js_name = "buildVotePollsByEndDateQuery")]
pub fn build_vote_polls_by_end_date_query(
    options: JsValue,
) -> Result<VotePollsByEndDateQueryWasm, WasmSdkError> {
    let opts = if options.is_null() || options.is_undefined() {
        VotePollsByEndDateQueryOptions::default()
    } else {
        serde_wasm_bindgen::from_value(options).map_err(|err| {
            WasmSdkError::invalid_argument(format!(
                "Invalid vote polls by end date options: {}",
                err
            ))
        })?
    };

    build_query_from_options(opts)
}

#[derive(Clone)]
#[wasm_bindgen(js_name = "VotePollsByEndDateEntry")]
pub struct VotePollsByEndDateEntryWasm {
    timestamp_ms: TimestampMillis,
    polls_js: Rc<Array>,
}

impl VotePollsByEndDateEntryWasm {
    fn new(timestamp_ms: TimestampMillis, polls: Vec<VotePollWasm>) -> Self {
        let array = Array::new();
        for poll in polls {
            array.push(&JsValue::from(poll));
        }

        Self {
            timestamp_ms,
            polls_js: Rc::new(array),
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
        self.polls_js.as_ref().clone()
    }
}

#[derive(Clone)]
#[wasm_bindgen(js_name = "VotePollsByEndDateResult")]
pub struct VotePollsByEndDateResultWasm {
    entries: Vec<VotePollsByEndDateEntryWasm>,
    entries_js: Rc<Array>,
}

impl VotePollsByEndDateResultWasm {
    fn new(entries: Vec<VotePollsByEndDateEntryWasm>) -> Self {
        let array = Array::new();
        for entry in &entries {
            array.push(&JsValue::from(entry.clone()));
        }

        Self {
            entries,
            entries_js: Rc::new(array),
        }
    }
}

#[wasm_bindgen(js_class = VotePollsByEndDateResult)]
impl VotePollsByEndDateResultWasm {
    #[wasm_bindgen(getter = entries)]
    pub fn entries(&self) -> Array {
        self.entries_js.as_ref().clone()
    }

    #[wasm_bindgen(js_name = "isEmpty")]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl From<VotePollsGroupedByTimestamp> for VotePollsByEndDateResultWasm {
    fn from(grouped: VotePollsGroupedByTimestamp) -> Self {
        let entries = grouped
            .into_iter()
            .map(|(timestamp, polls)| {
                let poll_wrappers = polls.into_iter().map(VotePollWasm::from).collect();
                VotePollsByEndDateEntryWasm::new(timestamp, poll_wrappers)
            })
            .collect();

        VotePollsByEndDateResultWasm::new(entries)
    }
}

#[wasm_bindgen]
impl WasmSdk {
    async fn fetch_vote_polls_by_end_date(
        &self,
        query: VotePollsByEndDateQueryWasm,
    ) -> Result<VotePollsByEndDateResultWasm, WasmSdkError> {
        let polls = VotePoll::fetch_many(self.as_ref(), query.into_inner()).await?;
        Ok(polls.into())
    }

    #[wasm_bindgen(js_name = "getVotePollsByEndDate")]
    pub async fn get_vote_polls_by_end_date(
        &self,
        options: JsValue,
    ) -> Result<VotePollsByEndDateResultWasm, WasmSdkError> {
        let query = build_vote_polls_by_end_date_query(options)?;
        self.fetch_vote_polls_by_end_date(query).await
    }

    #[wasm_bindgen(js_name = "getVotePollsByEndDateWithQuery")]
    pub async fn get_vote_polls_by_end_date_with_query(
        &self,
        query: VotePollsByEndDateQueryWasm,
    ) -> Result<VotePollsByEndDateResultWasm, WasmSdkError> {
        self.fetch_vote_polls_by_end_date(query).await
    }

    #[wasm_bindgen(js_name = "getVotePollsByEndDateWithProofInfo")]
    pub async fn get_vote_polls_by_end_date_with_proof_info(
        &self,
        options: JsValue,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        let query = build_vote_polls_by_end_date_query(options)?;
        self.get_vote_polls_by_end_date_with_proof_info_query(query)
            .await
    }

    #[wasm_bindgen(js_name = "getVotePollsByEndDateWithProofInfoQuery")]
    pub async fn get_vote_polls_by_end_date_with_proof_info_query(
        &self,
        query: VotePollsByEndDateQueryWasm,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        let (polls, metadata, proof) =
            VotePoll::fetch_many_with_metadata_and_proof(self.as_ref(), query.into_inner(), None)
                .await?;

        let result = VotePollsByEndDateResultWasm::from(polls);

        Ok(ProofMetadataResponseWasm::from_parts(
            JsValue::from(result),
            metadata.into(),
            proof.into(),
        ))
    }
}
