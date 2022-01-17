use rand::Rng;
use tempdir::TempDir;
use rs_drive::contract::{Contract, Document};
use rs_drive::drive::Drive;
use serde::{Deserialize, Serialize};
use rand::seq::SliceRandom;

mod common;

#[derive(Serialize, Deserialize)]
struct Person {
    first_name: String,
    middle_name: String,
    last_name: String,
    age: u8,
}

impl Person {
    fn random_people(count: u32) -> (Vec<Self>) {
        let first_names = common::text_file_strings("tests/supporting files/family/first_names.txt");
        let middle_names = common::text_file_strings("tests/supporting files/family/middle_names.txt");
        let last_names = common::text_file_strings("tests/supporting files/family/last_names.txt");
        let mut vec : Vec<Person> = vec![];
        for i in 0..count {
            let person = Person {
                first_name: first_names.choose(&mut rand::thread_rng()).unwrap().clone(),
                middle_name: middle_names.choose(&mut rand::thread_rng()).unwrap().clone(),
                last_name: last_names.choose(&mut rand::thread_rng()).unwrap().clone(),
                age: rand::thread_rng().gen_range(0..85),
            };
            vec.push(person);
        }
        vec
    }
}

pub fn setup() -> Drive {
    // setup code
    let (mut drive, contract) = common::setup_contract("family","tests/supporting files/family/family-contract.json");

    let people = Person::random_people(20000);
    for person in people {
        let value = serde_json::to_value(&person).expect("serialized person");
        let document_cbor = common::value_to_cbor(value);
        let random_owner_id = rand::thread_rng().gen::<[u8; 32]>();
        let document = Document::from_cbor(document_cbor.as_slice(), &random_owner_id).expect("document should be properly deserialized");
        drive.add_document_for_contract(&document, &document_cbor, &contract, "", &random_owner_id, true, None);
    }
    drive
}

#[test]
fn it_adds_two() {
    setup();
}