//! Verified average result + free-function proof verifiers for the
//! average surface.
//!
//! Average-side analog of [`super::document_sum::DocumentSum`].
//! Holds the `(count, sum)` pair recovered from a count-sum-bearing
//! tree proof (`CountSumTree` / `ProvableCountSumTree` /
//! `ProvableCountProvableSumTree`). `Aggregate` mode returns one
//! [`DocumentAverage`]; `Entries` mode returns
//! [`super::document_split_average::DocumentSplitAverages`] instead.
//!
//! Averages are NOT pre-divided server-side — the verifier surfaces
//! the raw `(count, sum)` and the caller divides. See the proto
//! file's `AverageResults` docstring for the rationale (precision +
//! client-chosen representation).
//!
//! The generic `FromProof<Q>` impl below intentionally rejects
//! calls (matching [`super::document_split_count::DocumentSplitCounts`]'s
//! pattern). Real dispatch lives in the
//! `FromProof<DocumentQuery>` impl in
//! `rs-sdk/src/platform/documents/document_average.rs`, which picks
//! among the free-function verifiers below based on the resolved
//! `DocumentAverageMode`.

use crate::error::MapGroveDbError;
use crate::verify::{document_query_proof_bytes, verify_tenderdash_proof};
use crate::{ContextProvider, Error};
use dapi_grpc::platform::v0::{Proof, ResponseMetadata};
use dpp::version::PlatformVersion;
use drive::query::drive_document_average_query::AverageEntry;
use drive::query::drive_document_sum_query::DriveDocumentSumQuery;

/// Verify a grovedb point-lookup proof against a count-sum-bearing
/// index terminator and return per-branch `(count, sum)` entries.
/// AVG analog of [`super::document_sum::verify_point_lookup_sum_proof`].
///
/// Thin tenderdash-composition wrapper over
/// [`DriveDocumentSumQuery::verify_point_lookup_count_and_sum_proof`].
/// Used by the prove path's `Aggregate` + Equal/In + no range
/// shape when the chosen index declares BOTH `summable: "<prop>"`
/// AND a `countable` terminator.
///
/// `platform_version` must be the protocol version `mtd` names, or the latest known
/// version when `mtd` names a newer one; otherwise the answer is refused with
/// [`Error::ProofProtocolVersionMismatch`].
pub fn verify_point_lookup_count_and_sum_proof(
    query: &DriveDocumentSumQuery,
    proof: &Proof,
    mtd: &ResponseMetadata,
    platform_version: &PlatformVersion,
    provider: &dyn ContextProvider,
) -> Result<Vec<AverageEntry>, Error> {
    let (root_hash, entries) = query
        .verify_point_lookup_count_and_sum_proof(
            document_query_proof_bytes(proof, mtd, platform_version)?,
            platform_version,
        )
        .map_drive_error(proof, mtd)?;

    verify_tenderdash_proof(proof, mtd, &root_hash, provider)?;

    Ok(entries)
}

/// Verify a per-distinct-key range-AVG proof against an index that
/// declares BOTH `rangeCountable: true` AND `rangeSummable: true`
/// (a `rangeAverageable: true` index) and return per-`(in_key,
/// key)` `(count, sum)` entries. AVG analog of
/// [`super::document_sum::verify_distinct_sum_proof`].
///
/// Thin tenderdash-composition wrapper over
/// [`DriveDocumentSumQuery::verify_distinct_count_and_sum_proof`].
/// Used by the prove path's `GroupByRange` / `GroupByCompound` +
/// range shape on the AVG surface.
///
/// `platform_version` must be the protocol version `mtd` names, or the latest known
/// version when `mtd` names a newer one; otherwise the answer is refused with
/// [`Error::ProofProtocolVersionMismatch`].
pub fn verify_distinct_count_and_sum_proof(
    query: &DriveDocumentSumQuery,
    proof: &Proof,
    mtd: &ResponseMetadata,
    limit: u16,
    left_to_right: bool,
    platform_version: &PlatformVersion,
    provider: &dyn ContextProvider,
) -> Result<Vec<AverageEntry>, Error> {
    let (root_hash, entries) = query
        .verify_distinct_count_and_sum_proof(
            document_query_proof_bytes(proof, mtd, platform_version)?,
            limit,
            left_to_right,
            platform_version,
        )
        .map_drive_error(proof, mtd)?;

    verify_tenderdash_proof(proof, mtd, &root_hash, provider)?;

    Ok(entries)
}

/// The `(count, sum)` pair across documents matching a query,
/// verified from proof. [`Self::to_decimal_string`] computes the exact
/// average `sum / count`; [`Self::as_f64`] an approximate one.
///
/// `count` is `u64` (counts are non-negative); `sum` is `i128`
/// (matching `DocumentSum`). The grovedb primitive that backs this
/// is `AggregateCountAndSumOnRange` (PCPS-leaf) for range-filtered
/// queries, or the primary-key count-sum-bearing element direct
/// read for empty-where queries on a
/// `documentsCountable + documentsSummable` doctype.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentAverage {
    /// Total matched-document count for the query.
    pub count: u64,
    /// Total aggregated value of the `sum_property` for the query.
    pub sum: i128,
}

impl DocumentAverage {
    /// Convenience: compute the average as `f64`. Returns `None`
    /// when `count == 0` (preserving the divide-by-zero contract
    /// rather than producing `NaN` / `inf`). An `f64` holds 53 bits
    /// of precision, so a large sum loses its low digits here; use
    /// [`Self::to_decimal_string`] for an exact average.
    pub fn as_f64(&self) -> Option<f64> {
        if self.count == 0 {
            None
        } else {
            Some(self.sum as f64 / self.count as f64)
        }
    }

    /// The average `sum / count` as a decimal string, exact to
    /// `fraction_digits` places ([`average_to_decimal_string`]).
    /// `None` when `count == 0`.
    pub fn to_decimal_string(&self, fraction_digits: u8) -> Option<String> {
        average_to_decimal_string(self.count, self.sum, fraction_digits)
    }
}

/// The average of `count` values that add up to `sum`, as a decimal
/// string with `fraction_digits` digits after the point, rounded half
/// away from zero at the last digit. `None` when `count == 0`: no
/// values have no average.
///
/// The division is done on integers, so the result is exact to the last
/// digit for every `sum` an `i128` holds, where an `f64` would already
/// lose digits past 2^53. A negative average that rounds to zero is
/// written without a sign (`"0.0000"`). `fraction_digits` is a `u8`, so
/// the work and the string are bounded (at most 255 places).
///
/// ```
/// use drive_proof_verifier::average_to_decimal_string;
///
/// assert_eq!(average_to_decimal_string(3, 10, 4).as_deref(), Some("3.3333"));
/// assert_eq!(average_to_decimal_string(3, -20, 2).as_deref(), Some("-6.67"));
/// assert_eq!(average_to_decimal_string(0, 10, 4), None);
/// ```
pub fn average_to_decimal_string(count: u64, sum: i128, fraction_digits: u8) -> Option<String> {
    if count == 0 {
        return None;
    }
    let count = u128::from(count);
    let magnitude = sum.unsigned_abs();
    let mut integer_part = magnitude / count;
    let mut remainder = magnitude % count;
    let mut fraction = Vec::with_capacity(usize::from(fraction_digits));
    for _ in 0..fraction_digits {
        // `remainder < count <= u64::MAX`, so ten times it fits a `u128`.
        remainder *= 10;
        fraction.push((remainder / count) as u8);
        remainder %= count;
    }
    // Round half away from zero on what is left: `remainder / count`
    // of one unit in the last place.
    if remainder * 2 >= count {
        let mut carry = true;
        for digit in fraction.iter_mut().rev() {
            if *digit == 9 {
                *digit = 0;
            } else {
                *digit += 1;
                carry = false;
                break;
            }
        }
        if carry {
            // `integer_part <= 2^127`, the magnitude of `i128::MIN`, so one more fits.
            integer_part += 1;
        }
    }
    let is_zero = integer_part == 0 && fraction.iter().all(|digit| *digit == 0);
    let mut text = String::new();
    if sum < 0 && !is_zero {
        text.push('-');
    }
    text.push_str(&integer_part.to_string());
    if fraction_digits > 0 {
        text.push('.');
        text.extend(fraction.iter().map(|digit| char::from(b'0' + digit)));
    }
    Some(text)
}

// No generic `FromProof<Q>` impl is provided here — see the
// `DocumentSum` docstring for the rationale. Callers reach this
// through `FromProof<DocumentQuery> for DocumentAverage` in
// `rs-sdk/src/platform/documents/document_average.rs`.

/// Verify a leaf-PCPS `AggregateCountAndSumOnRange` proof and the
/// surrounding tenderdash commit, returning the verified
/// `(count, sum)` pair. Used by the prove path's
/// `select=AVG, group_by=[]` with a range clause on an index that
/// declares BOTH `rangeCountable: true` AND `rangeSummable: true`
/// (i.e. the terminator is a `ProvableCountProvableSumTree`).
///
/// Thin tenderdash-composition wrapper over
/// [`DriveDocumentSumQuery::verify_aggregate_count_and_sum_proof`].
/// Both metrics come from one root-hash-committed traversal of the
/// PCPS terminator — no way for the server to splice a count from
/// one set with a sum from another.
///
/// `platform_version` must be the protocol version `mtd` names, or the latest known
/// version when `mtd` names a newer one; otherwise the answer is refused with
/// [`Error::ProofProtocolVersionMismatch`].
pub fn verify_aggregate_count_and_sum_proof(
    query: &DriveDocumentSumQuery,
    proof: &Proof,
    mtd: &ResponseMetadata,
    platform_version: &PlatformVersion,
    provider: &dyn ContextProvider,
) -> Result<(u64, i128), Error> {
    let (root_hash, count, sum) = query
        .verify_aggregate_count_and_sum_proof(
            document_query_proof_bytes(proof, mtd, platform_version)?,
            platform_version,
        )
        .map_drive_error(proof, mtd)?;

    verify_tenderdash_proof(proof, mtd, &root_hash, provider)?;

    Ok((count, sum))
}

/// Verify a grovedb proof of the document type's primary-key
/// count-sum-bearing element (`CountSumTree` /
/// `ProvableCountSumTree` / `ProvableCountProvableSumTree`) and
/// return the unfiltered `(count, sum)` pair.
///
/// Thin tenderdash-composition wrapper over
/// [`DriveDocumentSumQuery::verify_primary_key_count_sum_tree_proof`].
/// Used by the prove path's AVG fast path on a doctype that has
/// both `documentsCountable: true` and `documentsSummable: "<prop>"`
/// set, with empty where clauses — the server proves the
/// primary-key element directly and the SDK extracts both metrics
/// from one verified element.
///
/// `platform_version` must be the protocol version `mtd` names, or the latest known
/// version when `mtd` names a newer one; otherwise the answer is refused with
/// [`Error::ProofProtocolVersionMismatch`].
pub fn verify_primary_key_count_sum_tree_proof(
    contract_id: [u8; 32],
    document_type_name: &str,
    proof: &Proof,
    mtd: &ResponseMetadata,
    platform_version: &PlatformVersion,
    provider: &dyn ContextProvider,
) -> Result<(u64, i128), Error> {
    let (root_hash, count, sum) = DriveDocumentSumQuery::verify_primary_key_count_sum_tree_proof(
        document_query_proof_bytes(proof, mtd, platform_version)?,
        contract_id,
        document_type_name,
        platform_version,
    )
    .map_drive_error(proof, mtd)?;

    verify_tenderdash_proof(proof, mtd, &root_hash, provider)?;

    Ok((count, sum))
}

/// Verify a **carrier**-PCPS `AggregateCountAndSumOnRange` proof
/// and return the per-`In`-branch `(count, sum)` triples. AVG analog
/// of count's
/// [`super::document_count::verify_carrier_aggregate_count_proof`]
/// and sum's
/// [`super::document_sum::verify_carrier_aggregate_sum_proof`].
///
/// Thin tenderdash-composition wrapper over
/// [`DriveDocumentSumQuery::verify_carrier_aggregate_count_and_sum_proof`].
/// Used by the prove path when the request shape is `select=AVG,
/// group_by=[in_field], where = In(in_field) + range(other_field),
/// prove=true` against a PCPS-eligible index — drive routes it to
/// the carrier-PCPS executor.
///
/// Result: one [`AverageEntry`] per **present** In branch with
/// `in_key = None`, `key = <serialized In value>`, `count = Some(n)`,
/// `sum = Some(v)`. Absent In branches are omitted; the count and
/// sum axes never disagree on present/absent because the proof
/// commits both metrics from the same merk traversal.
///
/// Each total is keyed by the `In` value it belongs to (`in_key =
/// None`, `key = <serialized In value>`): the request groups by the `In`
/// field, so that value is its group, and this is the shape a point
/// lookup grouped by the `In` field gives.
/// A terminator `key` can itself be empty (a document missing an
/// optional range property is indexed under the empty key), so the
/// group is never inferred from an empty `key`.
///
/// `platform_version` must be the protocol version `mtd` names, or the latest known
/// version when `mtd` names a newer one; otherwise the answer is refused with
/// [`Error::ProofProtocolVersionMismatch`].
pub fn verify_carrier_aggregate_count_and_sum_proof(
    query: &DriveDocumentSumQuery,
    proof: &Proof,
    mtd: &ResponseMetadata,
    limit: Option<u16>,
    left_to_right: bool,
    platform_version: &PlatformVersion,
    provider: &dyn ContextProvider,
) -> Result<Vec<AverageEntry>, Error> {
    let (root_hash, per_key_count_sum) = query
        .verify_carrier_aggregate_count_and_sum_proof(
            document_query_proof_bytes(proof, mtd, platform_version)?,
            limit,
            left_to_right,
            platform_version,
        )
        .map_drive_error(proof, mtd)?;

    verify_tenderdash_proof(proof, mtd, &root_hash, provider)?;

    let entries = per_key_count_sum
        .into_iter()
        .map(|(in_value, count, sum)| AverageEntry {
            in_key: None,
            key: in_value,
            count: Some(count),
            sum: Some(sum),
        })
        .collect();
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_write_an_average_to_the_requested_places() {
        assert_eq!(
            average_to_decimal_string(3, 30, 4).as_deref(),
            Some("10.0000")
        );
        assert_eq!(
            average_to_decimal_string(3, 10, 4).as_deref(),
            Some("3.3333")
        );
        assert_eq!(
            average_to_decimal_string(3, 20, 4).as_deref(),
            Some("6.6667")
        );
        assert_eq!(average_to_decimal_string(8, 1, 2).as_deref(), Some("0.13"));
        assert_eq!(average_to_decimal_string(2, 7, 0).as_deref(), Some("4"));
        assert_eq!(average_to_decimal_string(3, 7, 0).as_deref(), Some("2"));
    }

    #[test]
    fn should_round_half_away_from_zero() {
        assert_eq!(average_to_decimal_string(8, 1, 2).as_deref(), Some("0.13"));
        assert_eq!(
            average_to_decimal_string(8, -1, 2).as_deref(),
            Some("-0.13")
        );
        assert_eq!(
            average_to_decimal_string(1, 999_995, 0).as_deref(),
            Some("999995")
        );
        assert_eq!(
            average_to_decimal_string(1000, 999_995, 2).as_deref(),
            Some("1000.00")
        );
        assert_eq!(
            average_to_decimal_string(1000, -999_995, 2).as_deref(),
            Some("-1000.00")
        );
    }

    #[test]
    fn should_write_a_negative_average_that_rounds_to_zero_without_a_sign() {
        assert_eq!(
            average_to_decimal_string(1000, -1, 2).as_deref(),
            Some("0.00")
        );
        assert_eq!(
            average_to_decimal_string(3, -10, 4).as_deref(),
            Some("-3.3333")
        );
    }

    #[test]
    fn should_write_the_widest_precision_a_u8_allows() {
        let average = average_to_decimal_string(3, 1, u8::MAX).expect("an average");
        assert_eq!(average.len(), "0.".len() + usize::from(u8::MAX));
        assert!(average.starts_with("0.3333"));
        assert!(average.ends_with('3'));
    }

    #[test]
    fn should_have_no_average_of_no_values() {
        assert_eq!(average_to_decimal_string(0, 0, 4), None);
        assert_eq!(average_to_decimal_string(0, 5, 4), None);
    }

    /// Three listings at the highest price an `i64` holds sum past 64 bits; their average
    /// is that price exactly, which an `f64` cannot write.
    #[test]
    fn should_average_a_sum_past_64_bits_exactly() {
        let average = DocumentAverage {
            count: 3,
            sum: 3 * i128::from(i64::MAX),
        };
        assert_eq!(
            average.to_decimal_string(4).as_deref(),
            Some("9223372036854775807.0000")
        );
        assert_eq!(
            average_to_decimal_string(2, 3 * i128::from(i64::MAX), 4).as_deref(),
            Some("13835058055282163710.5000")
        );
    }

    #[test]
    fn should_average_the_extremes_of_128_bits() {
        assert_eq!(
            average_to_decimal_string(1, i128::MAX, 1).as_deref(),
            Some("170141183460469231731687303715884105727.0")
        );
        assert_eq!(
            average_to_decimal_string(1, i128::MIN, 1).as_deref(),
            Some("-170141183460469231731687303715884105728.0")
        );
        assert_eq!(
            average_to_decimal_string(u64::MAX, i128::MAX, 2).as_deref(),
            Some("9223372036854775808.50")
        );
    }
}
