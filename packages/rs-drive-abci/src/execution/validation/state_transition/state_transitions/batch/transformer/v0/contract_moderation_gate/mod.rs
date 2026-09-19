mod v0;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use dpp::block::block_info::BlockInfo;
use dpp::consensus::state::contract_moderation::ContractModerationCounterpartyRole;
use dpp::consensus::ConsensusError;
use dpp::data_contract::DataContract;
use dpp::identifier::Identifier;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::batch_transition::batched_transition::document_transition::DocumentTransition;
use dpp::state_transition::batch_transition::BatchTransition;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::batch::batched_transition::BatchedTransitionAction;
use std::collections::{BTreeMap, BTreeSet};
use v0::BatchTransitionContractModerationGateV0;

/// What the gate hands back for a signer the contract bars.
pub(super) struct ContractModerationRefusal<'a> {
    /// The refusal of every transition the bar covers, each with its nonce bump.
    pub refused: ConsensusValidationResult<Vec<BatchedTransitionAction>>,
    /// The signer's deletions, by document type, which the bar does not cover: a barred
    /// identity may still take its own documents down. They carry on through the transformer.
    pub deletions: BTreeMap<&'a String, Vec<&'a DocumentTransition>>,
}

/// The contract moderation gate of the batch transformer (protocol version 14).
pub(super) trait BatchTransitionContractModerationGate {
    /// Gates the document transitions of `owner_id` against one contract on the contract's
    /// moderation lists. Returns the refusal when the signer is banned or under a live
    /// suspension and asks for anything but deletions, `None` to carry on with every
    /// transition. A suspension found lapsed is recorded in `lapsed_suspensions` for the batch
    /// to sweep.
    ///
    /// Before protocol version 14 the gate does not exist (`None` in the version table) and
    /// nothing is read.
    #[allow(clippy::too_many_arguments)]
    fn contract_moderation_gate<'a>(
        drive: &Drive,
        block_info: &BlockInfo,
        contract: &DataContract,
        owner_id: Identifier,
        document_transitions: &BTreeMap<&'a String, Vec<&'a DocumentTransition>>,
        lapsed_suspensions: &mut BTreeSet<(Identifier, Identifier)>,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<ContractModerationRefusal<'a>>, Error>;

    /// Gates the other party of a document transition: the recipient of a transfer or the
    /// seller of a purchase. Returns the error to refuse the transition with when that identity
    /// is banned or under a live suspension on the contract, `None` to carry on. Before
    /// protocol version 14 nothing is read.
    #[allow(clippy::too_many_arguments)]
    fn contract_moderation_counterparty_gate(
        drive: &Drive,
        block_info: &BlockInfo,
        contract: &DataContract,
        counterparty_id: Identifier,
        role: ContractModerationCounterpartyRole,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<ConsensusError>, Error>;
}

impl BatchTransitionContractModerationGate for BatchTransition {
    fn contract_moderation_gate<'a>(
        drive: &Drive,
        block_info: &BlockInfo,
        contract: &DataContract,
        owner_id: Identifier,
        document_transitions: &BTreeMap<&'a String, Vec<&'a DocumentTransition>>,
        lapsed_suspensions: &mut BTreeSet<(Identifier, Identifier)>,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<ContractModerationRefusal<'a>>, Error> {
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .batch_state_transition
            .contract_moderation_gate
        {
            None => Ok(None),
            Some(0) => Self::contract_moderation_gate_v0(
                drive,
                block_info,
                contract,
                owner_id,
                document_transitions,
                lapsed_suspensions,
                execution_context,
                transaction,
                platform_version,
            ),
            Some(version) => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "documents batch transition: contract_moderation_gate".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    fn contract_moderation_counterparty_gate(
        drive: &Drive,
        block_info: &BlockInfo,
        contract: &DataContract,
        counterparty_id: Identifier,
        role: ContractModerationCounterpartyRole,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<ConsensusError>, Error> {
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .batch_state_transition
            .contract_moderation_gate
        {
            None => Ok(None),
            Some(0) => Self::contract_moderation_counterparty_gate_v0(
                drive,
                block_info,
                contract,
                counterparty_id,
                role,
                execution_context,
                transaction,
                platform_version,
            ),
            Some(version) => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "documents batch transition: contract_moderation_counterparty_gate"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContextMethodsV0;
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
    use dpp::data_contract::config::moderation::{ContractModerationConfig, ContractModerators};
    use dpp::state_transition::StateTransition;
    use dpp::tests::fixtures::get_data_contract_fixture;
    use dpp::version::DefaultForPlatformVersion;

    fn moderated_contract(protocol_version: u32) -> DataContract {
        let mut contract =
            get_data_contract_fixture(None, 0, protocol_version).data_contract_owned();
        contract.set_config(contract.config().clone().with_moderation(Some(
            ContractModerationConfig {
                banlist: true,
                suspensions: true,
                moderators: ContractModerators::ContractOwner,
            },
        )));
        contract
    }

    /// No contract can declare moderation before protocol version 14, so this contract only
    /// exists in memory; the point is that the shared transformer asks nothing of Drive there.
    #[test]
    fn should_not_gate_or_read_before_protocol_version_14() {
        let platform_version = PlatformVersion::get(13).expect("protocol version 13");
        let platform = TestPlatformBuilder::new()
            .with_initial_protocol_version(13)
            .build_with_mock_rpc()
            .set_initial_state_structure();
        let contract = moderated_contract(13);
        let mut lapsed_suspensions = BTreeSet::new();
        let mut execution_context =
            StateTransitionExecutionContext::default_for_platform_version(platform_version)
                .expect("execution context");

        let refusal = BatchTransition::contract_moderation_gate(
            &platform.drive,
            &BlockInfo::default(),
            &contract,
            Identifier::from([1; 32]),
            &BTreeMap::new(),
            &mut lapsed_suspensions,
            &mut execution_context,
            None,
            platform_version,
        )
        .expect("expected the gate to run");

        assert!(refusal.is_none());
        assert!(lapsed_suspensions.is_empty());
        assert!(execution_context.operations_slice().is_empty());
    }

    #[test]
    fn should_not_read_for_a_contract_without_moderation() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();
        let contract = get_data_contract_fixture(None, 0, platform_version.protocol_version)
            .data_contract_owned();
        let mut execution_context =
            StateTransitionExecutionContext::default_for_platform_version(platform_version)
                .expect("execution context");

        let refusal = BatchTransition::contract_moderation_gate(
            &platform.drive,
            &BlockInfo::default(),
            &contract,
            Identifier::from([1; 32]),
            &BTreeMap::new(),
            &mut BTreeSet::new(),
            &mut execution_context,
            None,
            platform_version,
        )
        .expect("expected the gate to run");

        assert!(refusal.is_none());
        assert!(execution_context.operations_slice().is_empty());
    }

    /// One batch with two creates and one delete by a banned signer: the creates are refused,
    /// each with its own nonce bump, and the delete is kept for the transformer to carry on
    /// with. The batch cap is 1 at every protocol version, so this partition cannot be
    /// reached through the pipeline yet; it is pinned here on the gate itself.
    #[tokio::test]
    async fn should_refuse_each_barred_operation_and_keep_the_deletions_of_one_batch() {
        use crate::execution::validation::state_transition::tests::setup_identity;
        use dpp::dash_to_credits;
        use dpp::data_contract::document_type::random_document::{
            CreateRandomDocument, DocumentFieldFillSize, DocumentFieldFillType,
        };
        use dpp::identity::accessors::IdentityGettersV0;
        use dpp::platform_value::Bytes32;
        use dpp::state_transition::batch_transition::accessors::DocumentsBatchTransitionAccessorsV0;
        use dpp::state_transition::batch_transition::batched_transition::document_transition::DocumentTransitionV0Methods;
        use dpp::state_transition::batch_transition::batched_transition::BatchedTransitionRef;
        use dpp::state_transition::batch_transition::document_base_transition::v0::v0_methods::DocumentBaseTransitionV0Methods;
        use dpp::state_transition::batch_transition::methods::v0::DocumentsBatchTransitionMethodsV0;
        use drive::state_transition_action::system::bump_identity_data_contract_nonce_action::BumpIdentityDataContractNonceActionAccessorsV0;
        use rand::rngs::StdRng;
        use rand::SeedableRng;

        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();
        let (identity, signer, key) = setup_identity(&mut platform, 31, dash_to_credits!(1));
        let contract = moderated_contract(platform_version.protocol_version);
        platform
            .drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                None,
                None,
                platform_version,
            )
            .expect("expected to apply the contract");
        platform
            .drive
            .add_contract_ban(
                contract.id(),
                identity.id(),
                contract.owner_id(),
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to ban");

        let document_type = contract
            .document_type_for_name("niceDocument")
            .expect("expected the document type");
        let mut rng = StdRng::seed_from_u64(3);
        let mut batches = vec![];
        for nonce in 1..=3u64 {
            let entropy = Bytes32::random_with_rng(&mut rng);
            let document = document_type
                .random_document_with_identifier_and_entropy(
                    &mut rng,
                    identity.id(),
                    entropy,
                    DocumentFieldFillType::FillIfNotRequired,
                    DocumentFieldFillSize::MinDocumentFillSize,
                    platform_version,
                )
                .expect("expected a random document");
            let batch = if nonce == 2 {
                BatchTransition::new_document_deletion_transition_from_document(
                    document,
                    document_type,
                    &key,
                    nonce,
                    0,
                    None,
                    &signer,
                    platform_version,
                    None,
                )
                .await
            } else {
                BatchTransition::new_document_creation_transition_from_document(
                    document,
                    document_type,
                    entropy.0,
                    &key,
                    nonce,
                    0,
                    None,
                    &signer,
                    platform_version,
                    None,
                )
                .await
            }
            .expect("expected a batch");
            batches.push(batch);
        }
        let transitions: Vec<&DocumentTransition> = batches
            .iter()
            .flat_map(|batch| match batch {
                StateTransition::Batch(batch) => batch.transitions_iter().collect::<Vec<_>>(),
                _ => vec![],
            })
            .filter_map(|transition| match transition {
                BatchedTransitionRef::Document(document_transition) => Some(document_transition),
                BatchedTransitionRef::Token(_) => None,
            })
            .collect();
        let document_type_name = transitions[0].base().document_type_name();
        let document_transitions = BTreeMap::from([(document_type_name, transitions.clone())]);
        let mut execution_context =
            StateTransitionExecutionContext::default_for_platform_version(platform_version)
                .expect("execution context");

        let refusal = BatchTransition::contract_moderation_gate(
            &platform.drive,
            &BlockInfo::default(),
            &contract,
            identity.id(),
            &document_transitions,
            &mut BTreeSet::new(),
            &mut execution_context,
            None,
            platform_version,
        )
        .expect("expected the gate to run")
        .expect("expected the banned signer to be refused");

        // Two refused creates, one bump and one error each; the one delete kept.
        let refused_actions = refusal.refused.data.as_deref().unwrap_or_default();
        assert_eq!(refused_actions.len(), 2);
        assert_eq!(refusal.refused.errors.len(), 2);
        let bumped_nonces: BTreeSet<u64> = refused_actions
            .iter()
            .map(|action| match action {
                BatchedTransitionAction::BumpIdentityDataContractNonce(bump) => {
                    bump.identity_contract_nonce()
                }
                other => panic!("expected a nonce bump, got {other:?}"),
            })
            .collect();
        assert_eq!(bumped_nonces, BTreeSet::from([1, 3]));
        let kept: Vec<u64> = refusal
            .deletions
            .values()
            .flatten()
            .map(|transition| transition.base().identity_contract_nonce())
            .collect();
        assert_eq!(kept, vec![2]);
        // The status read was billed once for the whole batch.
        assert_eq!(execution_context.operations_slice().len(), 1);
    }
}
