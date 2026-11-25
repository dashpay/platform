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
//! The hold saves probes; it does not promise one per epoch. One answer cannot tell a local
//! clock running ahead of block time from an answer that is old, and the hold takes the
//! shorter reading, so a clock ahead ends it early by the difference and the next request asks
//! again: a hold too short costs a query, one too long refuses what the network serves.
//!
//! [`TransportRequest::needs_protocol_version`]: rs_dapi_client::transport::TransportRequest::needs_protocol_version

use super::{Sdk, SdkInstance};
use crate::error::{Error, StaleNodeError};
use crate::platform::types::epoch::{
    current_epoch_confirmation_query, current_epoch_probe_query, epoch_hint_from_metadata,
};
use crate::platform::{Fetch, FetchMany, ProtocolRequirement, Query, QuerySettings};
use dapi_grpc::platform::v0::ResponseMetadata;
use dash_async::single_flight::{Flight, SingleFlight};
use dpp::block::epoch::EpochIndex;
use dpp::block::extended_epoch_info::v0::ExtendedEpochInfoV0Getters;
use dpp::block::extended_epoch_info::ExtendedEpochInfo;
use dpp::fee::epoch::GENESIS_EPOCH_INDEX;
use dpp::version::PlatformVersion;
use futures::future::{BoxFuture, FutureExt};
use rs_dapi_client::RequestSettings;
use std::cmp::Ordering as Order;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

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

/// What a probe proved: the network ran protocol version `network` in an epoch that has ended
/// by `until_ms` at the latest, on the local clock. The protocol version changes only when an
/// epoch starts, so until that epoch ends no answer can show the network serving a newer
/// version. The epoch can end before `until_ms`, by about how late its first block came
/// ([`Self::from_epoch_starts`]), so the hold saves probes and is no proof of what the
/// network runs now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ProbeHold {
    /// The newest protocol version a verified answer was signed at when the probe ran.
    pub(super) network: u32,
    /// The probed epoch.
    pub(super) epoch: EpochIndex,
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
            epoch: current,
            until_ms: ends_by_local_time.min(ends_by_block_time),
        })
    }

    /// Whether the hold still shows, at `now_ms`, that the network does not serve `required`.
    /// A verified answer newer than the probe's (`verified`) shows the epoch changed since.
    fn covers(&self, required: u32, verified: u32, now_ms: u64) -> bool {
        self.network < required && verified <= self.network && now_ms < self.until_ms
    }
}

/// Why a probe failed, for the call that started it to take.
type ProbeFailure = Arc<Mutex<Option<Box<Error>>>>;

/// The [`ProbeHold`] an SDK and its clones share, and the probe one of them is running.
#[derive(Debug, Default)]
pub(super) struct SharedProbeHold {
    hold: Mutex<Option<ProbeHold>>,
    /// The probe running, which calls made meanwhile poll and wait for instead of asking too.
    /// Dropping the call that started it aborts it, so the others ask under their own settings.
    probing: SingleFlight<BoxFuture<'static, ProbeFailure>>,
    /// How many probes the network answered, so that a call that waited for one can tell
    /// that it was answered.
    answered: AtomicU64,
}

impl SharedProbeHold {
    pub(super) fn get(&self) -> Option<ProbeHold> {
        self.hold.lock().map(|hold| *hold).unwrap_or(None)
    }

    /// Keep the hold of the newest epoch proved: of the newer version, then of the later
    /// epoch, whichever probe finished last. Two holds of one epoch both bound when that
    /// epoch has ended, so the earlier bound is kept.
    pub(super) fn set(&self, hold: ProbeHold) {
        if let Ok(mut current) = self.hold.lock() {
            let newer = current.is_none_or(|held| {
                match (hold.network, hold.epoch).cmp(&(held.network, held.epoch)) {
                    Order::Greater => true,
                    Order::Equal => hold.until_ms < held.until_ms,
                    Order::Less => false,
                }
            });
            if newer {
                *current = Some(hold);
            }
        }
    }

    fn answered(&self) -> u64 {
        self.answered.load(Ordering::Acquire)
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
    // answer cannot be verified, so asking cannot show the network's version. While a
    // probe's epoch has not ended, asking again would show the same version.
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

    /// Whether this SDK is pinned to a version below `required`
    /// ([`SdkBuilder::with_version`](crate::SdkBuilder::with_version)): then no answer can let
    /// it build what `required` introduced, whatever the network runs.
    pub(crate) fn is_pinned_below(&self, required: u32) -> bool {
        pinned_below(required, &self.gate_state())
    }

    /// The refusal [`Self::ensure_network_serves`] gives a request that needs `required`,
    /// for a caller that finds what `required` introduced missing from the network's data
    /// while this SDK runs below it: the network does not serve it yet. `cause` is why the
    /// SDK could not learn the network's version just now, when it tried and failed, as a
    /// failed probe is the refusal's cause there
    /// ([`Error::is_transient_protocol_version_refusal`]).
    pub(crate) fn protocol_version_not_active(
        &self,
        required: u32,
        method: &'static str,
        feature: Option<&'static str>,
        cause: Option<Error>,
    ) -> Error {
        Error::ProtocolVersionNotActive {
            method,
            required,
            feature,
            network: self.query_settings().network_protocol_basis(),
            pinned: pinned_below(required, &self.gate_state()),
            cause: cause.map(Box::new),
        }
    }

    /// Whether a verified answer shows the network below `protocol_version`. A refusal
    /// ([`Error::ProtocolVersionNotActive`]) does not tell on its own: its `network` is the
    /// lower of this SDK's version and the network's, so an SDK pinned below the version
    /// refuses what the network serves. A caller that takes a refusal as "not there yet"
    /// rather than as this SDK's limit checks this first.
    ///
    /// A version shown below by the last probe's epoch hold ([`ProbeHold`]) is taken as it
    /// is. Otherwise the network is asked with the gate's probe, one proved query, pinned SDK
    /// or not, and the call fails when the probe does. `false` when the network is shown at
    /// or above `protocol_version`, and when nothing can show it now: an SDK without a context
    /// provider, or a mock SDK, whose expectations are not verified answers, unless a test set
    /// one ([`Self::mock_verified_protocol_version`]). A version verified earlier, by this SDK
    /// or a clone, shows nothing then without a hold: the network may have upgraded since.
    pub async fn network_proven_below(&self, protocol_version: u32) -> Result<bool, Error> {
        self.network_proven_below_with_settings(protocol_version, None)
            .await
    }

    /// [`Self::network_proven_below`], with the probe sent under the caller's request
    /// `settings` (the SDK's own when `None`), as the gate's probe is.
    pub(crate) async fn network_proven_below_with_settings(
        &self,
        protocol_version: u32,
        settings: Option<RequestSettings>,
    ) -> Result<bool, Error> {
        let state = self.gate_state();
        let verified = state.settings.verified_protocol_version;
        if verified >= protocol_version {
            return Ok(false);
        }
        if state
            .hold
            .is_some_and(|hold| hold.covers(protocol_version, verified, state.now_ms))
        {
            return Ok(true);
        }
        if !state.enforced || !state.has_context_provider {
            return Ok(false);
        }
        // One probe for every call asking at once, as the gate's own: a pinned SDK asks too,
        // since the question is the network's version, not what this SDK speaks.
        let failure = self
            .probe_once_while(protocol_version, settings, move |sdk| {
                let state = sdk.gate_state();
                let verified = state.settings.verified_protocol_version;
                verified < protocol_version
                    && !state
                        .hold
                        .is_some_and(|hold| hold.covers(protocol_version, verified, state.now_ms))
            })
            .await;
        // Another call may have proven the version meanwhile, whatever this probe did.
        let state = self.gate_state();
        let verified = state.settings.verified_protocol_version;
        if verified >= protocol_version {
            return Ok(false);
        }
        if let Some(failure) = failure {
            return Err(*failure);
        }
        Ok(verified != 0 && verified < protocol_version)
    }

    /// Whether a verified answer shows the network at or above `protocol_version` after
    /// asking it once more, past the probe hold ([`ProbeHold`]), under the caller's request
    /// `settings`: for a call that sent what the network served before `protocol_version`
    /// and was refused, as the network may have reached it since. A hold can outlast its
    /// epoch (when the epoch's first block came late), and the network can reach the
    /// version between a probe and the request.
    ///
    /// The hold itself is kept: a refusal is one node's word, not a verified answer. An
    /// answer that shows `protocol_version` makes the hold moot, and one that shows the
    /// network still below it renews the hold. The probe's failure is the error, unless
    /// another call proved the version meanwhile.
    pub(crate) async fn network_reached_past_hold(
        &self,
        protocol_version: u32,
        settings: Option<RequestSettings>,
    ) -> Result<bool, Error> {
        let state = self.gate_state();
        let failure = if state.enforced && state.has_context_provider {
            self.probe_once_while(protocol_version, settings, move |sdk| {
                sdk.query_settings().verified_protocol_version < protocol_version
            })
            .await
        } else {
            None
        };
        if self.query_settings().verified_protocol_version >= protocol_version {
            return Ok(true);
        }
        match failure {
            Some(failure) => Err(*failure),
            None => Ok(false),
        }
    }

    /// Take `protocol_version` as the network's verified version, as if a proved answer had
    /// shown it, for a mock SDK in a test that needs the network's version known: a mock's
    /// expectations are not verified answers. It never lowers the version. The network then
    /// reads as proven at that version for the rest of the test, as a probe in an epoch that
    /// outlasts the test shows it ([`Self::network_proven_below`]). A network SDK refuses it
    /// with [`Error::Config`].
    #[cfg(feature = "mocks")]
    pub fn mock_verified_protocol_version(&self, protocol_version: u32) -> Result<(), Error> {
        match self.inner {
            SdkInstance::Mock { .. } => {
                self.record_verified_protocol_version(protocol_version);
                self.probe_hold.set(ProbeHold {
                    network: protocol_version,
                    epoch: 0,
                    until_ms: u64::MAX,
                });
                Ok(())
            }
            SdkInstance::Dapi { .. } => Err(Error::Config(
                "only a mock SDK takes a verified protocol version from a test".to_string(),
            )),
        }
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
            Gate::Probe => self.probe_unless_probing(required, settings).await,
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

    /// Establish that the network serves `requirement` and this SDK speaks it, before a
    /// caller does work it cannot undo for a request that needs it: a wallet proving,
    /// reserving notes for or saving a state transition before broadcasting it. Same rules
    /// as the gate the request itself passes: an unpinned SDK asks the network with one
    /// proved query when no verified answer has shown its version yet, and raises its own
    /// version from the answer; a pinned SDK below `requirement` is refused. Refuses with
    /// [`Error::ProtocolVersionNotActive`].
    pub async fn ensure_protocol_requirement(
        &self,
        requirement: ProtocolRequirement,
    ) -> Result<(), Error> {
        self.establish_protocol_requirement(requirement, None).await
    }

    /// Establish, before a request built against the network's protocol version, that the
    /// network serves `requirement` and this SDK speaks it. The gate decides first, under the
    /// caller's `settings` ([`Self::ensure_network_serves`]): its probe asks with a proof
    /// whatever this SDK's proof setting and raises an unpinned SDK's version, and it refuses
    /// with [`Error::ProtocolVersionNotActive`], the probe's failure as its cause when it
    /// failed. An SDK that enforces no gate (a mock) is checked on its own version instead,
    /// refreshed first when it is below, under the same settings, the refresh's failure as the
    /// refusal's cause.
    pub(crate) async fn establish_protocol_requirement(
        &self,
        requirement: ProtocolRequirement,
        settings: Option<RequestSettings>,
    ) -> Result<(), Error> {
        self.ensure_network_serves(Some(requirement), settings)
            .await?;
        if self.enforces_protocol_gate()
            || self.protocol_version_number() >= requirement.protocol_version
        {
            return Ok(());
        }
        let (version, failure) = self.refresh_protocol_version_or_failure(settings).await;
        if version < requirement.protocol_version {
            return Err(self.protocol_version_not_active(
                requirement.protocol_version,
                requirement.method,
                requirement.feature,
                failure,
            ));
        }
        Ok(())
    }

    /// Probe for `required` ([`Self::probe_network_protocol_version`]) and return why the
    /// probe failed, if it did, unless another call is probing: then poll that probe too, wait
    /// for its answer and decide from it, as the call that probed does.
    ///
    /// A call waits for one probe at most, which runs under the settings of the call that
    /// started it. When that probe failed, or its call was dropped, the call asks under its
    /// own settings at once, without waiting for the others that ask too.
    async fn probe_unless_probing(
        &self,
        required: u32,
        settings: Option<RequestSettings>,
    ) -> Option<Box<Error>> {
        self.probe_once_while(required, settings, move |sdk| {
            decide(required, &sdk.gate_state()) == Gate::Probe
        })
        .await
    }

    /// [`Self::probe_unless_probing`], for a call that still needs to ask while `needed`
    /// says so: it does not ask when a probe answered while it waited, or `needed` no
    /// longer holds.
    async fn probe_once_while(
        &self,
        required: u32,
        settings: Option<RequestSettings>,
        needed: impl Fn(&Self) -> bool + Clone + Send + 'static,
    ) -> Option<Box<Error>> {
        let answered = self.probe_hold.answered();
        let started = match self.probe_hold.probing.join_or_start(|| {
            let sdk = self.clone();
            let needed = needed.clone();
            async move {
                let failure = sdk
                    .probe_unless_answered(required, settings, answered, needed)
                    .await;
                Arc::new(Mutex::new(failure))
            }
            .boxed()
        }) {
            Flight::Started(started) => started,
            Flight::Joined(running) => {
                // Polled here too, as the call that started it may not be polled.
                let _ = running.await;
                return self
                    .probe_unless_answered(required, settings, answered, needed)
                    .await;
            }
        };
        let failure = started.await.ok()?;
        let mut failure = failure.lock().unwrap_or_else(PoisonError::into_inner);
        failure.take()
    }

    /// Probe for `required` and return why it failed, unless a probe answered since
    /// `answered` was read, or `needed` no longer holds.
    async fn probe_unless_answered(
        &self,
        required: u32,
        settings: Option<RequestSettings>,
        answered: u64,
        needed: impl Fn(&Self) -> bool,
    ) -> Option<Box<Error>> {
        if self.probe_hold.answered() != answered || !needed(self) {
            return None;
        }
        let probed = self
            .probe_network_protocol_version(required, settings)
            .await;
        if probed.is_ok() {
            self.probe_hold.answered.fetch_add(1, Ordering::AcqRel);
        }
        probed.err().map(Box::new)
    }

    /// Ask the network for its protocol version before a node is banned for an answer this
    /// build could not read that names a version newer than the build knows
    /// ([`Self::classify_refused_answer`]).
    ///
    /// The question is the gate's probe query, the genesis epoch, without the epoch hold that
    /// may follow it there: a hold can outlast its epoch, and a ban is not to rest on it. It
    /// is kept short, since it runs inside the caller's request: one attempt, which bans no
    /// node, as an answer to it this build cannot read says as little about its node.
    ///
    /// Nothing is asked when nothing can be learned: by an SDK without nodes or a context
    /// provider, or once a verified answer has shown a newer version. Nor is it asked about
    /// an answer to a probe itself, which would wait for the probe it belongs to.
    ///
    /// While another call probes, this one waits for that probe and decides from what it
    /// showed, also when it failed: a call asks or waits once, never both, so calls made
    /// together cost one question.
    pub(super) async fn probe_for_newer_network(&self) {
        let required = PlatformVersion::latest().protocol_version.saturating_add(1);
        let state = self.gate_state();
        if self.inside_version_probe
            || !state.enforced
            || !state.has_context_provider
            || state.settings.verified_protocol_version >= required
        {
            return;
        }
        let flight = self.probe_hold.probing.join_or_start(|| {
            let sdk = self.probing();
            async move {
                sdk.ask_for_newer_network().await;
                ProbeFailure::default()
            }
            .boxed()
        });
        // A joined probe is polled here too, as the call that started it may not be polled.
        match flight {
            Flight::Started(started) => drop(started.await),
            Flight::Joined(running) => drop(running.await),
        }
    }

    /// The question of [`Self::probe_for_newer_network`], asked by an SDK that knows it
    /// probes ([`Self::probing`]).
    async fn ask_for_newer_network(&self) {
        let Ok(request) = current_epoch_probe_query().query(&QuerySettings {
            prove: true,
            ..self.query_settings()
        }) else {
            return;
        };
        let settings = RequestSettings {
            ban_failed_address: Some(false),
            retries: Some(0),
            ..Default::default()
        };
        if ExtendedEpochInfo::fetch_with_metadata(self, request, Some(settings))
            .await
            .is_ok()
        {
            self.probe_hold.answered.fetch_add(1, Ordering::AcqRel);
        }
    }

    /// A clone that knows it probes: an answer to one of its requests that cannot be read is
    /// classified as it is ([`Self::probe_for_newer_network`]).
    fn probing(&self) -> Sdk {
        Sdk {
            inside_version_probe: true,
            ..self.clone()
        }
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
        let sdk = &self.probing();
        let request = current_epoch_probe_query().query(&QuerySettings {
            prove: true,
            ..sdk.query_settings()
        })?;
        let mut settings = settings.unwrap_or_default();
        if !sdk.prove() {
            // An SDK that does not ask for proofs may hold a context provider that cannot
            // give quorum keys. Its probe then fails on every node, which says nothing
            // about the nodes, so it bans none of them.
            settings.ban_failed_address = Some(false);
        }
        let (genesis, metadata) =
            ExtendedEpochInfo::fetch_with_metadata(sdk, request, Some(settings)).await?;
        if let (Some(genesis), true) = (
            genesis.filter(|genesis| genesis.index() == GENESIS_EPOCH_INDEX),
            sdk.query_settings().network_protocol_basis() < required,
        ) {
            sdk.hold_probes(
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
