//! Integration test: `ResourceExhausted` with a `RateLimit-Reset` header causes
//! the node to be banned for that exact period (`ban_for`), while a missing
//! header falls back to the normal exponential health-ban ladder.

mod common;

use common::ScriptedRequest;
use dapi_grpc::tonic::metadata::MetadataValue;
use rs_dapi_client::transport::{AppliedRequestSettings, TransportError};
use rs_dapi_client::{
    update_address_ban_status, Address, AddressList, CanRetry, DapiClient, DapiClientError,
    DapiRequestExecutor, ExecutionError, ExecutionResult, RequestSettings,
};
use std::time::Duration;

fn make_address() -> rs_dapi_client::Address {
    "http://127.0.0.1:3000".parse().expect("valid address")
}

fn applied_settings(ban: bool) -> AppliedRequestSettings {
    AppliedRequestSettings {
        connect_timeout: None,
        timeout: Duration::from_secs(10),
        retries: 5,
        ban_failed_address: ban,
        max_decoding_message_size: None,
        #[cfg(not(target_arch = "wasm32"))]
        ca_certificate: None,
    }
}

/// `ResourceExhausted` + `ratelimit-reset: 45` → `ban_for` with a ~45s window.
/// `ban_count` must be set to at least 1 (diagnostics) but NOT escalated further.
#[test]
fn test_resource_exhausted_with_header_bans_for_advertised_period() {
    let mut address_list = AddressList::new();
    let addr = make_address();
    address_list.add(addr.clone());

    let mut status = dapi_grpc::tonic::Status::resource_exhausted("429");
    status
        .metadata_mut()
        .insert("ratelimit-reset", MetadataValue::try_from("45").unwrap());

    let result: ExecutionResult<i32, DapiClientError> = Err(ExecutionError {
        inner: DapiClientError::Transport(TransportError::Grpc(status)),
        retries: 0,
        address: Some(addr.clone()),
    });

    let before = chrono::Utc::now();
    update_address_ban_status(&address_list, &result, &applied_settings(true));
    let after = chrono::Utc::now();

    let info = address_list.ban_info();
    let entry = info.iter().find(|i| i.uri == addr.to_string()).unwrap();

    assert!(
        entry.banned,
        "node must be banned after ResourceExhausted+header"
    );
    assert_eq!(entry.ban_count, 1, "ban_for sets ban_count to max(0,1)=1");

    // Ban window must be approximately 45 s.
    let until = entry.banned_until.expect("banned_until must be set");
    let lo = (until - before).num_milliseconds() as f64 / 1000.0;
    let hi = (until - after).num_milliseconds() as f64 / 1000.0;
    assert!(
        lo >= 44.9 && hi <= 45.1,
        "ban window must be ~45 s; got lo={lo:.2}s hi={hi:.2}s"
    );
}

/// Large `ratelimit-reset` values are clamped to MAX_RATE_LIMIT_BAN_SECS (600).
#[test]
fn test_ratelimit_reset_clamped_to_max() {
    let mut address_list = AddressList::new();
    let addr = make_address();
    address_list.add(addr.clone());

    let mut status = dapi_grpc::tonic::Status::resource_exhausted("429");
    status
        .metadata_mut()
        .insert("ratelimit-reset", MetadataValue::try_from("9999").unwrap());

    let result: ExecutionResult<i32, DapiClientError> = Err(ExecutionError {
        inner: DapiClientError::Transport(TransportError::Grpc(status)),
        retries: 0,
        address: Some(addr.clone()),
    });

    let before = chrono::Utc::now();
    update_address_ban_status(&address_list, &result, &applied_settings(true));
    let after = chrono::Utc::now();

    let info = address_list.ban_info();
    let entry = info.iter().find(|i| i.uri == addr.to_string()).unwrap();

    let until = entry.banned_until.expect("banned_until set");
    let lo = (until - before).num_milliseconds() as f64 / 1000.0;
    let hi = (until - after).num_milliseconds() as f64 / 1000.0;
    // Clamped at 600 s.
    assert!(
        lo >= 599.5 && hi <= 600.5,
        "9999s must be clamped to 600s; got lo={lo:.2} hi={hi:.2}"
    );
}

/// `ratelimit-reset: 0` or non-numeric → `None` → normal `ban_with_reason` ladder.
///
/// The key assertion is the resulting `banned_until` window: the ladder's first
/// ban is `60 s × e^0 = 60 s`, **not** some header-derived value.  Checking only
/// `ban_count == 1` would pass even if the wrong path (ban_for) were taken.
#[test]
fn test_zero_and_garbage_header_falls_back_to_ladder() {
    // Default AddressList uses DEFAULT_BASE_BAN_PERIOD = 60 s.
    // First ladder ban: coefficient = e^0 = 1.0, window = 60 s.
    const EXPECTED_WINDOW_SECS: f64 = 60.0;

    for bad in &["0", "garbage", ""] {
        let mut address_list = AddressList::new();
        let addr = make_address();
        address_list.add(addr.clone());

        let mut status = dapi_grpc::tonic::Status::resource_exhausted("429");
        if !bad.is_empty() {
            status
                .metadata_mut()
                .insert("ratelimit-reset", MetadataValue::try_from(*bad).unwrap());
        }

        let result: ExecutionResult<i32, DapiClientError> = Err(ExecutionError {
            inner: DapiClientError::Transport(TransportError::Grpc(status)),
            retries: 0,
            address: Some(addr.clone()),
        });

        let before = chrono::Utc::now();
        update_address_ban_status(&address_list, &result, &applied_settings(true));
        let after = chrono::Utc::now();

        let info = address_list.ban_info();
        let entry = info.iter().find(|i| i.uri == addr.to_string()).unwrap();

        assert!(
            entry.banned,
            "bad header '{bad}' must still result in a ban via the ladder"
        );
        assert_eq!(
            entry.ban_count, 1,
            "ladder ban → ban_count = 1 for header '{bad}'"
        );

        // The ban window must be the exponential ladder's first rung (~60 s),
        // NOT a header-derived value.  This assertion fails if ban_for were
        // mistakenly called instead of ban_with_reason.
        let until = entry
            .banned_until
            .expect("banned_until set for header '{bad}'");
        let lo = (until - before).num_milliseconds() as f64 / 1000.0;
        let hi = (until - after).num_milliseconds() as f64 / 1000.0;
        assert!(
            lo >= EXPECTED_WINDOW_SECS - 0.5,
            "bad header '{bad}': ban window lower {lo:.1}s < expected ~{EXPECTED_WINDOW_SECS}s (ladder path)"
        );
        assert!(
            hi <= EXPECTED_WINDOW_SECS + 0.5,
            "bad header '{bad}': ban window upper {hi:.1}s > expected ~{EXPECTED_WINDOW_SECS}s (should be ladder, not ban_for)"
        );
    }
}

/// Missing `ratelimit-reset` header → `None` → normal exponential health-ban ladder.
///
/// Asserts the `banned_until` window is ~60 s (first ladder rung), NOT a
/// header-derived value.  A mere `ban_count == 1` check would pass even if
/// `ban_for` were wrongly invoked (both paths yield ban_count 1 on first ban).
#[test]
fn test_missing_header_falls_back_to_ladder() {
    let mut address_list = AddressList::new();
    let addr = make_address();
    address_list.add(addr.clone());

    let result: ExecutionResult<i32, DapiClientError> = Err(ExecutionError {
        inner: DapiClientError::Transport(TransportError::Grpc(
            dapi_grpc::tonic::Status::resource_exhausted("429"),
        )),
        retries: 0,
        address: Some(addr.clone()),
    });

    let before = chrono::Utc::now();
    update_address_ban_status(&address_list, &result, &applied_settings(true));
    let after = chrono::Utc::now();

    let info = address_list.ban_info();
    let entry = info.iter().find(|i| i.uri == addr.to_string()).unwrap();
    assert!(
        entry.banned,
        "missing header must still result in a ladder ban"
    );
    assert_eq!(entry.ban_count, 1, "first ladder ban → ban_count = 1");

    // Window must be the first exponential rung: 60 s × e^0 = 60 s.
    let until = entry.banned_until.expect("banned_until set");
    let lo = (until - before).num_milliseconds() as f64 / 1000.0;
    let hi = (until - after).num_milliseconds() as f64 / 1000.0;
    assert!(
        lo >= 59.5,
        "ladder window lower {lo:.1}s < expected ~60 s (missing header must use ladder)"
    );
    assert!(
        hi <= 60.5,
        "ladder window upper {hi:.1}s > expected ~60 s (should be ladder, not ban_for)"
    );
}

/// `ban_for` must never shorten an already-active ban.
///
/// (a) LONG then SHORT → `banned_until` stays at the LONG window; `ban_reason`
///     is preserved from the LONG call (SHORT must not overwrite it).
/// (b) SHORT then LONG → `banned_until` extends to the LONG window; `ban_reason`
///     is adopted from the LONG call (window was extended, so reason updates).
/// (c) `ban_count` ends at ≥ 1 in both scenarios.
///
/// QA-007: case (a) uses an exact `assert_eq!` on `banned_until` snapshots —
/// a last-wins regression would produce ~30 s and fail unambiguously.
/// QA-008: named reasons in both cases pin the `ban_reason` update contract.
#[test]
fn test_ban_for_never_shortens_active_ban() {
    let addr: rs_dapi_client::Address = "http://127.0.0.1:3000".parse().expect("valid address");

    // (a) LONG then SHORT — SHORT must NOT change banned_until or ban_reason.
    {
        let mut list = rs_dapi_client::AddressList::new();
        list.add(addr.clone());

        let long_period = Duration::from_secs(300);
        let short_period = Duration::from_secs(30);

        list.ban_for(&addr, long_period, Some("long-reason".into()));

        // Snapshot banned_until immediately after the LONG ban.
        let snapshot = list
            .ban_info()
            .into_iter()
            .find(|i| i.uri == addr.to_string())
            .unwrap()
            .banned_until
            .expect("banned_until set after LONG ban");

        list.ban_for(&addr, short_period, Some("short-reason".into()));

        let info = list.ban_info();
        let entry = info.iter().find(|i| i.uri == addr.to_string()).unwrap();

        // Exact equality: last-wins regression would produce ~30 s here.
        assert_eq!(
            entry.banned_until.expect("banned_until must still be set"),
            snapshot,
            "(a) LONG→SHORT must not change banned_until (last-wins regression guard)"
        );
        // ban_reason must be preserved from the LONG call.
        assert_eq!(
            entry.reason.as_deref(),
            Some("long-reason"),
            "(a) SHORT ban must not overwrite ban_reason set by LONG ban"
        );
        assert!(
            entry.ban_count >= 1,
            "(a) ban_count must be >= 1; got {}",
            entry.ban_count
        );
    }

    // (b) SHORT then LONG — LONG must extend banned_until AND adopt ban_reason.
    //     A trailing ban_for(SHORT) must then be a no-op (kills last-wins regression
    //     in this direction; case (a) kills the symmetric MIN-wins regression).
    {
        let mut list = rs_dapi_client::AddressList::new();
        list.add(addr.clone());

        let short_period = Duration::from_secs(30);
        let long_period = Duration::from_secs(300);

        let before_long = chrono::Utc::now();
        list.ban_for(&addr, short_period, Some("short-reason".into()));
        list.ban_for(&addr, long_period, Some("long-reason".into()));

        // Snapshot banned_until after LONG extension.
        let snapshot_after_long = list
            .ban_info()
            .into_iter()
            .find(|i| i.uri == addr.to_string())
            .unwrap()
            .banned_until
            .expect("banned_until set after LONG ban");

        let before_trailing = before_long; // already elapsed < 1 ms
        let until_after_long = snapshot_after_long;
        let remaining = (until_after_long - before_trailing).num_milliseconds() as f64 / 1000.0;
        assert!(
            remaining >= 299.0,
            "(b) SHORT→LONG must extend ban; remaining={remaining:.1}s, expected >=299s"
        );

        // Trailing SHORT — must not reduce the LONG window (last-wins guard).
        list.ban_for(&addr, short_period, Some("trailing-short".into()));

        let info = list.ban_info();
        let entry = info.iter().find(|i| i.uri == addr.to_string()).unwrap();

        // Exact equality: a last-wins impl would drop banned_until to ~30 s here.
        assert_eq!(
            entry.banned_until.expect("banned_until must still be set"),
            snapshot_after_long,
            "(b) trailing SHORT must not change banned_until set by LONG (last-wins regression guard)"
        );
        // ban_reason must be adopted from the LONG call (window was extended).
        assert_eq!(
            entry.reason.as_deref(),
            Some("long-reason"),
            "(b) LONG ban must update ban_reason when it extends the window"
        );
        assert!(
            entry.ban_count >= 1,
            "(b) ban_count must be >= 1; got {}",
            entry.ban_count
        );
    }
}

/// `rate_limit_ban_duration` on `CanRetry` returns `Some` only for
/// `ResourceExhausted` with a parseable positive `ratelimit-reset`.
#[test]
fn test_rate_limit_ban_duration_trait_delegation() {
    use rs_dapi_client::ExecutionError;

    // With header → Some(45s).
    let mut s = dapi_grpc::tonic::Status::resource_exhausted("429");
    s.metadata_mut()
        .insert("ratelimit-reset", MetadataValue::try_from("45").unwrap());
    let te = TransportError::Grpc(s);
    assert_eq!(te.rate_limit_ban_duration(), Some(Duration::from_secs(45)));

    // Unavailable → None.
    let unavail = TransportError::Grpc(dapi_grpc::tonic::Status::unavailable("down"));
    assert!(unavail.rate_limit_ban_duration().is_none());

    // ResourceExhausted without header → None.
    let re_no_header = TransportError::Grpc(dapi_grpc::tonic::Status::resource_exhausted("429"));
    assert!(re_no_header.rate_limit_ban_duration().is_none());

    // DapiClientError delegates.
    let dce = DapiClientError::Transport(TransportError::Grpc({
        let mut s2 = dapi_grpc::tonic::Status::resource_exhausted("429");
        s2.metadata_mut()
            .insert("ratelimit-reset", MetadataValue::try_from("30").unwrap());
        s2
    }));
    assert_eq!(dce.rate_limit_ban_duration(), Some(Duration::from_secs(30)));

    // ExecutionError delegates.
    let ee: ExecutionError<DapiClientError> = ExecutionError {
        inner: DapiClientError::Transport(TransportError::Grpc({
            let mut s3 = dapi_grpc::tonic::Status::resource_exhausted("429");
            s3.metadata_mut()
                .insert("ratelimit-reset", MetadataValue::try_from("20").unwrap());
            s3
        })),
        retries: 0,
        address: Some(make_address()),
    };
    assert_eq!(ee.rate_limit_ban_duration(), Some(Duration::from_secs(20)));
}

/// End-to-end proof: a `ResourceExhausted` response carrying `ratelimit-reset:
/// 300` that travels through the *real* `DapiClient::execute()` path causes the
/// node to be banned for the Envoy-advertised 300 s window (`ban_for`), NOT the
/// exponential ladder's first rung (~60 s).
///
/// The 300 s figure is deliberately chosen so that:
/// - it differs unambiguously from the ladder's first rung (60 s), and
/// - it is within the [1, 600] s clamp range.
///
/// `ban_count == 1` confirms the flat `ban_for` path, not the ladder.
#[tokio::test]
async fn rate_limited_node_banned_for_advertised_window_via_execute() {
    // Single-node pool: after one ResourceExhausted the node is banned and no
    // live address remains, which collapses to NoAvailableAddressesToRetry.
    let node: Address = "http://127.0.0.1:10201".parse().expect("valid address");
    let address_list: AddressList = "http://127.0.0.1:10201"
        .parse()
        .expect("valid address list");

    // The responder returns ResourceExhausted + ratelimit-reset: 300 on every call.
    let request = ScriptedRequest::new(|_uri| {
        let mut status = dapi_grpc::tonic::Status::resource_exhausted("429");
        status
            .metadata_mut()
            .insert("ratelimit-reset", MetadataValue::try_from("300").unwrap());
        Err(TransportError::Grpc(status))
    });

    let client = DapiClient::new(address_list, RequestSettings::default());

    let before = chrono::Utc::now();
    let err = client
        .execute(request.clone(), RequestSettings::default())
        .await
        .expect_err("single rate-limited node: expect NoAvailableAddressesToRetry");
    let after = chrono::Utc::now();

    // The only node must be banned.
    assert!(
        client.address_list().is_banned(&node),
        "rate-limited node must be banned after execute()"
    );

    // The error must be the address-exhaustion variant (single node, now banned).
    match err.inner {
        DapiClientError::NoAvailableAddressesToRetry(_) => {}
        other => panic!(
            "expected NoAvailableAddressesToRetry from exhausted single-node pool, got: {other:?}"
        ),
    }

    // The ban window must be ~300 s (from the ratelimit-reset header), NOT ~60 s
    // (the ladder's first rung).  This is the key assertion that proves ban_for
    // was called through execute(), not ban_with_reason.
    let info = client.address_list().ban_info();
    let entry = info
        .iter()
        .find(|i| i.uri == node.to_string())
        .expect("ban_info entry for the rate-limited node");

    assert!(entry.banned, "node must be effectively banned");

    // ban_count == 1 proves the flat ban_for floor, not the exponential ladder
    // (both yield ban_count 1 on first ban, but the window assertion below
    // disambiguates; this assertion additionally documents the intent).
    assert_eq!(
        entry.ban_count, 1,
        "ban_for sets ban_count to 1 (flat floor, not ladder escalation)"
    );

    let until = entry.banned_until.expect("banned_until must be set");
    let lo = (until - before).num_milliseconds() as f64 / 1000.0;
    let hi = (until - after).num_milliseconds() as f64 / 1000.0;
    assert!(
        lo >= 299.5 && hi <= 300.5,
        "ban window must be ~300 s (ratelimit-reset header); got lo={lo:.3}s hi={hi:.3}s"
    );
}
