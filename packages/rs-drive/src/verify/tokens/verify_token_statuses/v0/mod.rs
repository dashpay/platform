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
    pub(super) fn verify_token_statuses_v0<
        T: FromIterator<(I, Option<TokenStatus>)>,
        I: From<[u8; 32]>,
    >(
        proof: &[u8],
        token_ids: &[[u8; 32]],
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, T), Error> {
        let path_query = Self::token_statuses_query(token_ids);
        let (root_hash, proved_key_values) = if verify_subset_of_proof {
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
        if proved_key_values.len() == token_ids.len() {
            let values = proved_key_values
                .into_iter()
                .map(|proved_key_value| {
                    let token_id: [u8; 32] = proved_key_value.1.try_into().map_err(|_| {
                        Error::Proof(ProofError::IncorrectValueSize("token id size"))
                    })?;
                    match proved_key_value.2 {
                        Some(Item(value, ..)) => Ok((
                            token_id.into(),
                            Some(TokenStatus::deserialize_from_bytes(&value)?),
                        )),
                        None => Ok((token_id.into(), None)),
                        _ => Err(Error::Proof(ProofError::IncorrectValueSize(
                            "proof did not point to an item as expected for token info",
                        ))),
                    }
                })
                .collect::<Result<T, Error>>()?;
            Ok((root_hash, values))
        } else {
            Err(Error::Proof(ProofError::WrongElementCount {
                expected: token_ids.len(),
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

    fn create_token_contract() -> DataContract {
        DataContract::V1(DataContractV1 {
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
        })
    }

    #[test]
    fn should_prove_and_verify_token_statuses_paused() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let contract = create_token_contract();
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

        let token_ids = vec![token_id.to_buffer()];
        let proof = drive
            .prove_token_statuses(&token_ids, None, platform_version)
            .expect("expected to get proof");

        let (_, statuses): (_, BTreeMap<[u8; 32], Option<TokenStatus>>) =
            Drive::verify_token_statuses(proof.as_slice(), &token_ids, false, platform_version)
                .expect("expected proof verification to succeed");

        assert_eq!(statuses.len(), 1);
        assert_eq!(
            statuses.get(&token_id.to_buffer()),
            Some(&Some(TokenStatus::V0(TokenStatusV0 { paused: true })))
        );
    }

    #[test]
    fn should_prove_and_verify_absent_token_statuses() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        // Use a non-existent token id
        let non_existent_token_id = [0u8; 32];
        let token_ids = vec![non_existent_token_id];

        let proof = drive
            .prove_token_statuses(&token_ids, None, platform_version)
            .expect("expected to get proof");

        let (_, statuses): (_, BTreeMap<[u8; 32], Option<TokenStatus>>) =
            Drive::verify_token_statuses(proof.as_slice(), &token_ids, false, platform_version)
                .expect("expected proof verification to succeed");

        assert_eq!(statuses.len(), 1);
        assert_eq!(statuses.get(&non_existent_token_id), Some(&None));
    }
}
