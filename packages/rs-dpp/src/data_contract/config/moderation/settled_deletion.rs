use crate::data_contract::config::moderation::ContractModerationReason;
use crate::identity::TimestampMillis;
use crate::prelude::{IdentityNonce, Revision};
use crate::util::hash::hash_double;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_value::Identifier;
use serde::{Deserialize, Serialize};

/// Who must approve a moderator's deletion of a settled document: one past the window its
/// document type gives its moderators (`moderatorAbilities.deleteWithin`). The document type's
/// `moderatorAbilities.deleteSettled` (protocol version 14), on a contract whose moderators are
/// an elected team: so many members of the seated team, the leader counted among them when
/// it approves, and the leader among them only when `leader` is set; with `leader` unset, any
/// members meet it. While `approvers_predate_document` is set, a member the leader added
/// counts only for documents created after its addition.
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
    /// Whether a member the leader added (an `addedModerator`) proposes or approves the
    /// deletion of a document only when its addition was made before the document was created
    /// (`approversPredateDocument`, default `true` when `approvals` is above 1: a rule one
    /// approval meets, the leader meets alone). The leader names whom it adds, so without this
    /// it could add members who approve whatever it proposes, and take them off again once
    /// they had. The leader and the elected members always count: the election seated them,
    /// not the leader. Read from the document's `$createdAt`, which the type must then require
    /// at registration; a document without it admits no added member. What the rule needs is
    /// not lowered for it (see [`Self::approvals_needed`]), so a team whose members from before
    /// a document are too few never deletes that document once settled. Every addition comes
    /// after the seat, so a document written before the seat counts only the leader and the
    /// elected members: under a rule asking for more approvals than those, the team never
    /// deletes any document older than the seat once settled.
    pub approvers_predate_document: bool,
}

impl SettledDeletionRule {
    /// How many approvals meet the rule on a team that holds at most `team_capacity` members,
    /// the leader counted: its `approvals`, or all the team can hold when it asks for more, so
    /// that it can be met by the team seated.
    pub fn approvals_needed(&self, team_capacity: usize) -> usize {
        usize::from(self.approvals).min(team_capacity)
    }

    /// Whether `approvals`, identities of the seated team led by `leader_id`, each at most once,
    /// meet the rule on a team that holds at most `team_capacity` members
    /// ([`Self::approvals_needed`]), the leader among them when the rule says so.
    pub fn is_met_by(
        &self,
        approvals: &[Identifier],
        leader_id: Identifier,
        team_capacity: usize,
    ) -> bool {
        approvals.len() >= self.approvals_needed(team_capacity)
            && (!self.leader || approvals.contains(&leader_id))
    }

    /// Whether a member the leader added at `added_at` (its `addedModerator`'s `$createdAt`)
    /// may approve the deletion of a document created at `document_created_at`: always when
    /// `approvers_predate_document` is unset, and otherwise only when added before it. An
    /// addition in the same block as the document, at the same time, does not count: which of
    /// the two came first is not something the leader should be able to choose.
    pub fn admits_addition(
        &self,
        added_at: TimestampMillis,
        document_created_at: TimestampMillis,
    ) -> bool {
        !self.approvers_predate_document || added_at < document_created_at
    }
}

/// A proposal the seated moderation team of an elected contract votes on, kept under the
/// contract (protocol version 14): the team's counterpart of a token group action.
///
/// A member proposes, which is its own approval, and the others approve it by its id. Once
/// the approvals meet what the proposal needs, the action runs and the proposal moves from
/// the contract's active team actions to its closed ones, with the approvals that counted,
/// for good: those of members who left the team are dropped. A proposal that never gets there
/// stays active: nothing lapses, though an approval of a settled document's deletion is
/// refused once the document has changed since the proposal.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, DecodeUntrusted, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractTeamAction {
    /// The member of the seated team that proposed it, its first approval.
    pub proposer_id: Identifier,
    /// The time of the block of the proposal, in milliseconds.
    pub proposed_at: TimestampMillis,
    /// What runs once the approvals meet the rule.
    pub event: ContractTeamActionEvent,
}

/// What a [`ContractTeamAction`] does once approved.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, DecodeUntrusted, Serialize, Deserialize)]
#[serde(tag = "$type", rename_all = "camelCase")]
pub enum ContractTeamActionEvent {
    /// Deletes a settled document: one past the window its type gives its moderators
    /// (`moderatorAbilities.deleteWithin`), which the type's `moderatorAbilities.deleteSettled`
    /// lets the seated team delete together. The document goes as a moderator's
    /// `DeleteDocument` deletes it.
    #[serde(rename_all = "camelCase")]
    DeleteSettledDocument {
        /// The document's type.
        document_type_name: String,
        /// The document.
        document_id: Identifier,
        /// The document's last modification when proposed: its `$updatedAt`, or `$createdAt`
        /// on a type that carries no `$updatedAt`.
        document_last_modified_at: TimestampMillis,
        /// The document's `$revision` when proposed, `None` on a type whose documents carry
        /// none. Every change of the document moves it, a moderator's change of its fields
        /// included, which leaves `$updatedAt` alone.
        document_revision: Option<Revision>,
        /// Why, as the proposer gave it: stored with the removal record the deletion leaves.
        reason: ContractModerationReason,
    },
}

impl ContractTeamAction {
    /// The id of the proposal of a settled document's deletion: a double SHA-256 of the
    /// contract, the proposer, the proposer's nonce for the contract, the document type name,
    /// the document id and the reason. A proposer's nonce is used once, so no two proposals share
    /// an id, and the proposer's client knows it before broadcasting. The reason is part of it so
    /// that the proof of the proposal, which shows the proposer's approval under this id, is the
    /// proof of this proposal and not of another one signed with the same nonce.
    pub fn settled_deletion_action_id(
        contract_id: Identifier,
        proposer_id: Identifier,
        identity_contract_nonce: IdentityNonce,
        document_type_name: &str,
        document_id: Identifier,
        reason: &ContractModerationReason,
    ) -> Identifier {
        let mut bytes = b"action_contract_settled_deletion".to_vec();
        bytes.extend_from_slice(contract_id.as_slice());
        bytes.extend_from_slice(proposer_id.as_slice());
        bytes.extend_from_slice(&identity_contract_nonce.to_be_bytes());
        // A document type name is at most 64 bytes, so its length fits a byte and no two
        // (name, id) pairs hash alike
        bytes.push(document_type_name.len().min(u8::MAX as usize) as u8);
        bytes.extend_from_slice(document_type_name.as_bytes());
        bytes.extend_from_slice(document_id.as_slice());
        // The reason, every part of it in a shape no two reasons share: each optional part
        // behind a presence byte, the documents behind their count and each name behind its
        // length (both bounded far below a byte by the reason's validation), the text last
        // behind its length.
        match reason.code {
            Some(code) => {
                bytes.push(1);
                bytes.extend_from_slice(&code.to_be_bytes());
            }
            None => bytes.push(0),
        }
        match reason.reason_document_id {
            Some(reason_document_id) => {
                bytes.push(1);
                bytes.extend_from_slice(reason_document_id.as_slice());
            }
            None => bytes.push(0),
        }
        bytes.push(reason.documents.len().min(u8::MAX as usize) as u8);
        for document in &reason.documents {
            bytes.push(document.document_type_name.len().min(u8::MAX as usize) as u8);
            bytes.extend_from_slice(document.document_type_name.as_bytes());
            bytes.extend_from_slice(document.document_id.as_slice());
        }
        bytes.extend_from_slice(&(reason.text.len() as u64).to_be_bytes());
        bytes.extend_from_slice(reason.text.as_bytes());
        hash_double(bytes).into()
    }

    /// The document a settled deletion proposal names, as its document type name and its id.
    pub fn settled_document(&self) -> (&str, Identifier) {
        match &self.event {
            ContractTeamActionEvent::DeleteSettledDocument {
                document_type_name,
                document_id,
                ..
            } => (document_type_name.as_str(), *document_id),
        }
    }

    /// Whether the document a settled deletion proposal names is still as it was proposed:
    /// last modified at `document_last_modified_at` and at `document_revision`. An approval
    /// of a document changed since is refused, since the team would be approving the deletion
    /// of content it never saw.
    pub fn names_document_as(
        &self,
        document_last_modified_at: TimestampMillis,
        document_revision: Option<Revision>,
    ) -> bool {
        match &self.event {
            ContractTeamActionEvent::DeleteSettledDocument {
                document_last_modified_at: proposed_last_modified_at,
                document_revision: proposed_revision,
                ..
            } => {
                *proposed_last_modified_at == document_last_modified_at
                    && *proposed_revision == document_revision
            }
        }
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
            approvers_predate_document: true,
        };
        assert!(!leader_and_two.is_met_by(&[leader, member], leader, 31));
        assert!(!leader_and_two.is_met_by(&[member, other, Identifier::from([4; 32])], leader, 31));
        assert!(leader_and_two.is_met_by(&[member, leader, other], leader, 31));

        let leader_alone = SettledDeletionRule {
            leader: true,
            approvals: 1,
            approvers_predate_document: true,
        };
        assert!(leader_alone.is_met_by(&[leader], leader, 31));
        assert!(!leader_alone.is_met_by(&[member], leader, 31));

        let any_two = SettledDeletionRule {
            leader: false,
            approvals: 2,
            approvers_predate_document: true,
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
            approvers_predate_document: true,
        };
        // A team of three at most: all three meet the rule, two do not.
        assert!(thirty_one.is_met_by(&[member, leader, other], leader, 3));
        assert!(!thirty_one.is_met_by(&[member, leader], leader, 3));
        // The leader still has to be among them.
        assert!(!thirty_one.is_met_by(&[member, other], leader, 2));
    }

    #[test]
    fn should_admit_only_members_added_before_the_document_while_the_rule_says_so() {
        let predating = SettledDeletionRule {
            leader: true,
            approvals: 3,
            approvers_predate_document: true,
        };
        assert!(predating.admits_addition(999, 1_000));
        // Added in the block that created the document, or after it
        assert!(!predating.admits_addition(1_000, 1_000));
        assert!(!predating.admits_addition(1_001, 1_000));

        let any_member = SettledDeletionRule {
            approvers_predate_document: false,
            ..predating
        };
        assert!(any_member.admits_addition(1_001, 1_000));
    }

    #[test]
    fn should_refuse_a_document_changed_since_the_proposal() {
        let proposal = ContractTeamAction {
            proposer_id: Identifier::from([2; 32]),
            proposed_at: 1_000,
            event: ContractTeamActionEvent::DeleteSettledDocument {
                document_type_name: "post".to_string(),
                document_id: Identifier::from([5; 32]),
                document_last_modified_at: 10,
                document_revision: Some(2),
                reason: ContractModerationReason::from_text("spam"),
            },
        };
        assert!(proposal.names_document_as(10, Some(2)));
        // The document was replaced since
        assert!(!proposal.names_document_as(11, Some(3)));
        // A moderator changed its fields since: a new revision, the same `$updatedAt`
        assert!(!proposal.names_document_as(10, Some(3)));
        assert_eq!(
            proposal.settled_document(),
            ("post", Identifier::from([5; 32]))
        );
    }

    #[test]
    fn should_give_every_proposal_its_own_id() {
        let contract = Identifier::from([1; 32]);
        let proposer = Identifier::from([2; 32]);
        let document = Identifier::from([5; 32]);
        let spam = ContractModerationReason::from_text("spam");
        let id_of =
            |proposer: Identifier, nonce: u64, name: &str, reason: &ContractModerationReason| {
                ContractTeamAction::settled_deletion_action_id(
                    contract, proposer, nonce, name, document, reason,
                )
            };
        let id = id_of(proposer, 7, "post", &spam);
        assert_eq!(id, id_of(proposer, 7, "post", &spam));
        assert_ne!(id, id_of(proposer, 8, "post", &spam));
        assert_ne!(id, id_of(Identifier::from([3; 32]), 7, "post", &spam));
        assert_ne!(id, id_of(proposer, 7, "posts", &spam));
        // The same nonce with another reason is another proposal: its proof is not this one's
        assert_ne!(
            id,
            id_of(
                proposer,
                7,
                "post",
                &ContractModerationReason::from_text("doxxing")
            )
        );
        assert_ne!(
            id,
            id_of(
                proposer,
                7,
                "post",
                &ContractModerationReason::from_text("spam").with_reason_document(document)
            )
        );
    }
}
