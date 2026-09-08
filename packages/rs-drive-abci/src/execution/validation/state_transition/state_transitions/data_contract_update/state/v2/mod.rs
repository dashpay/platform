use std::collections::BTreeSet;
use std::sync::Arc;

use dpp::block::block_info::BlockInfo;
use dpp::consensus::basic::document::DataContractNotPresentError;
use dpp::consensus::basic::BasicError;
use dpp::consensus::state::data_contract::data_contract_not_found_error::DataContractNotFoundError;
use dpp::consensus::state::group::IdentityMemberOfGroupNotFoundError;
use dpp::consensus::state::identity::identity_for_token_configuration_not_found_error::{
    IdentityInTokenConfigurationNotFoundError, TokenConfigurationIdentityContext,
};
use dpp::consensus::state::state_error::StateError;
use dpp::consensus::state::token::InvalidTokenPositionStateError;
use dpp::consensus::ConsensusError;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::accessors::v1::DataContractV1Getters;
use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
use dpp::data_contract::associated_token::token_distribution_rules::accessors::v0::TokenDistributionRulesV0Getters;
use dpp::data_contract::associated_token::token_perpetual_distribution::distribution_recipient::TokenDistributionRecipient;
use dpp::data_contract::associated_token::token_perpetual_distribution::methods::v0::TokenPerpetualDistributionV0Accessors;
use dpp::data_contract::associated_token::token_pre_programmed_distribution::accessors::v0::TokenPreProgrammedDistributionV0Methods;
use dpp::data_contract::change_control_rules::authorized_action_takers::AuthorizedActionTakers;
use dpp::data_contract::document_type::accessors::DocumentTypeV1Getters;
use dpp::data_contract::group::accessors::v0::GroupV0Getters;
use dpp::data_contract::validate_update::DataContractUpdateValidationMethodsV0;
use dpp::data_contract::DataContract;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::data_contract_update_transition::{
    DataContractUpdateTransition, DataContractUpdateTransitionV1,
};
use dpp::version::PlatformVersion;
use dpp::ProtocolError;
use drive::drive::contract::DataContractFetchInfo;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::contract::data_contract_update::DataContractUpdateTransitionAction;
use drive::state_transition_action::system::bump_identity_data_contract_nonce_action::BumpIdentityDataContractNonceAction;
use drive::state_transition_action::StateTransitionAction;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};
use crate::execution::validation::state_transition::common::validate_identity_exists::validate_identity_exists;
use crate::execution::validation::state_transition::common::validate_non_masternode_identity_exists::validate_non_masternode_identity_exists;
use crate::execution::validation::state_transition::data_contract_common::data_contract_reference_validation::validate_data_contract_references;
use crate::execution::validation::state_transition::state_transitions::data_contract_update::state::v0::DataContractUpdateStateTransitionStateValidationV0;
use crate::execution::validation::state_transition::state_transitions::data_contract_update::state::v1::DataContractUpdateStateTransitionStateValidationV1;
use crate::execution::validation::state_transition::ValidationMode;
use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;

pub(in crate::execution::validation::state_transition::state_transitions::data_contract_update) trait DataContractUpdateStateTransitionStateValidationV2
{
    fn validate_state_v2<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        block_info: &BlockInfo,
        validation_mode: ValidationMode,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;

    fn transform_into_action_v1<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        block_info: &BlockInfo,
        validation_mode: ValidationMode,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

/// What merging a delta-based update onto the stored contract produced.
enum DeltaApplication {
    /// The update was rejected: the result carries the nonce bump and the errors.
    Rejected(ConsensusValidationResult<StateTransitionAction>),
    /// The merged contract, ready for the update rules.
    Applied {
        action: DataContractUpdateTransitionAction,
        old_data_contract: Arc<DataContractFetchInfo>,
    },
}

fn bump_nonce(
    transition: &DataContractUpdateTransition,
    errors: Vec<ConsensusError>,
) -> ConsensusValidationResult<StateTransitionAction> {
    ConsensusValidationResult::new_with_data_and_errors(
        StateTransitionAction::BumpIdentityDataContractNonceAction(
            BumpIdentityDataContractNonceAction::from_borrowed_data_contract_update_transition(
                transition,
            ),
        ),
        errors,
    )
}

/// Fetches the contract a delta-based update targets and merges the delta
/// onto it.
///
/// The fetch is charged whether or not the contract exists. A missing
/// contract and every check the delta shape itself allows (ownership,
/// updated entries exist, new entries do not) reject the update with a
/// nonce bump, exactly like the full-contract path rejects its failures.
#[allow(clippy::too_many_arguments)]
fn apply_delta<C: CoreRPCLike>(
    transition: &DataContractUpdateTransition,
    delta: &DataContractUpdateTransitionV1,
    platform: &PlatformRef<C>,
    block_info: &BlockInfo,
    validation_mode: ValidationMode,
    execution_context: &mut StateTransitionExecutionContext,
    tx: TransactionArg,
    platform_version: &PlatformVersion,
) -> Result<DeltaApplication, Error> {
    let (fee, contract_fetch_info) = platform.drive.get_contract_with_fetch_info_and_fee(
        delta.data_contract_id.to_buffer(),
        Some(&block_info.epoch),
        validation_mode.can_alter_cache(),
        tx,
        platform_version,
    )?;

    let fee = fee.ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution(
        "fee must exist in validate state for data contract update transition",
    )))?;

    // The fetch is paid for even when the contract does not exist or came from cache
    execution_context.add_operation(ValidationOperation::PrecalculatedOperation(fee));

    let Some(old_data_contract) = contract_fetch_info else {
        return Ok(DeltaApplication::Rejected(bump_nonce(
            transition,
            vec![
                BasicError::DataContractNotPresentError(DataContractNotPresentError::new(
                    delta.data_contract_id,
                ))
                .into(),
            ],
        )));
    };

    let mut validation_operations = vec![];

    let result = DataContractUpdateTransitionAction::try_from_borrowed_v1_transition(
        delta,
        &old_data_contract.contract,
        block_info,
        validation_mode.should_fully_validate_contract_on_transform_into_action(),
        &mut validation_operations,
        platform_version,
    );

    execution_context.add_dpp_operations(validation_operations);

    match result {
        // Rebuilding the merged contract fails the same way a malformed
        // embedded contract fails: as a consensus error
        Err(ProtocolError::ConsensusError(consensus_error)) => Ok(DeltaApplication::Rejected(
            bump_nonce(transition, vec![*consensus_error]),
        )),
        Err(protocol_error) => Err(protocol_error.into()),
        Ok(validation_result) => {
            if validation_result.is_valid() {
                Ok(DeltaApplication::Applied {
                    action: validation_result.into_data()?,
                    old_data_contract,
                })
            } else {
                Ok(DeltaApplication::Rejected(bump_nonce(
                    transition,
                    validation_result.errors,
                )))
            }
        }
    }
}

/// Every identity a new group or a new token names must exist.
///
/// Returns the rejecting result when one does not.
#[allow(clippy::too_many_arguments)]
fn validate_new_group_and_token_identities<C: CoreRPCLike>(
    transition: &DataContractUpdateTransition,
    delta: &DataContractUpdateTransitionV1,
    old_data_contract: &DataContract,
    new_data_contract: &DataContract,
    platform: &PlatformRef<C>,
    execution_context: &mut StateTransitionExecutionContext,
    tx: TransactionArg,
    platform_version: &PlatformVersion,
) -> Result<Option<ConsensusValidationResult<StateTransitionAction>>, Error> {
    let contract_id = new_data_contract.id();
    let mut validated_identities = BTreeSet::new();

    for (position, group) in &delta.new_groups {
        for member_identity_id in group.members().keys() {
            if validated_identities.contains(member_identity_id) {
                continue;
            }
            let identity_exists = validate_non_masternode_identity_exists(
                platform.drive,
                member_identity_id,
                execution_context,
                tx,
                platform_version,
            )?;
            if !identity_exists {
                return Ok(Some(bump_nonce(
                    transition,
                    vec![StateError::IdentityMemberOfGroupNotFoundError(
                        IdentityMemberOfGroupNotFoundError::new(
                            contract_id,
                            *position,
                            *member_identity_id,
                        ),
                    )
                    .into()],
                )));
            }
            validated_identities.insert(*member_identity_id);
        }
    }

    for (token_contract_position, token_configuration) in new_data_contract.tokens() {
        if old_data_contract
            .tokens()
            .contains_key(token_contract_position)
        {
            continue;
        }

        let not_found = |context: TokenConfigurationIdentityContext, identity_id| {
            Some(bump_nonce(
                transition,
                vec![StateError::IdentityInTokenConfigurationNotFoundError(
                    IdentityInTokenConfigurationNotFoundError::new(
                        contract_id,
                        *token_contract_position,
                        context,
                        identity_id,
                    ),
                )
                .into()],
            ))
        };

        for (name, change_control_rules) in token_configuration.all_change_control_rules() {
            if let AuthorizedActionTakers::Identity(identity_id) =
                change_control_rules.authorized_to_make_change_action_takers()
            {
                if validated_identities.contains(identity_id) {
                    continue;
                }
                let identity_exists = validate_non_masternode_identity_exists(
                    platform.drive,
                    identity_id,
                    execution_context,
                    tx,
                    platform_version,
                )?;
                if !identity_exists {
                    return Ok(not_found(
                        TokenConfigurationIdentityContext::ChangeControlRule(name.to_string()),
                        *identity_id,
                    ));
                }
                validated_identities.insert(*identity_id);
            }
        }

        if let Some(distribution) = token_configuration
            .distribution_rules()
            .perpetual_distribution()
        {
            if let TokenDistributionRecipient::Identity(identifier) =
                distribution.distribution_recipient()
            {
                if !validated_identities.contains(&identifier) {
                    let identity_exists = validate_identity_exists(
                        platform.drive,
                        &identifier,
                        execution_context,
                        tx,
                        platform_version,
                    )?;
                    if !identity_exists {
                        return Ok(not_found(
                            TokenConfigurationIdentityContext::PerpetualDistributionRecipient,
                            identifier,
                        ));
                    }
                    validated_identities.insert(identifier);
                }
            }
        }

        if let Some(distributions) = token_configuration
            .distribution_rules()
            .pre_programmed_distribution()
        {
            for distribution in distributions.distributions().values() {
                for identifier in distribution.keys() {
                    if validated_identities.contains(identifier) {
                        continue;
                    }
                    let identity_exists = validate_identity_exists(
                        platform.drive,
                        identifier,
                        execution_context,
                        tx,
                        platform_version,
                    )?;
                    if !identity_exists {
                        return Ok(not_found(
                            TokenConfigurationIdentityContext::PreProgrammedDistributionRecipient,
                            *identifier,
                        ));
                    }
                    validated_identities.insert(*identifier);
                }
            }
        }

        // The minting recipient can be an evonode, so existence is checked by balance
        if let Some(minting_recipient) = token_configuration
            .distribution_rules()
            .new_tokens_destination_identity()
        {
            if !validated_identities.contains(minting_recipient) {
                let identity_exists = validate_identity_exists(
                    platform.drive,
                    minting_recipient,
                    execution_context,
                    tx,
                    platform_version,
                )?;
                if !identity_exists {
                    return Ok(not_found(
                        TokenConfigurationIdentityContext::DefaultMintingRecipient,
                        *minting_recipient,
                    ));
                }
                validated_identities.insert(*minting_recipient);
            }
        }
    }

    Ok(None)
}

/// Document types paying costs in another contract's tokens must name
/// tokens that exist.
///
/// Returns the rejecting result when one does not.
fn validate_external_token_costs<C: CoreRPCLike>(
    transition: &DataContractUpdateTransition,
    new_data_contract: &DataContract,
    platform: &PlatformRef<C>,
    block_info: &BlockInfo,
    execution_context: &mut StateTransitionExecutionContext,
    tx: TransactionArg,
    platform_version: &PlatformVersion,
) -> Result<Option<ConsensusValidationResult<StateTransitionAction>>, Error> {
    for document_type in new_data_contract.document_types().values() {
        for (contract_id, token_positions) in
            document_type.all_external_token_costs_contract_tokens()
        {
            let (fee, contract_fetch_info) = platform.drive.get_contract_with_fetch_info_and_fee(
                contract_id.to_buffer(),
                Some(&block_info.epoch),
                false,
                tx,
                platform_version,
            )?;

            let fee = fee.ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution(
                "fee must exist in validate state for data contract update transition",
            )))?;

            // The fetch is paid for even when the contract does not exist or came from cache
            execution_context.add_operation(ValidationOperation::PrecalculatedOperation(fee));

            let Some(fetch_info) = contract_fetch_info else {
                return Ok(Some(bump_nonce(
                    transition,
                    vec![
                        StateError::DataContractNotFoundError(DataContractNotFoundError::new(
                            contract_id,
                        ))
                        .into(),
                    ],
                )));
            };

            let contract_tokens = fetch_info.contract.tokens();
            if let Some(token_position) = token_positions
                .iter()
                .find(|token_position| !contract_tokens.contains_key(token_position))
            {
                return Ok(Some(bump_nonce(
                    transition,
                    vec![StateError::InvalidTokenPositionStateError(
                        InvalidTokenPositionStateError::new(
                            contract_tokens
                                .last_key_value()
                                .map(|(token_contract_position, _)| *token_contract_position),
                            *token_position,
                        ),
                    )
                    .into()],
                )));
            }
        }
    }

    Ok(None)
}

impl DataContractUpdateStateTransitionStateValidationV2 for DataContractUpdateTransition {
    /// Generation 2 adds delta-based (V1) updates. A full-contract (V0)
    /// update keeps its generation-1 validation.
    ///
    /// A delta is merged onto the stored contract first, and the merged
    /// contract is then held to exactly the checks a full-contract update
    /// gets: the update rules, the identities new groups and tokens name,
    /// external token costs, and reference declarations.
    fn validate_state_v2<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        block_info: &BlockInfo,
        validation_mode: ValidationMode,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let DataContractUpdateTransition::V1(delta) = self else {
            return self.validate_state_v1(
                platform,
                block_info,
                validation_mode,
                execution_context,
                tx,
                platform_version,
            );
        };

        let (action, old_data_contract) = match apply_delta(
            self,
            delta,
            platform,
            block_info,
            validation_mode,
            execution_context,
            tx,
            platform_version,
        )? {
            DeltaApplication::Rejected(result) => return Ok(result),
            DeltaApplication::Applied {
                action,
                old_data_contract,
            } => (action, old_data_contract),
        };

        let old_data_contract = &old_data_contract.contract;
        let new_data_contract = action.data_contract_ref();

        let validation_result =
            old_data_contract.validate_update(new_data_contract, block_info, platform_version)?;
        if !validation_result.is_valid() {
            return Ok(bump_nonce(self, validation_result.errors));
        }

        if let Some(result) = validate_new_group_and_token_identities(
            self,
            delta,
            old_data_contract,
            new_data_contract,
            platform,
            execution_context,
            tx,
            platform_version,
        )? {
            return Ok(result);
        }

        if let Some(result) = validate_external_token_costs(
            self,
            new_data_contract,
            platform,
            block_info,
            execution_context,
            tx,
            platform_version,
        )? {
            return Ok(result);
        }

        // New document types and properties may declare references
        let reference_result = validate_data_contract_references(
            new_data_contract,
            platform.drive,
            block_info,
            execution_context,
            tx,
            platform_version,
        )?;
        if !reference_result.is_valid() {
            return Ok(bump_nonce(self, reference_result.errors));
        }

        Ok(ConsensusValidationResult::new_with_data(action.into()))
    }

    /// Generation 1 adds delta-based (V1) updates, which need the stored
    /// contract to become an action. A full-contract (V0) update keeps its
    /// generation-0 transformation.
    fn transform_into_action_v1<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        block_info: &BlockInfo,
        validation_mode: ValidationMode,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let DataContractUpdateTransition::V1(delta) = self else {
            return self.transform_into_action_v0(
                block_info,
                validation_mode,
                execution_context,
                platform_version,
            );
        };

        match apply_delta(
            self,
            delta,
            platform,
            block_info,
            validation_mode,
            execution_context,
            tx,
            platform_version,
        )? {
            DeltaApplication::Rejected(result) => Ok(result),
            DeltaApplication::Applied { action, .. } => {
                Ok(ConsensusValidationResult::new_with_data(action.into()))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution::validation::state_transition::processor::state::StateTransitionStateValidation;
    use crate::execution::validation::state_transition::transformer::StateTransitionActionTransformer;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::{TempPlatform, TestPlatformBuilder};
    use assert_matches::assert_matches;
    use dpp::consensus::basic::data_contract::DataContractUpdateEntryKind;
    use dpp::data_contract::accessors::v0::DataContractV0Setters;
    use dpp::data_contract::accessors::v1::DataContractV1Setters;
    use dpp::data_contract::schema::DataContractSchemaMethodsV0;
    use dpp::platform_value::{platform_value, Identifier, Value};
    use dpp::tests::fixtures::get_data_contract_fixture;
    use drive::state_transition_action::system::bump_identity_data_contract_nonce_action::BumpIdentityDataContractNonceActionAccessorsV0;
    use platform_version::DefaultForPlatformVersion;

    fn setup() -> (TempPlatform<MockCoreRPCLike>, DataContract) {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();
        let data_contract = get_data_contract_fixture(None, 0, platform_version.protocol_version)
            .data_contract_owned();
        (platform, data_contract)
    }

    fn store(platform: &TempPlatform<MockCoreRPCLike>, data_contract: &DataContract) {
        platform
            .drive
            .apply_contract(
                data_contract,
                BlockInfo::default(),
                true,
                None,
                None,
                PlatformVersion::latest(),
            )
            .expect("expected to store the contract");
    }

    fn new_document_schema() -> Value {
        platform_value!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "position": 0
                }
            },
            "additionalProperties": false
        })
    }

    /// The fixture contract with a new document type, two keywords and a description.
    fn extended(data_contract: &DataContract) -> DataContract {
        let mut updated = data_contract.clone();
        updated.increment_version();
        updated
            .set_document_schema(
                "newType",
                new_document_schema(),
                true,
                &mut vec![],
                PlatformVersion::latest(),
            )
            .expect("expected to add a document type");
        updated.set_keywords(vec!["alpha".to_string(), "beta".to_string()]);
        updated.set_description(Some("a described contract".to_string()));
        updated
    }

    fn delta(
        old_contract: &DataContract,
        new_contract: &DataContract,
    ) -> DataContractUpdateTransition {
        let transition = DataContractUpdateTransition::from_contract_update(
            old_contract,
            new_contract,
            1,
            PlatformVersion::latest(),
        )
        .expect("expected a delta-based update transition");
        assert!(matches!(transition, DataContractUpdateTransition::V1(_)));
        transition
    }

    fn validate(
        platform: &TempPlatform<MockCoreRPCLike>,
        transition: &DataContractUpdateTransition,
    ) -> ConsensusValidationResult<StateTransitionAction> {
        let platform_version = PlatformVersion::latest();
        let state = platform.state.load();
        let platform_ref = PlatformRef {
            drive: &platform.drive,
            state: &state,
            config: &platform.config,
            core_rpc: &platform.core_rpc,
        };
        let mut execution_context =
            StateTransitionExecutionContext::default_for_platform_version(platform_version)
                .expect("expected an execution context");
        transition
            .validate_state(
                None,
                &platform_ref,
                ValidationMode::Validator,
                &BlockInfo::default(),
                &mut execution_context,
                None,
            )
            .expect("expected state validation to run")
    }

    fn assert_bumps_nonce(
        result: &ConsensusValidationResult<StateTransitionAction>,
        data_contract: &DataContract,
    ) {
        assert_matches!(
            &result.data,
            Some(StateTransitionAction::BumpIdentityDataContractNonceAction(action))
                if action.identity_id() == data_contract.owner_id()
                    && action.data_contract_id() == data_contract.id()
                    && action.identity_contract_nonce() == 1
        );
    }

    #[test]
    fn delta_update_adds_a_document_type_keywords_and_a_description() {
        let (platform, data_contract) = setup();
        store(&platform, &data_contract);
        let updated = extended(&data_contract);
        let transition = delta(&data_contract, &updated);

        let result = validate(&platform, &transition);

        assert!(result.is_valid(), "unexpected errors: {:?}", result.errors);
        let StateTransitionAction::DataContractUpdateAction(action) =
            result.into_data().expect("expected an action")
        else {
            panic!("expected a data contract update action");
        };
        let stored = action.data_contract_ref();
        assert_eq!(stored.id(), data_contract.id());
        assert_eq!(stored.owner_id(), data_contract.owner_id());
        assert_eq!(stored.version(), 2);
        assert!(stored.has_document_type_for_name("newType"));
        assert_eq!(
            stored.document_types().len(),
            data_contract.document_types().len() + 1
        );
        assert_eq!(stored.keywords(), updated.keywords());
        assert_eq!(stored.description(), updated.description());
        assert_eq!(stored.created_at(), data_contract.created_at());
        assert_eq!(action.identity_contract_nonce(), 1);
    }

    #[test]
    fn delta_update_of_a_missing_contract_is_a_paid_consensus_error() {
        let (platform, data_contract) = setup();
        let updated = extended(&data_contract);
        let transition = delta(&data_contract, &updated);

        let result = validate(&platform, &transition);

        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(BasicError::DataContractNotPresentError(error))]
                if error.data_contract_id() == data_contract.id()
        );
        assert_bumps_nonce(&result, &data_contract);
    }

    #[test]
    fn delta_update_from_an_identity_that_does_not_own_the_contract_is_rejected() {
        let (platform, data_contract) = setup();
        store(&platform, &data_contract);
        let updated = extended(&data_contract);
        let DataContractUpdateTransition::V1(mut v1) = delta(&data_contract, &updated) else {
            unreachable!()
        };
        let intruder = Identifier::random();
        v1.owner_id = intruder;
        let transition = DataContractUpdateTransition::V1(v1);

        let result = validate(&platform, &transition);

        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::StateError(StateError::DataContractUpdatePermissionError(error))]
                if *error.data_contract_id() == data_contract.id() && *error.identity_id() == intruder
        );
        // the bump lands on the intruder's own nonce, never on the owner's
        assert_matches!(
            &result.data,
            Some(StateTransitionAction::BumpIdentityDataContractNonceAction(action))
                if action.identity_id() == intruder && action.data_contract_id() == data_contract.id()
        );
    }

    #[test]
    fn delta_update_adding_a_document_type_that_exists_is_rejected() {
        let (platform, data_contract) = setup();
        store(&platform, &data_contract);
        let mut updated = data_contract.clone();
        updated.increment_version();
        let DataContractUpdateTransition::V1(mut v1) = delta(&data_contract, &updated) else {
            unreachable!()
        };
        v1.new_document_schemas
            .insert("niceDocument".to_string(), new_document_schema());
        let transition = DataContractUpdateTransition::V1(v1);

        let result = validate(&platform, &transition);

        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::StateError(StateError::DataContractUpdateEntryAlreadyExistsError(error))]
                if error.entry_kind() == DataContractUpdateEntryKind::DocumentType
                    && error.name() == "niceDocument"
        );
        assert_bumps_nonce(&result, &data_contract);
    }

    #[test]
    fn delta_update_is_held_to_the_full_contract_update_rules() {
        let (platform, data_contract) = setup();
        store(&platform, &data_contract);
        let mut updated = data_contract.clone();
        updated.set_version(data_contract.version() + 2);
        let transition = delta(&data_contract, &updated);

        let result = validate(&platform, &transition);

        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(BasicError::InvalidDataContractVersionError(error))]
                if error.expected_version() == data_contract.version() + 1
                    && error.version() == data_contract.version() + 2
        );
        assert_bumps_nonce(&result, &data_contract);
    }

    #[test]
    fn delta_update_reaching_a_pre_v15_validator_is_an_unsupported_version_error() {
        use crate::execution::validation::state_transition::processor::basic_structure::StateTransitionBasicStructureValidationV0;
        use dpp::consensus::basic::UnsupportedVersionError;
        use dpp::dashcore::Network;
        use dpp::validation::operations::ProtocolValidationOperation;

        let (platform, data_contract) = setup();
        store(&platform, &data_contract);
        let updated = extended(&data_contract);
        let transition = delta(&data_contract, &updated);
        let platform_version_14 = PlatformVersion::get(14).expect("protocol version 14");

        // check_tx decodes without the version-range gate, so a node still on
        // protocol version 14 can hand the delta form to its generation-1
        // basic structure validator; it must answer with a consensus error
        let result = transition
            .validate_basic_structure(Network::Testnet, platform_version_14)
            .expect(
                "an unsupported transition version is a consensus error, not an execution error",
            );
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(BasicError::UnsupportedVersionError(error))]
                if *error == UnsupportedVersionError::new(1, 0, 0)
        );

        // the same holds for the full-contract transformer Drive exposes
        let mut validation_operations: Vec<ProtocolValidationOperation> = vec![];
        let result = DataContractUpdateTransitionAction::try_from_borrowed_transition(
            &transition,
            &BlockInfo::default(),
            true,
            &mut validation_operations,
            platform_version_14,
        );
        assert_matches!(
            result,
            Err(ProtocolError::ConsensusError(error))
                if matches!(*error, ConsensusError::BasicError(BasicError::UnsupportedVersionError(_)))
        );
    }

    #[test]
    fn transforming_a_delta_update_of_a_missing_contract_is_not_an_execution_error() {
        let (platform, data_contract) = setup();
        let updated = extended(&data_contract);
        let transition = delta(&data_contract, &updated);
        let platform_version = PlatformVersion::latest();
        let state = platform.state.load();
        let platform_ref = PlatformRef {
            drive: &platform.drive,
            state: &state,
            config: &platform.config,
            core_rpc: &platform.core_rpc,
        };
        let mut execution_context =
            StateTransitionExecutionContext::default_for_platform_version(platform_version)
                .expect("expected an execution context");

        let result = transition
            .transform_into_action(
                &platform_ref,
                &BlockInfo::default(),
                &None,
                ValidationMode::CheckTx,
                &mut execution_context,
                None,
            )
            .expect("a missing contract is a consensus error, not an execution error");

        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::DataContractNotPresentError(_)
            )]
        );
        assert_bumps_nonce(&result, &data_contract);
    }
}
