//! Epoch-related types and helpers
use async_trait::async_trait;
use dapi_grpc::platform::v0::{GetEpochsInfoRequest, Proof, ResponseMetadata};
use dpp::block::{
    epoch::{EpochIndex, MAX_EPOCH},
    extended_epoch_info::ExtendedEpochInfo,
};
use dpp::fee::epoch::GENESIS_EPOCH_INDEX;

use crate::platform::fetch_current_no_parameters::FetchCurrent;
use crate::{
    platform::{Fetch, LimitQuery, Query},
    Error, Sdk,
};

/// Epoch type used in the SDK.
pub type Epoch = ExtendedEpochInfo;

#[async_trait]
impl FetchCurrent for ExtendedEpochInfo {
    /// Fetch the current epoch.
    ///
    /// The proof verifier rejects proved descending epoch queries without an explicit
    /// start epoch: resolving "the last epoch" server-side would force verification to
    /// trust unsigned response metadata. An explicit descending start does not help by
    /// itself either — Drive pre-creates thousands of empty epoch trees above the
    /// current one, and a proved descending query starting inside that window provably
    /// returns nothing (the query limit is consumed by the empty trees).
    ///
    /// So the fetch is done in two proved, guard-compliant steps:
    ///
    /// 1. Probe: fetch the genesis epoch (explicit ascending start), and take the
    ///    current-epoch *hint* from the response metadata.
    /// 2. Fetch the newest started epoch at or below the hint (explicit descending
    ///    start).
    ///
    /// The hint only shapes the second request; proof verification stays a pure
    /// function of the request and quorum-signed state. A node inflating the hint
    /// lands the query in the provably-empty future window, yielding
    /// [`Error::EpochNotFound`] rather than a bogus epoch. A node deflating the hint
    /// can present a stale epoch as current — the same exposure as trusting the
    /// node's metadata directly, so treat the result accordingly.
    async fn fetch_current(sdk: &Sdk) -> Result<Self, Error> {
        let (epoch, _) = Self::fetch_current_with_metadata(sdk).await?;
        Ok(epoch)
    }

    async fn fetch_current_with_metadata(sdk: &Sdk) -> Result<(Self, ResponseMetadata), Error> {
        let (_, probe_metadata) =
            Self::fetch_with_metadata(sdk, current_epoch_probe_query(), None).await?;

        let hint = epoch_hint_from_metadata(&probe_metadata);
        let (epoch, metadata) =
            Self::fetch_with_metadata(sdk, current_epoch_query(hint), None).await?;

        Ok((epoch.ok_or(Error::EpochNotFound)?, metadata))
    }

    async fn fetch_current_with_metadata_and_proof(
        sdk: &Sdk,
    ) -> Result<(Self, ResponseMetadata, Proof), Error> {
        let (_, probe_metadata) =
            Self::fetch_with_metadata(sdk, current_epoch_probe_query(), None).await?;

        let hint = epoch_hint_from_metadata(&probe_metadata);
        let (epoch, metadata, proof) =
            Self::fetch_with_metadata_and_proof(sdk, current_epoch_query(hint), None).await?;

        Ok((epoch.ok_or(Error::EpochNotFound)?, metadata, proof))
    }
}

/// First step of [`ExtendedEpochInfo::fetch_current`]: a proved query with an
/// explicit start whose response metadata carries the current-epoch hint.
fn current_epoch_probe_query() -> LimitQuery<EpochQuery> {
    LimitQuery {
        query: EpochQuery::genesis(),
        limit: Some(1),
        start_info: None,
    }
}

/// Second step of [`ExtendedEpochInfo::fetch_current`]: fetch the newest started
/// epoch at or below the hinted index.
fn current_epoch_query(hint: EpochIndex) -> LimitQuery<EpochQuery> {
    LimitQuery {
        query: EpochQuery::newest_at_or_below(hint),
        limit: Some(1),
        start_info: None,
    }
}

/// Extract the current-epoch hint from response metadata, clamped to the range
/// Drive's epoch key encoding can express.
fn epoch_hint_from_metadata(metadata: &ResponseMetadata) -> EpochIndex {
    metadata.epoch.min(MAX_EPOCH as u32) as EpochIndex
}

/// Query used to fetch multiple epochs from Platform.
#[derive(Clone, Debug)]
pub struct EpochQuery {
    /// Starting number of epoch to fetch.
    ///
    /// It is first returned epoch in the set.
    ///
    /// Value of `None` has the following meaning:
    ///
    /// * if ascending is true, then it is the first epoch on Platform (eg. epoch 0).
    /// * if ascending is false, then it is the last epoch on Platform (eg. most recent epoch).
    ///   Note that proved descending queries without an explicit start are rejected by the
    ///   proof verifier (resolving "the last epoch" would require trusting unsigned response
    ///   metadata); use [`ExtendedEpochInfo::fetch_current`](crate::platform::fetch_current_no_parameters::FetchCurrent)
    ///   or [`EpochQuery::newest_at_or_below()`] instead.
    pub start: Option<EpochIndex>,
    /// Sort order. Default is ascending (true), which means that the first returned epoch is the oldest one.
    pub ascending: bool,
}

impl EpochQuery {
    /// Ascending query with an explicit start at the genesis epoch.
    ///
    /// Combined with `limit: Some(1)`, this is the cheapest proved epoch query with a
    /// fully request-derived range, used by [`ExtendedEpochInfo::fetch_current`] as a
    /// probe for the current-epoch hint in response metadata.
    pub fn genesis() -> Self {
        Self {
            start: Some(GENESIS_EPOCH_INDEX),
            ascending: true,
        }
    }

    /// Descending query with an explicit start, returning the newest started epochs at
    /// or below `start`.
    ///
    /// With `limit: Some(1)` and `start` set to the current epoch index, this returns
    /// the current epoch. Do not pass a `start` far above the current epoch: Drive
    /// pre-creates empty epoch trees ahead of the current one, and a proved descending
    /// query starting inside that empty window provably returns no epochs.
    pub fn newest_at_or_below(start: EpochIndex) -> Self {
        Self {
            start: Some(start),
            ascending: false,
        }
    }
}

impl Default for EpochQuery {
    fn default() -> Self {
        Self {
            start: None,
            ascending: true,
        }
    }
}

impl From<EpochIndex> for EpochQuery {
    fn from(start: EpochIndex) -> Self {
        Self {
            start: Some(start),
            ascending: true,
        }
    }
}

impl Query<GetEpochsInfoRequest> for EpochQuery {
    fn query(
        &self,
        settings: &crate::platform::QuerySettings<'_>,
    ) -> Result<GetEpochsInfoRequest, Error> {
        LimitQuery::from(self.clone()).query(settings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dapi_grpc::platform::v0::get_epochs_info_request;
    use dpp::block::epoch::EPOCH_KEY_OFFSET;
    use rs_dapi_client::RequestSettings;

    fn query_settings(request_settings: &RequestSettings) -> crate::platform::QuerySettings<'_> {
        crate::platform::QuerySettings {
            request_settings,
            protocol_version: dpp::version::PlatformVersion::latest(),
            prove: true,
        }
    }

    /// Both queries issued by `fetch_current` must satisfy the proof verifier's
    /// fail-closed guard: an explicit start epoch on every proved descending
    /// query, and starts that survive Drive's `+ EPOCH_KEY_OFFSET` key encoding
    /// without overflowing `u16`.
    #[test]
    fn should_build_current_epoch_queries_verifiable_without_metadata() {
        let request_settings = RequestSettings::default();
        let settings = query_settings(&request_settings);

        // Step 1: the probe is ascending with an explicit genesis start.
        let probe = current_epoch_probe_query()
            .query(&settings)
            .expect("build probe query");
        let get_epochs_info_request::Version::V0(probe_v0) = probe.version.expect("version");
        assert_eq!(probe_v0.start_epoch, Some(GENESIS_EPOCH_INDEX as u32));
        assert!(probe_v0.ascending);
        assert_eq!(probe_v0.count, 1);

        // Step 2: the fetch is descending with an explicit start at the hint.
        for hint in [0u32, 42, u32::MAX] {
            let metadata = dapi_grpc::platform::v0::ResponseMetadata {
                epoch: hint,
                ..Default::default()
            };
            let request = current_epoch_query(epoch_hint_from_metadata(&metadata))
                .query(&settings)
                .expect("build query");
            let get_epochs_info_request::Version::V0(v0) = request.version.expect("version");

            let start_epoch = v0.start_epoch.expect(
                "fetch_current must send an explicit start epoch, \
                 or proof verification rejects the descending query",
            );
            assert!(!v0.ascending);
            assert_eq!(v0.count, 1);
            assert_eq!(start_epoch, hint.min(MAX_EPOCH as u32));
            // Guard against overflow in Drive's prove/verify key encoding.
            u16::try_from(start_epoch)
                .expect("start epoch fits u16")
                .checked_add(EPOCH_KEY_OFFSET)
                .expect("start epoch must fit Drive's epoch key encoding");
        }
    }
}
