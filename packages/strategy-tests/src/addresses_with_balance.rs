use crate::operations::AmountRange;
use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use dpp::prelude::AddressNonce;
use rand::prelude::IteratorRandom;
use rand::Rng;
use std::collections::BTreeMap;
use std::mem;

#[derive(Clone, Debug, Default)]
pub struct AddressesWithBalance {
    pub addresses_with_balance: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
    pub addresses_in_block_with_new_balance: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
}

impl AddressesWithBalance {
    pub fn new() -> Self {
        Self {
            addresses_with_balance: BTreeMap::new(),
            addresses_in_block_with_new_balance: BTreeMap::new(),
        }
    }

    /// Commit all in-block updates into the main map.
    /// After this, the in-block map is empty.
    ///
    /// TODO: explain why we have this
    pub fn commit(&mut self) {
        // Take the map, leaving an empty one behind
        let staged = mem::take(&mut self.addresses_in_block_with_new_balance);

        if !staged.is_empty() {
            tracing::debug!(
                staged_count = staged.len(),
                "commit: committing {} staged address updates",
                staged.len()
            );
        }

        // Merge staged changes into the main map
        for (address, (nonce, credits)) in staged {
            tracing::trace!(
                address = %address,
                nonce = nonce,
                credits = credits,
                "commit: moving address from staged to committed"
            );
            self.addresses_with_balance
                .insert(address, (nonce, credits));
        }
    }

    /// Roll back in-block updates (discard them).
    /// The committed balances remain unchanged.
    ///
    /// TODO: explain why we have this
    pub fn rollback(&mut self) {
        self.addresses_in_block_with_new_balance.clear();
    }

    /// Get the effective balance/nonce for an address,
    /// preferring in-block updates if present.
    pub fn get_effective(&self, address: &PlatformAddress) -> Option<&(AddressNonce, Credits)> {
        self.addresses_in_block_with_new_balance
            .get(address)
            .or_else(|| self.addresses_with_balance.get(address))
    }

    /// Returns a random address whose *effective* balance is >= min_amount.
    /// Returns None if no such address exists.
    pub fn get_rng_with_min_amount<R: Rng>(
        &self,
        min_amount: Credits,
        rng: &mut R,
    ) -> Option<PlatformAddress> {
        let mut candidates: Vec<PlatformAddress> = Vec::new();

        // 1. Check in-block updates first (these override committed balances)
        for (addr, (_nonce, credits)) in &self.addresses_in_block_with_new_balance {
            if *credits >= min_amount {
                candidates.push(*addr);
            }
        }

        // 2. Check committed balances for addresses not overridden above
        for (addr, (_nonce, credits)) in &self.addresses_with_balance {
            if !self.addresses_in_block_with_new_balance.contains_key(addr)
                && *credits >= min_amount
            {
                candidates.push(*addr);
            }
        }

        // 3. Randomly choose one
        candidates.into_iter().choose(rng)
    }

    /// Internal helper:
    /// Randomly selects an address whose *effective* balance is >= min_amount,
    /// then chooses a random amount to take in [min_amount, max_amount],
    /// clamped by the address's available balance.
    ///
    /// **IMPORTANT**: Only selects addresses that haven't been used this block yet.
    /// This prevents creating multiple transactions from the same address in a block,
    /// which would fail due to nonce conflicts (each tx needs the previous to confirm first).
    ///
    /// Probabilities (when effective_max > min_amount):
    ///   - 25%: take `effective_max`
    ///   - 50%: take uniform in [min_amount, effective_max]
    ///   - 25%: take exactly `min_amount`
    ///
    /// The chosen address's balance is reduced in `addresses_in_block_with_new_balance`
    /// and the nonce is bumped.
    ///
    /// Returns (address, new_nonce, taken_amount) or None if no eligible address exists.
    fn take_random_amount_with_bounds<R: Rng + ?Sized>(
        &mut self,
        min_amount: Credits,
        max_amount: Credits,
        rng: &mut R,
    ) -> Option<(PlatformAddress, AddressNonce, Credits)> {
        if min_amount == 0 {
            // If you want to support 0 as a min, adjust this logic;
            // for now we assume strictly positive.
            return None;
        }

        // Collect candidates: (address, effective_credits)
        // ONLY from committed addresses that haven't been used this block yet.
        // This prevents nonce conflicts from multiple txs per address per block.
        let mut candidates: Vec<(PlatformAddress, Credits)> = Vec::new();

        for (addr, (_nonce, credits)) in &self.addresses_with_balance {
            // Skip addresses already used this block (they're in the staged map)
            if self.addresses_in_block_with_new_balance.contains_key(addr) {
                continue;
            }
            if *credits >= min_amount {
                candidates.push((*addr, *credits));
            }
        }

        if candidates.is_empty() {
            tracing::debug!(
                min_amount = min_amount,
                committed_count = self.addresses_with_balance.len(),
                already_used_this_block = self.addresses_in_block_with_new_balance.len(),
                "take_random_amount_with_bounds: no eligible addresses (all already used this block or insufficient balance)"
            );
            return None;
        }

        // Choose a random candidate
        let (address, available) = candidates
            .into_iter()
            .choose(rng)
            .expect("candidates is not empty; qed");

        // Clamp upper bound by what's actually available
        let effective_max = std::cmp::min(available, max_amount);
        if effective_max < min_amount {
            return None;
        }

        // Decide how much to take
        let taken = if effective_max == min_amount {
            min_amount
        } else {
            let roll: f64 = rng.gen(); // [0.0, 1.0)
            if roll < 0.25 {
                // 25%: take the (clamped) maximum
                effective_max
            } else if roll < 0.75 {
                // 50%: take something in the middle (uniform in [min_amount, effective_max])
                let range = (effective_max - min_amount) + 1;
                let offset = rng.gen_range(0..range);
                min_amount + offset
            } else {
                // 25%: take exactly the minimum
                min_amount
            }
        };

        debug_assert!(taken >= min_amount && taken <= effective_max);
        debug_assert!(taken <= available);

        let new_balance = available - taken;

        // Get current nonce (staged overrides committed)
        let (current_nonce, nonce_source) =
            if let Some((nonce, _)) = self.addresses_in_block_with_new_balance.get(&address) {
                (*nonce, "in_block_staged")
            } else if let Some((nonce, _)) = self.addresses_with_balance.get(&address) {
                (*nonce, "committed")
            } else {
                // Shouldn't happen, but be defensive
                tracing::error!(
                    ?address,
                    "Address not found in either staged or committed maps - this shouldn't happen"
                );
                return None;
            };

        // Bump nonce – adjust if AddressNonce isn't a plain integer
        let new_nonce = current_nonce + 1;

        tracing::debug!(
            address = %address,
            current_nonce = current_nonce,
            new_nonce = new_nonce,
            nonce_source = nonce_source,
            taken_amount = taken,
            new_balance = new_balance,
            "take_random_amount_with_bounds: spending from address"
        );

        // Stage the new (nonce, balance)
        self.addresses_in_block_with_new_balance
            .insert(address, (new_nonce, new_balance));

        Some((address, new_nonce, taken))
    }

    /// Randomly selects an address whose *effective* balance is >= range.start(),
    /// then chooses a random amount to take restricted by the inclusive range,
    /// but not exceeding that address's available balance.
    ///
    /// Uses the 25/50/25 logic described above (anchored at range.start()).
    ///
    /// Returns (address, new_nonce, taken_amount).
    pub fn take_random_amount_with_range<R: Rng + ?Sized>(
        &mut self,
        range: &AmountRange,
        rng: &mut R,
    ) -> Option<(PlatformAddress, AddressNonce, Credits)> {
        let range_min = *range.start();
        let range_max = *range.end();
        if range_min == 0 {
            return None;
        }

        self.take_random_amount_with_bounds(range_min, range_max, rng)
    }

    /// Randomly takes from one or more addresses so that the **total taken**
    /// falls within the given inclusive range (clamped by total available).
    ///
    /// Each individual take uses the same 25/50/25 rule (anchored to a
    /// dynamically chosen per-step minimum), and each step bumps the nonce
    /// and updates `addresses_in_block_with_new_balance`.
    ///
    /// Returns a vector of (address, new_nonce, taken_amount) triplets, or
    /// None if total available funds < range.start().
    pub fn take_random_amounts_with_range<R: Rng + ?Sized>(
        &mut self,
        range: &AmountRange,
        rng: &mut R,
    ) -> Option<BTreeMap<PlatformAddress, (AddressNonce, Credits)>> {
        let range_min = *range.start();
        let range_max = *range.end();
        if range_min == 0 {
            return None;
        }

        // 1. Compute total available effective balance
        let mut total_available: Credits = 0;

        // in-block first (overrides)
        for (_nonce, credits) in self.addresses_in_block_with_new_balance.values() {
            total_available += *credits;
        }

        // committed, skipping overridden
        for (addr, (_nonce, credits)) in &self.addresses_with_balance {
            if !self.addresses_in_block_with_new_balance.contains_key(addr) {
                total_available += *credits;
            }
        }

        if total_available < range_min {
            return None;
        }

        // Clamp upper bound by what's actually available
        let global_max = range_max.min(total_available);

        let mut taken_total: Credits = 0;
        let mut result: BTreeMap<PlatformAddress, (AddressNonce, Credits)> = BTreeMap::new();

        loop {
            // If we've hit the absolute upper bound, we must stop.
            if taken_total >= global_max {
                break;
            }

            // Remaining room we are allowed to take
            let remaining_max = global_max - taken_total;

            // While we haven't reached the minimum yet, we must ensure we don't
            // choose too tiny amounts. Once we hit range_min, we can be looser.
            let remaining_to_min = range_min.saturating_sub(taken_total);

            // Per-step min:
            //   - at least 1
            //   - at least enough so we can eventually reach range_min
            //   - but not more than remaining_max
            let step_min = remaining_to_min.max(1).min(remaining_max);

            // Per-step max is whatever room is left
            let step_max = remaining_max;

            if step_min == 0 || step_min > step_max {
                // Can't take any more without violating bounds
                break;
            }

            // Use the internal bounded helper for this step
            let maybe = self.take_random_amount_with_bounds(step_min, step_max, rng);
            let (addr, new_nonce, taken) = match maybe {
                Some(triplet) => triplet,
                None => {
                    // No address can satisfy this step; bail out
                    break;
                }
            };

            taken_total += taken;
            result.insert(addr, (new_nonce, taken));

            // If we have at least the minimum, we *may* stop.
            if taken_total >= range_min {
                // Simple 50% chance to stop; tweak as you like.
                let roll: f64 = rng.gen();
                if roll < 0.5 {
                    break;
                }
            }
        }

        if taken_total < range_min {
            // We failed to reach the minimum; you could roll back changes here
            // if you keep a snapshot of balances, but for now just signal None.
            // NOTE: if you *need* strict atomicity, we should add snapshot/rollback.
            return None;
        }

        Some(result)
    }

    /// Registers a new address with a balance in the in-block staging area.
    /// This is used when creating new addresses as outputs.
    pub fn register_new_address(&mut self, address: PlatformAddress, balance: Credits) {
        tracing::debug!(
            address = %address,
            balance = balance,
            initial_nonce = 0,
            "register_new_address: registering new address with initial nonce 0"
        );
        self.addresses_in_block_with_new_balance
            .insert(address, (0, balance));
    }

    /// Resets an address's nonce to a specific value, keeping the current balance.
    /// This is used to sync local nonce tracking with on-chain state after transaction failures.
    ///
    /// Updates both the staged and committed maps if the address exists in either.
    /// Returns true if the address was found and updated, false otherwise.
    pub fn reset_address_nonce(
        &mut self,
        address: &PlatformAddress,
        new_nonce: AddressNonce,
    ) -> bool {
        let mut found = false;

        // Update in staged map if present
        if let Some((nonce, _balance)) = self.addresses_in_block_with_new_balance.get_mut(address) {
            let old_nonce = *nonce;
            *nonce = new_nonce;
            tracing::info!(
                address = %address,
                old_nonce = old_nonce,
                new_nonce = new_nonce,
                "reset_address_nonce: reset nonce in staged map"
            );
            found = true;
        }

        // Also update in committed map if present (to keep them in sync)
        if let Some((nonce, _balance)) = self.addresses_with_balance.get_mut(address) {
            let old_nonce = *nonce;
            *nonce = new_nonce;
            tracing::info!(
                address = %address,
                old_nonce = old_nonce,
                new_nonce = new_nonce,
                "reset_address_nonce: reset nonce in committed map"
            );
            found = true;
        }

        if !found {
            tracing::warn!(
                address = %address,
                new_nonce = new_nonce,
                "reset_address_nonce: address not found in any map"
            );
        }

        found
    }

    /// Sets an address's nonce and balance to specific values.
    /// This is used to fully sync local state with on-chain state.
    ///
    /// Updates the staged map (which takes precedence over committed).
    pub fn set_address_state(
        &mut self,
        address: PlatformAddress,
        nonce: AddressNonce,
        balance: Credits,
    ) {
        tracing::info!(
            address = %address,
            nonce = nonce,
            balance = balance,
            "set_address_state: setting address state from platform"
        );
        self.addresses_in_block_with_new_balance
            .insert(address, (nonce, balance));
    }

    /// Syncs multiple addresses' states from platform data.
    /// Takes a map of address -> (nonce, balance) from platform queries.
    pub fn sync_from_platform(
        &mut self,
        platform_states: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
    ) {
        tracing::info!(
            count = platform_states.len(),
            "sync_from_platform: syncing {} addresses from platform state",
            platform_states.len()
        );
        for (address, (nonce, balance)) in platform_states {
            self.set_address_state(address, nonce, balance);
        }
    }
}
