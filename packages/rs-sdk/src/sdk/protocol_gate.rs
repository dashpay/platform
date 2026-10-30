//! Refusing requests that not every node of the network serves yet.
//!
//! A protocol upgrade can add a method or a request field. Until the network has activated
//! it, nodes of the previous release still answer such a request: with UNIMPLEMENTED, or by
//! dropping the field they do not know and proving a different question. Either answer gets
//! an honest node banned. So before sending a request whose
//! [`TransportRequest::needs_protocol_version`] is set, the fetch paths ask
//! [`Sdk::ensure_network_serves`], which sends it only once a verified answer shows the
//! network runs that version, and they check its answer with [`Sdk::ensure_answer_serves`].
//!
//! [`TransportRequest::needs_protocol_version`]: rs_dapi_client::transport::TransportRequest::needs_protocol_version

use super::{Sdk, SdkInstance};
use crate::error::{Error, StaleNodeError};
use crate::platform::types::epoch::current_epoch_probe_query;
use crate::platform::{Fetch, Query, QuerySettings};
use dapi_grpc::platform::v0::ResponseMetadata;
use dpp::block::extended_epoch_info::ExtendedEpochInfo;
use dpp::version::PlatformVersion;
use rs_dapi_client::RequestSettings;

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
    // sent it.
    if pinned_below(required, state) || !state.has_context_provider {
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

    /// Refuse `method`'s request, which needs protocol version `required` (see
    /// [`TransportRequest::needs_protocol_version`]), unless a verified answer shows the
    /// network runs it and this SDK speaks it. When the network may have reached it since,
    /// ask it first with one proved query, under the caller's `settings`.
    ///
    /// [`TransportRequest::needs_protocol_version`]: rs_dapi_client::transport::TransportRequest::needs_protocol_version
    pub(crate) async fn ensure_network_serves(
        &self,
        required: Option<u32>,
        method: &'static str,
        settings: Option<RequestSettings>,
    ) -> Result<(), Error> {
        let Some(required) = required else {
            return Ok(());
        };
        let state = self.gate_state();
        let pinned = pinned_below(required, &state);
        let cause = match decide(required, &state) {
            Gate::Send => return Ok(()),
            Gate::Refuse => None,
            Gate::Probe => self
                .probe_network_protocol_version(settings)
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
            network,
            pinned,
            cause,
        })
    }

    /// Learn the network's protocol version from the genesis epoch, which every node serves
    /// at every version: the first query of `ExtendedEpochInfo::fetch_current`. It asks for
    /// a proof whatever the SDK's proof setting, since only a verified answer counts.
    async fn probe_network_protocol_version(
        &self,
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
        ExtendedEpochInfo::fetch_with_metadata(self, request, Some(settings)).await?;
        Ok(())
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
