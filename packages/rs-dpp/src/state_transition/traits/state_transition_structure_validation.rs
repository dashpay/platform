#[cfg(feature = "state-transition-signing")]
use crate::address_funds::{AddressWitness, PlatformAddress};
#[cfg(any(feature = "state-transition-signing", test))]
use crate::consensus::basic::state_transition::StateTransitionNotActiveError;
#[cfg(any(feature = "state-transition-signing", test))]
use crate::consensus::ConsensusError;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::StateTransitionType;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use platform_version::version::feature_initial_protocol_versions::ADDRESS_FUNDS_INITIAL_PROTOCOL_VERSION;
use platform_version::version::PlatformVersion;
#[cfg(feature = "state-transition-signing")]
use platform_version::version::PLATFORM_VERSIONS;

/// Trait for validating the structure of a state transition
pub trait StateTransitionStructureValidation {
    /// Validates the structure of the state transition
    fn validate_structure(
        &self,
        platform_version: &PlatformVersion,
    ) -> SimpleConsensusValidationResult;
}

/// Converts a `SimpleConsensusValidationResult` into a `ProtocolError` when it
/// contains at least one consensus error, preserving the full error list.
///
#[cfg(any(feature = "state-transition-signing", test))]
pub(crate) fn consensus_errors_as_protocol_error(
    result: SimpleConsensusValidationResult,
) -> Option<ProtocolError> {
    (!result.errors.is_empty()).then(|| result.errors.into())
}

#[cfg(feature = "state-transition-signing")]
pub(crate) fn verify_address_witnesses<'a, I>(
    addresses: I,
    witnesses: &[AddressWitness],
    signable_bytes: &[u8],
) -> Result<(), ProtocolError>
where
    I: IntoIterator<Item = &'a PlatformAddress>,
    I::IntoIter: ExactSizeIterator,
{
    let addresses = addresses.into_iter();
    if addresses.len() != witnesses.len() {
        return Err(ProtocolError::AddressWitnessError(format!(
            "input witness count mismatch: {} addresses but {} witnesses",
            addresses.len(),
            witnesses.len()
        )));
    }

    for (address, witness) in addresses.zip(witnesses.iter()) {
        address.verify_bytes_against_witness(witness, signable_bytes)?;
    }

    Ok(())
}

#[cfg(feature = "state-transition-signing")]
fn address_funds_basic_structure_version(
    state_transition_type: StateTransitionType,
    platform_version: &PlatformVersion,
) -> Option<u16> {
    match state_transition_type {
        StateTransitionType::IdentityCreateFromAddresses => {
            platform_version
                .drive_abci
                .validation_and_processing
                .state_transitions
                .identity_create_from_addresses_state_transition
                .basic_structure
        }
        StateTransitionType::IdentityTopUpFromAddresses => {
            platform_version
                .drive_abci
                .validation_and_processing
                .state_transitions
                .identity_top_up_from_addresses_state_transition
                .basic_structure
        }
        StateTransitionType::IdentityCreditTransferToAddresses => {
            platform_version
                .drive_abci
                .validation_and_processing
                .state_transitions
                .identity_credit_transfer_to_addresses_state_transition
                .basic_structure
        }
        StateTransitionType::AddressFundsTransfer => {
            platform_version
                .drive_abci
                .validation_and_processing
                .state_transitions
                .address_funds_transfer
                .basic_structure
        }
        StateTransitionType::AddressFundingFromAssetLock => {
            platform_version
                .drive_abci
                .validation_and_processing
                .state_transitions
                .address_funds_from_asset_lock
                .basic_structure
        }
        StateTransitionType::AddressCreditWithdrawal => {
            platform_version
                .drive_abci
                .validation_and_processing
                .state_transitions
                .address_credit_withdrawal
                .basic_structure
        }
        StateTransitionType::DataContractCreate
        | StateTransitionType::Batch
        | StateTransitionType::IdentityCreate
        | StateTransitionType::IdentityTopUp
        | StateTransitionType::DataContractUpdate
        | StateTransitionType::IdentityUpdate
        | StateTransitionType::IdentityCreditWithdrawal
        | StateTransitionType::IdentityCreditTransfer
        | StateTransitionType::MasternodeVote
        | StateTransitionType::Shield
        | StateTransitionType::ShieldedTransfer
        | StateTransitionType::Unshield
        | StateTransitionType::ShieldFromAssetLock
        | StateTransitionType::ShieldedWithdrawal
        | StateTransitionType::IdentityCreateFromShieldedPool => unreachable!(
            "{state_transition_type} is not an address-funds constructor dispatch target"
        ),
    }
}

#[cfg(feature = "state-transition-signing")]
pub(crate) fn address_funds_constructor_dispatch_error(
    state_transition_type: StateTransitionType,
    platform_version: &PlatformVersion,
) -> Option<ProtocolError> {
    if platform_version.protocol_version < ADDRESS_FUNDS_INITIAL_PROTOCOL_VERSION {
        return Some(ProtocolError::from(ConsensusError::from(
            StateTransitionNotActiveError::new(
                state_transition_type.to_string(),
                platform_version.protocol_version,
                ADDRESS_FUNDS_INITIAL_PROTOCOL_VERSION,
            ),
        )));
    }

    let basic_structure =
        address_funds_basic_structure_version(state_transition_type, platform_version);

    match basic_structure {
        Some(0) => None,
        Some(version) => Some(ProtocolError::UnknownVersionMismatch {
            method: format!(
                "{state_transition_type}::try_from_inputs_with_signer/try_from_identity"
            ),
            known_versions: vec![0],
            received: version,
        }),
        None => {
            let first_active_version = PLATFORM_VERSIONS
                .iter()
                .find(|version| {
                    address_funds_basic_structure_version(state_transition_type, version) == Some(0)
                })
                .map(|version| version.protocol_version)
                .unwrap_or(platform_version.protocol_version);

            Some(ProtocolError::from(ConsensusError::from(
                StateTransitionNotActiveError::new(
                    state_transition_type.to_string(),
                    platform_version.protocol_version,
                    first_active_version,
                ),
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "state-transition-signing")]
    use crate::consensus::basic::BasicError;
    #[cfg(feature = "state-transition-signing")]
    use crate::state_transition::StateTransitionType;
    #[cfg(feature = "state-transition-signing")]
    use platform_version::version::PlatformVersion;

    #[test]
    fn helper_preserves_all_consensus_errors_for_multiple_errors() {
        let first = ConsensusError::from(StateTransitionNotActiveError::new("first", 1, 11));
        let second = ConsensusError::from(StateTransitionNotActiveError::new("second", 1, 11));
        let result =
            SimpleConsensusValidationResult::new_with_errors(vec![first.clone(), second.clone()]);

        let protocol_error = consensus_errors_as_protocol_error(result);

        assert!(matches!(
            protocol_error,
            Some(ProtocolError::ConsensusErrors(errors))
                if errors == vec![first, second]
        ));
    }

    #[cfg(feature = "state-transition-signing")]
    #[test]
    fn address_funds_dispatch_is_not_active_before_protocol_v11_even_when_basic_structure_is_v0() {
        let low_version = PlatformVersion::get(1)
            .expect("platform version 1 exists")
            .clone();

        for state_transition_type in [
            StateTransitionType::IdentityCreateFromAddresses,
            StateTransitionType::IdentityTopUpFromAddresses,
            StateTransitionType::IdentityCreditTransferToAddresses,
            StateTransitionType::AddressFundsTransfer,
            StateTransitionType::AddressFundingFromAssetLock,
            StateTransitionType::AddressCreditWithdrawal,
        ] {
            let result =
                address_funds_constructor_dispatch_error(state_transition_type, &low_version);

            assert!(matches!(
                result,
                Some(ProtocolError::ConsensusError(boxed))
                    if matches!(
                        *boxed,
                        ConsensusError::BasicError(BasicError::StateTransitionNotActiveError(
                            ref err
                        )) if err.current_protocol_version() == low_version.protocol_version
                            && err.required_protocol_version()
                                == ADDRESS_FUNDS_INITIAL_PROTOCOL_VERSION
                    )
            ));
        }
    }

    #[cfg(feature = "state-transition-signing")]
    #[test]
    fn address_funds_dispatch_still_reports_post_activation_version_mismatches() {
        let mut active_version = PlatformVersion::get(11)
            .expect("platform version 11 exists")
            .clone();
        active_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .address_funds_transfer
            .basic_structure = Some(1);

        let result = address_funds_constructor_dispatch_error(
            StateTransitionType::AddressFundsTransfer,
            &active_version,
        );

        assert!(matches!(
            result,
            Some(ProtocolError::UnknownVersionMismatch {
                received: 1,
                known_versions,
                ..
            }) if known_versions == vec![0]
        ));
    }
}
