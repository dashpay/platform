use crate::drive::tokens::paths::token_identity_infos_path;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::DirectQueryType;
use crate::util::grove_operations::QueryTarget::QueryTargetValue;
use dpp::serialization::PlatformDeserializable;
use dpp::tokens::info::IdentityTokenInfo;
use dpp::version::PlatformVersion;
use grovedb::Element::Item;
use grovedb::{TransactionArg, TreeType};

impl Drive {
    pub(super) fn fetch_identity_token_info_v0(
        &self,
        token_id: [u8; 32],
        identity_id: [u8; 32],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<IdentityTokenInfo>, Error> {
        self.fetch_identity_token_info_operations_v0(
            token_id,
            identity_id,
            true,
            transaction,
            &mut vec![],
            platform_version,
        )
    }

    pub(super) fn fetch_identity_token_info_operations_v0(
        &self,
        token_id: [u8; 32],
        identity_id: [u8; 32],
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<IdentityTokenInfo>, Error> {
        let direct_query_type = if apply {
            DirectQueryType::StatefulDirectQuery
        } else {
            DirectQueryType::StatelessDirectQuery {
                in_tree_type: TreeType::NormalTree,
                query_target: QueryTargetValue(8),
            }
        };

        let info_path = token_identity_infos_path(&token_id);

        match self.grove_get_raw_optional(
            (&info_path).into(),
            identity_id.as_slice(),
            direct_query_type,
            transaction,
            drive_operations,
            &platform_version.drive,
        ) {
            Ok(Some(Item(info, _))) => Ok(Some(IdentityTokenInfo::deserialize_from_bytes(
                info.as_slice(),
            )?)),

            Ok(None) => Ok(None),
            Err(Error::GroveDB(e)) if matches!(e.as_ref(), grovedb::Error::PathKeyNotFound(_)) => {
                Ok(None)
            }

            Ok(Some(_)) => Err(Error::Drive(DriveError::CorruptedElementType(
                "identity token info was present but was not an item",
            ))),

            Err(e) => Err(e),
        }
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

    /// Build a simple single-token data contract for tests.
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
    fn should_return_none_for_non_existent_token() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        // token_id that has never been created => path-key-not-found should collapse to None
        let token_id = [77u8; 32];
        let identity_id = [88u8; 32];

        let info = drive
            .fetch_identity_token_info_v0(token_id, identity_id, None, platform_version)
            .expect("expected fetch to succeed (non-existent token => None)");

        assert_eq!(info, None);
    }

    #[test]
    fn should_return_none_for_identity_without_info_record() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        // Real contract/token exists, but the identity has never had an info record
        // (never frozen/unfrozen). Expect None from the fetch.
        let contract = build_single_token_contract();
        let token_id = contract
            .token_id(0)
            .expect("expected token at position 0")
            .to_buffer();

        drive
            .insert_contract(
                &contract,
                BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to insert contract");

        // Arbitrary identity id that has never touched this token
        let identity_id = [42u8; 32];

        let info = drive
            .fetch_identity_token_info_v0(token_id, identity_id, None, platform_version)
            .expect("expected fetch to succeed");

        assert_eq!(info, None);
    }

    #[test]
    fn should_return_frozen_info_after_freeze() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let identity = Identity::random_identity(3, Some(5), platform_version)
            .expect("expected a platform identity");
        let identity_id = identity.id().to_buffer();

        let contract = build_single_token_contract();
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

        drive
            .token_freeze(
                token_id,
                identity.id(),
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to freeze token");

        let info = drive
            .fetch_identity_token_info_v0(token_id.to_buffer(), identity_id, None, platform_version)
            .expect("expected fetch to succeed");

        assert_eq!(
            info,
            Some(IdentityTokenInfo::V0(IdentityTokenInfoV0 { frozen: true }))
        );
    }

    #[test]
    fn should_return_info_with_costs_after_freeze() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let block_info = BlockInfo::default();

        let identity = Identity::random_identity(3, Some(6), platform_version)
            .expect("expected a platform identity");
        let identity_id = identity.id().to_buffer();

        let contract = build_single_token_contract();
        let token_id = contract.token_id(0).expect("expected token at position 0");

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
                token_id,
                identity.id(),
                &block_info,
                true,
                None,
                platform_version,
            )
            .expect("expected to freeze token");

        let (info, fees) = drive
            .fetch_identity_token_info_with_costs(
                token_id.to_buffer(),
                identity_id,
                &block_info,
                true,
                None,
                platform_version,
            )
            .expect("expected fetch with costs to succeed");

        assert_eq!(
            info,
            Some(IdentityTokenInfo::V0(IdentityTokenInfoV0 { frozen: true }))
        );
        assert!(
            fees.processing_fee > 0 || fees.storage_fee > 0,
            "expected non-zero fees for a stateful fetch"
        );
    }
}
