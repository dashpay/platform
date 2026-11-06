use dpp::block::block_info::BlockInfo;
use dpp::data_contract::document_type::accessors::{DocumentTypeV0Getters, DocumentTypeV1Getters};
use dpp::data_contract::document_type::methods::DocumentTypeBasicMethods;
use dpp::fee::fee_result::FeeResult;
use dpp::platform_value::Identifier;
use grovedb::TransactionArg;
use std::sync::Arc;
use dpp::prelude::{ConsensusValidationResult, UserFeeIncrease};
use dpp::ProtocolError;
use dpp::decision_proposal::is_decision_proposal;
use dpp::state_transition::batch_transition::document_create_transition::v0::DocumentCreateTransitionV0;
use dpp::voting::vote_info_storage::contested_document_vote_poll_stored_info::ContestedDocumentVotePollStoredInfo;
use dpp::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
use platform_version::version::PlatformVersion;
use crate::drive::contract::DataContractFetchInfo;
use crate::drive::Drive;
use crate::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::resolve::ContestedDocumentResourceVotePollResolver;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::state_transition_action::batch::batched_transition::BatchedTransitionAction;
use crate::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::{DocumentBaseTransitionAction, DocumentBaseTransitionActionAccessorsV0};
use crate::state_transition_action::batch::batched_transition::document_transition::document_create_transition_action::{DecisionProposalCreate, DocumentCreateTransitionActionV0};
use crate::state_transition_action::batch::batched_transition::document_transition::state_generated_properties::generate_state_read_properties;
use crate::state_transition_action::batch::batched_transition::document_transition::DocumentTransitionAction;
use crate::state_transition_action::system::bump_identity_data_contract_nonce_action::BumpIdentityDataContractNonceAction;

impl DocumentCreateTransitionActionV0 {
    /// try from borrowed document create transition with contract lookup
    #[allow(clippy::too_many_arguments)]
    pub fn try_from_borrowed_document_create_transition_with_contract_lookup(
        drive: &Drive,
        owner_id: Identifier,
        transaction: TransactionArg,
        value: &DocumentCreateTransitionV0,
        block_info: &BlockInfo,
        user_fee_increase: UserFeeIncrease,
        get_data_contract: impl Fn(Identifier) -> Result<Arc<DataContractFetchInfo>, ProtocolError>,
        platform_version: &PlatformVersion,
    ) -> Result<
        (
            ConsensusValidationResult<BatchedTransitionAction>,
            FeeResult,
        ),
        Error,
    > {
        let DocumentCreateTransitionV0 {
            base,
            data,
            prefunded_voting_balance,
            ..
        } = value;
        let base_action_validation_result =
            DocumentBaseTransitionAction::try_from_borrowed_base_transition_with_contract_lookup(
                base,
                get_data_contract,
                |document_type| document_type.document_creation_token_cost(),
                |action_fees| action_fees.document_creation_action_fee(),
                "create",
            )?;

        let base = match base_action_validation_result.is_valid() {
            true => base_action_validation_result.into_data()?,
            false => {
                let bump_action =
                    BumpIdentityDataContractNonceAction::from_borrowed_document_base_transition(
                        base,
                        owner_id,
                        user_fee_increase,
                    );
                let batched_action =
                    BatchedTransitionAction::BumpIdentityDataContractNonce(bump_action);

                return Ok((
                    ConsensusValidationResult::new_with_data_and_errors(
                        batched_action,
                        base_action_validation_result.errors,
                    ),
                    FeeResult::default(),
                ));
            }
        };

        let document_type = base.document_type()?;

        // Added in place at protocol version 14, inert before it: `fill_generated_properties`
        // is `None` there and leaves the data as sent. From 14 on, every `generatedFrom`
        // property the transition leaves out is generated from its params here, before the
        // contest resolution below and every later check read the data.
        let mut data = data.clone();
        document_type.fill_generated_properties(&mut data, platform_version)?;

        // Added in place at protocol version 15, inert before it, where no type declares a
        // `generatedFrom` function that reads state: every such property is written from
        // state here, or checked when sent, before the contest resolution and every later
        // check read the data. A refusal bumps the nonce, and the write pays for the reads
        // either way.
        let (refusal, mut fee_result) = generate_state_read_properties(
            drive,
            document_type,
            base.data_contract_fetch_info_ref(),
            &mut data,
            None,
            block_info,
            transaction,
            platform_version,
        )?;
        // Added in place at protocol version 15, inert before it, where no type declares a
        // `sys.random` function: every such property is written with its placeholder here,
        // which the draw from the block's random beacon replaces before the block commits, and
        // a sent value is refused, before the contest resolution and every later check read
        // the data. A refusal bumps the nonce.
        let refusal = refusal.or_else(|| document_type.generate_random_properties(&mut data, None));
        if let Some(error) = refusal {
            let bump_action =
                BumpIdentityDataContractNonceAction::from_borrowed_document_base_transition_action(
                    &base,
                    owner_id,
                    user_fee_increase,
                );
            return Ok((
                ConsensusValidationResult::new_with_data_and_errors(
                    BatchedTransitionAction::BumpIdentityDataContractNonce(bump_action),
                    vec![error],
                ),
                fee_result,
            ));
        }

        let document_type_indexes = document_type.indexes();

        // A decision proposal (protocol version 15; its contract does not exist before it)
        // states the decision proposal fund, which names its own type rather than a contested
        // index: it is kept as stated for validation to judge, and opens no contest.
        let decision_proposal =
            is_decision_proposal(&base.data_contract_id(), base.document_type_name()).then(|| {
                Box::new(DecisionProposalCreate {
                    stated_fund: prefunded_voting_balance.clone(),
                })
            });

        let prefunded_voting_balances_by_vote_poll = prefunded_voting_balance
            .as_ref()
            .filter(|_| decision_proposal.is_none())
            .map(|(index_name, credits)| {
                let index = document_type_indexes.get(index_name).ok_or(
                    ProtocolError::UnknownContestedIndexResolution(format!(
                        "index {} not found on document type {}",
                        index_name.clone(),
                        document_type.name()
                    )),
                )?;
                // Identifier values are written one way from protocol version 14, so every
                // contender of a contest names it with the same poll and prefunds the same
                // balance; before 14 they are taken as given, as they always were
                let index_values = index.extract_contested_values(
                    &data,
                    document_type.flattened_properties(),
                    platform_version,
                )?;

                let vote_poll = ContestedDocumentResourceVotePoll {
                    contract_id: base.data_contract_id(),
                    document_type_name: base.document_type_name().clone(),
                    index_name: index_name.clone(),
                    index_values,
                };

                let resolved_vote_poll = vote_poll
                    .resolve_owned_with_provided_arc_contract_fetch_info(
                        base.data_contract_fetch_info(),
                    )?;

                Ok::<_, Error>((resolved_vote_poll, *credits))
            })
            .transpose()?;

        let (current_store_contest_info, should_store_contest_info) =
            if let Some((contested_document_resource_vote_poll, _)) =
                &prefunded_voting_balances_by_vote_poll
            {
                let (fetch_fee_result, maybe_current_store_contest_info) = drive
                    .fetch_contested_document_vote_poll_stored_info(
                        contested_document_resource_vote_poll,
                        Some(&block_info.epoch),
                        transaction,
                        platform_version,
                    )?;

                fee_result.checked_add_assign(fetch_fee_result.ok_or(Error::Drive(
                    DriveError::CorruptedCodeExecution("expected fee result"),
                ))?)?;
                let should_store_contest_info = if maybe_current_store_contest_info.is_none() {
                    // We are starting a new contest
                    Some(ContestedDocumentVotePollStoredInfo::new(
                        *block_info,
                        platform_version,
                    )?)
                } else {
                    None
                };
                (maybe_current_store_contest_info, should_store_contest_info)
            } else {
                (None, None)
            };

        Ok((
            BatchedTransitionAction::DocumentAction(DocumentTransitionAction::CreateAction(
                DocumentCreateTransitionActionV0 {
                    base,
                    block_info: *block_info,
                    data,
                    prefunded_voting_balance: prefunded_voting_balances_by_vote_poll,
                    current_store_contest_info,
                    should_store_contest_info: should_store_contest_info.map(Box::new),
                    property_constraint_reads: Default::default(),
                    moderated: false,
                    consumed_documents: Vec::new(),
                    derived_index_values: None,
                    decision_proposal,
                }
                .into(),
            ))
            .into(),
            fee_result,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::data_contract::conversion::value::v0::DataContractValueConversionMethodsV0;
    use dpp::data_contract::DataContract;
    use dpp::platform_value::{platform_value, Value};
    use dpp::state_transition::batch_transition::document_base_transition::v0::DocumentBaseTransitionV0;
    use dpp::state_transition::batch_transition::document_base_transition::DocumentBaseTransition;
    use crate::state_transition_action::batch::batched_transition::document_transition::document_create_transition_action::DocumentFromCreateTransitionAction;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::document::Document;
    use grovedb_costs::OperationCost;
    use std::collections::BTreeMap;

    const OWNER: [u8; 32] = [42; 32];
    const CONTRACT_ID: [u8; 32] = [9; 32];

    /// A card whose `rarity` is drawn from 1 to 100, indexed by it when `indexed`.
    fn card_schema(indexed: bool) -> Value {
        let mut schema = platform_value!({
            "type": "object",
            "documentsMutable": false,
            "properties": {
                "name": { "type": "string", "maxLength": 32, "position": 0 },
                "rarity": {
                    "type": "integer",
                    "minimum": 1,
                    "maximum": 100,
                    "generatedFrom": { "function": "sys.random.uniform" },
                    "position": 1
                }
            },
            "required": ["name", "rarity"],
            "additionalProperties": false
        });
        if indexed {
            schema
                .insert(
                    "indices".to_string(),
                    platform_value!([{ "name": "byRarity", "properties": [{ "rarity": "asc" }] }]),
                )
                .expect("the schema is a map");
        }
        schema
    }

    /// A card of a `set`, filed by its drawn rarity and its set's name, which the card never
    /// stores: a derived index property.
    fn set_card_schema() -> Value {
        let mut schema = card_schema(false);
        schema
            .insert(
                "indices".to_string(),
                platform_value!([{
                    "name": "byRarityAndSetName",
                    "properties": [{ "rarity": "asc" }, { "setId.name": "asc" }]
                }]),
            )
            .expect("the schema is a map");
        schema
            .get_mut("properties")
            .expect("the schema has properties")
            .expect("the schema has properties")
            .insert(
                "setId".to_string(),
                platform_value!({
                    "type": "array",
                    "byteArray": true,
                    "minItems": 32,
                    "maxItems": 32,
                    "contentMediaType": "application/x.dash.dpp.identifier",
                    "refersTo": { "type": "permanentDocument", "documentType": "set" },
                    "position": 2
                }),
            )
            .expect("the properties are a map");
        schema
            .get_mut("required")
            .expect("the schema has required")
            .expect("the schema has required")
            .push(Value::Text("setId".to_string()))
            .expect("required is an array");
        schema
    }

    /// A contract with a `card` indexed by its drawn rarity and a `looseCard` that is not,
    /// registered at `platform_version`.
    fn contract(platform_version: &PlatformVersion) -> Arc<DataContractFetchInfo> {
        contract_with(
            platform_value!({
                "card": card_schema(true),
                "looseCard": card_schema(false)
            }),
            platform_version,
        )
    }

    /// A contract with a `set` and a `setCard` indexed by its drawn rarity and its set's name,
    /// registered at `platform_version`.
    fn set_contract(platform_version: &PlatformVersion) -> Arc<DataContractFetchInfo> {
        contract_with(
            platform_value!({
                "set": {
                    "type": "object",
                    "documentsMutable": false,
                    "canBeDeleted": false,
                    "properties": {
                        "name": { "type": "string", "maxLength": 32, "position": 0 }
                    },
                    "required": ["name"],
                    "additionalProperties": false
                },
                "setCard": set_card_schema()
            }),
            platform_version,
        )
    }

    /// A contract of `document_schemas`, registered at `platform_version`.
    fn contract_with(
        document_schemas: Value,
        platform_version: &PlatformVersion,
    ) -> Arc<DataContractFetchInfo> {
        let contract = DataContract::from_value(
            platform_value!({
                "$formatVersion": "1",
                "id": Identifier::from(CONTRACT_ID),
                "ownerId": Identifier::from(OWNER),
                "version": 1u32,
                "documentSchemas": document_schemas
            }),
            true,
            platform_version,
        )
        .expect("the contract parses");
        Arc::new(DataContractFetchInfo {
            contract,
            storage_flags: None,
            cost: OperationCost::default(),
            fee: None,
        })
    }

    /// A contract with a `namedCard` indexed by its drawn rarity and its name.
    fn named_contract(platform_version: &PlatformVersion) -> Arc<DataContractFetchInfo> {
        let mut schema = card_schema(false);
        schema
            .insert(
                "indices".to_string(),
                platform_value!([{
                    "name": "byRarityAndName",
                    "properties": [{ "rarity": "asc" }, { "name": "asc" }]
                }]),
            )
            .expect("the schema is a map");
        contract_with(platform_value!({ "namedCard": schema }), platform_version)
    }

    /// Builds the action of a create of a `document_type_name` document of the contract
    /// `contract` builds, holding `data` (a name `Ace` unless `data` names one), and returns
    /// what building it bills and, when its document is valid, what its insert prepays for the
    /// draw: `Drive::prepay_random_document_index_entries` of the document the action writes.
    fn create(
        contract: fn(&PlatformVersion) -> Arc<DataContractFetchInfo>,
        document_type_name: &str,
        mut data: BTreeMap<String, Value>,
    ) -> (FeeResult, FeeResult) {
        data.entry("name".to_string())
            .or_insert_with(|| Value::Text("Ace".to_string()));
        let platform_version = PlatformVersion::latest().clone();
        let drive = setup_drive_with_initial_state_structure(Some(&platform_version));
        let contract = contract(&platform_version);
        let transition = DocumentCreateTransitionV0 {
            base: DocumentBaseTransition::V0(DocumentBaseTransitionV0 {
                id: Identifier::from([1; 32]),
                identity_contract_nonce: 1,
                document_type_name: document_type_name.to_string(),
                data_contract_id: Identifier::from(CONTRACT_ID),
            }),
            entropy: [2; 32],
            data,
            prefunded_voting_balance: None,
        };
        let (result, build_fee) =
            DocumentCreateTransitionActionV0::try_from_borrowed_document_create_transition_with_contract_lookup(
                &drive,
                Identifier::from(OWNER),
                None,
                &transition,
                &BlockInfo::default(),
                0,
                |_| Ok(contract.clone()),
                &platform_version,
            )
            .expect("the action builds");
        assert!(
            result.is_valid(),
            "the create is valid: {:?}",
            result.errors
        );
        let BatchedTransitionAction::DocumentAction(DocumentTransitionAction::CreateAction(action)) =
            result.into_data().expect("an action")
        else {
            panic!("a create action");
        };
        let document = Document::try_from_create_transition_action(
            &action,
            Identifier::from(OWNER),
            &platform_version,
        )
        .expect("the document");
        let prepayment = drive
            .prepay_random_document_index_entries(
                &document,
                Identifier::from(OWNER),
                &contract.contract,
                contract
                    .contract
                    .document_type_for_name(document_type_name)
                    .expect("the type"),
                &BlockInfo::default(),
                None,
                &platform_version,
            )
            .expect("the prepayment is priced");
        (build_fee, prepayment)
    }

    /// What a valid create of a `document_type_name` document of [`contract`] prepays.
    fn prepayment(document_type_name: &str) -> FeeResult {
        let (build_fee, prepayment) = create(contract, document_type_name, BTreeMap::new());
        // Building the action bills no prepayment: the insert of its document charges it
        assert_eq!(build_fee, FeeResult::default());
        prepayment
    }

    #[test]
    fn should_prepay_the_draw_of_an_indexed_random_property_on_create() {
        let indexed = prepayment("card");
        let unindexed = prepayment("looseCard");
        assert!(indexed.storage_fee > unindexed.storage_fee);
        assert!(indexed.processing_fee > unindexed.processing_fee);
        // Pinned: the estimated cost of the `byRarity` entry a draw may create. A change to
        // the estimate, the index layout or the fee schedule moves it.
        assert_eq!(
            indexed,
            FeeResult {
                storage_fee: 13_392_000,
                processing_fee: 6_650_100,
                ..Default::default()
            }
        );
    }

    #[test]
    fn should_prepay_the_draw_into_an_index_holding_a_derived_index_property() {
        // The create's document does not carry its set's name until its references are
        // validated: the prepayment completes it as the insert does, estimated, rather than
        // key an entry on a value it lacks
        let (build_fee, fee) = create(
            set_contract,
            "setCard",
            BTreeMap::from([("setId".to_string(), Value::Identifier([3; 32]))]),
        );
        assert_eq!(build_fee, FeeResult::default());
        // Pinned: the estimated cost of the `byRarityAndSetName` entry a draw may create, with
        // the set's name at its typical size (nothing is read), and the worst-case read of the
        // set. A change to the estimate, the index layout or the fee schedule moves it.
        assert_eq!(
            fee,
            FeeResult {
                storage_fee: 20_574_000,
                processing_fee: 9_003_340,
                ..Default::default()
            }
        );
    }

    #[test]
    fn should_build_the_action_of_a_create_whose_values_the_schema_refuses() {
        // Building the action reads no value of the indexes a draw moves: a name of the wrong
        // type, or a set reference that is no identifier, builds the action, unbilled, and the
        // schema refuses it later, bumping the nonce
        for (contract, document_type_name, data) in [
            (
                named_contract as fn(&PlatformVersion) -> Arc<DataContractFetchInfo>,
                "namedCard",
                BTreeMap::from([("name".to_string(), Value::Bool(false))]),
            ),
            (
                set_contract,
                "setCard",
                BTreeMap::from([(
                    "setId".to_string(),
                    Value::Text("no identifier".to_string()),
                )]),
            ),
        ] {
            let platform_version = PlatformVersion::latest().clone();
            let drive = setup_drive_with_initial_state_structure(Some(&platform_version));
            let contract = contract(&platform_version);
            let mut data = data;
            data.entry("name".to_string())
                .or_insert_with(|| Value::Text("Ace".to_string()));
            let transition = DocumentCreateTransitionV0 {
                base: DocumentBaseTransition::V0(DocumentBaseTransitionV0 {
                    id: Identifier::from([1; 32]),
                    identity_contract_nonce: 1,
                    document_type_name: document_type_name.to_string(),
                    data_contract_id: Identifier::from(CONTRACT_ID),
                }),
                entropy: [2; 32],
                data,
                prefunded_voting_balance: None,
            };
            let (result, build_fee) =
                DocumentCreateTransitionActionV0::try_from_borrowed_document_create_transition_with_contract_lookup(
                    &drive,
                    Identifier::from(OWNER),
                    None,
                    &transition,
                    &BlockInfo::default(),
                    0,
                    |_| Ok(contract.clone()),
                    &platform_version,
                )
                .expect("the action builds");
            assert!(
                result.is_valid(),
                "{document_type_name}: {:?}",
                result.errors
            );
            assert_eq!(build_fee, FeeResult::default(), "{document_type_name}");
        }
    }

    #[test]
    fn should_prepay_nothing_for_a_random_property_in_no_index() {
        assert_eq!(prepayment("looseCard"), FeeResult::default());
    }
}
