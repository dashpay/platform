use crate::drive::Drive;
use grovedb::Element::Item;

use crate::error::proof::ProofError;
use crate::error::Error;

use crate::verify::RootHash;

use dpp::serialization::PlatformDeserializable;
use dpp::tokens::status::TokenStatus;
use grovedb::GroveDb;
use platform_version::version::PlatformVersion;

impl Drive {
    pub(super) fn verify_token_status_v0(
        proof: &[u8],
        token_id: [u8; 32],
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Option<TokenStatus>), Error> {
        let path_query = Self::token_status_query(token_id);
        let (root_hash, mut proved_key_values) = if verify_subset_of_proof {
            GroveDb::verify_subset_query_with_absence_proof(
                proof,
                &path_query,
                &platform_version.drive.grove_version,
            )?
        } else {
            GroveDb::verify_query_with_absence_proof(
                proof,
                &path_query,
                &platform_version.drive.grove_version,
            )?
        };
        if proved_key_values.len() == 1 {
            let proved_key_value = proved_key_values.remove(0);
            match proved_key_value.2 {
                Some(Item(value, ..)) => Ok((
                    root_hash,
                    Some(TokenStatus::deserialize_from_bytes(&value)?),
                )),
                None => Ok((root_hash, None)),
                _ => Err(Error::Proof(ProofError::IncorrectValueSize(
                    "proof did not point to an item",
                ))),
            }
        } else {
            Err(Error::Proof(ProofError::WrongElementCount {
                expected: 1,
                got: proved_key_values.len(),
            }))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::data_contract::accessors::v1::DataContractV1Getters;
    use dpp::data_contract::associated_token::token_configuration::v0::TokenConfigurationV0;
    use dpp::data_contract::associated_token::token_configuration::TokenConfiguration;
    use dpp::data_contract::config::v0::DataContractConfigV0;
    use dpp::data_contract::config::DataContractConfig;
    use dpp::data_contract::v1::DataContractV1;
    use dpp::prelude::DataContract;
    use dpp::tokens::status::v0::TokenStatusV0;
    use dpp::version::PlatformVersion;
    use std::collections::BTreeMap;

    #[test]
    fn should_prove_and_verify_single_token_status_paused() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let contract = DataContract::V1(DataContractV1 {
            id: Default::default(),
            version: 0,
            owner_id: Default::default(),
            document_types: Default::default(),
            config: DataContractConfig::V0(DataContractConfigV0 {
                can_be_deleted: false,
                readonly: false,
                keeps_history: false,
                documents_keep_history_contract_default: false,
                documents_mutable_contract_default: false,
                documents_can_be_deleted_contract_default: false,
                requires_identity_encryption_bounded_key: None,
                requires_identity_decryption_bounded_key: None,
            }),
            schema_defs: None,
            created_at: None,
            updated_at: None,
            created_at_block_height: None,
            updated_at_block_height: None,
            created_at_epoch: None,
            updated_at_epoch: None,
            groups: Default::default(),
            tokens: BTreeMap::from([(
                0,
                TokenConfiguration::V0(TokenConfigurationV0::default_most_restrictive()),
            )]),
            keywords: Vec::new(),
            description: None,
        });

        let token_id = contract.token_id(0).expect("expected token at position 0");

        drive
            .insert_contract(
                &contract,
                BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to insert contract");

        // Pause the token
        drive
            .token_apply_status(
                token_id.to_buffer(),
                TokenStatus::new(true, platform_version).expect("expected token status"),
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to pause token");

        // Generate proof using the same single-token query the verify function uses
        let path_query = Drive::token_status_query(token_id.to_buffer());
        let proof = drive
            .grove_get_proved_path_query(&path_query, None, &mut vec![], &platform_version.drive)
            .expect("expected to get proof");

        let (_, status) = Drive::verify_token_status(
            proof.as_slice(),
            token_id.to_buffer(),
            false,
            platform_version,
        )
        .expect("expected proof verification to succeed");

        assert_eq!(
            status,
            Some(TokenStatus::V0(TokenStatusV0 { paused: true }))
        );
    }

    #[test]
    fn should_prove_and_verify_absent_single_token_status() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        // Use a non-existent token id
        let non_existent_token_id = [0u8; 32];

        let path_query = Drive::token_status_query(non_existent_token_id);
        let proof = drive
            .grove_get_proved_path_query(&path_query, None, &mut vec![], &platform_version.drive)
            .expect("expected to get proof");

        let (_, status) = Drive::verify_token_status(
            proof.as_slice(),
            non_existent_token_id,
            false,
            platform_version,
        )
        .expect("expected proof verification to succeed");

        assert_eq!(status, None);
    }
}
