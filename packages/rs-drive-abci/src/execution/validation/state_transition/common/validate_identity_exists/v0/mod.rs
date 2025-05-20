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
