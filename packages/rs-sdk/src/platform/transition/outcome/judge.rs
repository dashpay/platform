//! The rules that turn proven reads of a transition's inputs into a verdict.
//!
//! Kept apart from the reads so every rule is tested on its own. Each read carries the height
//! its proof was signed at, and the rules lean on three facts about Platform state:
//!
//! - a consumed input stays consumed: a spent nullifier is never removed, and identity,
//!   identity-contract and address nonces only grow;
//! - a transition consumes all of its inputs or none of them, whether it executes or fails
//!   with a paid error;
//! - an anchor Platform pruned never returns to its anchor set, since the commitment tree only
//!   grows and so never has the same root twice.
//!
//! So a free input read at height `h` proves the transition had not executed by `h`, and a
//! consumed one read at `h` proves it cannot execute after `h`. No read says which transition
//! took a consumed input, so none proves this one executed; a read of what it would have
//! written, at or above that `h`, does.

/// One proven read of one input: whether it is consumed, and the height the proof was signed at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct InputRead {
    /// The input is taken: the nullifier is spent, or the nonce is used or out of reach.
    pub consumed: bool,
    /// The height of the block whose state the proof shows.
    pub height: u64,
}

/// What a set of input reads proves, before anchors are considered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Consumption {
    /// Every input is consumed.
    All {
        /// The lowest height among the reads: the transition cannot execute after it.
        consumed_by: u64,
    },
    /// An input was consumed while another was still free, as of a read at or after the read
    /// that showed it consumed. The transition consumes all of its inputs at once, so it did not
    /// execute, and the consumed input keeps it from ever executing.
    Partial,
    /// Some inputs are consumed and others free, but every free one was read at a lower height
    /// than every consumed one. The transition may have executed between the reads; it cannot
    /// execute any more.
    Unordered {
        /// The lowest height among the reads that showed an input consumed: the transition
        /// cannot execute after it.
        consumed_by: u64,
    },
    /// No input is consumed: the transition had not executed by `free_at`, the highest read.
    None {
        /// The highest height among the reads.
        free_at: u64,
    },
}

/// Judge a set of input reads. `reads` must not be empty: a transition with no input to read
/// proves nothing, and the caller refuses it before reading.
pub(super) fn consumption(reads: &[InputRead]) -> Consumption {
    let lowest_consumed = reads
        .iter()
        .filter(|read| read.consumed)
        .map(|read| read.height)
        .min();
    let highest_free = reads
        .iter()
        .filter(|read| !read.consumed)
        .map(|read| read.height)
        .max();
    match (lowest_consumed, highest_free) {
        (Some(consumed_by), None) => Consumption::All { consumed_by },
        (Some(consumed), Some(free)) if consumed <= free => Consumption::Partial,
        (Some(consumed_by), Some(_)) => Consumption::Unordered { consumed_by },
        (None, free) => Consumption::None {
            free_at: free.unwrap_or_default(),
        },
    }
}

/// What the reads prove about a transition's inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum InputsVerdict {
    /// Every input is consumed, or the inputs are consumed in an order the reads cannot place:
    /// it can no longer execute, and the reads do not show whether it did.
    ConsumedUnattributed {
        /// The lowest height of a read that showed an input consumed: the transition cannot
        /// execute after it.
        consumed_by: u64,
    },
    /// An input another transition took: it never executed and never will.
    InputUsed,
    /// A shielded spend's anchor, proven recorded by an earlier height, is gone from its pool,
    /// and its nullifiers were unspent at or after that: it never executed and never will.
    AnchorExpired,
    /// Not executed by `checked_at`, and it may still execute.
    NotYet {
        /// The highest height among the reads that showed every input free.
        checked_at: u64,
    },
    /// An anchor proven recorded is gone, but every nullifier read is older than the anchor
    /// read, so the reads cannot say whether the spend landed before the anchor was pruned.
    /// Read the nullifiers again.
    Unordered,
}

/// One shielded bundle's anchor, as its pool's anchor set showed it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct AnchorRead {
    /// The pool's anchor set holds the bundle's anchor.
    pub recorded: bool,
    /// The height the anchor set was proven at.
    pub height: u64,
}

/// Judge a pool-paid shielded spend from its nullifier reads (every bundle's, in every pool)
/// and its anchor reads (one per bundle). `anchors_recorded_by` is the caller's evidence of a
/// height by which every anchor was recorded.
///
/// `nullifiers` must not be empty.
pub(super) fn judge_shielded(
    nullifiers: &[InputRead],
    anchors: &[AnchorRead],
    anchors_recorded_by: Option<u64>,
) -> InputsVerdict {
    match consumption(nullifiers) {
        // Every note it spends is spent, but other transitions can spend the same notes, all
        // together or each on its own, so the reads do not say this one executed.
        Consumption::All { consumed_by } | Consumption::Unordered { consumed_by } => {
            InputsVerdict::ConsumedUnattributed { consumed_by }
        }
        Consumption::Partial => InputsVerdict::InputUsed,
        Consumption::None { free_at } => {
            // A missing anchor proves pruned only from a read at or above a height by which the
            // anchor was recorded: a node below that height may simply not have recorded it yet,
            // and then the spend can still execute.
            let lowest_pruned = anchors
                .iter()
                .filter(|anchor| {
                    !anchor.recorded
                        && anchors_recorded_by.is_some_and(|recorded| recorded <= anchor.height)
                })
                .map(|anchor| anchor.height)
                .min();
            match lowest_pruned {
                None => InputsVerdict::NotYet {
                    checked_at: free_at,
                },
                // The anchor was pruned by the height of the read that misses it, so the spend
                // could only have executed below that height. Its nullifiers unspent at or above
                // it prove it did not. A nullifier read below every such read leaves a window
                // where it could have.
                Some(pruned_by) if pruned_by <= free_at => InputsVerdict::AnchorExpired,
                Some(_) => InputsVerdict::Unordered,
            }
        }
    }
}

/// Judge a transition whose inputs are nonces: identity, identity-contract or address nonces.
///
/// Executing consumes every nonce, and so does a paid failure, so a consumed nonce does not say
/// whether this transition took it. `reads` must not be empty.
pub(super) fn judge_nonces(reads: &[InputRead]) -> InputsVerdict {
    match consumption(reads) {
        Consumption::All { consumed_by } | Consumption::Unordered { consumed_by } => {
            InputsVerdict::ConsumedUnattributed { consumed_by }
        }
        Consumption::Partial => InputsVerdict::InputUsed,
        Consumption::None { free_at } => InputsVerdict::NotYet {
            checked_at: free_at,
        },
    }
}

/// The last height at which a spend built on a credit pool anchor can execute, under the
/// retention rules `retention_blocks` and `pruning_interval`. A token pool prunes only when a
/// block writes to it, so this schedule does not apply to its anchors.
///
/// Pruning runs at the end of each block whose height is a multiple of `pruning_interval`,
/// after the block's own transitions ran, and removes the anchors recorded below that height
/// minus `retention_blocks`, except the pool's newest. So the anchor is gone after the first
/// pruning block that is both far enough past its recording and at which it is no longer the
/// newest; until then a spend built on it can still execute.
///
/// - The anchor was recorded at or below `anchor_read_height`, the height of the read that
///   found it, so the first pruning block above `anchor_read_height + retention_blocks` is
///   far enough past it.
/// - `newer_anchor_proven_at` is the height of a read, at or above `anchor_read_height`, that
///   shows a newer anchor. The anchor may have stayed the newest up to that height (the newer
///   one recorded at the end of that very block), so only a pruning block above it is known
///   to find the newer one. `None` when no read shows one: pruning keeps the newest anchor,
///   and nothing bounds the spend.
///
/// Both heights stand in for later or equal real ones, so the deadline is never earlier than
/// the real one.
pub(super) fn anchor_deadline(
    anchor_read_height: u64,
    newer_anchor_proven_at: Option<u64>,
    retention_blocks: u64,
    pruning_interval: u64,
) -> Option<u64> {
    let newer_anchor_proven_at = newer_anchor_proven_at?;
    if pruning_interval == 0 {
        return None;
    }
    let past = anchor_read_height
        .checked_add(retention_blocks)?
        .max(newer_anchor_proven_at);
    (past / pruning_interval)
        .checked_add(1)?
        .checked_mul(pruning_interval)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn free(height: u64) -> InputRead {
        InputRead {
            consumed: false,
            height,
        }
    }

    fn consumed(height: u64) -> InputRead {
        InputRead {
            consumed: true,
            height,
        }
    }

    fn recorded(height: u64) -> AnchorRead {
        AnchorRead {
            recorded: true,
            height,
        }
    }

    fn pruned(height: u64) -> AnchorRead {
        AnchorRead {
            recorded: false,
            height,
        }
    }

    #[test]
    fn should_report_all_consumed_when_every_read_shows_its_input_taken() {
        assert_eq!(
            consumption(&[consumed(10), consumed(12)]),
            Consumption::All { consumed_by: 10 }
        );
    }

    #[test]
    fn should_report_none_consumed_as_of_the_highest_read() {
        assert_eq!(
            consumption(&[free(10), free(14), free(12)]),
            Consumption::None { free_at: 14 }
        );
    }

    #[test]
    fn should_prove_another_transition_took_an_input_from_one_read() {
        // One read shows one input taken and another free: this transition, which takes both
        // at once, did not take the first.
        assert_eq!(consumption(&[consumed(10), free(10)]), Consumption::Partial);
    }

    #[test]
    fn should_prove_another_transition_took_an_input_when_the_free_read_is_later() {
        assert_eq!(consumption(&[consumed(10), free(11)]), Consumption::Partial);
    }

    #[test]
    fn should_not_attribute_a_consumed_input_read_after_the_free_one() {
        // Free at 10, consumed at 11: this transition may have taken both at 11.
        assert_eq!(
            consumption(&[free(10), consumed(11)]),
            Consumption::Unordered { consumed_by: 11 }
        );
    }

    #[test]
    fn should_not_attribute_a_spend_whose_nullifiers_are_all_spent() {
        // Other transitions may have spent the same notes, together or one by one.
        assert_eq!(
            judge_shielded(&[consumed(20), consumed(20)], &[pruned(30)], Some(10)),
            InputsVerdict::ConsumedUnattributed { consumed_by: 20 }
        );
    }

    #[test]
    fn should_report_input_used_when_some_nullifiers_are_spent_and_others_not() {
        assert_eq!(
            judge_shielded(&[consumed(20), free(20)], &[recorded(19)], None),
            InputsVerdict::InputUsed
        );
    }

    #[test]
    fn should_not_attribute_nullifiers_spent_after_a_free_read_in_another_pool() {
        assert_eq!(
            judge_shielded(
                &[free(20), consumed(21)],
                &[recorded(19), recorded(19)],
                None
            ),
            InputsVerdict::ConsumedUnattributed { consumed_by: 21 }
        );
    }

    #[test]
    fn should_report_anchor_expired_when_unspent_nullifiers_are_read_after_the_anchor() {
        assert_eq!(
            judge_shielded(&[free(30), free(30)], &[pruned(30)], Some(20)),
            InputsVerdict::AnchorExpired
        );
        assert_eq!(
            judge_shielded(&[free(31)], &[pruned(30)], Some(30)),
            InputsVerdict::AnchorExpired
        );
    }

    #[test]
    fn should_not_report_anchor_expired_without_evidence_the_anchor_was_recorded() {
        // The node answering may be behind the block that records the wallet's anchor.
        assert_eq!(
            judge_shielded(&[free(30)], &[pruned(30)], None),
            InputsVerdict::NotYet { checked_at: 30 }
        );
    }

    #[test]
    fn should_not_report_anchor_expired_from_a_node_below_the_recording_height() {
        // Recorded by 35, read at 30: the node has not reached the block that records it.
        assert_eq!(
            judge_shielded(&[free(30)], &[pruned(30)], Some(35)),
            InputsVerdict::NotYet { checked_at: 30 }
        );
    }

    #[test]
    fn should_not_report_anchor_expired_from_a_nullifier_read_older_than_the_anchor_read() {
        // The spend may have executed at 29, before the anchor was pruned by 30, and a node
        // answering the nullifiers from 28 would not show it.
        assert_eq!(
            judge_shielded(&[free(28)], &[pruned(30)], Some(20)),
            InputsVerdict::Unordered
        );
    }

    #[test]
    fn should_report_anchor_expired_when_one_of_two_pools_lost_its_anchor() {
        // A token pool transition needs both bundles' anchors; losing either one ends it.
        assert_eq!(
            judge_shielded(&[free(40), free(41)], &[recorded(39), pruned(40)], Some(39)),
            InputsVerdict::AnchorExpired
        );
    }

    #[test]
    fn should_report_not_yet_while_every_anchor_is_kept_and_no_nullifier_spent() {
        assert_eq!(
            judge_shielded(&[free(50), free(51)], &[recorded(49)], None),
            InputsVerdict::NotYet { checked_at: 51 }
        );
    }

    #[test]
    fn should_not_attribute_nonces_that_are_all_consumed() {
        // A paid failure consumes the nonce too, and so does any other transition signed with it.
        assert_eq!(
            judge_nonces(&[consumed(5)]),
            InputsVerdict::ConsumedUnattributed { consumed_by: 5 }
        );
    }

    #[test]
    fn should_report_input_used_when_one_address_nonce_moved_and_another_did_not() {
        assert_eq!(
            judge_nonces(&[consumed(5), free(5)]),
            InputsVerdict::InputUsed
        );
    }

    #[test]
    fn should_report_not_yet_while_every_nonce_is_free() {
        assert_eq!(
            judge_nonces(&[free(5), free(7)]),
            InputsVerdict::NotYet { checked_at: 7 }
        );
    }

    #[test]
    fn should_place_the_deadline_at_the_first_pruning_block_past_retention() {
        // Read at 1050 with 1000 blocks kept and pruning every 100: the anchor was recorded at
        // or below 1050, so the prune at the end of block 2100 removes it at the latest, and a
        // spend can still execute in block 2100 itself.
        assert_eq!(anchor_deadline(1050, Some(1050), 1000, 100), Some(2100));
        // Recorded at 1100 at the latest: pruning at 2100 removes only anchors below 1100, so
        // the next prune, at 2200, is the one that removes it.
        assert_eq!(anchor_deadline(1100, Some(1100), 1000, 100), Some(2200));
    }

    #[test]
    fn should_not_place_the_deadline_before_a_newer_anchor_is_proven() {
        // Found at 200 and still the newest, so kept by every prune, until a newer anchor is
        // proven at 1501: the prune at 1600 is the first known to find the newer one.
        assert_eq!(anchor_deadline(200, Some(1501), 1000, 100), Some(1600));
        // The newer anchor may have been recorded at the end of block 1300 itself, so the
        // prune at 1300 may still have kept this one as the newest.
        assert_eq!(anchor_deadline(200, Some(1300), 1000, 100), Some(1400));
        // Proven early enough, retention alone decides.
        assert_eq!(anchor_deadline(200, Some(1199), 1000, 100), Some(1300));
    }

    #[test]
    fn should_give_no_deadline_to_a_pool_s_newest_anchor() {
        // Pruning keeps the newest anchor while the pool is idle, for as long as it stays idle.
        assert_eq!(anchor_deadline(1050, None, 1000, 100), None);
    }

    #[test]
    fn should_give_no_deadline_when_nothing_is_pruned_or_the_height_overflows() {
        assert_eq!(anchor_deadline(1050, Some(1050), 1000, 0), None);
        assert_eq!(anchor_deadline(u64::MAX, Some(u64::MAX), 1000, 100), None);
        assert_eq!(anchor_deadline(1050, Some(u64::MAX), 1000, 100), None);
    }
}
