use crate::error::execution::ExecutionError;
use crate::error::Error;
use dpp::identity::PartialIdentity;
use dpp::state_transition::identity_credit_withdrawal_transition::accessors::IdentityCreditWithdrawalTransitionAccessorsV0;
use dpp::state_transition::identity_credit_withdrawal_transition::methods::validate_withdrawal_output_script;
use dpp::state_transition::identity_credit_withdrawal_transition::IdentityCreditWithdrawalTransition;
use dpp::state_transition::StateTransitionIdentitySigned;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;

pub(super) trait IdentityCreditWithdrawalStateTransitionSignaturePurposeMatchesRequirementsValidationV0
{
    fn validate_signature_purpose_matches_requirements_v0(
        &self,
        identity: &PartialIdentity,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}

impl IdentityCreditWithdrawalStateTransitionSignaturePurposeMatchesRequirementsValidationV0
    for IdentityCreditWithdrawalTransition
{
    fn validate_signature_purpose_matches_requirements_v0(
        &self,
        identity: &PartialIdentity,
        _platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let mut result = SimpleConsensusValidationResult::default();

        if let Some(output_script) = self.output_script() {
            let Some(signing_key) = identity
                .loaded_public_keys
                .get(&self.signature_public_key_id())
            else {
                return Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                    "we should have a loaded key at this point",
                )));
            };

            // The shared rule preserves the OWNER-only check and the same error payload for v0.
            if let Err(error) = validate_withdrawal_output_script(signing_key, Some(&output_script))
            {
                result.add_error(error);
            }
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::super::IdentityCreditWithdrawalStateTransitionSignaturePurposeMatchesRequirementsValidation;
    use super::*;
    use dpp::consensus::basic::identity::WithdrawalOutputScriptNotAllowedWhenSigningWithOwnerKeyError;
    use dpp::consensus::ConsensusError;
    use dpp::identity::core_script::CoreScript;
    use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use dpp::identity::Purpose;
    use dpp::state_transition::identity_credit_withdrawal_transition::v1::IdentityCreditWithdrawalTransitionV1;
    use dpp::version::v15::PLATFORM_V15;

    const SIGNING_KEY_ID: u32 = 17;

    fn withdrawal(output_script: Option<CoreScript>) -> IdentityCreditWithdrawalTransition {
        IdentityCreditWithdrawalTransitionV1 {
            output_script,
            signature_public_key_id: SIGNING_KEY_ID,
            ..Default::default()
        }
        .into()
    }

    fn partial_identity(purpose: Option<Purpose>) -> PartialIdentity {
        PartialIdentity {
            id: Default::default(),
            loaded_public_keys: purpose
                .map(|purpose| {
                    let key = IdentityPublicKeyV0 {
                        id: SIGNING_KEY_ID,
                        purpose,
                        ..Default::default()
                    };
                    (SIGNING_KEY_ID, key.into())
                })
                .into_iter()
                .collect(),
            balance: None,
            revision: None,
            not_found_public_keys: Default::default(),
        }
    }

    #[test]
    fn should_refuse_an_owner_key_with_an_output_script() {
        let output_script = CoreScript::from_bytes(vec![0x51]);
        let transition = withdrawal(Some(output_script.clone()));
        let identity = partial_identity(Some(Purpose::OWNER));
        let expected = ConsensusError::from(
            WithdrawalOutputScriptNotAllowedWhenSigningWithOwnerKeyError::new(
                output_script,
                SIGNING_KEY_ID,
            ),
        );

        // Platform 5.1 shipped protocol 15 with this validator.
        for platform_version in [&PLATFORM_V15, PlatformVersion::latest()] {
            let result = transition
                .validate_signature_purpose_matches_requirements(&identity, platform_version)
                .expect("consensus refusal is a validation result");

            assert_eq!(result.errors.as_slice(), std::slice::from_ref(&expected));
            assert!(result.data.is_none());
        }
    }

    #[test]
    fn should_allow_no_output_script_even_when_the_owner_key_is_not_loaded() {
        for purpose in [Some(Purpose::OWNER), None] {
            let result = withdrawal(None)
                .validate_signature_purpose_matches_requirements(
                    &partial_identity(purpose),
                    PlatformVersion::latest(),
                )
                .expect("validation without an output script does not require the key");

            assert!(result.errors.is_empty());
            assert!(result.data.is_none());
        }
    }

    #[test]
    fn should_allow_a_transfer_key_with_an_output_script() {
        let result = withdrawal(Some(CoreScript::from_bytes(vec![0x51])))
            .validate_signature_purpose_matches_requirements(
                &partial_identity(Some(Purpose::TRANSFER)),
                PlatformVersion::latest(),
            )
            .expect("TRANSFER should pass this validation");

        assert!(result.errors.is_empty());
        assert!(result.data.is_none());
    }

    #[test]
    fn should_return_an_internal_error_for_an_output_script_without_a_loaded_key() {
        let error = withdrawal(Some(CoreScript::from_bytes(vec![0x51])))
            .validate_signature_purpose_matches_requirements(
                &partial_identity(None),
                PlatformVersion::latest(),
            )
            .expect_err("validation with an output script requires the key");

        assert!(matches!(
            error,
            Error::Execution(ExecutionError::CorruptedCodeExecution(
                "we should have a loaded key at this point"
            ))
        ));
    }
}
