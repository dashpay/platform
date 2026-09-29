use crate::data_contract::document_type::DocumentPropertyType;
use platform_value::Value;
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::hash::{Hash, Hasher};

/// The integer encoding an `integerRange` source is stored with: the width
/// and signedness of the source property's [`DocumentPropertyType`], which
/// the schema infers from the property's `minimum` / `maximum`.
///
/// A bucket start is stored exactly like a value of the source property, so
/// the stored key stays directly comparable to the source values it buckets
/// and a resolved window equality serializes through the ordinary
/// schema-driven key path. That makes the source type part of the bucket
/// math: it fixes the lowest start a key can hold (see
/// [`IntegerRangeTransform::containing_starts`]).
///
/// 128-bit properties are not bucketable: the bucket math runs in `i128`,
/// which holds every 64-bit value with room for a step below it, but not a
/// `u128`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde-conversion", derive(Serialize, Deserialize))]
pub enum IntegerRangeKeyType {
    U8,
    I8,
    U16,
    I16,
    U32,
    I32,
    U64,
    /// The default only stands in until the document-type parser resolves
    /// the source property's type; see [`IntegerRangeTransform::key_type`].
    #[default]
    I64,
}

impl IntegerRangeKeyType {
    /// The key type for a property type, or `None` when the property cannot
    /// be an `integerRange` source (not an integer, or 128 bits wide).
    pub fn from_property_type(property_type: &DocumentPropertyType) -> Option<Self> {
        match property_type {
            DocumentPropertyType::U8 => Some(Self::U8),
            DocumentPropertyType::I8 => Some(Self::I8),
            DocumentPropertyType::U16 => Some(Self::U16),
            DocumentPropertyType::I16 => Some(Self::I16),
            DocumentPropertyType::U32 => Some(Self::U32),
            DocumentPropertyType::I32 => Some(Self::I32),
            DocumentPropertyType::U64 => Some(Self::U64),
            DocumentPropertyType::I64 => Some(Self::I64),
            _ => None,
        }
    }

    /// The smallest value a key of this type can hold.
    pub fn min_value(self) -> i128 {
        match self {
            Self::U8 | Self::U16 | Self::U32 | Self::U64 => 0,
            Self::I8 => i8::MIN as i128,
            Self::I16 => i16::MIN as i128,
            Self::I32 => i32::MIN as i128,
            Self::I64 => i64::MIN as i128,
        }
    }

    /// The largest value a key of this type can hold.
    pub fn max_value(self) -> i128 {
        match self {
            Self::U8 => u8::MAX as i128,
            Self::I8 => i8::MAX as i128,
            Self::U16 => u16::MAX as i128,
            Self::I16 => i16::MAX as i128,
            Self::U32 => u32::MAX as i128,
            Self::I32 => i32::MAX as i128,
            Self::U64 => u64::MAX as i128,
            Self::I64 => i64::MAX as i128,
        }
    }

    /// The encoded width in bytes.
    pub fn width(self) -> usize {
        match self {
            Self::U8 | Self::I8 => 1,
            Self::U16 | Self::I16 => 2,
            Self::U32 | Self::I32 => 4,
            Self::U64 | Self::I64 => 8,
        }
    }

    /// Whether `value` fits in a key of this type.
    pub fn holds(self, value: i128) -> bool {
        (self.min_value()..=self.max_value()).contains(&value)
    }

    /// `value` encoded exactly like a value of the source property, or
    /// `None` when it does not fit.
    pub fn encode(self, value: i128) -> Option<Vec<u8>> {
        Some(match self {
            Self::U8 => DocumentPropertyType::encode_u8(u8::try_from(value).ok()?),
            Self::I8 => DocumentPropertyType::encode_i8(i8::try_from(value).ok()?),
            Self::U16 => DocumentPropertyType::encode_u16(u16::try_from(value).ok()?),
            Self::I16 => DocumentPropertyType::encode_i16(i16::try_from(value).ok()?),
            Self::U32 => DocumentPropertyType::encode_u32(u32::try_from(value).ok()?),
            Self::I32 => DocumentPropertyType::encode_i32(i32::try_from(value).ok()?),
            Self::U64 => DocumentPropertyType::encode_u64(u64::try_from(value).ok()?),
            Self::I64 => DocumentPropertyType::encode_i64(i64::try_from(value).ok()?),
        })
    }

    /// `value` as the platform value a query equality on the source carries:
    /// `I64` when it fits, `U64` above that. `None` when the key type cannot
    /// hold it. Every window start the grid produces
    /// ([`IntegerRangeTransform::containing_starts`]) or accepts
    /// ([`IntegerRangeTransform::is_window_start`]) is held, so this is the
    /// one conversion the query resolver and the uniqueness probe share.
    pub fn value_of(self, value: i128) -> Option<Value> {
        if !self.holds(value) {
            return None;
        }
        match i64::try_from(value) {
            Ok(value) => Some(Value::I64(value)),
            Err(_) => u64::try_from(value).ok().map(Value::U64),
        }
    }

    /// Decodes a stored key of this type. Only an input of exactly this
    /// type's width decodes: the fixed-width decoders read a prefix of any
    /// longer input, and an empty input would index past its end.
    pub fn decode(self, raw: &[u8]) -> Option<i128> {
        if raw.len() != self.width() {
            return None;
        }
        Some(match self {
            Self::U8 => DocumentPropertyType::decode_u8(raw)? as i128,
            Self::I8 => DocumentPropertyType::decode_i8(raw)? as i128,
            Self::U16 => DocumentPropertyType::decode_u16(raw)? as i128,
            Self::I16 => DocumentPropertyType::decode_i16(raw)? as i128,
            Self::U32 => DocumentPropertyType::decode_u32(raw)? as i128,
            Self::I32 => DocumentPropertyType::decode_i32(raw)? as i128,
            Self::U64 => DocumentPropertyType::decode_u64(raw)? as i128,
            Self::I64 => DocumentPropertyType::decode_i64(raw)? as i128,
        })
    }
}

/// An index-level transform that buckets an integer index property into
/// fixed-length, regularly-spaced windows: the integer counterpart of
/// [`super::TimeRangeTransform`].
///
/// Windows start at `phase + k * step` for every integer `k` (negative
/// included, for signed sources) and cover `[start, start + range)`. When
/// `range > step` they overlap, so a value falls in `range / step` windows
/// and a document is indexed under that many bucket starts. The parameters
/// are in the source property's own units.
///
/// **The lowest window is clamped.** A start below the smallest value the
/// source's key type can hold ([`IntegerRangeKeyType::min_value`], `0` for
/// an unsigned property) cannot be stored, so every such start is raised to
/// that minimum: the windows that begin below it merge into one window that
/// starts at the minimum and ends where the last of them ends. That keeps
/// every value the property can hold in at least one window. When the
/// minimum is itself on the grid (a phase-0 grid over an unsigned property,
/// say) the clamped windows are subsets of the minimum's own window and the
/// clamp only drops them.
///
/// At the GroveDB storage layer the bucketed first property gets its own
/// index level keyed by [`Self::storage_key`], exactly like a time-range
/// grid; within it a bucket start is an ordinary key segment, so index
/// queries, count trees and proofs apply unchanged.
// The serde keys match the contract grammar (`on` / `range` / `step` /
// `phase`, see the `integerRange` entry in the v3 document meta-schema), so
// a serialized `Index` round-trips into the key set a contract author
// writes. `key_type` is not part of the grammar and is re-derived from the
// schema on every parse, so it is also left out of equality, ordering and
// hashing: two transforms are the same grid when the contract declares the
// same `on` / `range` / `step` / `phase`. A change of the source's integer
// width is the property's change, refused by the document type's integer
// encoding check, not an index definition change.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde-conversion", derive(Serialize, Deserialize))]
pub struct IntegerRangeTransform {
    /// The integer index property this transform buckets. Must be the first
    /// property of the index, a required integer property of at most 64
    /// bits (validated where the document schema is in scope).
    #[cfg_attr(feature = "serde-conversion", serde(rename = "on"))]
    pub source: String,
    /// Length of each window. A positive multiple of `step`.
    pub range: u64,
    /// Interval between successive window starts. Greater than zero.
    pub step: u64,
    /// Grid alignment: window starts are `phase + k * step`. Strictly less
    /// than `step`, so every grid has one spelling. Defaults to `0`.
    #[cfg_attr(feature = "serde-conversion", serde(default))]
    pub phase: u64,
    /// The source property's key encoding. The index grammar alone cannot
    /// see property types, so `Index` parsing leaves the default and the
    /// document-type parser (`parse_indices`), which every contract load
    /// runs, overwrites it with the source's real type before anything
    /// reads it.
    #[cfg_attr(feature = "serde-conversion", serde(skip))]
    pub key_type: IntegerRangeKeyType,
}

impl IntegerRangeTransform {
    /// The declared grid, the identity equality, ordering and hashing use.
    fn grid(&self) -> (&str, u64, u64, u64) {
        (self.source.as_str(), self.range, self.step, self.phase)
    }
}

impl PartialEq for IntegerRangeTransform {
    fn eq(&self, other: &Self) -> bool {
        self.grid() == other.grid()
    }
}

impl Eq for IntegerRangeTransform {}

impl PartialOrd for IntegerRangeTransform {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for IntegerRangeTransform {
    fn cmp(&self, other: &Self) -> Ordering {
        self.grid().cmp(&other.grid())
    }
}

impl Hash for IntegerRangeTransform {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.grid().hash(state);
    }
}

impl IntegerRangeTransform {
    /// The number of windows that contain any given value away from the
    /// clamped bottom: `range / step`. `0` only for a malformed zero-step
    /// transform, which a validated contract never carries.
    pub fn overlap_factor(&self) -> u64 {
        if self.step == 0 {
            return 0;
        }
        self.range / self.step
    }

    /// The GroveDB index-level key for this grid over `property_name`:
    /// `{property}#{range}#{step}`, with `#{phase}` appended iff the phase is
    /// non-zero. The same shape as a time-range grid's key; the two can never
    /// meet, since a time-range source is a system timestamp and an integer
    /// source a user property. `#` never appears in a property name, so the
    /// key cannot collide with a plain property level.
    ///
    /// **The single source of the key encoding**: contract setup, the
    /// document walkers, query path derivation, the uniqueness probe and
    /// proof verification all derive the level key through here (via
    /// [`super::Index::level_key`]).
    pub fn storage_key(&self, property_name: &str) -> String {
        if self.phase == 0 {
            format!("{}#{}#{}", property_name, self.range, self.step)
        } else {
            format!(
                "{}#{}#{}#{}",
                property_name, self.range, self.step, self.phase
            )
        }
    }

    /// The largest grid start `phase + k * step` that is `<= value`, before
    /// clamping. `None` only for a malformed zero-step transform.
    pub fn most_recent_start(&self, value: i128) -> Option<i128> {
        if self.step == 0 {
            return None;
        }
        let step = self.step as i128;
        let phase = self.phase as i128;
        Some(phase + (value - phase).div_euclid(step) * step)
    }

    /// The starts of every window containing `value`, newest first, as they
    /// are stored: each start below the key type's minimum is raised to it
    /// and the duplicates this creates are dropped (see the type docs).
    /// Empty only for a malformed zero-step transform, or a value outside
    /// the key type, which no document can hold.
    ///
    /// Stops at the first clamped start: every older start is lower still,
    /// so it would clamp to the same key.
    pub fn containing_starts(&self, value: i128) -> Vec<i128> {
        if !self.key_type.holds(value) {
            return Vec::new();
        }
        let Some(newest) = self.most_recent_start(value) else {
            return Vec::new();
        };
        let step = self.step as i128;
        let min = self.key_type.min_value();
        let mut starts = Vec::new();
        for j in 0..self.overlap_factor() as i128 {
            let start = newest - j * step;
            if start <= min {
                starts.push(min);
                break;
            }
            starts.push(start);
        }
        starts
    }

    /// A value in as many windows as the key type allows: `range` above the
    /// key type's minimum, so no window it falls in reaches down to the
    /// clamped bottom (or the key type's maximum, when the type is narrower
    /// than that). Cost estimates price a typical document with it instead
    /// of a value near the bottom, which sits in fewer windows.
    pub fn full_fan_out_value(&self) -> i128 {
        self.key_type
            .min_value()
            .saturating_add(self.range as i128)
            .min(self.key_type.max_value())
    }

    /// Whether `start` names a window of this grid: a grid start the key
    /// type can hold, or the key type's minimum when that is not on the
    /// grid (the clamped bottom window).
    ///
    /// The validity check for a `byStart` selection of an `IN_INTEGER_RANGE`
    /// query: an off-grid start names no window, and the resolver rejects it
    /// rather than snapping. Whether the window holds any documents is not
    /// checked: an empty window is a provable empty answer.
    pub fn is_window_start(&self, start: i128) -> bool {
        if self.step == 0 || !self.key_type.holds(start) {
            return false;
        }
        start == self.key_type.min_value()
            || (start - self.phase as i128).rem_euclid(self.step as i128) == 0
    }

    /// The set of index-entry keys a document with the given raw encoded
    /// value for the bucketed property is stored under.
    ///
    /// **The single source of truth for the fan-out rule**: the insert,
    /// delete and update walkers all derive their entry keys through this
    /// function (via [`super::IndexBucketing::entry_keys_for_raw`]).
    ///
    /// - An empty `raw` (null / absent) keeps the single ordinary null entry.
    ///   A validated source is required, so documents never reach it.
    /// - A value of the key type's exact width yields one key per containing
    ///   window: its start, encoded exactly like the value.
    /// - Anything else keeps its raw key, as a non-bucketed index stores it.
    pub fn entry_keys_for_raw(&self, raw: &[u8]) -> Vec<Vec<u8>> {
        if raw.is_empty() {
            return vec![Vec::new()];
        }
        match self.key_type.decode(raw) {
            Some(value) => self
                .containing_starts(value)
                .into_iter()
                .filter_map(|start| self.key_type.encode(start))
                .collect(),
            None => vec![raw.to_vec()],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn transform(
        range: u64,
        step: u64,
        phase: u64,
        key_type: IntegerRangeKeyType,
    ) -> IntegerRangeTransform {
        IntegerRangeTransform {
            source: "price".to_string(),
            range,
            step,
            phase,
            key_type,
        }
    }

    #[test]
    fn should_place_a_value_in_every_overlapping_window() {
        let t = transform(300, 100, 0, IntegerRangeKeyType::U32);
        assert_eq!(t.overlap_factor(), 3);
        assert_eq!(t.containing_starts(750), vec![700, 600, 500]);
        for start in t.containing_starts(750) {
            assert!(start <= 750 && 750 < start + 300);
        }
        // a boundary value opens its own window
        assert_eq!(t.containing_starts(700), vec![700, 600, 500]);
    }

    #[test]
    fn should_place_a_value_in_one_window_when_range_equals_step() {
        let t = transform(100, 100, 0, IntegerRangeKeyType::U16);
        assert_eq!(t.containing_starts(0), vec![0]);
        assert_eq!(t.containing_starts(99), vec![0]);
        assert_eq!(t.containing_starts(100), vec![100]);
        assert_eq!(t.containing_starts(65_535), vec![65_500]);
    }

    #[test]
    fn should_shift_window_boundaries_by_the_phase() {
        let t = transform(100, 100, 25, IntegerRangeKeyType::I32);
        assert_eq!(t.containing_starts(25), vec![25]);
        assert_eq!(t.containing_starts(124), vec![25]);
        assert_eq!(t.containing_starts(125), vec![125]);
        assert!(t.is_window_start(125));
        assert!(!t.is_window_start(100));
    }

    #[test]
    fn should_floor_negative_values_to_the_window_below() {
        let t = transform(100, 100, 0, IntegerRangeKeyType::I32);
        assert_eq!(t.containing_starts(-1), vec![-100]);
        assert_eq!(t.containing_starts(-100), vec![-100]);
        assert_eq!(t.containing_starts(-101), vec![-200]);
        let overlapping = transform(300, 100, 0, IntegerRangeKeyType::I32);
        assert_eq!(overlapping.containing_starts(-50), vec![-100, -200, -300]);
    }

    #[test]
    fn should_drop_windows_below_zero_for_an_unsigned_phase_zero_grid() {
        // Starts -100 and -200 clamp to 0, which is already the value's
        // newest window: the clamp only drops them.
        let t = transform(300, 100, 0, IntegerRangeKeyType::U16);
        assert_eq!(t.containing_starts(50), vec![0]);
        assert_eq!(t.containing_starts(150), vec![100, 0]);
        assert_eq!(t.containing_starts(250), vec![200, 100, 0]);
    }

    #[test]
    fn should_clamp_the_bottom_window_to_the_type_minimum() {
        // Unsigned with a phase: values below the first grid start fall in
        // the window that would start at -50, clamped to 0.
        let t = transform(100, 100, 50, IntegerRangeKeyType::U8);
        assert_eq!(t.containing_starts(0), vec![0]);
        assert_eq!(t.containing_starts(49), vec![0]);
        assert_eq!(t.containing_starts(50), vec![50]);
        // the top window runs past the type's maximum and is simply partial
        assert_eq!(t.containing_starts(255), vec![250]);
        assert!(t.is_window_start(0));
        assert!(t.is_window_start(250));
        assert!(!t.is_window_start(100));
        assert!(!t.is_window_start(350));

        // Signed with overlap: the windows starting at -200, -300 and -400
        // all clamp to -128 and merge into one key.
        let t = transform(300, 100, 0, IntegerRangeKeyType::I8);
        assert_eq!(t.containing_starts(-120), vec![-128]);
        assert_eq!(t.containing_starts(-50), vec![-100, -128]);
        assert_eq!(t.containing_starts(50), vec![0, -100, -128]);
        assert_eq!(t.containing_starts(127), vec![100, 0, -100]);
        assert!(t.is_window_start(-128));
        assert!(t.is_window_start(-100));
        assert!(!t.is_window_start(-200));
    }

    #[test]
    fn should_bucket_the_extremes_of_64_bit_types() {
        let t = transform(100, 100, 0, IntegerRangeKeyType::U64);
        let max = u64::MAX as i128;
        assert_eq!(t.containing_starts(max), vec![max - max % 100]);
        let t = transform(300, 100, 0, IntegerRangeKeyType::I64);
        let min = i64::MIN as i128;
        assert_eq!(t.containing_starts(min), vec![min]);
        assert_eq!(t.containing_starts(i64::MAX as i128).len(), 3);
        // a step wider than the whole type puts every value in one window
        let wide = transform(u64::MAX, u64::MAX, 0, IntegerRangeKeyType::I64);
        assert_eq!(wide.containing_starts(-5), vec![min]);
        assert_eq!(wide.containing_starts(5), vec![0]);
    }

    #[test]
    fn should_derive_entry_keys_from_the_source_encoding() {
        let t = transform(300, 100, 0, IntegerRangeKeyType::I16);
        // null keeps the single ordinary null entry
        assert_eq!(t.entry_keys_for_raw(&[]), vec![Vec::<u8>::new()]);
        let raw = DocumentPropertyType::encode_i16(-50);
        assert_eq!(
            t.entry_keys_for_raw(&raw),
            vec![
                DocumentPropertyType::encode_i16(-100),
                DocumentPropertyType::encode_i16(-200),
                DocumentPropertyType::encode_i16(-300),
            ]
        );
        // a value of another width keeps its raw key rather than decoding a
        // prefix or indexing past the end
        assert_eq!(t.entry_keys_for_raw(&[1]), vec![vec![1]]);
        assert_eq!(t.entry_keys_for_raw(&[1, 2, 3]), vec![vec![1, 2, 3]]);
    }

    #[test]
    fn should_keep_bucket_keys_in_the_order_of_the_values() {
        // Stored keys sort like the starts they encode, so a range over
        // bucket keys walks windows in order.
        let t = transform(10, 10, 0, IntegerRangeKeyType::I32);
        let keys: Vec<Vec<u8>> = [-30i128, -20, -10, 0, 10, 20]
            .into_iter()
            .map(|start| t.key_type.encode(start).expect("fits"))
            .collect();
        let mut sorted = keys.clone();
        sorted.sort();
        assert_eq!(keys, sorted);
    }

    #[test]
    fn should_qualify_the_storage_key_with_the_grid() {
        assert_eq!(
            transform(300, 100, 0, IntegerRangeKeyType::U32).storage_key("price"),
            "price#300#100"
        );
        assert_eq!(
            transform(300, 100, 7, IntegerRangeKeyType::U32).storage_key("price"),
            "price#300#100#7"
        );
    }

    #[test]
    fn should_answer_a_malformed_zero_step_without_dividing_by_zero() {
        let t = transform(100, 0, 0, IntegerRangeKeyType::U32);
        assert_eq!(t.overlap_factor(), 0);
        assert_eq!(t.most_recent_start(5), None);
        assert!(t.containing_starts(5).is_empty());
        assert!(!t.is_window_start(0));
    }

    #[test]
    fn should_pick_a_sample_value_in_every_window() {
        let t = transform(300, 100, 0, IntegerRangeKeyType::U32);
        assert_eq!(t.containing_starts(t.full_fan_out_value()).len(), 3);
        let t = transform(300, 100, 50, IntegerRangeKeyType::I8);
        assert_eq!(t.containing_starts(t.full_fan_out_value()).len(), 3);
        // a type narrower than one window keeps what the type allows
        let t = transform(1_000, 100, 0, IntegerRangeKeyType::U8);
        assert_eq!(t.full_fan_out_value(), 255);
    }

    #[test]
    fn should_compare_transforms_by_the_declared_grid_only() {
        let narrow = transform(300, 100, 0, IntegerRangeKeyType::U16);
        let wide = transform(300, 100, 0, IntegerRangeKeyType::U32);
        assert_eq!(narrow, wide);
        assert_ne!(narrow, transform(300, 100, 5, IntegerRangeKeyType::U16));
    }

    #[test]
    fn should_convert_held_starts_to_query_values() {
        assert_eq!(
            IntegerRangeKeyType::I8.value_of(-128),
            Some(Value::I64(-128))
        );
        assert_eq!(
            IntegerRangeKeyType::U64.value_of(u64::MAX as i128),
            Some(Value::U64(u64::MAX))
        );
        assert_eq!(IntegerRangeKeyType::U8.value_of(-1), None);
        assert_eq!(IntegerRangeKeyType::U8.value_of(256), None);
    }

    #[test]
    fn should_hold_no_windows_for_values_outside_the_key_type() {
        let t = transform(100, 100, 0, IntegerRangeKeyType::U8);
        assert!(t.containing_starts(256).is_empty());
        assert!(t.containing_starts(-1).is_empty());
        assert!(!t.is_window_start(300));
    }
}
