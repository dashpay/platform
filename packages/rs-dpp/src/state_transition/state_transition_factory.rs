use anyhow::anyhow;
use std::convert::{TryFrom, TryInto};

use crate::consensus::basic::state_transition::InvalidStateTransitionTypeError;
use crate::data_contract::errors::DataContractNotPresentError;
use crate::data_contract::state_transition::errors::MissingDataContractIdError;
use crate::{
    consensus::{basic::BasicError, ConsensusError},
    data_contract::{
        state_transition::{DataContractCreateTransition, DataContractUpdateTransition},
        DataContract,
    },
    document::DocumentsBatchTransition,
    identity::state_transition::{
        identity_create_transition::IdentityCreateTransition,
        identity_credit_withdrawal_transition::IdentityCreditWithdrawalTransition,
        identity_topup_transition::IdentityTopUpTransition,
    },
    prelude::Identifier,
    state_repository::StateRepositoryLike,
    util::json_value::JsonValueExt,
    ProtocolError,
};
use serde_json::Value as JsonValue;

use super::{
    state_transition_execution_context::StateTransitionExecutionContext, StateTransition,
    StateTransitionType,
};

//todo: change from JsonValue to Platform Value
pub async fn create_state_transition(
    state_repository: &impl StateRepositoryLike,
    raw_state_transition: JsonValue,
) -> Result<StateTransition, ProtocolError> {
    let transition_type = try_get_transition_type(&raw_state_transition)?;
    let execution_context = StateTransitionExecutionContext::default();

    match transition_type {
        StateTransitionType::DataContractCreate => {
            let transition = DataContractCreateTransition::from_raw_object(raw_state_transition)?;
            Ok(StateTransition::DataContractCreate(transition))
        }
        StateTransitionType::DataContractUpdate => {
            let transition = DataContractUpdateTransition::from_raw_object(raw_state_transition)?;
            Ok(StateTransition::DataContractUpdate(transition))
        }
        StateTransitionType::IdentityCreate => {
            let transition = IdentityCreateTransition::new(raw_state_transition)?;
            Ok(StateTransition::IdentityCreate(transition))
        }
        StateTransitionType::IdentityTopUp => {
            let transition = IdentityTopUpTransition::new(raw_state_transition)?;
            Ok(StateTransition::IdentityTopUp(transition))
        }
        StateTransitionType::IdentityCreditWithdrawal => {
            let transition =
                IdentityCreditWithdrawalTransition::from_raw_object(raw_state_transition)?;
            Ok(StateTransition::IdentityCreditWithdrawal(transition))
        }
        StateTransitionType::DocumentsBatch => {
            let maybe_transitions = raw_state_transition
                .get("transitions")
                .ok_or_else(|| anyhow!("the transitions property doesn't exist"))?;
            let raw_transitions = maybe_transitions
                .as_array()
                .ok_or_else(|| anyhow!("property transitions isn't an array"))?;
            let data_contracts = fetch_data_contracts_for_document_transition(
                state_repository,
                raw_transitions,
                &execution_context,
            )
            .await?;
            let documents_batch_transition = DocumentsBatchTransition::from_raw_object(
                raw_state_transition.into(),
                data_contracts,
            )?;
            Ok(StateTransition::DocumentsBatch(documents_batch_transition))
        }
        // TODO!! add basic validation
        StateTransitionType::IdentityUpdate => Err(ProtocolError::InvalidStateTransitionTypeError(
            InvalidStateTransitionTypeError::new(transition_type as u8),
        )),
    }
}

async fn fetch_data_contracts_for_document_transition(
    state_repository: &impl StateRepositoryLike,
    raw_document_transitions: impl IntoIterator<Item = &JsonValue>,
    execution_context: &StateTransitionExecutionContext,
) -> Result<Vec<DataContract>, ProtocolError> {
    let mut data_contracts = vec![];
    for transition in raw_document_transitions {
        let data_contract_id_bytes = transition.get_bytes("$dataContractId").map_err(|_| {
            ProtocolError::MissingDataContractIdError(MissingDataContractIdError::new(
                transition.to_owned(),
            ))
        })?;

        let data_contract_id = Identifier::from_bytes(&data_contract_id_bytes)?;
        let data_contract: DataContract = state_repository
            .fetch_data_contract(&data_contract_id, execution_context)
            .await?
            .map(TryInto::try_into)
            .transpose()
            .map_err(Into::into)?
            .ok_or_else(|| {
                ProtocolError::DataContractNotPresentError(DataContractNotPresentError::new(
                    data_contract_id,
                ))
            })?;

        data_contracts.push(data_contract);
    }

    Ok(data_contracts)
}

pub fn try_get_transition_type(
    raw_state_transition: &JsonValue,
) -> Result<StateTransitionType, ProtocolError> {
    let transition_type = raw_state_transition
        .get_u64("type")
        .map_err(|_| missing_state_transition_error())?;
    StateTransitionType::try_from(transition_type as u8).map_err(|_| {
        ProtocolError::InvalidStateTransitionTypeError(InvalidStateTransitionTypeError::new(
            transition_type as u8,
        ))
    })
}

fn missing_state_transition_error() -> ProtocolError {
    ProtocolError::AbstractConsensusError(Box::new(ConsensusError::BasicError(Box::new(
        BasicError::MissingStateTransitionTypeError,
    ))))
}

#[cfg(test)]
mod test {
    use dashcore::network::constants::PROTOCOL_VERSION;
    use platform_value::Value;
    use serde_json::json;
    use std::collections::BTreeMap;

    use crate::{
        data_contract::state_transition::DataContractCreateTransition,
        document::{
            document_transition::{Action, DocumentTransitionObjectLike},
            DocumentsBatchTransition,
        },
        state_repository::MockStateRepositoryLike,
        state_transition::{StateTransition, StateTransitionConvert},
        tests::fixtures::get_documents_fixture_with_owner_id_from_contract,
        tests::fixtures::{get_data_contract_fixture, get_document_transitions_fixture},
        ProtocolError,
    };

    use super::create_state_transition;

    #[tokio::test]
    async fn should_create_data_contract_transition_if_type_is_data_contract_create() {
        let data_contract = get_data_contract_fixture(None);
        let mut state_repostiory_mock = MockStateRepositoryLike::new();
        let data_contract_to_return = data_contract.clone();
        state_repostiory_mock
            .expect_fetch_data_contract()
            .returning(move |_, _| Ok(Some(data_contract_to_return.clone())));

        let state_transition_data = json!( {
                    "protocolVersion" :  PROTOCOL_VERSION,
                    "entropy": data_contract.entropy,
                    "dataContract": data_contract.to_object(false).unwrap(),
                }
        );
        let data_contract_create_state_transition =
            DataContractCreateTransition::from_raw_object(state_transition_data).unwrap();

        let result = create_state_transition(
            &state_repostiory_mock,
            data_contract_create_state_transition
                .to_object(false)
                .unwrap(),
        )
        .await
        .expect("the state transition should be created");

        assert!(
            matches!(result, StateTransition::DataContractCreate(transition) if  {
                transition.get_data_contract().to_object(false).unwrap() == data_contract.to_object(false).unwrap()
            })
        )
    }

    #[tokio::test]
    async fn should_return_document_batch_transition_if_type_is_documents() {
        let data_contract = get_data_contract_fixture(None);
        let documents =
            get_documents_fixture_with_owner_id_from_contract(data_contract.clone()).unwrap();
        let document_transitions =
            get_document_transitions_fixture(vec![(Action::Create, documents)]);

        let raw_document_transitions: Vec<Value> = document_transitions
            .iter()
            .map(|t| t.to_object().unwrap())
            .collect();

        let mut state_repostiory_mock = MockStateRepositoryLike::new();
        let data_contract_to_return = data_contract.clone();
        state_repostiory_mock
            .expect_fetch_data_contract()
            .returning(move |_, _| Ok(Some(data_contract_to_return.clone())));

        let mut map = BTreeMap::new();
        map.insert("protocolVersion".to_string(), Value::U32(PROTOCOL_VERSION));
        map.insert(
            "ownerId".to_string(),
            Value::Identifier(data_contract.owner_id.buffer),
        );
        map.insert(
            "transitions".to_string(),
            Value::Array(raw_document_transitions.clone()),
        );

        let documents_batch_state_transition =
            DocumentsBatchTransition::from_value_map(map, vec![data_contract]).unwrap();

        let result = create_state_transition(
            &state_repostiory_mock,
            documents_batch_state_transition.to_object(false).unwrap(),
        )
        .await
        .expect("the state transition should be created");

        assert!(matches!(result, StateTransition::DocumentsBatch(_)));

        let StateTransition::DocumentsBatch(transition) = result else {
            panic!("must be a DocumentsBatch transition")
        };
        let values = transition
            .get_transitions()
            .iter()
            .map(|t| t.to_object().unwrap())
            .collect::<Vec<Value>>();

        assert_eq!(values, raw_document_transitions);
    }

    #[tokio::test]
    async fn should_return_invalid_state_transition_type_if_type_is_invalid() {
        let state_repostiory_mock = MockStateRepositoryLike::new();
        let raw_state_transition = json!( {
            "type" : 666

        });

        let result = create_state_transition(&state_repostiory_mock, raw_state_transition).await;
        let err = get_protocol_error(result);

        match err {
            ProtocolError::InvalidStateTransitionTypeError(err) => {
                assert_eq!(err.transition_type(), 154);
            }
            _ => panic!("expected InvalidStateTransitionTypeError, got {}", err),
        }
    }

    pub fn get_protocol_error<T>(result: Result<T, ProtocolError>) -> ProtocolError {
        match result {
            Ok(_) => panic!("expected to get ProtocolError, got valid result"),
            Err(e) => e,
        }
    }
}
