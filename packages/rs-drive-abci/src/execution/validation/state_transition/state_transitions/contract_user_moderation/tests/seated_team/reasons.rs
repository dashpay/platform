//! The reasons a seated team acts on (protocol version 14): every ban, suspension, warning and
//! document deletion of the team names a `reason` document its proposal lists, in a block and
//! in the mempool; a proposal that lists none can take no such action. Lifting and restoring
//! carry no reason, and the interim is not bound.

use super::*;

const MODERATION_REASON_NOT_LISTED: u32 = 41203;

/// Every action a seated team is bound in, by the leader, naming `reason_document_id`, none for
/// `None`, over the stranger and a post of the stranger's
fn bound_actions(
    setup: &Setup,
    post: &Document,
    reason_document_id: Option<Identifier>,
) -> Vec<ContractUserModerationAction> {
    // The ban last: it would bar a suspension after it.
    let actions = vec![
        super::super::suspend_action(setup.stranger.id(), LATER),
        super::super::warn_action(setup.stranger.id(), "calm down"),
        super::super::delete_action(POST, post.id()),
        super::super::ban_action(setup.stranger.id()),
    ];
    match reason_document_id {
        None => actions,
        Some(reason_document_id) => actions
            .into_iter()
            .map(|action| citing(action, reason_document_id))
            .collect(),
    }
}

/// A ban, a suspension, a warning and a deletion by a member of the seated team is refused,
/// paid, in a block and in the mempool, when its reason names no reason document, or one the
/// proposal does not list, whether it exists or not; the refused action counts for nothing.
/// The listed reason passes, and so does a reversal, which names none.
#[tokio::test]
async fn should_refuse_a_seated_teams_action_that_names_no_listed_reason() {
    let team = Team::new(InterimModerators::ContractOwner).await;
    let setup = &team.setup;
    let post = team.posted_by(&setup.stranger).await;
    team.award();

    let transaction = setup.platform.drive.grove.start_transaction();
    for reason_document_id in [
        None,
        Some(UNLISTED_REASON),
        Some(Identifier::new([0xEE; 32])),
    ] {
        for action in bound_actions(setup, &post, reason_document_id) {
            let moderation = setup.moderate(&team.leader, action).await;
            assert_eq!(
                setup
                    .check_tx(&moderation)
                    .iter()
                    .map(|error| error.code())
                    .collect::<Vec<_>>(),
                vec![MODERATION_REASON_NOT_LISTED],
                "{reason_document_id:?}"
            );
            assert_paid_with_code(
                &setup.process(&moderation, &transaction),
                MODERATION_REASON_NOT_LISTED,
            );
        }
    }
    assert!(setup
        .platform
        .drive
        .fetch_contract_moderation_action_counts(
            setup.contract.id(),
            31,
            Some(&transaction),
            PlatformVersion::latest(),
        )
        .expect("expected to read the counts")
        .is_empty());

    for action in bound_actions(setup, &post, Some(LISTED_REASON)) {
        let moderation = setup.moderate(&team.member, action).await;
        assert_success(&setup.process(&moderation, &transaction));
    }
    // The reason is stored as named.
    assert_eq!(
        setup
            .status_on(
                setup.stranger.id(),
                &[ContractModerationList::Warnings],
                Some(&transaction)
            )
            .warnings
            .last()
            .and_then(|warning| warning.reason.reason_document_id),
        Some(LISTED_REASON)
    );
    for action in [
        unban_action(setup.stranger.id()),
        clear_warnings_action(setup.stranger.id()),
    ] {
        let moderation = setup.moderate(&team.leader, action).await;
        assert_success(&setup.process(&moderation, &transaction));
    }
}

/// A proposal that lists no reason is a team that can take no bound action; it may still lift
/// what the interim did.
#[tokio::test]
async fn should_let_a_team_whose_proposal_lists_no_reason_take_no_bound_action() {
    let team = Team::with_reasons(InterimModerators::ContractOwner, vec![]).await;
    let setup = &team.setup;
    let post = team.posted_by(&setup.stranger).await;
    let transaction = setup.platform.drive.grove.start_transaction();
    let interim_ban = setup
        .moderate(&setup.owner, super::super::ban_action(setup.user.id()))
        .await;
    assert_success(&setup.process(&interim_ban, &transaction));
    setup.commit(transaction);
    team.award();

    let transaction = setup.platform.drive.grove.start_transaction();
    for action in bound_actions(setup, &post, Some(LISTED_REASON)) {
        let moderation = setup.moderate(&team.leader, action).await;
        assert_paid_with_code(
            &setup.process(&moderation, &transaction),
            MODERATION_REASON_NOT_LISTED,
        );
    }
    let unban = setup
        .moderate(&team.leader, unban_action(setup.user.id()))
        .await;
    assert_success(&setup.process(&unban, &transaction));
}

/// Until a charter is seated the interim moderates on any reason, a reason document named or
/// not, listed by the contender's proposal or not, and the one it names is stored as written.
#[tokio::test]
async fn should_not_bind_the_interim_to_a_listed_reason() {
    let team = Team::new(InterimModerators::ContractOwner).await;
    let setup = &team.setup;
    let transaction = setup.platform.drive.grove.start_transaction();
    let unnamed = setup
        .moderate(&setup.owner, super::super::ban_action(setup.user.id()))
        .await;
    assert_success(&setup.process(&unnamed, &transaction));
    let unlisted = setup
        .moderate(
            &setup.owner,
            citing(
                super::super::warn_action(setup.stranger.id(), "calm down"),
                UNLISTED_REASON,
            ),
        )
        .await;
    assert_success(&setup.process(&unlisted, &transaction));
    assert_eq!(
        setup
            .status_on(
                setup.stranger.id(),
                &[ContractModerationList::Warnings],
                Some(&transaction)
            )
            .warnings
            .last()
            .and_then(|warning| warning.reason.reason_document_id),
        Some(UNLISTED_REASON)
    );
}
