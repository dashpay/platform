use crate::error::Error;
use crate::execution::types::execution_event::ExecutionEvent;
use crate::execution::validation::state_transition::transformer::StateTransitionActionTransformerV0;
use crate::platform_types::platform::{PlatformRef, PlatformStateRef};
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::identity::PartialIdentity;
use dpp::prelude::ConsensusValidationResult;

use crate::error::execution::ExecutionError;
use dpp::serialization::Signable;
use dpp::state_transition::StateTransition;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::{DefaultForPlatformVersion, PlatformVersion};
use drive::drive::Drive;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::StateTransitionAction;

use crate::execution::types::state_transition_execution_context::{StateTransitionExecutionContext};
use crate::execution::validation::state_transition::common::validate_state_transition_identity_signed::{ValidateStateTransitionIdentitySignature};
use crate::execution::validation::state_transition::state_transitions::identity_update::identity_and_signatures::v0::IdentityUpdateStateTransitionIdentityAndSignaturesValidationV0;
use crate::execution::validation::state_transition::state_transitions::identity_create::identity_and_signatures::v0::IdentityCreateStateTransitionIdentityAndSignaturesValidationV0;
use crate::execution::validation::state_transition::state_transitions::identity_top_up::identity_retrieval::v0::IdentityTopUpStateTransitionIdentityRetrievalV0;
use crate::execution::validation::state_transition::ValidationMode;

pub(in crate::execution) fn process_state_transition_v0<'a, C: CoreRPCLike>(
    platform: &'a PlatformRef<C>,
    block_info: &BlockInfo,
    state_transition: StateTransition,
    transaction: TransactionArg,
    platform_version: &PlatformVersion,
) -> Result<ConsensusValidationResult<ExecutionEvent<'a>>, Error> {
    let mut state_transition_execution_context =
        StateTransitionExecutionContext::default_for_platform_version(platform_version)?;

    let action = if state_transition.requires_state_to_validate_identity_and_signatures() {
        let state_transition_action_result = state_transition.transform_into_action(
            platform,
            block_info,
            ValidationMode::Validator,
            &mut state_transition_execution_context,
            transaction,
        )?;
        if !state_transition_action_result.is_valid_with_data() {
            return Ok(
                ConsensusValidationResult::<ExecutionEvent>::new_with_errors(
                    state_transition_action_result.errors,
                ),
            );
        }
        Some(state_transition_action_result.into_data()?)
    } else {
        None
    };

    // Validating signatures
    let result = state_transition.validate_identity_and_signatures(
        platform.drive,
        action.as_ref(),
        transaction,
        &mut state_transition_execution_context,
        platform_version,
    )?;

    if !result.is_valid() {
        // If the signature is not valid we do not have the user pay for the state transition
        // Since it is most likely not from them
        // Proposers should remove such transactions from the block
        // Other validators should reject blocks with such transactions
        return Ok(ConsensusValidationResult::<ExecutionEvent>::new_with_errors(result.errors));
    }

    let mut maybe_identity = result.into_data()?;

    if state_transition.has_nonces_validation() {
        // Validating identity contract nonce, this must happen after validating the signature
        let result = state_transition.validate_nonces(
            &platform.into(),
            platform.state.last_block_info(),
            transaction,
            platform_version,
        )?;

        if !result.is_valid() {
            // If the nonce is not valid the state transition is not paid for, most likely because
            // this is just a replayed block
            // Proposers should remove such transactions from the block
            // Other validators should reject blocks with such transactions
            return Ok(ConsensusValidationResult::<ExecutionEvent>::new_with_errors(result.errors));
        }
    }

    if state_transition.has_basic_structure_validation() {
        // We validate basic structure validation after verifying the identity,
        // this is structure validation that does not require state and is already checked on check_tx
        let consensus_result = state_transition.validate_basic_structure(platform_version)?;

        if !consensus_result.is_valid() {
            // Basic structure validation is extremely cheap to process, because of this attacks are
            // not likely.
            // Often the basic structure validation is necessary for estimated costs
            // Proposers should remove such transactions from the block
            // Other validators should reject blocks with such transactions
            return Ok(
                ConsensusValidationResult::<ExecutionEvent>::new_with_errors(
                    consensus_result.errors,
                ),
            );
        }
    }

    if state_transition.has_advanced_structure_validation() {
        // Next we have advanced structure validation, this is structure validation that does not require
        // state but isn't checked on check_tx. If advanced structure fails identity nonces or identity
        // contract nonces will be bumped
        let consensus_result = state_transition.validate_advanced_structure(platform_version)?;

        if !consensus_result.is_valid() {
            return consensus_result.map_result(|action| {
                ExecutionEvent::create_from_state_transition_action(
                    action,
                    maybe_identity,
                    platform.state.last_committed_block_epoch_ref(),
                    state_transition_execution_context,
                    platform_version,
                )
            });
        }
    }

    let action = if state_transition.requires_advance_structure_validation_from_state() {
        let action = if let Some(action) = action {
            action
        } else {
            let state_transition_action_result = state_transition.transform_into_action(
                platform,
                block_info,
                ValidationMode::Validator,
                &mut state_transition_execution_context,
                transaction,
            )?;
            if !state_transition_action_result.is_valid_with_data() {
                return Ok(
                    ConsensusValidationResult::<ExecutionEvent>::new_with_errors(
                        state_transition_action_result.errors,
                    ),
                );
            }
            state_transition_action_result.into_data()?
        };

        // Validating structure
        let result = state_transition.validate_advanced_structure_from_state(
            &platform.into(),
            &action,
            platform_version,
        )?;
        if !result.is_valid() {
            return Ok(ConsensusValidationResult::<ExecutionEvent>::new_with_errors(result.errors));
        }

        Some(action)
    } else {
        None
    };

    if state_transition.has_balance_validation() {
        // Validating that we have sufficient balance for a transfer or withdrawal,
        // this must happen after validating the signature
        let result = state_transition.validate_balance(
            maybe_identity.as_mut(),
            &platform.into(),
            platform.state.last_block_info(),
            transaction,
            platform_version,
        )?;

        if !result.is_valid() {
            return Ok(ConsensusValidationResult::<ExecutionEvent>::new_with_errors(result.errors));
        }
    }

    // Validating state
    let result = state_transition.validate_state(
        action,
        platform,
        ValidationMode::Validator,
        &mut state_transition_execution_context,
        transaction,
    )?;

    result.map_result(|action| {
        ExecutionEvent::create_from_state_transition_action(
            action,
            maybe_identity,
            platform.state.last_committed_block_epoch_ref(),
            state_transition_execution_context,
            platform_version,
        )
    })
}

/// A trait for validating state transitions within a blockchain.
pub(crate) trait StateTransitionSignatureValidationV0 {
    /// Validates the identity and signatures of a transaction to ensure its authenticity.
    ///
    /// # Arguments
    ///
    /// * `drive` - A reference to the drive containing the transaction data.
    /// * `tx` - The transaction argument to be authenticated.
    /// * `execution_context` - A mutable reference to the StateTransitionExecutionContext that provides the context for validation.
    /// * `platform_version` - A reference to the PlatformVersion to be used for validation.
    ///
    /// # Returns
    ///
    /// Returns a `Result` with either:
    /// - `Ok(ConsensusValidationResult<Option<PartialIdentity>>)`: Indicates that the transaction has passed authentication, and the result contains an optional `PartialIdentity`.
    /// - `Err(Error)`: Indicates that the transaction failed authentication, and the result contains an `Error` indicating the reason for failure.
    ///
    fn validate_identity_and_signatures(
        &self,
        drive: &Drive,
        action: Option<&StateTransitionAction>,
        tx: TransactionArg,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<Option<PartialIdentity>>, Error>;

    /// This means we should transform into the action before validation of the identity and signatures
    fn requires_state_to_validate_identity_and_signatures(&self) -> bool {
        false
    }
}

/// A trait for validating state transitions within a blockchain.
pub(crate) trait StateTransitionBasicStructureValidationV0 {
    /// Validates the structure of a transaction by checking its basic elements.
    ///
    /// # Arguments
    ///
    /// * `platform` - A reference to the platform state ref.
    /// * `platform_version` - The platform version.
    ///
    /// # Returns
    ///
    /// * `Result<SimpleConsensusValidationResult, Error>` - A result with either a SimpleConsensusValidationResult or an Error.
    fn validate_basic_structure(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;

    /// True if the state transition has basic structure validation.
    /// Currently only data contract update does not
    fn has_basic_structure_validation(&self) -> bool {
        true
    }
}

/// A trait for validating state transitions within a blockchain.
/// The advanced structure validation should always happen in a block
/// and not in check_tx
pub(crate) trait StateTransitionAdvancedStructureValidationV0 {
    /// Validates the structure of a transaction by checking its basic elements.
    ///
    /// # Arguments
    ///
    /// * `platform` - A reference to the platform state ref.
    /// * `platform_version` - The platform version.
    ///
    /// # Returns
    ///
    /// * `Result<SimpleConsensusValidationResult, Error>` - A result with either a SimpleConsensusValidationResult or an Error.
    fn validate_advanced_structure(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;

    /// True if the state transition has advanced structure validation.
    /// This structure validation makes users pay if there is a failure
    fn has_advanced_structure_validation(&self) -> bool;
}

/// A trait for validating state transitions within a blockchain.
pub(crate) trait StateTransitionNonceValidationV0 {
    /// Validates the structure of a transaction by checking its basic elements.
    ///
    /// # Arguments
    ///
    /// * `platform_version` - The platform version.
    ///
    /// # Returns
    ///
    /// * `Result<SimpleConsensusValidationResult, Error>` - A result with either a SimpleConsensusValidationResult or an Error.
    fn validate_nonces(
        &self,
        platform: &PlatformStateRef,
        block_info: &BlockInfo,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;

    /// True if the state transition validates nonces, either identity nonces or identity contract
    /// nonces
    fn has_nonces_validation(&self) -> bool {
        true
    }
}

/// A trait for validating state transitions within a blockchain.
pub(crate) trait StateTransitionStructureKnownInStateValidationV0 {
    /// Validates the structure of a transaction by checking its basic elements.
    ///
    /// # Arguments
    ///
    /// * `platform` - A reference to the platform state ref.
    /// * `action` - An optional reference to the state transition action.
    /// * `platform_version` - The platform version.
    ///
    /// # Returns
    ///
    /// * `Result<SimpleConsensusValidationResult, Error>` - A result with either a SimpleConsensusValidationResult or an Error.
    fn validate_advanced_structure_from_state(
        &self,
        platform: &PlatformStateRef,
        action: &StateTransitionAction,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;

    /// This means we should transform into the action before validation of the structure
    fn requires_advance_structure_validation_from_state(&self) -> bool {
        false
    }
}

/// A trait for validating state transitions within a blockchain.
pub(crate) trait StateTransitionBalanceValidationV0 {
    /// Validates the state transition by analyzing the changes in the platform state after applying the transaction.
    ///
    /// # Arguments
    ///
    /// * `platform` - A reference to the platform containing the state data.
    /// * `tx` - The transaction argument to be applied.
    ///
    /// # Type Parameters
    ///
    /// * `C: CoreRPCLike` - A type constraint indicating that C should implement `CoreRPCLike`.
    ///
    /// # Returns
    ///
    /// * `Result<ConsensusValidationResult<StateTransitionAction>, Error>` - A result with either a ConsensusValidationResult containing a StateTransitionAction or an Error.
    fn validate_balance(
        &self,
        identity: Option<&mut PartialIdentity>,
        platform: &PlatformStateRef,
        block_info: &BlockInfo,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;

    /// True if the state transition has a balance validation.
    /// This balance validation is not for the operations of the state transition, but more as a
    /// quick early verification that the user has the balance they want to transfer or withdraw.
    fn has_balance_validation(&self) -> bool {
        true
    }
}

/// A trait for validating state transitions within a blockchain.
pub(crate) trait StateTransitionStateValidationV0:
    StateTransitionActionTransformerV0
{
    /// Validates the state transition by analyzing the changes in the platform state after applying the transaction.
    ///
    /// # Arguments
    ///
    /// * `platform` - A reference to the platform containing the state data.
    /// * `tx` - The transaction argument to be applied.
    ///
    /// # Type Parameters
    ///
    /// * `C: CoreRPCLike` - A type constraint indicating that C should implement `CoreRPCLike`.
    ///
    /// # Returns
    ///
    /// * `Result<ConsensusValidationResult<StateTransitionAction>, Error>` - A result with either a ConsensusValidationResult containing a StateTransitionAction or an Error.
    fn validate_state<C: CoreRPCLike>(
        &self,
        action: Option<StateTransitionAction>,
        platform: &PlatformRef<C>,
        validation_mode: ValidationMode,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl StateTransitionBasicStructureValidationV0 for StateTransition {
    fn validate_basic_structure(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        match self {
            StateTransition::DataContractCreate(st) => {
                st.validate_basic_structure(platform_version)
            }
            StateTransition::DataContractUpdate(_) => {
                // no basic structure validation
                Ok(SimpleConsensusValidationResult::new())
            }
            StateTransition::IdentityCreate(st) => st.validate_basic_structure(platform_version),
            StateTransition::IdentityUpdate(st) => st.validate_basic_structure(platform_version),
            StateTransition::IdentityTopUp(st) => st.validate_basic_structure(platform_version),
            StateTransition::IdentityCreditWithdrawal(st) => {
                st.validate_basic_structure(platform_version)
            }
            StateTransition::DocumentsBatch(st) => st.validate_basic_structure(platform_version),
            StateTransition::IdentityCreditTransfer(st) => {
                st.validate_basic_structure(platform_version)
            }
        }
    }
    fn has_basic_structure_validation(&self) -> bool {
        !matches!(self, StateTransition::DataContractUpdate(_))
    }
}

impl StateTransitionNonceValidationV0 for StateTransition {
    fn validate_nonces(
        &self,
        platform: &PlatformStateRef,
        block_info: &BlockInfo,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        match self {
            StateTransition::DocumentsBatch(st) => {
                st.validate_nonces(platform, block_info, tx, platform_version)
            }
            StateTransition::DataContractCreate(st) => {
                st.validate_nonces(platform, block_info, tx, platform_version)
            }
            StateTransition::DataContractUpdate(st) => {
                st.validate_nonces(platform, block_info, tx, platform_version)
            }
            StateTransition::IdentityUpdate(st) => {
                st.validate_nonces(platform, block_info, tx, platform_version)
            }
            StateTransition::IdentityCreditTransfer(st) => {
                st.validate_nonces(platform, block_info, tx, platform_version)
            }
            StateTransition::IdentityCreditWithdrawal(st) => {
                st.validate_nonces(platform, block_info, tx, platform_version)
            }
            _ => Ok(SimpleConsensusValidationResult::new()),
        }
    }

    fn has_nonces_validation(&self) -> bool {
        matches!(
            self,
            StateTransition::DocumentsBatch(_)
                | StateTransition::DataContractCreate(_)
                | StateTransition::DataContractUpdate(_)
                | StateTransition::IdentityUpdate(_)
                | StateTransition::IdentityCreditTransfer(_)
                | StateTransition::IdentityCreditWithdrawal(_)
        )
    }
}

impl StateTransitionBalanceValidationV0 for StateTransition {
    fn validate_balance(
        &self,
        identity: Option<&mut PartialIdentity>,
        platform: &PlatformStateRef,
        block_info: &BlockInfo,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        match self {
            StateTransition::IdentityCreditTransfer(st) => {
                st.validate_balance(identity, platform, block_info, tx, platform_version)
            }
            StateTransition::IdentityCreditWithdrawal(st) => {
                st.validate_balance(identity, platform, block_info, tx, platform_version)
            }
            _ => Ok(SimpleConsensusValidationResult::new()),
        }
    }

    fn has_balance_validation(&self) -> bool {
        matches!(
            self,
            StateTransition::IdentityCreditTransfer(_)
                | StateTransition::IdentityCreditWithdrawal(_)
        )
    }
}

impl StateTransitionAdvancedStructureValidationV0 for StateTransition {
    fn has_advanced_structure_validation(&self) -> bool {
        false
    }

    fn validate_advanced_structure(
        &self,
        _platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        Ok(ConsensusValidationResult::<StateTransitionAction>::new())
    }
}

impl StateTransitionStructureKnownInStateValidationV0 for StateTransition {
    fn validate_advanced_structure_from_state(
        &self,
        platform: &PlatformStateRef,
        action: &StateTransitionAction,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        match self {
            StateTransition::DocumentsBatch(st) => {
                st.validate_advanced_structure_from_state(platform, action, platform_version)
            }
            _ => Ok(SimpleConsensusValidationResult::new()),
        }
    }

    /// This means we should transform into the action before validation of the structure
    fn requires_advance_structure_validation_from_state(&self) -> bool {
        matches!(self, StateTransition::DocumentsBatch(_))
    }
}

impl StateTransitionSignatureValidationV0 for StateTransition {
    fn validate_identity_and_signatures(
        &self,
        drive: &Drive,
        action: Option<&StateTransitionAction>,
        tx: TransactionArg,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<Option<PartialIdentity>>, Error> {
        match self {
            StateTransition::DataContractCreate(_)
            | StateTransition::DataContractUpdate(_)
            | StateTransition::IdentityCreditWithdrawal(_)
            | StateTransition::IdentityCreditTransfer(_)
            | StateTransition::DocumentsBatch(_) => {
                //Basic signature verification
                Ok(self
                    .validate_state_transition_identity_signed(
                        drive,
                        action,
                        false,
                        tx,
                        execution_context,
                        platform_version,
                    )?
                    .map(Some))
            }
            StateTransition::IdentityUpdate(st) => {
                match platform_version
                    .drive_abci
                    .validation_and_processing
                    .state_transitions
                    .identity_update_state_transition
                    .identity_signatures
                {
                    Some(0) => {
                        let signable_bytes: Vec<u8> = self.signable_bytes()?;
                        let mut validation_result = self
                            .validate_state_transition_identity_signed(
                                drive,
                                action,
                                true,
                                tx,
                                execution_context,
                                platform_version,
                            )?;
                        if !validation_result.is_valid() {
                            Ok(validation_result.map(Some))
                        } else {
                            let partial_identity = validation_result.data_as_borrowed()?;
                            let result = st
                                .validate_identity_update_state_transition_signatures_v0(
                                    signable_bytes,
                                    partial_identity,
                                    execution_context,
                                )?;
                            validation_result.merge(result);
                            Ok(validation_result.map(Some))
                        }
                    }
                    None => Err(Error::Execution(ExecutionError::VersionNotActive {
                        method: "identity update transition: validate_identity_and_signatures"
                            .to_string(),
                        known_versions: vec![0],
                    })),
                    Some(version) => {
                        Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                            method: "identity update transition: validate_identity_and_signatures"
                                .to_string(),
                            known_versions: vec![0],
                            received: version,
                        }))
                    }
                }
            }
            StateTransition::IdentityCreate(st) => {
                match platform_version
                    .drive_abci
                    .validation_and_processing
                    .state_transitions
                    .identity_create_state_transition
                    .identity_signatures
                {
                    Some(0) => {
                        let mut validation_result =
                            ConsensusValidationResult::<Option<PartialIdentity>>::default();

                        let signable_bytes: Vec<u8> = self.signable_bytes()?;

                        let result = st.validate_identity_create_state_transition_signatures_v0(
                            signable_bytes,
                            execution_context,
                        )?;

                        validation_result.merge(result);
                        validation_result.set_data(None);

                        Ok(validation_result)
                    }
                    None => Err(Error::Execution(ExecutionError::VersionNotActive {
                        method: "identity create transition: validate_identity_and_signatures"
                            .to_string(),
                        known_versions: vec![0],
                    })),
                    Some(version) => {
                        Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                            method: "identity create transition: validate_identity_and_signatures"
                                .to_string(),
                            known_versions: vec![0],
                            received: version,
                        }))
                    }
                }
            }
            StateTransition::IdentityTopUp(st) => {
                match platform_version
                    .drive_abci
                    .validation_and_processing
                    .state_transitions
                    .identity_top_up_state_transition
                    .identity_signatures
                {
                    // The validation of the signature happens on the state level
                    Some(0) => Ok(st
                        .retrieve_topped_up_identity(drive, tx, platform_version)?
                        .map(Some)),
                    None => Err(Error::Execution(ExecutionError::VersionNotActive {
                        method: "identity top up transition: validate_identity_and_signatures"
                            .to_string(),
                        known_versions: vec![0],
                    })),
                    Some(version) => {
                        Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                            method: "identity top up transition: validate_identity_and_signatures"
                                .to_string(),
                            known_versions: vec![0],
                            received: version,
                        }))
                    }
                }
            }
        }
    }

    /// This means we should transform into the action before validation of the identity and signatures
    fn requires_state_to_validate_identity_and_signatures(&self) -> bool {
        matches!(self, StateTransition::DocumentsBatch(_))
    }
}

impl StateTransitionStateValidationV0 for StateTransition {
    fn validate_state<C: CoreRPCLike>(
        &self,
        action: Option<StateTransitionAction>,
        platform: &PlatformRef<C>,
        validation_mode: ValidationMode,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        match self {
            // The replay attack is prevented by checking if a data contract exists with this id first
            StateTransition::DataContractCreate(st) => {
                st.validate_state(action, platform, validation_mode, execution_context, tx)
            }
            // The replay attack is prevented by identity data contract nonce
            StateTransition::DataContractUpdate(st) => {
                st.validate_state(action, platform, validation_mode, execution_context, tx)
            }
            StateTransition::IdentityCreate(st) => {
                st.validate_state(action, platform, validation_mode, execution_context, tx)
            }
            StateTransition::IdentityUpdate(st) => {
                st.validate_state(action, platform, validation_mode, execution_context, tx)
            }
            StateTransition::IdentityTopUp(st) => {
                st.validate_state(action, platform, validation_mode, execution_context, tx)
            }
            StateTransition::IdentityCreditWithdrawal(st) => {
                st.validate_state(action, platform, validation_mode, execution_context, tx)
            }
            // The replay attack is prevented by identity data contract nonce
            StateTransition::DocumentsBatch(st) => {
                st.validate_state(action, platform, validation_mode, execution_context, tx)
            }
            StateTransition::IdentityCreditTransfer(st) => {
                st.validate_state(action, platform, validation_mode, execution_context, tx)
            }
        }
    }
}
