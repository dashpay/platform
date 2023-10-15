use bincode::{Decode, Encode};
use rand::distributions::{Distribution, Uniform};
use rand::prelude::StdRng;
use rand::Rng;
use std::collections::BTreeSet;
use std::ops::Range;

#[derive(Clone, Debug, Default, PartialEq, Encode, Decode)]
pub struct Frequency {
    pub times_per_block_range: Range<u16>, //insertion count when block is chosen
    pub chance_per_block: Option<f64>,     //chance of insertion if set
}

impl Frequency {
    pub fn is_deterministic(&self) -> bool {
        self.chance_per_block.is_none()
            && (self.times_per_block_range.end - self.times_per_block_range.start == 1)
    }

    pub fn is_set(&self) -> bool {
        self.chance_per_block.is_some() || !self.times_per_block_range.is_empty()
    }

    pub fn check_hit(&self, rng: &mut StdRng) -> bool {
        match self.chance_per_block {
            None => true,
            Some(chance) => rng.gen_bool(chance),
        }
    }

    pub fn events(&self, rng: &mut StdRng) -> u16 {
        if self.times_per_block_range.is_empty() {
            0
        } else {
            rng.gen_range(self.times_per_block_range.clone())
        }
    }

    pub fn events_if_hit(&self, rng: &mut StdRng) -> u16 {
        if self.check_hit(rng) {
            self.events(rng)
        } else {
            0
        }
    }

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

    pub fn pick_in_range(&self, rng: &mut impl Rng, range: Range<u16>) -> Vec<u16> {
        if !self.is_set() || range.is_empty() {
            return vec![];
        }
        let mut picked_numbers = Vec::new();
        let chance_per_block_hit = self
            .chance_per_block
            .map_or(true, |chance| rng.gen_bool(chance));

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

    pub fn pick_from(&self, rng: &mut impl Rng, from_elements: &BTreeSet<u16>) -> Vec<u16> {
        if !self.is_set() || from_elements.is_empty() {
            return vec![];
        }

        let from_elements_vec: Vec<_> = from_elements.iter().collect();

        let mut picked_numbers = Vec::new();
        let chance_per_block_hit = self
            .chance_per_block
            .map_or(true, |chance| rng.gen_bool(chance));

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
            .map_or(true, |chance| rng.gen_bool(chance));

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
            .map_or(true, |chance| rng.gen_bool(chance));

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
