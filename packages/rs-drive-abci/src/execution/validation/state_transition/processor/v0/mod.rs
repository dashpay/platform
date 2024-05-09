use crate::error::Error;
use crate::execution::types::execution_event::ExecutionEvent;
use crate::execution::validation::state_transition::transformer::StateTransitionActionTransformerV0;
use crate::platform_types::platform::{PlatformRef, PlatformStateRef};
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::block::epoch::Epoch;
use dpp::identity::PartialIdentity;
use dpp::prelude::ConsensusValidationResult;
use dpp::ProtocolError;

use crate::error::execution::ExecutionError;
use dpp::serialization::Signable;
use dpp::state_transition::StateTransition;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::{DefaultForPlatformVersion, PlatformVersion};
use drive::drive::Drive;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::StateTransitionAction;

use crate::execution::types::state_transition_execution_context::{StateTransitionExecutionContext};
use crate::execution::validation::state_transition::common::validate_simple_pre_check_balance::ValidateSimplePreCheckBalance;
use crate::execution::validation::state_transition::common::validate_state_transition_identity_signed::{ValidateStateTransitionIdentitySignature};
use crate::execution::validation::state_transition::identity_create::{StateTransitionStateValidationForIdentityCreateTransitionV0, StateTransitionStructureKnownInStateValidationForIdentityCreateTransitionV0};
use crate::execution::validation::state_transition::identity_top_up::StateTransitionIdentityTopUpTransitionActionTransformer;
use crate::execution::validation::state_transition::state_transitions::identity_update::advanced_structure::v0::IdentityUpdateStateTransitionIdentityAndSignaturesValidationV0;
use crate::execution::validation::state_transition::state_transitions::identity_top_up::identity_retrieval::v0::IdentityTopUpStateTransitionIdentityRetrievalV0;
use crate::execution::validation::state_transition::ValidationMode;
pub(super) fn process_state_transition_v0<'a, C: CoreRPCLike>(
    platform: &'a PlatformRef<C>,
    block_info: &BlockInfo,
    state_transition: StateTransition,
    transaction: TransactionArg,
    platform_version: &PlatformVersion,
) -> Result<ConsensusValidationResult<ExecutionEvent<'a>>, Error> {
    let mut state_transition_execution_context =
        StateTransitionExecutionContext::default_for_platform_version(platform_version)?;

    // Only identity create does not use identity in state validation, because it doesn't yet have the identity in state
    let mut maybe_identity = if state_transition.uses_identity_in_state() {
        // Validating signature for identity based state transitions (all those except identity create and identity top up)
        // As we already have removed identity create above, it just splits between identity top up (below - false) and
        // all other state transitions (above - true)
        let result = if state_transition.validates_signature_based_on_identity_info() {
            state_transition.validate_identity_signed_state_transition(
                platform.drive,
                transaction,
                &mut state_transition_execution_context,
                platform_version,
            )
        } else {
            // Currently only identity top up uses this,
            // We will add the cost for a balance retrieval
            state_transition.retrieve_identity_info(
                platform.drive,
                transaction,
                &mut state_transition_execution_context,
                platform_version,
            )
        }?;
        if !result.is_valid() {
            // If the signature is not valid or if we could not retrieve identity info
            // we do not have the user pay for the state transition.
            // Since it is most likely not from them
            // Proposers should remove such transactions from the block
            // Other validators should reject blocks with such transactions
            return Ok(ConsensusValidationResult::<ExecutionEvent>::new_with_errors(result.errors));
        }
        Some(result.into_data()?)
    } else {
        // Currently only identity create
        None
    };

    // Only identity top up and identity create do not have nonces validation
    if state_transition.has_nonces_validation() {
        // Validating identity contract nonce, this must happen after validating the signature
        let result = state_transition.validate_nonces(
            &platform.into(),
            platform.state.last_block_info(),
            transaction,
            &mut state_transition_execution_context,
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

    // Only Data contract state transitions do not have basic structure validation
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

    // For identity credit withdrawal and identity credit transfers we have a balance pre check that includes a
    // processing amount and the transfer amount.
    // For other state transitions we only check a min balance for an amount set per version.
    // This is not done for identity create and identity top up who don't have this check here
    if state_transition.has_balance_pre_check_validation() {
        // Validating that we have sufficient balance for a transfer or withdrawal,
        // this must happen after validating the signature

        let identity = maybe_identity
            .as_mut()
            .ok_or(ProtocolError::CorruptedCodeExecution(
                "identity must be known to validate the balance".to_string(),
            ))?;
        let result =
            state_transition.validate_minimum_balance_pre_check(identity, platform_version)?;

        if !result.is_valid() {
            return Ok(ConsensusValidationResult::<ExecutionEvent>::new_with_errors(result.errors));
        }
    }

    // Only identity update and data contract create have advanced structure validation without state
    if state_transition.has_advanced_structure_validation_without_state() {
        // Currently only used for Identity Update
        // Next we have advanced structure validation, this is structure validation that does not require
        // state but isn't checked on check_tx. If advanced structure fails identity nonces or identity
        // contract nonces will be bumped
        let identity = maybe_identity
            .as_ref()
            .ok_or(ProtocolError::CorruptedCodeExecution(
                "the identity should always be known on advanced structure validation".to_string(),
            ))?;
        let consensus_result = state_transition.validate_advanced_structure(
            identity,
            &mut state_transition_execution_context,
            platform_version,
        )?;

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

    // Identity create and documents batch both have advanced structure validation with state
    let action = if state_transition.has_advanced_structure_validation_with_state() {
        // Currently used for identity create and documents batch
        let state_transition_action_result = state_transition.transform_into_action(
            platform,
            block_info,
            ValidationMode::Validator,
            &mut state_transition_execution_context,
            transaction,
        )?;
        if !state_transition_action_result.is_valid_with_data() {
            return state_transition_action_result.map_result(|action| {
                ExecutionEvent::create_from_state_transition_action(
                    action,
                    maybe_identity,
                    platform.state.last_committed_block_epoch_ref(),
                    state_transition_execution_context,
                    platform_version,
                )
            });
        }
        let action = state_transition_action_result.into_data()?;

        // Validating structure
        let result = state_transition.validate_advanced_structure_from_state(
            &action,
            maybe_identity.as_ref(),
            &mut state_transition_execution_context,
            platform_version,
        )?;
        if !result.is_valid() {
            return result.map_result(|action| {
                ExecutionEvent::create_from_state_transition_action(
                    action,
                    maybe_identity,
                    platform.state.last_committed_block_epoch_ref(),
                    state_transition_execution_context,
                    platform_version,
                )
            });
        }

        Some(action)
    } else {
        None
    };

    // Validating state
    // Only identity Top up does not validate state and instead just returns the action for topping up
    let result = state_transition.validate_state(
        action,
        platform,
        ValidationMode::Validator,
        &block_info.epoch,
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
pub(crate) trait StateTransitionIdentityBasedSignatureValidationV0 {
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
    fn validate_identity_signed_state_transition(
        &self,
        drive: &Drive,
        tx: TransactionArg,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<PartialIdentity>, Error>;

    /// fetches identity info
    fn retrieve_identity_info(
        &self,
        drive: &Drive,
        tx: TransactionArg,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<PartialIdentity>, Error>;

    /// Is the state transition supposed to have an identity in the state to succeed
    fn uses_identity_in_state(&self) -> bool;

    /// Do we validate the signature based on identity info?
    fn validates_signature_based_on_identity_info(&self) -> bool;
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
        identity: &PartialIdentity,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;

    /// True if the state transition has advanced structure validation.
    /// This structure validation makes users pay if there is a failure
    fn has_advanced_structure_validation_without_state(&self) -> bool;
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
        execution_context: &mut StateTransitionExecutionContext,
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
    /// * `action` - An optional reference to the state transition action.
    /// * `platform_version` - The platform version.
    ///
    /// # Returns
    ///
    /// * `Result<SimpleConsensusValidationResult, Error>` - A result with either a SimpleConsensusValidationResult or an Error.
    fn validate_advanced_structure_from_state(
        &self,
        action: &StateTransitionAction,
        maybe_identity: Option<&PartialIdentity>,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;

    /// This means we should transform into the action before validation of the structure
    fn has_advanced_structure_validation_with_state(&self) -> bool;
    /// This means we should transform into the action before validation of the advanced structure,
    /// and that we must even do this on check_tx
    fn requires_advanced_structure_validation_with_state_on_check_tx(&self) -> bool;
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
    fn validate_minimum_balance_pre_check(
        &self,
        identity: &PartialIdentity,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;

    /// True if the state transition has a balance validation.
    /// This balance validation is not for the operations of the state transition, but more as a
    /// quick early verification that the user has the balance they want to transfer or withdraw.
    fn has_balance_pre_check_validation(&self) -> bool {
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
        epoch: &Epoch,
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
            StateTransition::DataContractCreate(_) | StateTransition::DataContractUpdate(_) => {
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
        !matches!(
            self,
            StateTransition::DataContractCreate(_) | StateTransition::DataContractUpdate(_)
        )
    }
}

impl StateTransitionNonceValidationV0 for StateTransition {
    fn validate_nonces(
        &self,
        platform: &PlatformStateRef,
        block_info: &BlockInfo,
        tx: TransactionArg,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        match self {
            StateTransition::DocumentsBatch(st) => st.validate_nonces(
                platform,
                block_info,
                tx,
                execution_context,
                platform_version,
            ),
            StateTransition::DataContractCreate(st) => st.validate_nonces(
                platform,
                block_info,
                tx,
                execution_context,
                platform_version,
            ),
            StateTransition::DataContractUpdate(st) => st.validate_nonces(
                platform,
                block_info,
                tx,
                execution_context,
                platform_version,
            ),
            StateTransition::IdentityUpdate(st) => st.validate_nonces(
                platform,
                block_info,
                tx,
                execution_context,
                platform_version,
            ),
            StateTransition::IdentityCreditTransfer(st) => st.validate_nonces(
                platform,
                block_info,
                tx,
                execution_context,
                platform_version,
            ),
            StateTransition::IdentityCreditWithdrawal(st) => st.validate_nonces(
                platform,
                block_info,
                tx,
                execution_context,
                platform_version,
            ),
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
    fn validate_minimum_balance_pre_check(
        &self,
        identity: &PartialIdentity,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        match self {
            StateTransition::IdentityCreditTransfer(st) => {
                st.validate_minimum_balance_pre_check(identity, platform_version)
            }
            StateTransition::IdentityCreditWithdrawal(st) => {
                st.validate_minimum_balance_pre_check(identity, platform_version)
            }
            StateTransition::DocumentsBatch(st) => {
                st.validate_minimum_balance_pre_check(identity, platform_version)
            }
            StateTransition::DataContractCreate(_)
            | StateTransition::DataContractUpdate(_)
            | StateTransition::IdentityUpdate(_) => {
                self.validate_simple_pre_check_minimum_balance(identity, platform_version)
            }
            StateTransition::IdentityCreate(_) | StateTransition::IdentityTopUp(_) => {
                Ok(SimpleConsensusValidationResult::new())
            }
        }
    }

    fn has_balance_pre_check_validation(&self) -> bool {
        matches!(
            self,
            StateTransition::IdentityCreditTransfer(_)
                | StateTransition::IdentityCreditWithdrawal(_)
                | StateTransition::DataContractCreate(_)
                | StateTransition::DataContractUpdate(_)
                | StateTransition::DocumentsBatch(_)
                | StateTransition::IdentityUpdate(_)
        )
    }
}

impl StateTransitionAdvancedStructureValidationV0 for StateTransition {
    fn validate_advanced_structure(
        &self,
        identity: &PartialIdentity,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        match self {
            StateTransition::IdentityUpdate(st) => {
                match platform_version
                    .drive_abci
                    .validation_and_processing
                    .state_transitions
                    .identity_update_state_transition
                    .advanced_structure
                {
                    Some(0) => {
                        let signable_bytes: Vec<u8> = self.signable_bytes()?;
                        st.validate_identity_update_state_transition_signatures_v0(
                            signable_bytes,
                            identity,
                            execution_context,
                        )
                    }
                    Some(version) => {
                        Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                            method: "identity update transition: validate_advanced_structure"
                                .to_string(),
                            known_versions: vec![0],
                            received: version,
                        }))
                    }
                    None => Err(Error::Execution(ExecutionError::VersionNotActive {
                        method: "identity update transition: validate_advanced_structure"
                            .to_string(),
                        known_versions: vec![0],
                    })),
                }
            }
            StateTransition::DataContractCreate(st) => {
                st.validate_advanced_structure(identity, execution_context, platform_version)
            }
            _ => Ok(ConsensusValidationResult::<StateTransitionAction>::new()),
        }
    }

    fn has_advanced_structure_validation_without_state(&self) -> bool {
        matches!(
            self,
            StateTransition::IdentityUpdate(_) | StateTransition::DataContractCreate(_)
        )
    }
}

impl StateTransitionStructureKnownInStateValidationV0 for StateTransition {
    fn validate_advanced_structure_from_state(
        &self,
        action: &StateTransitionAction,
        maybe_identity: Option<&PartialIdentity>,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        match self {
            StateTransition::DocumentsBatch(st) => st.validate_advanced_structure_from_state(
                action,
                maybe_identity,
                execution_context,
                platform_version,
            ),
            StateTransition::IdentityCreate(st) => {
                let signable_bytes = self.signable_bytes()?;
                let StateTransitionAction::IdentityCreateAction(identity_create_action) = action
                else {
                    return Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "action must be a identity create transition action",
                    )));
                };
                st.validate_advanced_structure_from_state_for_identity_create_transition(
                    identity_create_action,
                    signable_bytes,
                    execution_context,
                    platform_version,
                )
            }
            _ => Ok(ConsensusValidationResult::new()),
        }
    }

    /// This means we should transform into the action before validation of the advanced structure
    fn has_advanced_structure_validation_with_state(&self) -> bool {
        matches!(
            self,
            StateTransition::DocumentsBatch(_) | StateTransition::IdentityCreate(_)
        )
    }

    /// This means we should transform into the action before validation of the advanced structure,
    /// and that we must even do this on check_tx
    fn requires_advanced_structure_validation_with_state_on_check_tx(&self) -> bool {
        matches!(self, StateTransition::DocumentsBatch(_))
    }
}

impl StateTransitionIdentityBasedSignatureValidationV0 for StateTransition {
    fn validate_identity_signed_state_transition(
        &self,
        drive: &Drive,
        tx: TransactionArg,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<PartialIdentity>, Error> {
        match self {
            StateTransition::DataContractCreate(_)
            | StateTransition::DataContractUpdate(_)
            | StateTransition::IdentityCreditWithdrawal(_)
            | StateTransition::IdentityCreditTransfer(_)
            | StateTransition::DocumentsBatch(_) => {
                //Basic signature verification
                Ok(self.validate_state_transition_identity_signed(
                    drive,
                    false,
                    tx,
                    execution_context,
                    platform_version,
                )?)
            }
            StateTransition::IdentityUpdate(_) => {
                //Basic signature verification
                Ok(self.validate_state_transition_identity_signed(
                    drive,
                    true,
                    tx,
                    execution_context,
                    platform_version,
                )?)
            }
            StateTransition::IdentityCreate(_) => Ok(ConsensusValidationResult::new()),
            StateTransition::IdentityTopUp(_) => Ok(ConsensusValidationResult::new()),
        }
    }

    fn retrieve_identity_info(
        &self,
        drive: &Drive,
        tx: TransactionArg,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<PartialIdentity>, Error> {
        match self {
            StateTransition::IdentityTopUp(st) => Ok(st.retrieve_topped_up_identity(
                drive,
                tx,
                execution_context,
                platform_version,
            )?),
            _ => Ok(ConsensusValidationResult::new()),
        }
    }

    /// Is the state transition supposed to have an identity in the state to succeed
    fn uses_identity_in_state(&self) -> bool {
        !matches!(self, StateTransition::IdentityCreate(_))
    }

    /// Do we validate the signature based on identity info?
    fn validates_signature_based_on_identity_info(&self) -> bool {
        !matches!(
            self,
            StateTransition::IdentityCreate(_) | StateTransition::IdentityTopUp(_)
        )
    }
}

impl StateTransitionStateValidationV0 for StateTransition {
    fn validate_state<C: CoreRPCLike>(
        &self,
        action: Option<StateTransitionAction>,
        platform: &PlatformRef<C>,
        validation_mode: ValidationMode,
        epoch: &Epoch,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        match self {
            // The replay attack is prevented by checking if a data contract exists with this id first
            StateTransition::DataContractCreate(st) => st.validate_state(
                action,
                platform,
                validation_mode,
                epoch,
                execution_context,
                tx,
            ),
            // The replay attack is prevented by identity data contract nonce
            StateTransition::DataContractUpdate(st) => st.validate_state(
                action,
                platform,
                validation_mode,
                epoch,
                execution_context,
                tx,
            ),
            StateTransition::IdentityCreate(st) => {
                let action =
                    action.ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "identity create validation should always an action",
                    )))?;
                let StateTransitionAction::IdentityCreateAction(action) = action else {
                    return Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "action must be a identity create transition action",
                    )));
                };
                st.validate_state_for_identity_create_transition(
                    action,
                    platform,
                    execution_context,
                    tx,
                )
            }
            StateTransition::IdentityUpdate(st) => st.validate_state(
                action,
                platform,
                validation_mode,
                epoch,
                execution_context,
                tx,
            ),
            StateTransition::IdentityTopUp(st) => {
                // Nothing to validate from state
                if let Some(action) = action {
                    Ok(ConsensusValidationResult::new_with_data(action))
                } else {
                    let signable_bytes = self.signable_bytes()?;
                    st.transform_into_action_for_identity_top_up_transition(
                        platform,
                        signable_bytes,
                        validation_mode,
                        execution_context,
                        tx,
                    )
                }
            }
            StateTransition::IdentityCreditWithdrawal(st) => st.validate_state(
                action,
                platform,
                validation_mode,
                epoch,
                execution_context,
                tx,
            ),
            // The replay attack is prevented by identity data contract nonce
            StateTransition::DocumentsBatch(st) => st.validate_state(
                action,
                platform,
                validation_mode,
                epoch,
                execution_context,
                tx,
            ),
            StateTransition::IdentityCreditTransfer(st) => st.validate_state(
                action,
                platform,
                validation_mode,
                epoch,
                execution_context,
                tx,
            ),
        }
    }
}
