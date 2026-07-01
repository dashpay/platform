//! Shared fake-transport harness for integration tests.
//!
//! Lives in `tests/common/mod.rs` (subdir form) so Cargo does **not** treat
//! it as a standalone test target.  Include it from a test file with
//! `mod common;`.

use std::sync::{Arc, Mutex};

use dapi_grpc::mock::Mockable;
use rs_dapi_client::transport::{
    AppliedRequestSettings, BoxFuture, TransportClient, TransportError, TransportRequest,
};
use rs_dapi_client::{ConnectionPool, RequestSettings, Uri};

// ── Fake client ─────────────────────────────────────────────────────────────

/// Minimal fake transport client: remembers the URI it was built for.
/// There is no real socket — `execute_transport` is never dispatched to the
/// network; the `ScriptedRequest` responder drives it synthetically.
pub struct FakeClient {
    pub uri: Uri,
}

impl TransportClient for FakeClient {
    fn with_uri(uri: Uri, _pool: &ConnectionPool) -> Result<Self, TransportError> {
        Ok(Self { uri })
    }

    fn with_uri_and_settings(
        uri: Uri,
        _settings: &AppliedRequestSettings,
        _pool: &ConnectionPool,
    ) -> Result<Self, TransportError> {
        Ok(Self { uri })
    }
}

// ── Fake response ────────────────────────────────────────────────────────────

/// Trivial fake response returned by the scripted responder on success.
#[derive(Debug, Clone)]
pub struct FakeResponse;

impl Mockable for FakeResponse {}

// ── Scripted request ─────────────────────────────────────────────────────────

/// Generic scripted fake request.
///
/// A caller-supplied `responder` closure decides what to return for each
/// attempt.  The `hit_uris` field records every URI that
/// `execute_transport` was invoked with, in order, so tests can inspect
/// how many nodes were tried and which ones.
///
/// The design handles both "N failures then success" (via a counter captured
/// in the closure) and "always ResourceExhausted+header" patterns.
#[derive(Clone)]
pub struct ScriptedRequest {
    pub responder: Arc<dyn Fn(Uri) -> Result<FakeResponse, TransportError> + Send + Sync>,
    /// Every URI that `execute_transport` was called with, in invocation order.
    pub hit_uris: Arc<Mutex<Vec<Uri>>>,
}

impl ScriptedRequest {
    /// Create a new `ScriptedRequest` driven by `responder`.
    pub fn new(
        responder: impl Fn(Uri) -> Result<FakeResponse, TransportError> + Send + Sync + 'static,
    ) -> Self {
        Self {
            responder: Arc::new(responder),
            hit_uris: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl std::fmt::Debug for ScriptedRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScriptedRequest")
            .field("hit_uris", &self.hit_uris)
            .finish()
    }
}

impl Mockable for ScriptedRequest {}

impl TransportRequest for ScriptedRequest {
    type Client = FakeClient;
    type Response = FakeResponse;

    const SETTINGS_OVERRIDES: RequestSettings = RequestSettings::default();

    fn method_name(&self) -> &'static str {
        "fake_method"
    }

    fn execute_transport<'c>(
        self,
        client: &'c mut Self::Client,
        _settings: &AppliedRequestSettings,
    ) -> BoxFuture<'c, Result<Self::Response, TransportError>> {
        let uri = client.uri.clone();
        self.hit_uris.lock().unwrap().push(uri.clone());
        let result = (self.responder)(uri);
        Box::pin(async move { result })
    }
}
