use super::*;
use crate::drive::document::expiration::paths::documents_expirations_path_vec;
use crate::drive::document::expiration::pricing::document_expiration_cleanup_fee;
use crate::drive::document::fixture_contracts::{
    leave_out_optional_indexed_values, leave_out_optional_unique_values, leave_out_skip_properties,
    small_sums, CONTRACTS,
};
use crate::drive::document::make_document_reference;
use crate::drive::{Drive, RootTree};
use crate::util::grove_operations::DirectQueryType;
use crate::util::object_size_info::DocumentInfo::DocumentRefInfo;
use crate::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
use crate::util::storage_flags::StorageFlags;
use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
use crate::util::test_helpers::setup_contract;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::document_type::random_document::CreateRandomDocument;
use dpp::data_contract::DataContractFactory;
use dpp::document::{DocumentV0Getters, DocumentV0Setters};
use dpp::fee::fee_result::FeeResult;
use dpp::platform_value::{platform_value, Identifier};
use std::borrow::Cow;

/// The elements inserting `document` writes.
fn writes_of(
    contract: &DataContract,
    document_type: DocumentTypeRef,
    document: &Document,
) -> Vec<Write> {
    let platform_version = PlatformVersion::latest();
    let serialized = document
        .serialize(document_type, contract, platform_version)
        .expect("expected to serialize the document");
    document_writes(
        contract,
        document_type,
        document,
        &serialized,
        platform_version,
    )
    .expect("expected the writes")
}

/// Inserts `document` as a document create does: owned by its owner, in
/// one epoch.
fn insert(
    drive: &Drive,
    contract: &DataContract,
    document_type: DocumentTypeRef,
    document: &Document,
) -> Result<FeeResult, Error> {
    insert_at(drive, contract, document_type, document, 0)
}

/// [`insert`] in a block at `time_ms`.
fn insert_at(
    drive: &Drive,
    contract: &DataContract,
    document_type: DocumentTypeRef,
    document: &Document,
    time_ms: u64,
) -> Result<FeeResult, Error> {
    let flags = StorageFlags::new_single_epoch(0, Some(document.owner_id().to_buffer()));
    drive.add_document_for_contract(
        DocumentAndContractInfo {
            owned_document_info: OwnedDocumentInfo {
                document_info: DocumentRefInfo((document, Some(Cow::Owned(flags)))),
                owner_id: None,
            },
            contract,
            document_type,
        },
        false,
        BlockInfo::default_with_time(time_ms),
        true,
        None,
        PlatformVersion::latest(),
        None,
    )
}

/// The element at `path` and `key` below the document type tree, if any.
fn element(
    drive: &Drive,
    contract: &DataContract,
    type_name: &str,
    path: &[Vec<u8>],
    key: &[u8],
) -> Option<grovedb::Element> {
    let path: Vec<Vec<u8>> = [
        vec![
            vec![RootTree::DataContractDocuments as u8],
            contract.id().to_vec(),
            vec![1],
            type_name.as_bytes().to_vec(),
        ],
        path.to_vec(),
    ]
    .concat();
    match drive.grove_get_raw(
        path.as_slice().into(),
        key,
        DirectQueryType::StatefulDirectQuery,
        None,
        &mut vec![],
        &PlatformVersion::latest().drive,
    ) {
        Ok(element) => element,
        // A tree above it is missing too.
        Err(Error::GroveDB(error))
            if matches!(
                *error,
                grovedb::Error::PathNotFound(_)
                    | grovedb::Error::PathParentLayerNotFound(_)
                    | grovedb::Error::PathKeyNotFound(_)
            ) =>
        {
            None
        }
        Err(error) => panic!("expected to read: {error}"),
    }
}

/// Whether the element `write` names exists already.
fn exists(drive: &Drive, contract: &DataContract, type_name: &str, write: &Write) -> bool {
    if write.expiration {
        let path = [documents_expirations_path_vec(), write.path.clone()].concat();
        return drive
            .grove_get_raw(
                path.as_slice().into(),
                &write.key,
                DirectQueryType::StatefulDirectQuery,
                None,
                &mut vec![],
                &PlatformVersion::latest().drive,
            )
            .map(|element| element.is_some())
            .unwrap_or(false);
    }
    let type_name = write.referring_type.as_deref().unwrap_or(type_name);
    element(drive, contract, type_name, &write.path, &write.key).is_some()
}

/// The bytes a ranked entry's row adds: all of it for a new entry; for an
/// entry already stored, only what the row grew by (GroveDB bills a moved
/// row's bytes, up to the old row's size, as replaced).
fn row_bytes(
    write: &Write,
    before: Option<(u64, i64)>,
    after: (u64, i64),
    platform_version: &PlatformVersion,
) -> u64 {
    let row = write.ranking.as_ref().expect("a ranking row");
    let bytes = |(count, sum): (u64, i64)| {
        let (key, priced) =
            writes::ranking_row(row.axis, &row.entry_key, count, sum, platform_version)
                .expect("expected the row");
        u64::from(new_element_bytes(
            key.len() as u32,
            priced,
            NodeKind::of_tree(write.parent),
        ))
    };
    match before {
        None => bytes(after),
        Some(before) => bytes(after).saturating_sub(bytes(before)),
    }
}

fn apply(drive: &Drive, index: usize, path: &str) -> DataContract {
    setup_contract(
        drive,
        path,
        Some([index as u8 + 1; 32]),
        None,
        None::<fn(&mut DataContract)>,
        None,
        Some(PlatformVersion::latest()),
    )
}

#[test]
fn should_price_what_drive_charges() {
    let platform_version = PlatformVersion::latest();
    let credits_per_byte = platform_version
        .fee_version
        .storage
        .storage_disk_usage_credit_per_byte;
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));
    let mut mismatches = Vec::new();
    let mut inserts = 0;
    let mut values_already_stored = 0;

    for (index, path) in CONTRACTS.into_iter().enumerate() {
        let contract = apply(&drive, index, path);
        for (name, document_type) in contract.document_types() {
            let document_type = document_type.as_ref();
            for seed in 1..=11u64 {
                let mut document = document_type
                    .random_document(Some(seed), platform_version)
                    .expect("expected a random document");
                // Seed 11 is the document of middle sizes the SDK prices.
                if seed == 11 {
                    document = sized_document(
                        &contract,
                        document_type,
                        &Default::default(),
                        platform_version,
                    )
                    .expect("expected a sized document")
                    .0;
                    if document_type.indexes().values().any(|index| index.unique) {
                        document.set_id([0xab; 32].into());
                    }
                }
                small_sums(&mut document, document_type, seed);
                if seed <= 2 {
                    leave_out_optional_unique_values(&mut document, document_type);
                }
                // Seeds 3 and 4 leave out every skip property, so the
                // `skipIfAbsent` indexes skip them.
                if (3..=4).contains(&seed) {
                    leave_out_skip_properties(&mut document, document_type);
                }
                // Seeds 5 and 6 leave out the optional indexed properties,
                // all or every other one, so sibling branches meet missing
                // values beside present ones.
                if (5..=6).contains(&seed) {
                    leave_out_optional_indexed_values(&mut document, document_type, seed == 6);
                }
                // Seed 10 repeats the values of seed 9 under a new id, where
                // no unique index forbids it.
                if seed == 10 {
                    if document_type.indexes().values().any(|index| index.unique) {
                        continue;
                    }
                    let mut previous = document_type
                        .random_document(Some(9), platform_version)
                        .expect("expected a random document");
                    small_sums(&mut previous, document_type, 9);
                    previous.set_id(document.id());
                    previous.set_owner_id(document.owner_id());
                    document = previous;
                }

                let writes = writes_of(&contract, document_type, &document);
                let mut expected_bytes = 0u64;
                let entry_of = |write: &Write| {
                    let row = write.ranking.as_ref()?;
                    let type_name = write.referring_type.as_deref().unwrap_or(name);
                    element(
                        &drive,
                        &contract,
                        type_name,
                        &row.entry_path,
                        &row.entry_key,
                    )
                    .map(|entry| entry.count_sum_value_or_default())
                };
                let entries_before: Vec<Option<(u64, i64)>> =
                    writes.iter().map(&entry_of).collect();
                for write in writes.iter().filter(|write| write.ranking.is_none()) {
                    let present = write.if_absent && exists(&drive, &contract, name, write);
                    values_already_stored += usize::from(present);
                    if !present && !write.ephemeral {
                        expected_bytes += u64::from(new_element_bytes(
                            write.key.len() as u32,
                            write.element,
                            NodeKind::of_tree(write.parent),
                        ));
                    }
                }
                let fee = match insert(&drive, &contract, document_type, &document) {
                    Ok(fee) => fee,
                    // An indexOnly entry keyed by the repeated values collides.
                    Err(_) if seed == 10 && document_type.index_only() => continue,
                    Err(error) => panic!("{path} {name} seed {seed}: {error}"),
                };
                inserts += 1;
                // A ranked tree's rows carry the entry's aggregate after the
                // insert.
                for (write, before) in writes.iter().zip(&entries_before) {
                    // Rows under a ttl window are priced as processing.
                    if write.ranking.is_none() || write.ephemeral {
                        continue;
                    }
                    let after = entry_of(write).expect("expected the ranked entry");
                    expected_bytes += row_bytes(write, *before, after, platform_version);
                }
                if fee.storage_fee != expected_bytes * credits_per_byte {
                    mismatches.push(format!(
                        "{path} {name} seed {seed}: Drive charged {} bytes, the model says {}",
                        fee.storage_fee / credits_per_byte,
                        expected_bytes
                    ));
                }
            }
        }
    }

    assert!(
        mismatches.is_empty(),
        "the model disagrees with Drive:\n{}",
        mismatches.join("\n")
    );
    assert!(inserts > 100, "only {inserts} inserts ran");
    assert!(
        values_already_stored > 0,
        "no insert found a value already stored"
    );
}

#[test]
fn should_estimate_the_processing_of_the_writes_within_a_factor_of_two() {
    let platform_version = PlatformVersion::latest();
    let mut ratios = Vec::new();
    for index in [0usize, 3, 4, 7, 9, 15, 22] {
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        let contract = apply(&drive, index, CONTRACTS[index]);
        for (name, document_type) in contract.document_types() {
            let document_type = document_type.as_ref();
            let type_entries = u64::from(!document_type.index_only())
                + document_type.index_structure().sub_levels().len() as u64;
            for seed in 0..=100u64 {
                let mut document = document_type
                    .random_document(Some(seed), platform_version)
                    .expect("expected a random document");
                small_sums(&mut document, document_type, seed);
                let writes = writes_of(&contract, document_type, &document);
                let written: Vec<bool> = writes
                    .iter()
                    .map(|write| !write.if_absent || !exists(&drive, &contract, name, write))
                    .collect();
                let known = processed_writes(&writes, &written);
                let mut assumptions = CostAssumptions::new(platform_version);
                assumptions.existing_documents = seed;
                let estimate = write_processing(
                    &writes,
                    &known,
                    type_entries,
                    &assumptions,
                    platform_version,
                );
                let fee =
                    insert(&drive, &contract, document_type, &document).unwrap_or_else(|error| {
                        panic!("{} {name} seed {seed}: {error}", CONTRACTS[index])
                    });
                if [1, 10, 100].contains(&seed) {
                    ratios.push((
                        format!("{} {name} after {seed} documents", CONTRACTS[index]),
                        estimate as f64 / fee.processing_fee as f64,
                    ));
                }
            }
        }
    }
    assert!(ratios.len() > 30, "only {} checkpoints", ratios.len());
    let outside: Vec<String> = ratios
        .iter()
        .filter(|(_, ratio)| !(0.5..=2.0).contains(ratio))
        .map(|(at, ratio)| format!("{at}: estimate / Drive = {ratio:.2}"))
        .collect();
    assert!(
        outside.is_empty(),
        "the processing estimate strays from Drive:\n{}",
        outside.join("\n")
    );
}

fn contract_with(schemas: Value) -> DataContract {
    DataContractFactory::new(PlatformVersion::latest().protocol_version)
        .expect("factory")
        .create_with_value_config(Identifier::from([7; 32]), 0, schemas, None, None)
        .expect("contract")
        .data_contract_owned()
}

fn note_schema() -> Value {
    platform_value!({
        "type": "object",
        "documentsMutable": true,
        "canBeDeleted": true,
        "properties": {
            "tag": { "type": "string", "maxLength": 20, "position": 0 },
            "code": { "type": "string", "maxLength": 20, "position": 1 },
            "text": { "type": "string", "maxLength": 60, "position": 2 },
        },
        "indices": [
            { "name": "byTag", "properties": [{ "tag": "asc" }] },
            { "name": "byTagText", "properties": [{ "tag": "asc" }, { "text": "asc" }] },
            { "name": "byCode", "properties": [{ "code": "asc" }], "unique": true },
        ],
        "required": ["tag", "code", "text"],
        "additionalProperties": false,
    })
}

fn note_document(contract: &DataContract) -> Document {
    let document_type = contract.document_type_for_name("note").expect("note");
    let mut document = document_type
        .random_document(Some(1), PlatformVersion::latest())
        .expect("expected a random document");
    document.set("tag", Value::Text("travel".to_string()));
    document.set("code", Value::Text("n-1".to_string()));
    document.set("text", Value::Text("a note about a trip".to_string()));
    document
}

#[test]
fn should_split_the_storage_into_primary_storage_and_each_index() {
    let platform_version = PlatformVersion::latest();
    let contract = contract_with(platform_value!({ "note": note_schema() }));
    let note = contract.document_type_for_name("note").expect("note");
    let cost = document_create_cost(
        &contract,
        note,
        &note_document(&contract),
        &CostAssumptions::new(platform_version),
        platform_version,
    )
    .expect("expected a cost");

    // Every byte is primary storage or an index's, a shared layer once.
    let mut counted = cost.primary_bytes;
    let mut shared_once = std::collections::BTreeMap::new();
    for element in cost
        .elements
        .iter()
        .filter(|element| element.indexes.len() > 1)
    {
        shared_once.insert(element.path.clone(), element.bytes);
    }
    for index in &cost.indexes {
        counted.add(index.own_bytes);
    }
    counted.new_values += shared_once.values().sum::<u64>();
    assert_eq!(counted.new_values, cost.storage_bytes.new_values);

    let by_tag = cost
        .indexes
        .iter()
        .find(|index| index.name == "byTag")
        .expect("byTag");
    assert_eq!(by_tag.shared_with, vec!["byTagText".to_string()]);
    assert!(by_tag.shared_bytes.new_values > 0);
    // The tag's value tree already exists for a known tag; only the
    // document's own entries are written.
    assert!(by_tag.shared_bytes.known_values == 0);
    assert!(by_tag.own_bytes.known_values > 0);
    assert!(by_tag.own_bytes.known_values < by_tag.own_bytes.new_values);

    // A unique value is new whatever is stored.
    let by_code = cost
        .indexes
        .iter()
        .find(|index| index.name == "byCode")
        .expect("byCode");
    assert!(by_code.shared_with.is_empty());
    assert_eq!(by_code.own_bytes.known_values, by_code.own_bytes.new_values);

    assert_eq!(
        cost.storage_credits.new_values,
        cost.storage_bytes.new_values * cost.credits_per_byte
    );
    // A delete in the same epoch refunds all but the epoch's share.
    let same_epoch = cost.refund_same_epoch.expect("a refund");
    let after_one_year = cost.refund_after_one_year.expect("a refund");
    assert!(same_epoch.new_values < cost.storage_credits.new_values);
    assert!(same_epoch.new_values > cost.storage_credits.new_values * 99 / 100);
    assert!(after_one_year.new_values < same_epoch.new_values);
}

#[test]
fn should_refund_a_document_only_a_consume_deletes_as_one_its_owner_deletes() {
    // A consume refunds the owner as the owner's delete would, so the estimate is the same;
    // a type whose documents nothing deletes refunds nothing
    let platform_version = PlatformVersion::latest();
    let refunds = |can_be_deleted: Value| {
        let mut schema = note_schema();
        schema
            .insert("canBeDeleted".to_string(), can_be_deleted)
            .expect("expected to set canBeDeleted");
        let contract = contract_with(platform_value!({ "note": schema }));
        let note = contract.document_type_for_name("note").expect("note");
        let cost = document_create_cost(
            &contract,
            note,
            &note_document(&contract),
            &CostAssumptions::new(platform_version),
            platform_version,
        )
        .expect("expected a cost");
        (cost.refund_same_epoch, cost.refund_after_one_year)
    };

    let (same_epoch, after_one_year) = refunds(Value::Bool(true));
    assert!(same_epoch.expect("a refund").new_values > 0);
    assert!(after_one_year.expect("a refund").new_values > 0);
    assert_eq!(
        refunds(Value::Text("onlyWhenConsumed".to_string())),
        (same_epoch, after_one_year)
    );
    assert_eq!(
        refunds(Value::Bool(false)),
        (Some(Scenarios::default()), Some(Scenarios::default()))
    );
}

#[test]
fn should_charge_no_layer_to_a_skip_index_that_skips_the_document() {
    let platform_version = PlatformVersion::latest();
    let contract = contract_with(platform_value!({ "post": {
        "type": "object",
        "documentsMutable": true,
        "canBeDeleted": true,
        "properties": {
            "language": { "type": "string", "maxLength": 8, "position": 0 },
            "hashtag": { "type": "string", "maxLength": 20, "position": 1 },
        },
        "indices": [
            { "name": "byLanguage", "properties": [{ "language": "asc" }] },
            {
                "name": "byLanguageHashtag",
                "properties": [{ "language": "asc" }, { "hashtag": "asc" }],
                "skipIfAbsent": ["hashtag"],
            },
        ],
        "required": ["language"],
        "additionalProperties": false,
    }}));
    let post = contract.document_type_for_name("post").expect("post");
    let mut document = post
        .random_document(Some(1), platform_version)
        .expect("expected a random document");
    document.set_properties(
        [("language".to_string(), Value::Text("en".to_string()))]
            .into_iter()
            .collect(),
    );
    let cost = document_create_cost(
        &contract,
        post,
        &document,
        &CostAssumptions::new(platform_version),
        platform_version,
    )
    .expect("expected a cost");

    // The language's value tree is byLanguage's alone: byLanguageHashtag
    // skips a post without a hashtag and adds no layer.
    let shares = |name: &str| {
        cost.indexes
            .iter()
            .find(|index| index.name == name)
            .map(|index| {
                (
                    index.shared_with.clone(),
                    index.shared_bytes.new_values,
                    index.own_bytes.new_values,
                )
            })
            .unwrap_or_default()
    };
    let (by_language_shared_with, by_language_shared_bytes, by_language_own_bytes) =
        shares("byLanguage");
    assert!(by_language_shared_with.is_empty());
    assert_eq!(by_language_shared_bytes, 0);
    assert!(by_language_own_bytes > 0);
    assert_eq!(shares("byLanguageHashtag"), (vec![], 0, 0));
}

#[test]
fn should_skip_on_a_derived_value_only_when_it_or_its_reference_is_absent() {
    // A reply may quote a permanent post; `byBodyQuotedOwner` files it under the quoted post's
    // owner, which the reply never stores, and skips a reply without one. It shares the body's
    // value tree with `byBody`, which skips nothing.
    let platform_version = PlatformVersion::latest();
    let contract = contract_with(platform_value!({
        "post": {
            "type": "object",
            "documentsMutable": false,
            "canBeDeleted": false,
            "properties": {
                "text": { "type": "string", "maxLength": 50, "position": 0 },
            },
            "additionalProperties": false,
        },
        "reply": {
            "type": "object",
            "documentsMutable": false,
            "canBeDeleted": true,
            "properties": {
                "body": { "type": "string", "maxLength": 20, "position": 0 },
                "quoteId": {
                    "type": "array",
                    "byteArray": true,
                    "minItems": 32,
                    "maxItems": 32,
                    "contentMediaType": "application/x.dash.dpp.identifier",
                    "refersTo": { "type": "permanentDocument", "documentType": "post" },
                    "position": 1,
                },
            },
            "indices": [
                { "name": "byBody", "properties": [{ "body": "asc" }] },
                {
                    "name": "byBodyQuotedOwner",
                    "properties": [{ "body": "asc" }, { "quoteId.$ownerId": "asc" }],
                    "skipIfAbsent": ["quoteId.$ownerId"],
                },
            ],
            "required": ["body"],
            "additionalProperties": false,
        },
    }));
    let reply = contract.document_type_for_name("reply").expect("reply");
    let quoted = Value::Identifier([5; 32]);
    let quoted_owner = Value::Identifier([6; 32]);
    // (case, the quote, the derived value the caller puts in, whether the reply takes part)
    let cases = [
        ("unresolved, quoting", Some(&quoted), None, true),
        ("unresolved, quoting nothing", None, None, false),
        (
            "resolved to null, quoting",
            Some(&quoted),
            Some(&Value::Null),
            false,
        ),
        (
            "resolved, quoting",
            Some(&quoted),
            Some(&quoted_owner),
            true,
        ),
    ];
    let mut own_bytes_taking_part = Vec::new();
    for (case, quote, derived, takes_part) in cases {
        let mut document = reply
            .random_document(Some(1), platform_version)
            .expect("expected a random document");
        let mut properties: BTreeMap<String, Value> =
            [("body".to_string(), Value::Text("hi".to_string()))]
                .into_iter()
                .collect();
        if let Some(quote) = quote {
            properties.insert("quoteId".to_string(), quote.clone());
        }
        // Under the derived name, as Drive puts it: not a path into `quoteId`
        if let Some(derived) = derived {
            properties.insert("quoteId.$ownerId".to_string(), derived.clone());
        }
        document.set_properties(properties);
        let cost = document_create_cost(
            &contract,
            reply,
            &document,
            &CostAssumptions::new(platform_version),
            platform_version,
        )
        .expect("expected a cost");
        let shares = |name: &str| {
            cost.indexes
                .iter()
                .find(|index| index.name == name)
                .map(|index| {
                    (
                        index.shared_with.clone(),
                        index.shared_bytes.new_values,
                        index.own_bytes.new_values,
                    )
                })
                .unwrap_or_default()
        };
        let (by_body_shared_with, by_body_shared_bytes, by_body_own_bytes) = shares("byBody");
        assert!(by_body_own_bytes > 0, "{case}: byBody keeps its entry");
        let (shared_with, shared_bytes, own_bytes) = shares("byBodyQuotedOwner");
        if takes_part {
            // The body's value tree is shared, and the derived index pays its own layer
            assert_eq!(
                by_body_shared_with,
                vec!["byBodyQuotedOwner".to_string()],
                "{case}"
            );
            assert!(by_body_shared_bytes > 0, "{case}");
            assert_eq!(shared_with, vec!["byBody".to_string()], "{case}");
            assert!(shared_bytes > 0, "{case}");
            assert!(own_bytes > 0, "{case}");
            own_bytes_taking_part.push(own_bytes);
        } else {
            // Skipped: no share of the body's value tree and no layer of its own
            assert!(by_body_shared_with.is_empty(), "{case}");
            assert_eq!(by_body_shared_bytes, 0, "{case}");
            assert_eq!(
                (shared_with, shared_bytes, own_bytes),
                (vec![], 0, 0),
                "{case}"
            );
        }
    }
    // An owner the caller did not resolve is priced at an identifier's size, as a resolved one
    assert_eq!(own_bytes_taking_part.len(), 2);
    assert_eq!(own_bytes_taking_part[0], own_bytes_taking_part[1]);
}

#[test]
fn should_add_the_contract_charges() {
    let platform_version = PlatformVersion::latest();
    let mut schema = note_schema();
    schema
        .insert(
            "actionFees".to_string(),
            platform_value!({ "pricing": "fixed", "create": { "owner": 1000u64, "moderators": 500u64 } }),
        )
        .expect("expected to set the action fees");
    let contract = contract_with(platform_value!({ "note": schema }));
    let note = contract.document_type_for_name("note").expect("note");
    let cost = document_create_cost(
        &contract,
        note,
        &note_document(&contract),
        &CostAssumptions::new(platform_version),
        platform_version,
    )
    .expect("expected a cost");
    assert!(cost.contract_charges.iter().any(|charge| matches!(
        charge,
        ContractCharge::ActionFee { charged, .. } if charged.owner == 1000 && charged.moderators == 500
    )));
    let fees_and_storage = cost
        .storage_credits
        .new_values
        .saturating_add(cost.processing_credits().new_values);
    assert_eq!(cost.total_credits().new_values, fees_and_storage + 1500);

    // A DPNS name may be contested: its create shows the vote fund.
    let dpns = dpp::system_data_contracts::load_system_data_contract(
        dpp::system_data_contracts::SystemDataContract::DPNS,
        platform_version,
    )
    .expect("dpns");
    let domain = dpns.document_type_for_name("domain").expect("domain");
    let document = domain
        .random_document(Some(1), platform_version)
        .expect("expected a random document");
    let cost = document_create_cost(
        &dpns,
        domain,
        &document,
        &CostAssumptions::new(platform_version),
        platform_version,
    )
    .expect("expected a cost");
    assert!(cost.contract_charges.iter().any(|charge| matches!(
        charge,
        ContractCharge::ContestFund { credits, .. }
            if *credits == platform_version
                .fee_version
                .vote_resolution_fund_fees
                .contested_document_vote_resolution_fund_required_amount
    )));
}

#[test]
fn should_charge_a_later_document_with_known_values_no_more_than_the_first() {
    let platform_version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));
    for (index, path) in CONTRACTS.into_iter().enumerate() {
        let contract = apply(&drive, index, path);
        for (name, document_type) in contract.document_types() {
            let cost = document_type_create_cost(
                &contract,
                document_type.as_ref(),
                &Default::default(),
                &CostAssumptions::new(platform_version),
                platform_version,
            )
            .expect("expected a cost");
            let total = cost.total_credits();
            assert!(
                total.known_values <= total.new_values,
                "{path} {name}: {} credits with known values, {} with new ones",
                total.known_values,
                total.new_values
            );
            assert!(cost.storage_bytes.known_values <= cost.storage_bytes.new_values);
        }
    }
}

#[test]
fn should_price_a_time_window_with_a_ttl_without_flags_as_processing() {
    let platform_version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));
    let contract = contract_with(platform_value!({ "ping": {
        "type": "object",
        "documentsMutable": true,
        "canBeDeleted": true,
        "properties": { "tag": { "type": "string", "maxLength": 20, "position": 0 } },
        "indices": [{
            "name": "recent",
            "properties": [{ "$createdAt": "asc" }, { "tag": "asc" }],
            "timeRange": { "on": "$createdAt", "range": 3600, "step": 900, "ttl": 86400 },
        }],
        "required": ["$createdAt", "tag"],
        "additionalProperties": false,
    }}));
    drive
        .apply_contract(
            &contract,
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            platform_version,
        )
        .expect("expected to apply the contract");
    let ping = contract.document_type_for_name("ping").expect("ping");
    let (document, _) = sized_document(&contract, ping, &Default::default(), platform_version)
        .expect("expected a sized document");

    // Every entry under the window is ephemeral: its references carry no
    // flags, as Drive strips them.
    let writes = writes_of(&contract, ping, &document);
    let flagless_reference = make_document_reference(&document, ping, None)
        .serialized_size(&platform_version.drive.grove_version)
        .expect("expected the reference size") as u32;
    let references: Vec<&Write> = writes
        .iter()
        .filter(|write| write.role == LayoutRole::Member)
        .collect();
    assert_eq!(
        references.len(),
        4,
        "a window of an hour every quarter hour"
    );
    for write in references {
        assert!(write.ephemeral && !write.flagged);
        assert_eq!(
            write.element,
            PricedElement::Serialized {
                serialized_len: flagless_reference
            }
        );
    }

    let cost = document_create_cost(
        &contract,
        ping,
        &document,
        &CostAssumptions::new(platform_version),
        platform_version,
    )
    .expect("expected a cost");
    assert!(cost
        .processing
        .iter()
        .any(|part| part.code == "timeWindowTtl" && part.credits.new_values > 0));
    // What is not ephemeral is storage, exactly as Drive charges it.
    let fee = insert_at(
        &drive,
        &contract,
        ping,
        &document,
        document.created_at().expect("created at"),
    )
    .expect("expected to insert the document");
    assert_eq!(fee.storage_fee, cost.storage_credits.new_values);
}

#[test]
fn should_price_every_window_of_an_integer_range_and_match_what_drive_charges() {
    // A `u32` price sampled as 1 would sit in the clamped bottom window only;
    // the sample is taken clear of it, so all three windows are priced, and
    // the estimate still equals what Drive charges for that document.
    let platform_version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));
    let contract = contract_with(platform_value!({ "listing": {
        "type": "object",
        "documentsMutable": true,
        "canBeDeleted": true,
        "properties": {
            "price": { "type": "integer", "minimum": 0, "maximum": 1_000_000, "position": 0 },
            "category": { "type": "string", "maxLength": 20, "position": 1 }
        },
        "indices": [{
            "name": "byPriceBand",
            "properties": [{ "price": "asc" }, { "category": "asc" }],
            "integerRange": { "on": "price", "range": 300, "step": 100 },
        }],
        "required": ["price", "category"],
        "additionalProperties": false,
    }}));
    drive
        .apply_contract(
            &contract,
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            platform_version,
        )
        .expect("expected to apply the contract");
    let listing = contract.document_type_for_name("listing").expect("listing");
    let (document, _) = sized_document(&contract, listing, &Default::default(), platform_version)
        .expect("expected a sized document");

    let references = writes_of(&contract, listing, &document)
        .into_iter()
        .filter(|write| write.role == LayoutRole::Member)
        .count();
    assert_eq!(references, 3, "one reference per window");

    let cost = document_create_cost(
        &contract,
        listing,
        &document,
        &CostAssumptions::new(platform_version),
        platform_version,
    )
    .expect("expected a cost");
    let fee = insert(&drive, &contract, listing, &document).expect("expected to insert");
    assert_eq!(fee.storage_fee, cost.storage_credits.new_values);
}

#[test]
fn should_count_trees_keyed_by_the_document_id_as_new_and_unrefunded() {
    let platform_version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));
    // Likes' `byPost` index is preallocated for every post, keyed by its id.
    let contract = apply(&drive, 10, CONTRACTS[10]);
    let post = contract.document_type_for_name("post").expect("post");
    let cost = document_type_create_cost(
        &contract,
        post,
        &Default::default(),
        &CostAssumptions::new(platform_version),
        platform_version,
    )
    .expect("expected a cost");
    assert!(cost.preallocated_bytes.new_values > 0);
    // Every tree keyed by the post's id, and what is under it, is new
    // whatever else is stored; a level keyed by a value another post shares
    // (its hashtag) may already exist.
    let (document, _) = sized_document(&contract, post, &Default::default(), platform_version)
        .expect("expected a sized document");
    let id = document.id().to_vec();
    let writes = writes_of(&contract, post, &document);
    let known = written_when_values_known(&writes, &id);
    let keyed_by_id: Vec<bool> = writes
        .iter()
        .zip(&known)
        .filter(|(write, _)| {
            write.referring_type.is_some() && (write.key == id || write.path.contains(&id))
        })
        .map(|(_, known)| *known)
        .collect();
    assert!(!keyed_by_id.is_empty());
    assert!(keyed_by_id.iter().all(|known| *known));
    // A delete keeps preallocated trees: they are not refunded.
    let kept = (cost.storage_bytes.new_values - cost.preallocated_bytes.new_values)
        * cost.credits_per_byte;
    assert!(cost.refund_same_epoch.expect("a refund").new_values < kept);
}

#[test]
fn should_refuse_an_earlier_protocol_version() {
    let platform_version = PlatformVersion::latest();
    let contract = contract_with(platform_value!({ "note": note_schema() }));
    let note = contract.document_type_for_name("note").expect("note");
    let version_13 = PlatformVersion::get(13).expect("protocol version 13");
    assert!(matches!(
        document_create_cost(
            &contract,
            note,
            &note_document(&contract),
            &CostAssumptions::new(platform_version),
            version_13
        ),
        Err(Error::Drive(DriveError::UnknownVersionMismatch { .. }))
    ));
}

#[test]
fn should_price_a_document_with_a_ttl_by_its_lifetime() {
    let platform_version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));
    let mut schema = note_schema();
    schema
        .insert("ttl".to_string(), Value::U32(86_400))
        .expect("expected to set the ttl");
    schema
        .insert(
            "required".to_string(),
            platform_value!(["$createdAt", "tag", "code", "text"]),
        )
        .expect("expected to set required");
    let contract = contract_with(platform_value!({ "note": schema }));
    drive
        .apply_contract(
            &contract,
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            platform_version,
        )
        .expect("expected to apply the contract");
    let note = contract.document_type_for_name("note").expect("note");
    let (document, _) = sized_document(&contract, note, &Default::default(), platform_version)
        .expect("expected a sized document");
    let cost = document_create_cost(
        &contract,
        note,
        &document,
        &CostAssumptions::new(platform_version),
        platform_version,
    )
    .expect("expected a cost");

    // A day costs far less per byte than keeping the bytes for good.
    assert!(
        cost.credits_per_byte
            < platform_version
                .fee_version
                .storage
                .storage_disk_usage_credit_per_byte
    );
    assert!(cost.expiration_bytes.new_values > 0);
    // A later document expires at another time: its entry is new too.
    assert_eq!(
        cost.expiration_bytes.known_values,
        cost.expiration_bytes.new_values
    );
    assert_eq!(cost.refund_same_epoch, Some(Scenarios::default()));
    let cleanup = cost
        .processing
        .iter()
        .find(|part| part.code == "ttlCleanup")
        .expect("the cleanup prepay");
    assert_eq!(
        cleanup.credits.new_values,
        document_expiration_cleanup_fee(note, cost.document_bytes, &platform_version.fee_version)
            .expect("the cleanup fee")
    );

    // Created in the block it is written in: all of its day left to live.
    let fee = insert_at(
        &drive,
        &contract,
        note,
        &document,
        document.created_at().expect("created at"),
    )
    .expect("expected to insert the document");
    assert_eq!(fee.storage_fee, cost.storage_credits.new_values);
}
