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
    pub(super) fn fetch_identity_token_infos_v0(
        &self,
        token_ids: &[[u8; 32]],
        identity_id: [u8; 32],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<[u8; 32], Option<IdentityTokenInfo>>, Error> {
        self.fetch_identity_token_infos_operations_v0(
            token_ids,
            identity_id,
            transaction,
            &mut vec![],
            platform_version,
        )
    }

    pub(super) fn fetch_identity_token_infos_operations_v0(
        &self,
        token_ids: &[[u8; 32]],
        identity_id: [u8; 32],
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<[u8; 32], Option<IdentityTokenInfo>>, Error> {
        let path_query = Drive::token_infos_for_identity_id_query(token_ids, identity_id);

        self.grove_get_raw_path_query_with_optional(
            &path_query,
            false,
            transaction,
            drive_operations,
            &platform_version.drive,
        )?
        .into_iter()
        .map(|(path, _, element)| {
            let token_id: [u8; 32] = path
                .get(2)
                .ok_or(Error::Drive(DriveError::CorruptedDriveState(
                    "returned path item should always have a third part at index 2".to_string(),
                )))?
                .clone()
                .try_into()
                .map_err(|_| {
                    Error::Drive(DriveError::CorruptedDriveState(
                        "token id not 32 bytes".to_string(),
                    ))
                })?;
            match element {
                Some(Item(value, ..)) => Ok((
                    token_id,
                    Some(IdentityTokenInfo::deserialize_from_bytes(&value)?),
                )),
                None => Ok((token_id, None)),
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

    /// Build a contract with two tokens at positions 0 and 1.
    fn build_two_token_contract() -> DataContract {
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
            tokens: BTreeMap::from([
                (
                    0,
                    TokenConfiguration::V0(TokenConfigurationV0::default_most_restrictive()),
                ),
                (
                    1,
                    TokenConfiguration::V0(TokenConfigurationV0::default_most_restrictive()),
                ),
            ]),
            keywords: Vec::new(),
            description: None,
        })
    }

    #[test]
    fn should_return_empty_for_empty_token_id_list() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let identity_id = [1u8; 32];

        let infos = drive
            .fetch_identity_token_infos_v0(&[], identity_id, None, platform_version)
            .expect("expected fetch with empty token list to succeed");

        assert!(
            infos.is_empty(),
            "expected empty result map for empty token list, got {:?}",
            infos
        );
    }

    #[test]
    fn should_return_partial_hits_across_multiple_tokens() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let identity = Identity::random_identity(3, Some(7), platform_version)
            .expect("expected a platform identity");
        let identity_id = identity.id().to_buffer();

        let contract = build_two_token_contract();
        let token_id_0 = contract.token_id(0).expect("expected token at position 0");
        let token_id_1 = contract.token_id(1).expect("expected token at position 1");

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

        // Only freeze the first token — the second should have no info record.
        drive
            .token_freeze(
                token_id_0,
                identity.id(),
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to freeze token 0");

        let infos = drive
            .fetch_identity_token_infos_v0(
                &[token_id_0.to_buffer(), token_id_1.to_buffer()],
                identity_id,
                None,
                platform_version,
            )
            .expect("expected fetch to succeed");

        assert_eq!(
            infos,
            BTreeMap::from([
                (
                    token_id_0.to_buffer(),
                    Some(IdentityTokenInfo::V0(IdentityTokenInfoV0 { frozen: true })),
                ),
                (token_id_1.to_buffer(), None),
            ])
        );
    }

    #[test]
    fn should_return_none_entries_for_all_non_existent_tokens() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let identity_id = [9u8; 32];
        // Use token ids that have no on-disk info tree at all.
        let token_ids = [[200u8; 32], [201u8; 32]];

        let infos = drive
            .fetch_identity_token_infos_v0(&token_ids, identity_id, None, platform_version)
            .expect("expected fetch to succeed even for non-existent tokens");

        // grove_get_raw_path_query_with_optional returns entries for each requested key
        // even when nothing exists — they should all be None.
        let expected: BTreeMap<[u8; 32], Option<IdentityTokenInfo>> =
            token_ids.into_iter().map(|id| (id, None)).collect();
        assert_eq!(
            infos, expected,
            "expected a None entry for every requested token"
        );
    }

    #[test]
    fn should_return_costs_for_fetch_with_costs() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let block_info = BlockInfo::default();

        let identity = Identity::random_identity(3, Some(8), platform_version)
            .expect("expected a platform identity");
        let identity_id = identity.id().to_buffer();

        let contract = build_two_token_contract();
        let token_id_0 = contract.token_id(0).expect("expected token at position 0");
        let token_id_1 = contract.token_id(1).expect("expected token at position 1");

        drive
            .add_new_identity(
                identity.clone(),
                false,
                &block_info,
                true,
                None,
                platform_version,
            )
            .expect("expected to add an identity");

        drive
            .insert_contract(&contract, block_info, true, None, platform_version)
            .expect("expected to insert contract");

        drive
            .token_freeze(
                token_id_0,
                identity.id(),
                &block_info,
                true,
                None,
                platform_version,
            )
            .expect("expected to freeze token 0");

        let (infos, fees) = drive
            .fetch_identity_token_infos_with_costs(
                &[token_id_0.to_buffer(), token_id_1.to_buffer()],
                identity_id,
                &block_info,
                None,
                platform_version,
            )
            .expect("expected fetch with costs to succeed");

        assert_eq!(
            infos,
            BTreeMap::from([
                (
                    token_id_0.to_buffer(),
                    Some(IdentityTokenInfo::V0(IdentityTokenInfoV0 { frozen: true })),
                ),
                (token_id_1.to_buffer(), None),
            ])
        );
        assert!(
            fees.processing_fee > 0 || fees.storage_fee > 0,
            "expected non-zero fees for a fetch-with-costs call"
        );
    }
}
