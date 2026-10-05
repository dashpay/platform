use super::{IntegerRangeTransform, TimeRangeTransform};

/// How an index buckets its first property into windows: by time
/// (`timeRange`) or by integer value (`integerRange`).
///
/// The storage layout, the document walkers and the query admissibility
/// rules are the same for both kinds, so they read this one type. What only
/// time has (the `ttl`, the `newest` / `oldest` selectors) stays on
/// [`TimeRangeTransform`], reached through [`Self::time_range`].
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum IndexBucketing {
    /// A `timeRange` grid over a system timestamp.
    Time(TimeRangeTransform),
    /// An `integerRange` grid over an integer property.
    Integer(IntegerRangeTransform),
}

impl From<TimeRangeTransform> for IndexBucketing {
    fn from(transform: TimeRangeTransform) -> Self {
        IndexBucketing::Time(transform)
    }
}

impl From<IntegerRangeTransform> for IndexBucketing {
    fn from(transform: IntegerRangeTransform) -> Self {
        IndexBucketing::Integer(transform)
    }
}

impl IndexBucketing {
    /// The bucketed property: always the index's first property.
    pub fn source(&self) -> &str {
        match self {
            IndexBucketing::Time(transform) => &transform.source,
            IndexBucketing::Integer(transform) => &transform.source,
        }
    }

    /// The contract keyword that declares this kind of grid.
    pub fn keyword(&self) -> &'static str {
        match self {
            IndexBucketing::Time(_) => super::TIME_RANGE,
            IndexBucketing::Integer(_) => super::INTEGER_RANGE,
        }
    }

    /// The grid-qualified GroveDB level key for `property_name`.
    pub fn storage_key(&self, property_name: &str) -> String {
        match self {
            IndexBucketing::Time(transform) => transform.storage_key(property_name),
            IndexBucketing::Integer(transform) => transform.storage_key(property_name),
        }
    }

    /// The number of windows one value falls in: `range / step`.
    pub fn overlap_factor(&self) -> u64 {
        match self {
            IndexBucketing::Time(transform) => transform.overlap_factor(),
            IndexBucketing::Integer(transform) => transform.overlap_factor(),
        }
    }

    /// The index-entry keys a document with the raw encoded source value is
    /// stored under: the one fan-out rule every walker derives its keys from.
    pub fn entry_keys_for_raw(&self, raw: &[u8]) -> Vec<Vec<u8>> {
        match self {
            IndexBucketing::Time(transform) => transform.entry_keys_for_raw(raw),
            IndexBucketing::Integer(transform) => transform.entry_keys_for_raw(raw),
        }
    }

    /// The time-range grid, when this is one.
    pub fn time_range(&self) -> Option<&TimeRangeTransform> {
        match self {
            IndexBucketing::Time(transform) => Some(transform),
            IndexBucketing::Integer(_) => None,
        }
    }

    /// The integer-range grid, when this is one.
    pub fn integer_range(&self) -> Option<&IntegerRangeTransform> {
        match self {
            IndexBucketing::Time(_) => None,
            IndexBucketing::Integer(transform) => Some(transform),
        }
    }
}
