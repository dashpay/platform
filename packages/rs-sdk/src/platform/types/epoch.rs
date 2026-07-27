//! Epoch-related types and helpers
use async_trait::async_trait;
use dapi_grpc::platform::v0::{GetEpochsInfoRequest, Proof, ResponseMetadata};
use dpp::block::{
    epoch::{EpochIndex, MAX_EPOCH},
    extended_epoch_info::ExtendedEpochInfo,
};
use dpp::fee::epoch::GENESIS_EPOCH_INDEX;

use crate::error::StaleNodeError;
use crate::platform::fetch_current_no_parameters::FetchCurrent;
use crate::{
    platform::{Fetch, FetchMany, LimitQuery, Query},
    Error, Sdk,
};

/// Epoch type used in the SDK.
pub type Epoch = ExtendedEpochInfo;

/// How many times [`ExtendedEpochInfo::fetch_current`] repeats its confirmation
/// query after a proof shows that a newer epoch has already started.
///
/// One repeat absorbs the honest race where the epoch turns over between the
/// probe and the confirmation. Past that, the node keeps hinting an epoch that
/// its own proofs contradict, and the lookup fails closed.
const MAX_CURRENT_EPOCH_REFINEMENTS: usize = 2;

#[async_trait]
impl FetchCurrent for ExtendedEpochInfo {
    /// Fetch the current epoch.
    ///
    /// See [`fetch_current_with_metadata_and_proof`](ExtendedEpochInfo::fetch_current_with_metadata_and_proof)
    /// for how the current epoch is selected and authenticated.
    async fn fetch_current(sdk: &Sdk) -> Result<Self, Error> {
        let (epoch, _, _) = resolve_current_epoch(sdk).await?;
        Ok(epoch)
    }

    /// Fetch the current epoch together with the metadata of the response that
    /// authenticated it.
    ///
    /// See [`fetch_current_with_metadata_and_proof`](ExtendedEpochInfo::fetch_current_with_metadata_and_proof)
    /// for how the current epoch is selected and authenticated.
    async fn fetch_current_with_metadata(sdk: &Sdk) -> Result<(Self, ResponseMetadata), Error> {
        let (epoch, metadata, _) = resolve_current_epoch(sdk).await?;
        Ok((epoch, metadata))
    }

    /// Fetch the current epoch together with the metadata and proof that
    /// authenticated it.
    ///
    /// # Why this takes two queries
    ///
    /// The proof verifier rejects proved descending epoch queries without an
    /// explicit start epoch: resolving "the last epoch" server-side would force
    /// verification to trust unsigned response metadata. An explicit descending
    /// start does not help by itself either — Drive pre-creates thousands of
    /// empty epoch trees above the current one, and a proved descending query
    /// starting inside that window provably returns nothing (the query limit is
    /// consumed by the empty trees).
    ///
    /// So the fetch is done in two proved, guard-compliant steps:
    ///
    /// 1. **Probe**: fetch the genesis epoch (explicit ascending start), and take
    ///    the current-epoch *hint* from the response metadata.
    /// 2. **Confirm**: fetch two epochs ascending from the hint. Drive stores
    ///    epoch `n + 1` as an empty tree until epoch `n + 1` starts, and an empty
    ///    tree consumes query limit without contributing elements — so a
    ///    single-epoch result is *proof* that the hint is the newest started
    ///    epoch.
    ///
    /// # What is authenticated
    ///
    /// The metadata hint only shapes the second request; it is never trusted as
    /// an answer. The returned epoch and the absence of any epoch above it both
    /// come from the same quorum-signed GroveDB proof:
    ///
    /// * A hint **above** the current epoch lands in the pre-created empty window
    ///   and provably matches nothing — [`Error::EpochNotFound`].
    /// * A hint **below** the current epoch is contradicted by the node's own
    ///   proof, which then carries the newer epoch. The query is repeated from
    ///   that proven index (up to [`MAX_CURRENT_EPOCH_REFINEMENTS`] times, enough
    ///   for an epoch that turns over mid-fetch) and otherwise fails closed with
    ///   [`StaleNodeError::Epoch`].
    ///
    /// The returned proof is the one from the confirming query, so it covers both
    /// the epoch itself and the evidence that no later epoch has started.
    async fn fetch_current_with_metadata_and_proof(
        sdk: &Sdk,
    ) -> Result<(Self, ResponseMetadata, Proof), Error> {
        resolve_current_epoch(sdk).await
    }
}

/// Fetch the newest started epoch, proving that it is in fact the newest.
///
/// See [`ExtendedEpochInfo::fetch_current_with_metadata_and_proof`] for the
/// two-step protocol and its trust boundary.
async fn resolve_current_epoch(
    sdk: &Sdk,
) -> Result<(ExtendedEpochInfo, ResponseMetadata, Proof), Error> {
    let (_, probe_metadata) =
        ExtendedEpochInfo::fetch_with_metadata(sdk, current_epoch_probe_query(), None).await?;

    let hint = epoch_hint_from_metadata(&probe_metadata);
    let mut candidate = hint;

    for _ in 0..=MAX_CURRENT_EPOCH_REFINEMENTS {
        let (epochs, metadata, proof) = ExtendedEpochInfo::fetch_many_with_metadata_and_proof(
            sdk,
            current_epoch_confirmation_query(candidate),
            None,
        )
        .await?;

        let mut started = epochs
            .into_iter()
            .filter_map(|(index, info)| info.map(|info| (index, info)));

        // Nothing at or above the candidate has started: the query landed in
        // Drive's pre-created empty epoch window.
        let Some((index, info)) = started.next() else {
            return Err(Error::EpochNotFound);
        };

        match started.next() {
            // Only the candidate came back, so the proof also covers the epoch
            // above it and shows it as not started: the candidate is current.
            None if index == candidate => return Ok((info, metadata, proof)),
            // Epochs are initialized contiguously, so a gap at the candidate with
            // a started epoch above it cannot happen in a well-formed state.
            None => {
                return Err(Error::InvalidProvedResponse(format!(
                    "epoch {candidate} is not started but epoch {index} is; \
                     epochs must be initialized contiguously"
                )))
            }
            // The hint was below the chain tip; retry from the proven epoch.
            Some((newer_index, _)) => candidate = newer_index,
        }
    }

    Err(StaleNodeError::Epoch {
        hinted_epoch: hint,
        proven_epoch: candidate,
    }
    .into())
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

/// Second step of [`ExtendedEpochInfo::fetch_current`]: fetch `candidate` and the
/// epoch above it, so the proof shows whether `candidate` is the newest started
/// epoch.
fn current_epoch_confirmation_query(candidate: EpochIndex) -> LimitQuery<EpochQuery> {
    LimitQuery {
        query: EpochQuery::ascending_from(candidate),
        limit: Some(2),
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
    ///   instead.
    pub start: Option<EpochIndex>,
    /// Sort order. Default is ascending (true), which means that the first returned epoch is the oldest one.
    pub ascending: bool,
}

impl EpochQuery {
    /// Ascending query with an explicit start at `start`.
    ///
    /// The returned range is fully described by the request, so proved responses
    /// verify without consulting unsigned response metadata.
    pub fn ascending_from(start: EpochIndex) -> Self {
        Self {
            start: Some(start),
            ascending: true,
        }
    }

    /// Ascending query with an explicit start at the genesis epoch.
    ///
    /// Combined with `limit: Some(1)`, this is the cheapest proved epoch query with a
    /// fully request-derived range, used by [`ExtendedEpochInfo::fetch_current`] as a
    /// probe for the current-epoch hint in response metadata.
    pub fn genesis() -> Self {
        Self::ascending_from(GENESIS_EPOCH_INDEX)
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
        Self::ascending_from(start)
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
    /// fail-closed guard: an explicit start epoch on every proved query, and
    /// starts that survive Drive's `+ EPOCH_KEY_OFFSET` key encoding without
    /// overflowing `u16`.
    ///
    /// The confirmation query must also ask for *two* epochs: the second slot is
    /// what turns "here is epoch n" into "epoch n is the newest started epoch",
    /// because Drive's pre-created empty tree at `n + 1` consumes limit without
    /// returning elements.
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

        // Step 2: the confirmation is ascending from the hint, two epochs wide.
        for hint in [0u32, 42, u32::MAX] {
            let metadata = dapi_grpc::platform::v0::ResponseMetadata {
                epoch: hint,
                ..Default::default()
            };
            let request = current_epoch_confirmation_query(epoch_hint_from_metadata(&metadata))
                .query(&settings)
                .expect("build query");
            let get_epochs_info_request::Version::V0(v0) = request.version.expect("version");

            let start_epoch = v0.start_epoch.expect(
                "fetch_current must send an explicit start epoch, \
                 or proof verification rejects the query",
            );
            assert!(v0.ascending);
            assert_eq!(
                v0.count, 2,
                "the confirmation query must cover the epoch above the hint"
            );
            assert_eq!(start_epoch, hint.min(MAX_EPOCH as u32));
            // Guard against overflow in Drive's prove/verify key encoding.
            u16::try_from(start_epoch)
                .expect("start epoch fits u16")
                .checked_add(EPOCH_KEY_OFFSET)
                .expect("start epoch must fit Drive's epoch key encoding");
        }
    }
}

/// How `fetch_current` reacts to what the confirming proof says, with the
/// unsigned hint held at 0 (what the mock reports in response metadata).
#[cfg(all(test, feature = "mocks"))]
mod mock_tests {
    use super::*;
    use crate::SdkBuilder;
    use dpp::block::extended_epoch_info::v0::{ExtendedEpochInfoV0, ExtendedEpochInfoV0Getters};
    use drive_proof_verifier::types::ExtendedEpochInfos;

    fn epoch_at(index: EpochIndex) -> ExtendedEpochInfo {
        ExtendedEpochInfoV0 {
            index,
            first_block_time: 1_000 * index as u64,
            first_block_height: 10 * index as u64,
            first_core_block_height: index as u32,
            fee_multiplier_permille: 1000,
            protocol_version: dpp::version::LATEST_VERSION,
        }
        .into()
    }

    /// Mock SDK where epochs `0..=newest_started` have started.
    ///
    /// Each confirmation query is answered the way a real proof would answer it:
    /// with the (at most two) started epochs at or above its start. Answering
    /// with two is how the chain says "your start is stale"; answering with one
    /// is how it says "your start is the newest".
    async fn mock_sdk_with_started_epochs(newest_started: EpochIndex) -> Sdk {
        let mut sdk = SdkBuilder::new_mock().build().expect("build mock sdk");

        sdk.mock()
            .expect_fetch::<ExtendedEpochInfo, _>(
                current_epoch_probe_query(),
                Some(epoch_at(GENESIS_EPOCH_INDEX)),
            )
            .await
            .expect("register probe expectation");

        for candidate in 0..=newest_started {
            let started: ExtendedEpochInfos = (candidate..=newest_started)
                .take(2)
                .map(|index| (index, Some(epoch_at(index))))
                .collect();
            sdk.mock()
                .expect_fetch_many::<_, ExtendedEpochInfo, _, ExtendedEpochInfos>(
                    current_epoch_confirmation_query(candidate),
                    Some(started),
                )
                .await
                .expect("register confirmation expectation");
        }

        sdk
    }

    /// The hint is right: one epoch comes back, and that is the answer.
    #[tokio::test]
    async fn should_return_the_hinted_epoch_when_the_proof_confirms_it() {
        let sdk = mock_sdk_with_started_epochs(0).await;

        let epoch = ExtendedEpochInfo::fetch_current(&sdk)
            .await
            .expect("fetch current epoch");

        assert_eq!(epoch.index(), 0);
    }

    /// The hint is one epoch stale — an epoch turning over mid-fetch, or a node
    /// deflating it. The proof carries the newer epoch, so the query repeats
    /// from there and the newer epoch wins.
    #[tokio::test]
    async fn should_advance_past_a_hint_the_proof_shows_as_stale() {
        let sdk = mock_sdk_with_started_epochs(1).await;

        let epoch = ExtendedEpochInfo::fetch_current(&sdk)
            .await
            .expect("fetch current epoch");

        assert_eq!(
            epoch.index(),
            1,
            "the epoch proven to be newer must win over the hinted one"
        );
    }

    /// A node that keeps hinting far below the tip cannot walk the SDK into
    /// accepting a stale epoch: refinements are bounded and the lookup fails.
    #[tokio::test]
    async fn should_fail_closed_when_the_hint_stays_below_the_proven_tip() {
        let sdk =
            mock_sdk_with_started_epochs(MAX_CURRENT_EPOCH_REFINEMENTS as EpochIndex + 5).await;

        let error = ExtendedEpochInfo::fetch_current(&sdk)
            .await
            .expect_err("a persistently deflated hint must not resolve");

        assert!(
            matches!(
                error,
                Error::StaleNode(StaleNodeError::Epoch {
                    hinted_epoch: 0,
                    ..
                })
            ),
            "expected a stale-node epoch error, got: {error}"
        );
    }

    /// An inflated hint lands in Drive's pre-created empty epoch window, where
    /// the proof matches nothing at all.
    #[tokio::test]
    async fn should_fail_closed_when_the_hint_is_above_every_started_epoch() {
        let mut sdk = SdkBuilder::new_mock().build().expect("build mock sdk");
        sdk.mock()
            .expect_fetch::<ExtendedEpochInfo, _>(
                current_epoch_probe_query(),
                Some(epoch_at(GENESIS_EPOCH_INDEX)),
            )
            .await
            .expect("register probe expectation");
        sdk.mock()
            .expect_fetch_many::<_, ExtendedEpochInfo, _, ExtendedEpochInfos>(
                current_epoch_confirmation_query(0),
                None,
            )
            .await
            .expect("register confirmation expectation");

        let error = ExtendedEpochInfo::fetch_current(&sdk)
            .await
            .expect_err("an unstarted epoch must not resolve");

        assert!(
            matches!(error, Error::EpochNotFound),
            "expected EpochNotFound, got: {error}"
        );
    }
}
