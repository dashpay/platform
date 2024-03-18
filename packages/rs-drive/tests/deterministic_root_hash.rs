// MIT LICENSE
//
// Copyright (c) 2021 Dash Core Group
//
// Permission is hereby granted, free of charge, to any
// person obtaining a copy of this software and associated
// documentation files (the "Software"), to deal in the
// Software without restriction, including without
// limitation the rights to use, copy, modify, merge,
// publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software
// is furnished to do so, subject to the following
// conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions
// of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
// ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
// TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
// PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
// SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
// CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
// OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
// IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.
//

//! Deterministic Root Hash Tests

use grovedb_path::SubtreePath;

#[cfg(feature = "full")]
use std::borrow::Cow;
#[cfg(feature = "full")]
use std::option::Option::None;

#[cfg(feature = "full")]
use dpp::document::Document;
#[cfg(feature = "full")]
use dpp::util::cbor_serializer;
#[cfg(feature = "full")]
use drive::common;

#[cfg(feature = "full")]
use grovedb::{Element, Transaction, TransactionArg};
#[cfg(feature = "full")]
use rand::seq::SliceRandom;
#[cfg(feature = "full")]
use rand::{Rng, SeedableRng};
#[cfg(feature = "full")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "full")]
use drive::drive::flags::StorageFlags;

#[cfg(feature = "full")]
use drive::drive::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
#[cfg(feature = "full")]
use drive::drive::{Drive, RootTree};

#[cfg(feature = "full")]
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::DataContract;
use dpp::document::serialization_traits::DocumentCborMethodsV0;
use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
use dpp::version::PlatformVersion;

use drive::drive::object_size_info::DocumentInfo::DocumentRefInfo;
use drive::tests::helpers::setup::setup_drive;

#[cfg(feature = "full")]
/// Contains the unique ID for a Dash identity.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Records {
    dash_unique_identity_id: Vec<u8>,
}

#[cfg(feature = "full")]
/// Info about a DPNS name.
// In the real dpns label is required, we make it optional here for a test
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Domain {
    #[serde(rename = "$id")]
    id: Vec<u8>,
    #[serde(rename = "$ownerId")]
    owner_id: Vec<u8>,
    label: Option<String>,
    normalized_label: Option<String>,
    normalized_parent_domain_name: String,
    records: Records,
    preorder_salt: Vec<u8>,
    subdomain_rules: bool,
}

#[cfg(feature = "full")]
impl Domain {
    /// Creates domains with random data for a given normalized parent domain name.
    fn random_domains_in_parent(
        count: u32,
        seed: u64,
        normalized_parent_domain_name: &str,
    ) -> Vec<Self> {
        let first_names =
            common::text_file_strings("tests/supporting_files/contract/family/first-names.txt");
        let mut vec: Vec<Domain> = Vec::with_capacity(count as usize);

        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
        for _i in 0..count {
            let label = first_names.choose(&mut rng).unwrap();
            let domain = Domain {
                id: Vec::from(rng.gen::<[u8; 32]>()),
                owner_id: Vec::from(rng.gen::<[u8; 32]>()),
                label: Some(label.clone()),
                normalized_label: Some(label.to_lowercase()),
                normalized_parent_domain_name: normalized_parent_domain_name.to_string(),
                records: Records {
                    dash_unique_identity_id: Vec::from(rng.gen::<[u8; 32]>()),
                },
                preorder_salt: Vec::from(rng.gen::<[u8; 32]>()),
                subdomain_rules: false,
            };
            vec.push(domain);
        }
        vec
    }
}

#[cfg(feature = "full")]
/// Creates and adds to a contract domains with random data.
pub fn add_domains_to_contract(
    drive: &Drive,
    contract: &DataContract,
    transaction: TransactionArg,
    count: u32,
    seed: u64,
) {
    let platform_version = PlatformVersion::latest();
    let domains = Domain::random_domains_in_parent(count, seed, "dash");
    for domain in domains {
        let value = serde_json::to_value(domain).expect("serialized domain");
        let document_cbor = cbor_serializer::serializable_value_to_cbor(&value, Some(0))
            .expect("expected to serialize to cbor");
        let document = Document::from_cbor(document_cbor.as_slice(), None, None, platform_version)
            .expect("document should be properly deserialized");
        let document_type = contract
            .document_type_for_name("domain")
            .expect("expected to get document type");

        let storage_flags = Some(Cow::Owned(StorageFlags::SingleEpoch(0)));

        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((&document, storage_flags)),
                        owner_id: None,
                    },
                    contract,
                    document_type,
                },
                true,
                BlockInfo::genesis(),
                true,
                transaction,
                platform_version,
            )
            .expect("document should be inserted");
    }
}

#[cfg(feature = "full")]
/// Tests that the root hash is being calculated correctly after inserting empty subtrees into
/// the root tree and the DPNS contract.
fn test_root_hash_with_batches(drive: &Drive, db_transaction: &Transaction) {
    let platform_version = PlatformVersion::latest();

    // [1644293142180] INFO (35 on bf3bb2a2796a): createTree
    //     path: []
    //     pathHash: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    //     key: "00"
    //     value: "0000000000000000000000000000000000000000000000000000000000000000"
    //     valueHash: "66687aadf862bd776c8fc18b8e9f8e20089714856ee233b3902a591d0d5f2925"
    //     useTransaction: true
    //     type: "tree"
    //     method: "insert"
    //     appHash: "0000000000000000000000000000000000000000000000000000000000000000"

    drive
        .grove
        .insert(
            SubtreePath::empty(),
            Into::<&[u8; 1]>::into(RootTree::Identities),
            Element::empty_tree(),
            None,
            Some(db_transaction),
        )
        .unwrap()
        .expect("should insert tree");

    let app_hash = drive
        .grove
        .root_hash(Some(db_transaction))
        .unwrap()
        .expect("should return app hash");

    assert_eq!(
        hex::encode(app_hash),
        "1e4cda5099f53c9accd6e68321b93519ee998fa2ec754002b0e0f1407953bc58"
    );

    //[1644293142181] INFO (35 on bf3bb2a2796a): createTree
    //     path: []
    //     pathHash: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    //     key: "02"
    //     value: "0000000000000000000000000000000000000000000000000000000000000000"
    //     valueHash: "66687aadf862bd776c8fc18b8e9f8e20089714856ee233b3902a591d0d5f2925"
    //     useTransaction: true
    //     type: "tree"
    //     method: "insert"
    //     appHash: "f5a5fd42d16a20302798ef6ed309979b43003d2320d9f0e8ea9831a92759fb4b"

    drive
        .grove
        .insert(
            SubtreePath::empty(),
            Into::<&[u8; 1]>::into(RootTree::UniquePublicKeyHashesToIdentities),
            Element::empty_tree(),
            None,
            Some(db_transaction),
        )
        .unwrap()
        .expect("should insert tree");

    let app_hash = drive
        .grove
        .root_hash(Some(db_transaction))
        .unwrap()
        .expect("should return app hash");

    assert_eq!(
        hex::encode(app_hash),
        "f48c73a70469df637f0683b8341479c8561aceb7ebeb2c95200f5788a7387cd6"
    );

    // [1644293142181] INFO (35 on bf3bb2a2796a): createTree
    //     path: []
    //     pathHash: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    //     key: "01"
    //     value: "0000000000000000000000000000000000000000000000000000000000000000"
    //     valueHash: "66687aadf862bd776c8fc18b8e9f8e20089714856ee233b3902a591d0d5f2925"
    //     useTransaction: true
    //     type: "tree"
    //     method: "insert"
    //     appHash: "7a0501f5957bdf9cb3a8ff4966f02265f968658b7a9c62642cba1165e86642f5"

    drive
        .grove
        .insert(
            SubtreePath::empty(),
            Into::<&[u8; 1]>::into(RootTree::DataContractDocuments),
            Element::empty_tree(),
            None,
            Some(db_transaction),
        )
        .unwrap()
        .expect("should insert tree");

    let app_hash = drive
        .grove
        .root_hash(Some(db_transaction))
        .unwrap()
        .expect("should return app hash");

    assert_eq!(
        hex::encode(app_hash),
        "4238e5fe09b99e0f7ea2a4bcea60efd37cf2638743883da547e8fbe254427073"
    );

    // [1644293142182] INFO (35 on bf3bb2a2796a): createTree
    //     path: []
    //     pathHash: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    //     key: "03"
    //     value: "0000000000000000000000000000000000000000000000000000000000000000"
    //     valueHash: "66687aadf862bd776c8fc18b8e9f8e20089714856ee233b3902a591d0d5f2925"
    //     useTransaction: true
    //     type: "tree"
    //     method: "insert"
    //     appHash: "db56114e00fdd4c1f85c892bf35ac9a89289aaecb1ebd0a96cde606a748b5d71"

    drive
        .grove
        .insert(
            SubtreePath::empty(),
            Into::<&[u8; 1]>::into(RootTree::SpentAssetLockTransactions),
            Element::empty_tree(),
            None,
            Some(db_transaction),
        )
        .unwrap()
        .expect("should insert tree");

    let app_hash = drive
        .grove
        .root_hash(Some(db_transaction))
        .unwrap()
        .expect("should return app hash");

    assert_eq!(
        hex::encode(app_hash),
        "8d03a90f52a625e711b8edd47f05ae0e6fff3c7ed72ce2fa5e017a9a07792ee0"
    );

    // [1644293142182] INFO (35 on bf3bb2a2796a): createTree
    //     path: [
    //       "03"
    //     ]
    //     pathHash: "084fed08b978af4d7d196a7446a86b58009e636b611db16211b65a9aadff29c5"
    //     key: "00"
    //     value: "0000000000000000000000000000000000000000000000000000000000000000"
    //     valueHash: "66687aadf862bd776c8fc18b8e9f8e20089714856ee233b3902a591d0d5f2925"
    //     useTransaction: true
    //     type: "tree"
    //     method: "insert"
    //     appHash: "2bca13b0f7e68d9c0be5c7352484f8bfba5be6c78f094551c1a0f849f4d7cde0"

    drive
        .grove
        .insert(
            &[Into::<&[u8; 1]>::into(RootTree::SpentAssetLockTransactions).as_slice()],
            &[0],
            Element::empty_tree(),
            None,
            Some(db_transaction),
        )
        .unwrap()
        .expect("should insert tree");

    let app_hash = drive
        .grove
        .root_hash(Some(db_transaction))
        .unwrap()
        .expect("should return app hash");

    assert_eq!(
        hex::encode(app_hash),
        "8971441ae66a2f198260930fb6f4f259eda94cbe2be136b63939e4b46f8be730"
    );

    // [1644295643055] INFO (36 on a5bc48c228d6): put
    //     path: [
    //       "00"
    //     ]
    //     pathHash: "6e340b9cffb37a989ca544e6bb780a2c78901d3fb33738768511a30617afa01d"
    //     key: "f00100b0c1e3762b8bc1421e113c76b2a635c5930b9abf2b336583be5987a715"
    //     value: "01a46269645820f00100b0c1e3762b8bc1421e113c76b2a635c5930b9abf2b336583be5987a7156762616c616e636500687265766973696f6e006a7075626c69634b65797381a662696400646461746158210328f474ce2d61d6fdb45c1fb437ddbf167924e6af3303c167f64d8c8857e39ca564747970650067707572706f73650068726561644f6e6c79f76d73656375726974794c6576656c00"
    //     valueHash: "d7fef03318e2db119a9f5a2d6bcbf9a03fc280b4f4a3f94307736be193c320d4"
    //     useTransaction: true
    //     type: "item"
    //     method: "insert"
    //     appHash: "76c595401762ddbaa0393dda2068327aab60585242483da3388f3af221bb65c4"

    drive.grove.insert(
        &[Into::<&[u8; 1]>::into(RootTree::Identities).as_slice()],
        hex::decode("f00100b0c1e3762b8bc1421e113c76b2a635c5930b9abf2b336583be5987a715").unwrap().as_slice(),
        Element::new_item(hex::decode("01a46269645820f00100b0c1e3762b8bc1421e113c76b2a635c5930b9abf2b336583be5987a7156762616c616e636500687265766973696f6e006a7075626c69634b65797381a662696400646461746158210328f474ce2d61d6fdb45c1fb437ddbf167924e6af3303c167f64d8c8857e39ca564747970650067707572706f73650068726561644f6e6c79f76d73656375726974794c6576656c00").unwrap()),
        None,
        Some(db_transaction),
    ).unwrap().expect("should insert");

    let app_hash = drive
        .grove
        .root_hash(Some(db_transaction))
        .unwrap()
        .expect("should return app hash");

    assert_eq!(
        hex::encode(app_hash),
        "76c595401762ddbaa0393dda2068327aab60585242483da3388f3af221bb65c4"
    );

    // [1644295643057] INFO (36 on a5bc48c228d6): put
    //     path: [
    //       "02"
    //     ]
    //     pathHash: "dbc1b4c900ffe48d575b5da5c638040125f65db0fe3e24494b76ea986457d986"
    //     key: "6198bae2a577044d7975f4d1a06a8d13a9eab9b0"
    //     value: "815820f00100b0c1e3762b8bc1421e113c76b2a635c5930b9abf2b336583be5987a715"
    //     valueHash: "d8c99c5e59a7c1a1cd47aeeef820585df42a21070d0ece24f316061328212636"
    //     useTransaction: true
    //     type: "item"
    //     method: "insert"
    //     appHash: "e34e316e84c4639f44c512c5e602ee7d674d33ce69f02237de87af5f6151cdf6"

    drive
        .grove
        .insert(
            &[Into::<&[u8; 1]>::into(RootTree::UniquePublicKeyHashesToIdentities).as_slice()],
            hex::decode("6198bae2a577044d7975f4d1a06a8d13a9eab9b0")
                .unwrap()
                .as_slice(),
            Element::new_item(
                hex::decode(
                    "815820f00100b0c1e3762b8bc1421e113c76b2a635c5930b9abf2b336583be5987a715",
                )
                .unwrap(),
            ),
            None,
            Some(db_transaction),
        )
        .unwrap()
        .expect("should insert");

    let app_hash = drive
        .grove
        .root_hash(Some(db_transaction))
        .unwrap()
        .expect("should return app hash");

    assert_eq!(
        hex::encode(app_hash),
        "e34e316e84c4639f44c512c5e602ee7d674d33ce69f02237de87af5f6151cdf6"
    );

    let dpns_contract = load_system_data_contract(SystemDataContract::DPNS, platform_version)
        .expect("should load dpns contract");

    drive
        .apply_contract(
            &dpns_contract,
            BlockInfo::genesis(),
            true,
            StorageFlags::optional_default_as_cow(),
            Some(db_transaction),
            platform_version,
        )
        .expect("apply contract");

    let app_hash = drive
        .grove
        .root_hash(Some(db_transaction))
        .unwrap()
        .expect("should return app hash");

    let expected_app_hash = "6b8bbf1f069858abf57573f43a62e27d60e6139c4d23e1fe572fa3fe34057973";

    assert_eq!(hex::encode(app_hash), expected_app_hash);
}

#[cfg(feature = "full")]
/// Runs `test_root_hash_with_batches` 10 times.
#[test]
fn test_deterministic_root_hash_with_batches() {
    let drive = setup_drive(None);

    let db_transaction = drive.grove.start_transaction();

    for _ in 0..10 {
        test_root_hash_with_batches(&drive, &db_transaction);

        drive
            .grove
            .rollback_transaction(&db_transaction)
            .expect("transaction should be rolled back");
    }
}
