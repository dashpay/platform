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
//! - carrier walk (`RangeAggregateCarrierProof`), SUM / AVG: `0` →
//!   `None`, because those servers keep an unset limit as an
//!   unbounded outer walk;
//! - carrier walk, COUNT: shape-dependent — see
//!   [`ServerCappedLimit::count_carrier_walk_limit`], whose
//!   range-outer arm enforces the stricter compile-time cap the
//!   COUNT dispatcher applies.

use drive::config::{DEFAULT_MAX_QUERY_LIMIT, DEFAULT_QUERY_LIMIT};

// The distinct-walk fallback (`DEFAULT_QUERY_LIMIT`, mirroring the
// server's `limit.unwrap_or(DEFAULT_QUERY_LIMIT)`) must itself pass
// the cap, exactly as the server checks its own fallback against
// `max_query_limit`. Both are 100 today with no compile-time link;
// this pin makes a future divergence a build error here instead of
// a silent parity break.
const _: () = assert!(DEFAULT_QUERY_LIMIT <= DEFAULT_MAX_QUERY_LIMIT);

/// A request limit that has passed the server's aggregate
/// prove-path cap. Constructing one via [`check_within_server_cap`]
/// is the only way to reach the walk-shape converters, so the
/// "cap check runs first" ordering — what makes their narrowing
/// casts exact — is enforced by the type system rather than a
/// `debug_assert!` that vanishes from release builds.
#[derive(Debug)]
pub(crate) struct ServerCappedLimit(u32);

/// Reject a limit the server's aggregate prove paths would refuse
/// with `InvalidLimit`. Run this before any proof or
/// context-provider machinery; the returned [`ServerCappedLimit`]
/// carries the proof that it passed. `0` (unset sentinel) always
/// passes — its meaning is resolved per walk shape.
pub(crate) fn check_within_server_cap(
    limit: u32,
    surface: &str,
) -> Result<ServerCappedLimit, drive_proof_verifier::Error> {
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
    Ok(ServerCappedLimit(limit))
}

impl ServerCappedLimit {
    /// Distinct-walk (`RangeDistinctProof`) limit: `0` falls back
    /// to [`DEFAULT_QUERY_LIMIT`], mirroring the server's
    /// `limit.unwrap_or(DEFAULT_QUERY_LIMIT)`.
    pub(crate) fn distinct_walk_limit(&self) -> u16 {
        if self.0 == 0 {
            DEFAULT_QUERY_LIMIT
        } else {
            self.0 as u16
        }
    }

    /// Carrier-walk (`RangeAggregateCarrierProof`) limit for SUM /
    /// AVG: `0` stays `None` (unbounded outer walk), mirroring
    /// those servers keeping an unset request limit as `None`.
    /// COUNT must use [`Self::count_carrier_walk_limit`] instead —
    /// its dispatcher applies shape-dependent rules this converter
    /// does not know about.
    pub(crate) fn carrier_walk_limit(&self) -> Option<u16> {
        if self.0 == 0 {
            None
        } else {
            Some(self.0 as u16)
        }
    }

    /// COUNT carrier-walk (`RangeAggregateCarrierProof`) limit,
    /// mirroring the two shape-dependent rules of the COUNT
    /// dispatcher's carrier arm (`drive_document_count_query/
    /// drive_dispatcher.rs`) exactly — both change the
    /// proof-sensitive `SizedQuery::limit` bytes, so a mismatch
    /// either rejects honest proofs or verifies request/proof
    /// pairings no server produced:
    ///
    /// - **Range-outer carrier (G8)** — `GROUP BY` one range field
    ///   with two range clauses on distinct fields
    ///   (`has_outer_range`): the server lowers an unset limit to
    ///   the compile-time cap
    ///   [`MAX_CARRIER_AGGREGATE_OUTER_RANGE_LIMIT`] and refuses
    ///   explicit limits above it with `InvalidLimit` before
    ///   producing proof bytes.
    /// - **In-outer carrier (G7)**: the `In` array already bounds
    ///   the walk. An unset limit stays `None`; the server refuses
    ///   every explicit limit here, so the verifier must too.
    ///
    /// [`MAX_CARRIER_AGGREGATE_OUTER_RANGE_LIMIT`]:
    /// drive::query::drive_document_count_query::MAX_CARRIER_AGGREGATE_OUTER_RANGE_LIMIT
    pub(crate) fn count_carrier_walk_limit(
        &self,
        has_outer_range: bool,
    ) -> Result<Option<u16>, drive_proof_verifier::Error> {
        use drive::query::drive_document_count_query::MAX_CARRIER_AGGREGATE_OUTER_RANGE_LIMIT;

        if has_outer_range {
            if self.0 == 0 {
                return Ok(Some(MAX_CARRIER_AGGREGATE_OUTER_RANGE_LIMIT));
            }
            if self.0 > u32::from(MAX_CARRIER_AGGREGATE_OUTER_RANGE_LIMIT) {
                return Err(drive_proof_verifier::Error::RequestError {
                    error: format!(
                        "limit {} exceeds the carrier-aggregate range-outer cap \
                         {MAX_CARRIER_AGGREGATE_OUTER_RANGE_LIMIT} (COUNT); the server \
                         refuses such requests with InvalidLimit before producing proof \
                         bytes, so no proved response can belong to this request",
                        self.0
                    ),
                });
            }
            Ok(Some(self.0 as u16))
        } else {
            if self.0 != 0 {
                return Err(drive_proof_verifier::Error::RequestError {
                    error: format!(
                        "limit {} on a carrier-aggregate In-outer COUNT; the server refuses \
                         every explicit limit here (the In array bounds the walk), so no \
                         proved response can belong to this request",
                        self.0
                    ),
                });
            }
            Ok(None)
        }
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

    fn capped(limit: u32) -> ServerCappedLimit {
        check_within_server_cap(limit, "TEST").expect("limit within cap")
    }

    /// Distinct walks translate the `0` sentinel to
    /// `DEFAULT_QUERY_LIMIT`, mirroring the server's
    /// `limit.unwrap_or(DEFAULT_QUERY_LIMIT)`; in-range values pass
    /// through untouched.
    #[test]
    fn distinct_walk_sentinel_translation() {
        assert_eq!(capped(0).distinct_walk_limit(), DEFAULT_QUERY_LIMIT);
        assert_eq!(capped(1).distinct_walk_limit(), 1);
        assert_eq!(
            capped(u32::from(DEFAULT_MAX_QUERY_LIMIT)).distinct_walk_limit(),
            DEFAULT_MAX_QUERY_LIMIT
        );
    }

    /// SUM / AVG carrier walks keep the `0` sentinel as `None`
    /// (unbounded outer walk), mirroring those servers keeping an
    /// unset request limit as `None`; in-range values pass through
    /// untouched.
    #[test]
    fn carrier_walk_sentinel_translation() {
        assert_eq!(capped(0).carrier_walk_limit(), None);
        assert_eq!(capped(1).carrier_walk_limit(), Some(1));
        assert_eq!(
            capped(u32::from(DEFAULT_MAX_QUERY_LIMIT)).carrier_walk_limit(),
            Some(DEFAULT_MAX_QUERY_LIMIT)
        );
    }

    /// COUNT range-outer (G8) carriers mirror the COUNT
    /// dispatcher: `0` lowers to the compile-time carrier cap,
    /// explicit limits pass through up to that cap inclusively,
    /// and anything above it is rejected even though it fits the
    /// shared 100-item cap.
    #[test]
    fn count_carrier_range_outer_translation() {
        use drive::query::drive_document_count_query::MAX_CARRIER_AGGREGATE_OUTER_RANGE_LIMIT;
        let cap = MAX_CARRIER_AGGREGATE_OUTER_RANGE_LIMIT;

        assert_eq!(
            capped(0).count_carrier_walk_limit(true).unwrap(),
            Some(cap),
            "unset must lower to the carrier cap, not None"
        );
        assert_eq!(capped(1).count_carrier_walk_limit(true).unwrap(), Some(1));
        assert_eq!(
            capped(u32::from(cap))
                .count_carrier_walk_limit(true)
                .unwrap(),
            Some(cap)
        );

        let error = capped(u32::from(cap) + 1)
            .count_carrier_walk_limit(true)
            .expect_err("one past the carrier cap must be rejected");
        assert!(
            error
                .to_string()
                .contains("exceeds the carrier-aggregate range-outer cap"),
            "unexpected error: {error}"
        );
    }

    /// COUNT In-outer (G7) carriers mirror the COUNT dispatcher:
    /// `0` stays `None` (the In array bounds the walk) and every
    /// explicit limit is rejected.
    #[test]
    fn count_carrier_in_outer_translation() {
        assert_eq!(capped(0).count_carrier_walk_limit(false).unwrap(), None);

        let error = capped(1)
            .count_carrier_walk_limit(false)
            .expect_err("explicit In-outer limits must be rejected");
        assert!(
            error
                .to_string()
                .contains("carrier-aggregate In-outer COUNT"),
            "unexpected error: {error}"
        );
    }
}
