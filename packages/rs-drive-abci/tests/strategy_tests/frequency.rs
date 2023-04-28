use rand::distributions::{Distribution, Uniform};
use rand::prelude::StdRng;
use rand::Rng;
use std::ops::Range;

#[derive(Clone, Debug, Default)]
pub struct Frequency {
    pub times_per_block_range: Range<u16>, //insertion count when block is chosen
    pub chance_per_block: Option<f64>,     //chance of insertion if set
}

impl Frequency {
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
            let avg_times_per_block_range =
                (self.times_per_block_range.start + self.times_per_block_range.end) as f64 / 2.0;
            avg_times_per_block_range * chance_per_block
        } else {
            (self.times_per_block_range.start + self.times_per_block_range.end) as f64 / 2.0
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
}
