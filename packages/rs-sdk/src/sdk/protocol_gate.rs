//! Refusing requests that not every node of the network serves yet.
//!
//! A protocol upgrade can add a method, a request field or a query feature. Until the network
//! has activated it, nodes of the previous release still answer such a request: with
//! UNIMPLEMENTED, by dropping the field they do not know and proving a different question,
//! or by refusing or reinterpreting the feature. Several of these answers get an honest node
//! banned, and none serves the request. So before sending a request whose
//! [`Query::protocol_requirement`] is set (a request's
//! [`TransportRequest::needs_protocol_version`], or the features of a document query), the
//! fetch paths ask [`Sdk::ensure_network_serves`], which sends it only once a verified answer
//! shows the network runs that version, and they check its answer with
//! [`Sdk::ensure_answer_serves`].
//!
//! The protocol version changes only when an epoch starts. A probe that proves the network
//! below a requirement also proves the current epoch and when it started ([`ProbeHold`]), and
//! the gate refuses requests that need more without asking again until the current epoch can
//! have ended. Any verified answer that shows the network at the requirement, or at any version
//! newer than the probe saw, lifts the refusal at once.
//!
//! [`TransportRequest::needs_protocol_version`]: rs_dapi_client::transport::TransportRequest::needs_protocol_version

use super::{Sdk, SdkInstance};
use crate::error::{Error, StaleNodeError};
use crate::platform::types::epoch::{
    current_epoch_confirmation_query, current_epoch_probe_query, epoch_hint_from_metadata,
};
use crate::platform::{Fetch, FetchMany, ProtocolRequirement, Query, QuerySettings};
use dapi_grpc::platform::v0::ResponseMetadata;
use dpp::block::epoch::EpochIndex;
use dpp::block::extended_epoch_info::v0::ExtendedEpochInfoV0Getters;
use dpp::block::extended_epoch_info::ExtendedEpochInfo;
use dpp::fee::epoch::GENESIS_EPOCH_INDEX;
use dpp::version::PlatformVersion;
use rs_dapi_client::RequestSettings;
use std::collections::BTreeMap;
use std::sync::Mutex;

/// What to do with a request that needs a protocol version.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Gate {
    /// Every node serves it: send it.
    Send,
    /// The network may serve it but has not proven so: ask for its version first.
    Probe,
    /// No answer can show that both the network and this SDK serve it: refuse it.
    Refuse,
}

/// What the decision reads from the SDK.
#[derive(Debug, Clone, Copy)]
pub(super) struct GateState<'a> {
    /// The SDK's encoding settings, which carry its version and the network's verified one.
    pub(super) settings: QuerySettings<'a>,
    /// Whether the SDK's version is pinned.
    pub(super) pinned: bool,
    /// Whether the SDK can verify proofs.
    pub(super) has_context_provider: bool,
    /// Whether the SDK enforces the gate. Mock SDKs answer what a test registered and have
    /// no nodes to ban, and their expectations are never verified answers.
    pub(super) enforced: bool,
    /// What the last probe proved about the epoch it ran in, if anything.
    pub(super) hold: Option<ProbeHold>,
    /// The local clock, in milliseconds since the Unix epoch.
    pub(super) now_ms: u64,
}

/// What a probe proved: the network ran protocol version `network` in an epoch that may not
/// end before `until_ms`, on the local clock. The protocol version changes only when an epoch
/// starts, so until then no answer can show the network serving a newer version.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ProbeHold {
    /// The newest protocol version a verified answer was signed at when the probe ran.
    pub(super) network: u32,
    /// When the probed epoch has ended at the latest, on the local clock, in milliseconds
    /// since the Unix epoch.
    pub(super) until_ms: u64,
}

impl ProbeHold {
    /// The hold from verified start times: of the genesis epoch (`genesis_ms`) and of the
    /// probed epoch, `current`, which started at `current_ms`. `block_time_ms` is the verified
    /// time of the answer that proved them, and `local_now_ms` the local clock when it arrived.
    ///
    /// Epochs follow a fixed grid: epoch `n` runs from `n` epoch lengths after genesis to
    /// `n + 1`, and starts with its first block. Epoch `current` started no earlier than
    /// `current` lengths after genesis, so the length is at most
    /// `(current_ms - genesis_ms) / current`, and the epoch ends no later than that much after
    /// it started. The hold outlasts the epoch by about how late the epoch's first block came
    /// after the epoch's grid start (for example after a halt).
    ///
    /// The hold is measured on the local clock from when the answer arrived, so a local clock
    /// running behind block time does not lengthen it, and it never runs past the bound by
    /// block time, so an answer older than the local clock does not lengthen it either. `None`
    /// for the genesis epoch, or when the times are not in order.
    pub(super) fn from_epoch_starts(
        network: u32,
        genesis_ms: u64,
        current: EpochIndex,
        current_ms: u64,
        block_time_ms: u64,
        local_now_ms: u64,
    ) -> Option<Self> {
        let longest_epoch = current_ms
            .checked_sub(genesis_ms)?
            .checked_div(u64::from(current))
            .filter(|length| *length > 0)?;
        let ends_by_block_time = current_ms.checked_add(longest_epoch)?;
        let ends_by_local_time =
            local_now_ms.checked_add(ends_by_block_time.saturating_sub(block_time_ms))?;
        Some(Self {
            network,
            until_ms: ends_by_local_time.min(ends_by_block_time),
        })
    }

    /// Whether the hold still shows, at `now_ms`, that the network does not serve `required`.
    /// A verified answer newer than the probe's (`verified`) shows the epoch changed since.
    fn covers(&self, required: u32, verified: u32, now_ms: u64) -> bool {
        self.network < required && verified <= self.network && now_ms < self.until_ms
    }
}

/// The [`ProbeHold`] an SDK and its clones share.
#[derive(Debug, Default)]
pub(super) struct SharedProbeHold(Mutex<Option<ProbeHold>>);

impl SharedProbeHold {
    fn get(&self) -> Option<ProbeHold> {
        self.0.lock().map(|hold| *hold).unwrap_or(None)
    }

    fn set(&self, hold: ProbeHold) {
        if let Ok(mut current) = self.0.lock() {
            *current = Some(hold);
        }
    }
}

/// Whether a request that needs `required` is sent, refused, or sent only after asking the
/// network for its version.
pub(super) fn decide(required: u32, state: &GateState<'_>) -> Gate {
    if !state.enforced || state.settings.network_protocol_basis() >= required {
        return Gate::Send;
    }
    // Asking raises the verified version, and the version of an unpinned SDK; a pinned SDK
    // below `required` stays below it whatever the answer. Without a context provider the
    // answer cannot be verified, and an answer the SDK cannot verify bans the node that
    // sent it. While a probe's epoch has not ended, asking again would show the same version.
    if pinned_below(required, state)
        || !state.has_context_provider
        || state.hold.is_some_and(|hold| {
            hold.covers(
                required,
                state.settings.verified_protocol_version,
                state.now_ms,
            )
        })
    {
        Gate::Refuse
    } else {
        Gate::Probe
    }
}

/// Whether the SDK is pinned to a version below `required`.
fn pinned_below(required: u32, state: &GateState<'_>) -> bool {
    state.pinned && state.settings.protocol_version.protocol_version < required
}

/// The version an SDK encodes requests at: its stored one, except that an unpinned SDK
/// whose verified answers show the network runs a version this build does not know speaks
/// the newest version it knows, the rules it already reads those answers with.
pub(super) fn version_for_requests(
    stored: &'static PlatformVersion,
    verified: u32,
    pinned: bool,
) -> &'static PlatformVersion {
    let newest = PlatformVersion::latest();
    if !pinned && verified > newest.protocol_version {
        newest
    } else {
        stored
    }
}

impl Sdk {
    fn gate_state(&self) -> GateState<'_> {
        GateState {
            settings: self.query_settings(),
            pinned: self.version_pinned,
            has_context_provider: self.context_provider().is_some(),
            enforced: self.enforces_protocol_gate(),
            hold: self.probe_hold.get(),
            now_ms: chrono::Utc::now().timestamp_millis() as u64,
        }
    }

    fn enforces_protocol_gate(&self) -> bool {
        match self.inner {
            SdkInstance::Dapi { .. } => true,
            #[cfg(feature = "mocks")]
            SdkInstance::Mock { .. } => self.mock_enforces_protocol_gate(),
        }
    }

    #[cfg(all(feature = "mocks", test))]
    fn mock_enforces_protocol_gate(&self) -> bool {
        self.enforce_protocol_gate_on_mock
    }

    #[cfg(all(feature = "mocks", not(test)))]
    fn mock_enforces_protocol_gate(&self) -> bool {
        false
    }

    /// Enforce the gate on a mock SDK, as on a network one.
    #[cfg(all(feature = "mocks", test))]
    pub(super) fn with_protocol_gate_enforced(mut self) -> Self {
        self.enforce_protocol_gate_on_mock = true;
        self
    }

    /// Refuse a request that needs `requirement` (see [`Query::protocol_requirement`]),
    /// unless a verified answer shows the network runs its protocol version and this SDK
    /// speaks it. When the network may have reached it since, ask it first with one proved
    /// query, under the caller's `settings`.
    pub(crate) async fn ensure_network_serves(
        &self,
        requirement: Option<ProtocolRequirement>,
        settings: Option<RequestSettings>,
    ) -> Result<(), Error> {
        let Some(ProtocolRequirement {
            protocol_version: required,
            method,
            feature,
        }) = requirement
        else {
            return Ok(());
        };
        let state = self.gate_state();
        let pinned = pinned_below(required, &state);
        let cause = match decide(required, &state) {
            Gate::Send => return Ok(()),
            Gate::Refuse => None,
            Gate::Probe => self
                .probe_network_protocol_version(required, settings)
                .await
                .err()
                .map(Box::new),
        };
        // Another call may have verified the version meanwhile, whatever this probe did.
        let network = self.query_settings().network_protocol_basis();
        if network >= required {
            return Ok(());
        }
        Err(Error::ProtocolVersionNotActive {
            method,
            required,
            feature,
            network,
            pinned,
            cause,
        })
    }

    /// Learn the network's protocol version from the genesis epoch, which every node serves
    /// at every version: the first query of `ExtendedEpochInfo::fetch_current`. It asks for
    /// a proof whatever the SDK's proof setting, since only a verified answer counts.
    ///
    /// When the answer shows the network below `required`, also prove the current epoch,
    /// to hold further probes until it can end ([`Self::hold_probes`]).
    async fn probe_network_protocol_version(
        &self,
        required: u32,
        settings: Option<RequestSettings>,
    ) -> Result<(), Error> {
        let request = current_epoch_probe_query().query(&QuerySettings {
            prove: true,
            ..self.query_settings()
        })?;
        let mut settings = settings.unwrap_or_default();
        if !self.prove() {
            // An SDK that does not ask for proofs may hold a context provider that cannot
            // give quorum keys. Its probe then fails on every node, which says nothing
            // about the nodes, so it bans none of them.
            settings.ban_failed_address = Some(false);
        }
        let (genesis, metadata) =
            ExtendedEpochInfo::fetch_with_metadata(self, request, Some(settings)).await?;
        if let (Some(genesis), true) = (
            genesis.filter(|genesis| genesis.index() == GENESIS_EPOCH_INDEX),
            self.query_settings().network_protocol_basis() < required,
        ) {
            self.hold_probes(
                genesis.first_block_time(),
                epoch_hint_from_metadata(&metadata),
                settings,
            )
            .await;
        }
        Ok(())
    }

    /// Prove that `hinted` is the current epoch and when it started, and hold probes until it
    /// can have ended ([`ProbeHold`]), from the start of the genesis epoch, `genesis_ms`, which
    /// the probe proved.
    ///
    /// The epoch index in response metadata is not covered by the quorum signature, so the
    /// hint only shapes this proved query; the hold is built from what its proof shows. When
    /// it shows something else, the query fails, or `hinted` is the genesis epoch, nothing
    /// is held and the next request that needs more asks again.
    async fn hold_probes(&self, genesis_ms: u64, hinted: EpochIndex, settings: RequestSettings) {
        if hinted == GENESIS_EPOCH_INDEX {
            return;
        }
        let request = match current_epoch_confirmation_query(hinted).query(&QuerySettings {
            prove: true,
            ..self.query_settings()
        }) {
            Ok(request) => request,
            Err(error) => {
                tracing::debug!(%error, "cannot ask for the current epoch; probes are not held");
                return;
            }
        };
        let (epochs, metadata) = match ExtendedEpochInfo::fetch_many_with_metadata(
            self,
            request,
            Some(settings),
        )
        .await
        {
            Ok(answer) => answer,
            Err(error) => {
                tracing::debug!(%error, "the current epoch is not proven; probes are not held");
                return;
            }
        };
        let local_now_ms = chrono::Utc::now().timestamp_millis() as u64;
        // The query reads `hinted` and the epoch above: `hinted` is current when it is the
        // only one that started.
        let started: BTreeMap<EpochIndex, ExtendedEpochInfo> = epochs
            .into_iter()
            .filter_map(|(index, epoch)| epoch.map(|epoch| (index, epoch)))
            .collect();
        let (Some(current), 1) = (started.get(&hinted), started.len()) else {
            return;
        };
        if let Some(hold) = ProbeHold::from_epoch_starts(
            metadata.protocol_version,
            genesis_ms,
            hinted,
            current.first_block_time(),
            metadata.time_ms,
            local_now_ms,
        ) {
            self.probe_hold.set(hold);
        }
    }

    /// Refuse an answer to a request that needs protocol version `required` when it is
    /// signed at an older one: only a node behind the network's activation of `required`
    /// signs it, and it answered under rules it does not follow.
    pub(crate) fn ensure_answer_serves(
        &self,
        required: Option<u32>,
        metadata: &ResponseMetadata,
    ) -> Result<(), Error> {
        match required {
            Some(required)
                if metadata.protocol_version < required && self.enforces_protocol_gate() =>
            {
                Err(StaleNodeError::BehindProtocolUpgrade {
                    required,
                    signed: metadata.protocol_version,
                }
                .into())
            }
            _ => Ok(()),
        }
    }
}
