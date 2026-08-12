use crate::error::Error;

use dpp::consensus::basic::identity::TooManyPublicKeysOfPurposeError;

use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::{KeyID, Purpose};
use dpp::platform_value::Identifier;

use dpp::state_transition::public_key_in_creation::accessors::IdentityPublicKeyInCreationV0Getters;
use dpp::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use dpp::validation::SimpleConsensusValidationResult;

use drive::drive::identity::key::fetch::{
    IdentityKeysRequest, KeyIDIdentityPublicKeyPairBTreeMap, KeyRequestType,
};
use drive::drive::Drive;
use drive::grovedb::TransactionArg;

use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use dpp::version::PlatformVersion;

/// Validates that after this transition the identity holds at most one active
/// key of each DIP-33 payment purpose.
///
/// The in-transition half (at most one of each payment purpose among the added
/// keys) is enforced by `validate_identity_public_keys_structure` v1; this
/// checks the against-state half for identity updates: an added payment key is
/// only valid if the identity has no other active key of that purpose, unless
/// that key is being disabled in the same transition (the rotation path).
///
/// Payment purposes are not searchable (no per-purpose key reference tree), so
/// this fetches all of the identity's keys and filters in memory. The fetch is
/// skipped entirely when no payment key is being added, which also makes this
/// check unreachable at protocol versions that reject payment purposes at
/// structure validation.
#[allow(clippy::too_many_arguments)]
pub(super) fn validate_payment_key_uniqueness_in_state_v0(
    identity_id: Identifier,
    public_keys_being_added: &[IdentityPublicKeyInCreation],
    public_key_ids_to_disable: &[KeyID],
    drive: &Drive,
    _execution_context: &mut StateTransitionExecutionContext,
    transaction: TransactionArg,
    platform_version: &PlatformVersion,
) -> Result<SimpleConsensusValidationResult, Error> {
    let payment_purposes_being_added: Vec<Purpose> = Purpose::payment_purposes()
        .into_iter()
        .filter(|purpose| {
            public_keys_being_added
                .iter()
                .any(|key| key.purpose() == *purpose)
        })
        .collect();

    if payment_purposes_being_added.is_empty() {
        return Ok(SimpleConsensusValidationResult::new());
    }

    let identity_key_request = IdentityKeysRequest {
        identity_id: identity_id.to_buffer(),
        request_type: KeyRequestType::AllKeys,
        limit: None,
        offset: None,
    };
    let existing_keys = drive.fetch_identity_keys::<KeyIDIdentityPublicKeyPairBTreeMap>(
        identity_key_request,
        transaction,
        platform_version,
    )?;

    for purpose in payment_purposes_being_added {
        let conflicting_active_key_exists = existing_keys.values().any(|key| {
            key.purpose() == purpose
                && key.disabled_at().is_none()
                && !public_key_ids_to_disable.contains(&key.id())
        });
        if conflicting_active_key_exists {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                TooManyPublicKeysOfPurposeError::new(purpose, 1).into(),
            ));
        }
    }

    Ok(SimpleConsensusValidationResult::new())
}
