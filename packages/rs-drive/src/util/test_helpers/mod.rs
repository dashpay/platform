#[cfg(feature = "server")]
use std::fs::File;
#[cfg(feature = "server")]
use std::io;
#[cfg(feature = "server")]
use std::io::BufRead;
#[cfg(feature = "server")]
use std::path::Path;

#[cfg(feature = "fixtures-and-mocks")]
use grovedb::TransactionArg;

#[cfg(feature = "fixtures-and-mocks")]
use crate::drive::Drive;
#[cfg(feature = "fixtures-and-mocks")]
use dpp::data_contract::DataContract;

#[cfg(feature = "fixtures-and-mocks")]
use dpp::block::block_info::BlockInfo;
#[cfg(feature = "fixtures-and-mocks")]
use dpp::prelude::Identifier;

#[cfg(feature = "fixtures-and-mocks")]
use crate::drive::votes::paths::vote_end_date_queries_tree_path_vec;
#[cfg(feature = "fixtures-and-mocks")]
use crate::query::VotePollsByEndDateDriveQuery;
#[cfg(feature = "fixtures-and-mocks")]
use crate::util::common::encode::decode_u64;
#[cfg(feature = "fixtures-and-mocks")]
use dpp::prelude::TimestampMillis;
#[cfg(feature = "fixtures-and-mocks")]
use dpp::tests::json_document::json_document_to_contract_with_ids;
#[cfg(feature = "fixtures-and-mocks")]
use dpp::version::PlatformVersion;
#[cfg(feature = "fixtures-and-mocks")]
use grovedb::query_result_type::QueryResultType;
#[cfg(feature = "fixtures-and-mocks")]
use grovedb::{PathQuery, Query};
#[cfg(feature = "fixtures-and-mocks")]
use std::collections::{BTreeMap, BTreeSet};

#[cfg(test)]
use ciborium::value::Value;

#[cfg(any(test, feature = "server"))]
pub mod setup;
#[cfg(any(test, feature = "fixtures-and-mocks"))]
/// test utils
pub mod test_utils;

#[cfg(feature = "fixtures-and-mocks")]
/// Applies to Drive a JSON contract from the file system.
pub fn setup_contract(
    drive: &Drive,
    path: &str,
    contract_id: Option<[u8; 32]>,
    owner_id: Option<[u8; 32]>,
    contract_modification: Option<impl FnOnce(&mut DataContract)>,
    transaction: TransactionArg,
    use_platform_version: Option<&PlatformVersion>,
) -> DataContract {
    let platform_version = use_platform_version.unwrap_or(PlatformVersion::latest());
    let mut contract = json_document_to_contract_with_ids(
        path,
        contract_id.map(Identifier::from),
        owner_id.map(Identifier::from),
        false, //no need to validate the data contracts in tests for drive
        platform_version,
    )
    .expect("expected to get json based contract");

    if let Some(contract_modification) = contract_modification {
        contract_modification(&mut contract);
    }

    drive
        .apply_contract(
            &contract,
            BlockInfo::default(),
            true,
            None,
            transaction,
            platform_version,
        )
        .expect("contract should be applied");
    contract
}

#[cfg(feature = "fixtures-and-mocks")]
/// Every end date of the vote poll end-date queries, with the unique ids of the vote polls listed
/// under it; an end date that lists none shows as an empty set.
pub fn vote_poll_end_dates(
    drive: &Drive,
    platform_version: &PlatformVersion,
) -> BTreeMap<TimestampMillis, BTreeSet<Identifier>> {
    let mut query = Query::new();
    query.insert_all();
    let (end_dates, _) = drive
        .grove_get_raw_path_query(
            &PathQuery::new_unsized(vote_end_date_queries_tree_path_vec(), query),
            None,
            QueryResultType::QueryKeyElementPairResultType,
            &mut vec![],
            &platform_version.drive,
        )
        .expect("expected to read the end dates");
    end_dates
        .to_keys()
        .into_iter()
        .map(|end_date_key| {
            let end_date = decode_u64(&end_date_key).expect("expected an encoded end date");
            let unique_ids =
                VotePollsByEndDateDriveQuery::execute_no_proof_keys_for_single_end_time(
                    end_date,
                    None,
                    drive,
                    None,
                    &mut vec![],
                    platform_version,
                )
                .expect("expected to read the vote polls of an end date")
                .into_iter()
                .map(|key| Identifier::from_bytes(&key).expect("expected a vote poll id"))
                .collect();
            (end_date, unique_ids)
        })
        .collect()
}

#[cfg(test)]
/// Serializes a hex string to CBOR.
pub fn cbor_from_hex(hex_string: String) -> Vec<u8> {
    hex::decode(hex_string).expect("Decoding failed")
}

#[cfg(feature = "server")]
/// Takes a file and returns the lines as a list of strings.
pub fn text_file_strings(path: impl AsRef<Path>) -> Vec<String> {
    let file = File::open(path).expect("file not found");
    let reader = io::BufReader::new(file).lines();
    reader.into_iter().map(|a| a.unwrap()).collect()
}

#[cfg(test)]
/// Retrieves the value of a key from a CBOR map.
pub fn get_key_from_cbor_map<'a>(
    cbor_map: &'a [(Value, Value)],
    key: &'a str,
) -> Option<&'a Value> {
    for (cbor_key, cbor_value) in cbor_map.iter() {
        if !cbor_key.is_text() {
            continue;
        }

        if cbor_key.as_text().expect("confirmed as text") == key {
            return Some(cbor_value);
        }
    }
    None
}

#[cfg(test)]
/// Retrieves the value of a key from a CBOR map if it's a map itself.
pub fn cbor_inner_map_value<'a>(
    document_type: &'a [(Value, Value)],
    key: &'a str,
) -> Option<&'a Vec<(Value, Value)>> {
    let key_value = get_key_from_cbor_map(document_type, key)?;
    if let Value::Map(map_value) = key_value {
        return Some(map_value);
    }
    None
}
