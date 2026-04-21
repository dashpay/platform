use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::serialization::PlatformDeserializable;
use dpp::tokens::info::IdentityTokenInfo;
use dpp::version::PlatformVersion;
use grovedb::Element::Item;
use grovedb::TransactionArg;
use std::collections::BTreeMap;

impl Drive {
    pub(super) fn fetch_identities_token_infos_v0(
        &self,
        token_id: [u8; 32],
        identity_ids: &[[u8; 32]],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<[u8; 32], Option<IdentityTokenInfo>>, Error> {
        self.fetch_identities_token_infos_operations_v0(
            token_id,
            identity_ids,
            transaction,
            &mut vec![],
            platform_version,
        )
    }

    pub(super) fn fetch_identities_token_infos_operations_v0(
        &self,
        token_id: [u8; 32],
        identity_ids: &[[u8; 32]],
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<[u8; 32], Option<IdentityTokenInfo>>, Error> {
        let path_query = Self::token_infos_for_identity_ids_query(token_id, identity_ids);

        self.grove_get_raw_path_query_with_optional(
            &path_query,
            false,
            transaction,
            drive_operations,
            &platform_version.drive,
        )?
        .into_iter()
        .map(|(_, key, element)| {
            let identity_id: [u8; 32] = key.try_into().map_err(|_| {
                Error::Drive(DriveError::CorruptedDriveState(
                    "identity id not 32 bytes".to_string(),
                ))
            })?;
            match element {
                Some(Item(value, ..)) => Ok((
                    identity_id,
                    Some(IdentityTokenInfo::deserialize_from_bytes(&value)?),
                )),
                None => Ok((identity_id, None)),
                _ => Err(Error::Drive(DriveError::CorruptedDriveState(
                    "token tree for infos should contain only items".to_string(),
                ))),
            }
        })
        .collect()
    }
}

#[cfg(test)]
mod tests {
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
    use dpp::tokens::info::IdentityTokenInfo;
    use dpp::version::PlatformVersion;
    use std::collections::BTreeMap;

    fn build_single_token_contract() -> DataContract {
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
    fn should_return_empty_for_empty_identity_id_list() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let token_id = [3u8; 32];

        let infos = drive
            .fetch_identities_token_infos_v0(token_id, &[], None, platform_version)
            .expect("expected fetch with empty identity list to succeed");

        assert!(
            infos.is_empty(),
            "expected empty map for empty identity list, got {:?}",
            infos
        );
    }

    #[test]
    fn should_return_partial_hits_across_multiple_identities() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let identity_frozen = Identity::random_identity(3, Some(20), platform_version)
            .expect("expected a platform identity");
        let identity_frozen_id = identity_frozen.id().to_buffer();

        let identity_other = Identity::random_identity(3, Some(21), platform_version)
            .expect("expected a platform identity");
        let identity_other_id = identity_other.id().to_buffer();

        let contract = build_single_token_contract();
        let token_id = contract.token_id(0).expect("expected token at position 0");

        drive
            .add_new_identity(
                identity_frozen.clone(),
                false,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to add identity_frozen");
        drive
            .add_new_identity(
                identity_other.clone(),
                false,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to add identity_other");

        drive
            .insert_contract(
                &contract,
                BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to insert contract");

        drive
            .token_freeze(
                token_id,
                identity_frozen.id(),
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to freeze first identity");

        let infos = drive
            .fetch_identities_token_infos_v0(
                token_id.to_buffer(),
                &[identity_frozen_id, identity_other_id],
                None,
                platform_version,
            )
            .expect("expected fetch to succeed");

        assert_eq!(
            infos,
            BTreeMap::from([
                (
                    identity_frozen_id,
                    Some(IdentityTokenInfo::V0(IdentityTokenInfoV0 { frozen: true })),
                ),
                (identity_other_id, None),
            ])
        );
    }

    #[test]
    fn should_return_none_entries_for_non_existent_token() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        // No contract/token ever inserted for this token_id
        let token_id = [123u8; 32];
        let identity_ids = [[1u8; 32], [2u8; 32], [3u8; 32]];

        let infos = drive
            .fetch_identities_token_infos_v0(token_id, &identity_ids, None, platform_version)
            .expect("expected fetch to succeed even when token tree does not exist");

        let expected: BTreeMap<[u8; 32], Option<IdentityTokenInfo>> =
            identity_ids.into_iter().map(|id| (id, None)).collect();
        assert_eq!(
            infos, expected,
            "expected a None entry for every requested identity"
        );
    }

    #[test]
    fn should_return_costs_for_fetch_with_costs() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let block_info = BlockInfo::default();

        let identity_frozen = Identity::random_identity(3, Some(22), platform_version)
            .expect("expected a platform identity");
        let identity_frozen_id = identity_frozen.id().to_buffer();

        let identity_other = Identity::random_identity(3, Some(23), platform_version)
            .expect("expected a platform identity");
        let identity_other_id = identity_other.id().to_buffer();

        let contract = build_single_token_contract();
        let token_id = contract.token_id(0).expect("expected token at position 0");

        drive
            .add_new_identity(
                identity_frozen.clone(),
                false,
                &block_info,
                true,
                None,
                platform_version,
            )
            .expect("expected to add identity_frozen");
        drive
            .add_new_identity(
                identity_other.clone(),
                false,
                &block_info,
                true,
                None,
                platform_version,
            )
            .expect("expected to add identity_other");

        drive
            .insert_contract(&contract, block_info, true, None, platform_version)
            .expect("expected to insert contract");

        drive
            .token_freeze(
                token_id,
                identity_frozen.id(),
                &block_info,
                true,
                None,
                platform_version,
            )
            .expect("expected to freeze first identity");

        let (infos, fees) = drive
            .fetch_identities_token_infos_with_costs(
                token_id.to_buffer(),
                &[identity_frozen_id, identity_other_id],
                &block_info,
                None,
                platform_version,
            )
            .expect("expected fetch with costs to succeed");

        assert_eq!(
            infos,
            BTreeMap::from([
                (
                    identity_frozen_id,
                    Some(IdentityTokenInfo::V0(IdentityTokenInfoV0 { frozen: true })),
                ),
                (identity_other_id, None),
            ])
        );
        assert!(
            fees.processing_fee > 0 || fees.storage_fee > 0,
            "expected non-zero fees for a fetch-with-costs call"
        );
    }
}
