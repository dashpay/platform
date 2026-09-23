use super::{
    property_names, validate_submitted_charter, ElectedCharter, ModerationCharterRewardSplit,
    SubmittedCharter, FULL_MODERATORS_SHARE,
};
use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use platform_value::{Identifier, Value};
use platform_version::version::PlatformVersion;

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

fn first_basic_error(
    result: &crate::validation::ConsensusValidationResult<SubmittedCharter>,
) -> &BasicError {
    match result.errors.first() {
        Some(ConsensusError::BasicError(error)) => error,
        other => panic!("expected a basic error, got {other:?}"),
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
fn should_accept_a_proposal_without_reasons() {
    let proposal = SubmittedCharter {
        reasons: vec![],
        ..proposal()
    };
    let result = validate_submitted_charter(
        &proposal.to_document_properties(),
        PlatformVersion::latest(),
    )
    .expect("validation executes");
    assert!(result.is_valid_with_data());
}

#[test]
fn should_accept_a_valid_proposal() {
    let result = validate_submitted_charter(
        &proposal().to_document_properties(),
        PlatformVersion::latest(),
    )
    .expect("validation executes");
    assert!(result.is_valid_with_data(), "{:?}", result.errors);
}

#[test]
fn should_refuse_a_reward_split_that_does_not_sum_to_one_hundred() {
    for (leader, equal, actions) in [(10, 40, 40), (50, 50, 1), (0, 0, 0)] {
        let proposal = SubmittedCharter {
            reward_split: ModerationCharterRewardSplit {
                leader,
                equal,
                actions,
            },
            ..proposal()
        };
        let result = validate_submitted_charter(
            &proposal.to_document_properties(),
            PlatformVersion::latest(),
        )
        .expect("validation executes");
        assert!(matches!(
            first_basic_error(&result),
            BasicError::ModerationCharterRewardSplitNotOneHundredError(e)
                if (e.leader(), e.equal(), e.actions()) == (leader, equal, actions)
        ));
    }
}

#[test]
fn should_refuse_a_description_over_the_byte_limit() {
    let platform_version = PlatformVersion::latest();
    let limit = platform_version
        .system_limits
        .max_moderation_charter_description_length as usize;

    let at_limit = SubmittedCharter {
        description: "a".repeat(limit),
        ..proposal()
    };
    assert!(
        validate_submitted_charter(&at_limit.to_document_properties(), platform_version)
            .expect("validation executes")
            .is_valid_with_data()
    );

    // Fewer characters than the schema's maxLength, more bytes than the consensus cap.
    let over = SubmittedCharter {
        description: "é".repeat(limit / 2 + 1),
        ..proposal()
    };
    let result = validate_submitted_charter(&over.to_document_properties(), platform_version)
        .expect("validation executes");
    assert!(matches!(
        first_basic_error(&result),
        BasicError::ModerationCharterDescriptionTooLongError(e)
            if e.length() == (limit + 2) as u64
    ));
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
fn should_refuse_to_validate_below_protocol_version_14() {
    let platform_version = PlatformVersion::get(13).expect("version 13");
    assert!(proposal().validate(platform_version).is_err());
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
