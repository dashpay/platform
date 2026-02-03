use crate::state_transition_action::contract::data_contract_create::v0::DataContractCreateTransitionActionV0;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v1::DataContractV1Setters;
use dpp::data_contract::serialized_version::v1::DataContractInSerializationFormatV1;
use dpp::data_contract::serialized_version::DataContractInSerializationFormat;
use dpp::prelude::DataContract;
use dpp::state_transition::data_contract_create_transition::{
    DataContractCreateTransitionV0, DataContractCreateTransitionV1,
};
use dpp::validation::operations::ProtocolValidationOperation;
use dpp::ProtocolError;
use platform_version::version::PlatformVersion;

impl DataContractCreateTransitionActionV0 {
    pub(in crate::state_transition_action::contract::data_contract_create) fn try_from_v0_transition(
        value: &DataContractCreateTransitionV0,
        block_info: &BlockInfo,
        full_validation: bool,
        validation_operations: &mut Vec<ProtocolValidationOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        let mut data_contract = DataContract::try_from_platform_versioned(
            value.data_contract.clone(),
            full_validation,
            validation_operations,
            platform_version,
        )?;
        data_contract.set_created_at(Some(block_info.time_ms));
        data_contract.set_created_at_epoch(Some(block_info.epoch.index));
        data_contract.set_created_at_block_height(Some(block_info.height));
        Ok(DataContractCreateTransitionActionV0 {
            data_contract,
            identity_nonce: value.identity_nonce,
            user_fee_increase: value.user_fee_increase,
        })
    }

    pub(in crate::state_transition_action::contract::data_contract_create) fn try_from_v1_transition(
        value: &DataContractCreateTransitionV1,
        block_info: &BlockInfo,
        full_validation: bool,
        validation_operations: &mut Vec<ProtocolValidationOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        let DataContractCreateTransitionV1 {
            owner_id,
            config,
            schema_defs,
            document_schemas,
            groups,
            tokens,
            keywords,
            description,
            identity_nonce,
            user_fee_increase,
            ..
        } = value;

        // Generate contract ID from owner_id and identity_nonce
        let id = DataContract::generate_data_contract_id_v0(*owner_id, *identity_nonce);

        // Create a serialization format from the V1 transition fields
        let serialization_format =
            DataContractInSerializationFormat::V1(DataContractInSerializationFormatV1 {
                id,
                config: *config,
                version: 1, // New contract starts at version 1
                owner_id: *owner_id,
                schema_defs: schema_defs.clone(),
                document_schemas: document_schemas.clone(),
                created_at: None,
                updated_at: None,
                created_at_block_height: None,
                updated_at_block_height: None,
                created_at_epoch: None,
                updated_at_epoch: None,
                groups: groups.clone(),
                tokens: tokens.clone(),
                keywords: keywords.clone(),
                description: description.clone(),
            });

        let mut data_contract = DataContract::try_from_platform_versioned(
            serialization_format,
            full_validation,
            validation_operations,
            platform_version,
        )?;

        data_contract.set_created_at(Some(block_info.time_ms));
        data_contract.set_created_at_epoch(Some(block_info.epoch.index));
        data_contract.set_created_at_block_height(Some(block_info.height));

        Ok(DataContractCreateTransitionActionV0 {
            data_contract,
            identity_nonce: *identity_nonce,
            user_fee_increase: *user_fee_increase,
        })
    }
}
