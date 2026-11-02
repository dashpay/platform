//! A seated team's moderators pot (protocol version 14): claimed by the leader or an active
//! member and split by the proposal's reward split, 10/40/50 in these tests (the leader's
//! share, the share split equally between the other members, and the share split by each
//! one's moderation action count, the leader's included), every part rounded down with the
//! remainder left in the pot; and settled the same way before every change of the team,
//! whatever the epoch's claim.

use super::*;
use dpp::data_contract::document_type::action_fees::ContractFeePotLastClaim;

const CONTRACT_FEES_ALREADY_CLAIMED_THIS_EPOCH: u32 = 41111;

/// A claim of the contract's moderators pot by `actor`
async fn claim_by(setup: &Setup, actor: &Actor) -> StateTransition {
    ContractFeeClaimTransition::try_from_identity_with_signer(
        &actor.identity,
        &CRITICAL_KEY_ID,
        setup.contract.id(),
        ContractFeePot::Moderators,
        actor.contract_nonce(),
        0,
        &actor.signer,
        PlatformVersion::latest(),
        None,
    )
    .await
    .expect("expected to build the claim")
}

/// A warning of the stranger by `actor`, processed: a counted action
async fn warn_stranger(team: &Team, actor: &Actor, transaction: &Transaction<'_>) {
    let setup = &team.setup;
    let warn = setup
        .moderate(actor, warn_action(setup.stranger.id(), "calm down"))
        .await;
    assert_success(&setup.process(&warn, transaction));
}

/// The contract's moderation action counts
fn counts(team: &Team, transaction: &Transaction) -> BTreeMap<Identifier, u32> {
    team.setup
        .platform
        .drive
        .fetch_contract_moderation_action_counts(
            team.setup.contract.id(),
            31,
            Some(transaction),
            PlatformVersion::latest(),
        )
        .expect("expected to read the counts")
}

/// The balances of `actors`, in order
fn balances(setup: &Setup, actors: &[&Actor], transaction: &Transaction) -> Vec<Credits> {
    actors
        .iter()
        .map(|actor| setup.balance(actor.id(), Some(transaction)))
        .collect()
}

/// What `actor`'s balance moved by since `before`
fn gained(setup: &Setup, actor: &Actor, before: Credits, transaction: &Transaction) -> i128 {
    setup.balance(actor.id(), Some(transaction)) as i128 - before as i128
}

/// What `execution` cost `actor`: its gas, less the storage refund a deletion gives it
fn net_cost(execution: &StateTransitionExecutionResult, actor: &Actor) -> i128 {
    let fees = fees_of(execution);
    let refund = fees
        .fee_refunds
        .calculate_refunds_amount_for_identity(actor.id())
        .unwrap_or_default();
    fees.total_base_fee() as i128 - refund as i128
}

/// The last claim of the contract's moderators pot
fn last_claim(team: &Team, transaction: &Transaction) -> Option<ContractFeePotLastClaim> {
    team.setup
        .platform
        .drive
        .fetch_contract_fee_pot(
            team.setup.contract.id(),
            ContractFeePot::Moderators,
            Some(transaction),
            PlatformVersion::latest(),
        )
        .expect("expected to fetch the moderators pot")
        .last_claim
}

/// Three posts fill the pot with 300_000_000 credits. The leader warns once, the elected member
/// twice and an added member four times: the action share, 150_000_000, goes 1:2:4, which
/// rounds down twice. A member claims for the team: the leader takes 10% and a seventh of the
/// action share, the two members 20% each and their sevenths, and the two credits the split
/// leaves wait in the pot. The counts start over. A joiner the leader did not add, and the
/// interim, claim nothing.
#[tokio::test]
async fn should_split_a_seated_teams_claim_by_its_reward_split_rounding_each_part_down() {
    let team = Team::new(InterimModerators::ContractOwner).await;
    let setup = &team.setup;
    let added = &team.joiners[0];
    team.award();
    // An addition while the pot is empty and nobody acted settles nothing.
    team.process_and_commit(&team.addition_of(added).await);
    for _ in 0..3 {
        team.posted_by(&setup.user).await;
    }
    let pot = 3 * MODERATORS_PART;

    let transaction = setup.platform.drive.grove.start_transaction();
    for (actor, times) in [(&team.leader, 1), (&team.member, 2), (added, 4)] {
        for _ in 0..times {
            warn_stranger(&team, actor, &transaction).await;
        }
    }
    assert_eq!(
        counts(&team, &transaction),
        BTreeMap::from([
            (team.leader.id(), 1),
            (team.member.id(), 2),
            (added.id(), 4)
        ])
    );
    assert_eq!(team.moderators_pot(&transaction), pot);

    // Only the team claims: not a joiner the leader did not add, nor the interim.
    for actor in [&team.joiners[1], &setup.owner] {
        assert_paid_with_code(
            &setup.process(&claim_by(setup, actor).await, &transaction),
            CONTRACT_FEE_CLAIM_NOT_ALLOWED,
        );
    }

    let before = balances(setup, &[&team.leader, &team.member, added], &transaction);
    let claim = claim_by(setup, &team.member).await;
    assert!(setup.check_tx(&claim).is_empty());
    let execution = setup.process(&claim, &transaction);
    assert_success(&execution);

    let leader_share = 30_000_000;
    let equal_each = 60_000_000;
    let (leader_actions, member_actions, added_actions) = (21_428_571, 42_857_142, 85_714_285);
    assert_eq!(
        gained(setup, &team.leader, before[0], &transaction),
        (leader_share + leader_actions) as i128
    );
    assert_eq!(
        gained(setup, added, before[2], &transaction),
        (equal_each + added_actions) as i128
    );
    // The claimant is paid its part and pays the claim's gas.
    assert_eq!(
        gained(setup, &team.member, before[1], &transaction),
        (equal_each + member_actions) as i128 - gas_of(&execution) as i128
    );
    assert_eq!(
        team.moderators_pot(&transaction),
        pot - leader_share - 2 * equal_each - (leader_actions + member_actions + added_actions)
    );
    assert_eq!(team.moderators_pot(&transaction), 2);
    assert!(counts(&team, &transaction).is_empty());
    assert_eq!(
        last_claim(&team, &transaction).map(|claim| claim.claimant_id),
        Some(team.member.id())
    );
}

/// Nobody acted since the last settle: the action share is split equally between the leader
/// and the member, as the equal share goes to the member alone.
#[tokio::test]
async fn should_split_the_action_share_equally_when_the_team_did_not_act() {
    let team = Team::new(InterimModerators::ContractOwner).await;
    let setup = &team.setup;
    team.award();
    for _ in 0..3 {
        team.posted_by(&setup.user).await;
    }

    let transaction = setup.platform.drive.grove.start_transaction();
    assert!(counts(&team, &transaction).is_empty());
    let before = setup.balance(team.leader.id(), Some(&transaction));
    let execution = setup.process(&claim_by(setup, &team.member).await, &transaction);
    assert_success(&execution);
    // 10% and half of 50% of 300_000_000 to the leader, 40% and the other half to the member.
    assert_eq!(
        gained(setup, &team.leader, before, &transaction),
        30_000_000 + 75_000_000
    );
    assert_eq!(team.moderators_pot(&transaction), 0);
}

/// A ban, a suspension, a warning and a document deletion by a member of the seated team count
/// for it; lifting them and a restore do not, and neither do the interim's actions before the
/// seating. Every settle resets the counts: a claim, and a change of the team.
#[tokio::test]
async fn should_count_each_signers_actions_and_reset_the_counts_at_every_settle() {
    let team = Team::new(InterimModerators::ContractOwner).await;
    let setup = &team.setup;
    let post = team.posted_by(&setup.stranger).await;

    // The interim acts before the seating: nothing counts.
    let transaction = setup.platform.drive.grove.start_transaction();
    let interim_warn = setup
        .moderate(&setup.owner, warn_action(setup.user.id(), "first"))
        .await;
    assert_success(&setup.process(&interim_warn, &transaction));
    assert!(counts(&team, &transaction).is_empty());
    setup.commit(transaction);
    team.award();

    let transaction = setup.platform.drive.grove.start_transaction();
    let stored = setup
        .stored_document(POST, post.id(), Some(&transaction))
        .expect("expected the post to be stored");
    let leader_actions = [
        ban_action(setup.user.id()),
        suspend_action(setup.stranger.id(), LATER),
        warn_action(setup.stranger.id(), "calm down"),
        delete_action(POST, post.id()),
    ];
    for (count, action) in (1..).zip(leader_actions) {
        let moderation = setup.moderate(&team.leader, action).await;
        assert_success(&setup.process(&moderation, &transaction));
        assert_eq!(
            counts(&team, &transaction),
            BTreeMap::from([(team.leader.id(), count)])
        );
    }
    let reversals = [
        unban_action(setup.user.id()),
        unsuspend_action(setup.stranger.id()),
        clear_warnings_action(setup.stranger.id()),
        restore_action(POST, setup.document_bytes(POST, &stored)),
    ];
    for action in reversals {
        let moderation = setup.moderate(&team.member, action).await;
        assert_success(&setup.process(&moderation, &transaction));
    }
    warn_stranger(&team, &team.member, &transaction).await;
    assert_eq!(
        counts(&team, &transaction),
        BTreeMap::from([(team.leader.id(), 4), (team.member.id(), 1)])
    );

    // A claim resets them.
    assert_success(&setup.process(&claim_by(setup, &team.leader).await, &transaction));
    assert!(counts(&team, &transaction).is_empty());

    // So does a change of the team.
    warn_stranger(&team, &team.member, &transaction).await;
    warn_stranger(&team, &team.member, &transaction).await;
    assert_eq!(
        counts(&team, &transaction),
        BTreeMap::from([(team.member.id(), 2)])
    );
    assert_success(&setup.process(&team.addition_of(&team.joiners[0]).await, &transaction));
    assert!(counts(&team, &transaction).is_empty());
}

/// Before the leader adds a member, the pot is paid out to the team as it was: the new member
/// gets nothing of what was earned before it came, and the counts start over. The settle is no
/// claim: the last claim stays as it was and the team still claims in the same epoch.
#[tokio::test]
async fn should_settle_the_pot_to_the_team_as_it_was_before_an_addition() {
    let team = Team::new(InterimModerators::ContractOwner).await;
    let setup = &team.setup;
    let added = &team.joiners[0];
    team.award();
    team.posted_by(&setup.user).await;

    let transaction = setup.platform.drive.grove.start_transaction();
    warn_stranger(&team, &team.member, &transaction).await;
    let before = balances(setup, &[&team.leader, &team.member, added], &transaction);
    let execution = setup.process(&team.addition_of(added).await, &transaction);
    assert_success(&execution);
    // 100_000_000: 10% to the leader; the member's 40% and the whole action share, its the
    // only count.
    assert_eq!(
        gained(setup, &team.member, before[1], &transaction),
        90_000_000
    );
    assert_eq!(
        gained(setup, &team.leader, before[0], &transaction),
        10_000_000 - gas_of(&execution) as i128
    );
    assert_eq!(gained(setup, added, before[2], &transaction), 0);
    assert_eq!(team.moderators_pot(&transaction), 0);
    assert!(counts(&team, &transaction).is_empty());
    assert_eq!(last_claim(&team, &transaction), None);
    setup.commit(transaction);

    // The added member shares what is earned from now on, and the team claims in the same
    // epoch the settle ran in.
    team.posted_by(&setup.user).await;
    let transaction = setup.platform.drive.grove.start_transaction();
    let before = setup.balance(added.id(), Some(&transaction));
    assert_success(&setup.process(&claim_by(setup, &team.leader).await, &transaction));
    // 20% and a third of 50% of 100_000_000.
    assert_eq!(
        gained(setup, added, before, &transaction),
        20_000_000 + 16_666_666
    );
}

/// Before the leader removes a member, and before it undoes a removal or an addition by
/// deleting it, the pot is paid out to the team as it was, even in an epoch the team already
/// claimed in: the removed member is paid for what it did, a member coming back gets nothing
/// of what was earned while it was away, and one whose addition is taken back is paid first.
#[tokio::test]
async fn should_settle_the_pot_before_a_removal_and_before_a_change_is_undone() {
    let team = Team::new(InterimModerators::ContractOwner).await;
    let setup = &team.setup;
    let added = &team.joiners[0];
    team.award();
    team.posted_by(&setup.user).await;

    // The team claims in this epoch; a change still settles.
    let transaction = setup.platform.drive.grove.start_transaction();
    assert_success(&setup.process(&claim_by(setup, &team.leader).await, &transaction));
    setup.commit(transaction);
    team.posted_by(&setup.user).await;

    let transaction = setup.platform.drive.grove.start_transaction();
    assert_paid_with_code(
        &setup.process(&claim_by(setup, &team.member).await, &transaction),
        CONTRACT_FEES_ALREADY_CLAIMED_THIS_EPOCH,
    );
    warn_stranger(&team, &team.member, &transaction).await;
    let before = balances(setup, &[&team.leader, &team.member], &transaction);
    let (removal, removing) = team.removed(&team.member).await;
    let execution = setup.process(&removing, &transaction);
    assert_success(&execution);
    assert_eq!(
        gained(setup, &team.member, before[1], &transaction),
        90_000_000
    );
    assert_eq!(
        gained(setup, &team.leader, before[0], &transaction),
        10_000_000 - gas_of(&execution) as i128
    );
    assert_eq!(team.moderators_pot(&transaction), 0);
    assert!(counts(&team, &transaction).is_empty());
    setup.commit(transaction);

    // Earned while the member was away: the leader, alone on the team, takes it all when the
    // removal is undone.
    team.posted_by(&setup.user).await;
    let transaction = setup.platform.drive.grove.start_transaction();
    let before = balances(setup, &[&team.leader, &team.member], &transaction);
    let execution = setup.process(
        &team
            .undoing(REMOVED_MODERATOR_DOCUMENT_TYPE_NAME, removal)
            .await,
        &transaction,
    );
    assert_success(&execution);
    // The deletion also refunds the leader the storage of its removal.
    assert_eq!(
        gained(setup, &team.leader, before[0], &transaction),
        MODERATORS_PART as i128 - net_cost(&execution, &team.leader)
    );
    assert_eq!(gained(setup, &team.member, before[1], &transaction), 0);
    setup.commit(transaction);

    // An added member whose addition is taken back is paid its part first: with nobody
    // acting, a third of 50% and half of 40% of 100_000_000.
    let (addition, adding) = team.added(added).await;
    team.process_and_commit(&adding);
    team.posted_by(&setup.user).await;
    let transaction = setup.platform.drive.grove.start_transaction();
    let before = balances(setup, &[&team.member, added], &transaction);
    assert_success(
        &setup.process(
            &team
                .undoing(ADDED_MODERATOR_DOCUMENT_TYPE_NAME, addition)
                .await,
            &transaction,
        ),
    );
    for (actor, before) in [(&team.member, before[0]), (added, before[1])] {
        assert_eq!(
            gained(setup, actor, before, &transaction),
            20_000_000 + 16_666_666
        );
    }
    // What the thirds leave waits in the pot.
    assert_eq!(team.moderators_pot(&transaction), 2);
}

/// The proof of a seated team's claim shows the pot with its last claim and the claimant's
/// balance: the contract does not say which team a seated charter pays, so neither the prover
/// nor a client verifying against the contract could name the others.
#[tokio::test]
async fn should_prove_a_seated_teams_claim_with_the_claimants_balance() {
    let team = Team::new(InterimModerators::ContractOwner).await;
    let setup = &team.setup;
    team.award();
    team.posted_by(&setup.user).await;
    let claim = claim_by(setup, &team.member).await;
    team.process_and_commit(&claim);

    let platform_version = PlatformVersion::latest();
    let proof = setup
        .platform
        .drive
        .prove_state_transition(&claim, None, platform_version)
        .expect("expected to prove the state transition")
        .into_data()
        .expect("expected proof bytes");
    let contract = setup.contract.clone();
    let (_, outcome) = Drive::verify_state_transition_was_executed_with_proof(
        &claim,
        &BlockInfo::default(),
        &proof,
        &|id| Ok((*id == contract.id()).then(|| Arc::new(contract.clone()))),
        platform_version,
    )
    .expect("expected the proof to verify");
    let StateTransitionProofResult::VerifiedContractFeeClaim(
        contract_id,
        pot,
        last_claim,
        remaining,
        balances,
    ) = outcome.into_result()
    else {
        panic!("expected a contract fee claim result");
    };
    assert_eq!(contract_id, setup.contract.id());
    assert_eq!(pot, ContractFeePot::Moderators);
    assert_eq!(last_claim.claimant_id, team.member.id());
    assert_eq!(remaining, 0);
    assert_eq!(
        balances,
        BTreeMap::from([(team.member.id(), setup.balance(team.member.id(), None))])
    );
}
