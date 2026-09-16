use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::serialization::PlatformDeserializable;
use dpp::tokens::contract_info::v0::TokenContractInfoV0Accessors;
use dpp::tokens::contract_info::TokenContractInfo;
use dpp::tokens::contract_lifecycle::{ContractTokenLifecycle, TokenLifecycle};
use dpp::version::PlatformVersion;
use grovedb::Element::Item;
use grovedb::TransactionArg;
use std::collections::{BTreeMap, BTreeSet};

impl Drive {
    pub(super) fn fetch_token_lifecycles_v0(
        &self,
        token_ids: &[[u8; 32]],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<[u8; 32], TokenLifecycle>, Error> {
        if token_ids.is_empty() {
            return Ok(BTreeMap::new());
        }

        let mut drive_operations = vec![];

        // Token id to issuer, one merged read over the contract info leaves.
        let mut issuer_of_token: BTreeMap<[u8; 32], [u8; 32]> = BTreeMap::new();
        for (_, key, element) in self.grove_get_raw_path_query_with_optional(
            &Self::token_contract_infos_query(token_ids),
            false,
            transaction,
            &mut drive_operations,
            &platform_version.drive,
        )? {
            let token_id: [u8; 32] = key.try_into().map_err(|_| {
                Error::Drive(DriveError::CorruptedDriveState(
                    "token id not 32 bytes".to_string(),
                ))
            })?;
            match element {
                Some(Item(bytes, ..)) => {
                    let info = TokenContractInfo::deserialize_from_bytes(&bytes)?;
                    issuer_of_token.insert(token_id, info.contract_id().to_buffer());
                }
                None => {}
                _ => {
                    return Err(Error::Drive(DriveError::CorruptedDriveState(
                        "token contract info tree should contain only items".to_string(),
                    )))
                }
            }
        }

        if issuer_of_token.is_empty() {
            return Ok(BTreeMap::new());
        }

        // Issuer to lifecycle, one merged read over the records.
        let contract_ids: Vec<[u8; 32]> = issuer_of_token
            .values()
            .copied()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let mut lifecycle_of_issuer: BTreeMap<[u8; 32], TokenLifecycle> = BTreeMap::new();
        for (_, key, element) in self.grove_get_raw_path_query_with_optional(
            &Self::contract_token_lifecycles_query(&contract_ids),
            false,
            transaction,
            &mut drive_operations,
            &platform_version.drive,
        )? {
            let contract_id: [u8; 32] = key.try_into().map_err(|_| {
                Error::Drive(DriveError::CorruptedDriveState(
                    "contract id not 32 bytes".to_string(),
                ))
            })?;
            match element {
                Some(Item(bytes, ..)) => {
                    let record = ContractTokenLifecycle::deserialize_from_bytes(&bytes)?;
                    lifecycle_of_issuer.insert(contract_id, record.token_lifecycle());
                }
                None => {
                    // Every token has a contract info leaf and every issuer a record from the
                    // version that created the ledger on; a token whose issuer has none is
                    // corruption, not a live issuer.
                    return Err(Error::Drive(DriveError::CorruptedDriveState(format!(
                        "token issuer {} has no lifecycle record",
                        hex::encode(contract_id)
                    ))));
                }
                _ => {
                    return Err(Error::Drive(DriveError::CorruptedDriveState(
                        "token contract lifecycle tree should contain only items".to_string(),
                    )))
                }
            }
        }

        issuer_of_token
            .into_iter()
            .map(|(token_id, contract_id)| {
                lifecycle_of_issuer
                    .get(&contract_id)
                    .copied()
                    .map(|lifecycle| (token_id, lifecycle))
                    .ok_or_else(|| {
                        Error::Drive(DriveError::CorruptedDriveState(format!(
                            "token issuer {} was not resolved",
                            hex::encode(contract_id)
                        )))
                    })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::drive::tokens::paths::token_contract_lifecycles_root_path;
    use crate::error::drive::DriveError;
    use crate::error::Error;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::prelude::Identifier;
    use dpp::tokens::contract_lifecycle::TokenLifecycle;
    use dpp::version::PlatformVersion;
    use grovedb::Element;

    #[test]
    fn should_return_nothing_for_no_tokens() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let lifecycles = drive
            .fetch_token_lifecycles(&[], None, platform_version)
            .expect("expected to resolve");

        assert!(lifecycles.is_empty());
    }

    #[test]
    fn should_resolve_live_and_wiped_issuers_and_skip_unknown_tokens() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let block_info = BlockInfo {
            height: 40,
            ..Default::default()
        };
        let live_contract = Identifier::from([1u8; 32]);
        let wiped_contract = Identifier::from([2u8; 32]);
        let live_token = [11u8; 32];
        let wiped_token_a = [21u8; 32];
        let wiped_token_b = [22u8; 32];
        let unknown_token = [99u8; 32];

        for (contract_id, position, token_id) in [
            (live_contract, 0, live_token),
            (wiped_contract, 0, wiped_token_a),
            (wiped_contract, 1, wiped_token_b),
        ] {
            drive
                .create_token_trees(
                    contract_id,
                    position,
                    token_id,
                    false,
                    false,
                    &block_info,
                    true,
                    None,
                    platform_version,
                )
                .expect("expected to create token trees");
        }

        drive
            .destroy_token_issuer(
                wiped_contract.to_buffer(),
                &block_info,
                true,
                None,
                platform_version,
            )
            .expect("expected to destroy the issuer");

        let lifecycles = drive
            .fetch_token_lifecycles(
                &[live_token, wiped_token_a, unknown_token, wiped_token_b],
                None,
                platform_version,
            )
            .expect("expected to resolve");

        assert_eq!(lifecycles.len(), 3);
        assert_eq!(lifecycles.get(&live_token), Some(&TokenLifecycle::Live));
        assert_eq!(
            lifecycles.get(&wiped_token_a),
            Some(&TokenLifecycle::Wiped { block_height: 40 })
        );
        assert_eq!(
            lifecycles.get(&wiped_token_b),
            Some(&TokenLifecycle::Wiped { block_height: 40 })
        );
        assert_eq!(lifecycles.get(&unknown_token), None);
    }

    #[test]
    fn should_fail_when_a_known_token_has_no_issuer_record() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let contract_id = Identifier::from([4u8; 32]);
        let token_id = [41u8; 32];

        drive
            .create_token_trees(
                contract_id,
                0,
                token_id,
                false,
                false,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to create token trees");

        drive
            .grove
            .delete(
                &token_contract_lifecycles_root_path(),
                contract_id.as_slice(),
                None,
                None,
                &platform_version.drive.grove_version,
            )
            .unwrap()
            .expect("expected to delete the record");

        let result = drive.fetch_token_lifecycles(&[token_id], None, platform_version);

        assert!(matches!(
            result,
            Err(Error::Drive(DriveError::CorruptedDriveState(_)))
        ));
    }

    #[test]
    fn should_reject_a_record_that_is_not_an_item() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let contract_id = Identifier::from([6u8; 32]);
        let token_id = [61u8; 32];

        drive
            .create_token_trees(
                contract_id,
                0,
                token_id,
                false,
                false,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to create token trees");

        drive
            .grove
            .delete(
                &token_contract_lifecycles_root_path(),
                contract_id.as_slice(),
                None,
                None,
                &platform_version.drive.grove_version,
            )
            .unwrap()
            .expect("expected to delete the record");
        drive
            .grove
            .insert(
                &token_contract_lifecycles_root_path(),
                contract_id.as_slice(),
                Element::empty_tree(),
                None,
                None,
                &platform_version.drive.grove_version,
            )
            .unwrap()
            .expect("expected to insert a tree");

        let result = drive.fetch_token_lifecycles(&[token_id], None, platform_version);

        assert!(matches!(
            result,
            Err(Error::Drive(DriveError::CorruptedDriveState(_)))
        ));
    }

    #[test]
    fn should_not_be_active_before_the_ledger_exists() {
        let platform_version = PlatformVersion::get(14).expect("expected protocol version 14");
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));

        let result = drive.fetch_token_lifecycles(&[[1u8; 32]], None, platform_version);

        assert!(matches!(
            result,
            Err(Error::Drive(DriveError::VersionNotActive { .. }))
        ));
    }
}
