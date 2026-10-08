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

#[cfg(feature = "fixtures-and-mocks")]
use dpp::data_contract::schema::DataContractSchemaMethodsV0;
#[cfg(feature = "fixtures-and-mocks")]
use dpp::platform_value::Value as PlatformValue;

#[cfg(test)]
use ciborium::value::Value;

#[cfg(test)]
use crate::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfo;
#[cfg(test)]
use crate::util::object_size_info::DocumentInfo::DocumentRefInfo;
#[cfg(test)]
use crate::util::object_size_info::{DataContractOwnedResolvedInfo, OwnedDocumentInfo};
#[cfg(test)]
use crate::util::storage_flags::StorageFlags;
#[cfg(test)]
use dpp::data_contract::accessors::v0::DataContractV0Getters;
#[cfg(test)]
use dpp::data_contract::document_type::random_document::{
    CreateRandomDocument, DocumentFieldFillSize, DocumentFieldFillType,
};
#[cfg(test)]
use dpp::document::DocumentV0Setters;
#[cfg(test)]
use dpp::platform_value;
#[cfg(test)]
use dpp::platform_value::Bytes32;
#[cfg(test)]
use dpp::voting::vote_info_storage::contested_document_vote_poll_stored_info::ContestedDocumentVotePollStoredInfo;
#[cfg(test)]
use rand::rngs::StdRng;
#[cfg(test)]
use rand::SeedableRng;

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
/// The identity id of DPNS name contender `n` of [`add_dpns_name_contenders`]: `n + 1` big
/// endian in its first 8 bytes, so contenders sort by `n`.
pub(crate) fn dpns_name_contender_id(n: u64) -> Identifier {
    let mut id = [0u8; 32];
    id[..8].copy_from_slice(&(n + 1).to_be_bytes());
    Identifier::from(id)
}

#[cfg(test)]
/// Adds contenders `contenders` (see [`dpns_name_contender_id`]) to the contest on the DPNS name
/// `label` under `dash`, written straight to Drive at `block_info` with no validation. Contender
/// `n`'s document is created at `created_at(n)`. Contender 0 starts the contest, writing its
/// stored info. Returns the poll.
pub(crate) fn add_dpns_name_contenders(
    drive: &Drive,
    dpns_contract: &DataContract,
    label: &str,
    contenders: std::ops::Range<u64>,
    created_at: impl Fn(u64) -> TimestampMillis,
    block_info: &BlockInfo,
    platform_version: &PlatformVersion,
) -> ContestedDocumentResourceVotePollWithContractInfo {
    let document_type = dpns_contract
        .document_type_for_name("domain")
        .expect("expected the domain document type");
    let vote_poll = ContestedDocumentResourceVotePollWithContractInfo {
        contract: DataContractOwnedResolvedInfo::OwnedDataContract(dpns_contract.clone()),
        document_type_name: "domain".to_string(),
        index_name: "parentNameAndLabel".to_string(),
        index_values: vec![
            platform_value::Value::Text("dash".to_string()),
            platform_value::Value::Text(label.to_string()),
        ],
    };
    let mut rng = StdRng::seed_from_u64(contenders.start);
    for n in contenders {
        let owner_id = dpns_name_contender_id(n);
        let mut document = document_type
            .random_document_with_params(
                owner_id,
                Bytes32::random_with_rng(&mut rng),
                Some(created_at(n)),
                Some(block_info.height),
                Some(block_info.core_height),
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::MinDocumentFillSize,
                &mut rng,
                platform_version,
            )
            .expect("expected a random domain");
        document.set("parentDomainName", "dash".into());
        document.set("normalizedParentDomainName", "dash".into());
        document.set("label", label.into());
        document.set("normalizedLabel", label.into());
        document.set("records.identity", owner_id.into());
        document.set("subdomainRules.allowSubdomains", false.into());
        let stored_info = (n == 0).then(|| {
            ContestedDocumentVotePollStoredInfo::new(*block_info, platform_version)
                .expect("expected the poll's stored info")
        });
        drive
            .add_contested_document(
                OwnedDocumentInfo {
                    document_info: DocumentRefInfo((
                        &document,
                        StorageFlags::optional_default_as_cow(),
                    )),
                    owner_id: Some(owner_id.to_buffer()),
                },
                vote_poll.clone(),
                false,
                stored_info,
                block_info,
                true,
                None,
                platform_version,
            )
            .expect("expected to add the contender");
    }
    vote_poll
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

#[cfg(feature = "fixtures-and-mocks")]
/// The yappr-likes `contract` with its `like` type read whole through
/// `byLiker`: without a proof, a chained read is refused when its inner index
/// lacks a property, as a documents query through that index is, and
/// `byLiker` lacks the like's optional hashtag; so the like keeps only what
/// `byLiker` holds (no hashtag, no `byHashtagPost`, no `where` on `postId`,
/// which takes the first position). Shared by Drive's and drive-abci's chained
/// query tests.
pub fn with_likes_read_whole_through_by_liker(
    mut contract: DataContract,
    platform_version: &PlatformVersion,
) -> DataContract {
    let mut schemas: BTreeMap<String, PlatformValue> = contract
        .document_schemas()
        .into_iter()
        .map(|(name, schema)| (name, schema.clone()))
        .collect();
    let like = schemas.get_mut("like").expect("a like type");
    {
        let properties = like
            .get_mut("properties")
            .expect("like properties readable")
            .expect("like properties");
        properties.remove("hashtag").expect("a like hashtag");
        let post_id = properties
            .get_mut("postId")
            .expect("like postId readable")
            .expect("like postId");
        post_id
            .set_value("position", PlatformValue::U64(0))
            .expect("postId position set");
        post_id
            .get_mut("refersTo")
            .expect("postId refersTo readable")
            .expect("postId refersTo")
            .remove("where")
            .expect("postId where");
    }
    like.get_mut("indices")
        .expect("like indices readable")
        .expect("like indices")
        .as_array_mut()
        .expect("like indices are an array")
        .retain(|index| index.get_optional_str("name").ok().flatten() != Some("byHashtagPost"));
    let defs = contract.schema_defs().cloned();
    contract
        .set_document_schemas(schemas, defs, true, &mut vec![], platform_version)
        .expect("expected the like read whole through byLiker to parse");
    contract
}
