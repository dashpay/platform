use crate::error::Error;
use crate::execution::types::execution_operation::{RetrieveIdentityInfo, ValidationOperation};
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use drive::drive::identity::key::fetch::KeyRequestType::LatestAuthenticationMasterKey;
use drive::drive::identity::key::fetch::{
    IdentityKeysRequest, OptionalSingleIdentityPublicKeyOutcome,
};
use drive::drive::Drive;
use drive::grovedb::TransactionArg;

pub(super) fn validate_non_masternode_identity_exists_v0(
    drive: &Drive,
    identity_id: &Identifier,
    execution_context: &mut StateTransitionExecutionContext,
    tx: TransactionArg,
    platform_version: &PlatformVersion,
) -> Result<bool, Error> {
    let maybe_key = drive.fetch_identity_keys::<OptionalSingleIdentityPublicKeyOutcome>(
        IdentityKeysRequest {
            identity_id: identity_id.to_buffer(),
            request_type: LatestAuthenticationMasterKey,
            limit: Some(1),
            offset: None,
        },
        tx,
        platform_version,
    )?;

    execution_context.add_operation(ValidationOperation::RetrieveIdentity(
        RetrieveIdentityInfo::one_key(),
    ));

    Ok(maybe_key.is_some())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContextMethodsV0;
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::block::block_info::BlockInfo;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::Identity;
    use dpp::version::DefaultForPlatformVersion;
    use rand::SeedableRng;

    #[test]
    fn should_return_false_when_identity_not_present() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let missing_identity_id = Identifier::random();

        let mut execution_context =
            StateTransitionExecutionContext::default_for_platform_version(platform_version)
                .expect("should create execution context");

        let exists = validate_non_masternode_identity_exists_v0(
            &platform.drive,
            &missing_identity_id,
            &mut execution_context,
            None,
            platform_version,
        )
        .expect("should not error");

        assert!(!exists, "non-masternode identity should not be found");
        assert!(
            !execution_context.operations_slice().is_empty(),
            "should record the key-fetch operation"
        );
    }

    #[test]
    fn should_return_true_when_identity_with_master_key_present() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        // random_identity_with_main_keys_with_private_key generates identities with
        // a master authentication key, so the LatestAuthenticationMasterKey request
        // should find it.
        let (identity, _keys): (Identity, Vec<(dpp::identity::IdentityPublicKey, [u8; 32])>) =
            Identity::random_identity_with_main_keys_with_private_key(
                3,
                &mut rand::rngs::StdRng::seed_from_u64(42),
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

        let exists = validate_non_masternode_identity_exists_v0(
            &platform.drive,
            &identity_id,
            &mut execution_context,
            None,
            platform_version,
        )
        .expect("should not error");

        assert!(exists, "identity should be found with its master key");
    }
}
