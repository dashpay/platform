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
