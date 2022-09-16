use crate::{
    consensus::signature::SignatureError,
    prelude::{Identifier, Identity},
    state_repository::StateRepositoryLike,
    validation::ValidationResult,
    ProtocolError,
};

pub async fn validate_identity_existence(
    state_repository: &impl StateRepositoryLike,
    identity_id: &Identifier,
) -> Result<ValidationResult<Identity>, ProtocolError> {
    let mut result = ValidationResult::<Identity>::default();

    let maybe_identity: Option<Identity> = state_repository.fetch_identity(identity_id).await?;
    match maybe_identity {
        None => result.add_error(SignatureError::IdentityNotFoundError {
            identity_id: identity_id.to_owned(),
        }),

        Some(identity) => {
            result.set_data(identity);
        }
    }

    Ok(result)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        state_repository::MockStateRepositoryLike,
        tests::{fixtures::identity_fixture_raw_object, utils::get_signature_error_from_result},
    };

    #[tokio::test]
    async fn should_return_invalid_result_if_identity_is_not_found() {
        let mut state_repository_mock = MockStateRepositoryLike::new();
        let raw_identity = identity_fixture_raw_object();
        let identity = Identity::from_raw_object(raw_identity).unwrap();

        state_repository_mock
            .expect_fetch_identity::<Identity>()
            .returning(|_| Ok(None));

        let result = validate_identity_existence(&state_repository_mock, identity.get_id())
            .await
            .expect("validation result should be returned");
        let signature_error = get_signature_error_from_result(&result, 0);

        assert!(
            matches!(signature_error, SignatureError::IdentityNotFoundError { identity_id } if identity_id == identity.get_id())
        )
    }

    #[tokio::test]
    async fn should_return_valid_result() {
        let mut state_repository_mock = MockStateRepositoryLike::new();
        let raw_identity = identity_fixture_raw_object();
        let identity = Identity::from_raw_object(raw_identity).unwrap();

        let identity_to_return = identity.clone();
        state_repository_mock
            .expect_fetch_identity::<Identity>()
            .returning(move |_| Ok(Some(identity_to_return.clone())));

        let result = validate_identity_existence(&state_repository_mock, identity.get_id())
            .await
            .expect("validation result should be returned");
        assert!(result.is_valid());
    }
}
