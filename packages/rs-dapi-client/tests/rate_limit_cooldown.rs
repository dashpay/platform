//! Tests that CLIENT-FIXABLE / transient gRPC errors (`ResourceExhausted`,
//! `DeadlineExceeded`, `Aborted`, `Cancelled`) trigger a short, flat 5 s
//! cooldown on the affected node rather than the 60 s×exp ban reserved for
//! genuinely unhealthy nodes (`Unavailable`, `Internal`, `DataLoss`, …).
//!
//! Key invariants verified:
//! - `ban_count` stays 0 after a transient-error cooldown.
//! - The affected node's `ban_info` shows `banned=true` (temporarily
//!   excluded) but `ban_count=0` (not a health failure).
//! - A second node is tried on retry and eventually succeeds.
//! - Genuinely server-side codes (`Unavailable`, `Internal`) still fire the
//!   exponential ban (`ban_count == 1` after one failure).

use std::sync::{Arc, Mutex};

use dapi_grpc::mock::Mockable;
use dapi_grpc::tonic::{Code, Status};
use rs_dapi_client::transport::{
    AppliedRequestSettings, BoxFuture, TransportClient, TransportError, TransportRequest,
};
use rs_dapi_client::{
    Address, AddressList, CanRetry, ConnectionPool, DapiClient, DapiClientError,
    DapiRequestExecutor, ExecutionError, RequestSettings, Uri,
};

// ── shared fake transport infrastructure ─────────────────────────────────────

struct FakeClient {
    uri: Uri,
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

#[derive(Debug)]
struct FakeResponse;
impl Mockable for FakeResponse {}

/// A request that answers `error_code` for the first `fail_count` calls and
/// succeeds afterwards, recording which URIs received the error.
#[derive(Clone, Debug)]
struct FaultyRequest {
    fail_left: Arc<Mutex<usize>>,
    error_code: Code,
    faulted_uris: Arc<Mutex<Vec<Uri>>>,
}

impl FaultyRequest {
    fn with_failures(count: usize, code: Code) -> Self {
        Self {
            fail_left: Arc::new(Mutex::new(count)),
            error_code: code,
            faulted_uris: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl Mockable for FaultyRequest {}

impl TransportRequest for FaultyRequest {
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
        let result = {
            let mut left = self.fail_left.lock().unwrap();
            if *left > 0 {
                *left -= 1;
                self.faulted_uris.lock().unwrap().push(client.uri.clone());
                Err(TransportError::Grpc(Status::new(
                    self.error_code,
                    "fake error",
                )))
            } else {
                Ok(FakeResponse)
            }
        };
        Box::pin(async move { result })
    }
}

// ── is_transient_error propagation ────────────────────────────────────────────

/// All four client-fixable codes must propagate `is_transient_error() == true`
/// through the full error chain: tonic::Status → TransportError → DapiClientError
/// → ExecutionError.  Genuinely server-side codes must NOT.
#[test]
fn transient_codes_propagate_through_error_chain() {
    let transient_codes = [
        Code::ResourceExhausted,
        Code::DeadlineExceeded,
        Code::Aborted,
        Code::Cancelled,
    ];
    for code in transient_codes {
        let status = Status::new(code, "transient");
        assert!(
            status.is_transient_error(),
            "tonic::Status({code:?}) must be is_transient_error"
        );
        assert!(
            status.can_retry(),
            "tonic::Status({code:?}) must also be can_retry"
        );

        let te = TransportError::Grpc(status);
        assert!(
            te.is_transient_error(),
            "TransportError({code:?}) must propagate"
        );

        let dapi = DapiClientError::Transport(te);
        assert!(
            dapi.is_transient_error(),
            "DapiClientError({code:?}) must propagate"
        );

        let exec: ExecutionError<DapiClientError> = ExecutionError {
            inner: dapi,
            retries: 0,
            address: None,
        };
        assert!(
            exec.is_transient_error(),
            "ExecutionError({code:?}) must propagate"
        );
    }
}

/// Genuine server-side codes must NOT be is_transient_error — they still
/// trigger the full exponential ban.
#[test]
fn server_side_codes_are_not_transient() {
    let server_side_codes = [
        Code::Unavailable,
        Code::Internal,
        Code::DataLoss,
        Code::Unknown,
        Code::Unimplemented,
    ];
    for code in server_side_codes {
        let status = Status::new(code, "server error");
        assert!(
            !status.is_transient_error(),
            "tonic::Status({code:?}) must NOT be is_transient_error"
        );
        let te = TransportError::Grpc(status);
        assert!(
            !te.is_transient_error(),
            "TransportError({code:?}) must NOT propagate"
        );
    }
}

// ── integration: cooldown vs ban behaviour ────────────────────────────────────

/// Helper: run a two-node client where node A returns `error_code` once, node
/// B succeeds.  Returns ban info for node A.
async fn run_one_failure(error_code: Code) -> (Address, Vec<rs_dapi_client::AddressBanInfo>) {
    let request = FaultyRequest::with_failures(1, error_code);
    let address_list: AddressList = "http://127.0.0.1:30001,http://127.0.0.1:30002"
        .parse()
        .expect("valid list");
    let client = DapiClient::new(
        address_list,
        RequestSettings {
            retries: Some(1),
            ..Default::default()
        },
    );

    client
        .execute(request.clone(), RequestSettings::default())
        .await
        .expect("should succeed on retry");

    let faulted_uri = request.faulted_uris.lock().unwrap()[0].clone();
    let faulted_addr = Address::try_from(faulted_uri).expect("valid addr");
    let ban_info = client.address_list().ban_info();
    (faulted_addr, ban_info)
}

/// ResourceExhausted (rate-limit) → flat cooldown, ban_count stays 0.
#[tokio::test]
async fn resource_exhausted_triggers_cooldown_not_ban() {
    let (addr, info) = run_one_failure(Code::ResourceExhausted).await;
    let entry = info.iter().find(|i| i.uri == addr.to_string()).unwrap();
    assert!(
        entry.banned,
        "rate-limited node must be temporarily excluded"
    );
    assert_eq!(entry.ban_count, 0, "ban_count must be 0 (not escalated)");
}

/// DeadlineExceeded (client timeout) → flat cooldown, ban_count stays 0.
#[tokio::test]
async fn deadline_exceeded_triggers_cooldown_not_ban() {
    let (addr, info) = run_one_failure(Code::DeadlineExceeded).await;
    let entry = info.iter().find(|i| i.uri == addr.to_string()).unwrap();
    assert!(entry.banned, "timed-out node must be temporarily excluded");
    assert_eq!(
        entry.ban_count, 0,
        "ban_count must be 0 (timeout ≠ node death)"
    );
}

/// Aborted (MVCC tx conflict) → flat cooldown, ban_count stays 0.
#[tokio::test]
async fn aborted_triggers_cooldown_not_ban() {
    let (addr, info) = run_one_failure(Code::Aborted).await;
    let entry = info.iter().find(|i| i.uri == addr.to_string()).unwrap();
    assert!(
        entry.banned,
        "node that returned Aborted must be temporarily excluded"
    );
    assert_eq!(
        entry.ban_count, 0,
        "ban_count must be 0 (MVCC conflict ≠ node death)"
    );
}

/// Cancelled (client cancelled) → flat cooldown, ban_count stays 0.
#[tokio::test]
async fn cancelled_triggers_cooldown_not_ban() {
    let (addr, info) = run_one_failure(Code::Cancelled).await;
    let entry = info.iter().find(|i| i.uri == addr.to_string()).unwrap();
    assert!(
        entry.banned,
        "node that received Cancelled must be temporarily excluded"
    );
    assert_eq!(
        entry.ban_count, 0,
        "ban_count must be 0 (client cancel ≠ node fault)"
    );
}

/// Unavailable (node genuinely down) → FULL exponential ban, ban_count == 1.
#[tokio::test]
async fn unavailable_triggers_full_ban() {
    let (addr, info) = run_one_failure(Code::Unavailable).await;
    let entry = info.iter().find(|i| i.uri == addr.to_string()).unwrap();
    assert!(entry.banned, "unavailable node must be banned");
    assert_eq!(
        entry.ban_count, 1,
        "ban_count must be 1 — Unavailable IS a node health failure"
    );
}

/// ban_count stays 0 after repeated transient errors and rises to 1 only after
/// the first genuine failure. Verified via the public AddressList + ban_info API.
#[tokio::test]
async fn repeated_transient_errors_do_not_escalate_to_genuine_ban() {
    // 5 ResourceExhausted responses in a row, each triggering a cooldown.
    let request = FaultyRequest::with_failures(5, Code::ResourceExhausted);
    let address_list: AddressList =
        "http://127.0.0.1:40001,http://127.0.0.1:40002,http://127.0.0.1:40003,http://127.0.0.1:40004,http://127.0.0.1:40005,http://127.0.0.1:40006"
            .parse()
            .expect("valid list");
    let client = DapiClient::new(
        address_list,
        RequestSettings {
            retries: Some(5),
            ..Default::default()
        },
    );

    // Should succeed: first 5 tries rate-limited, 6th (different node) succeeds.
    client
        .execute(request.clone(), RequestSettings::default())
        .await
        .expect("should eventually succeed");

    // All rate-limited nodes must have ban_count == 0 — flat cooldown only.
    let ban_info = client.address_list().ban_info();
    for entry in &ban_info {
        if entry.banned {
            assert_eq!(
                entry.ban_count, 0,
                "rate-limited node {} must have ban_count=0, got {}",
                entry.uri, entry.ban_count
            );
        }
    }
}
