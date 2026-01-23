//! Address balance tracking for strategy tests.
//!
//! This module provides [`AddressesWithBalance`], a data structure that tracks platform
//! addresses along with their nonces and credit balances during test execution.
//!
//! # Design: Two-Phase Balance Tracking
//!
//! The core design uses two separate maps to handle the complexities of blockchain
//! transaction processing:
//!
//! 1. **Committed balances** (`addresses_with_balance`): Represents confirmed on-chain state.
//!    These are balances that have been finalized in previous blocks.
//!
//! 2. **Staged/in-block balances** (`addresses_in_block_with_new_balance`): Represents
//!    pending changes within the current block being constructed. These changes haven't
//!    been confirmed yet.
//!
//! This two-phase approach is necessary because:
//!
//! - **Nonce conflicts**: Each address can only have one transaction per block. If we
//!   tried to spend from the same address twice in a block, the second transaction would
//!   fail because it would use an incorrect nonce (it wouldn't know about the first
//!   transaction's nonce increment until that transaction is confirmed).
//!
//! - **Atomicity**: If block processing fails, we can roll back staged changes without
//!   affecting the committed state.
//!
//! - **Accurate balance queries**: When selecting addresses for new transactions, we need
//!   to see the "effective" balance (staged if modified this block, otherwise committed)
//!   to avoid overspending.
//!
//! # Typical Workflow
//!
//! ```text
//! 1. Start with committed balances from previous block
//! 2. For each transaction in the new block:
//!    - Query effective balance (staged overrides committed)
//!    - Deduct amount and bump nonce in staged map
//! 3. On block success: commit() merges staged → committed
//! 4. On block failure: rollback() discards staged changes
//! ```

use crate::operations::AmountRange;
use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use dpp::prelude::AddressNonce;
use rand::prelude::IteratorRandom;
use rand::Rng;
use std::collections::BTreeMap;
use std::mem;

/// Tracks platform addresses with their nonces and credit balances.
///
/// Uses a two-phase design with committed and staged (in-block) balance maps
/// to handle transaction ordering and block atomicity. See module documentation
/// for details on the design rationale.
#[derive(Clone, Debug, Default)]
pub struct AddressesWithBalance {
    /// Committed balances representing confirmed on-chain state from previous blocks.
    /// Maps each address to its (nonce, credits) pair.
    pub addresses_with_balance: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,

    /// Staged balances for the current block being constructed.
    /// These override committed balances when querying effective state.
    /// Merged into committed on [`commit()`](Self::commit) or discarded on
    /// [`rollback()`](Self::rollback).
    pub addresses_in_block_with_new_balance: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
}

impl AddressesWithBalance {
    /// Creates a new empty tracker with no addresses.
    pub fn new() -> Self {
        Self {
            addresses_with_balance: BTreeMap::new(),
            addresses_in_block_with_new_balance: BTreeMap::new(),
        }
    }

    /// Returns the total balance across all tracked addresses.
    ///
    /// This sums the effective balance of each address (staged if present,
    /// otherwise committed).
    pub fn total_balance(&self) -> Credits {
        let mut seen = std::collections::BTreeSet::new();
        let mut total: Credits = 0;

        // Sum staged balances
        for (addr, (_, balance)) in &self.addresses_in_block_with_new_balance {
            total = total.saturating_add(*balance);
            seen.insert(addr);
        }

        // Sum committed balances for addresses not in staged
        for (addr, (_, balance)) in &self.addresses_with_balance {
            if !seen.contains(addr) {
                total = total.saturating_add(*balance);
            }
        }

        total
    }

    /// Returns the total balance of committed addresses only.
    pub fn committed_balance(&self) -> Credits {
        self.addresses_with_balance
            .values()
            .map(|(_, balance)| *balance)
            .fold(0, |acc, b| acc.saturating_add(b))
    }

    /// Returns the number of committed addresses.
    pub fn committed_count(&self) -> usize {
        self.addresses_with_balance.len()
    }

    /// Returns the number of staged (in-block) addresses.
    pub fn staged_count(&self) -> usize {
        self.addresses_in_block_with_new_balance.len()
    }

    /// Returns the number of committed addresses available for spending
    /// (not already used this block).
    pub fn available_for_spending_count(&self) -> usize {
        self.addresses_with_balance
            .keys()
            .filter(|addr| !self.addresses_in_block_with_new_balance.contains_key(*addr))
            .count()
    }

    /// Returns the maximum balance among addresses available for spending.
    pub fn max_available_balance(&self) -> Credits {
        self.addresses_with_balance
            .iter()
            .filter(|(addr, _)| !self.addresses_in_block_with_new_balance.contains_key(*addr))
            .map(|(_, (_, balance))| *balance)
            .max()
            .unwrap_or(0)
    }

    /// Commits all staged (in-block) updates into the committed map.
    ///
    /// Call this after a block has been successfully processed. All addresses
    /// that were modified during block construction will have their staged
    /// nonces and balances become the new committed state.
    ///
    /// After calling this method, the staged map is empty and ready for the
    /// next block's transactions.
    pub fn commit(&mut self) {
        // Take the map, leaving an empty one behind
        let staged = mem::take(&mut self.addresses_in_block_with_new_balance);

        // Merge staged changes into the main map
        for (address, (nonce, credits)) in staged {
            self.addresses_with_balance
                .insert(address, (nonce, credits));
        }
    }

    /// Discards all staged (in-block) updates, restoring the committed state.
    ///
    /// Call this when block processing fails or is aborted. All addresses
    /// that were modified during block construction will revert to their
    /// previously committed nonces and balances.
    ///
    /// This enables atomic block construction: either all transactions in
    /// a block succeed and are committed, or none of them affect the state.
    pub fn rollback(&mut self) {
        self.addresses_in_block_with_new_balance.clear();
    }

    /// Returns the effective (nonce, credits) for an address.
    ///
    /// The "effective" state is the most up-to-date view of an address:
    /// - If the address has been modified this block (exists in staged map),
    ///   returns the staged state.
    /// - Otherwise, returns the committed state from previous blocks.
    ///
    /// This is essential for accurate balance checking when constructing
    /// multiple transactions in the same block, as each transaction needs
    /// to see the cumulative effect of prior transactions.
    pub fn get_effective(&self, address: &PlatformAddress) -> Option<&(AddressNonce, Credits)> {
        self.addresses_in_block_with_new_balance
            .get(address)
            .or_else(|| self.addresses_with_balance.get(address))
    }

    /// Returns a randomly selected address with effective balance >= `min_amount`.
    ///
    /// Considers both staged and committed balances (staged takes precedence).
    /// Returns `None` if no address meets the minimum balance requirement.
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

    /// Internal helper to select a random address and take an amount within bounds.
    ///
    /// # Address Selection
    ///
    /// Only considers addresses from the **committed** map that have NOT been
    /// used this block (i.e., not present in the staged map). This prevents
    /// nonce conflicts: each address can only have one transaction per block
    /// because subsequent transactions would need to know the updated nonce.
    ///
    /// # Amount Selection (25/50/25 Rule)
    ///
    /// When `effective_max > min_amount`:
    /// - 25% chance: take `effective_max` (the maximum available, clamped by `max_amount`)
    /// - 50% chance: take uniform random in `[min_amount, effective_max]`
    /// - 25% chance: take exactly `min_amount`
    ///
    /// # Side Effects
    ///
    /// On success, the address is added to the staged map with:
    /// - Nonce incremented by 1
    /// - Balance reduced by the taken amount
    ///
    /// # Returns
    ///
    /// `Some((address, new_nonce, taken_amount))` or `None` if no eligible address exists.
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
                "No eligible addresses (all already used this block or insufficient balance)"
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
        let (current_nonce, _nonce_source) =
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

        // Stage the new (nonce, balance)
        self.addresses_in_block_with_new_balance
            .insert(address, (new_nonce, new_balance));

        Some((address, new_nonce, taken))
    }

    /// Selects a random address and takes a random amount within the given range.
    ///
    /// Only considers addresses that:
    /// - Have not been used this block (not in staged map)
    /// - Have effective balance >= `range.start()`
    ///
    /// The amount taken follows the 25/50/25 probability distribution:
    /// - 25%: take the maximum (clamped by available balance)
    /// - 50%: take a uniform random amount between min and max
    /// - 25%: take exactly the minimum
    ///
    /// Updates the staged map with the new nonce and reduced balance.
    ///
    /// Returns `Some((address, new_nonce, taken_amount))` on success,
    /// or `None` if no eligible address exists.
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

    /// Takes from multiple addresses so that the total amount falls within the given range.
    ///
    /// This method aggregates funds from multiple addresses when a single address
    /// cannot satisfy the required amount. Useful for large transactions that
    /// exceed any individual address's balance.
    ///
    /// Each individual take uses the 25/50/25 probability distribution and
    /// updates the staged map with new nonces and reduced balances.
    ///
    /// Once the minimum is reached, there's a 50% chance to stop or continue
    /// taking more (up to the maximum).
    ///
    /// Returns `Some(map)` where map contains `address -> (new_nonce, taken_amount)`,
    /// or `None` if total available funds < `range.start()`.
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

    /// Registers a new address with an initial balance in the staged map.
    ///
    /// Use this when a transaction creates a new address as an output (e.g., a
    /// transfer to a fresh address). The address is added to the staged map
    /// with nonce 0 and the specified balance.
    ///
    /// The address will become available for spending in subsequent transactions
    /// within the same block (after staged state is visible), and will be
    /// committed to the main map when [`commit()`](Self::commit) is called.
    pub fn register_new_address(&mut self, address: PlatformAddress, balance: Credits) {
        self.addresses_in_block_with_new_balance
            .insert(address, (0, balance));
    }

    /// Registers a new address and keeps only the top N addresses by balance.
    ///
    /// Use this when creating many addresses but you only want to track the highest
    /// balance ones (e.g., for memory efficiency in stress tests).
    ///
    /// After inserting the new address, if the total count of staged addresses
    /// exceeds `keep_top_n`, the lowest balance addresses are removed until
    /// only `keep_top_n` remain.
    ///
    /// Note: This only affects the staged map (`addresses_in_block_with_new_balance`),
    /// not the committed map.
    pub fn register_new_address_keep_only_highest(
        &mut self,
        address: PlatformAddress,
        balance: Credits,
        keep_top_n: Option<u32>,
    ) {
        self.addresses_in_block_with_new_balance
            .insert(address, (0, balance));

        if let Some(keep_top_n) = keep_top_n {
            // If we exceed the limit, keep only the top N by balance
            if self.addresses_in_block_with_new_balance.len() > keep_top_n as usize {
                // Take the map and sort by balance descending
                let mut entries: Vec<_> = mem::take(&mut self.addresses_in_block_with_new_balance)
                    .into_iter()
                    .collect();
                entries.sort_by(|a, b| b.1 .1.cmp(&a.1 .1)); // Sort by balance descending
                entries.truncate(keep_top_n as usize);

                // Rebuild the map with only top N
                self.addresses_in_block_with_new_balance = entries.into_iter().collect();
            }
        }
    }

    /// Resets an address's nonce to a specific value, preserving its current balance.
    ///
    /// Use this to resynchronize local nonce tracking with on-chain state after
    /// transaction failures. When a transaction fails, the on-chain nonce doesn't
    /// increment, but our local tracking may have already bumped it.
    ///
    /// Updates both staged and committed maps if the address exists in either,
    /// ensuring consistency regardless of which map is queried.
    ///
    /// Returns `true` if the address was found and updated, `false` otherwise.
    pub fn reset_address_nonce(
        &mut self,
        address: &PlatformAddress,
        new_nonce: AddressNonce,
    ) -> bool {
        let mut found = false;

        // Update in staged map if present
        if let Some((nonce, _balance)) = self.addresses_in_block_with_new_balance.get_mut(address) {
            *nonce = new_nonce;
            found = true;
        }

        // Also update in committed map if present (to keep them in sync)
        if let Some((nonce, _balance)) = self.addresses_with_balance.get_mut(address) {
            *nonce = new_nonce;
            found = true;
        }

        found
    }

    /// Sets an address's nonce and balance to exact values.
    ///
    /// Use this to fully synchronize local state with on-chain state, typically
    /// after querying the platform for current address information.
    ///
    /// Updates the staged map, which takes precedence over committed state
    /// when querying effective balances.
    pub fn set_address_state(
        &mut self,
        address: PlatformAddress,
        nonce: AddressNonce,
        balance: Credits,
    ) {
        self.addresses_in_block_with_new_balance
            .insert(address, (nonce, balance));
    }

    /// Bulk synchronizes multiple addresses from platform query results.
    ///
    /// Applies [`set_address_state`](Self::set_address_state) for each address
    /// in the provided map, updating the staged map with current on-chain
    /// nonces and balances.
    ///
    /// Use this after fetching address states from the platform to ensure
    /// local tracking matches the actual chain state.
    pub fn sync_from_platform(
        &mut self,
        platform_states: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
    ) {
        for (address, (nonce, balance)) in platform_states {
            self.set_address_state(address, nonce, balance);
        }
    }
}
