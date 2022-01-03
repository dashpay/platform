use crate::contract::{Contract, DocumentType};
use ciborium::value::{Value as CborValue, Value};
use grovedb::{Element, Error, GroveDb};
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::collections::HashMap;
use std::path::Path;

pub struct Drive {
    grove: GroveDb,
}

pub enum RootTree {
    // Input data errors
    Identities,
    ContractDocuments,
    PublicKeyHashesToIdentities,
    Misc,
}

pub const STORAGE_COST: i32 = 50;

impl From<RootTree> for &'static [u8] {
    fn from(root_tree: RootTree) -> Self {
        match root_tree {
            RootTree::Identities => b"0",
            RootTree::ContractDocuments => b"1",
            RootTree::PublicKeyHashesToIdentities => b"2",
            RootTree::Misc => b"3",
        }
    }
}

impl From<RootTree> for Vec<u8> {
    fn from(root_tree: RootTree) -> Self {
        match root_tree {
            RootTree::Identities => b"0".to_vec(),
            RootTree::ContractDocuments => b"1".to_vec(),
            RootTree::PublicKeyHashesToIdentities => b"2".to_vec(),
            RootTree::Misc => b"3".to_vec(),
        }
    }
}

// // split_contract_indices will take an array of indices and construct an array of group indices
// // grouped indices will group on identical first indices then on identical second indices
// // if the first index is common and so forth
// pub fn split_contract_indices(contract_indices : Vec<Vec<Vec<u8>>>) -> HashMap<&[u8], &[u8]> {
// //    [firstName, lastName]
// //    [firstName]
// //    [firstName, lastName, age]
// //    [age]
// //    =>
// //    [firstName : [b"0", {lastName : [b"0", {age : b"0" }]}], age: b"0"],
// //
// }

fn contract_root_path(contract_id: &[u8]) -> Vec<&[u8]> {
    vec![RootTree::ContractDocuments.into(), contract_id]
}

fn contract_documents_path(contract_id: &[u8]) -> Vec<&[u8]> {
    vec![RootTree::ContractDocuments.into(), contract_id, b"1"]
}

fn contract_documents_primary_key_path(contract_id: &[u8]) -> Vec<&[u8]> {
    vec![RootTree::ContractDocuments.into(), contract_id, b"1", b"0"]
}

fn base58_value_as_bytes_from_hash_map(
    document: &HashMap<String, CborValue>,
    key: &str,
) -> Option<Vec<u8>> {
    document
        .get(key)
        .map(|id_cbor| {
            if let CborValue::Text(b) = id_cbor {
                match bs58::decode(b).into_vec() {
                    Ok(data) => Some(data),
                    Err(_) => None,
                }
            } else {
                None
            }
        })
        .flatten()
}

impl Drive {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        match GroveDb::open(path) {
            Ok(grove) => Ok(Drive { grove }),
            Err(e) => Err(e),
        }
    }

    fn create_root_tree(&mut self) -> Result<(), Error> {
        self.grove
            .insert(&[], RootTree::Identities.into(), Element::empty_tree())?;
        self.grove.insert(
            &[],
            RootTree::ContractDocuments.into(),
            Element::empty_tree(),
        )?;
        self.grove.insert(
            &[],
            RootTree::PublicKeyHashesToIdentities.into(),
            Element::empty_tree(),
        )?;
        self.grove
            .insert(&[], RootTree::Misc.into(), Element::empty_tree())?;
        Ok(())
    }

    fn insert_contract(
        &mut self,
        contract_bytes: Element,
        contract: &Contract,
    ) -> Result<u64, Error> {
        let contract_root_path = contract_root_path(&contract.id);

        self.grove.insert(
            &[RootTree::ContractDocuments.into()],
            contract.id.clone(),
            Element::empty_tree(),
        )?;

        let mut cost: u64 = 0;

        // unsafe {
        //     cost += contract_cbor.size_of() * STORAGE_COST;
        // }

        // the contract
        self.grove
            .insert(&contract_root_path, b"0".to_vec(), contract_bytes)?;

        // the documents
        self.grove
            .insert(&contract_root_path, b"1".to_vec(), Element::empty_tree())?;

        // next we should store each document type
        // right now we are referring them by name
        // toDo: change this to be a reference by index
        let contract_documents_path = contract_documents_path(&contract.id);
        for (type_key, document_type) in &contract.document_types {
            self.grove.insert(
                &contract_documents_path,
                type_key.as_bytes().to_vec(),
                Element::empty_tree(),
            )?;

            let mut type_path = contract_documents_path.clone();
            type_path.push(type_key.as_bytes());

            // for each type we should insert the indices that are top level
            for index in document_type.top_level_indices()? {
                // toDo: change this to be a reference by index
                self.grove.insert(
                    &type_path,
                    Vec::from(index.name.as_bytes()),
                    Element::empty_tree(),
                )?;
            }
        }

        Ok(cost)
    }

    fn update_contract(
        &mut self,
        contract_bytes: Element,
        contract: &Contract,
    ) -> Result<u64, Error> {
        let contract_root_path = contract_root_path(&contract.id);

        let mut cost: u64 = 0;

        // this will override the previous contract
        self.grove
            .insert(&contract_root_path, b"0".to_vec(), contract_bytes)?;

        let contract_documents_path = contract_documents_path(&contract.id);
        for (type_key, document_type) in &contract.document_types {
            let mut type_path = contract_documents_path.clone();
            type_path.push(type_key.as_bytes());

            // for each type we should insert the indices that are top level
            for index in document_type.top_level_indices()? {
                // toDo: change this to be a reference by index
                self.grove.insert_if_not_exists(
                    &type_path,
                    Vec::from(index.name.as_bytes()),
                    Element::empty_tree(),
                )?;
            }
        }

        Ok(cost)
    }

    pub fn apply_contract(&mut self, contract_cbor: &[u8]) -> Result<u64, Error> {
        // first we need to deserialize the contract
        let contract = Contract::from_cbor(contract_cbor)?;

        let contract_bytes = Vec::from(contract_cbor);
        let contract_element = Element::Item(contract_bytes.clone());

        // overlying structure
        let mut already_exists = false;
        let mut different_contract_data = false;

        match self.grove.get(&*contract_root_path(&contract.id), b"0") {
            Ok(stored_Element) => {
                already_exists = true;
                match stored_Element {
                    Element::Item(stored_contract_bytes) => {
                        if contract_bytes != stored_contract_bytes {
                            different_contract_data = true;
                        }
                    }
                    _ => {
                        already_exists = false;
                    }
                }
            }
            Err(_) => {
                // the element doesn't exist
                // no need to do anything
            }
        };

        if already_exists {
            if different_contract_data {
                self.update_contract(contract_element, &contract)
            } else {
                Ok(0)
            }
        } else {
            self.insert_contract(contract_element, &contract)
        }
    }

    pub fn add_document(&mut self, document_cbor: &[u8]) -> Result<(), Error> {
        todo!()
    }

    pub fn add_document_for_contract(
        &mut self,
        document_cbor: &[u8],
        contract_cbor: &[u8],
        document_type: &str,
    ) -> Result<(), Error> {
        // first we need to deserialize the document and contract indices
        let document: HashMap<String, CborValue> = ciborium::de::from_reader(document_cbor)
            .map_err(|_| Error::CorruptedData(String::from("unable to decode contract")))?;
        let document_id: &[u8] = document
            .get("documentID")
            .map(|id_cbor| {
                if let CborValue::Bytes(b) = id_cbor {
                    Some(b)
                } else {
                    None
                }
            })
            .flatten()
            .ok_or(Error::CorruptedData(String::from(
                "unable to get document id",
            )))?;
        let contract = Contract::from_cbor(contract_cbor)?;

        // second we need to construct the path for documents on the contract
        // the path is
        //  * Document and Contract root tree
        //  * Contract ID recovered from document
        //  * 0 to signify Documents and not Contract
        let contract_path = contract_documents_path(&contract.id);

        // third we need to store the document for it's primary key
        let mut primary_key_path = contract_documents_primary_key_path(&contract.id);
        let document_element = Element::Item(Vec::from(document_cbor));
        self.grove
            .insert(&primary_key_path, Vec::from(document_id), document_element)?;

        let document_type =
            contract
                .document_types
                .get(document_type)
                .ok_or(Error::CorruptedData(String::from(
                    "can not get document type from contract",
                )))?;
        // fourth we need to store a reference to the document for each index
        for index in &document_type.indices {
            // at this point the contract path is to the contract documents
            // for each index the top index component will already have been added
            // when the contract itself was created
            let mut index_path = contract_path.clone();
            let top_index_property =
                index
                    .properties
                    .get(0)
                    .ok_or(Error::CorruptedData(String::from(
                        "invalid contract indices",
                    )))?;
            index_path.push(top_index_property.name.as_bytes());
            // with the example of the dashpay contract's first index
            // the index path is now something like Contracts/ContractID/Documents(1)/$ownerId

            let document_top_field: &[u8] = document
                .get(&top_index_property.name)
                .map(|id_cbor| {
                    if let CborValue::Bytes(b) = id_cbor {
                        Some(b)
                    } else {
                        None
                    }
                })
                .flatten()
                .ok_or(Error::CorruptedData(String::from(
                    "unable to get document top index field",
                )))?;

            // here we are inserting an empty tree that will have a subtree of all other index properties
            self.grove.insert_if_not_exists(
                &index_path,
                Vec::from(document_top_field),
                Element::empty_tree(),
            )?;

            // we push the actual value of the index path
            index_path.push(document_top_field);
            // the index path is now something like Contracts/ContractID/Documents(1)/$ownerId/<ownerId>

            for i in 1..index.properties.len() {
                let index_property =
                    index
                        .properties
                        .get(i)
                        .ok_or(Error::CorruptedData(String::from(
                            "invalid contract indices",
                        )))?;
                // here we are inserting an empty tree that will have a subtree of all other index properties
                self.grove.insert_if_not_exists(
                    &index_path,
                    index_property.name.as_bytes().to_vec(),
                    Element::empty_tree(),
                )?;

                index_path.push(index_property.name.as_bytes());
                // Iteration 1. the index path is now something like Contracts/ContractID/Documents(1)/$ownerId/<ownerId>/toUserId
                // Iteration 2. the index path is now something like Contracts/ContractID/Documents(1)/$ownerId/<ownerId>/toUserId/<ToUserId>/accountReference

                let document_top_field: &[u8] = document
                    .get(&index_property.name)
                    .map(|id_cbor| {
                        if let CborValue::Bytes(b) = id_cbor {
                            Some(b)
                        } else {
                            None
                        }
                    })
                    .flatten()
                    .ok_or(Error::CorruptedData(String::from(
                        "unable to get document field",
                    )))?;

                // here we are inserting an empty tree that will have a subtree of all other index properties
                self.grove.insert_if_not_exists(
                    &index_path,
                    Vec::from(document_top_field),
                    Element::empty_tree(),
                )?;

                // we push the actual value of the index path
                index_path.push(document_top_field);
                // Iteration 1. the index path is now something like Contracts/ContractID/Documents(1)/$ownerId/<ownerId>/toUserId/<ToUserId>/
                // Iteration 2. the index path is now something like Contracts/ContractID/Documents(1)/$ownerId/<ownerId>/toUserId/<ToUserId>/accountReference/<accountReference>
            }

            // we need to construct the reference to the original document
            let document_reference =
                Element::Reference(primary_key_path.iter().map(|x| x.to_vec()).collect());

            // unique indexes will be stored under key "0"
            // non unique indices should have a tree at key "0" that has all elements based off of primary key
            if !index.unique {
                // here we are inserting an empty tree that will have a subtree of all other index properties
                self.grove.insert_if_not_exists(
                    &index_path,
                    b"0".to_vec(),
                    Element::empty_tree(),
                )?;
                index_path.push(b"0");
            }

            // here we should return an error if the element already exists
            self.grove
                .insert(&index_path, b"0".to_vec(), document_reference)?;
        }

        Ok(())
    }

    pub fn update_document_for_contract(
        &mut self,
        document_cbor: &[u8],
        contract_cbor: &[u8],
        document_type: &str,
    ) -> Result<(), Error> {
        // first we need to deserialize the document and contract indices
        let document: HashMap<String, CborValue> = ciborium::de::from_reader(document_cbor)
            .map_err(|_| Error::CorruptedData(String::from("unable to decode contract")))?;
        let document_id: &[u8] = document
            .get("documentID")
            .map(|id_cbor| {
                if let CborValue::Bytes(b) = id_cbor {
                    Some(b)
                } else {
                    None
                }
            })
            .flatten()
            .ok_or(Error::CorruptedData(String::from(
                "unable to get document id",
            )))?;

        // for now updating a document will delete the document, then insert a new document
        self.delete_document_for_contract(document_id, contract_cbor, document_type)?;
        self.add_document_for_contract(document_cbor, contract_cbor, document_type)
    }

    pub fn delete_document_for_contract(
        &mut self,
        document_id: &[u8],
        contract_cbor: &[u8],
        document_type: &str,
    ) -> Result<(), Error> {
        let contract = Contract::from_cbor(contract_cbor)?;
        let document_type =
            contract
                .document_types
                .get(document_type)
                .ok_or(Error::CorruptedData(String::from(
                    "can not get document type from contract",
                )))?;
        // first we need to construct the path for documents on the contract
        // the path is
        //  * Document and Contract root tree
        //  * Contract ID recovered from document
        //  * 0 to signify Documents and not Contract
        let contract_documents_primary_key_path = contract_documents_primary_key_path(&contract.id);

        // next we need to get the document from storage
        let document_element: Element = self
            .grove
            .get(&contract_documents_primary_key_path, document_id)?;

        let mut document_bytes: Option<Vec<u8>> = None;
        match document_element {
            Element::Item(data) => {
                document_bytes = Some(data);
            }
            _ => {} // Can the element ever not be an item
        }

        // possibility that document might not be in storage
        // TODO: how should this be handled
        if document_bytes.is_none() {
            todo!()
        }

        let document: HashMap<String, CborValue> = ciborium::de::from_reader(
            document_bytes
                .expect("Can't be none handled above")
                .as_slice(),
        )
        .map_err(|_| Error::CorruptedData(String::from("unable to decode document")))?;

        // third we need to delete the document for it's primary key
        self.grove
            .delete(&contract_documents_primary_key_path, Vec::from(document_id))?;

        let contract_path = contract_documents_path(&contract.id);

        // fourth we need delete all references to the document
        // to do this we need to go through each index
        for index in &document_type.indices {
            // at this point the contract path is to the contract documents
            // for each index the top index component will already have been added
            // when the contract itself was created
            let mut index_path = contract_path.clone();
            let top_index_property =
                index
                    .properties
                    .get(0)
                    .ok_or(Error::CorruptedData(String::from(
                        "invalid contract indices",
                    )))?;
            index_path.push(top_index_property.name.as_bytes());
            // with the example of the dashpay contract's first index
            // the index path is now something like Contracts/ContractID/Documents(1)/$ownerId

            let document_top_field: &[u8] = document
                .get(&top_index_property.name)
                .map(|id_cbor| {
                    if let CborValue::Bytes(b) = id_cbor {
                        Some(b)
                    } else {
                        None
                    }
                })
                .flatten()
                .ok_or(Error::CorruptedData(String::from(
                    "unable to get document top index field",
                )))?;

            // we push the actual value of the index path
            index_path.push(document_top_field);
            // the index path is now something like Contracts/ContractID/Documents(1)/$ownerId/<ownerId>

            for i in 1..index.properties.len() {
                let index_property =
                    index
                        .properties
                        .get(i)
                        .ok_or(Error::CorruptedData(String::from(
                            "invalid contract indices",
                        )))?;

                index_path.push(index_property.name.as_bytes());
                // Iteration 1. the index path is now something like Contracts/ContractID/Documents(1)/$ownerId/<ownerId>/toUserId
                // Iteration 2. the index path is now something like Contracts/ContractID/Documents(1)/$ownerId/<ownerId>/toUserId/<ToUserId>/accountReference

                let document_top_field: &[u8] = document
                    .get(&index_property.name)
                    .map(|id_cbor| {
                        if let CborValue::Bytes(b) = id_cbor {
                            Some(b)
                        } else {
                            None
                        }
                    })
                    .flatten()
                    .ok_or(Error::CorruptedData(String::from(
                        "unable to get document field",
                    )))?;

                // we push the actual value of the index path
                index_path.push(document_top_field);
                // Iteration 1. the index path is now something like Contracts/ContractID/Documents(1)/$ownerId/<ownerId>/toUserId/<ToUserId>/
                // Iteration 2. the index path is now something like Contracts/ContractID/Documents(1)/$ownerId/<ownerId>/toUserId/<ToUserId>/accountReference/<accountReference>
            }

            // unique indexes will be stored under key "0"
            // non unique indices should have a tree at key "0" that has all elements based off of primary key
            if !index.unique {
                index_path.push(b"0");
            }

            // here we should return an error if the element already exists
            self.grove.delete(&index_path, Vec::from(document_id))?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::drive::Drive;
    use serde::{Deserialize, Serialize};
    use std::{collections::HashMap, fs::File, io::BufReader, path::Path};
    use tempdir::TempDir;

    fn json_document_to_cbor(path: impl AsRef<Path>) -> Vec<u8> {
        let file = File::open(path).expect("file not found");
        let reader = BufReader::new(file);
        let json: serde_json::Value =
            serde_json::from_reader(reader).expect("expected a valid json");
        let mut buffer: Vec<u8> = Vec::new();
        ciborium::ser::into_writer(&json, &mut buffer).expect("unable to serialize into cbor");
        buffer
    }

    #[test]
    fn test_add_dashpay_data_contract() {
        let tmp_dir = TempDir::new("db").unwrap();
        let mut drive: Drive = Drive::open(tmp_dir).expect("expected to open Drive successfully");

        drive
            .create_root_tree()
            .expect("expected to create root tree successfully");

        // let's construct the grovedb structure for the dashpay data contract
        let dashpay_cbor = json_document_to_cbor("dashpay-contract.json");
        drive
            .apply_contract(&dashpay_cbor)
            .expect("expected to apply contract successfully");

        // dashpay_profile_document_cbor = json_document_to_cbor("dashpay-profile-1.json")?;
        // drive.add_document(dashpay_profile_document_cbor, dashpay_cbor);
    }

    #[test]
    fn store_document_1() {
        let tmp_dir = TempDir::new("db").unwrap();
        let _drive = Drive::open(tmp_dir);
    }

    #[test]
    fn test_cbor_deserialization() {
        let document_cbor = json_document_to_cbor("simple.json");
        let document: HashMap<String, ciborium::value::Value> =
            ciborium::de::from_reader(document_cbor.as_slice()).expect("cannot deserialize cbor");
        assert!(document.get("a").is_some());
        let tmp_dir = TempDir::new("db").unwrap();
        let _drive = Drive::open(tmp_dir);
    }
}
