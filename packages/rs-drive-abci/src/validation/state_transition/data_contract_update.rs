use dpp::consensus::basic::data_contract::InvalidDataContractVersionError;
use dpp::consensus::basic::document::DataContractNotPresentError;
use dpp::consensus::state::data_contract::data_contract_is_readonly_error::DataContractIsReadonlyError;
use dpp::data_contract::state_transition::data_contract_update_transition::validation::basic::DATA_CONTRACT_UPDATE_SCHEMA_VALIDATOR;
use dpp::identity::PartialIdentity;
use dpp::{
    consensus::basic::{
        data_contract::{
            DataContractImmutablePropertiesUpdateError, IncompatibleDataContractSchemaError,
        },
        BasicError,
    },
    data_contract::{
        property_names,
        state_transition::data_contract_update_transition::{
            validation::basic::{
                get_operation_and_property_name, get_operation_and_property_name_json,
                schema_compatibility_validator::{
                    validate_schema_compatibility, DiffVAlidatorError,
                },
                validate_indices_are_backward_compatible, EMPTY_JSON,
            },
            DataContractUpdateTransition,
        },
    },
    platform_value::{self, Value},
    state_transition::StateTransitionAction,
    Convertible, ProtocolError,
};
use dpp::{
    data_contract::state_transition::data_contract_update_transition::DataContractUpdateTransitionAction,
    validation::{ConsensusValidationResult, SimpleConsensusValidationResult},
};
use drive::drive::Drive;
use drive::grovedb::TransactionArg;
use serde_json::Value as JsonValue;
use dpp::data_contract::state_transition::data_contract_update_transition::validation::basic::schema_compatibility_validator::any_schema_changes;

use crate::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;
use crate::validation::state_transition::key_validation::validate_state_transition_identity_signature;
use crate::{error::Error, validation::state_transition::common::validate_schema};

use super::StateTransitionValidation;

impl StateTransitionValidation for DataContractUpdateTransition {
    fn validate_structure(
        &self,
        _drive: &Drive,
        _tx: TransactionArg,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let result = validate_schema(&DATA_CONTRACT_UPDATE_SCHEMA_VALIDATOR, self);
        if !result.is_valid() {
            return Ok(result);
        }

        // Validate protocol version
        //todo: redo versioning
        // let protocol_version_validator = ProtocolVersionValidator::default();
        // let result = protocol_version_validator
        //     .validate(self.protocol_version)
        //     .expect("TODO: again, how this will ever fail, why do we even need a validator trait");
        // if !result.is_valid() {
        //     return Ok(result);
        // }

        self.data_contract
            .validate_structure()
            .map_err(Error::Protocol)
    }

    fn validate_identity_and_signatures(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
    ) -> Result<ConsensusValidationResult<Option<PartialIdentity>>, Error> {
        Ok(
            validate_state_transition_identity_signature(drive, self, false, transaction)?
                .map(Some),
        )
    }

    fn validate_state<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let drive = platform.drive;
        let mut validation_result = ConsensusValidationResult::default();

        // Data contract should exist
        let add_to_cache_if_pulled = tx.is_some();
        // Data contract should exist
        let Some(contract_fetch_info) =
            drive
                .get_contract_with_fetch_info_and_fee(self.data_contract.id.0 .0, None, add_to_cache_if_pulled, tx)?
                .1
            else {
                validation_result
                    .add_error(BasicError::DataContractNotPresentError(
                        DataContractNotPresentError::new(self.data_contract.id.0.0.into())
                    ));
                return Ok(validation_result);
            };

        let existing_data_contract = &contract_fetch_info.contract;

        let new_version = self.data_contract.version;
        let old_version = existing_data_contract.version;
        if new_version < old_version || new_version - old_version != 1 {
            validation_result.add_error(BasicError::InvalidDataContractVersionError(
                InvalidDataContractVersionError::new(old_version + 1, new_version),
            ))
        }

        let mut existing_data_contract_object = existing_data_contract.to_object()?;
        let new_data_contract_object = self.data_contract.to_object()?;

        existing_data_contract_object
            .remove_many(&vec![
                property_names::DEFINITIONS,
                property_names::DOCUMENTS,
                property_names::VERSION,
            ])
            .map_err(ProtocolError::ValueError)?;

        let mut new_base_data_contract = new_data_contract_object.clone();
        new_base_data_contract
            .remove_many(&vec![
                property_names::DEFINITIONS,
                property_names::DOCUMENTS,
                property_names::VERSION,
            ])
            .map_err(ProtocolError::ValueError)?;

        let base_data_contract_diff =
            platform_value::patch::diff(&existing_data_contract_object, &new_base_data_contract);

        for diff in base_data_contract_diff.0.iter() {
            let (operation, property_name) = get_operation_and_property_name(diff);
            validation_result.add_error(BasicError::DataContractImmutablePropertiesUpdateError(
                DataContractImmutablePropertiesUpdateError::new(
                    operation.to_owned(),
                    property_name.to_owned(),
                    existing_data_contract_object
                        .get(property_name.split_at(1).1)
                        .ok()
                        .flatten()
                        .cloned()
                        .unwrap_or(Value::Null),
                    new_base_data_contract
                        .get(property_name.split_at(1).1)
                        .ok()
                        .flatten()
                        .cloned()
                        .unwrap_or(Value::Null),
                ),
            ))
        }
        if !validation_result.is_valid() {
            return Ok(validation_result);
        }

        // Schema should be backward compatible
        let old_schema = &existing_data_contract.documents;
        let new_schema: JsonValue = new_data_contract_object
            .get_value("documents")
            .map_err(ProtocolError::ValueError)?
            .clone()
            .try_into_validating_json() //maybe (not sure) / could be just try_into
            .map_err(ProtocolError::ValueError)?;

        if existing_data_contract.config.readonly && any_schema_changes(old_schema, &new_schema) {
            validation_result.add_error(DataContractIsReadonlyError::new(
                self.data_contract.id.0.0.into(),
            ));
            return Ok(validation_result);
        }

        for (document_type, document_schema) in old_schema.iter() {
            let new_document_schema = new_schema.get(document_type).unwrap_or(&EMPTY_JSON);
            let result = validate_schema_compatibility(document_schema, new_document_schema);
            match result {
                Ok(_) => {}
                Err(DiffVAlidatorError::SchemaCompatibilityError { diffs }) => {
                    let (operation_name, property_name) =
                        get_operation_and_property_name_json(&diffs[0]);
                    validation_result.add_error(BasicError::IncompatibleDataContractSchemaError(
                        IncompatibleDataContractSchemaError::new(
                            existing_data_contract.id,
                            operation_name.to_owned(),
                            property_name.to_owned(),
                            document_schema.clone(),
                            new_document_schema.clone(),
                        ),
                    ));
                }
                Err(DiffVAlidatorError::DataStructureError(e)) => {
                    return Err(ProtocolError::ParsingError(e.to_string()).into())
                }
            }
        }

        if !validation_result.is_valid() {
            return Ok(validation_result);
        }

        // check indices are not changed
        let new_documents: JsonValue = new_data_contract_object
            .get_value("documents")
            .and_then(|a| a.clone().try_into())
            .map_err(ProtocolError::ValueError)?;
        let new_documents = new_documents.as_object().ok_or_else(|| {
            ProtocolError::ParsingError("new documents is not a json object".to_owned())
        })?;
        validation_result.merge(validate_indices_are_backward_compatible(
            existing_data_contract.documents.iter(),
            new_documents,
        )?);
        if !validation_result.is_valid() {
            return Ok(validation_result);
        }

        self.transform_into_action(platform, tx)
    }

    fn transform_into_action<C: CoreRPCLike>(
        &self,
        _platform: &PlatformRef<C>,
        _tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let action: StateTransitionAction =
            Into::<DataContractUpdateTransitionAction>::into(self).into();
        Ok(action.into())
    }
}

#[cfg(test)]
mod tests {
    use dpp::block::block_info::BlockInfo;
    use crate::config::{PlatformConfig, PlatformTestConfig};
    use crate::platform::{Platform, PlatformRef};
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::{TempPlatform, TestPlatformBuilder};
    use dpp::data_contract::state_transition::data_contract_update_transition::DataContractUpdateTransition;
    use dpp::data_contract::DataContract;
    use dpp::platform_value::{BinaryData, Value};
    use dpp::state_transition::{StateTransitionConvert, StateTransitionType};
    use dpp::tests::fixtures::{get_data_contract_fixture, get_protocol_version_validator_fixture};
    use dpp::version::{ProtocolVersionValidator, LATEST_VERSION};

    struct TestData<T> {
        raw_state_transition: Value,
        data_contract: DataContract,
        platform: TempPlatform<T>,
    }

    pub struct PlatformTestHelper {
        platform: TempPlatform<MockCoreRPCLike>
    }

    impl PlatformTestHelper {
        pub fn apply_data_contract(&self, data_contract: &DataContract, block_info: BlockInfo) {
            self.platform
                .drive
                .apply_contract(data_contract, block_info, true, None, None)
                .expect("to apply contract");
        }
    }

    fn apply_contract(platform: &TempPlatform<MockCoreRPCLike>, data_contract: &DataContract, block_info: BlockInfo) {
        platform
            .drive
            .apply_contract(data_contract, block_info, true, None, None)
            .expect("to apply contract");
    }

    fn setup_test() -> TestData<MockCoreRPCLike> {
        let data_contract = get_data_contract_fixture(None).data_contract;
        let mut updated_data_contract = data_contract.clone();
        updated_data_contract.increment_version();

        let state_transition = DataContractUpdateTransition {
            protocol_version: LATEST_VERSION,
            data_contract: updated_data_contract,
            signature: BinaryData::new(vec![0; 65]),
            signature_public_key_id: 0,
            transition_type: StateTransitionType::DataContractUpdate,
        };

        let raw_state_transition = state_transition.to_object(false).unwrap();

        let dc = data_contract.clone();

        let config = PlatformConfig {
            verify_sum_trees: true,
            quorum_size: 10,
            validator_set_quorum_rotation_block_count: 25,
            block_spacing_ms: 300,
            testing_configs: PlatformTestConfig::default_with_no_block_signing(),
            ..Default::default()
        };
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

        TestData {
            raw_state_transition,
            data_contract: dc,
            platform: platform.set_initial_state_structure(),
        }
    }

    mod validate_state {
        use std::sync::Arc;
        use super::super::StateTransitionValidation;
        use super::*;
        use serde_json::json;
        use tracing_subscriber::util::SubscriberInitExt;
        use dpp::assert_state_consensus_errors;
        use dpp::errors::consensus::ConsensusError;
        use dpp::consensus::state::state_error::StateError::DataContractIsReadonlyError;
        use drive::drive::contract::ContractHistoryFetchInfo;

        #[test]
        pub fn should_return_error_if_trying_to_update_document_schema_in_a_readonly_contract() {
            let TestData {
                raw_state_transition,
                mut data_contract,
                mut platform,
            } = setup_test();

            data_contract.config.readonly = true;
            apply_contract(&platform, &data_contract, Default::default());

            let updated_document = json!({
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string"
                    },
                    "newProp": {
                        "type": "integer",
                        "minimum": 0
                    }
                },
                "required": [
                "$createdAt"
                ],
                "additionalProperties": false
            });

            data_contract.increment_version();
            data_contract
                .set_document_schema("niceDocument".into(), updated_document)
                .expect("to be able to set document schema");

            let state_transition = DataContractUpdateTransition {
                protocol_version: LATEST_VERSION,
                data_contract,
                signature: BinaryData::new(vec![0; 65]),
                signature_public_key_id: 0,
                transition_type: StateTransitionType::DataContractUpdate,
            };

            let platform_ref = PlatformRef {
                drive: &platform.drive,
                state: &platform.state.read().unwrap(),
                config: &platform.config,
                core_rpc: &platform.core_rpc,
            };

            let result = state_transition
                .validate_state(&platform_ref, None)
                .expect("state transition to be validated");

            assert!(!result.is_valid());
            assert_state_consensus_errors!(result, DataContractIsReadonlyError, 1);
        }

        #[test]
        pub fn should_keep_history_if_contract_config_keeps_history_is_true() {
            let TestData {
                raw_state_transition,
                mut data_contract,
                mut platform,
            } = setup_test();

            data_contract.config.keeps_history = true;
            data_contract.config.readonly = false;

            // TODO: check that keep_history actually works
            apply_contract(&platform, &data_contract, BlockInfo {
                time_ms: 1000,
                height: 100,
                core_height: 10,
                epoch: Default::default(),
            });

            let updated_document = json!({
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string"
                    },
                    "newProp": {
                        "type": "integer",
                        "minimum": 0
                    }
                },
                "required": [
                "$createdAt"
                ],
                "additionalProperties": false
            });

            data_contract.increment_version();
            data_contract
                .set_document_schema("niceDocument".into(), updated_document)
                .expect("to be able to set document schema");

            // TODO: add a data contract stop transition
            let state_transition = DataContractUpdateTransition {
                protocol_version: LATEST_VERSION,
                data_contract: data_contract.clone(),
                signature: BinaryData::new(vec![0; 65]),
                signature_public_key_id: 0,
                transition_type: StateTransitionType::DataContractUpdate,
            };

            let platform_ref = PlatformRef {
                drive: &platform.drive,
                state: &platform.state.read().unwrap(),
                config: &platform.config,
                core_rpc: &platform.core_rpc,
            };

            let result = state_transition
                .validate_state(&platform_ref, None)
                .expect("state transition to be validated");

            assert!(result.is_valid());

            // This should store update and history
            apply_contract(&platform, &data_contract, BlockInfo {
                time_ms: 2000,
                height: 110,
                core_height: 11,
                epoch: Default::default(),
            });

            // TODO: this shouldn't be working, investigate one day
            let res = platform.drive.fetch_contract_with_history(
                *data_contract.id.as_bytes(),
                None,
                Some(true),
                None
            );

            // // TODO: what to actually check here?
            // let fetch_result = match res.value {
            //     Ok(v) => {
            //         match v {
            //             None => {
            //                 panic!("Contract history info is none");
            //             }
            //             Some(v2) => {
            //                 v2
            //             }
            //         }
            //     },
            //     Err(e) => {
            //         println!("Received error: {:?}", e);
            //         panic!("Expected to receive a contract history");
            //     },
            // };

            // println!("History len: {:?}", fetch_result.history.len());
            // println!("{:?}", fetch_result.history);
        }
    }
}
