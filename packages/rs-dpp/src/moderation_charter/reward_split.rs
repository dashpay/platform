//! How a seated team's moderators pot is paid out: its proposal's `rewardSplit`.

use super::ModerationCharterRewardSplit;
use crate::balances::credits::Credits;
use crate::ProtocolError;
use platform_value::Identifier;
use std::collections::{BTreeMap, BTreeSet};

/// The percentages of a reward split are of a whole of 100.
const WHOLE: u128 = 100;

impl ModerationCharterRewardSplit {
    /// What a settle of `pot` credits pays each identity of a seated team: the leader
    /// `leader_id`, and `members`, the active members besides it. `action_counts` holds the
    /// moderation actions each one signed since the pot was last settled; a count of an
    /// identity that is not on the team is left out.
    ///
    /// - The leader share, `leader` percent of the pot, goes to the leader.
    /// - The equal share, `equal` percent, is split equally between the members; with no
    ///   member besides the leader it goes to the leader, who is then the whole team.
    /// - The action share, `actions` percent, is split between the team, the leader included,
    ///   in proportion to each one's action count; when nobody acted, it is split equally
    ///   between them.
    ///
    /// Every share and every part of one is rounded down to the credit, so a settle never pays
    /// more than the pot holds. What the rounding leaves, a few credits at most, stays in the
    /// pot for the next settle, so no identity is favoured by the order of the identity ids.
    /// An identity whose parts all round down to nothing is left out of the result.
    ///
    /// Fails when the three percentages do not add up to 100, which the charter contract's
    /// `propertyConstraints` rule `rewardSplitIsWhole` refuses at every write.
    pub fn payouts(
        &self,
        pot: Credits,
        leader_id: Identifier,
        members: &BTreeSet<Identifier>,
        action_counts: &BTreeMap<Identifier, u32>,
    ) -> Result<BTreeMap<Identifier, Credits>, ProtocolError> {
        let percentages = [self.leader, self.equal, self.actions];
        if percentages
            .iter()
            .map(|share| u128::from(*share))
            .sum::<u128>()
            != WHOLE
        {
            return Err(ProtocolError::CorruptedCodeExecution(format!(
                "a reward split of {}/{}/{} does not add up to 100",
                self.leader, self.equal, self.actions
            )));
        }
        let pot = u128::from(pot);
        let percent_of_pot = |share: u8| pot * u128::from(share) / WHOLE;

        let mut payouts: BTreeMap<Identifier, u128> = BTreeMap::new();
        let mut pay = |identity_id: Identifier, amount: u128| {
            if amount > 0 {
                *payouts.entry(identity_id).or_default() += amount;
            }
        };

        pay(leader_id, percent_of_pot(self.leader));

        let equal_share = percent_of_pot(self.equal);
        let others: Vec<Identifier> = members
            .iter()
            .filter(|member| **member != leader_id)
            .copied()
            .collect();
        if others.is_empty() {
            pay(leader_id, equal_share);
        } else {
            let each = equal_share / others.len() as u128;
            for member in &others {
                pay(*member, each);
            }
        }

        let action_share = percent_of_pot(self.actions);
        let team: Vec<Identifier> = std::iter::once(leader_id).chain(others).collect();
        let counted: Vec<(Identifier, u128)> = team
            .iter()
            .map(|identity_id| {
                (
                    *identity_id,
                    u128::from(action_counts.get(identity_id).copied().unwrap_or_default()),
                )
            })
            .collect();
        let total_actions: u128 = counted.iter().map(|(_, count)| count).sum();
        // In proportion to the counts; with no count at all, equally.
        let by_count: Option<Vec<(Identifier, u128)>> = counted
            .iter()
            .map(|(identity_id, count)| {
                (action_share * count)
                    .checked_div(total_actions)
                    .map(|amount| (*identity_id, amount))
            })
            .collect();
        match by_count {
            Some(parts) => {
                for (identity_id, amount) in parts {
                    pay(identity_id, amount);
                }
            }
            None => {
                let each = action_share / team.len() as u128;
                for identity_id in &team {
                    pay(*identity_id, each);
                }
            }
        }

        payouts
            .into_iter()
            .map(|(identity_id, amount)| {
                // Every part is a share of the pot, and the parts add up to at most the pot.
                Credits::try_from(amount)
                    .map(|amount| (identity_id, amount))
                    .map_err(|_| {
                        ProtocolError::CorruptedCodeExecution(
                            "a payout of a moderators pot exceeds the pot".to_string(),
                        )
                    })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(seed: u8) -> Identifier {
        Identifier::from([seed; 32])
    }

    fn split(leader: u8, equal: u8, actions: u8) -> ModerationCharterRewardSplit {
        ModerationCharterRewardSplit {
            leader,
            equal,
            actions,
        }
    }

    fn members(seeds: &[u8]) -> BTreeSet<Identifier> {
        seeds.iter().map(|seed| id(*seed)).collect()
    }

    fn counts(entries: &[(u8, u32)]) -> BTreeMap<Identifier, u32> {
        entries
            .iter()
            .map(|(seed, count)| (id(*seed), *count))
            .collect()
    }

    #[test]
    fn should_pay_the_leader_share_the_equal_share_and_the_action_share_by_count() {
        // 10/40/50 of 1_000: the leader takes 100, the two members 200 each of the equal 400,
        // and the 500 of the action share goes 1:3:1 between the leader and the members.
        let payouts = split(10, 40, 50)
            .payouts(
                1_000,
                id(1),
                &members(&[2, 3]),
                &counts(&[(1, 1), (2, 3), (3, 1)]),
            )
            .expect("expected a payout");
        assert_eq!(
            payouts,
            BTreeMap::from([(id(1), 100 + 100), (id(2), 200 + 300), (id(3), 200 + 100)])
        );
        assert_eq!(payouts.values().sum::<Credits>(), 1_000);
    }

    #[test]
    fn should_round_every_part_down_and_leave_the_remainder_in_the_pot() {
        // 10/40/50 of 1_003: leader 100 (100.3), equal 401 (401.2) split in three is 133 each,
        // actions 501 (501.5) split 2:1 between two members is 334 and 167. Paid: 100 + 399 +
        // 501 = 1_000; three credits stay in the pot.
        let payouts = split(10, 40, 50)
            .payouts(
                1_003,
                id(1),
                &members(&[2, 3, 4]),
                &counts(&[(2, 2), (3, 1)]),
            )
            .expect("expected a payout");
        assert_eq!(
            payouts,
            BTreeMap::from([
                (id(1), 100),
                (id(2), 133 + 334),
                (id(3), 133 + 167),
                (id(4), 133),
            ])
        );
        assert_eq!(1_003 - payouts.values().sum::<Credits>(), 3);
    }

    #[test]
    fn should_split_the_action_share_equally_when_nobody_acted() {
        // Nobody acted since the last settle: the 600 of the action share goes 200 each to the
        // leader and the two members.
        let payouts = split(0, 40, 60)
            .payouts(1_000, id(1), &members(&[2, 3]), &BTreeMap::new())
            .expect("expected a payout");
        assert_eq!(
            payouts,
            BTreeMap::from([(id(1), 200), (id(2), 200 + 200), (id(3), 200 + 200)])
        );
    }

    #[test]
    fn should_pay_a_leader_alone_the_whole_pot() {
        let payouts = split(10, 40, 50)
            .payouts(1_000, id(1), &BTreeSet::new(), &counts(&[(1, 7)]))
            .expect("expected a payout");
        assert_eq!(payouts, BTreeMap::from([(id(1), 1_000)]));
    }

    #[test]
    fn should_ignore_the_count_of_an_identity_that_is_not_on_the_team() {
        let payouts = split(0, 0, 100)
            .payouts(900, id(1), &members(&[2]), &counts(&[(2, 1), (9, 5)]))
            .expect("expected a payout");
        assert_eq!(payouts, BTreeMap::from([(id(2), 900)]));
    }

    #[test]
    fn should_leave_out_an_identity_whose_parts_round_to_nothing() {
        // 2 credits split 0/0/100 by counts 1:1:1 is 0 each: nobody is paid.
        let payouts = split(0, 0, 100)
            .payouts(
                2,
                id(1),
                &members(&[2, 3]),
                &counts(&[(1, 1), (2, 1), (3, 1)]),
            )
            .expect("expected no failure");
        assert!(payouts.is_empty());
        // A member without an action gets nothing of an action-only split.
        let payouts = split(0, 0, 100)
            .payouts(10, id(1), &members(&[2]), &counts(&[(1, 1)]))
            .expect("expected a payout");
        assert_eq!(payouts, BTreeMap::from([(id(1), 10)]));
    }

    #[test]
    fn should_never_leave_the_leader_among_the_members() {
        // A member set that names the leader counts it once, as the leader.
        let payouts = split(0, 100, 0)
            .payouts(100, id(1), &members(&[1, 2]), &BTreeMap::new())
            .expect("expected a payout");
        assert_eq!(payouts, BTreeMap::from([(id(2), 100)]));
    }

    #[test]
    fn should_pay_the_largest_pot_without_overflowing() {
        let pot = Credits::MAX;
        let payouts = split(33, 33, 34)
            .payouts(
                pot,
                id(1),
                &members(&[2]),
                &counts(&[(1, u32::MAX), (2, u32::MAX)]),
            )
            .expect("expected a payout");
        let paid = payouts
            .values()
            .try_fold(0 as Credits, |sum, amount| sum.checked_add(*amount));
        assert!(paid.is_some_and(|paid| paid <= pot));
    }

    #[test]
    fn should_refuse_a_split_that_does_not_add_up_to_one_hundred() {
        assert!(split(10, 40, 40)
            .payouts(1_000, id(1), &members(&[2]), &BTreeMap::new())
            .is_err());
        assert!(split(100, 100, 0)
            .payouts(1_000, id(1), &members(&[2]), &BTreeMap::new())
            .is_err());
    }
}
