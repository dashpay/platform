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
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::block::block_info::BlockInfo;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::Identity;
    use dpp::version::DefaultForPlatformVersion;

    #[test]
    fn should_return_true_when_identity_exists() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let identity =
            Identity::random_identity(3, Some(50), platform_version).expect("got an identity");
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

        let result = validate_identity_exists_v0(
            &platform.drive,
            &identity_id,
            &mut execution_context,
            None,
            platform_version,
        )
        .expect("should succeed");

        assert!(result, "identity should exist");
    }

    #[test]
    fn should_return_false_when_identity_does_not_exist() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let non_existent_id = Identifier::random();
        let mut execution_context =
            StateTransitionExecutionContext::default_for_platform_version(platform_version)
                .expect("should create execution context");

        let result = validate_identity_exists_v0(
            &platform.drive,
            &non_existent_id,
            &mut execution_context,
            None,
            platform_version,
        )
        .expect("should succeed");

        assert!(!result, "identity should not exist");
    }
}
