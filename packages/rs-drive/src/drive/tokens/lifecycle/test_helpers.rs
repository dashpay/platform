use crate::drive::balances::total_tokens_root_supply_path_vec;
use crate::drive::tokens::paths::token_contract_lifecycles_root_path_vec;
use crate::drive::Drive;
use crate::query::QueryResultType;
use dpp::serialization::PlatformDeserializable;
use dpp::tokens::contract_info::v0::TokenContractInfoV0Accessors;
use dpp::tokens::contract_lifecycle::v0::ContractTokenLifecycleV0Accessors;
use dpp::tokens::contract_lifecycle::ContractTokenLifecycle;
use dpp::version::PlatformVersion;
use grovedb::{Element, PathQuery, QueryItem, TransactionArg};
use std::collections::BTreeMap;
use std::ops::RangeFull;

impl Drive {
    /// Test-only walker over the whole token state: asserts that every token's supply leaf
    /// equals the stored sum of its balance tree, that every issuer's lifecycle record equals
    /// the sum of its tokens' supplies, that no record exists for a contract without tokens
    /// unless it is wiped, and that the destroyed supply scalar equals the summed rollups of
    /// the wiped issuers. Panics with the first discrepancy.
    ///
    /// The production paths never walk holders or tokens; this is the independent check the
    /// tests run after every step.
    pub fn assert_token_rollups_consistent(
        &self,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) {
        let supplies_query = PathQuery::new_single_query_item(
            total_tokens_root_supply_path_vec(),
            QueryItem::RangeFull(RangeFull),
        );
        let (supplies, _) = self
            .grove_get_raw_path_query(
                &supplies_query,
                transaction,
                QueryResultType::QueryKeyElementPairResultType,
                &mut vec![],
                &platform_version.drive,
            )
            .expect("expected to read the token supplies");

        let mut expected_rollups: BTreeMap<[u8; 32], u128> = BTreeMap::new();
        for (key, element) in supplies.to_key_elements() {
            let token_id: [u8; 32] = key.try_into().expect("token id must be 32 bytes");
            let supply = match element {
                Element::SumItem(supply, _) => supply,
                other => panic!("supply of token {} is {:?}", hex::encode(token_id), other),
            };
            let balance_sum = self
                .fetch_token_total_aggregated_identity_balances(
                    token_id,
                    transaction,
                    platform_version,
                )
                .expect("expected to read the balance sum")
                .unwrap_or_else(|| panic!("token {} has no balance tree", hex::encode(token_id)));
            assert_eq!(
                supply as u64,
                balance_sum,
                "token {} supply and balance sum differ",
                hex::encode(token_id)
            );
            let contract_id = self
                .fetch_token_contract_info(token_id, transaction, platform_version)
                .expect("expected to read the contract info")
                .unwrap_or_else(|| panic!("token {} has no contract info", hex::encode(token_id)))
                .contract_id()
                .to_buffer();
            *expected_rollups.entry(contract_id).or_insert(0) += supply as u128;
        }

        let records_query = PathQuery::new_single_query_item(
            token_contract_lifecycles_root_path_vec(),
            QueryItem::RangeFull(RangeFull),
        );
        let (records, _) = self
            .grove_get_raw_path_query(
                &records_query,
                transaction,
                QueryResultType::QueryKeyElementPairResultType,
                &mut vec![],
                &platform_version.drive,
            )
            .expect("expected to read the lifecycle ledger");

        let mut stored_rollups: BTreeMap<[u8; 32], u128> = BTreeMap::new();
        let mut destroyed_supply: u128 = 0;
        for (key, element) in records.to_key_elements() {
            if key.len() != 32 {
                continue;
            }
            let contract_id: [u8; 32] = key.try_into().expect("checked the length above");
            let bytes = match element {
                Element::Item(bytes, _) => bytes,
                other => panic!(
                    "record of issuer {} is {:?}",
                    hex::encode(contract_id),
                    other
                ),
            };
            let record = ContractTokenLifecycle::deserialize_from_bytes(&bytes)
                .expect("expected a lifecycle record");
            if record.is_wiped() {
                destroyed_supply += record.issued_supply();
            }
            stored_rollups.insert(contract_id, record.issued_supply());
        }

        for (contract_id, expected) in &expected_rollups {
            let stored = stored_rollups.get(contract_id).unwrap_or_else(|| {
                panic!(
                    "issuer {} has tokens but no record",
                    hex::encode(contract_id)
                )
            });
            assert_eq!(
                stored,
                expected,
                "issuer {} rollup differs from its summed supplies",
                hex::encode(contract_id)
            );
        }
        for (contract_id, stored) in &stored_rollups {
            if !expected_rollups.contains_key(contract_id) {
                assert_eq!(
                    *stored,
                    0,
                    "issuer {} has a rollup but no tokens",
                    hex::encode(contract_id)
                );
            }
        }

        let totals = self
            .calculate_total_tokens_balance(transaction, platform_version)
            .expect("expected the token totals");
        assert_eq!(
            totals.total_destroyed_supply as u128, destroyed_supply,
            "destroyed supply scalar differs from the summed wiped rollups"
        );
        assert!(
            totals.ok().expect("expected a verdict"),
            "token conservation failed: {totals}"
        );
    }
}
