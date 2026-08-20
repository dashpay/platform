//! Shared limit cap for the aggregate (COUNT / SUM / AVG) proof
//! verifiers.
//!
//! Server counterpart: the prove-path arms of drive's aggregate
//! dispatchers (`rs-drive`'s
//! `query/drive_document_{count,sum,average}_query/drive_dispatcher.rs`)
//! refuse any request whose limit exceeds
//! `drive_config.max_query_limit` with
//! `QuerySyntaxError::InvalidLimit` *before* producing proof bytes.
//! The verifier mirrors that gate against the compile-time
//! [`DEFAULT_MAX_QUERY_LIMIT`] — `max_query_limit`'s config default
//! (100); the SDK cannot see an operator's runtime tuning, and proof
//! bytes never depend on it — so a request the server would refuse
//! can never reach a proof primitive. Without the cap, an untrusted
//! transport could pair a server-invalid request (limit 101..=65535
//! fits the wire's `u32` and even a `u16`) with a genuine proof
//! produced for a different, server-permitted query.
//!
//! The `0` sentinel ("no limit set on the wire": V0's `limit: 0`,
//! V1's `limit: None`) is translated per walk shape, mirroring the
//! server dispatchers exactly:
//! - distinct walk (`RangeDistinctProof`): `0` →
//!   [`DEFAULT_QUERY_LIMIT`], because the server applies
//!   `limit.unwrap_or(DEFAULT_QUERY_LIMIT)` before building the
//!   path query;
//! - carrier walk (`RangeAggregateCarrierProof`): `0` → `None`,
//!   because the server keeps an unset limit as an unbounded outer
//!   walk.

use drive::config::{DEFAULT_MAX_QUERY_LIMIT, DEFAULT_QUERY_LIMIT};

// The distinct-walk fallback (`DEFAULT_QUERY_LIMIT`, mirroring the
// server's `limit.unwrap_or(DEFAULT_QUERY_LIMIT)`) must itself pass
// the cap, exactly as the server checks its own fallback against
// `max_query_limit`. Both are 100 today with no compile-time link;
// this pin makes a future divergence a build error here instead of
// a silent parity break.
const _: () = assert!(DEFAULT_QUERY_LIMIT <= DEFAULT_MAX_QUERY_LIMIT);

/// Reject a limit the server's aggregate prove paths would refuse
/// with `InvalidLimit`. Run this before any proof or
/// context-provider machinery; the walk-shape converters below
/// assume it has already passed. `0` (unset sentinel) always
/// passes — its meaning is resolved per walk shape.
pub(crate) fn check_within_server_cap(
    limit: u32,
    surface: &str,
) -> Result<(), drive_proof_verifier::Error> {
    if limit > u32::from(DEFAULT_MAX_QUERY_LIMIT) {
        return Err(drive_proof_verifier::Error::RequestError {
            error: format!(
                "limit {limit} exceeds the server's max_query_limit {DEFAULT_MAX_QUERY_LIMIT} \
                 on the prove path ({surface}); the server refuses such requests with \
                 InvalidLimit before producing proof bytes, so no proved response can \
                 belong to this request"
            ),
        });
    }
    Ok(())
}

/// Distinct-walk (`RangeDistinctProof`) limit: `0` falls back to
/// [`DEFAULT_QUERY_LIMIT`], mirroring the server's
/// `limit.unwrap_or(DEFAULT_QUERY_LIMIT)`. Callers must have run
/// [`check_within_server_cap`] first, which is what makes the
/// narrowing cast exact.
pub(crate) fn distinct_walk_limit(limit: u32) -> u16 {
    debug_assert!(limit <= u32::from(DEFAULT_MAX_QUERY_LIMIT));
    if limit == 0 {
        DEFAULT_QUERY_LIMIT
    } else {
        limit as u16
    }
}

/// Carrier-walk (`RangeAggregateCarrierProof`) limit: `0` stays
/// `None` (unbounded outer walk), mirroring the server keeping an
/// unset request limit as `None`. Callers must have run
/// [`check_within_server_cap`] first, which is what makes the
/// narrowing cast exact.
pub(crate) fn carrier_walk_limit(limit: u32) -> Option<u16> {
    debug_assert!(limit <= u32::from(DEFAULT_MAX_QUERY_LIMIT));
    if limit == 0 {
        None
    } else {
        Some(limit as u16)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The cap is inclusive at `DEFAULT_MAX_QUERY_LIMIT`, exclusive
    /// one past it, and the `0` sentinel always passes (its meaning
    /// is resolved per walk shape) — the same acceptance set as the
    /// server dispatchers' `> max_query_limit` rejection.
    #[test]
    fn cap_boundaries() {
        assert!(check_within_server_cap(0, "TEST").is_ok());
        assert!(check_within_server_cap(1, "TEST").is_ok());
        assert!(check_within_server_cap(u32::from(DEFAULT_MAX_QUERY_LIMIT), "TEST").is_ok());

        let error = check_within_server_cap(u32::from(DEFAULT_MAX_QUERY_LIMIT) + 1, "TEST")
            .expect_err("one past the cap must be rejected");
        assert!(
            error
                .to_string()
                .contains("exceeds the server's max_query_limit"),
            "unexpected error: {error}"
        );
    }

    /// Distinct walks translate the `0` sentinel to
    /// `DEFAULT_QUERY_LIMIT`, mirroring the server's
    /// `limit.unwrap_or(DEFAULT_QUERY_LIMIT)`; in-range values pass
    /// through untouched.
    #[test]
    fn distinct_walk_sentinel_translation() {
        assert_eq!(distinct_walk_limit(0), DEFAULT_QUERY_LIMIT);
        assert_eq!(distinct_walk_limit(1), 1);
        assert_eq!(
            distinct_walk_limit(u32::from(DEFAULT_MAX_QUERY_LIMIT)),
            DEFAULT_MAX_QUERY_LIMIT
        );
    }

    /// Carrier walks keep the `0` sentinel as `None` (unbounded
    /// outer walk), mirroring the server keeping an unset request
    /// limit as `None`; in-range values pass through untouched.
    #[test]
    fn carrier_walk_sentinel_translation() {
        assert_eq!(carrier_walk_limit(0), None);
        assert_eq!(carrier_walk_limit(1), Some(1));
        assert_eq!(
            carrier_walk_limit(u32::from(DEFAULT_MAX_QUERY_LIMIT)),
            Some(DEFAULT_MAX_QUERY_LIMIT)
        );
    }
}
