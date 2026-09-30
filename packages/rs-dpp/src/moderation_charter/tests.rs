use super::{
    charter_election_target, moderators_share_of, property_names, ElectedCharter,
    ModerationCharterRewardSplit, SubmittedCharter, ELECTED_CHARTER_DOCUMENT_TYPE_NAME,
    FULL_MODERATORS_SHARE, MODERATION_CHARTERS_CONTRACT_ID, SUBMITTED_CHARTER_DOCUMENT_TYPE_NAME,
};
use crate::balances::credits::MAX_CREDITS;
use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use platform_value::{Identifier, Value};

fn proposal() -> SubmittedCharter {
    SubmittedCharter {
        target_contract_id: Identifier::from([9u8; 32]),
        description: "We remove spam and doxing within a day and warn before we act.".to_string(),
        reasons: vec![Identifier::from([3u8; 32]), Identifier::from([4u8; 32])],
        moderators_share: Some(60),
        reward_split: ModerationCharterRewardSplit {
            leader: 10,
            equal: 40,
            actions: 50,
        },
    }
}

#[test]
fn should_round_trip_a_proposal_through_its_document_properties() {
    let proposal = proposal();
    let read = SubmittedCharter::from_document_properties(&proposal.to_document_properties());
    assert!(read.is_valid_with_data());
    assert_eq!(read.into_data().expect("data"), proposal);
}

#[test]
fn should_keep_an_absent_moderators_share_absent_and_read_it_as_the_full_amount() {
    let proposal = SubmittedCharter {
        moderators_share: None,
        ..proposal()
    };
    let properties = proposal.to_document_properties();
    assert!(!properties.contains_key(property_names::MODERATORS_SHARE));
    let read = SubmittedCharter::from_document_properties(&properties)
        .into_data()
        .expect("data");
    assert_eq!(read.moderators_share, None);
    assert_eq!(read.moderators_share_or_full(), FULL_MODERATORS_SHARE);
}

#[test]
fn should_read_a_zero_share_as_zero() {
    let proposal = SubmittedCharter {
        moderators_share: Some(0),
        ..proposal()
    };
    assert_eq!(proposal.moderators_share_or_full(), 0);
}

#[test]
fn should_read_a_proposal_without_reasons() {
    let proposal = SubmittedCharter {
        reasons: vec![],
        ..proposal()
    };
    let read = SubmittedCharter::from_document_properties(&proposal.to_document_properties());
    assert!(read.is_valid_with_data(), "{:?}", read.errors);
    assert_eq!(read.into_data().expect("data"), proposal);
}

#[test]
fn should_refuse_a_missing_or_mistyped_property() {
    for field in [
        property_names::TARGET_CONTRACT_ID,
        property_names::DESCRIPTION,
        property_names::REASONS,
        property_names::REWARD_SPLIT,
    ] {
        let mut properties = proposal().to_document_properties();
        properties.remove(field);
        let result = SubmittedCharter::from_document_properties(&properties);
        assert!(
            matches!(
                result.errors.first(),
                Some(ConsensusError::BasicError(
                    BasicError::ModerationCharterMalformedFieldError(e)
                )) if e.field() == field
            ),
            "{field}"
        );
    }

    let mut properties = proposal().to_document_properties();
    properties.insert(
        property_names::REASONS.to_string(),
        Value::Array(vec![Value::Text("not an id".to_string())]),
    );
    assert!(matches!(
        SubmittedCharter::from_document_properties(&properties).errors.first(),
        Some(ConsensusError::BasicError(
            BasicError::ModerationCharterMalformedFieldError(e)
        )) if e.field() == property_names::REASONS
    ));
}

#[test]
fn should_round_trip_an_elected_charter_through_its_document_properties() {
    let charter = ElectedCharter {
        target_contract_id: Identifier::from([9u8; 32]),
        submitted_charter_id: Identifier::from([7u8; 32]),
        members: vec![Identifier::from([2u8; 32]), Identifier::from([5u8; 32])],
    };
    let read = ElectedCharter::from_document_properties(&charter.to_document_properties());
    assert!(read.is_valid_with_data());
    assert_eq!(read.into_data().expect("data"), charter);

    let alone = ElectedCharter {
        members: vec![],
        ..charter
    };
    assert_eq!(
        ElectedCharter::from_document_properties(&alone.to_document_properties())
            .into_data()
            .expect("data"),
        alone
    );
}

#[test]
fn should_combine_the_elected_members_the_additions_and_the_removals() {
    let id = |byte: u8| Identifier::from([byte; 32]);
    let leader = id(1);
    let charter = ElectedCharter {
        target_contract_id: id(9),
        submitted_charter_id: id(7),
        members: vec![id(2), id(3), id(4)],
    };

    // Nothing filed since the election: the elected team
    assert_eq!(
        charter.active_members(leader, &[], &[]),
        [id(2), id(3), id(4)].into()
    );

    // An addition joins; a removal leaves, whether the member was elected or added
    assert_eq!(
        charter.active_members(leader, &[id(5), id(6)], &[id(2), id(6)]),
        [id(3), id(4), id(5)].into()
    );

    // A removal wins over an addition of the same elected member
    assert_eq!(
        charter.active_members(leader, &[id(2)], &[id(2)]),
        [id(3), id(4)].into()
    );

    // The leader is never among the members
    assert_eq!(
        charter.active_members(leader, &[leader], &[]),
        [id(2), id(3), id(4)].into()
    );
}

#[test]
fn should_read_a_charter_election_target_from_every_accepted_identifier_form() {
    let target = Identifier::new([0x7A; 32]);
    let target_of = |contract_id: &Identifier, document_type_name: &str, value: Value| {
        charter_election_target(contract_id, document_type_name, &[value])
    };

    // Every form validation accepts for an identifier property
    for value in [
        Value::Identifier([0x7A; 32]),
        Value::Bytes32([0x7A; 32]),
        Value::Bytes(vec![0x7A; 32]),
        Value::Array(vec![Value::U8(0x7A); 32]),
        Value::Array(vec![Value::U64(0x7A); 32]),
    ] {
        assert_eq!(
            target_of(
                &MODERATION_CHARTERS_CONTRACT_ID,
                ELECTED_CHARTER_DOCUMENT_TYPE_NAME,
                value
            ),
            Some(target)
        );
    }

    // The same contract written as base58 text, an array holding a value that is not a byte,
    // a short byte string, another type of the charter contract, and another contract
    for (contract_id, document_type_name, value) in [
        (
            MODERATION_CHARTERS_CONTRACT_ID,
            ELECTED_CHARTER_DOCUMENT_TYPE_NAME,
            Value::Text(bs58::encode([0x7A; 32]).into_string()),
        ),
        (
            MODERATION_CHARTERS_CONTRACT_ID,
            ELECTED_CHARTER_DOCUMENT_TYPE_NAME,
            Value::Array(
                std::iter::once(Value::U64(256))
                    .chain(std::iter::repeat_n(Value::U8(0x7A), 31))
                    .collect(),
            ),
        ),
        (
            MODERATION_CHARTERS_CONTRACT_ID,
            ELECTED_CHARTER_DOCUMENT_TYPE_NAME,
            Value::Bytes(vec![0x7A; 31]),
        ),
        (
            MODERATION_CHARTERS_CONTRACT_ID,
            SUBMITTED_CHARTER_DOCUMENT_TYPE_NAME,
            Value::Identifier([0x7A; 32]),
        ),
        (
            Identifier::new([0x01; 32]),
            ELECTED_CHARTER_DOCUMENT_TYPE_NAME,
            Value::Identifier([0x7A; 32]),
        ),
    ] {
        assert_eq!(target_of(&contract_id, document_type_name, value), None);
    }
}

#[test]
fn should_take_the_share_of_the_declared_moderators_part_rounded_down() {
    assert_eq!(moderators_share_of(100_000_000, 60), 60_000_000);
    assert_eq!(
        moderators_share_of(100_000_000, FULL_MODERATORS_SHARE),
        100_000_000
    );
    assert_eq!(moderators_share_of(100_000_000, 0), 0);
    // 33% of 1001 credits is 330.33 credits: the team charges 330, never a fraction.
    assert_eq!(moderators_share_of(1_001, 33), 330);
    assert_eq!(moderators_share_of(1, 99), 0);
    // The largest declarable amount does not overflow.
    assert_eq!(
        moderators_share_of(MAX_CREDITS, FULL_MODERATORS_SHARE),
        MAX_CREDITS
    );
    assert_eq!(moderators_share_of(MAX_CREDITS, 50), MAX_CREDITS / 2);
    // A share over 100 is refused by the schema; one that got through never raises the amount.
    assert_eq!(moderators_share_of(1_000, 150), 1_000);
}
