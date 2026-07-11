use crate::error::Error;

mod verify_contract;
mod verify_contract_history;
mod verify_contract_return_serialization;

fn retry_contract_verification_with_history<R, RetryFn, HasPresentContractFn>(
    result: Result<R, Error>,
    contract_known_keeps_history: Option<bool>,
    contract_id: [u8; 32],
    in_multiple_contract_proof_form: bool,
    retry: RetryFn,
    has_present_contract: HasPresentContractFn,
) -> Result<R, Error>
where
    RetryFn: FnOnce() -> Result<R, Error>,
    HasPresentContractFn: Fn(&R) -> bool,
{
    if contract_known_keeps_history.is_some() {
        return result;
    }

    match &result {
        Ok(value) if has_present_contract(value) => result,
        Ok(_) => {
            tracing::debug!(
                ?contract_id,
                keeps_history = false,
                retry_keeps_history = true,
                in_multiple_contract_proof_form,
                "retrying contract verification with history enabled after absence"
            );

            let retry_result = retry();
            if matches!(retry_result.as_ref(), Ok(value) if has_present_contract(value)) {
                retry_result
            } else {
                result
            }
        }
        Err(error) => {
            tracing::debug!(
                ?contract_id,
                keeps_history = false,
                retry_keeps_history = true,
                in_multiple_contract_proof_form,
                error = ?error,
                "retrying contract verification with history enabled after error"
            );

            let retry_result = retry();
            if matches!(retry_result.as_ref(), Ok(value) if has_present_contract(value)) {
                retry_result
            } else {
                result
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::retry_contract_verification_with_history;
    use crate::error::proof::ProofError;
    use crate::error::Error;

    #[test]
    fn should_preserve_original_error_when_retry_returns_absence() {
        let result = retry_contract_verification_with_history(
            Err(Error::Proof(ProofError::IncompleteProof("first error"))),
            None,
            [1; 32],
            false,
            || Ok(None::<u8>),
            Option::is_some,
        );

        assert!(matches!(
            result,
            Err(Error::Proof(ProofError::IncompleteProof("first error")))
        ));
    }

    #[test]
    fn should_return_retry_result_when_retry_finds_contract_after_error() {
        let result = retry_contract_verification_with_history(
            Err(Error::Proof(ProofError::IncompleteProof("first error"))),
            None,
            [1; 32],
            false,
            || Ok(Some(7u8)),
            Option::is_some,
        );

        assert!(matches!(result, Ok(Some(7))));
    }
}
