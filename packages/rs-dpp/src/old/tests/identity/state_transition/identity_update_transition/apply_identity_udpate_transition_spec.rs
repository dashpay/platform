use crate::identity::IdentityPublicKey;
use crate::state_transition::state_transition_execution_context::StateTransitionExecutionContext;
use crate::{
    identity::state_transition::identity_update_transition::{
        apply_identity_update_transition::apply_identity_update_transition,
        identity_update_transition::IdentityUpdateTransition,
    },
    state_repository::MockStateRepositoryLike,
    tests::fixtures::get_identity_update_transition_fixture,
};
use mockall::predicate::{always, eq};

struct TestData {
    state_transition: IdentityUpdateTransition,
    state_repository_mock: MockStateRepositoryLike,
}

fn setup_test() -> TestData {
    let mut state_transition = get_identity_update_transition_fixture();

    state_transition.set_revision(state_transition.revision() + 1);

    let state_repository_mock = MockStateRepositoryLike::new();

    TestData {
        state_transition,
        state_repository_mock,
    }
}

#[tokio::test]
async fn should_add_and_disable_public_keys() {
    let TestData {
        state_transition,
        mut state_repository_mock,
    } = setup_test();

    let IdentityUpdateTransition {
        identity_id,
        revision,
        add_public_keys,
        disable_public_keys,
        public_keys_disabled_at,
        ..
    } = state_transition.clone();

    state_repository_mock
        .expect_update_identity_revision()
        .times(1)
        .with(eq(identity_id), eq(revision), always())
        .returning(|_, _, _| Ok(()));

    state_repository_mock
        .expect_disable_identity_keys()
        .times(1)
        .with(
            eq(identity_id),
            eq(disable_public_keys),
            eq(public_keys_disabled_at.expect("disabled_at must be set")),
            always(),
        )
        .returning(|_, _, _, _| Ok(()));

    let keys_to_add = add_public_keys
        .iter()
        .cloned()
        .map(|pk| pk.to_identity_public_key())
        .collect::<Vec<IdentityPublicKey>>();

    state_repository_mock
        .expect_add_keys_to_identity()
        .times(1)
        .with(eq(identity_id), eq(keys_to_add), always())
        .returning(|_, _, _| Ok(()));

    let execution_context = StateTransitionExecutionContext::default();

    let result = apply_identity_update_transition(
        &state_repository_mock,
        &state_transition,
        &execution_context,
    )
    .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn should_add_and_disable_public_keys_on_dry_run() {
    let TestData {
        state_transition, ..
    } = setup_test();

    let IdentityUpdateTransition {
        identity_id,
        revision,
        add_public_keys,
        disable_public_keys,
        public_keys_disabled_at,
        ..
    } = state_transition.clone();

    let mut state_repository_mock = MockStateRepositoryLike::new();

    state_repository_mock
        .expect_update_identity_revision()
        .times(1)
        .with(eq(identity_id), eq(revision), always())
        .returning(|_, _, _| Ok(()));

    state_repository_mock
        .expect_disable_identity_keys()
        .times(1)
        .with(
            eq(identity_id),
            eq(disable_public_keys),
            eq(public_keys_disabled_at.expect("disabled_at must be set")),
            always(),
        )
        .returning(|_, _, _, _| Ok(()));

    let keys_to_add = add_public_keys
        .iter()
        .cloned()
        .map(|pk| pk.to_identity_public_key())
        .collect::<Vec<IdentityPublicKey>>();

    state_repository_mock
        .expect_add_keys_to_identity()
        .times(1)
        .with(eq(identity_id), eq(keys_to_add), always())
        .returning(|_, _, _| Ok(()));

    let execution_context = StateTransitionExecutionContext::default().with_dry_run();

    let result = apply_identity_update_transition(
        &state_repository_mock,
        &state_transition,
        &execution_context,
    )
    .await;

    assert!(result.is_ok());
}
