#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, PartialOrd, Clone, Eq)]
#[cfg_attr(feature = "serde-conversion", derive(Serialize, Deserialize))]
pub enum OrderBy {
    #[cfg_attr(feature = "serde-conversion", serde(rename = "asc"))]
    Asc,
    #[cfg_attr(feature = "serde-conversion", serde(rename = "desc"))]
    Desc,
}

use crate::data_contract::errors::DataContractError;

use crate::ProtocolError;
use anyhow::anyhow;

use crate::data_contract::document_type::ContestedIndexResolution::MasternodeVote;
#[cfg(feature = "validation")]
use crate::data_contract::errors::DataContractError::RegexError;
use platform_value::{Value, ValueMap};
use regex::Regex;
use std::cmp::Ordering;
use std::sync::OnceLock;
use std::{collections::BTreeMap, convert::TryFrom};

pub mod random_index;

/// Index-level keyword opting the index's terminal property-name tree into the
/// **Count** ranking axis: an ordered secondary tree keyed by each group's
/// document count, so "top / bottom K groups by count" is O(log n + k) with a
/// proof. Requires `rangeCountable: true` on the same index. Only recognized
/// from document meta-schema v3 (protocol version 14); see
/// [`Index::try_from_value_map`].
pub const RANKED_COUNTABLE: &str = "rankedCountable";
/// Index-level keyword opting the index's terminal property-name tree into the
/// **Sum** ranking axis (ordered by each group's sum of the `summable`
/// property). Requires `rangeSummable` on the same index. Meta-schema v3+.
pub const RANKED_SUMMABLE: &str = "rankedSummable";
/// Index-level keyword opting the index's terminal property-name tree into the
/// **Avg** ranking axis (ordered by each group's average of the `averageable`
/// property). Requires `rangeAverageable` semantics — i.e. both range axes.
/// Meta-schema v3+.
///
/// Deliberately *not* sugar for the other two ranked flags: each ranking axis
/// costs its own secondary tree, so `rankedAverageable` adds the Avg axis and
/// nothing else.
pub const RANKED_AVERAGEABLE: &str = "rankedAverageable";

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Ord, PartialOrd)]
#[cfg_attr(feature = "serde-conversion", derive(Serialize, Deserialize))]
pub enum ContestedIndexResolution {
    MasternodeVote = 0,
}

impl TryFrom<u8> for ContestedIndexResolution {
    type Error = ProtocolError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(MasternodeVote),
            value => Err(ProtocolError::UnknownStorageKeyRequirements(format!(
                "contested index resolution unknown: {}",
                value
            ))),
        }
    }
}

#[repr(u8)]
#[derive(Debug)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(
        into = "ContestedIndexFieldMatchRepr",
        from = "ContestedIndexFieldMatchRepr"
    )
)]
pub enum ContestedIndexFieldMatch {
    Regex(LazyRegex),
    PositiveIntegerMatch(u128),
}

// Internal-`$type` serde shape with a uniform `value` payload, via a
// struct-variant Repr (tuple variants can't auto-internal-tag). `LazyRegex`
// round-trips as a bare string; the `u128` uses `json_safe_u128_content` rather
// than the plain `json_safe_u128` because internal tagging buffers the map
// through serde's `Content`, which can't hold a `u128` — see that helper's docs.
#[cfg(feature = "serde-conversion")]
#[derive(Serialize, Deserialize)]
#[serde(tag = "$type", rename_all = "camelCase")]
enum ContestedIndexFieldMatchRepr {
    Regex {
        value: LazyRegex,
    },
    PositiveIntegerMatch {
        #[serde(with = "crate::serialization::json_safe_u128_content")]
        value: u128,
    },
}

#[cfg(feature = "serde-conversion")]
impl From<ContestedIndexFieldMatch> for ContestedIndexFieldMatchRepr {
    fn from(m: ContestedIndexFieldMatch) -> Self {
        match m {
            ContestedIndexFieldMatch::Regex(value) => Self::Regex { value },
            ContestedIndexFieldMatch::PositiveIntegerMatch(value) => {
                Self::PositiveIntegerMatch { value }
            }
        }
    }
}

#[cfg(feature = "serde-conversion")]
impl From<ContestedIndexFieldMatchRepr> for ContestedIndexFieldMatch {
    fn from(r: ContestedIndexFieldMatchRepr) -> Self {
        match r {
            ContestedIndexFieldMatchRepr::Regex { value } => Self::Regex(value),
            ContestedIndexFieldMatchRepr::PositiveIntegerMatch { value } => {
                Self::PositiveIntegerMatch(value)
            }
        }
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(from = "String", into = "String")
)]
pub struct LazyRegex {
    regex: OnceLock<Regex>,
    regex_str: String,
}

#[cfg(feature = "serde-conversion")]
impl From<String> for LazyRegex {
    fn from(regex_str: String) -> Self {
        LazyRegex::new(regex_str)
    }
}

#[cfg(feature = "serde-conversion")]
impl From<LazyRegex> for String {
    fn from(value: LazyRegex) -> Self {
        value.regex_str
    }
}

impl LazyRegex {
    pub fn new(regex_str: String) -> Self {
        LazyRegex {
            regex: OnceLock::new(),
            regex_str,
        }
    }

    pub fn is_match(&self, string: &str) -> bool {
        let regexp = self
            .regex
            .get_or_init(|| Regex::new(&self.regex_str).expect("valid regexp"));

        regexp.is_match(string)
    }

    pub fn as_str(&self) -> &str {
        self.regex_str.as_str()
    }
}

// Manual Serialize/Deserialize impls deleted in Phase D step 11.
// The previous custom Serialize emitted PascalCase variant tags
// (`{"Regex": ...}`) while the custom Deserialize expected snake_case
// (`{"regex": ...}`) — non-round-trippable. The replacement uses serde
// `rename_all = "camelCase"` matching the rest of the codebase's
// JSON wire-shape convention. `LazyRegex` round-trips as a plain
// string via `serde(from = "String", into = "String")` above.

#[allow(clippy::non_canonical_partial_ord_impl)]
impl PartialOrd for ContestedIndexFieldMatch {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        use ContestedIndexFieldMatch::*;
        match (self, other) {
            // Comparing two integers
            (PositiveIntegerMatch(a), PositiveIntegerMatch(b)) => a.partial_cmp(b),

            // Arbitrarily decide that any Regex is less than any PositiveIntegerMatch
            (Regex(_), PositiveIntegerMatch(_)) => Some(Ordering::Less),
            (PositiveIntegerMatch(_), Regex(_)) => Some(Ordering::Greater),

            // Comparing Regex with Regex, perhaps based on pattern length
            (Regex(a), Regex(b)) => a.as_str().len().partial_cmp(&b.as_str().len()),
        }
    }
}

impl Ord for ContestedIndexFieldMatch {
    fn cmp(&self, other: &Self) -> Ordering {
        use ContestedIndexFieldMatch::*;
        match (self, other) {
            // Directly compare integers
            (PositiveIntegerMatch(a), PositiveIntegerMatch(b)) => a.cmp(b),

            // Compare Regex based on pattern string length
            (Regex(a), Regex(b)) => a.as_str().len().cmp(&b.as_str().len()),

            // Regex is considered less than a positive integer
            (Regex(_), PositiveIntegerMatch(_)) => Ordering::Less,
            (PositiveIntegerMatch(_), Regex(_)) => Ordering::Greater,
        }
    }
}

impl Clone for ContestedIndexFieldMatch {
    fn clone(&self) -> Self {
        match self {
            ContestedIndexFieldMatch::Regex(regex) => {
                ContestedIndexFieldMatch::Regex(regex.clone())
            }
            ContestedIndexFieldMatch::PositiveIntegerMatch(int) => {
                ContestedIndexFieldMatch::PositiveIntegerMatch(*int)
            }
        }
    }
}

impl PartialEq for ContestedIndexFieldMatch {
    fn eq(&self, other: &Self) -> bool {
        match self {
            ContestedIndexFieldMatch::Regex(regex) => match other {
                ContestedIndexFieldMatch::Regex(other_regex) => {
                    regex.as_str() == other_regex.as_str()
                }
                _ => false,
            },
            ContestedIndexFieldMatch::PositiveIntegerMatch(int) => match other {
                ContestedIndexFieldMatch::PositiveIntegerMatch(other_int) => int == other_int,
                _ => false,
            },
        }
    }
}

impl Eq for ContestedIndexFieldMatch {}

impl ContestedIndexFieldMatch {
    pub fn matches(&self, value: &Value) -> bool {
        match self {
            ContestedIndexFieldMatch::Regex(regex) => {
                if let Some(string) = value.as_str() {
                    regex.is_match(string)
                } else {
                    false
                }
            }
            ContestedIndexFieldMatch::PositiveIntegerMatch(int) => value
                .as_integer::<u128>()
                .map(|i| i == *int)
                .unwrap_or(false),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Ord, PartialOrd)]
#[cfg_attr(feature = "serde-conversion", derive(Serialize, Deserialize))]
pub struct ContestedIndexInformation {
    pub field_matches: BTreeMap<String, ContestedIndexFieldMatch>,
    pub resolution: ContestedIndexResolution,
}

impl Default for ContestedIndexInformation {
    fn default() -> Self {
        ContestedIndexInformation {
            field_matches: BTreeMap::new(),
            resolution: ContestedIndexResolution::MasternodeVote,
        }
    }
}

/// What countable operations the index's tree supports.
///
/// - `NotCountable` — plain `NormalTree`. Counts on this index require enumerating
///   documents (no fast path).
/// - `Countable` — `CountTree`. The total count of documents under any covering
///   equality / `In` prefix is an O(1) read (or O(distinct values) for partial
///   prefixes).
/// - `CountableAllowingOffset` — `ProvableCountTree`. Same total-count semantics
///   as `Countable`, plus every internal node carries the count of its left and
///   right subtrees, so future range / offset queries (e.g. "the next 50 items
///   starting after key X") will be answerable in O(log n) without enumerating.
///
/// `CountableAllowingOffset` is strictly more capable than `Countable` but also
/// strictly more expensive (every node carries count metadata, not just the
/// root). Pick `Countable` when you only need totals; pick
/// `CountableAllowingOffset` when you also need range/offset queries on this
/// index.
///
/// **Note on `unique` indexes.** A unique index stores its terminal as a bare
/// `Reference` at key `[0]` rather than wrapping it in a `CountTree`, so for
/// documents whose indexed fields are *all* non-null the `countable` flag is a
/// no-op at the storage level. It still does meaningful work for **null-bearing**
/// entries: when a document has any null value among the indexed properties,
/// insertion takes the same count-tree branch a non-unique index uses (because
/// uniqueness can't be enforced on null), and the count tree at that path
/// aggregates them. So `Countable` / `CountableAllowingOffset` on a unique index
/// is meaningful exactly when at least one of the indexed properties is
/// optional in the document schema. Counts on all-non-null exact matches still
/// return the correct value (1 if present, 0 if not) because grovedb's
/// `Element::count_value_or_default()` returns 1 for non-`CountTree` elements
/// like `Reference`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Default)]
#[cfg_attr(feature = "serde-conversion", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-conversion", serde(rename_all = "camelCase"))]
pub enum IndexCountability {
    /// The index uses a plain `NormalTree` and does not support count fast paths.
    #[default]
    NotCountable,
    /// The index uses a `CountTree` — total counts are O(1) via the root count.
    Countable,
    /// The index uses a `ProvableCountTree` — same as `Countable` plus per-node
    /// counts that enable future O(log n) range / offset queries.
    CountableAllowingOffset,
}

impl IndexCountability {
    /// Returns true if this index supports count fast paths (either variant).
    pub fn is_countable(&self) -> bool {
        !matches!(self, Self::NotCountable)
    }

    /// Returns true if this index uses the provable variant (per-node counts,
    /// enabling future range / offset support).
    pub fn allows_offset(&self) -> bool {
        matches!(self, Self::CountableAllowingOffset)
    }
}

// Indices documentation:  https://dashplatform.readme.io/docs/reference-data-contracts#document-indices
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde-conversion", derive(Serialize, Deserialize))]
pub struct Index {
    pub name: String,
    pub properties: Vec<IndexProperty>,
    pub unique: bool,
    /// Null searchable indicates what to do if all members of the index are null
    /// If this is set to false then we do not insert references which makes such items non-searchable
    pub null_searchable: bool,
    /// Contested indexes are useful when a resource is considered valuable
    pub contested_index: Option<ContestedIndexInformation>,
    /// Whether and how the index supports count fast paths. See
    /// [`IndexCountability`].
    //
    // `serde(default)` on this and the three fields below: they were added
    // after the struct's serde shape was already in the wild (#3623 count
    // fields, #3661 sum fields), so JSON serialized before then must still
    // deserialize.
    #[cfg_attr(feature = "serde-conversion", serde(default))]
    pub countable: IndexCountability,
    /// Whether the index supports O(log n) count queries over a *range* of
    /// values for the index's last property (the terminator). The flag
    /// only affects the storage layout at the last property level — all
    /// preceding (prefix) properties keep their default tree shape:
    /// - The property-name tree at the *last* property (whose keys are
    ///   that property's distinct values) is stored as a
    ///   `ProvableCountTree`, so range queries over distinct values can
    ///   be answered by walking the boundary in O(log n).
    /// - Each value tree under it is stored as a `CountTree`, so the
    ///   property-name aggregate sums per-value counts cleanly.
    /// - Sibling continuations inside each value tree (compound-index
    ///   suffixes) are wrapped with `Element::NonCounted` so their counts
    ///   do not pollute the value tree's count.
    ///
    /// `range_countable: true` requires `countable` to be `Countable` or
    /// `CountableAllowingOffset` (it's additive, not a replacement).
    #[cfg_attr(feature = "serde-conversion", serde(default))]
    pub range_countable: bool,
    /// When set to `Some(property_name)`, this index's value-tree is laid out
    /// as a `SumTree` (or `CountSumTree` if [`Index::countable`] is also set
    /// and [`Index::range_summable`] is false) and every reference under the
    /// index path carries an `ItemWithSumItem` contribution equal to the
    /// document's named-property value at insert time. The named property
    /// must be `type: integer` and listed in the document type's `required`
    /// array (the validator enforces this at contract creation), and must
    /// match the doctype-level
    /// [`DocumentTypeV2::documents_summable`] when both are set.
    ///
    /// O(1) `sum(named_property) WHERE <index_properties_exactly_covered>`
    /// queries land on this index. See
    /// `book/src/drive/document-sum-trees.md` and
    /// `book/src/drive/sum-index-examples.md` for the worked example.
    ///
    /// **Note on `unique` indexes.** Same caveat as
    /// [`IndexCountability::Countable`] on a unique index: the storage
    /// effect is a no-op for documents whose indexed fields are *all*
    /// non-null (the terminal is a bare reference at key `[0]`), and it
    /// does meaningful sum-aggregation work only for null-bearing entries
    /// (which take the same sum-tree branch a non-unique index uses).
    #[cfg_attr(feature = "serde-conversion", serde(default))]
    pub summable: Option<String>,
    /// When `true`, this index supports O(log n) range-sum queries on its
    /// last property. The storage-layout effect mirrors
    /// [`Index::range_countable`] but on the sum surface:
    /// - The property-name level (the level *above* the last property's
    ///   value-tree level) is laid out as a `ProvableSumTree`, so range
    ///   queries over the last property's distinct values can be answered
    ///   by walking the boundary nodes' committed sub-sums in O(log n).
    /// - Each value tree under it is laid out as a `SumTree` (so the
    ///   property-name aggregate combines per-value sums cleanly).
    /// - Sibling continuations inside each value tree (compound-index
    ///   suffixes) are wrapped with `Element::NonCountedItemWithSumItem`
    ///   so their sums don't pollute the value tree's running sum.
    ///
    /// `range_summable: true` requires `summable` to be `Some` (it's
    /// additive on top of summable, not a replacement). Mutually
    /// compatible with `countable` and `range_countable` — combining
    /// the flags promotes the tree to a `ProvableCountSumTree` so a
    /// single tree carries both metrics. The dispatcher in
    /// `packages/rs-drive/src/drive/document/primary_key_tree_type.rs`
    /// picks the appropriate variant.
    #[cfg_attr(feature = "serde-conversion", serde(default))]
    pub range_summable: bool,
    /// When `true`, this index's **terminal property-name tree** (the level
    /// whose children are the last index property's value trees — i.e. one
    /// child per group) is upgraded from its `Provable*` form to the matching
    /// *indexed* tree, gaining an ordered secondary tree on the **Count** axis.
    /// That secondary tree is keyed by each group's document count, so
    /// "top / bottom K groups by count" is answerable in O(log n + k) with a
    /// proof instead of enumerating every group.
    ///
    /// The indexed primary is a byte-compatible mirror of the tree it replaces,
    /// so every existing range aggregate (`AggregateCountOnRange` &co.) keeps
    /// working against it unchanged.
    ///
    /// `ranked_countable: true` requires [`Index::range_countable`] (which in
    /// turn requires [`Index::countable`]): the ranking secondary is built from
    /// the per-group counts the range-count layout already maintains.
    ///
    /// See `book/src/drive/document-ranked-trees.md` and
    /// `book/src/drive/ranked-index-examples.md` for the worked example.
    //
    // `serde(default)`: added after the struct's serde shape was in the wild
    // (see the note on `countable` above), so pre-existing JSON must still
    // deserialize.
    #[cfg_attr(feature = "serde-conversion", serde(default))]
    pub ranked_countable: bool,
    /// Sum-axis counterpart of [`Index::ranked_countable`]: the terminal
    /// property-name tree gains an ordered secondary keyed by each group's sum
    /// of the [`Index::summable`] property, making "top / bottom K groups by
    /// sum" O(log n + k) with a proof.
    ///
    /// Requires [`Index::range_summable`] (which in turn requires
    /// [`Index::summable`]).
    ///
    /// See `book/src/drive/document-ranked-trees.md` and
    /// `book/src/drive/ranked-index-examples.md` for the worked example.
    #[cfg_attr(feature = "serde-conversion", serde(default))]
    pub ranked_summable: bool,
    /// Average-axis counterpart of [`Index::ranked_countable`]: the terminal
    /// property-name tree gains an ordered secondary keyed by each group's
    /// average — stored as the (count, sum) pair the client divides, same
    /// no-server-division rule the average query surface already follows.
    ///
    /// Requires `rangeAverageable` *semantics*: both [`Index::range_countable`]
    /// and [`Index::range_summable`], however they were declared (via the
    /// `rangeAverageable` / `averageable` sugar or the explicit
    /// `countable` + `summable` + `rangeCountable` + `rangeSummable` longhand).
    ///
    /// The three ranking axes are **independent**: `ranked_averageable` does
    /// NOT imply `ranked_countable` or `ranked_summable`. Each axis costs its
    /// own ordered secondary tree, so each is opted into explicitly.
    ///
    /// See `book/src/drive/document-ranked-trees.md` and
    /// `book/src/drive/ranked-index-examples.md` for the worked example.
    #[cfg_attr(feature = "serde-conversion", serde(default))]
    pub ranked_averageable: bool,
}

impl Index {
    /// Check to see if two objects are conflicting
    pub fn objects_are_conflicting(&self, object1: &ValueMap, object2: &ValueMap) -> bool {
        if !self.unique {
            return false;
        }
        self.properties.iter().all(|property| {
            //if either or both are null then there can not be an overlap
            let Some(value1) = Value::get_optional_from_map(object1, property.name.as_str()) else {
                return false;
            };
            let Some(value2) = Value::get_optional_from_map(object2, property.name.as_str()) else {
                return false;
            };
            value1 == value2
        })
    }
    /// The field names of the index
    pub fn property_names(&self) -> Vec<String> {
        self.properties
            .iter()
            .map(|property| property.name.clone())
            .collect()
    }

    /// Get values
    pub fn extract_values(&self, data: &BTreeMap<String, Value>) -> Vec<Value> {
        self.properties
            .iter()
            .map(|property| data.get(&property.name).cloned().unwrap_or(Value::Null))
            .collect()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
#[cfg_attr(feature = "serde-conversion", derive(Serialize, Deserialize))]
pub struct IndexProperty {
    pub name: String,
    pub ascending: bool,
}

impl TryFrom<BTreeMap<String, String>> for IndexProperty {
    type Error = ProtocolError;

    fn try_from(value: BTreeMap<String, String>) -> Result<Self, Self::Error> {
        if value.is_empty() {
            return Err(ProtocolError::Error(anyhow!(
                "property in the index definition cannot be empty"
            )));
        }
        if value.len() > 1 {
            return Err(ProtocolError::Error(anyhow!(
                "property in the index cannot contain more than one item: {:#?}",
                value
            )));
        }

        // the unwrap is safe because of the checks above
        let raw_property = value.into_iter().next().unwrap();
        let ascending = match raw_property.1.as_str() {
            "asc" => true,
            "desc" => false,
            sort_order => {
                return Err(ProtocolError::Error(anyhow!(
                    "invalid sorting order: '{}'",
                    sort_order
                )))
            }
        };

        Ok(Self {
            name: raw_property.0,
            ascending,
        })
    }
}

impl Index {
    // The matches function will take a slice of an array of strings and an optional sort on value.
    // An index matches if all the index_names in the slice are consecutively the index's properties
    // with leftovers permitted.
    // If a sort_on value is provided it must match the last index property.
    // The number returned is the number of unused index properties

    // A case for example if we have an index on person's name and age
    // where we say name == 'Sam' sort by age
    // there is no field operator on age
    // The return value for name == 'Sam' sort by age would be 0
    // The return value for name == 'Sam and age > 5 sort by age would be 0
    // the return value for sort by age would be 1
    pub fn matches(
        &self,
        index_names: &[&str],
        in_field_name: Option<&str>,
        order_by: &[&str],
    ) -> Option<u16> {
        // Here we are trying to figure out if the Index matches the order by
        // To do so we take the index and go backwards as we need the order by clauses to be
        // continuous, but they do not need to be at the end.
        let mut reduced_properties = self.properties.as_slice();
        // let mut should_ignore: Vec<String> = order_by.iter().map(|&str| str.to_string()).collect();
        if !order_by.is_empty() {
            for _ in 0..self.properties.len() {
                if reduced_properties.len() < order_by.len() {
                    return None;
                }
                let matched_ordering = reduced_properties
                    .iter()
                    .rev()
                    .zip(order_by.iter().rev())
                    .all(|(property, &sort)| property.name.as_str() == sort);
                if matched_ordering {
                    break;
                }
                if let Some((_last, elements)) = reduced_properties.split_last() {
                    // should_ignore.push(last.name.clone());
                    reduced_properties = elements;
                } else {
                    return None;
                }
            }
        }

        let last_property = self.properties.last()?;

        // the in field can only be on the last or before last property
        if let Some(in_field_name) = in_field_name {
            if last_property.name.as_str() != in_field_name {
                // it can also be on the before last
                if self.properties.len() == 1 {
                    return None;
                }
                let before_last_property = self.properties.get(self.properties.len() - 2)?;
                if before_last_property.name.as_str() != in_field_name {
                    return None;
                }
            }
        }

        let mut d = self.properties.len();

        for search_name in index_names.iter() {
            if !reduced_properties
                .iter()
                .any(|property| property.name.as_str() == *search_name)
            {
                return None;
            }
            d -= 1;
        }

        Some(d as u16)
    }
}

impl TryFrom<&[(Value, Value)]> for Index {
    type Error = DataContractError;

    /// Parses an index definition **without** the ranked-aggregate grammar.
    ///
    /// This is the pre-meta-schema-v3 (pre protocol version 14) surface: the
    /// three `ranked*` keywords are unknown property names here and are
    /// rejected as such. Call sites that know the contract's
    /// `document_type_schema` version must go through
    /// [`Index::try_from_value_map`] instead so PV14+ contracts can use them.
    fn try_from(index_type_value_map: &[(Value, Value)]) -> Result<Self, Self::Error> {
        Index::try_from_value_map(index_type_value_map, false)
    }
}

impl Index {
    /// Parses an index definition from its `(key, value)` map form.
    ///
    /// `ranked_aggregates_allowed` mirrors `document_type_schema >= 3` (i.e.
    /// document meta-schema v3, protocol version 14 and later). When it is
    /// `false` the three ranked keywords (`rankedCountable`, `rankedSummable`,
    /// `rankedAverageable`) are not part of the grammar at all: they fall
    /// through to the unknown-key arm and are rejected with exactly the error a
    /// pre-v14 node produced for them, so a non-validating parse (check_tx,
    /// cache warm-up, restore) cannot smuggle a ranked index past a node whose
    /// protocol version does not know how to lay one out on disk.
    ///
    /// The meta-schema is the other half of this gate — v2 rejects the keys via
    /// `additionalProperties: false` — but it only runs under
    /// `full_validation`, which is why the grammar itself is version-gated too.
    pub fn try_from_value_map(
        index_type_value_map: &[(Value, Value)],
        ranked_aggregates_allowed: bool,
    ) -> Result<Self, DataContractError> {
        // Decouple the map
        // It contains properties and a unique key
        // If the unique key is absent, then unique is false
        // If present, then use that value
        // For properties, we iterate each and move it to IndexProperty

        let mut unique = false;
        // The default for null searchable should be true. Do not change this without very
        // careful thought and consideration.
        let mut null_searchable = true;
        let mut name = None;
        let mut contested_index = None;
        let mut index_properties: Vec<IndexProperty> = Vec::new();
        let mut countable = IndexCountability::NotCountable;
        // Tracks whether `countable` was explicitly present in the
        // input map (regardless of value). After the loop, the default
        // `NotCountable` is indistinguishable from an explicit
        // `countable: "notCountable"` on the parsed enum — we need
        // this bit to know whether `averageable` may silently promote
        // (omitted countable: yes) or must reject (explicit
        // `notCountable`: contradiction with averageable's implied
        // countability).
        let mut countable_was_explicit = false;
        let mut range_countable = false;
        // Same explicit-vs-default tracking for `rangeCountable` and
        // `rangeSummable`. After the loop the default `false` is
        // indistinguishable from an explicit `rangeCountable: false`
        // on the parsed bool — but the two have different conflict
        // semantics under `rangeAverageable: true`: omitted is
        // silently promotable; explicit `false` is a contradiction
        // we surface to the author.
        let mut range_countable_was_explicit = false;
        let mut summable: Option<String> = None;
        let mut range_summable = false;
        let mut range_summable_was_explicit = false;
        // `averageable` / `rangeAverageable` are syntactic sugar for the
        // count+sum combination — same on-disk layout and same query
        // surface, just a friendlier name for authors who think in terms
        // of averages rather than (count, sum) pairs. Parsed into the
        // existing flags below after the value-key loop; intermediate
        // bindings here let us detect conflicts (e.g. `averageable: "x"`
        // alongside `summable: "y"`) before the merge.
        let mut averageable: Option<String> = None;
        let mut range_averageable = false;
        // Ranking axes (meta-schema v3 / PV14+). Each one is an independent
        // opt-in that adds one ordered secondary tree to the terminal
        // property-name tree; unlike `averageable` / `rangeAverageable` there
        // is no sugar relationship between them, so no explicit-vs-default
        // tracking is needed — nothing ever promotes them.
        let mut ranked_countable = false;
        let mut ranked_summable = false;
        let mut ranked_averageable = false;

        for (key_value, value_value) in index_type_value_map {
            let key = key_value.to_str()?;

            match key {
                "name" => {
                    name = Some(
                        value_value
                            .as_text()
                            .ok_or(DataContractError::InvalidContractStructure(
                                "index name should be a string".to_string(),
                            ))?
                            .to_owned(),
                    );
                }
                "unique" => {
                    if value_value.is_bool() {
                        unique = value_value.as_bool().expect("confirmed as bool");
                    }
                }
                "nullSearchable" => {
                    if value_value.is_bool() {
                        null_searchable = value_value.as_bool().expect("confirmed as bool");
                    }
                }
                "contested" => {
                    let contested_properties_value_map = value_value.to_map()?;

                    let mut contested_index_information = ContestedIndexInformation::default();

                    for (contested_key_value, contested_value) in contested_properties_value_map {
                        let contested_key = contested_key_value
                            .to_str()
                            .map_err(|e| DataContractError::ValueDecodingError(e.to_string()))?;
                        match contested_key {
                            "fieldMatches" => {
                                let field_matches_array = contested_value.to_array_ref()?;
                                for field_match in field_matches_array {
                                    let field_match_map = field_match.to_map()?;
                                    let mut name = None;
                                    let mut field_matches = None;
                                    for (field_match_key_as_value, field_match_value) in
                                        field_match_map
                                    {
                                        let field_match_key =
                                            field_match_key_as_value.to_str().map_err(|e| {
                                                DataContractError::ValueDecodingError(e.to_string())
                                            })?;
                                        match field_match_key {
                                            "field" => {
                                                let field = field_match_value.to_str()?.to_owned();
                                                name = Some(field);
                                            }
                                            "regexPattern" => {
                                                let regex_str =
                                                    field_match_value.to_str()?.to_owned();

                                                #[cfg(feature = "validation")]
                                                Regex::new(&regex_str).map_err(|e| {
                                                    RegexError(format!(
                                                        "invalid field match regex: {}",
                                                        e
                                                    ))
                                                })?;

                                                field_matches =
                                                    Some(ContestedIndexFieldMatch::Regex(
                                                        LazyRegex::new(regex_str),
                                                    ));
                                            }
                                            key => {
                                                return Err(DataContractError::ValueWrongType(
                                                    format!("unexpected field match key {}", key),
                                                ));
                                            }
                                        }
                                    }
                                    if name.is_none() {
                                        return Err(DataContractError::FieldRequirementUnmet(
                                            format!(
                                                "field not present in contested fieldMatches {}",
                                                key
                                            ),
                                        ));
                                    }
                                    if field_matches.is_none() {
                                        return Err(DataContractError::FieldRequirementUnmet(
                                            format!(
                                                "field not present in contested fieldMatches {}",
                                                key
                                            ),
                                        ));
                                    }
                                    contested_index_information
                                        .field_matches
                                        .insert(name.unwrap(), field_matches.unwrap());
                                }
                            }
                            "resolution" => {
                                let resolution_int = contested_value.to_integer::<u8>()?;
                                contested_index_information.resolution =
                                    resolution_int.try_into().map_err(|e: ProtocolError| {
                                        DataContractError::ValueWrongType(e.to_string())
                                    })?;
                            }
                            "description" => {}
                            key => {
                                return Err(DataContractError::ValueWrongType(format!(
                                    "unexpected contested key {}",
                                    key
                                )));
                            }
                        }
                    }
                    contested_index = Some(contested_index_information);
                }
                "countable" => {
                    // Accept either:
                    //   - boolean: `true` → Countable, `false` → NotCountable.
                    //     This preserves v0 contracts (whose meta-schema enforces
                    //     `"type": "boolean"`) and any v1 contracts written before
                    //     the enum form was introduced.
                    //   - string: one of `"notCountable"`, `"countable"`,
                    //     `"countableAllowingOffset"` (camelCase, matching the
                    //     `IndexCountability` serde rename rule).
                    countable_was_explicit = true;
                    countable = match value_value {
                        Value::Bool(true) => IndexCountability::Countable,
                        Value::Bool(false) => IndexCountability::NotCountable,
                        Value::Text(s) => match s.as_str() {
                            "notCountable" => IndexCountability::NotCountable,
                            "countable" => IndexCountability::Countable,
                            "countableAllowingOffset" => IndexCountability::CountableAllowingOffset,
                            other => {
                                return Err(DataContractError::ValueWrongType(format!(
                                    "countable value must be a boolean or one of \
                                     \"notCountable\" / \"countable\" / \
                                     \"countableAllowingOffset\"; got {:?}",
                                    other
                                )))
                            }
                        },
                        _ => {
                            return Err(DataContractError::ValueWrongType(
                                "countable value must be a boolean or a string".to_string(),
                            ))
                        }
                    };
                }
                "rangeCountable" => {
                    range_countable_was_explicit = true;
                    range_countable =
                        value_value
                            .as_bool()
                            .ok_or(DataContractError::ValueWrongType(
                                "rangeCountable value must be a boolean".to_string(),
                            ))?;
                }
                "summable" => {
                    // `summable` names the integer property whose value-per-
                    // document contributes to the index's running sum. Two
                    // accepted shapes:
                    //   - `null` → not summable (same as omitting the key).
                    //   - string → property name (must exist on the doctype,
                    //     be `type: integer`, and appear in `required`;
                    //     enforced by higher-level doctype validation).
                    summable = match value_value {
                        Value::Null => None,
                        Value::Text(s) if !s.is_empty() => Some(s.clone()),
                        Value::Text(_) => {
                            return Err(DataContractError::ValueWrongType(
                                "summable value must be a non-empty string naming an integer \
                                 property, or null"
                                    .to_string(),
                            ))
                        }
                        _ => {
                            return Err(DataContractError::ValueWrongType(
                                "summable value must be a string naming an integer property, \
                                 or null"
                                    .to_string(),
                            ))
                        }
                    };
                }
                "rangeSummable" => {
                    range_summable_was_explicit = true;
                    range_summable =
                        value_value
                            .as_bool()
                            .ok_or(DataContractError::ValueWrongType(
                                "rangeSummable value must be a boolean".to_string(),
                            ))?;
                }
                "averageable" => {
                    // `averageable: "<prop>"` is shorthand for
                    // `countable: "countable"` + `summable: "<prop>"`.
                    // Same parsing rules as `summable`: null = not
                    // averageable, non-empty string = property name.
                    averageable =
                        match value_value {
                            Value::Null => None,
                            Value::Text(s) if !s.is_empty() => Some(s.clone()),
                            Value::Text(_) => return Err(DataContractError::ValueWrongType(
                                "averageable value must be a non-empty string naming an integer \
                                 property, or null"
                                    .to_string(),
                            )),
                            _ => return Err(DataContractError::ValueWrongType(
                                "averageable value must be a string naming an integer property, \
                                 or null"
                                    .to_string(),
                            )),
                        };
                }
                "rangeAverageable" => {
                    // `rangeAverageable: true` is shorthand for
                    // `rangeCountable: true` + `rangeSummable: true`.
                    range_averageable =
                        value_value
                            .as_bool()
                            .ok_or(DataContractError::ValueWrongType(
                                "rangeAverageable value must be a boolean".to_string(),
                            ))?;
                }
                // The three ranking keywords are guarded on
                // `ranked_aggregates_allowed`: when the contract's
                // `document_type_schema` version predates v3 the guard fails,
                // the arm doesn't match, and the key falls through to the
                // unknown-property arm below — byte-identical to how a node
                // without this feature rejects it.
                RANKED_COUNTABLE if ranked_aggregates_allowed => {
                    ranked_countable =
                        value_value
                            .as_bool()
                            .ok_or(DataContractError::ValueWrongType(
                                "rankedCountable value must be a boolean".to_string(),
                            ))?;
                }
                RANKED_SUMMABLE if ranked_aggregates_allowed => {
                    ranked_summable =
                        value_value
                            .as_bool()
                            .ok_or(DataContractError::ValueWrongType(
                                "rankedSummable value must be a boolean".to_string(),
                            ))?;
                }
                RANKED_AVERAGEABLE if ranked_aggregates_allowed => {
                    ranked_averageable =
                        value_value
                            .as_bool()
                            .ok_or(DataContractError::ValueWrongType(
                                "rankedAverageable value must be a boolean".to_string(),
                            ))?;
                }
                "properties" => {
                    let properties =
                        value_value
                            .as_array()
                            .ok_or(DataContractError::ValueWrongType(
                                "properties value should be an array".to_string(),
                            ))?;

                    // Iterate over this and get the index properties
                    for property in properties {
                        let property_map =
                            property.as_map().ok_or(DataContractError::ValueWrongType(
                                "each property of an index should be a map".to_string(),
                            ))?;

                        let index_property = IndexProperty::from_platform_value(property_map)?;
                        index_properties.push(index_property);
                    }
                }
                _ => {
                    return Err(DataContractError::ValueWrongType(
                        "unexpected property name".to_string(),
                    ))
                }
            }
        }

        if contested_index.is_some() && !unique {
            return Err(DataContractError::InvalidContractStructure(
                "contest supported only for unique indexes".to_string(),
            ));
        }

        // Desugar `averageable` / `rangeAverageable` into the
        // count + sum flags they're shorthand for. Conflict rules:
        // - `averageable` + `summable` must name the same property (or
        //   `summable` must be absent). They're describing the same
        //   on-disk layout from two different angles; differing names
        //   are an authoring mistake.
        // - `averageable` + `countable: notCountable` is a conflict —
        //   `averageable` implies countable but the author explicitly
        //   said no. Setting `countable` to `countable` or
        //   `countableAllowingOffset` alongside `averageable` is fine
        //   because they agree.
        // - `rangeAverageable: true` requires `averageable` to be set
        //   (mirrors `rangeSummable` requires `summable`). Caught via
        //   the existing range_summable check after the merge below.
        if let Some(avg_prop) = &averageable {
            if let Some(sum_prop) = &summable {
                if sum_prop != avg_prop {
                    return Err(DataContractError::InvalidContractStructure(format!(
                        "averageable=\"{}\" conflicts with summable=\"{}\": both flags name \
                         the property whose values are aggregated into the index's sum tree, \
                         so they must agree (or only one should be set — averageable is \
                         shorthand for countable + summable on the same property)",
                        avg_prop, sum_prop,
                    )));
                }
            }
            // `averageable` implies countable. Three cases:
            //  1. `countable` not present in input → silently promote to
            //     `Countable` (this is the canonical shorthand: write
            //     just `averageable: "x"` to get countable + summable).
            //  2. `countable` explicitly present and already countable
            //     (`"countable"` / `"countableAllowingOffset"`) → no-op,
            //     the author agreed.
            //  3. `countable` explicitly present as `"notCountable"` (or
            //     boolean `false`) → reject. The author actively said
            //     "not countable" while also saying "averageable" — a
            //     direct contradiction we surface rather than silently
            //     override.
            if !countable_was_explicit {
                countable = IndexCountability::Countable;
            } else if !countable.is_countable() {
                return Err(DataContractError::InvalidContractStructure(format!(
                    "averageable=\"{}\" implies the index must be countable, but `countable` \
                     is explicitly set to a non-countable value. Remove the explicit \
                     `countable: \"notCountable\"` (or set it to `\"countable\"` / \
                     `\"countableAllowingOffset\"`); averageable is shorthand for \
                     countable + summable on the named property.",
                    avg_prop,
                )));
            }
            // Promote `summable` to the same property.
            summable = Some(avg_prop.clone());
        } else if range_averageable {
            return Err(DataContractError::InvalidContractStructure(
                "rangeAverageable: true requires averageable: \"<prop>\" to name the integer \
                 property to average; rangeAverageable on its own has no property to aggregate"
                    .to_string(),
            ));
        }
        if range_averageable {
            // `rangeAverageable: true` ⇒ both range axes opt in.
            // Reject explicit-`false` contradictions on either range
            // axis — silently flipping the author's explicit value
            // would emit on-disk layout the author didn't ask for.
            // Omitted (default-false) flags are promoted silently;
            // explicit `true` is a redundant no-op.
            if range_countable_was_explicit && !range_countable {
                return Err(DataContractError::InvalidContractStructure(
                    "rangeAverageable: true conflicts with explicit rangeCountable: false: \
                     rangeAverageable is shorthand for rangeCountable + rangeSummable on \
                     the averageable property. Remove the explicit `rangeCountable: false` \
                     (or drop rangeAverageable in favor of rangeSummable alone)."
                        .to_string(),
                ));
            }
            if range_summable_was_explicit && !range_summable {
                return Err(DataContractError::InvalidContractStructure(
                    "rangeAverageable: true conflicts with explicit rangeSummable: false: \
                     rangeAverageable is shorthand for rangeCountable + rangeSummable on \
                     the averageable property. Remove the explicit `rangeSummable: false` \
                     (or drop rangeAverageable in favor of rangeCountable alone)."
                        .to_string(),
                ));
            }
            range_countable = true;
            range_summable = true;
        }

        // `rangeCountable` is additive on top of `countable`: it changes how
        // the index's tree is laid out (property-name → ProvableCountTree,
        // value level → CountTree, sibling continuations → NonCounted) so
        // that range-count queries can be answered in O(log n). It is
        // meaningless without the underlying countability.
        if range_countable && !countable.is_countable() {
            return Err(DataContractError::InvalidContractStructure(
                "rangeCountable requires countable to be \"countable\" or \
                 \"countableAllowingOffset\"; range-count queries only make \
                 sense on a count-bearing index"
                    .to_string(),
            ));
        }

        // `rangeSummable` is additive on top of `summable`: it changes how
        // the index's tree is laid out (property-name → ProvableSumTree,
        // value level → SumTree, sibling continuations →
        // NonCountedItemWithSumItem) so that range-sum queries can be
        // answered in O(log n). It's meaningless without the underlying
        // summability.
        if range_summable && summable.is_none() {
            return Err(DataContractError::InvalidContractStructure(
                "rangeSummable requires summable to be set to a property name; \
                 range-sum queries only make sense on a sum-bearing index"
                    .to_string(),
            ));
        }

        // The ranking axes are checked after the `averageable` /
        // `rangeAverageable` desugar above, so they see the *resolved* range
        // flags: both the sugar form (`averageable` + `rangeAverageable`) and
        // the explicit longhand (`countable` + `summable` + `rangeCountable` +
        // `rangeSummable`) satisfy them identically.
        //
        // Each ranked flag adds one ordered secondary tree keyed by the
        // aggregate the corresponding range axis already maintains per group,
        // so the range axis is a hard prerequisite: without it the terminal
        // property-name tree carries no per-group aggregate to rank by.
        if ranked_countable && !range_countable {
            return Err(DataContractError::InvalidContractStructure(
                "rankedCountable requires rangeCountable: true; ranking groups by \
                 count needs the per-group counts the range-count layout \
                 maintains"
                    .to_string(),
            ));
        }

        if ranked_summable && !range_summable {
            return Err(DataContractError::InvalidContractStructure(
                "rankedSummable requires rangeSummable: true; ranking groups by \
                 sum needs the per-group sums the range-sum layout maintains"
                    .to_string(),
            ));
        }

        // `rangeAverageable` semantics = both range axes (that is exactly what
        // the sugar desugars into), which transitively pulls in `countable`
        // and `summable` through the two checks above.
        if ranked_averageable && !(range_countable && range_summable) {
            return Err(DataContractError::InvalidContractStructure(
                "rankedAverageable requires rangeAverageable semantics — both \
                 rangeCountable and rangeSummable must be in effect (declare \
                 `averageable` + `rangeAverageable`, or the explicit `countable` \
                 + `summable` + `rangeCountable` + `rangeSummable` longhand); \
                 ranking groups by average needs both the per-group counts and \
                 the per-group sums"
                    .to_string(),
            ));
        }

        // Ranking orders groups (the distinct values of the index's last
        // property) by a per-group aggregate. On a unique index every group
        // holds at most one document, so every ranked ordering degenerates to
        // a constant-per-group ordering that a plain range query already
        // serves — while still paying for an indexed tree and its secondary
        // maintenance on every write. Contested indexes are unique by
        // construction (checked above).
        if (ranked_countable || ranked_summable || ranked_averageable) && unique {
            return Err(DataContractError::InvalidContractStructure(
                "ranked aggregates are not supported on unique indexes: each \
                 group of a unique index contains at most one document, so \
                 there is nothing meaningful to rank"
                    .to_string(),
            ));
        }

        // Ranked aggregates are restricted to single-property indexes in
        // this protocol version. Two reasons: a compound index whose prefix
        // level also terminates an aggregating index would need its ranked
        // terminal tree wrapped in a NonCounted/NotSummed shell, which the
        // storage layer structurally rejects for indexed trees (the wrapper
        // would neutralize the very aggregates the ranking indexes); and the
        // ranked query surface deliberately has no equality-prefix routing
        // yet. Both are relaxable at a future protocol version.
        if (ranked_countable || ranked_summable || ranked_averageable) && index_properties.len() > 1
        {
            return Err(DataContractError::InvalidContractStructure(
                "ranked aggregates are only supported on single-property \
                 indexes in this protocol version"
                    .to_string(),
            ));
        }

        // `nullSearchable: false` suppresses the terminal reference for a
        // document that leaves the indexed property out — but the document
        // index walker has already created that document's value tree by the
        // time the terminal handler declines. Under a ranked index that value
        // tree is an entry of a grovedb indexed primary, so the secondary
        // mirrors it as a group whose aggregates are all zero: an
        // authenticated TOP/BOTTOM answer would contain a group the index is
        // supposed to exclude, with no document behind it. `nullSearchable`
        // is only ever `false` when the contract says so explicitly — the
        // default is `true` — and under `true` the null documents get their
        // real reference and form a legitimate rankable group, which is the
        // combination authors actually want.
        if (ranked_countable || ranked_summable || ranked_averageable) && !null_searchable {
            return Err(DataContractError::InvalidContractStructure(
                "ranked aggregates are not supported with nullSearchable: false: a \
                 document missing the indexed property still creates the null group's \
                 value tree, but its reference is suppressed, so the ranking would \
                 expose a phantom group with zero aggregates and no documents behind \
                 it. Leave nullSearchable at its default (true), where documents with \
                 a null value form a real, rankable group"
                    .to_string(),
            ));
        }

        // If the index didn't have a name, derive one deterministically from
        // its properties and their directions. Every document meta-schema
        // (v0/v1/v2) requires `name`, so an unnamed index can only reach this
        // point when schema validation is skipped (check_tx, legacy fixtures,
        // client-side parses of contracts that could never register); a random
        // name here would make two parses of the same contract disagree on the
        // index name and on the iteration order of the name-keyed indices map.
        // Properties are joined with `|`, which cannot appear in a validated
        // property path (path segments match `^[a-zA-Z0-9-_]{1,64}$`, joined
        // by `.`, plus `$`-prefixed system properties), so two distinct index
        // declarations can never derive the same name; only true duplicate
        // declarations collide and collapse to one entry in the name-keyed
        // map, which is the right outcome for a duplicate index.
        let name = name.unwrap_or_else(|| {
            if index_properties.is_empty() {
                "index".to_string()
            } else {
                index_properties
                    .iter()
                    .map(|property| {
                        format!(
                            "{}_{}",
                            property.name,
                            if property.ascending { "asc" } else { "desc" }
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("|")
            }
        });

        Ok(Index {
            name,
            properties: index_properties,
            unique,
            null_searchable,
            contested_index,
            countable,
            range_countable,
            summable,
            range_summable,
            ranked_countable,
            ranked_summable,
            ranked_averageable,
        })
    }
}

impl IndexProperty {
    pub fn from_platform_value(
        index_property_map: &[(Value, Value)],
    ) -> Result<Self, DataContractError> {
        // The document meta-schema enforces `minProperties: 1` /
        // `maxProperties: 1` on each index property object, but that
        // validation is skipped in check_tx (full_validation=false), so a
        // crafted contract can reach this point with an empty or oversized
        // map. Guard explicitly to avoid panicking on an out-of-bounds index.
        if index_property_map.len() != 1 {
            return Err(DataContractError::InvalidContractStructure(
                "index property entry must contain exactly one key/value".to_string(),
            ));
        }
        let property = &index_property_map[0];

        let key = property
            .0 // key
            .as_text()
            .ok_or(DataContractError::KeyWrongType(
                "key should be of type string".to_string(),
            ))?;
        let value = property
            .1 // value
            .as_text()
            .ok_or(DataContractError::ValueWrongType(
                "value should be of type string".to_string(),
            ))?;

        let ascending = value == "asc";

        Ok(IndexProperty {
            name: key.to_string(),
            ascending,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_index_property(name: &str, ascending: bool) -> IndexProperty {
        IndexProperty {
            name: name.to_string(),
            ascending,
        }
    }

    fn make_index(name: &str, properties: Vec<(&str, bool)>, unique: bool) -> Index {
        Index {
            name: name.to_string(),
            properties: properties
                .into_iter()
                .map(|(n, asc)| make_index_property(n, asc))
                .collect(),
            unique,
            null_searchable: true,
            contested_index: None,
            countable: IndexCountability::NotCountable,
            range_countable: false,
            summable: None,
            range_summable: false,
            ranked_countable: false,
            ranked_summable: false,
            ranked_averageable: false,
        }
    }

    // -----------------------------------------------------------------------
    // ContestedIndexResolution tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_contested_index_resolution_try_from_valid() {
        let res = ContestedIndexResolution::try_from(0u8).unwrap();
        assert_eq!(res, ContestedIndexResolution::MasternodeVote);
    }

    #[test]
    fn test_contested_index_resolution_try_from_invalid() {
        let res = ContestedIndexResolution::try_from(1u8);
        assert!(res.is_err());
    }

    #[test]
    fn test_contested_index_resolution_try_from_255() {
        let res = ContestedIndexResolution::try_from(255u8);
        assert!(res.is_err());
    }

    // -----------------------------------------------------------------------
    // LazyRegex tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_lazy_regex_match() {
        let lr = LazyRegex::new("^[a-z]+$".to_string());
        assert!(lr.is_match("hello"));
        assert!(!lr.is_match("Hello"));
        assert!(!lr.is_match("123"));
    }

    #[test]
    fn test_lazy_regex_as_str() {
        let lr = LazyRegex::new("test_pattern".to_string());
        assert_eq!(lr.as_str(), "test_pattern");
    }

    // -----------------------------------------------------------------------
    // ContestedIndexFieldMatch tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_contested_index_field_match_regex_matches() {
        let m = ContestedIndexFieldMatch::Regex(LazyRegex::new("^dash".to_string()));
        assert!(m.matches(&Value::Text("dashname".to_string())));
        assert!(!m.matches(&Value::Text("notdash".to_string())));
    }

    #[test]
    fn test_contested_index_field_match_regex_non_string() {
        let m = ContestedIndexFieldMatch::Regex(LazyRegex::new(".*".to_string()));
        assert!(!m.matches(&Value::U64(42)));
    }

    #[test]
    fn test_contested_index_field_match_positive_integer_matches() {
        let m = ContestedIndexFieldMatch::PositiveIntegerMatch(42);
        assert!(m.matches(&Value::U64(42)));
        assert!(!m.matches(&Value::U64(43)));
    }

    #[test]
    fn test_contested_index_field_match_positive_integer_non_integer() {
        let m = ContestedIndexFieldMatch::PositiveIntegerMatch(42);
        assert!(!m.matches(&Value::Text("42".to_string())));
    }

    // -----------------------------------------------------------------------
    // ContestedIndexFieldMatch ordering tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_contested_index_field_match_ord_integers() {
        let a = ContestedIndexFieldMatch::PositiveIntegerMatch(10);
        let b = ContestedIndexFieldMatch::PositiveIntegerMatch(20);
        assert!(a < b);
    }

    #[test]
    fn test_contested_index_field_match_ord_regex_vs_integer() {
        let regex = ContestedIndexFieldMatch::Regex(LazyRegex::new("abc".to_string()));
        let integer = ContestedIndexFieldMatch::PositiveIntegerMatch(10);
        assert!(regex < integer);
        assert!(integer > regex);
    }

    #[test]
    fn test_contested_index_field_match_ord_regex_vs_regex() {
        let short = ContestedIndexFieldMatch::Regex(LazyRegex::new("a".to_string()));
        let long = ContestedIndexFieldMatch::Regex(LazyRegex::new("abc".to_string()));
        assert!(short < long);
    }

    // -----------------------------------------------------------------------
    // ContestedIndexFieldMatch equality tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_contested_index_field_match_eq_regex() {
        let a = ContestedIndexFieldMatch::Regex(LazyRegex::new("^test$".to_string()));
        let b = ContestedIndexFieldMatch::Regex(LazyRegex::new("^test$".to_string()));
        assert_eq!(a, b);
    }

    #[test]
    fn test_contested_index_field_match_eq_different_regex() {
        let a = ContestedIndexFieldMatch::Regex(LazyRegex::new("^a$".to_string()));
        let b = ContestedIndexFieldMatch::Regex(LazyRegex::new("^b$".to_string()));
        assert_ne!(a, b);
    }

    #[test]
    fn test_contested_index_field_match_eq_integer() {
        let a = ContestedIndexFieldMatch::PositiveIntegerMatch(42);
        let b = ContestedIndexFieldMatch::PositiveIntegerMatch(42);
        assert_eq!(a, b);
    }

    #[test]
    fn test_contested_index_field_match_eq_different_types() {
        let regex = ContestedIndexFieldMatch::Regex(LazyRegex::new("42".to_string()));
        let integer = ContestedIndexFieldMatch::PositiveIntegerMatch(42);
        assert_ne!(regex, integer);
    }

    // -----------------------------------------------------------------------
    // ContestedIndexFieldMatch clone tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_contested_index_field_match_clone_regex() {
        let original = ContestedIndexFieldMatch::Regex(LazyRegex::new("^test$".to_string()));
        let cloned = original.clone();
        assert_eq!(original, cloned);
    }

    #[test]
    fn test_contested_index_field_match_clone_integer() {
        let original = ContestedIndexFieldMatch::PositiveIntegerMatch(100);
        let cloned = original.clone();
        assert_eq!(original, cloned);
    }

    // -----------------------------------------------------------------------
    // ContestedIndexInformation default tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_contested_index_information_default() {
        let info = ContestedIndexInformation::default();
        assert!(info.field_matches.is_empty());
        assert_eq!(info.resolution, ContestedIndexResolution::MasternodeVote);
    }

    // -----------------------------------------------------------------------
    // Index::objects_are_conflicting tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_objects_are_conflicting_non_unique_always_false() {
        let index = make_index("idx", vec![("name", true)], false);
        let obj1: ValueMap = vec![(
            Value::Text("name".to_string()),
            Value::Text("Sam".to_string()),
        )];
        let obj2: ValueMap = vec![(
            Value::Text("name".to_string()),
            Value::Text("Sam".to_string()),
        )];
        assert!(!index.objects_are_conflicting(&obj1, &obj2));
    }

    #[test]
    fn test_objects_are_conflicting_unique_same_values() {
        let index = make_index("idx", vec![("name", true)], true);
        let obj1: ValueMap = vec![(
            Value::Text("name".to_string()),
            Value::Text("Sam".to_string()),
        )];
        let obj2: ValueMap = vec![(
            Value::Text("name".to_string()),
            Value::Text("Sam".to_string()),
        )];
        assert!(index.objects_are_conflicting(&obj1, &obj2));
    }

    #[test]
    fn test_objects_are_conflicting_unique_different_values() {
        let index = make_index("idx", vec![("name", true)], true);
        let obj1: ValueMap = vec![(
            Value::Text("name".to_string()),
            Value::Text("Sam".to_string()),
        )];
        let obj2: ValueMap = vec![(
            Value::Text("name".to_string()),
            Value::Text("Alice".to_string()),
        )];
        assert!(!index.objects_are_conflicting(&obj1, &obj2));
    }

    #[test]
    fn test_objects_are_conflicting_one_missing_property() {
        let index = make_index("idx", vec![("name", true)], true);
        let obj1: ValueMap = vec![(
            Value::Text("name".to_string()),
            Value::Text("Sam".to_string()),
        )];
        let obj2: ValueMap = vec![];
        assert!(!index.objects_are_conflicting(&obj1, &obj2));
    }

    #[test]
    fn test_objects_are_conflicting_multi_property() {
        let index = make_index("idx", vec![("name", true), ("age", true)], true);
        let obj1: ValueMap = vec![
            (
                Value::Text("name".to_string()),
                Value::Text("Sam".to_string()),
            ),
            (Value::Text("age".to_string()), Value::U64(30)),
        ];
        let obj2: ValueMap = vec![
            (
                Value::Text("name".to_string()),
                Value::Text("Sam".to_string()),
            ),
            (Value::Text("age".to_string()), Value::U64(30)),
        ];
        assert!(index.objects_are_conflicting(&obj1, &obj2));

        let obj3: ValueMap = vec![
            (
                Value::Text("name".to_string()),
                Value::Text("Sam".to_string()),
            ),
            (Value::Text("age".to_string()), Value::U64(25)),
        ];
        assert!(!index.objects_are_conflicting(&obj1, &obj3));
    }

    // -----------------------------------------------------------------------
    // Index::property_names() tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_property_names() {
        let index = make_index("idx", vec![("name", true), ("age", false)], false);
        let names = index.property_names();
        assert_eq!(names, vec!["name".to_string(), "age".to_string()]);
    }

    // -----------------------------------------------------------------------
    // Index::extract_values() tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_extract_values_with_matching_data() {
        let index = make_index("idx", vec![("name", true), ("age", true)], false);
        let mut data = BTreeMap::new();
        data.insert("name".to_string(), Value::Text("Sam".to_string()));
        data.insert("age".to_string(), Value::U64(30));
        let values = index.extract_values(&data);
        assert_eq!(values.len(), 2);
        assert_eq!(values[0], Value::Text("Sam".to_string()));
        assert_eq!(values[1], Value::U64(30));
    }

    #[test]
    fn test_extract_values_with_missing_data() {
        let index = make_index("idx", vec![("name", true), ("age", true)], false);
        let mut data = BTreeMap::new();
        data.insert("name".to_string(), Value::Text("Sam".to_string()));
        let values = index.extract_values(&data);
        assert_eq!(values.len(), 2);
        assert_eq!(values[0], Value::Text("Sam".to_string()));
        assert_eq!(values[1], Value::Null); // missing key returns Null
    }

    // -----------------------------------------------------------------------
    // Index::matches() tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_matches_exact_match() {
        let index = make_index("idx", vec![("name", true), ("age", true)], false);
        let result = index.matches(&["name", "age"], None, &[]);
        assert_eq!(result, Some(0));
    }

    #[test]
    fn test_matches_partial_match() {
        let index = make_index("idx", vec![("name", true), ("age", true)], false);
        let result = index.matches(&["name"], None, &[]);
        assert_eq!(result, Some(1));
    }

    #[test]
    fn test_matches_no_match() {
        let index = make_index("idx", vec![("name", true), ("age", true)], false);
        let result = index.matches(&["email"], None, &[]);
        assert_eq!(result, None);
    }

    #[test]
    fn test_matches_with_order_by() {
        let index = make_index("idx", vec![("name", true), ("age", true)], false);
        // Matching on "name" with order_by "age": d starts at 2, one match decrements to 1
        let result = index.matches(&["name"], None, &["age"]);
        assert_eq!(result, Some(1));
    }

    #[test]
    fn test_matches_in_field_last_property() {
        let index = make_index("idx", vec![("name", true), ("age", true)], false);
        let result = index.matches(&["name"], Some("age"), &[]);
        assert_eq!(result, Some(1));
    }

    #[test]
    fn test_matches_in_field_before_last() {
        let index = make_index("idx", vec![("name", true), ("age", true)], false);
        let result = index.matches(&["age"], Some("name"), &[]);
        assert_eq!(result, Some(1));
    }

    #[test]
    fn test_matches_in_field_not_matching() {
        let index = make_index("idx", vec![("name", true), ("age", true)], false);
        let result = index.matches(&["name"], Some("email"), &[]);
        assert_eq!(result, None);
    }

    // -----------------------------------------------------------------------
    // IndexProperty::try_from tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_index_property_try_from_asc() {
        let mut map = BTreeMap::new();
        map.insert("name".to_string(), "asc".to_string());
        let prop = IndexProperty::try_from(map).unwrap();
        assert_eq!(prop.name, "name");
        assert!(prop.ascending);
    }

    #[test]
    fn test_index_property_try_from_desc() {
        let mut map = BTreeMap::new();
        map.insert("age".to_string(), "desc".to_string());
        let prop = IndexProperty::try_from(map).unwrap();
        assert_eq!(prop.name, "age");
        assert!(!prop.ascending);
    }

    #[test]
    fn test_index_property_try_from_empty_map_error() {
        let map: BTreeMap<String, String> = BTreeMap::new();
        let result = IndexProperty::try_from(map);
        assert!(result.is_err());
    }

    #[test]
    fn test_index_property_try_from_multiple_entries_error() {
        let mut map = BTreeMap::new();
        map.insert("name".to_string(), "asc".to_string());
        map.insert("age".to_string(), "desc".to_string());
        let result = IndexProperty::try_from(map);
        assert!(result.is_err());
    }

    #[test]
    fn test_index_property_try_from_invalid_sort_order_error() {
        let mut map = BTreeMap::new();
        map.insert("name".to_string(), "random".to_string());
        let result = IndexProperty::try_from(map);
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // IndexProperty::from_platform_value() tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_index_property_from_platform_value_asc() {
        let map = vec![(
            Value::Text("fieldName".to_string()),
            Value::Text("asc".to_string()),
        )];
        let prop = IndexProperty::from_platform_value(&map).unwrap();
        assert_eq!(prop.name, "fieldName");
        assert!(prop.ascending);
    }

    #[test]
    fn test_index_property_from_platform_value_desc() {
        let map = vec![(
            Value::Text("fieldName".to_string()),
            Value::Text("desc".to_string()),
        )];
        let prop = IndexProperty::from_platform_value(&map).unwrap();
        assert_eq!(prop.name, "fieldName");
        assert!(!prop.ascending);
    }

    #[test]
    fn test_index_property_from_platform_value_bad_key_type() {
        let map = vec![(Value::U64(42), Value::Text("asc".to_string()))];
        let result = IndexProperty::from_platform_value(&map);
        assert!(result.is_err());
    }

    #[test]
    fn test_index_property_from_platform_value_bad_value_type() {
        let map = vec![(Value::Text("field".to_string()), Value::U64(1))];
        let result = IndexProperty::from_platform_value(&map);
        assert!(result.is_err());
    }

    #[test]
    fn test_index_property_from_platform_value_empty_map_returns_err() {
        // An empty index property object `{}` must not panic with an
        // out-of-bounds index. This is reachable in check_tx, where the
        // document meta-schema (minProperties: 1) is not enforced.
        let result = IndexProperty::from_platform_value(&[]);
        assert!(matches!(
            result,
            Err(DataContractError::InvalidContractStructure(_))
        ));
    }

    #[test]
    fn test_index_property_from_platform_value_multiple_entries_returns_err() {
        // More than one key/value violates the meta-schema `maxProperties: 1`,
        // which is also skipped in check_tx.
        let map = vec![
            (Value::Text("a".to_string()), Value::Text("asc".to_string())),
            (Value::Text("b".to_string()), Value::Text("asc".to_string())),
        ];
        let result = IndexProperty::from_platform_value(&map);
        assert!(matches!(
            result,
            Err(DataContractError::InvalidContractStructure(_))
        ));
    }

    // -----------------------------------------------------------------------
    // Index TryFrom<&[(Value, Value)]> tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_index_try_from_basic() {
        let index_map: Vec<(Value, Value)> = vec![
            (
                Value::Text("name".to_string()),
                Value::Text("test_index".to_string()),
            ),
            (Value::Text("unique".to_string()), Value::Bool(true)),
            (
                Value::Text("nullSearchable".to_string()),
                Value::Bool(false),
            ),
            (
                Value::Text("properties".to_string()),
                Value::Array(vec![Value::Map(vec![(
                    Value::Text("fieldA".to_string()),
                    Value::Text("asc".to_string()),
                )])]),
            ),
        ];
        let index = Index::try_from(index_map.as_slice()).unwrap();
        assert_eq!(index.name, "test_index");
        assert!(index.unique);
        assert!(!index.null_searchable);
        assert_eq!(index.properties.len(), 1);
        assert_eq!(index.properties[0].name, "fieldA");
        assert!(index.properties[0].ascending);
        assert!(index.contested_index.is_none());
    }

    #[test]
    fn test_index_try_from_without_name_derives_deterministic_name() {
        let index_map: Vec<(Value, Value)> = vec![(
            Value::Text("properties".to_string()),
            Value::Array(vec![Value::Map(vec![(
                Value::Text("fieldA".to_string()),
                Value::Text("asc".to_string()),
            )])]),
        )];
        let index = Index::try_from(index_map.as_slice()).unwrap();
        assert_eq!(index.name, "fieldA_asc");

        // Parsing the same definition again must produce the same name
        let again = Index::try_from(index_map.as_slice()).unwrap();
        assert_eq!(again.name, index.name);
    }

    #[test]
    fn test_index_try_from_without_name_multi_property_directions() {
        let index_map: Vec<(Value, Value)> = vec![(
            Value::Text("properties".to_string()),
            Value::Array(vec![
                Value::Map(vec![(
                    Value::Text("ownerId".to_string()),
                    Value::Text("asc".to_string()),
                )]),
                Value::Map(vec![(
                    Value::Text("createdAt".to_string()),
                    Value::Text("desc".to_string()),
                )]),
            ]),
        )];
        let index = Index::try_from(index_map.as_slice()).unwrap();
        assert_eq!(index.name, "ownerId_asc|createdAt_desc");
    }

    #[test]
    fn test_index_try_from_without_name_empty_properties_falls_back() {
        let index_map: Vec<(Value, Value)> =
            vec![(Value::Text("properties".to_string()), Value::Array(vec![]))];
        let index = Index::try_from(index_map.as_slice()).unwrap();
        assert_eq!(index.name, "index");
    }

    #[test]
    fn test_index_try_from_derived_names_distinct_for_distinct_definitions() {
        // A single property literally named "a_asc_b" (desc) must not derive
        // the same name as a compound index over "a" (asc) + "b" (desc):
        // the `|` joiner cannot appear in a validated property path.
        let single: Vec<(Value, Value)> = vec![(
            Value::Text("properties".to_string()),
            Value::Array(vec![Value::Map(vec![(
                Value::Text("a_asc_b".to_string()),
                Value::Text("desc".to_string()),
            )])]),
        )];
        let compound: Vec<(Value, Value)> = vec![(
            Value::Text("properties".to_string()),
            Value::Array(vec![
                Value::Map(vec![(
                    Value::Text("a".to_string()),
                    Value::Text("asc".to_string()),
                )]),
                Value::Map(vec![(
                    Value::Text("b".to_string()),
                    Value::Text("desc".to_string()),
                )]),
            ]),
        )];
        let single_index = Index::try_from(single.as_slice()).unwrap();
        let compound_index = Index::try_from(compound.as_slice()).unwrap();
        assert_eq!(single_index.name, "a_asc_b_desc");
        assert_eq!(compound_index.name, "a_asc|b_desc");
        assert_ne!(single_index.name, compound_index.name);
    }

    #[test]
    fn test_index_try_from_default_null_searchable_true() {
        let index_map: Vec<(Value, Value)> = vec![(
            Value::Text("properties".to_string()),
            Value::Array(vec![Value::Map(vec![(
                Value::Text("fieldA".to_string()),
                Value::Text("asc".to_string()),
            )])]),
        )];
        let index = Index::try_from(index_map.as_slice()).unwrap();
        assert!(index.null_searchable); // default is true
    }

    #[test]
    fn test_index_try_from_unknown_key_error() {
        let index_map: Vec<(Value, Value)> =
            vec![(Value::Text("unknownKey".to_string()), Value::Bool(true))];
        let result = Index::try_from(index_map.as_slice());
        assert!(result.is_err());
    }

    #[test]
    fn test_index_try_from_summable_string_sets_property() {
        let index_map: Vec<(Value, Value)> = vec![
            (
                Value::Text("properties".to_string()),
                Value::Array(vec![Value::Map(vec![(
                    Value::Text("recipient".to_string()),
                    Value::Text("asc".to_string()),
                )])]),
            ),
            (
                Value::Text("summable".to_string()),
                Value::Text("amount".to_string()),
            ),
        ];
        let index = Index::try_from(index_map.as_slice()).expect("valid index parses");
        assert_eq!(index.summable.as_deref(), Some("amount"));
        assert!(!index.range_summable);
    }

    #[test]
    fn test_index_try_from_summable_null_treated_as_none() {
        let index_map: Vec<(Value, Value)> = vec![
            (
                Value::Text("properties".to_string()),
                Value::Array(vec![Value::Map(vec![(
                    Value::Text("recipient".to_string()),
                    Value::Text("asc".to_string()),
                )])]),
            ),
            (Value::Text("summable".to_string()), Value::Null),
        ];
        let index = Index::try_from(index_map.as_slice()).expect("null summable parses");
        assert_eq!(index.summable, None);
    }

    #[test]
    fn test_index_try_from_summable_empty_string_rejected() {
        // Empty `summable: ""` is a contract bug — must reject at parse
        // time, not silently store `Some("")` and fail later in the
        // index picker.
        let index_map: Vec<(Value, Value)> = vec![
            (
                Value::Text("properties".to_string()),
                Value::Array(vec![Value::Map(vec![(
                    Value::Text("recipient".to_string()),
                    Value::Text("asc".to_string()),
                )])]),
            ),
            (
                Value::Text("summable".to_string()),
                Value::Text(String::new()),
            ),
        ];
        let result = Index::try_from(index_map.as_slice());
        assert!(result.is_err(), "empty summable string must error");
        let msg = format!("{:?}", result.unwrap_err());
        assert!(
            msg.contains("non-empty"),
            "error must reference the non-empty requirement; got: {msg}"
        );
    }

    #[test]
    fn test_index_try_from_summable_non_string_rejected() {
        let index_map: Vec<(Value, Value)> = vec![
            (
                Value::Text("properties".to_string()),
                Value::Array(vec![Value::Map(vec![(
                    Value::Text("recipient".to_string()),
                    Value::Text("asc".to_string()),
                )])]),
            ),
            (Value::Text("summable".to_string()), Value::Bool(true)),
        ];
        let result = Index::try_from(index_map.as_slice());
        assert!(result.is_err(), "non-string/non-null summable must error");
    }

    /// Canonical shorthand `{averageable: "x", rangeAverageable: true}`
    /// (no explicit `countable`) must succeed and desugar to all four
    /// underlying flags. Regression test for an inversion in the
    /// promotion logic where `range_averageable: true` blocked the
    /// silent-promote path and forced the explicit-contradiction path,
    /// rejecting the canonical shape.
    #[test]
    fn test_index_try_from_averageable_with_range_averageable_promotes_all_flags() {
        let index_map: Vec<(Value, Value)> = vec![
            (
                Value::Text("properties".to_string()),
                Value::Array(vec![Value::Map(vec![(
                    Value::Text("score".to_string()),
                    Value::Text("asc".to_string()),
                )])]),
            ),
            (
                Value::Text("averageable".to_string()),
                Value::Text("score".to_string()),
            ),
            (
                Value::Text("rangeAverageable".to_string()),
                Value::Bool(true),
            ),
        ];
        let index = Index::try_from(index_map.as_slice()).expect("canonical shorthand parses");
        assert!(
            index.countable.is_countable(),
            "averageable promotes countable"
        );
        assert_eq!(index.summable.as_deref(), Some("score"));
        assert!(
            index.range_countable,
            "rangeAverageable promotes range_countable"
        );
        assert!(
            index.range_summable,
            "rangeAverageable promotes range_summable"
        );
    }

    /// `averageable` + explicit `countable: "notCountable"` is a direct
    /// contradiction: the author wrote both "yes, averageable (which
    /// implies countable)" and "no, not countable" in the same index.
    /// Must reject. Regression test for the inversion that silently
    /// promoted the explicit `notCountable` to `Countable`.
    #[test]
    fn test_index_try_from_averageable_with_explicit_not_countable_rejected() {
        let index_map: Vec<(Value, Value)> = vec![
            (
                Value::Text("properties".to_string()),
                Value::Array(vec![Value::Map(vec![(
                    Value::Text("score".to_string()),
                    Value::Text("asc".to_string()),
                )])]),
            ),
            (
                Value::Text("averageable".to_string()),
                Value::Text("score".to_string()),
            ),
            (
                Value::Text("countable".to_string()),
                Value::Text("notCountable".to_string()),
            ),
        ];
        let result = Index::try_from(index_map.as_slice());
        assert!(
            result.is_err(),
            "averageable + explicit notCountable must be rejected"
        );
        let msg = format!("{:?}", result.unwrap_err());
        assert!(
            msg.contains("averageable") && msg.contains("countable"),
            "error must reference both averageable and countable; got {msg}"
        );
    }

    /// `averageable` alone (the simplest shorthand) must silently
    /// promote `countable` (and set `summable`) without requiring the
    /// author to also write `countable: "countable"`.
    #[test]
    fn test_index_try_from_averageable_alone_silently_promotes_countable() {
        let index_map: Vec<(Value, Value)> = vec![
            (
                Value::Text("properties".to_string()),
                Value::Array(vec![Value::Map(vec![(
                    Value::Text("score".to_string()),
                    Value::Text("asc".to_string()),
                )])]),
            ),
            (
                Value::Text("averageable".to_string()),
                Value::Text("score".to_string()),
            ),
        ];
        let index = Index::try_from(index_map.as_slice()).expect("averageable alone parses");
        assert!(index.countable.is_countable());
        assert_eq!(index.summable.as_deref(), Some("score"));
        assert!(!index.range_countable);
        assert!(!index.range_summable);
    }

    /// `rangeAverageable: true` + explicit `rangeCountable: false` is a
    /// direct contradiction: rangeAverageable is shorthand for both
    /// range axes opting in, but the author explicitly said "no range
    /// count". Must reject rather than silently flip.
    #[test]
    fn test_index_try_from_range_averageable_with_explicit_range_countable_false_rejected() {
        let index_map: Vec<(Value, Value)> = vec![
            (
                Value::Text("properties".to_string()),
                Value::Array(vec![Value::Map(vec![(
                    Value::Text("score".to_string()),
                    Value::Text("asc".to_string()),
                )])]),
            ),
            (
                Value::Text("averageable".to_string()),
                Value::Text("score".to_string()),
            ),
            (
                Value::Text("rangeAverageable".to_string()),
                Value::Bool(true),
            ),
            (
                Value::Text("rangeCountable".to_string()),
                Value::Bool(false),
            ),
        ];
        let result = Index::try_from(index_map.as_slice());
        assert!(
            result.is_err(),
            "rangeAverageable + explicit rangeCountable: false must reject"
        );
        let msg = format!("{:?}", result.unwrap_err());
        assert!(
            msg.contains("rangeAverageable") && msg.contains("rangeCountable: false"),
            "error must reference both flags; got {msg}"
        );
    }

    /// Symmetric case: `rangeAverageable: true` + explicit
    /// `rangeSummable: false` must also reject.
    #[test]
    fn test_index_try_from_range_averageable_with_explicit_range_summable_false_rejected() {
        let index_map: Vec<(Value, Value)> = vec![
            (
                Value::Text("properties".to_string()),
                Value::Array(vec![Value::Map(vec![(
                    Value::Text("score".to_string()),
                    Value::Text("asc".to_string()),
                )])]),
            ),
            (
                Value::Text("averageable".to_string()),
                Value::Text("score".to_string()),
            ),
            (
                Value::Text("rangeAverageable".to_string()),
                Value::Bool(true),
            ),
            (Value::Text("rangeSummable".to_string()), Value::Bool(false)),
        ];
        let result = Index::try_from(index_map.as_slice());
        assert!(
            result.is_err(),
            "rangeAverageable + explicit rangeSummable: false must reject"
        );
        let msg = format!("{:?}", result.unwrap_err());
        assert!(
            msg.contains("rangeAverageable") && msg.contains("rangeSummable: false"),
            "error must reference both flags; got {msg}"
        );
    }

    /// `rangeAverageable: true` + redundant explicit `rangeCountable:
    /// true` (and / or `rangeSummable: true`) is fine — the author
    /// agreed with what averageable promotes, no contradiction.
    #[test]
    fn test_index_try_from_range_averageable_with_explicit_range_countable_true_ok() {
        let index_map: Vec<(Value, Value)> = vec![
            (
                Value::Text("properties".to_string()),
                Value::Array(vec![Value::Map(vec![(
                    Value::Text("score".to_string()),
                    Value::Text("asc".to_string()),
                )])]),
            ),
            (
                Value::Text("averageable".to_string()),
                Value::Text("score".to_string()),
            ),
            (
                Value::Text("rangeAverageable".to_string()),
                Value::Bool(true),
            ),
            (Value::Text("rangeCountable".to_string()), Value::Bool(true)),
            (Value::Text("rangeSummable".to_string()), Value::Bool(true)),
        ];
        let index = Index::try_from(index_map.as_slice())
            .expect("rangeAverageable + redundant explicit true must parse");
        assert!(index.range_countable);
        assert!(index.range_summable);
        assert!(index.countable.is_countable());
        assert_eq!(index.summable.as_deref(), Some("score"));
    }

    // -----------------------------------------------------------------------
    // Ranked aggregate index tests (meta-schema v3 / PV14 grammar)
    // -----------------------------------------------------------------------

    /// Helper: an index map on `score` with the supplied extra entries.
    /// `properties` is always `[{score: asc}]` so the terminator — the level
    /// the ranking axes attach to — is unambiguous.
    fn ranked_index_map(extra: Vec<(&str, Value)>) -> Vec<(Value, Value)> {
        let mut map: Vec<(Value, Value)> = vec![(
            Value::Text("properties".to_string()),
            Value::Array(vec![Value::Map(vec![(
                Value::Text("score".to_string()),
                Value::Text("asc".to_string()),
            )])]),
        )];
        map.extend(
            extra
                .into_iter()
                .map(|(k, v)| (Value::Text(k.to_string()), v)),
        );
        map
    }

    /// All three ranking keywords parse as booleans on top of a fully
    /// range-averageable index.
    #[test]
    fn test_index_try_from_all_three_ranked_keywords_parse() {
        let index_map = ranked_index_map(vec![
            ("averageable", Value::Text("score".to_string())),
            ("rangeAverageable", Value::Bool(true)),
            ("rankedCountable", Value::Bool(true)),
            ("rankedSummable", Value::Bool(true)),
            ("rankedAverageable", Value::Bool(true)),
        ]);
        let index = Index::try_from_value_map(index_map.as_slice(), true)
            .expect("all three ranked keywords must parse when the grammar allows them");
        assert!(index.ranked_countable);
        assert!(index.ranked_summable);
        assert!(index.ranked_averageable);
        // The underlying range axes are still what the sugar desugared into.
        assert!(index.range_countable);
        assert!(index.range_summable);
    }

    /// Omitting the ranking keywords leaves all three axes off — the parser's
    /// default matches the `serde(default)` on the struct fields.
    #[test]
    fn test_index_try_from_ranked_flags_default_false() {
        let index_map = ranked_index_map(vec![
            ("averageable", Value::Text("score".to_string())),
            ("rangeAverageable", Value::Bool(true)),
        ]);
        let index = Index::try_from_value_map(index_map.as_slice(), true)
            .expect("index without ranked keywords must parse");
        assert!(!index.ranked_countable);
        assert!(!index.ranked_summable);
        assert!(!index.ranked_averageable);
    }

    /// Ranked flags on compound indexes are a v1 scope restriction (wrapper
    /// conflict under aggregating prefixes + no equality-prefix query
    /// routing), rejected at parse time so the limitation is visible at
    /// registration rather than at document-insert or query time.
    #[test]
    fn test_index_try_from_ranked_on_compound_index_rejected() {
        let mut index_map = ranked_index_map(vec![
            ("averageable", Value::Text("score".to_string())),
            ("rangeAverageable", Value::Bool(true)),
            ("rankedAverageable", Value::Bool(true)),
        ]);
        // Turn the single-property fixture into a compound [region, score].
        index_map[0].1 = Value::Array(vec![
            Value::Map(vec![(
                Value::Text("region".to_string()),
                Value::Text("asc".to_string()),
            )]),
            Value::Map(vec![(
                Value::Text("score".to_string()),
                Value::Text("asc".to_string()),
            )]),
        ]);
        let result = Index::try_from_value_map(index_map.as_slice(), true);
        assert!(
            result.is_err(),
            "ranked flags on a compound index must be rejected"
        );
        let msg = format!("{:?}", result.unwrap_err());
        assert!(
            msg.contains("single-property"),
            "error must explain the single-property restriction; got {msg}"
        );
    }

    /// Ranking is meaningless on a unique index: every group holds at most
    /// one document, so ranked flags there are an authoring mistake and are
    /// rejected rather than silently laying out an indexed tree.
    #[test]
    fn test_index_try_from_ranked_on_unique_index_rejected() {
        let index_map = ranked_index_map(vec![
            ("unique", Value::Bool(true)),
            ("averageable", Value::Text("score".to_string())),
            ("rangeAverageable", Value::Bool(true)),
            ("rankedAverageable", Value::Bool(true)),
        ]);
        let result = Index::try_from_value_map(index_map.as_slice(), true);
        assert!(
            result.is_err(),
            "ranked flags on a unique index must be rejected"
        );
        let msg = format!("{:?}", result.unwrap_err());
        assert!(
            msg.contains("unique"),
            "error must explain the unique-index restriction; got {msg}"
        );
    }

    /// A ranking axis needs the matching range axis: the ordered secondary is
    /// built from the per-group aggregate only the range layout maintains.
    #[test]
    fn test_index_try_from_ranked_countable_without_range_countable_rejected() {
        // `countable` alone (no `rangeCountable`) is not enough.
        let index_map = ranked_index_map(vec![
            ("countable", Value::Text("countable".to_string())),
            ("rankedCountable", Value::Bool(true)),
        ]);
        let result = Index::try_from_value_map(index_map.as_slice(), true);
        assert!(
            result.is_err(),
            "rankedCountable without rangeCountable must be rejected"
        );
        let msg = format!("{:?}", result.unwrap_err());
        assert!(
            msg.contains("rankedCountable") && msg.contains("rangeCountable"),
            "error must name both flags; got {msg}"
        );
    }

    #[test]
    fn test_index_try_from_ranked_summable_without_range_summable_rejected() {
        let index_map = ranked_index_map(vec![
            ("summable", Value::Text("score".to_string())),
            ("rankedSummable", Value::Bool(true)),
        ]);
        let result = Index::try_from_value_map(index_map.as_slice(), true);
        assert!(
            result.is_err(),
            "rankedSummable without rangeSummable must be rejected"
        );
        let msg = format!("{:?}", result.unwrap_err());
        assert!(
            msg.contains("rankedSummable") && msg.contains("rangeSummable"),
            "error must name both flags; got {msg}"
        );
    }

    /// `rankedAverageable` needs BOTH range axes. Only the count half here.
    #[test]
    fn test_index_try_from_ranked_averageable_without_range_summable_rejected() {
        let index_map = ranked_index_map(vec![
            ("countable", Value::Text("countable".to_string())),
            ("rangeCountable", Value::Bool(true)),
            ("rankedAverageable", Value::Bool(true)),
        ]);
        let result = Index::try_from_value_map(index_map.as_slice(), true);
        assert!(
            result.is_err(),
            "rankedAverageable with only the count range axis must be rejected"
        );
        let msg = format!("{:?}", result.unwrap_err());
        assert!(
            msg.contains("rankedAverageable"),
            "error must name rankedAverageable; got {msg}"
        );
    }

    /// Only the sum half — symmetric rejection.
    #[test]
    fn test_index_try_from_ranked_averageable_without_range_countable_rejected() {
        let index_map = ranked_index_map(vec![
            ("summable", Value::Text("score".to_string())),
            ("rangeSummable", Value::Bool(true)),
            ("rankedAverageable", Value::Bool(true)),
        ]);
        let result = Index::try_from_value_map(index_map.as_slice(), true);
        assert!(
            result.is_err(),
            "rankedAverageable with only the sum range axis must be rejected"
        );
        let msg = format!("{:?}", result.unwrap_err());
        assert!(
            msg.contains("rankedAverageable"),
            "error must name rankedAverageable; got {msg}"
        );
    }

    /// `rankedAverageable` accepted through the sugar form — `averageable` +
    /// `rangeAverageable` desugar into both range axes before the ranking
    /// checks run.
    #[test]
    fn test_index_try_from_ranked_averageable_with_sugar_form() {
        let index_map = ranked_index_map(vec![
            ("averageable", Value::Text("score".to_string())),
            ("rangeAverageable", Value::Bool(true)),
            ("rankedAverageable", Value::Bool(true)),
        ]);
        let index = Index::try_from_value_map(index_map.as_slice(), true)
            .expect("rankedAverageable on the averageable sugar form must parse");
        assert!(index.ranked_averageable);
        assert!(index.countable.is_countable());
        assert_eq!(index.summable.as_deref(), Some("score"));
        assert!(index.range_countable);
        assert!(index.range_summable);
        // Ranking axes are independent: the Avg opt-in adds no other axis.
        assert!(
            !index.ranked_countable && !index.ranked_summable,
            "rankedAverageable must not imply rankedCountable / rankedSummable — each \
             ranking axis costs its own ordered secondary tree"
        );
    }

    /// Same acceptance through the explicit longhand — `countable`, `summable`
    /// and both range flags spelled out, no sugar keyword anywhere. The
    /// structural rule is stated in terms of the resolved range axes, so both
    /// spellings pass.
    #[test]
    fn test_index_try_from_ranked_averageable_with_explicit_longhand_form() {
        let index_map = ranked_index_map(vec![
            ("countable", Value::Text("countable".to_string())),
            ("summable", Value::Text("score".to_string())),
            ("rangeCountable", Value::Bool(true)),
            ("rangeSummable", Value::Bool(true)),
            ("rankedAverageable", Value::Bool(true)),
        ]);
        let index = Index::try_from_value_map(index_map.as_slice(), true)
            .expect("rankedAverageable on the explicit longhand form must parse");
        assert!(index.ranked_averageable);
        assert!(index.range_countable);
        assert!(index.range_summable);
        assert!(!index.ranked_countable);
        assert!(!index.ranked_summable);
    }

    /// The keywords only accept booleans.
    #[test]
    fn test_index_try_from_ranked_non_boolean_rejected() {
        for key in ["rankedCountable", "rankedSummable", "rankedAverageable"] {
            let index_map = ranked_index_map(vec![
                ("averageable", Value::Text("score".to_string())),
                ("rangeAverageable", Value::Bool(true)),
                (key, Value::Text("yes".to_string())),
            ]);
            let result = Index::try_from_value_map(index_map.as_slice(), true);
            assert!(result.is_err(), "{key} must reject a non-boolean value");
            let msg = format!("{:?}", result.unwrap_err());
            assert!(
                msg.contains(key) && msg.contains("boolean"),
                "error must name {key} and the boolean requirement; got {msg}"
            );
        }
    }

    /// With the pre-v3 grammar (`ranked_aggregates_allowed: false`, i.e.
    /// `document_type_schema < 3`) the ranked keywords are not keywords at
    /// all — they hit the unknown-property arm and produce exactly the error a
    /// node without this feature produces. This is the gate that stops a
    /// non-validating parse from smuggling a ranked index onto a pre-PV14 node.
    #[test]
    fn test_index_try_from_ranked_keys_rejected_when_grammar_disallows() {
        for key in ["rankedCountable", "rankedSummable", "rankedAverageable"] {
            for value in [Value::Bool(true), Value::Bool(false)] {
                let index_map = ranked_index_map(vec![
                    ("averageable", Value::Text("score".to_string())),
                    ("rangeAverageable", Value::Bool(true)),
                    (key, value.clone()),
                ]);
                let result = Index::try_from_value_map(index_map.as_slice(), false);
                assert!(
                    result.is_err(),
                    "{key}: {value:?} must be rejected when the ranked grammar is off"
                );
                let msg = format!("{:?}", result.unwrap_err());
                assert!(
                    msg.contains("unexpected property name"),
                    "pre-v3 rejection must be the unknown-key error (byte-identical to a \
                     node without the feature); got {msg}"
                );
            }
        }
    }

    /// The `TryFrom` impl is the pre-v3 surface, so it rejects ranked keys.
    #[test]
    fn test_index_try_from_trait_impl_rejects_ranked_keys() {
        let index_map = ranked_index_map(vec![
            ("averageable", Value::Text("score".to_string())),
            ("rangeAverageable", Value::Bool(true)),
            ("rankedCountable", Value::Bool(true)),
        ]);
        let result = Index::try_from(index_map.as_slice());
        assert!(
            result.is_err(),
            "TryFrom is the pre-meta-schema-v3 grammar and must reject ranked keys"
        );
    }

    /// The three range/ranked flag pairs that make a single-property index
    /// legally rankable, one per axis. Used by the `nullSearchable` tests so
    /// each axis is checked on its own rather than through the Avg superset.
    fn ranked_axis_fixtures() -> Vec<(&'static str, Vec<(&'static str, Value)>)> {
        vec![
            (
                "rankedCountable",
                vec![
                    ("countable", Value::Text("countable".to_string())),
                    ("rangeCountable", Value::Bool(true)),
                    ("rankedCountable", Value::Bool(true)),
                ],
            ),
            (
                "rankedSummable",
                vec![
                    ("summable", Value::Text("score".to_string())),
                    ("rangeSummable", Value::Bool(true)),
                    ("rankedSummable", Value::Bool(true)),
                ],
            ),
            (
                "rankedAverageable",
                vec![
                    ("averageable", Value::Text("score".to_string())),
                    ("rangeAverageable", Value::Bool(true)),
                    ("rankedAverageable", Value::Bool(true)),
                ],
            ),
        ]
    }

    /// `nullSearchable: false` next to any ranking axis is refused at parse
    /// time. The write path would still create the null group's value tree —
    /// that happens in the index walker, before the terminal handler decides
    /// to suppress the reference — so grovedb's secondary would carry a group
    /// with zero aggregates and no documents behind it, and an authenticated
    /// TOP/BOTTOM answer would include a group the index excludes.
    #[test]
    fn test_index_try_from_ranked_with_explicit_null_searchable_false_rejected() {
        for (axis, mut extra) in ranked_axis_fixtures() {
            extra.push(("nullSearchable", Value::Bool(false)));
            let index_map = ranked_index_map(extra);
            let result = Index::try_from_value_map(index_map.as_slice(), true);
            assert!(
                result.is_err(),
                "{axis} with nullSearchable: false must be rejected"
            );
            let msg = format!("{:?}", result.unwrap_err());
            assert!(
                msg.contains("nullSearchable") && msg.contains("phantom"),
                "error must name nullSearchable and explain the phantom group; got {msg}"
            );
        }
    }

    /// Omitting the key leaves the default (`true`), which is the shape the
    /// rule permits: null documents get their real reference and form a
    /// legitimate rankable group.
    #[test]
    fn test_index_try_from_ranked_without_null_searchable_key_accepted() {
        for (axis, extra) in ranked_axis_fixtures() {
            let index_map = ranked_index_map(extra);
            let index = Index::try_from_value_map(index_map.as_slice(), true)
                .unwrap_or_else(|e| panic!("{axis} with no nullSearchable key must parse: {e:?}"));
            assert!(
                index.null_searchable,
                "{axis}: the absent key must still default to true"
            );
        }
    }

    /// Spelling out the default explicitly is equally fine — the rule is
    /// about the resolved value, and `true` is the safe one.
    #[test]
    fn test_index_try_from_ranked_with_explicit_null_searchable_true_accepted() {
        for (axis, mut extra) in ranked_axis_fixtures() {
            extra.push(("nullSearchable", Value::Bool(true)));
            let index_map = ranked_index_map(extra);
            let index = Index::try_from_value_map(index_map.as_slice(), true).unwrap_or_else(|e| {
                panic!("{axis} with an explicit nullSearchable: true must parse: {e:?}")
            });
            assert!(index.null_searchable);
        }
    }

    /// The restriction is scoped to ranked indexes: `nullSearchable: false`
    /// on an index without a ranking axis keeps working exactly as before,
    /// including on the aggregating (range) layouts the ranking axes extend.
    #[test]
    fn test_index_try_from_non_ranked_with_null_searchable_false_still_accepted() {
        let plain = ranked_index_map(vec![("nullSearchable", Value::Bool(false))]);
        let index = Index::try_from_value_map(plain.as_slice(), true)
            .expect("nullSearchable: false on a plain index must still parse");
        assert!(!index.null_searchable);

        let aggregating = ranked_index_map(vec![
            ("averageable", Value::Text("score".to_string())),
            ("rangeAverageable", Value::Bool(true)),
            ("nullSearchable", Value::Bool(false)),
        ]);
        let index = Index::try_from_value_map(aggregating.as_slice(), true)
            .expect("nullSearchable: false on a range-averageable index must still parse");
        assert!(!index.null_searchable);
        assert!(!index.ranked_averageable);
    }

    #[test]
    fn test_index_try_from_contested_without_unique_error() {
        let index_map: Vec<(Value, Value)> = vec![
            (
                Value::Text("properties".to_string()),
                Value::Array(vec![Value::Map(vec![(
                    Value::Text("fieldA".to_string()),
                    Value::Text("asc".to_string()),
                )])]),
            ),
            (
                Value::Text("contested".to_string()),
                Value::Map(vec![(Value::Text("resolution".to_string()), Value::U64(0))]),
            ),
        ];
        let result = Index::try_from(index_map.as_slice());
        assert!(result.is_err()); // contest supported only for unique indexes
    }

    #[test]
    fn test_index_try_from_contested_with_unique() {
        let index_map: Vec<(Value, Value)> = vec![
            (Value::Text("unique".to_string()), Value::Bool(true)),
            (
                Value::Text("properties".to_string()),
                Value::Array(vec![Value::Map(vec![(
                    Value::Text("fieldA".to_string()),
                    Value::Text("asc".to_string()),
                )])]),
            ),
            (
                Value::Text("contested".to_string()),
                Value::Map(vec![
                    (Value::Text("resolution".to_string()), Value::U64(0)),
                    (
                        Value::Text("fieldMatches".to_string()),
                        Value::Array(vec![Value::Map(vec![
                            (
                                Value::Text("field".to_string()),
                                Value::Text("normalizedLabel".to_string()),
                            ),
                            (
                                Value::Text("regexPattern".to_string()),
                                Value::Text("^[a-zA-Z]+$".to_string()),
                            ),
                        ])]),
                    ),
                ]),
            ),
        ];
        let index = Index::try_from(index_map.as_slice()).unwrap();
        assert!(index.unique);
        assert!(index.contested_index.is_some());
        let contested = index.contested_index.unwrap();
        assert_eq!(
            contested.resolution,
            ContestedIndexResolution::MasternodeVote
        );
        assert!(contested.field_matches.contains_key("normalizedLabel"));
    }

    // -----------------------------------------------------------------------
    // OrderBy tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_order_by_partial_ord() {
        assert!(OrderBy::Asc < OrderBy::Desc);
    }

    // -----------------------------------------------------------------------
    // Additional objects_are_conflicting tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_objects_are_conflicting_both_null_values_not_conflicting() {
        // If either property is null (missing) for either object, they should not conflict
        let index = make_index("idx", vec![("name", true), ("age", true)], true);
        // obj1 has name but not age, obj2 has name but not age
        let obj1: ValueMap = vec![(
            Value::Text("name".to_string()),
            Value::Text("Sam".to_string()),
        )];
        let obj2: ValueMap = vec![(
            Value::Text("name".to_string()),
            Value::Text("Sam".to_string()),
        )];
        // Even though "name" matches, "age" is missing in both, so no conflict
        assert!(!index.objects_are_conflicting(&obj1, &obj2));
    }

    #[test]
    fn test_objects_are_conflicting_unique_three_properties_all_match() {
        let index = make_index("idx", vec![("a", true), ("b", true), ("c", true)], true);
        let obj1: ValueMap = vec![
            (Value::Text("a".to_string()), Value::U64(1)),
            (Value::Text("b".to_string()), Value::U64(2)),
            (Value::Text("c".to_string()), Value::U64(3)),
        ];
        let obj2: ValueMap = vec![
            (Value::Text("a".to_string()), Value::U64(1)),
            (Value::Text("b".to_string()), Value::U64(2)),
            (Value::Text("c".to_string()), Value::U64(3)),
        ];
        assert!(index.objects_are_conflicting(&obj1, &obj2));
    }

    #[test]
    fn test_objects_are_conflicting_unique_three_properties_one_different() {
        let index = make_index("idx", vec![("a", true), ("b", true), ("c", true)], true);
        let obj1: ValueMap = vec![
            (Value::Text("a".to_string()), Value::U64(1)),
            (Value::Text("b".to_string()), Value::U64(2)),
            (Value::Text("c".to_string()), Value::U64(3)),
        ];
        let obj2: ValueMap = vec![
            (Value::Text("a".to_string()), Value::U64(1)),
            (Value::Text("b".to_string()), Value::U64(999)), // different
            (Value::Text("c".to_string()), Value::U64(3)),
        ];
        assert!(!index.objects_are_conflicting(&obj1, &obj2));
    }

    #[test]
    fn test_objects_are_conflicting_non_unique_same_values_still_false() {
        // Even with identical values, non-unique index should never conflict
        let index = make_index("idx", vec![("x", true), ("y", true)], false);
        let obj1: ValueMap = vec![
            (Value::Text("x".to_string()), Value::U64(1)),
            (Value::Text("y".to_string()), Value::U64(2)),
        ];
        let obj2: ValueMap = vec![
            (Value::Text("x".to_string()), Value::U64(1)),
            (Value::Text("y".to_string()), Value::U64(2)),
        ];
        assert!(!index.objects_are_conflicting(&obj1, &obj2));
    }

    #[test]
    fn test_objects_are_conflicting_first_obj_missing_property() {
        let index = make_index("idx", vec![("name", true)], true);
        let obj1: ValueMap = vec![];
        let obj2: ValueMap = vec![(
            Value::Text("name".to_string()),
            Value::Text("Sam".to_string()),
        )];
        assert!(!index.objects_are_conflicting(&obj1, &obj2));
    }

    // -----------------------------------------------------------------------
    // Additional ContestedIndexFieldMatch::matches() tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_contested_index_field_match_regex_full_match() {
        let m = ContestedIndexFieldMatch::Regex(LazyRegex::new("^[0-9]{3}$".to_string()));
        assert!(m.matches(&Value::Text("123".to_string())));
        assert!(!m.matches(&Value::Text("1234".to_string())));
        assert!(!m.matches(&Value::Text("ab3".to_string())));
    }

    #[test]
    fn test_contested_index_field_match_regex_empty_string() {
        let m = ContestedIndexFieldMatch::Regex(LazyRegex::new("^$".to_string()));
        assert!(m.matches(&Value::Text("".to_string())));
        assert!(!m.matches(&Value::Text("x".to_string())));
    }

    #[test]
    fn test_contested_index_field_match_regex_null_value() {
        let m = ContestedIndexFieldMatch::Regex(LazyRegex::new(".*".to_string()));
        assert!(!m.matches(&Value::Null));
    }

    #[test]
    fn test_contested_index_field_match_regex_bool_value() {
        let m = ContestedIndexFieldMatch::Regex(LazyRegex::new("true".to_string()));
        assert!(!m.matches(&Value::Bool(true)));
    }

    #[test]
    fn test_contested_index_field_match_positive_integer_zero() {
        let m = ContestedIndexFieldMatch::PositiveIntegerMatch(0);
        assert!(m.matches(&Value::U64(0)));
        assert!(!m.matches(&Value::U64(1)));
    }

    #[test]
    fn test_contested_index_field_match_positive_integer_null_value() {
        let m = ContestedIndexFieldMatch::PositiveIntegerMatch(42);
        assert!(!m.matches(&Value::Null));
    }

    #[test]
    fn test_contested_index_field_match_positive_integer_bool_value() {
        let m = ContestedIndexFieldMatch::PositiveIntegerMatch(1);
        assert!(!m.matches(&Value::Bool(true)));
    }

    // -----------------------------------------------------------------------
    // Additional ContestedIndexFieldMatch Ord tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_contested_index_field_match_ord_regex_same_length() {
        let a = ContestedIndexFieldMatch::Regex(LazyRegex::new("ab".to_string()));
        let b = ContestedIndexFieldMatch::Regex(LazyRegex::new("cd".to_string()));
        // Same length means Equal
        assert_eq!(a.cmp(&b), Ordering::Equal);
    }

    #[test]
    fn test_contested_index_field_match_ord_integer_equal() {
        let a = ContestedIndexFieldMatch::PositiveIntegerMatch(100);
        let b = ContestedIndexFieldMatch::PositiveIntegerMatch(100);
        assert_eq!(a.cmp(&b), Ordering::Equal);
    }

    #[test]
    fn test_contested_index_field_match_partial_ord_regex_vs_integer() {
        let regex = ContestedIndexFieldMatch::Regex(LazyRegex::new("abc".to_string()));
        let integer = ContestedIndexFieldMatch::PositiveIntegerMatch(10);
        assert_eq!(regex.partial_cmp(&integer), Some(Ordering::Less));
        assert_eq!(integer.partial_cmp(&regex), Some(Ordering::Greater));
    }

    #[test]
    fn test_contested_index_field_match_partial_ord_integers() {
        let a = ContestedIndexFieldMatch::PositiveIntegerMatch(5);
        let b = ContestedIndexFieldMatch::PositiveIntegerMatch(10);
        assert_eq!(a.partial_cmp(&b), Some(Ordering::Less));
        assert_eq!(b.partial_cmp(&a), Some(Ordering::Greater));
        let c = ContestedIndexFieldMatch::PositiveIntegerMatch(5);
        assert_eq!(a.partial_cmp(&c), Some(Ordering::Equal));
    }

    #[test]
    fn test_contested_index_field_match_partial_ord_regex_by_length() {
        let short = ContestedIndexFieldMatch::Regex(LazyRegex::new("x".to_string()));
        let long = ContestedIndexFieldMatch::Regex(LazyRegex::new("xxxxxxxxxxxx".to_string()));
        assert_eq!(short.partial_cmp(&long), Some(Ordering::Less));
        assert_eq!(long.partial_cmp(&short), Some(Ordering::Greater));
    }

    // -----------------------------------------------------------------------
    // Additional IndexProperty::TryFrom<BTreeMap> tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_index_property_try_from_unknown_direction() {
        let mut map = BTreeMap::new();
        map.insert("field".to_string(), "up".to_string());
        let result = IndexProperty::try_from(map);
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("up"));
    }

    #[test]
    fn test_index_property_try_from_empty_map() {
        let map: BTreeMap<String, String> = BTreeMap::new();
        let result = IndexProperty::try_from(map);
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("empty"));
    }

    #[test]
    fn test_index_property_try_from_three_entries_error() {
        let mut map = BTreeMap::new();
        map.insert("a".to_string(), "asc".to_string());
        map.insert("b".to_string(), "desc".to_string());
        map.insert("c".to_string(), "asc".to_string());
        let result = IndexProperty::try_from(map);
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("more than one"));
    }
}

// --- canonical conversion trait impls (unification pass 1) ---
#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
impl crate::serialization::JsonConvertible for OrderBy {}

#[cfg(all(feature = "value-conversion", feature = "serde-conversion"))]
impl crate::serialization::ValueConvertible for OrderBy {}

#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
impl crate::serialization::JsonConvertible for ContestedIndexResolution {}

#[cfg(all(feature = "value-conversion", feature = "serde-conversion"))]
impl crate::serialization::ValueConvertible for ContestedIndexResolution {}

#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
impl crate::serialization::JsonConvertible for ContestedIndexFieldMatch {}

#[cfg(all(feature = "value-conversion", feature = "serde-conversion"))]
impl crate::serialization::ValueConvertible for ContestedIndexFieldMatch {}

#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
impl crate::serialization::JsonConvertible for ContestedIndexInformation {}

#[cfg(all(feature = "value-conversion", feature = "serde-conversion"))]
impl crate::serialization::ValueConvertible for ContestedIndexInformation {}

#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
impl crate::serialization::JsonConvertible for Index {}

#[cfg(all(feature = "value-conversion", feature = "serde-conversion"))]
impl crate::serialization::ValueConvertible for Index {}

#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
impl crate::serialization::JsonConvertible for IndexProperty {}

#[cfg(all(feature = "value-conversion", feature = "serde-conversion"))]
impl crate::serialization::ValueConvertible for IndexProperty {}

#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
impl crate::serialization::JsonConvertible for IndexCountability {}

#[cfg(all(feature = "value-conversion", feature = "serde-conversion"))]
impl crate::serialization::ValueConvertible for IndexCountability {}

#[cfg(all(
    test,
    feature = "json-conversion",
    feature = "value-conversion",
    feature = "serde-conversion"
))]
mod json_convertible_tests {
    use super::*;

    fn ix_info_fixture() -> ContestedIndexInformation {
        ContestedIndexInformation::default()
    }

    #[test]
    fn json_round_trip_contested_index_information() {
        use crate::serialization::JsonConvertible;
        let original = ix_info_fixture();
        let json = original.to_json().expect("to_json");
        let recovered = ContestedIndexInformation::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_contested_index_information() {
        use crate::serialization::ValueConvertible;
        let original = ix_info_fixture();
        let value = original.to_object().expect("to_object");
        let recovered = ContestedIndexInformation::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }

    #[test]
    fn json_round_trip_order_by() {
        use crate::serialization::JsonConvertible;
        for original in [OrderBy::Asc, OrderBy::Desc] {
            let json = original.to_json().expect("to_json");
            let recovered = OrderBy::from_json(json).expect("from_json");
            assert_eq!(original, recovered, "variant: {:?}", original);
        }
    }

    #[test]
    fn json_round_trip_contested_index_resolution() {
        use crate::serialization::JsonConvertible;
        let original = ContestedIndexResolution::MasternodeVote;
        let json = original.to_json().expect("to_json");
        let recovered = ContestedIndexResolution::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    // --- ContestedIndexFieldMatch (internal `$type` tag) ---
    // Wire shape: internally tagged with a uniform `value` payload.
    //   `{"$type":"regex","value":"<pattern>"}` -> Regex(LazyRegex)
    //   `{"$type":"positiveIntegerMatch","value":<u128>}` -> PositiveIntegerMatch
    // LazyRegex serializes as the bare regex string via
    // `serde(from = "String", into = "String")`, carried in `value`.

    #[test]
    fn json_round_trip_contested_index_field_match_regex() {
        use crate::serialization::JsonConvertible;
        let original = ContestedIndexFieldMatch::Regex(LazyRegex::new("^dash$".to_string()));
        let json = original.to_json().expect("to_json");
        assert_eq!(
            json,
            serde_json::json!({ "$type": "regex", "value": "^dash$" })
        );
        let recovered = ContestedIndexFieldMatch::from_json(json).expect("from_json");
        match recovered {
            ContestedIndexFieldMatch::Regex(r) => assert_eq!(r.as_str(), "^dash$"),
            other => panic!("expected Regex, got {:?}", other),
        }
    }

    #[test]
    fn json_round_trip_contested_index_field_match_positive_integer() {
        use crate::serialization::JsonConvertible;
        let original = ContestedIndexFieldMatch::PositiveIntegerMatch(42);
        let json = original.to_json().expect("to_json");
        assert_eq!(
            json,
            serde_json::json!({ "$type": "positiveIntegerMatch", "value": 42 })
        );
        let recovered = ContestedIndexFieldMatch::from_json(json).expect("from_json");
        match recovered {
            ContestedIndexFieldMatch::PositiveIntegerMatch(n) => assert_eq!(n, 42),
            other => panic!("expected PositiveIntegerMatch, got {:?}", other),
        }
    }

    #[test]
    fn value_round_trip_contested_index_field_match_regex() {
        use crate::serialization::ValueConvertible;
        let original = ContestedIndexFieldMatch::Regex(LazyRegex::new("[a-z]+".to_string()));
        let value = original.to_object().expect("to_object");
        // LazyRegex serializes as a bare string in non-HR Value too.
        assert_eq!(
            value,
            platform_value::platform_value!({ "$type": "regex", "value": "[a-z]+" })
        );
        let recovered = ContestedIndexFieldMatch::from_object(value).expect("from_object");
        match recovered {
            ContestedIndexFieldMatch::Regex(r) => assert_eq!(r.as_str(), "[a-z]+"),
            other => panic!("expected Regex, got {:?}", other),
        }
    }

    #[test]
    fn value_round_trip_contested_index_field_match_positive_integer() {
        use crate::serialization::ValueConvertible;
        let original = ContestedIndexFieldMatch::PositiveIntegerMatch(u128::MAX);
        let value = original.to_object().expect("to_object");
        // u128::MAX exceeds u64::MAX, so it's encoded as a string (Content-safe;
        // serde's internal-tag buffer can't hold a 128-bit int). Values that fit
        // in u64 stay numeric.
        assert_eq!(
            value,
            platform_value::platform_value!({ "$type": "positiveIntegerMatch", "value": "340282366920938463463374607431768211455" })
        );
        let recovered = ContestedIndexFieldMatch::from_object(value).expect("from_object");
        match recovered {
            ContestedIndexFieldMatch::PositiveIntegerMatch(n) => assert_eq!(n, u128::MAX),
            other => panic!("expected PositiveIntegerMatch, got {:?}", other),
        }
    }

    // --- Index / IndexProperty / IndexCountability (count + sum fields from
    // base PRs #3623 / #3661) ---

    fn index_fixture() -> Index {
        Index {
            name: "byOwnerAndPrice".to_string(),
            properties: vec![
                IndexProperty {
                    name: "ownerId".to_string(),
                    ascending: true,
                },
                IndexProperty {
                    name: "price".to_string(),
                    ascending: false,
                },
            ],
            unique: false,
            null_searchable: true,
            contested_index: None,
            countable: IndexCountability::CountableAllowingOffset,
            range_countable: true,
            summable: Some("price".to_string()),
            range_summable: true,
            // Ranking axes set asymmetrically on purpose: the round-trip has to
            // prove each flag survives independently, which a uniform
            // all-true / all-false fixture cannot.
            ranked_countable: true,
            ranked_summable: false,
            ranked_averageable: true,
        }
    }

    #[test]
    fn index_json_round_trip_with_full_wire_shape() {
        use crate::serialization::JsonConvertible;
        let original = index_fixture();
        let json = original.to_json().expect("to_json");
        // Internal (non-user-authored) shape: snake_case field names, no
        // rename_all on the struct. `countable` is the camelCase-renamed
        // IndexCountability unit enum.
        assert_eq!(
            json,
            serde_json::json!({
                "name": "byOwnerAndPrice",
                "properties": [
                    {"name": "ownerId", "ascending": true},
                    {"name": "price", "ascending": false},
                ],
                "unique": false,
                "null_searchable": true,
                "contested_index": serde_json::Value::Null,
                "countable": "countableAllowingOffset",
                "range_countable": true,
                "summable": "price",
                "range_summable": true,
                "ranked_countable": true,
                "ranked_summable": false,
                "ranked_averageable": true,
            })
        );
        let recovered = Index::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn index_value_round_trip() {
        use crate::serialization::ValueConvertible;
        let original = index_fixture();
        let value = original.to_object().expect("to_object");
        let recovered = Index::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }

    /// JSON serialized before the count (#3623) and sum (#3661) fields existed
    /// must still deserialize — the four new fields default (NotCountable /
    /// false / None / false). Without `serde(default)` this fails with
    /// "missing field `countable`". The three ranking flags (added later still,
    /// for the PV14 ranked-aggregate grammar) default the same way.
    #[test]
    fn index_deserializes_pre_count_sum_json() {
        use crate::serialization::JsonConvertible;
        let old_json = serde_json::json!({
            "name": "byOwner",
            "properties": [{"name": "ownerId", "ascending": true}],
            "unique": true,
            "null_searchable": false,
            "contested_index": serde_json::Value::Null,
        });
        let recovered = Index::from_json(old_json).expect("pre-#3623 JSON must deserialize");
        assert_eq!(recovered.countable, IndexCountability::NotCountable);
        assert!(!recovered.range_countable);
        assert_eq!(recovered.summable, None);
        assert!(!recovered.range_summable);
        assert!(!recovered.ranked_countable);
        assert!(!recovered.ranked_summable);
        assert!(!recovered.ranked_averageable);
    }

    /// JSON that predates only the *ranking* flags (it already carries the
    /// count and sum fields) must also deserialize, with all three ranking
    /// axes defaulting to `false`. This is the shape any `Index` serialized
    /// between the sum work and the PV14 ranked grammar has.
    #[test]
    fn index_deserializes_pre_ranked_json() {
        use crate::serialization::JsonConvertible;
        let old_json = serde_json::json!({
            "name": "byOwnerAndPrice",
            "properties": [{"name": "ownerId", "ascending": true}],
            "unique": false,
            "null_searchable": true,
            "contested_index": serde_json::Value::Null,
            "countable": "countable",
            "range_countable": true,
            "summable": "price",
            "range_summable": true,
        });
        let recovered = Index::from_json(old_json).expect("pre-ranked JSON must deserialize");
        assert_eq!(recovered.countable, IndexCountability::Countable);
        assert!(recovered.range_countable);
        assert_eq!(recovered.summable.as_deref(), Some("price"));
        assert!(recovered.range_summable);
        assert!(!recovered.ranked_countable);
        assert!(!recovered.ranked_summable);
        assert!(!recovered.ranked_averageable);
    }

    #[test]
    fn index_property_json_round_trip_with_full_wire_shape() {
        use crate::serialization::JsonConvertible;
        let original = IndexProperty {
            name: "ownerId".to_string(),
            ascending: false,
        };
        let json = original.to_json().expect("to_json");
        assert_eq!(
            json,
            serde_json::json!({"name": "ownerId", "ascending": false})
        );
        let recovered = IndexProperty::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn index_property_value_round_trip_with_full_wire_shape() {
        use crate::serialization::ValueConvertible;
        use platform_value::platform_value;
        let original = IndexProperty {
            name: "ownerId".to_string(),
            ascending: false,
        };
        let value = original.to_object().expect("to_object");
        assert_eq!(
            value,
            platform_value!({"name": "ownerId", "ascending": false})
        );
        let recovered = IndexProperty::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }

    #[test]
    fn index_countability_round_trips_all_variants() {
        use crate::serialization::{JsonConvertible, ValueConvertible};
        let cases = [
            (IndexCountability::NotCountable, "notCountable"),
            (IndexCountability::Countable, "countable"),
            (
                IndexCountability::CountableAllowingOffset,
                "countableAllowingOffset",
            ),
        ];
        for (original, expected) in cases {
            let json_v = original.to_json().expect("to_json");
            assert_eq!(json_v, serde_json::json!(expected));
            assert_eq!(
                IndexCountability::from_json(json_v).expect("from_json"),
                original
            );
            let value = original.to_object().expect("to_object");
            assert_eq!(value, platform_value::Value::Text(expected.to_string()));
            assert_eq!(
                IndexCountability::from_object(value).expect("from_object"),
                original
            );
        }
    }
}
