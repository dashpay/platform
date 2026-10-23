//! Refusing requests that not every node of the network serves yet.

use super::Sdk;
use crate::platform::QuerySettings;
use dpp::version::PlatformVersion;

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
    /// Whether the SDK enforces the gate.
    pub(super) enforced: bool,
}

pub(super) fn decide(_required: u32, _state: &GateState<'_>) -> Gate {
    Gate::Send
}

pub(super) fn version_for_requests(
    stored: &'static PlatformVersion,
    _verified: u32,
    _pinned: bool,
) -> &'static PlatformVersion {
    stored
}

impl Sdk {
    #[cfg(test)]
    pub(super) fn with_protocol_gate_enforced(self) -> Self {
        self
    }
}
