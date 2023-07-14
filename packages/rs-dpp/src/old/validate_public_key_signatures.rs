use platform_value::Value;

use crate::consensus::basic::identity::InvalidIdentityKeySignatureError;
use crate::consensus::basic::state_transition::InvalidStateTransitionTypeError;
use crate::{
    consensus::{basic::BasicError, ConsensusError},
    object_names,
    state_transition::{
        try_get_transition_type, StateTransition, StateTransitionLike, StateTransitionType,
    },
    validation::SimpleConsensusValidationResult,
    BlsModule, NonConsensusError, ProtocolError,
};

pub trait TPublicKeysSignaturesValidator {
    fn validate_public_key_signatures<'a>(
        &self,
        raw_state_transition: &Value,
        raw_public_keys: impl IntoIterator<Item = &'a Value>,
    ) -> Result<SimpleConsensusValidationResult, NonConsensusError>;
}

pub struct PublicKeysSignaturesValidator<T: BlsModule> {
    bls: T,
}

impl<T: BlsModule> PublicKeysSignaturesValidator<T> {
    pub fn new(bls: T) -> Self {
        Self { bls }
    }
}

impl<T: BlsModule> TPublicKeysSignaturesValidator for PublicKeysSignaturesValidator<T> {
    fn validate_public_key_signatures<'a>(
        &self,
        raw_state_transition: &Value,
        raw_public_keys: impl IntoIterator<Item = &'a Value>,
    ) -> Result<SimpleConsensusValidationResult, NonConsensusError> {
        validate_public_key_signatures(raw_state_transition, raw_public_keys, &self.bls)
    }
}

pub fn validate_public_key_signatures<'a, T: BlsModule>(
    raw_state_transition: &Value,
    raw_public_keys: impl IntoIterator<Item = &'a Value>,
    bls: &T,
) -> Result<SimpleConsensusValidationResult, NonConsensusError> {
    let mut validation_result = SimpleConsensusValidationResult::default();

    let transition_type = try_get_transition_type(raw_state_transition)
        .map_err(|e| NonConsensusError::InvalidDataProcessedError(format!("{e:#}")))?;
    // We don't use a universal constructor from the state transition factory, because the constructor
    // depends on State Repository. The dependency is used only while creating a `DocumentsBatch`.
    // As we never create  `DocumentsBatch` there is no reason to introduce an additional dependency to the validator.
    let mut state_transition =
        match transition_type {
            StateTransitionType::IdentityCreate => {
                let transition =
                    IdentityCreateTransition::from_object(raw_state_transition.clone())
                        .map_err(|e| NonConsensusError::ObjectCreationError {
                            object_name: object_names::STATE_TRANSITION,
                            details: format!("{e:#}"),
                        })?;
                StateTransition::IdentityCreate(transition)
            }
            StateTransitionType::IdentityUpdate => {
                let transition =
                    IdentityUpdateTransition::from_object(raw_state_transition.clone())
                        .map_err(|e| NonConsensusError::ObjectCreationError {
                            object_name: object_names::STATE_TRANSITION,
                            details: format!("{e:#}"),
                        })?;
                StateTransition::IdentityUpdate(transition)
            }
            transition_type => {
                return Err(NonConsensusError::InvalidDataProcessedError(format!(
                    "The transition '{}' is not supported when validating public key signatures ",
                    transition_type
                )))
            }
        };

    let add_public_key_transitions: Vec<IdentityPublicKeyInCreation> = raw_public_keys
        .into_iter()
        .map(|k| {
            IdentityPublicKeyInCreation::from_object(k.to_owned())
                .map_err(|e| NonConsensusError::IdentityPublicKeyCreateError(format!("{:#}", e)))
        })
        .collect::<Result<_, _>>()?;

    let maybe_invalid_public_key_transition =
        find_invalid_public_key(&mut state_transition, add_public_key_transitions, bls);
    if let Some(invalid_key_transition) = maybe_invalid_public_key_transition {
        validation_result.add_error(BasicError::InvalidIdentityKeySignatureError(
            InvalidIdentityKeySignatureError::new(invalid_key_transition.id),
        ))
    }

    Ok(validation_result)
}

fn invalid_state_transition_type_error(transition_type: u8) -> ProtocolError {
    ProtocolError::ConsensusError(Box::new(ConsensusError::BasicError(
        BasicError::InvalidStateTransitionTypeError(InvalidStateTransitionTypeError::new(
            transition_type,
        )),
    )))
}

fn find_invalid_public_key<T: BlsModule>(
    state_transition: &mut impl StateTransitionLike,
    public_keys: impl IntoIterator<Item = IdentityPublicKeyInCreation>,
    bls: &T,
) -> Option<IdentityPublicKeyInCreation> {
    for public_key in public_keys {
        state_transition.set_signature(public_key.signature.clone());
        if state_transition
            .verify_by_public_key(public_key.data.as_slice(), public_key.key_type, bls)
            .is_err()
        {
            return Some(public_key);
        }
    }
    None
}
