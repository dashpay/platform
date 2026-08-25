#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};

/// An index-level transform that buckets a timestamp index property into
/// fixed-length, regularly-spaced time ranges.
///
/// The window is **declared in seconds** and **identified in milliseconds**.
/// A contract author writes `range` / `step` / `phase` as second counts
/// because the finest clock a bucket selection ever sees is block time, whose
/// target interval is five seconds — a window declared to the millisecond
/// would be precision the protocol cannot deliver. A time range is still
/// identified by a single `u64`: the **start time of the range** as a
/// millisecond timestamp, because the source fields (`$createdAt` &co.) are
/// millisecond timestamps and the stored index key has to stay directly
/// comparable to them. [`Self::range_ms`] and its siblings are the one place
/// the two units meet.
///
/// Each range covers `[start, start + range)`. New ranges start every `step`,
/// on the grid `phase + k * step` — `phase` is a pure alignment offset,
/// validated to be strictly less than `step`, so the grid covers all of time
/// (bar the sub-`step` sliver at the epoch that no real timestamp can ever
/// fall into). When `range > step` the ranges overlap, so a single timestamp
/// falls into `range / step` ranges (the "overlap factor") and a document is
/// indexed under that many bucket-start values.
///
/// The canonical use case is "trending" leaderboards: index on
/// `(timeRange($createdAt), hashtag)` with `countable`, then query a single
/// bucket — e.g. per-hashtag counts within the bucket (`COUNT(*)` grouped by
/// `hashtag`, with the client ordering the returned groups). Overlapping
/// ranges guarantee that, at any instant, there is always an active range
/// covering a near-full `range` window of history (see
/// [`Self::oldest_active_start`]).
///
/// Note that the *server-ordered* form (`ORDER BY COUNT(*)` — the ranked
/// query surface) cannot yet be combined with a time-range selection: ranked
/// queries accept no where clauses in this protocol version (their routing
/// deliberately has no equality-prefix support yet), so "top K by count
/// within the bucket" is served as the grouped count above with client-side
/// ordering until ranked prefix routing lands at a future protocol version.
///
/// At the GroveDB storage layer, a transformed first property gets its own
/// index level keyed by [`Self::storage_key`] — the property name qualified
/// with the grid — so several grids over the same timestamp coexist as
/// sibling subtrees. Within a grid's subtree a bucket start is an ordinary
/// `u64` key segment (encoded exactly like a `$createdAt` value), so existing
/// index queries, count trees and proofs apply unchanged — the only novelty
/// is that one document produces several index entries.
// The serde keys deliberately match the contract grammar (`on` / `range` /
// `step` / `phase`, see the `timeRange` entry in the v3 document
// meta-schema), so a serialized `Index` round-trips into the same key set a
// contract author writes rather than a second, camelCased spelling.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde-conversion", derive(Serialize, Deserialize))]
pub struct TimeRangeTransform {
    /// The source timestamp index property this transform buckets. Must be
    /// the first property of the index and must name one of the system
    /// timestamps (`$createdAt` / `$updatedAt` / `$transferredAt`); the
    /// document-schema grammar has no user property type that parses to a
    /// millisecond timestamp, so user-defined sources are rejected at
    /// contract validation until such a representation exists.
    #[cfg_attr(feature = "serde-conversion", serde(rename = "on"))]
    pub source: String,
    /// Length of each range window, in seconds. Must be a positive multiple
    /// of `step_seconds`. The window a document is measured against is this
    /// length expressed in milliseconds ([`Self::range_ms`]), since bucket
    /// starts are millisecond timestamps.
    #[cfg_attr(feature = "serde-conversion", serde(rename = "range"))]
    pub range_seconds: u64,
    /// Interval between successive range starts, in seconds. Must be greater
    /// than zero. Consecutive bucket starts are [`Self::step_ms`] apart on the
    /// millisecond timeline.
    #[cfg_attr(feature = "serde-conversion", serde(rename = "step"))]
    pub step_seconds: u64,
    /// Grid alignment phase, in seconds. Range starts are the millisecond
    /// timestamps `phase_ms() + k * step_ms()` for `k = 0, 1, 2, …`. A pure
    /// phase offset: contract validation requires `phase < step`, so shifting
    /// the grid never excludes any real timestamp — it only moves where the
    /// window boundaries fall (e.g. daily windows cut at 06:00 UTC instead of
    /// midnight). Defaults to `0`.
    #[cfg_attr(feature = "serde-conversion", serde(rename = "phase", default))]
    pub phase_seconds: u64,
}

impl TimeRangeTransform {
    /// The window length on the millisecond timeline.
    ///
    /// The three `*_ms` accessors are the single crossing point between the
    /// transform's two units: the parameters are seconds because that is the
    /// resolution a contract author can meaningfully declare, while every
    /// quantity the bucket math consumes or produces — a document's
    /// `$createdAt`, a bucket start, a stored index key — is a millisecond
    /// timestamp, so the parameters have to be scaled before they can take
    /// part.
    ///
    /// Saturating rather than checked: contract validation rejects any
    /// parameter above `u64::MAX / 1_000`, so a transform that came from a
    /// validated contract can never reach the ceiling. A transform built
    /// outside validation degrades into a pinned-to-the-maximum window
    /// instead of panicking — the same defensive posture as the
    /// `step_seconds == 0` handling below, which returns `None` / an empty
    /// bucket set rather than dividing by zero.
    pub fn range_ms(&self) -> u64 {
        self.range_seconds.saturating_mul(1_000)
    }

    /// The interval between successive range starts on the millisecond
    /// timeline. See [`Self::range_ms`] for why the accessor exists and why
    /// it saturates.
    pub fn step_ms(&self) -> u64 {
        self.step_seconds.saturating_mul(1_000)
    }

    /// The grid alignment phase on the millisecond timeline — the first
    /// range's start. See [`Self::range_ms`] for why the accessor exists and
    /// why it saturates.
    pub fn phase_ms(&self) -> u64 {
        self.phase_seconds.saturating_mul(1_000)
    }

    /// The number of overlapping ranges that contain any given instant, i.e.
    /// the number of bucket-start values a single document is indexed under.
    /// Equal to `range / step`.
    ///
    /// A ratio of two same-unit quantities, so it is unit-invariant and reads
    /// the declared seconds directly rather than scaling both sides first.
    ///
    /// Returns `0` only for a malformed transform with a zero step; callers
    /// constructing from a validated contract never observe that.
    pub fn overlap_factor(&self) -> u64 {
        if self.step_seconds == 0 {
            return 0;
        }
        self.range_seconds / self.step_seconds
    }

    /// The GroveDB index-level key for this grid over `property_name`: the
    /// property name qualified with the grid parameters, so different grids
    /// over the same timestamp fork into sibling subtrees instead of
    /// colliding in one keyspace (every 6-hour bucket start is also a
    /// 3-hour bucket start — unqualified, the two grids' entries would be
    /// indistinguishable).
    ///
    /// **This is the single source of the key encoding.** Contract setup,
    /// the insert/delete/update walkers, query path derivation, the
    /// uniqueness probe and proof verification must all derive the level key
    /// through this function; a second implementation that disagreed on any
    /// detail would split one logical index into two trees.
    ///
    /// Format: `{property}#{range}#{step}` with `#{phase}` appended **iff**
    /// the phase is non-zero — omitted-means-zero is canonical (nothing ever
    /// writes `#0`), mirroring the contract grammar where `phase` is an
    /// omittable key. The numbers are the contract-declared **seconds**
    /// verbatim, which are already canonical. `#` can never appear in a
    /// schema property name (`^[a-zA-Z0-9-_]{1,64}$`, dot-joined for nested
    /// paths), so a qualified key can never collide with a plain property
    /// level.
    pub fn storage_key(&self, property_name: &str) -> String {
        if self.phase_seconds == 0 {
            format!(
                "{}#{}#{}",
                property_name, self.range_seconds, self.step_seconds
            )
        } else {
            format!(
                "{}#{}#{}#{}",
                property_name, self.range_seconds, self.step_seconds, self.phase_seconds
            )
        }
    }

    /// The start of the most recent range that has begun at or before the
    /// millisecond timestamp `t`, i.e. the largest `phase + k * step` that is
    /// `<= t`.
    ///
    /// Returns `None` for `t` before the phase anchor — with a validated
    /// transform (`phase < step`) that is only the sub-`step` sliver at the
    /// epoch, which no real timestamp reaches; the arm is defensive. (Also
    /// `None` for a malformed zero-step transform, which a validated contract
    /// can never carry.)
    pub fn most_recent_start(&self, t: u64) -> Option<u64> {
        let (step_ms, phase_ms) = (self.step_ms(), self.phase_ms());
        if step_ms == 0 || t < phase_ms {
            return None;
        }
        let elapsed = t - phase_ms;
        Some(phase_ms + (elapsed / step_ms) * step_ms)
    }

    /// All bucket-start values whose range `[start, start + range)` contains
    /// the millisecond timestamp `t`. This is the set of index entries a
    /// document with timestamp `t` must be written under.
    ///
    /// The result is sorted in descending order (newest range first) and has
    /// exactly [`Self::overlap_factor`] elements, except within the first
    /// `range` after the epoch where fewer ranges have started. For `t`
    /// before the phase anchor (the sub-`step` epoch sliver no real timestamp
    /// reaches) the result is empty — insert, delete and update all share
    /// this rule, keeping the index consistent.
    pub fn containing_buckets(&self, t: u64) -> Vec<u64> {
        let overlap = self.overlap_factor();
        if overlap == 0 {
            return Vec::new();
        }
        let Some(newest) = self.most_recent_start(t) else {
            return Vec::new();
        };
        let (step_ms, phase_ms) = (self.step_ms(), self.phase_ms());
        (0..overlap)
            .filter_map(|j| {
                let offset = j.checked_mul(step_ms)?;
                newest.checked_sub(offset)
            })
            .filter(|start| *start >= phase_ms)
            .collect()
    }

    /// The start of the newest range that is active at the millisecond
    /// timestamp `now` (the freshest started range). Querying this bucket
    /// returns documents from the latest partial slice — between `0` and one
    /// `step` of history.
    ///
    /// Returns `None` only when `now` predates the phase anchor (the epoch
    /// sliver; unreachable for real block times).
    pub fn newest_active_start(&self, now: u64) -> Option<u64> {
        self.most_recent_start(now)
    }

    /// The start of the oldest range still active at the millisecond timestamp
    /// `now`. Its window `[start, start + range)` still contains `now`, so
    /// querying this bucket returns a near-full trailing window of `~range` of
    /// history (between `range - step` and `range`). This is the bucket to
    /// query for "trending over the last range window".
    ///
    /// Returns `None` only when `now` predates the phase anchor (the epoch
    /// sliver; unreachable for real block times).
    pub fn oldest_active_start(&self, now: u64) -> Option<u64> {
        let overlap = self.overlap_factor();
        if overlap == 0 {
            return None;
        }
        let newest = self.most_recent_start(now)?;
        let back = (overlap - 1).saturating_mul(self.step_ms());
        Some(newest.saturating_sub(back).max(self.phase_ms()))
    }

    /// The set of index-entry keys a document with the given raw encoded
    /// value for the bucketed property must be stored under.
    ///
    /// **This is the single source of truth for the fan-out rule**: the
    /// insert, delete and update walkers in rs-drive must all derive their
    /// entry keys through this function, or a document written by one walker
    /// becomes unfindable by another (a consensus break on
    /// update-after-insert).
    ///
    /// - An empty `raw` (the null / property-absent case) keeps the single
    ///   ordinary null entry every index gives null values.
    /// - A decodable millisecond timestamp yields one key per containing
    ///   bucket — the bucket *start*, encoded exactly like the timestamp
    ///   itself. (A timestamp inside the sub-`step` epoch sliver before the
    ///   phase anchor yields no keys; no real timestamp reaches it.)
    /// - A non-empty value that fails to decode keeps its raw key, exactly as
    ///   a non-time-range index would store it.
    pub fn entry_keys_for_raw(&self, raw: &[u8]) -> Vec<Vec<u8>> {
        use crate::data_contract::document_type::DocumentPropertyType;
        if raw.is_empty() {
            return vec![Vec::new()];
        }
        match DocumentPropertyType::decode_date_timestamp(raw) {
            Some(timestamp) => self
                .containing_buckets(timestamp)
                .into_iter()
                .map(DocumentPropertyType::encode_date_timestamp)
                .collect(),
            None => vec![raw.to_vec()],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One hour as the transform declares it (seconds) and as every timestamp
    /// below is expressed (milliseconds).
    const HOUR_SECONDS: u64 = 3_600;
    const HOUR_MS: u64 = 3_600_000;

    fn transform() -> TimeRangeTransform {
        // range = 6h, step = 2h, phase = 0 → overlap factor 3.
        TimeRangeTransform {
            source: "$createdAt".to_string(),
            range_seconds: 6 * HOUR_SECONDS,
            step_seconds: 2 * HOUR_SECONDS,
            phase_seconds: 0,
        }
    }

    #[test]
    fn overlap_factor_is_range_over_step() {
        assert_eq!(transform().overlap_factor(), 3);
    }

    #[test]
    fn seconds_that_cannot_be_scaled_saturate_rather_than_panic() {
        // Contract validation refuses parameters this large, so the only way
        // to build one is in code; the accessors must degrade into a pinned
        // window instead of panicking on the multiplication.
        let t = TimeRangeTransform {
            source: "$createdAt".to_string(),
            range_seconds: u64::MAX,
            step_seconds: u64::MAX,
            phase_seconds: 0,
        };
        assert_eq!(t.range_ms(), u64::MAX);
        assert_eq!(t.overlap_factor(), 1);
        assert_eq!(t.most_recent_start(u64::MAX), Some(u64::MAX));
    }

    #[test]
    fn most_recent_start_floors_to_step_multiple() {
        let t = transform();
        let h = HOUR_MS;
        // now = 7h → most recent start = 6h
        assert_eq!(t.most_recent_start(7 * h), Some(6 * h));
        // exactly on a boundary stays put
        assert_eq!(t.most_recent_start(6 * h), Some(6 * h));
        // exactly at the epoch is the first range
        assert_eq!(t.most_recent_start(0), Some(0));
    }

    #[test]
    fn the_epoch_sliver_before_the_phase_has_no_buckets() {
        // A one-minute window stepping every twenty seconds, its grid phased
        // five seconds into each step. The only timestamps outside every
        // range are the first five seconds of 1970 — a sliver no real
        // timestamp reaches; the math must still answer it honestly.
        let t = TimeRangeTransform {
            source: "$createdAt".to_string(),
            range_seconds: 60,
            step_seconds: 20,
            phase_seconds: 5,
        };
        assert_eq!(t.most_recent_start(4_999), None);
        assert_eq!(t.containing_buckets(4_999), Vec::<u64>::new());
        assert_eq!(t.newest_active_start(4_999), None);
        assert_eq!(t.oldest_active_start(4_999), None);
        // at the phase anchor the first range starts
        assert_eq!(t.most_recent_start(5_000), Some(5_000));
        assert_eq!(t.containing_buckets(5_000), vec![5_000]);
        // every returned bucket actually contains the timestamp
        for now in [5_000u64, 15_000, 64_999, 105_000] {
            for start in t.containing_buckets(now) {
                assert!(start <= now && now < start + t.range_ms());
            }
        }
    }

    #[test]
    fn containing_buckets_are_the_overlapping_ranges() {
        let t = transform();
        let h = HOUR_MS;
        // doc at 7h belongs to ranges starting at 6h, 4h, 2h
        assert_eq!(t.containing_buckets(7 * h), vec![6 * h, 4 * h, 2 * h]);
        // every returned range actually contains the timestamp
        for start in t.containing_buckets(7 * h) {
            assert!(start <= 7 * h && 7 * h < start + t.range_ms());
        }
    }

    #[test]
    fn containing_buckets_truncate_near_the_epoch() {
        let t = transform();
        let h = HOUR_MS;
        // doc at 3h: ranges starting at 2h and 0h (4h start would be in future)
        assert_eq!(t.containing_buckets(3 * h), vec![2 * h, 0]);
    }

    #[test]
    fn newest_vs_oldest_active() {
        let t = transform();
        let h = HOUR_MS;
        let now = 7 * h;
        // newest active = freshest started range
        assert_eq!(t.newest_active_start(now), Some(6 * h));
        // oldest active = covers the full trailing window
        assert_eq!(t.oldest_active_start(now), Some(2 * h));
        // oldest active range still contains now
        let oldest = t.oldest_active_start(now).expect("a range is active");
        assert!(oldest <= now && now < oldest + t.range_ms());
    }

    #[test]
    fn entry_keys_follow_the_shared_fan_out_rule() {
        use crate::data_contract::document_type::DocumentPropertyType;
        let t = transform();
        let h = HOUR_MS;
        // null keeps the single ordinary null entry
        assert_eq!(t.entry_keys_for_raw(&[]), vec![Vec::<u8>::new()]);
        // a decodable timestamp fans out into its containing buckets
        let raw = DocumentPropertyType::encode_date_timestamp(7 * h);
        assert_eq!(
            t.entry_keys_for_raw(&raw),
            vec![
                DocumentPropertyType::encode_date_timestamp(6 * h),
                DocumentPropertyType::encode_date_timestamp(4 * h),
                DocumentPropertyType::encode_date_timestamp(2 * h),
            ]
        );
        // an undecodable non-empty value keeps its raw key
        assert_eq!(t.entry_keys_for_raw(&[1, 2, 3]), vec![vec![1, 2, 3]]);
        // a timestamp inside the epoch sliver belongs to no range: no keys
        let t_phased = TimeRangeTransform {
            source: "$createdAt".to_string(),
            range_seconds: 60,
            step_seconds: 20,
            phase_seconds: 5,
        };
        let raw = DocumentPropertyType::encode_date_timestamp(4_999);
        assert_eq!(t_phased.entry_keys_for_raw(&raw), Vec::<Vec<u8>>::new());
    }

    #[test]
    fn phase_shifts_alignment() {
        let t = TimeRangeTransform {
            source: "$createdAt".to_string(),
            range_seconds: 60,
            step_seconds: 20,
            phase_seconds: 5,
        };
        // starts are the 5th, 25th, 45th, ... second; now = the 50th second →
        // most recent start is the 45th
        assert_eq!(t.most_recent_start(50_000), Some(45_000));
        assert_eq!(t.containing_buckets(50_000), vec![45_000, 25_000, 5_000]);
    }

    #[test]
    fn storage_keys_qualify_the_property_with_the_grid() {
        // phase 0 is spelled by omission — the canonical three-part form
        assert_eq!(
            transform().storage_key("$createdAt"),
            "$createdAt#21600#7200"
        );
        // a non-zero phase appends the fourth part
        let t = TimeRangeTransform {
            source: "$createdAt".to_string(),
            range_seconds: 60,
            step_seconds: 20,
            phase_seconds: 5,
        };
        assert_eq!(t.storage_key("$createdAt"), "$createdAt#60#20#5");
        // two grids over the same property produce distinct sibling keys —
        // the collision the qualification exists to prevent (every 6h start
        // is also a 3h start, so unqualified keys would interleave)
        let six_hourly = TimeRangeTransform {
            source: "$createdAt".to_string(),
            range_seconds: 6 * HOUR_SECONDS,
            step_seconds: 6 * HOUR_SECONDS,
            phase_seconds: 0,
        };
        let three_hourly = TimeRangeTransform {
            source: "$createdAt".to_string(),
            range_seconds: 3 * HOUR_SECONDS,
            step_seconds: 3 * HOUR_SECONDS,
            phase_seconds: 0,
        };
        assert_ne!(
            six_hourly.storage_key("$createdAt"),
            three_hourly.storage_key("$createdAt")
        );
    }
}
