//! Index declarations covering the native index catalogue, except index
//! properties read from a referenced document and indexes on the elements of a
//! typed array, which are not declarable yet.
//!
//! Combinations that native rejects (a range count without a count, a ranking
//! without the matching range axis, a prefix ranking with a sum axis, a time
//! range on a user property) are not re-validated here: native validation is
//! the authority and the build crate surfaces its errors.

use alloc::string::String;
use alloc::vec::Vec;

use super::DeclarationOrigin;
use crate::identity::{IndexName, PropertyName, PropertyPath};

/// Count fast path on an index.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Countability {
    /// Plain tree, no count.
    #[default]
    NotCountable,
    /// Count tree: totals in constant time.
    Countable,
    /// Provable count tree: totals plus offset and range queries.
    CountableAllowingOffset,
}

impl Countability {
    /// Whether either count form is selected.
    pub fn is_countable(&self) -> bool {
        !matches!(self, Countability::NotCountable)
    }
}

/// Contested index resolution.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ContestedResolution {
    /// Masternodes vote for a contender, abstain, or lock the value so nobody
    /// gets it.
    #[default]
    MasternodeVote,
    /// Masternodes vote for a contender or abstain; there is no lock choice,
    /// so the contest always ends with a winner, and a lone contender at the
    /// end of the join window is awarded at once.
    MasternodeVoteNoLocking,
}

/// Contested index parameters. The native award is an internal native action;
/// declaring these parameters is the only way a contract influences it.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ContestedSpec {
    /// Index property to regular expression the value must match to be
    /// contested.
    pub field_matches: Vec<(PropertyPath, String)>,
    /// How a contest is resolved.
    pub resolution: ContestedResolution,
    /// Human description.
    pub description: Option<String>,
}

impl ContestedSpec {
    /// A masternode-vote contest with the given field matches.
    pub fn masternode_vote(field_matches: Vec<(PropertyPath, String)>) -> Self {
        ContestedSpec {
            field_matches,
            resolution: ContestedResolution::MasternodeVote,
            description: None,
        }
    }

    /// Adds a description.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Sets the resolution.
    pub fn resolution(mut self, resolution: ContestedResolution) -> Self {
        self.resolution = resolution;
        self
    }
}

/// Where one ranking axis is placed.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum RankedLevels {
    /// No ranking on this axis.
    #[default]
    None,
    /// At the terminal level.
    Terminal,
    /// At the named index property levels, which may include prefixes. An
    /// explicit list names at least one level.
    At(Vec<PropertyPath>),
}

/// Ranking axes; each costs its own secondary tree. Which axes may sit at a
/// prefix level, and beside which others, is native validation.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Ranking {
    /// Count ranking.
    pub count: RankedLevels,
    /// Sum ranking.
    pub sum: RankedLevels,
    /// Average ranking.
    pub average: RankedLevels,
}

impl Ranking {
    /// The three axes with their option names, in declaration order.
    pub fn axes(&self) -> [(&'static str, &RankedLevels); 3] {
        [
            ("ranked_count", &self.count),
            ("ranked_sum", &self.sum),
            ("ranked_average", &self.average),
        ]
    }

    fn axes_mut(&mut self) -> [&mut RankedLevels; 3] {
        [&mut self.count, &mut self.sum, &mut self.average]
    }

    /// A copy with every level list sorted; level order carries no meaning.
    pub fn normalized(&self) -> Ranking {
        let mut normalized = self.clone();
        for axis in normalized.axes_mut() {
            if let RankedLevels::At(levels) = axis {
                levels.sort();
            }
        }
        normalized
    }
}

/// Which absent properties keep a document out of the index.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum SkipIfAbsent {
    /// Every document is indexed; a missing value is indexed as null.
    #[default]
    Never,
    /// A document missing any optional property of the index writes no entry
    /// (native `skipIfAbsent: true`).
    Optional,
    /// A document missing any of the named index properties writes no entry
    /// (native `skipIfAbsent: [...]`). An explicit list names at least one
    /// property; order carries no meaning.
    Properties(Vec<PropertyPath>),
}

/// Bucketing of the index's first property into time windows.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TimeRangeSpec {
    /// The bucketed property; must be the index's first property and a system
    /// timestamp.
    pub on: PropertyPath,
    /// Window length in seconds.
    pub range_secs: u64,
    /// Interval between window starts in seconds.
    pub step_secs: u64,
    /// Grid offset in seconds.
    pub phase_secs: u64,
    /// Time to live in seconds: entries expire this long after their bucket
    /// starts and expired windows are not queryable. `None` keeps entries
    /// indefinitely. The native cap and the shared-grid rules are native
    /// validation.
    pub ttl_secs: Option<u64>,
}

/// Bucketing of the index's first property, an integer, into value windows.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct IntegerRangeSpec {
    /// The bucketed property; must be the index's first property.
    pub on: PropertyPath,
    /// Window width in units of the property.
    pub range: u64,
    /// Interval between window starts.
    pub step: u64,
    /// Grid offset, default 0.
    pub phase: u64,
}

impl IntegerRangeSpec {
    /// Windows of `range` starting every `step` on `on`, with no offset. The
    /// grid limits (a multiple, at most 24 windows per value, a phase below
    /// the step) are native validation.
    pub fn new(on: PropertyPath, range: u64, step: u64) -> Self {
        IntegerRangeSpec {
            on,
            range,
            step,
            phase: 0,
        }
    }

    /// Offsets the window grid.
    pub fn phase(mut self, phase: u64) -> Self {
        self.phase = phase;
        self
    }
}

/// Index-only options: only meaningful when the collection is index-only.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct IndexOnlySpec {
    /// The member key: one property, or an ordered list of properties whose
    /// encoded values are concatenated (a composite terminal). `$ownerId`
    /// when empty. Order is part of the key and is preserved.
    pub terminal: Vec<PropertyPath>,
    /// Preallocate the index path when the referenced document is created.
    pub preallocated: bool,
    /// On a time-range index with a time to live: a delete leaves the entries
    /// to expire with their window, and a create over an existing entry
    /// writes over it.
    pub outlives_delete: bool,
    /// Keep one counter per group holding the number of entries of the named
    /// source index of the same collection, and no entry per document.
    pub summable_off_count_index: Option<IndexName>,
}

/// An index declaration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IndexSpec {
    /// Where the spec came from.
    pub origin: DeclarationOrigin,
    /// The index's identity within its collection.
    pub name: IndexName,
    /// Indexed property paths in order, all ascending.
    pub properties: Vec<PropertyPath>,
    /// Unique index.
    pub unique: bool,
    /// Null values are searchable, default true.
    pub null_searchable: bool,
    /// Contested parameters.
    pub contested: Option<ContestedSpec>,
    /// Count fast path. `None` when the author said nothing, so that average
    /// sugar may promote it; an explicit `Some(NotCountable)` next to
    /// `average` is a conflict, as it is natively.
    pub count: Option<Countability>,
    /// Range counts; explicitness as for `count`.
    pub range_count: Option<bool>,
    /// Integer property summed at the index.
    pub sum: Option<PropertyName>,
    /// Range sums; explicitness as for `count`.
    pub range_sum: Option<bool>,
    /// Sugar for `count` plus `sum`; expanded by the validator.
    pub average: Option<PropertyName>,
    /// Sugar for `range_count` plus `range_sum`; expanded by the validator.
    pub range_average: bool,
    /// Ranking axes.
    pub ranked: Ranking,
    /// Time range bucketing.
    pub time_range: Option<TimeRangeSpec>,
    /// Integer value bucketing.
    pub integer_range: Option<IntegerRangeSpec>,
    /// Conditional participation.
    pub skip_if_absent: SkipIfAbsent,
    /// Index-only options.
    pub index_only: Option<IndexOnlySpec>,
}

impl IndexSpec {
    /// A plain ascending index over the given properties.
    pub fn new(name: IndexName, properties: Vec<PropertyPath>) -> Self {
        IndexSpec {
            origin: DeclarationOrigin::Builder,
            name,
            properties,
            unique: false,
            null_searchable: true,
            contested: None,
            count: None,
            range_count: None,
            sum: None,
            range_sum: None,
            average: None,
            range_average: false,
            ranked: Ranking::default(),
            time_range: None,
            integer_range: None,
            skip_if_absent: SkipIfAbsent::Never,
            index_only: None,
        }
    }

    /// Records the origin.
    pub fn with_origin(mut self, origin: DeclarationOrigin) -> Self {
        self.origin = origin;
        self
    }

    /// Makes the index unique.
    pub fn unique(mut self, unique: bool) -> Self {
        self.unique = unique;
        self
    }

    /// Sets null searchability.
    pub fn null_searchable(mut self, null_searchable: bool) -> Self {
        self.null_searchable = null_searchable;
        self
    }

    /// Makes the index contested.
    pub fn contested(mut self, contested: ContestedSpec) -> Self {
        self.contested = Some(contested);
        self
    }

    /// Selects a count tree.
    pub fn count(self) -> Self {
        self.countability(Countability::Countable)
    }

    /// Selects a provable count tree.
    pub fn count_allowing_offset(self) -> Self {
        self.countability(Countability::CountableAllowingOffset)
    }

    /// Sets the count fast path explicitly, including `NotCountable`.
    pub fn countability(mut self, count: Countability) -> Self {
        self.count = Some(count);
        self
    }

    /// Enables range counts.
    pub fn range_count(mut self, range_count: bool) -> Self {
        self.range_count = Some(range_count);
        self
    }

    /// Sums a property at the index.
    pub fn sum(mut self, property: PropertyName) -> Self {
        self.sum = Some(property);
        self
    }

    /// Enables range sums.
    pub fn range_sum(mut self, range_sum: bool) -> Self {
        self.range_sum = Some(range_sum);
        self
    }

    /// Average sugar: `count` plus `sum` on the property.
    pub fn average(mut self, property: PropertyName) -> Self {
        self.average = Some(property);
        self
    }

    /// Range average sugar: `range_count` plus `range_sum`.
    pub fn range_average(mut self, range_average: bool) -> Self {
        self.range_average = range_average;
        self
    }

    /// Count ranking at the terminal level.
    pub fn ranked_count(mut self) -> Self {
        self.ranked.count = RankedLevels::Terminal;
        self
    }

    /// Count ranking at the named levels.
    pub fn ranked_count_at(mut self, levels: Vec<PropertyPath>) -> Self {
        self.ranked.count = RankedLevels::At(levels);
        self
    }

    /// Sum ranking at the terminal level, or none.
    pub fn ranked_sum(mut self, ranked_sum: bool) -> Self {
        self.ranked.sum = terminal_or_none(ranked_sum);
        self
    }

    /// Sum ranking at the named levels.
    pub fn ranked_sum_at(mut self, levels: Vec<PropertyPath>) -> Self {
        self.ranked.sum = RankedLevels::At(levels);
        self
    }

    /// Average ranking at the terminal level, or none.
    pub fn ranked_average(mut self, ranked_average: bool) -> Self {
        self.ranked.average = terminal_or_none(ranked_average);
        self
    }

    /// Average ranking at the named levels.
    pub fn ranked_average_at(mut self, levels: Vec<PropertyPath>) -> Self {
        self.ranked.average = RankedLevels::At(levels);
        self
    }

    /// Buckets the first property into time windows.
    pub fn time_range(mut self, time_range: TimeRangeSpec) -> Self {
        self.time_range = Some(time_range);
        self
    }

    /// Buckets the first property, an integer, into value windows.
    pub fn integer_range(mut self, integer_range: IntegerRangeSpec) -> Self {
        self.integer_range = Some(integer_range);
        self
    }

    /// Sets conditional participation.
    pub fn skip_if_absent(mut self, skip_if_absent: SkipIfAbsent) -> Self {
        self.skip_if_absent = skip_if_absent;
        self
    }

    /// Sets index-only options.
    pub fn index_only(mut self, options: IndexOnlySpec) -> Self {
        self.index_only = Some(options);
        self
    }

    /// A copy with every order-insensitive member in canonical order:
    /// contested field matches by property, ranked levels and skip properties
    /// by property.
    pub fn normalized(&self) -> IndexSpec {
        let mut normalized = self.clone();
        if let Some(contested) = normalized.contested.as_mut() {
            contested.field_matches.sort();
        }
        normalized.ranked = normalized.ranked.normalized();
        if let SkipIfAbsent::Properties(properties) = &mut normalized.skip_if_absent {
            properties.sort();
        }
        normalized
    }
}

fn terminal_or_none(enabled: bool) -> RankedLevels {
    if enabled {
        RankedLevels::Terminal
    } else {
        RankedLevels::None
    }
}
