//! Range queries, ordering and range counts over an unsigned integer index
//! property on both sides of protocol version 14, which keys an unsigned
//! value by its big-endian bytes (`serialize_value_for_key` 1). Before, the
//! top bit was flipped as for a signed value, so the values from 128 up of a
//! `u8` sorted below the rest.

use crate::config::DriveConfig;
use crate::drive::Drive;
use crate::error::Error;
use crate::query::drive_document_count_query::drive_dispatcher::{
    DocumentCountRequest, DocumentCountResponse,
};
use crate::query::{
    CountMode, DriveDocumentCountQuery, DriveDocumentQuery, WhereClause, WhereOperator,
};
use crate::util::object_size_info::DocumentInfo::DocumentRefInfo;
use crate::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
use crate::util::storage_flags::StorageFlags;
use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::{DataContract, DataContractFactory};
use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
use dpp::document::{Document, DocumentV0, DocumentV0Getters};
use dpp::identifier::Identifier;
use dpp::platform_value::{platform_value, Value};
use dpp::version::PlatformVersion;
use std::borrow::Cow;
use std::collections::BTreeMap;

/// Grades on both sides of a `u8`'s top bit.
const GRADES: [u8; 7] = [5, 100, 127, 128, 150, 200, 255];

/// A contract whose `grade` type indexes a `u8` grade (its bounds pick u8)
/// with a range-countable index.
pub(crate) fn grade_contract(platform_version: &PlatformVersion) -> DataContract {
    let factory = DataContractFactory::new(platform_version.protocol_version)
        .expect("expected a contract factory");
    let grade = platform_value!({
        "type": "object",
        "properties": {
            "grade": {"type": "integer", "minimum": 0, "maximum": 255, "position": 0},
        },
        "required": ["grade"],
        "indices": [{
            "name": "byGrade",
            "properties": [{"grade": "asc"}],
            "countable": "countable",
            "rangeCountable": true,
        }],
        "additionalProperties": false,
    });
    factory
        .create_with_value_config(
            Identifier::new([7; 32]),
            0,
            platform_value!({ "grade": grade }),
            None,
            None,
        )
        .expect("expected the grade contract")
        .data_contract_owned()
}

/// A drive at `platform_version` holding the grade contract and one
/// document per grade of `GRADES`.
pub(crate) fn setup_grades(platform_version: &PlatformVersion) -> (Drive, DataContract) {
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));
    let contract = grade_contract(platform_version);
    drive
        .apply_contract(
            &contract,
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            platform_version,
        )
        .expect("expected to apply the grade contract");
    for (position, grade) in GRADES.iter().enumerate() {
        insert_grade(
            &drive,
            &contract,
            position as u8 + 1,
            *grade,
            platform_version,
        );
    }
    (drive, contract)
}

/// Inserts a grade document with id `[id; 32]`.
pub(crate) fn insert_grade(
    drive: &Drive,
    contract: &DataContract,
    id: u8,
    grade: u8,
    platform_version: &PlatformVersion,
) {
    let document: Document = DocumentV0 {
        id: Identifier::new([id; 32]),
        owner_id: Identifier::new([1; 32]),
        properties: BTreeMap::from([("grade".to_string(), Value::U8(grade))]),
        ..Default::default()
    }
    .into();
    insert_document(drive, contract, "grade", &document, platform_version);
}

/// Inserts `document` as a document of `document_type_name`.
pub(crate) fn insert_document(
    drive: &Drive,
    contract: &DataContract,
    document_type_name: &str,
    document: &Document,
    platform_version: &PlatformVersion,
) {
    drive
        .add_document_for_contract(
            DocumentAndContractInfo {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentRefInfo((
                        document,
                        Some(Cow::Owned(StorageFlags::SingleEpoch(0))),
                    )),
                    owner_id: None,
                },
                contract,
                document_type: contract
                    .document_type_for_name(document_type_name)
                    .expect("expected the document type"),
            },
            false,
            BlockInfo::default(),
            true,
            None,
            platform_version,
            None,
        )
        .expect("expected to insert the document");
}

pub(crate) fn above_100() -> WhereClause {
    WhereClause {
        field: "grade".to_string(),
        operator: WhereOperator::GreaterThan,
        value: Value::U8(100),
    }
}

fn from_100_to_200() -> WhereClause {
    WhereClause {
        field: "grade".to_string(),
        operator: WhereOperator::Between,
        value: Value::Array(vec![Value::U8(100), Value::U8(200)]),
    }
}

/// The aggregate count of the grades matching `where_clause`, without and
/// with a proof; the proved count is checked against the no-proof one.
pub(crate) fn count_grades(
    drive: &Drive,
    contract: &DataContract,
    where_clause: WhereClause,
    platform_version: &PlatformVersion,
) -> Result<u64, Error> {
    let document_type = contract
        .document_type_for_name("grade")
        .expect("expected the grade type");
    let drive_config = DriveConfig::default();
    let request = |prove| DocumentCountRequest {
        contract,
        document_type,
        where_clauses: vec![where_clause.clone()],
        order_clauses: Vec::new(),
        mode: CountMode::Aggregate,
        limit: None,
        prove,
        drive_config: &drive_config,
        resolved_time_ranges: vec![],
    };
    let count =
        match drive.execute_document_count_request(request(false), None, platform_version)? {
            DocumentCountResponse::Aggregate(count) => count,
            other => panic!("expected an aggregate count, got {other:?}"),
        };
    let proof = match drive.execute_document_count_request(request(true), None, platform_version)? {
        DocumentCountResponse::Proof(proof) => proof,
        other => panic!("expected a proof, got {other:?}"),
    };
    let index = document_type
        .indexes()
        .get("byGrade")
        .expect("expected the byGrade index");
    let (root_hash, proved_count) = DriveDocumentCountQuery {
        document_type,
        contract_id: contract.id().to_buffer(),
        document_type_name: "grade".to_string(),
        index,
        where_clauses: vec![where_clause],
    }
    .verify_aggregate_count_proof(&proof, platform_version)?;
    assert_eq!(
        root_hash,
        drive
            .grove
            .root_hash(None, &platform_version.drive.grove_version)
            .unwrap()
            .expect("expected the root hash")
    );
    assert_eq!(proved_count, count, "the proof proves the count");
    Ok(count)
}

/// The grades a document query returns, without and with a proof.
pub(crate) fn query_grades(
    drive: &Drive,
    contract: &DataContract,
    sql: &str,
    platform_version: &PlatformVersion,
) -> Vec<u8> {
    query_values(drive, contract, "grade", sql, "grade", platform_version)
        .into_iter()
        .map(|grade| u8::try_from(grade).expect("a grade is a u8"))
        .collect()
}

/// The values of `property` in the documents of `document_type_name` a
/// document query returns, without and with a proof; the proved documents are
/// checked against the others.
pub(crate) fn query_values(
    drive: &Drive,
    contract: &DataContract,
    document_type_name: &str,
    sql: &str,
    property: &str,
    platform_version: &PlatformVersion,
) -> Vec<u64> {
    let document_type = contract
        .document_type_for_name(document_type_name)
        .expect("expected the document type");
    let value_of = |document: &Document| {
        document
            .properties()
            .get(property)
            .and_then(|value| value.to_integer::<u64>().ok())
            .expect("expected the property")
    };
    let query =
        DriveDocumentQuery::from_sql_expr(sql, contract, Some(&drive.config), platform_version)
            .expect("expected the query to parse");
    let (documents, _, _) = query
        .execute_raw_results_no_proof(drive, None, None, platform_version)
        .expect("expected the query to execute");
    let values: Vec<u64> = documents
        .iter()
        .map(|bytes| {
            value_of(
                &Document::from_bytes(bytes, document_type, platform_version)
                    .expect("expected a document"),
            )
        })
        .collect();
    let (proof, _) = query
        .clone()
        .execute_with_proof(drive, None, None, platform_version)
        .expect("expected the query to prove");
    let (_, proved) = query
        .verify_proof(&proof, platform_version)
        .expect("expected the proof to verify");
    assert_eq!(
        proved.iter().map(value_of).collect::<Vec<u64>>(),
        values,
        "the proof proves the documents"
    );
    values
}

#[test]
fn should_count_grades_above_100_in_value_order() {
    let platform_version = PlatformVersion::latest();
    let (drive, contract) = setup_grades(platform_version);
    assert_eq!(
        count_grades(&drive, &contract, above_100(), platform_version).expect("expected the count"),
        5,
        "127, 128, 150, 200 and 255"
    );
}

#[test]
fn should_count_grades_between_100_and_200() {
    let platform_version = PlatformVersion::latest();
    let (drive, contract) = setup_grades(platform_version);
    assert_eq!(
        count_grades(&drive, &contract, from_100_to_200(), platform_version)
            .expect("expected the count"),
        5,
        "100, 127, 128, 150 and 200"
    );
}

#[test]
fn should_return_grades_above_100_in_order_both_ways() {
    let platform_version = PlatformVersion::latest();
    let (drive, contract) = setup_grades(platform_version);
    assert_eq!(
        query_grades(
            &drive,
            &contract,
            "select * from grade where grade > 100 order by grade asc",
            platform_version,
        ),
        vec![127, 128, 150, 200, 255]
    );
    assert_eq!(
        query_grades(
            &drive,
            &contract,
            "select * from grade where grade > 100 order by grade desc",
            platform_version,
        ),
        vec![255, 200, 150, 128, 127]
    );
    assert_eq!(
        query_grades(
            &drive,
            &contract,
            "select * from grade where grade >= 0 order by grade asc limit 3",
            platform_version,
        ),
        vec![5, 100, 127]
    );
}

/// Protocol version 13 keys an unsigned value with its top bit flipped: the
/// grades from 128 up sort below 5, so a range from 100 sees only 127 and a
/// range from 100 to 200 has its bounds the wrong way round. Unchanged.
#[test]
fn should_keep_the_flipped_order_before_protocol_version_14() {
    let platform_version = PlatformVersion::get(13).expect("expected protocol version 13");
    let (drive, contract) = setup_grades(platform_version);
    assert_eq!(
        count_grades(&drive, &contract, above_100(), platform_version).expect("expected the count"),
        1
    );
    assert!(count_grades(&drive, &contract, from_100_to_200(), platform_version).is_err());
    assert_eq!(
        query_grades(
            &drive,
            &contract,
            "select * from grade where grade > 100 order by grade asc",
            platform_version,
        ),
        vec![127]
    );
}
