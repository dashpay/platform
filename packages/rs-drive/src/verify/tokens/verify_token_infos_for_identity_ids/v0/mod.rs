use crate::drive::Drive;

use crate::error::proof::ProofError;
use crate::error::Error;

use crate::verify::RootHash;

use dpp::serialization::PlatformDeserializable;
use dpp::tokens::info::IdentityTokenInfo;
use grovedb::GroveDb;
use platform_version::version::PlatformVersion;

impl Drive {
    pub(super) fn verify_token_infos_for_identity_ids_v0<
        T: FromIterator<(I, Option<IdentityTokenInfo>)>,
        I: From<[u8; 32]>,
    >(
        proof: &[u8],
        token_id: [u8; 32],
        identity_ids: &[[u8; 32]],
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, T), Error> {
        let path_query = Self::token_infos_for_identity_ids_query(token_id, identity_ids);
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
        if proved_key_values.len() == identity_ids.len() {
            let values = proved_key_values
                .into_iter()
                .map(|proved_key_value| {
                    let key: [u8; 32] = proved_key_value
                        .1
                        .try_into()
                        .map_err(|_| Error::Proof(ProofError::IncorrectValueSize("value size")))?;
                    let maybe_element = proved_key_value.2;
                    match maybe_element {
                        None => Ok((key.into(), None)),
                        Some(element) => {
                            let info_bytes = element.as_item_bytes().map_err(Error::from)?;
                            let info = IdentityTokenInfo::deserialize_from_bytes(info_bytes)?;
                            Ok((key.into(), Some(info)))
                        }
                    }
                })
                .collect::<Result<T, Error>>()?;
            Ok((root_hash, values))
        } else {
            Err(Error::Proof(ProofError::WrongElementCount {
                expected: identity_ids.len(),
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
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::Identity;
    use dpp::prelude::DataContract;
    use dpp::tokens::info::v0::IdentityTokenInfoV0;
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
    fn should_prove_and_verify_token_infos_for_multiple_identity_ids() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let identity1 = Identity::random_identity(3, Some(14), platform_version)
            .expect("expected a platform identity");
        let identity2 = Identity::random_identity(3, Some(15), platform_version)
            .expect("expected a platform identity");

        let contract = create_token_contract();
        let token_id = contract.token_id(0).expect("expected token at position 0");

        // Add both identities
        drive
            .add_new_identity(
                identity1.clone(),
                false,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to add identity 1");

        drive
            .add_new_identity(
                identity2.clone(),
                false,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to add identity 2");

        drive
            .insert_contract(
                &contract,
                BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to insert contract");

        // Freeze only identity1 for this token
        drive
            .token_freeze(
                token_id,
                identity1.id(),
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to freeze token for identity1");

        let identity_ids = vec![identity1.id().to_buffer(), identity2.id().to_buffer()];

        let proof = drive
            .prove_identities_token_infos(
                token_id.to_buffer(),
                &identity_ids,
                None,
                platform_version,
            )
            .expect("expected to get proof");

        let (_, infos): (_, BTreeMap<[u8; 32], Option<IdentityTokenInfo>>) =
            Drive::verify_token_infos_for_identity_ids(
                proof.as_slice(),
                token_id.to_buffer(),
                &identity_ids,
                false,
                platform_version,
            )
            .expect("expected proof verification to succeed");

        assert_eq!(infos.len(), 2);
        assert_eq!(
            infos.get(&identity1.id().to_buffer()),
            Some(&Some(IdentityTokenInfo::V0(IdentityTokenInfoV0 {
                frozen: true
            })))
        );
        assert_eq!(infos.get(&identity2.id().to_buffer()), Some(&None));
    }

    #[test]
    fn should_prove_and_verify_absent_token_infos_for_identity_ids() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let identity = Identity::random_identity(3, Some(14), platform_version)
            .expect("expected a platform identity");

        let contract = create_token_contract();
        let token_id = contract.token_id(0).expect("expected token at position 0");

        drive
            .add_new_identity(
                identity.clone(),
                false,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to add an identity");

        drive
            .insert_contract(
                &contract,
                BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to insert contract");

        let identity_ids = vec![identity.id().to_buffer()];

        let proof = drive
            .prove_identities_token_infos(
                token_id.to_buffer(),
                &identity_ids,
                None,
                platform_version,
            )
            .expect("expected to get proof");

        let (_, infos): (_, BTreeMap<[u8; 32], Option<IdentityTokenInfo>>) =
            Drive::verify_token_infos_for_identity_ids(
                proof.as_slice(),
                token_id.to_buffer(),
                &identity_ids,
                false,
                platform_version,
            )
            .expect("expected proof verification to succeed");

        assert_eq!(infos.len(), 1);
        assert_eq!(infos.get(&identity.id().to_buffer()), Some(&None));
    }
}
