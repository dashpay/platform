use crate::{
    consensus::{basic::BasicError, ConsensusError},
    prelude::{Identifier, Identity},
    state_repository::StateRepositoryLike,
    state_transition::StateTransitionLike,
    ProtocolError,
};

use super::identity_update_transition::IdentityUpdateTransition;

/// Apply Identity Update state transition
pub async fn apply_identity_update_transition(
    state_repository: &impl StateRepositoryLike,
    state_transition: IdentityUpdateTransition,
) -> Result<(), ProtocolError> {
    let maybe_identity: Option<Identity> = state_repository
        .fetch_identity(
            state_transition.get_identity_id(),
            state_transition.get_execution_context(),
        )
        .await?;
    let mut identity = match maybe_identity {
        None => {
            return Err(identity_not_found_error(
                state_transition.get_identity_id().to_owned(),
            ))
        }
        Some(id) => id,
    };

    identity.revision = state_transition.get_revision();

    if !state_transition.get_public_key_ids_to_disable().is_empty() {
        for id in state_transition.get_public_key_ids_to_disable() {
            if let Some(ref mut public_key) = identity.get_public_key_by_id_mut(*id) {
                public_key.disabled_at = state_transition.get_public_keys_disabled_at();
            }
        }
    }

    if !state_transition.get_public_keys_to_add().is_empty() {
        identity.add_public_keys(
            state_transition
                .get_public_keys_to_add()
                .iter()
                .cloned()
                .map(|mut pk| {
                    pk.set_signature(vec![]);
                    pk
                }),
        );
        let public_key_hashes: Vec<Vec<u8>> = state_transition
            .get_public_keys_to_add()
            .iter()
            .map(|pk| pk.hash())
            .collect::<Result<_, _>>()?;

        state_repository
            .store_identity_public_key_hashes(
                identity.get_id(),
                public_key_hashes,
                state_transition.get_execution_context(),
            )
            .await?;
    }

    state_repository
        .update_identity(&identity, state_transition.get_execution_context())
        .await?;

    Ok(())
}

fn identity_not_found_error(identity_id: Identifier) -> ProtocolError {
    ProtocolError::AbstractConsensusError(Box::new(ConsensusError::BasicError(Box::new(
        BasicError::IdentityNotFoundError { identity_id },
    ))))
}
