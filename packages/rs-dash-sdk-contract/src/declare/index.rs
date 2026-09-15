//! Index declarations covering the full native index catalogue.
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
    /// Masternode vote, the only native resolution.
    #[default]
    MasternodeVote,
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
}

/// Where count rankings are placed.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum RankedCount {
    /// No count ranking.
    #[default]
    None,
    /// At the terminal level.
    Terminal,
    /// At the named index property levels, which may include prefixes.
    At(Vec<PropertyPath>),
}

/// Ranking axes; each costs its own secondary tree.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Ranking {
    /// Count ranking.
    pub count: RankedCount,
    /// Sum ranking at the terminal level.
    pub sum: bool,
    /// Average ranking at the terminal level.
    pub average: bool,
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
}

/// Index-only options: only meaningful when the collection is index-only.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct IndexOnlySpec {
    /// The member key property; `$ownerId` when absent.
    pub terminal: Option<PropertyPath>,
    /// Preallocate the index path when the referenced document is created.
    pub preallocated: bool,
    /// Write no entry when the first property is absent.
    pub skip_if_absent: bool,
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
        self.ranked.count = RankedCount::Terminal;
        self
    }

    /// Count ranking at the named levels.
    pub fn ranked_count_at(mut self, levels: Vec<PropertyPath>) -> Self {
        self.ranked.count = RankedCount::At(levels);
        self
    }

    /// Sum ranking.
    pub fn ranked_sum(mut self, ranked_sum: bool) -> Self {
        self.ranked.sum = ranked_sum;
        self
    }

    /// Average ranking.
    pub fn ranked_average(mut self, ranked_average: bool) -> Self {
        self.ranked.average = ranked_average;
        self
    }

    /// Buckets the first property into time windows.
    pub fn time_range(mut self, time_range: TimeRangeSpec) -> Self {
        self.time_range = Some(time_range);
        self
    }

    /// Sets index-only options.
    pub fn index_only(mut self, options: IndexOnlySpec) -> Self {
        self.index_only = Some(options);
        self
    }

    /// A copy with every order-insensitive member in canonical order:
    /// contested field matches by property and ranked levels by property.
    pub fn normalized(&self) -> IndexSpec {
        let mut normalized = self.clone();
        if let Some(contested) = normalized.contested.as_mut() {
            contested.field_matches.sort();
        }
        if let RankedCount::At(levels) = &mut normalized.ranked.count {
            levels.sort();
        }
        normalized
    }
}
