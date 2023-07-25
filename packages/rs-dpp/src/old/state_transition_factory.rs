use crate::consensus::basic::BasicError;
use platform_value::Value;
use std::{
    convert::{TryFrom, TryInto},
    sync::Arc,
};

use crate::consensus::basic::state_transition::{
    InvalidStateTransitionTypeError, MissingStateTransitionTypeError,
};

use crate::consensus::basic::decode::SerializedObjectParsingError;
use crate::consensus::ConsensusError;
use crate::data_contract::errors::DataContractNotPresentError;
use crate::data_contract::state_transition::errors::MissingDataContractIdError;
use crate::serialization::PlatformDeserializable;
use crate::state_transition::errors::StateTransitionError;
use crate::state_transition::{StateTransitionFieldTypes, StateTransitionType};
use crate::ProtocolError;

#[derive(Default)]
pub struct StateTransitionFactoryOptions {
    pub skip_validation: bool,
}

pub struct StateTransitionFactory<SR, BLS>
where
    SR: StateRepositoryLike,
    BLS: BlsModule,
{
    state_repository: Arc<SR>,
    basic_validator:
        Arc<StateTransitionBasicValidator<SR, StateTransitionByTypeValidator<SR, BLS>>>,
}

impl<SR, BLS> StateTransitionFactory<SR, BLS>
where
    SR: StateRepositoryLike,
    BLS: BlsModule,
{
    pub fn new(
        state_repository: Arc<SR>,
        basic_validator: Arc<
            StateTransitionBasicValidator<SR, StateTransitionByTypeValidator<SR, BLS>>,
        >,
    ) -> Self {
        StateTransitionFactory {
            state_repository,
            basic_validator,
        }
    }

    pub async fn create_from_object(
        &self,
        raw_state_transition: Value,
        options: Option<StateTransitionFactoryOptions>,
    ) -> Result<StateTransition, ProtocolError> {
        let options = options.unwrap_or_default();

        if !options.skip_validation {
            self.validate_basic(&raw_state_transition).await?;
        }

        create_state_transition(self.state_repository.as_ref(), raw_state_transition).await
    }

    pub async fn create_from_buffer(
        &self,
        state_transition_buffer: &[u8],
        options: Option<StateTransitionFactoryOptions>,
    ) -> Result<StateTransition, ProtocolError> {
        let state_transition: StateTransition =
            StateTransition::deserialize(state_transition_buffer).map_err(|e| {
                ConsensusError::BasicError(BasicError::SerializedObjectParsingError(
                    SerializedObjectParsingError::new(format!("Decode protocol entity: {:#?}", e)),
                ))
            })?;

        if !options
            .as_ref()
            .map(|options| options.skip_validation)
            .unwrap_or_default()
        {
            self.validate_basic(&state_transition.to_cleaned_object(false)?)
                .await?;
        }

        Ok(state_transition)
    }

    pub async fn validate_basic(&self, raw_state_transition: &Value) -> Result<(), ProtocolError> {
        let execution_context = StateTransitionExecutionContext::default();

        let validation_result = self
            .basic_validator
            .validate(raw_state_transition, &execution_context)
            .await?;

        if !validation_result.is_valid() {
            return Err(ProtocolError::StateTransitionError(
                StateTransitionError::InvalidStateTransitionError {
                    errors: validation_result.errors,
                    raw_state_transition: raw_state_transition.to_owned(),
                },
            ));
        }

        Ok(())
    }
}
//
// pub async fn create_state_transition(
//     state_repository: &impl StateRepositoryLike,
//     raw_state_transition: Value,
// ) -> Result<StateTransition, ProtocolError> {
//     let transition_type = try_get_transition_type(&raw_state_transition)?;
//     let execution_context = StateTransitionExecutionContext::default();
//
//     match transition_type {
//         StateTransitionType::DataContractCreate => {
//             let transition = DataContractCreateTransition::from_object(raw_state_transition)?;
//             Ok(StateTransition::DataContractCreate(transition))
//         }
//         StateTransitionType::DataContractUpdate => {
//             let transition = DataContractUpdateTransition::from_object(raw_state_transition)?;
//             Ok(StateTransition::DataContractUpdate(transition))
//         }
//         StateTransitionType::IdentityCreate => {
//             let transition = IdentityCreateTransition::from_object(raw_state_transition)?;
//             Ok(StateTransition::IdentityCreate(transition))
//         }
//         StateTransitionType::IdentityTopUp => {
//             let transition = IdentityTopUpTransition::new(raw_state_transition)?;
//             Ok(StateTransition::IdentityTopUp(transition))
//         }
//         StateTransitionType::IdentityCreditWithdrawal => {
//             let transition =
//                 IdentityCreditWithdrawalTransition::from_object(raw_state_transition)?;
//             Ok(StateTransition::IdentityCreditWithdrawal(transition))
//         }
//         StateTransitionType::DocumentsBatch => {
//             let raw_transitions = raw_state_transition
//                 .get_array_ref("transitions")
//                 .map_err(ProtocolError::ValueError)?;
//             let data_contracts = fetch_data_contracts_for_document_transition(
//                 state_repository,
//                 raw_transitions,
//                 &execution_context,
//             )
//             .await?;
//             let documents_batch_transition =
//                 DocumentsBatchTransition::from_object_with_contracts(
//                     raw_state_transition,
//                     data_contracts,
//                 )?;
//             Ok(StateTransition::DocumentsBatch(documents_batch_transition))
//         }
//         StateTransitionType::IdentityUpdate => {
//             let transition = IdentityUpdateTransition::new(raw_state_transition)?;
//             Ok(StateTransition::IdentityUpdate(transition))
//         }
//         StateTransitionType::IdentityCreditTransfer => {
//             let transition = IdentityCreditTransferTransition::new(raw_state_transition)?;
//             Ok(StateTransition::IdentityCreditTransfer(transition))
//         }
//     }
// }
//
// async fn fetch_data_contracts_for_document_transition(
//     state_repository: &impl StateRepositoryLike,
//     raw_document_transitions: impl IntoIterator<Item = &Value>,
//     execution_context: &StateTransitionExecutionContext,
// ) -> Result<Vec<DataContract>, ProtocolError> {
//     let mut data_contracts = vec![];
//     for transition in raw_document_transitions {
//         let data_contract_id_bytes = transition.get_bytes("$dataContractId").map_err(|_| {
//             ProtocolError::MissingDataContractIdError(MissingDataContractIdError::new(
//                 transition.to_owned(),
//             ))
//         })?;
//
//         let data_contract_id = Identifier::from_bytes(&data_contract_id_bytes)?;
//         let data_contract: DataContract = state_repository
//             .fetch_data_contract(&data_contract_id, Some(execution_context))
//             .await?
//             .map(TryInto::try_into)
//             .transpose()
//             .map_err(Into::into)?
//             .ok_or_else(|| {
//                 ProtocolError::DataContractNotPresentError(DataContractNotPresentError::new(
//                     data_contract_id,
//                 ))
//             })?;
//
//         data_contracts.push(data_contract);
//     }
//
//     Ok(data_contracts)
// }

pub fn try_get_transition_type(
    raw_state_transition: &Value,
) -> Result<StateTransitionType, ProtocolError> {
    let transition_type: u8 = raw_state_transition
        .get_optional_integer("type")
        .map_err(ProtocolError::ValueError)?
        .ok_or(missing_state_transition_error())?;
    StateTransitionType::try_from(transition_type).map_err(|_| {
        ProtocolError::InvalidStateTransitionTypeError(InvalidStateTransitionTypeError::new(
            transition_type,
        ))
    })
}

fn missing_state_transition_error() -> ProtocolError {
    ProtocolError::ConsensusError(Box::new(ConsensusError::BasicError(
        BasicError::MissingStateTransitionTypeError(MissingStateTransitionTypeError::new()),
    )))
}

#[cfg(test)]
mod test {
    use crate::convertible::Convertible;
    use dashcore::network::constants::PROTOCOL_VERSION;
    use platform_value::{platform_value, Value};
    use std::collections::BTreeMap;

    use crate::{
        data_contract::state_transition::data_contract_create_transition::DataContractCreateTransition,
        document::{
            document_transition::{Action, DocumentTransitionObjectLike},
            DocumentsBatchTransition,
        },
        state_repository::MockStateRepositoryLike,
        state_transition::{StateTransition, StateTransitionFieldTypes},
        tests::fixtures::get_extended_documents_fixture_with_owner_id_from_contract,
        tests::fixtures::{get_data_contract_fixture, get_document_transitions_fixture},
        ProtocolError,
    };

    use super::create_state_transition;

    #[tokio::test]
    async fn should_create_data_contract_transition_if_type_is_data_contract_create() {
        let created_data_contract = get_data_contract_fixture(None);
        let mut state_repostiory_mock = MockStateRepositoryLike::new();
        let data_contract_to_return = created_data_contract.data_contract.clone();
        state_repostiory_mock
            .expect_fetch_data_contract()
            .returning(move |_, _| Ok(Some(data_contract_to_return.clone())));

        let state_transition_data = platform_value!( {
                    "protocolVersion" :  PROTOCOL_VERSION as u32,
                    "entropy": created_data_contract.entropy_used,
                    "dataContract": created_data_contract.data_contract.to_object().unwrap(),
                }
        );
        let data_contract_create_state_transition =
            DataContractCreateTransition::from_object(state_transition_data).unwrap();

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
                transition.data_contract().to_json_object().unwrap() == created_data_contract.data_contract.to_json_object().unwrap()
            })
        )
    }

    #[tokio::test]
    async fn should_return_document_batch_transition_if_type_is_documents() {
        let data_contract = get_data_contract_fixture(None).data_contract;
        let documents =
            get_extended_documents_fixture_with_owner_id_from_contract(data_contract.clone())
                .unwrap();
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
            Value::Identifier(data_contract.owner_id.to_buffer()),
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
        let state_repository_mock = MockStateRepositoryLike::new();
        let raw_state_transition = platform_value!( {
            "type" : 110u8
        });

        let result = create_state_transition(&state_repository_mock, raw_state_transition).await;
        let err = get_protocol_error(result);

        match err {
            ProtocolError::InvalidStateTransitionTypeError(err) => {
                assert_eq!(err.transition_type(), 110);
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
