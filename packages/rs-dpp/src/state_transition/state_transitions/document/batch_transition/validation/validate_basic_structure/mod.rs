use crate::state_transition::batch_transition::accessors::DocumentsBatchTransitionAccessorsV0;
use crate::state_transition::batch_transition::document_create_transition::validate_structure::DocumentCreateTransitionStructureValidation;
use crate::state_transition::batch_transition::BatchTransition;
use crate::state_transition::StateTransitionOwned;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

#[cfg(feature = "state-transition-signing")]
use crate::identity::signer::Signer;
#[cfg(feature = "state-transition-signing")]
use crate::identity::{IdentityPublicKey, SecurityLevel};
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::batch_transition::methods::StateTransitionCreationOptions;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::{GetDataContractSecurityLevelRequirementFn, StateTransition};

mod v0;

impl BatchTransition {
    /// Validates the base structure of a batch transition before broadcast.
    ///
    /// This always performs batch-level checks such as emptiness, max count,
    /// duplicate document IDs, and document/token nonce bounds.
    ///
    /// The document branch is intentionally batch-level only in this
    /// server-reachable validator. Document transition-local checks either
    /// depend on contract/state context or are reserved for constructor-only
    /// pre-sign validation. Token transition variants still receive their
    /// client-side typed structure checks here.
    pub fn validate_base_structure(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        match platform_version
            .dpp
            .state_transitions
            .documents
            .documents_batch_transition
            .validation
            .validate_base_structure
        {
            0 => self.validate_base_structure_v0(platform_version),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DocumentsBatchTransition::validate_base_structure".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    /// Runs constructor-side batch base-structure validation, adds
    /// constructor-only create document ID checks, and maps consensus
    /// validation failures into `ProtocolError`.
    ///
    /// Used by `BatchTransition::new_*` constructors to fail fast before
    /// signing when the freshly-constructed transition is structurally invalid.
    /// When `state-transition-signing` is enabled this helper is compiled in
    /// together with `batch-base-structure-validation`, so the constructor
    /// pre-sign hook is not cfg-elided. The create-transition ID check is
    /// constructor defense-in-depth; SDK create builders normalize document IDs
    /// before calling this hook, so that error is not user-reachable there.
    #[cfg(any(test, feature = "state-transition-signing"))]
    pub(crate) fn validate_base_structure_pre_sign(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<(), ProtocolError> {
        let mut result = match platform_version
            .dpp
            .state_transitions
            .documents
            .documents_batch_transition
            .validation
            .validate_base_structure
        {
            0 => self.validate_base_structure_pre_sign_v0(platform_version)?,
            version => {
                return Err(ProtocolError::UnknownVersionMismatch {
                    method: "DocumentsBatchTransition::validate_base_structure_pre_sign"
                        .to_string(),
                    known_versions: vec![0],
                    received: version,
                })
            }
        };

        for batch_transition in self.transitions_iter() {
            let crate::state_transition::batch_transition::batched_transition::BatchedTransitionRef::Document(
                crate::state_transition::state_transitions::document::batch_transition::batched_transition::document_transition::DocumentTransition::Create(
                    create_transition,
                ),
            ) = batch_transition
            else {
                continue;
            };

            let create_result =
                create_transition.validate_structure(self.owner_id(), platform_version)?;
            if !create_result.is_valid() {
                result.merge(create_result);
            }
        }

        match result.errors.len() {
            0 => Ok(()),
            1 => Err(ProtocolError::ConsensusError(Box::new(
                result.errors.pop().expect("validated single error count"),
            ))),
            _ => Err(ProtocolError::ConsensusErrors(result.errors)),
        }
    }

    /// Runs the constructor pre-sign validation, converts the batch into a
    /// `StateTransition`, and signs it.
    ///
    /// This consolidates the duplicated validate-and-sign sequence used by all
    /// `BatchTransition::new_*` constructors (document and token alike).
    ///
    /// `required_security_level` lets document constructors pin the
    /// signing key's security level to the document type's requirement; token
    /// constructors pass `None` so the default per-state-transition logic
    /// applies.
    #[cfg(feature = "state-transition-signing")]
    pub(crate) async fn validate_and_sign<S: Signer<IdentityPublicKey>>(
        self,
        identity_public_key: &IdentityPublicKey,
        signer: &S,
        required_security_level: Option<SecurityLevel>,
        platform_version: &PlatformVersion,
        options: Option<StateTransitionCreationOptions>,
    ) -> Result<StateTransition, ProtocolError> {
        self.validate_base_structure_pre_sign(platform_version)?;
        let resolved_options = options.unwrap_or_default();
        let mut state_transition: StateTransition = self.into();
        match required_security_level {
            Some(level) => {
                state_transition
                    .sign_external_with_options(
                        identity_public_key,
                        signer,
                        Some(move |_, _| Ok(level)),
                        resolved_options.signing_options,
                    )
                    .await?;
            }
            None => {
                state_transition
                    .sign_external_with_options(
                        identity_public_key,
                        signer,
                        None::<GetDataContractSecurityLevelRequirementFn>,
                        resolved_options.signing_options,
                    )
                    .await?;
            }
        }
        Ok(state_transition)
    }
}
