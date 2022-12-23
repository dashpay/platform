use crate::{
    identity::state_transition::identity_update_transition::{
        apply_identity_update_transition::apply_identity_update_transition,
        identity_update_transition::IdentityUpdateTransition,
    },
    prelude::Identity,
    state_repository::MockStateRepositoryLike,
    state_transition::StateTransitionLike,
    tests::fixtures::{get_identity_update_transition_fixture, identity_fixture},
};

struct TestData {
    state_transition: IdentityUpdateTransition,
    state_repository_mock: MockStateRepositoryLike,
}

fn setup_test() -> TestData {
    let mut state_transition = get_identity_update_transition_fixture();
    state_transition.set_revision(state_transition.get_revision() + 1);

    let identity = identity_fixture();

    let mut state_repository_mock = MockStateRepositoryLike::new();
    let identity_to_return = identity;
    state_repository_mock
        .expect_fetch_identity()
        .returning(move |_, _| Ok(Some(identity_to_return.clone())));

    TestData {
        state_transition,
        state_repository_mock,
    }
}

#[tokio::test]
async fn should_add_public_key() {
    let TestData {
        mut state_transition,
        mut state_repository_mock,
    } = setup_test();

    state_transition.set_public_keys_disabled_at(None);
    state_transition.set_public_key_ids_to_disable(vec![]);
    state_repository_mock
        .expect_store_identity_public_key_hashes()
        .returning(|_, _, _| Ok(()));
    state_repository_mock
        .expect_update_identity()
        .returning(|_, _| Ok(()));

    let result = apply_identity_update_transition(&state_repository_mock, state_transition).await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn should_use_max_identity_on_dry_run() {
    let TestData {
        mut state_transition,
        ..
    } = setup_test();

    state_transition.set_public_keys_disabled_at(None);
    state_transition.set_public_key_ids_to_disable(vec![]);
    state_transition.get_execution_context().enable_dry_run();

    let mut state_repository_mock = MockStateRepositoryLike::new();
    state_repository_mock
        .expect_fetch_identity::<Identity>()
        .returning(|_, _| Ok(None));
    state_repository_mock
        .expect_store_identity_public_key_hashes()
        .returning(|_, _, _| Ok(()));
    state_repository_mock
        .expect_update_identity()
        .returning(|_, _| Ok(()));

    let result = apply_identity_update_transition(&state_repository_mock, state_transition).await;

    assert!(result.is_ok());
}
