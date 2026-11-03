//! The team a seated charter acts with.

use super::readers::{elected_charter_of, member_ids};
use crate::platform::Document;
use crate::Error;
use dpp::document::DocumentV0Getters;
use dpp::moderation_charter::property_names;
use dpp::platform_value::Identifier;
use std::collections::BTreeSet;

/// The team that moderates a contract: the seated charter's leader and its active members.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModerationTeam {
    /// The seated `electedCharter` document.
    pub elected_charter_id: Identifier,
    /// The proposal the team runs on.
    pub submitted_charter_id: Identifier,
    /// The leader, the owner of the elected charter.
    pub leader_id: Identifier,
    /// The members besides the leader: the elected members and those the leader added, less
    /// those the leader removed.
    pub members: BTreeSet<Identifier>,
}

impl ModerationTeam {
    /// The team of the `electedCharter` document `elected_charter`, given the charter's
    /// `addedModerator` and `removedModerator` documents: exactly
    /// [`ElectedCharter::active_members`](dpp::moderation_charter::ElectedCharter::active_members)
    /// over their `memberId`s. A document that names another charter is refused rather than
    /// counted.
    pub fn from_documents(
        elected_charter: &Document,
        added_moderators: &[Document],
        removed_moderators: &[Document],
    ) -> Result<Self, Error> {
        let charter = elected_charter_of(elected_charter)?;
        let elected_charter_id = elected_charter.id();
        for change in added_moderators.iter().chain(removed_moderators) {
            let names = change
                .properties()
                .get(property_names::ELECTED_CHARTER_ID)
                .and_then(|value| value.to_identifier().ok());
            if names != Some(elected_charter_id) {
                return Err(Error::Generic(format!(
                    "team change {} is not a change of the charter {elected_charter_id}",
                    change.id()
                )));
            }
        }
        let leader_id = elected_charter.owner_id();
        let added = member_ids(added_moderators)?;
        let removed = member_ids(removed_moderators)?;
        Ok(Self {
            elected_charter_id,
            submitted_charter_id: charter.submitted_charter_id,
            leader_id,
            members: charter.active_members(leader_id, &added, &removed),
        })
    }

    /// Whether `identity_id` is on the team: the leader or an active member.
    pub fn contains(&self, identity_id: &Identifier) -> bool {
        *identity_id == self.leader_id || self.members.contains(identity_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::document::DocumentV0;
    use dpp::moderation_charter::ElectedCharter;
    use dpp::platform_value::Value;
    use std::collections::BTreeMap;

    fn id(byte: u8) -> Identifier {
        Identifier::from([byte; 32])
    }

    const CHARTER: u8 = 0xC1;
    const LEADER: u8 = 0x01;

    fn elected_charter(members: &[u8]) -> (Document, ElectedCharter) {
        let charter = ElectedCharter {
            target_contract_id: id(0xAA),
            submitted_charter_id: id(0xBB),
            members: members.iter().copied().map(id).collect(),
        };
        let document = DocumentV0 {
            id: id(CHARTER),
            owner_id: id(LEADER),
            properties: charter.to_document_properties(),
            ..Default::default()
        }
        .into();
        (document, charter)
    }

    fn change(document_id: u8, charter: u8, member: u8) -> Document {
        DocumentV0 {
            id: id(document_id),
            owner_id: id(LEADER),
            properties: BTreeMap::from([
                (
                    property_names::ELECTED_CHARTER_ID.to_string(),
                    Value::Identifier(id(charter).to_buffer()),
                ),
                (
                    property_names::MEMBER_ID.to_string(),
                    Value::Identifier(id(member).to_buffer()),
                ),
            ]),
            ..Default::default()
        }
        .into()
    }

    #[test]
    fn should_combine_members_additions_and_removals_as_active_members_does() {
        // Elected 2, 3, 4; added 5 and 6; 3 removed. An addition the leader deleted, or a
        // removal, is not read at all.
        let (document, charter) = elected_charter(&[2, 3, 4]);
        let added = [change(0x50, CHARTER, 5), change(0x51, CHARTER, 6)];
        let removed = [change(0x60, CHARTER, 3)];

        let team = ModerationTeam::from_documents(&document, &added, &removed).expect("reads");

        let expected = charter.active_members(id(LEADER), &[id(5), id(6)], &[id(3)]);
        assert_eq!(team.members, expected);
        assert_eq!(team.members, BTreeSet::from([id(2), id(4), id(5), id(6)]));
        assert_eq!(team.leader_id, id(LEADER));
        assert_eq!(team.elected_charter_id, id(CHARTER));
        assert_eq!(team.submitted_charter_id, id(0xBB));
        assert!(team.contains(&id(LEADER)));
        assert!(team.contains(&id(5)));
        assert!(!team.contains(&id(3)));
    }

    #[test]
    fn should_be_the_leader_and_the_elected_members_without_changes() {
        let (document, charter) = elected_charter(&[2, 3]);
        let team = ModerationTeam::from_documents(&document, &[], &[]).expect("reads");
        assert_eq!(team.members, charter.active_members(id(LEADER), &[], &[]));
        assert_eq!(team.members, BTreeSet::from([id(2), id(3)]));

        let (alone, _) = elected_charter(&[]);
        assert!(ModerationTeam::from_documents(&alone, &[], &[])
            .expect("reads")
            .members
            .is_empty());
    }

    #[test]
    fn should_refuse_a_team_change_of_another_charter() {
        let (document, _) = elected_charter(&[2]);
        assert!(ModerationTeam::from_documents(&document, &[change(0x50, 0xC2, 5)], &[]).is_err());
        assert!(ModerationTeam::from_documents(&document, &[], &[change(0x60, 0xC2, 2)]).is_err());
    }
}
