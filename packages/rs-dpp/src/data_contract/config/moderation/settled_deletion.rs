use crate::data_contract::config::moderation::ContractModerationReason;
use crate::identity::TimestampMillis;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_value::Identifier;
use serde::{Deserialize, Serialize};

/// Who must approve a moderator's deletion of a settled document: one past the window its
/// document type gives its moderators (`moderatorAbilities.deleteWithin`). The document type's
/// `moderatorAbilities.deleteSettled` (protocol version 14), on a contract whose moderators are
/// an elected team: the team's leader, and in all so many members of the seated team, the
/// leader counted among them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, DecodeUntrusted)]
pub struct SettledDeletionRule {
    /// Whether the team's leader must be among the approvals.
    pub leader: bool,
    /// How many moderators of the seated team must approve, the leader counted among them: at
    /// least 1, and at registration at most the members the declared team can hold (its
    /// leader, `SystemLimits::max_moderation_charter_elected_members` elected members and the
    /// declaration's `maxAddedModerators`). A seated team that can hold fewer, its charter
    /// electing fewer members, must have all it can hold approve.
    pub approvals: u16,
}

impl SettledDeletionRule {
    /// Whether `approvals`, identities of the seated team led by `leader_id`, each at most once,
    /// meet the rule. The team holds at most `team_capacity` members, the leader counted: a rule
    /// asking for more asks for that many, so that it can be met by the team seated.
    pub fn is_met_by(
        &self,
        approvals: &[Identifier],
        leader_id: Identifier,
        team_capacity: usize,
    ) -> bool {
        approvals.len() >= usize::from(self.approvals).min(team_capacity)
            && (!self.leader || approvals.contains(&leader_id))
    }
}

/// The approvals a contract keeps of a moderator's deletion of a settled document (protocol
/// version 14).
///
/// A document past its type's `moderatorAbilities.deleteWithin` window is settled: no moderator
/// deletes it alone. A type whose `moderatorAbilities.deleteSettled` says who must agree lets
/// the members of an elected contract's seated team delete it together: each sends the same
/// deletion, for the same reason, and the one whose approval meets the rule deletes the
/// document. This is what the contract keeps meanwhile, and after: stored under the contract, by
/// document type then document id, paid for by the moderators that approve, and never deleted.
///
/// The approvals hold for the document as it was when the first was given, and for
/// `SystemLimits::contract_settled_deletion_approval_window_ms` after it: a replace of the
/// document, or the lapse, lets the next approval start afresh in its place, for its own
/// reason. A moderator who has left the team no longer counts: every approval checks the others
/// again and keeps only those still on it, and starts afresh when none is.
#[derive(
    Debug, Clone, PartialEq, Eq, Default, Encode, Decode, DecodeUntrusted, Serialize, Deserialize,
)]
#[serde(rename_all = "camelCase")]
pub struct ContractSettledDeletion {
    /// The time of the block of the first approval, in milliseconds.
    pub proposed_at: TimestampMillis,
    /// The document's last modification when the first approval was given: its `$updatedAt`,
    /// or `$createdAt` on a type that carries no `$updatedAt`. The approvals are of the document
    /// as it was then.
    pub document_last_modified_at: TimestampMillis,
    /// Why, as the first approval gave it and every later one repeated it.
    pub reason: ContractModerationReason,
    /// The members of the seated team that approved, in the order they did, each still on the
    /// team when the last approval was given.
    pub approvals: Vec<Identifier>,
    /// The time of the block whose approval met the rule and deleted the document, in
    /// milliseconds. `None` while the approvals fall short.
    pub deleted_at: Option<TimestampMillis>,
}

impl ContractSettledDeletion {
    /// Whether the document was deleted: the approvals met the rule.
    pub fn is_deleted(&self) -> bool {
        self.deleted_at.is_some()
    }

    /// Whether a further approval adds to these at `block_time_ms`, for a document last
    /// modified at `document_last_modified_at`, approvals lapsing `approval_window_ms` after
    /// the first: they have not met the rule yet, are of the document as it is, and have not
    /// lapsed. Otherwise the next approval starts afresh in their place.
    pub fn is_open_at(
        &self,
        block_time_ms: TimestampMillis,
        document_last_modified_at: TimestampMillis,
        approval_window_ms: u64,
    ) -> bool {
        !self.is_deleted()
            && self.document_last_modified_at == document_last_modified_at
            && block_time_ms <= self.proposed_at.saturating_add(approval_window_ms)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_need_the_leader_among_the_approvals_when_the_rule_says_so() {
        let leader = Identifier::from([1; 32]);
        let member = Identifier::from([2; 32]);
        let other = Identifier::from([3; 32]);
        let leader_and_two = SettledDeletionRule {
            leader: true,
            approvals: 3,
        };
        assert!(!leader_and_two.is_met_by(&[leader, member], leader, 31));
        assert!(!leader_and_two.is_met_by(&[member, other, Identifier::from([4; 32])], leader, 31));
        assert!(leader_and_two.is_met_by(&[member, leader, other], leader, 31));

        let leader_alone = SettledDeletionRule {
            leader: true,
            approvals: 1,
        };
        assert!(leader_alone.is_met_by(&[leader], leader, 31));
        assert!(!leader_alone.is_met_by(&[member], leader, 31));

        let any_two = SettledDeletionRule {
            leader: false,
            approvals: 2,
        };
        assert!(any_two.is_met_by(&[member, other], leader, 31));
        assert!(!any_two.is_met_by(&[member], leader, 31));
    }

    #[test]
    fn should_ask_a_team_that_holds_fewer_than_the_rule_for_all_it_holds() {
        let leader = Identifier::from([1; 32]);
        let member = Identifier::from([2; 32]);
        let other = Identifier::from([3; 32]);
        let thirty_one = SettledDeletionRule {
            leader: true,
            approvals: 31,
        };
        // A team of three at most: all three meet the rule, two do not.
        assert!(thirty_one.is_met_by(&[member, leader, other], leader, 3));
        assert!(!thirty_one.is_met_by(&[member, leader], leader, 3));
        // The leader still has to be among them.
        assert!(!thirty_one.is_met_by(&[member, other], leader, 2));
    }

    #[test]
    fn should_close_on_deletion_on_a_replace_and_after_the_window() {
        let approvals = ContractSettledDeletion {
            proposed_at: 1_000,
            document_last_modified_at: 10,
            reason: ContractModerationReason::from_text("spam"),
            approvals: vec![Identifier::from([2; 32])],
            deleted_at: None,
        };
        assert!(approvals.is_open_at(1_500, 10, 500));
        // The window's last millisecond still counts
        assert!(!approvals.is_open_at(1_501, 10, 500));
        // The document was replaced since
        assert!(!approvals.is_open_at(1_200, 11, 500));
        let deleted = ContractSettledDeletion {
            deleted_at: Some(1_100),
            ..approvals
        };
        assert!(!deleted.is_open_at(1_200, 10, 500));
    }
}
