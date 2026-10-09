#[cfg(all(feature = "network-testing", not(feature = "offline-testing")))]
/// Tests that require connectivity to the server
mod online {
    use crate::fetch::{common::setup_logs, config::Config};
    use dash_sdk::dash_platform_queries::subscriptions::{Role, StateTransitionFilter};
    use dash_sdk::platform::subscriptions::SubscriptionEvent;
    use std::time::Duration;

    /// A live-only subscription is accepted and reports where it starts.
    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn should_start_a_subscription_with_a_checkpoint() {
        setup_logs();

        let cfg = Config::new();
        let sdk = cfg
            .setup_api("should_start_a_subscription_with_a_checkpoint")
            .await;
        let mut subscription = sdk
            .subscribe_to_state_transitions(
                vec![StateTransitionFilter::Identities {
                    identity_ids: vec![cfg.existing_identity_id],
                    role: Role::Any,
                }],
                None,
            )
            .await
            .expect("subscription accepted");

        let event = tokio::time::timeout(Duration::from_secs(30), subscription.next())
            .await
            .expect("first message within 30s")
            .expect("first message");
        assert!(matches!(event, SubscriptionEvent::Checkpoint { .. }));
    }
}
