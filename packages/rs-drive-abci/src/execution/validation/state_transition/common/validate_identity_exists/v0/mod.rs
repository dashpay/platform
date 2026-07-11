use crate::error::Error;
use crate::execution::types::execution_operation::{RetrieveIdentityInfo, ValidationOperation};
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use drive::grovedb::TransactionArg;

pub(super) fn validate_identity_exists_v0(
    drive: &Drive,
    identity_id: &Identifier,
    execution_context: &mut StateTransitionExecutionContext,
    tx: TransactionArg,
    platform_version: &PlatformVersion,
) -> Result<bool, Error> {
    let maybe_revision =
        drive.fetch_identity_revision(identity_id.to_buffer(), true, tx, platform_version)?;

    execution_context.add_operation(ValidationOperation::RetrieveIdentity(
        RetrieveIdentityInfo::only_revision(),
    ));

    Ok(maybe_revision.is_some())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution::types::execution_operation::ValidationOperation;
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::block::block_info::BlockInfo;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::Identity;
    use dpp::version::DefaultForPlatformVersion;
    use rand::SeedableRng;

    #[test]
    fn should_return_false_when_identity_not_in_state() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let missing_identity_id = Identifier::random();

        let mut execution_context =
            StateTransitionExecutionContext::default_for_platform_version(platform_version)
                .expect("should create execution context");

        let exists = validate_identity_exists_v0(
            &platform.drive,
            &missing_identity_id,
            &mut execution_context,
            None,
            platform_version,
        )
        .expect("should not error");

        assert!(!exists, "identity should not be found");

        // Confirm the fetch operation was recorded for fee accounting
        let has_retrieve_op = execution_context
            .operations_slice()
            .iter()
            .any(|op| matches!(op, ValidationOperation::RetrieveIdentity(_)));
        assert!(
            has_retrieve_op,
            "should record a RetrieveIdentity validation operation"
        );
    }

    #[test]
    fn should_return_true_when_identity_is_in_state() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let (identity, _keys): (Identity, Vec<(dpp::identity::IdentityPublicKey, [u8; 32])>) =
            Identity::random_identity_with_main_keys_with_private_key(
                2,
                &mut rand::rngs::StdRng::seed_from_u64(17),
                platform_version,
            )
            .expect("got identity");

        let identity_id = identity.id();
        platform
            .drive
            .add_new_identity(
                identity,
                false,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("should add identity");

        let mut execution_context =
            StateTransitionExecutionContext::default_for_platform_version(platform_version)
                .expect("should create execution context");

        let exists = validate_identity_exists_v0(
            &platform.drive,
            &identity_id,
            &mut execution_context,
            None,
            platform_version,
        )
        .expect("should not error");

        assert!(exists, "identity should be found after being added");
    }
}
