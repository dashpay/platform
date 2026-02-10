//! Frequency configuration for randomized event generation in strategy tests.
//!
//! This module provides [`Frequency`], a configuration type that controls how often
//! events (such as transactions, state transitions, or other operations) occur
//! during test execution.
//!
//! # Two-Level Randomization
//!
//! Frequency uses a two-level randomization approach:
//!
//! 1. **Block-level chance** (`chance_per_block`): An optional probability (0.0 to 1.0)
//!    that determines whether any events occur in a given block. If `None`, events
//!    always occur (100% chance).
//!
//! 2. **Event count range** (`times_per_block_range`): When events do occur, this
//!    range determines how many. A random count is selected uniformly from this range.
//!
//! # Examples
//!
//! ```text
//! // Always exactly 5 events per block
//! Frequency { times_per_block_range: 5..6, chance_per_block: None }
//!
//! // 50% chance of 1-3 events per block
//! Frequency { times_per_block_range: 1..4, chance_per_block: Some(0.5) }
//!
//! // 10% chance of 10-20 events (burst pattern)
//! Frequency { times_per_block_range: 10..21, chance_per_block: Some(0.1) }
//! ```
//!
//! # Picking Elements
//!
//! The module also provides methods for randomly selecting elements from ranges
//! or collections based on the frequency configuration. This is useful for
//! selecting which specific items (e.g., validators, contracts) should be
//! affected by an operation.

use bincode::{Decode, Encode};
use rand::distributions::{Distribution, Uniform};
use rand::prelude::StdRng;
use rand::Rng;
use std::collections::BTreeSet;
use std::ops::Range;

/// Controls the frequency and count of randomized events in strategy tests.
///
/// Combines an optional per-block probability with a range for event count,
/// enabling flexible configuration of test scenarios from deterministic
/// to highly random patterns.
///
/// See module documentation for detailed examples and usage patterns.
#[derive(Clone, Debug, Default, PartialEq, Encode, Decode)]
pub struct Frequency {
    /// Range for the number of events when a block is selected.
    /// Uses standard Rust range semantics: `start..end` where `end` is exclusive.
    /// An empty range (start >= end) means no events.
    pub times_per_block_range: Range<u16>,

    /// Optional probability (0.0 to 1.0) that events occur in a given block.
    /// - `None`: Events always occur (equivalent to probability 1.0)
    /// - `Some(0.5)`: 50% chance events occur
    /// - `Some(0.0)`: Events never occur
    pub chance_per_block: Option<f64>,
}

impl Frequency {
    /// Returns `true` if this frequency produces exactly the same event count every block.
    ///
    /// A frequency is deterministic when:
    /// - No per-block chance is set (events always occur)
    /// - The range contains exactly one value (e.g., `5..6` always yields 5)
    pub fn is_deterministic(&self) -> bool {
        self.chance_per_block.is_none()
            && (self.times_per_block_range.end - self.times_per_block_range.start == 1)
    }

    /// Returns `true` if this frequency is configured to potentially produce events.
    ///
    /// A frequency is "set" if it has either:
    /// - A per-block chance configured, OR
    /// - A non-empty times-per-block range
    ///
    /// An unset frequency (default) will never produce events.
    pub fn is_set(&self) -> bool {
        self.chance_per_block.is_some() || !self.times_per_block_range.is_empty()
    }

    /// Rolls the per-block chance and returns whether events should occur.
    ///
    /// - If `chance_per_block` is `None`, always returns `true`
    /// - Otherwise, returns `true` with the configured probability
    pub fn check_hit(&self, rng: &mut StdRng) -> bool {
        match self.chance_per_block {
            None => true,
            Some(chance) => rng.gen_bool(chance),
        }
    }

    /// Returns a random event count from the configured range.
    ///
    /// Does NOT check the per-block chance first. Use [`events_if_hit`](Self::events_if_hit)
    /// if you want the full two-level randomization.
    ///
    /// Returns 0 if the range is empty.
    pub fn events(&self, rng: &mut StdRng) -> u16 {
        if self.times_per_block_range.is_empty() {
            0
        } else {
            rng.gen_range(self.times_per_block_range.clone())
        }
    }

    /// Returns the event count for this block using full two-level randomization.
    ///
    /// First checks if events should occur (per-block chance), then if so,
    /// returns a random count from the range. Returns 0 if the chance check fails.
    ///
    /// This is the primary method for determining how many events to generate.
    pub fn events_if_hit(&self, rng: &mut StdRng) -> u16 {
        if self.check_hit(rng) {
            self.events(rng)
        } else {
            0
        }
    }

    /// Calculates the expected (average) number of events per block.
    ///
    /// Takes into account both the per-block chance and the range midpoint.
    /// Useful for estimating total events over many blocks or for test planning.
    ///
    /// Formula: `average_range_value * chance_per_block`
    pub fn average_event_count(&self) -> f64 {
        if let Some(chance_per_block) = self.chance_per_block {
            let avg_times_per_block_range = if self.times_per_block_range.start
                == self.times_per_block_range.end
            {
                0.0
            } else {
                (self.times_per_block_range.start as f64 + self.times_per_block_range.end as f64
                    - 1.0)
                    / 2.0
            };
            avg_times_per_block_range * chance_per_block
        } else if self.times_per_block_range.start == self.times_per_block_range.end {
            0.0
        } else {
            (self.times_per_block_range.start as f64 + self.times_per_block_range.end as f64 - 1.0)
                / 2.0
        }
    }

    /// Returns the maximum possible event count (the range's upper bound).
    ///
    /// Note: Since ranges are exclusive, this returns the exclusive upper bound,
    /// meaning the actual maximum count is `end - 1`.
    pub fn max_event_count(&self) -> u16 {
        self.times_per_block_range.end
    }

    /// Picks random values from a numeric range based on this frequency.
    ///
    /// Uses full two-level randomization:
    /// 1. Checks per-block chance (if set)
    /// 2. If hit, determines how many values to pick from the times-per-block range
    /// 3. Picks that many random values uniformly from `range`
    ///
    /// Values may be picked multiple times (sampling with replacement).
    ///
    /// Returns an empty vector if:
    /// - The frequency is not set
    /// - The input range is empty
    /// - The per-block chance check fails
    pub fn pick_in_range(&self, rng: &mut impl Rng, range: Range<u16>) -> Vec<u16> {
        if !self.is_set() || range.is_empty() {
            return vec![];
        }
        let mut picked_numbers = Vec::new();
        let chance_per_block_hit = self
            .chance_per_block
            .is_none_or(|chance| rng.gen_bool(chance));

        if chance_per_block_hit {
            let times_per_block_dist = Uniform::from(self.times_per_block_range.clone());
            let num_elements_to_pick = times_per_block_dist.sample(rng);
            let range_dist = Uniform::from(range);

            picked_numbers = rng
                .sample_iter(&range_dist)
                .take(num_elements_to_pick as usize)
                .collect();
        }

        picked_numbers
    }

    /// Picks random elements from a set based on this frequency.
    ///
    /// Similar to [`pick_in_range`](Self::pick_in_range), but selects from an
    /// explicit set of values rather than a numeric range.
    ///
    /// Uses full two-level randomization and sampling with replacement
    /// (the same element may be picked multiple times).
    ///
    /// Returns an empty vector if the frequency is not set, the set is empty,
    /// or the per-block chance check fails.
    pub fn pick_from(&self, rng: &mut impl Rng, from_elements: &BTreeSet<u16>) -> Vec<u16> {
        if !self.is_set() || from_elements.is_empty() {
            return vec![];
        }

        let from_elements_vec: Vec<_> = from_elements.iter().collect();

        let mut picked_numbers = Vec::new();
        let chance_per_block_hit = self
            .chance_per_block
            .is_none_or(|chance| rng.gen_bool(chance));

        if chance_per_block_hit {
            let times_per_block_dist = Uniform::from(self.times_per_block_range.clone());
            let num_elements_to_pick = times_per_block_dist.sample(rng);
            let banned_dist = Uniform::from(0..from_elements.len());

            picked_numbers = rng
                .sample_iter(&banned_dist)
                .take(num_elements_to_pick as usize)
                .map(|index| *from_elements_vec[index])
                .collect();
        }

        picked_numbers
    }

    /// Picks random elements from a set, excluding specified values.
    ///
    /// Like [`pick_from`](Self::pick_from), but first filters out any elements
    /// present in `exclude_set`. Useful when certain values have already been
    /// used or are otherwise unavailable.
    ///
    /// Uses full two-level randomization and sampling with replacement.
    ///
    /// Returns an empty vector if:
    /// - The frequency is not set
    /// - The collection is empty
    /// - All elements are excluded
    /// - The per-block chance check fails
    pub fn pick_from_not_in<R: Rng>(
        &self,
        rng: &mut R,
        collection: &BTreeSet<u16>,
        exclude_set: &BTreeSet<u16>,
    ) -> Vec<u16> {
        if !self.is_set() || collection.is_empty() {
            return vec![];
        }

        let available_choices: Vec<u16> = collection
            .iter()
            .cloned()
            .filter(|x| !exclude_set.contains(x))
            .collect();

        if available_choices.is_empty() {
            return vec![];
        }

        let mut picked_numbers = Vec::new();
        let chance_per_block_hit = self
            .chance_per_block
            .is_none_or(|chance| rng.gen_bool(chance));

        if chance_per_block_hit {
            let times_per_block_dist = Uniform::from(self.times_per_block_range.clone());
            let num_elements_to_pick = times_per_block_dist.sample(rng);
            let choice_dist = Uniform::from(0..available_choices.len());

            picked_numbers = rng
                .sample_iter(&choice_dist)
                .take(num_elements_to_pick as usize)
                .map(|index| available_choices[index])
                .collect();
        }

        picked_numbers
    }

    /// Picks random values from a numeric range, excluding specified values.
    ///
    /// Like [`pick_in_range`](Self::pick_in_range), but first filters out any
    /// values present in `exclude_set`. Useful when certain indices or IDs
    /// have already been used or are otherwise unavailable.
    ///
    /// Uses full two-level randomization and sampling with replacement
    /// from the remaining available values.
    ///
    /// Returns an empty vector if:
    /// - The frequency is not set
    /// - The range is empty
    /// - All values in the range are excluded
    /// - The per-block chance check fails
    pub fn pick_in_range_not_from<R: Rng>(
        &self,
        rng: &mut R,
        range: Range<u16>,
        exclude_set: &BTreeSet<u16>,
    ) -> Vec<u16> {
        if !self.is_set() || range.is_empty() {
            return vec![];
        }

        let available_choices: Vec<u16> = range
            .into_iter()
            .filter(|x| !exclude_set.contains(x))
            .collect();

        if available_choices.is_empty() {
            return vec![];
        }

        let mut picked_numbers = Vec::new();
        let chance_per_block_hit = self
            .chance_per_block
            .is_none_or(|chance| rng.gen_bool(chance));

        if chance_per_block_hit {
            let times_per_block_dist = Uniform::from(self.times_per_block_range.clone());
            let num_elements_to_pick = times_per_block_dist.sample(rng);
            let choice_dist = Uniform::from(0..available_choices.len());

            picked_numbers = rng
                .sample_iter(&choice_dist)
                .take(num_elements_to_pick as usize)
                .map(|index| available_choices[index])
                .collect();
        }

        picked_numbers
    }
}
