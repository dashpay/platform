use grovedb::{Element, Error, GroveDb};
use std::collections::{BTreeMap, HashMap};
use std::path::Path;

pub struct Drive {
    grove: GroveDb,
}

pub struct IndexProperty {
    name: String,
    ascending: bool,
}

pub struct Index {
    indices: Vec<IndexProperty>,
    unique: bool,
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

fn top_level_indices(indices: Vec<Index>) -> Vec<IndexProperty> {
    let mut top_level_indices: Vec<IndexProperty> = vec![];
    for index in indices {
        if index.indices.len() == 1 {
            let owned_index_property = index.indices.first().unwrap();
            let top_level = *owned_index_property.clone();
            top_level_indices.push(top_level);
        }
    }
    top_level_indices
}

fn contract_indices(contract: HashMap<String, Vec<u8>>) -> HashMap<String, Vec<Index>> {
    HashMap::new()
}

fn contract_root_path(contract_id: &[u8]) -> Vec<&[u8]> {
    vec![RootTree::ContractDocuments.into(), contract_id]
}

fn contract_documents_path(contract_id: &[u8]) -> Vec<&[u8]> {
    vec![RootTree::ContractDocuments.into(), contract_id, b"1"]
}

fn contract_documents_primary_key_path(contract_id: &[u8]) -> Vec<&[u8]> {
    vec![RootTree::ContractDocuments.into(), contract_id, b"1", b"0"]
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
        contract: HashMap<String, Vec<u8>>,
        contract_id: &[u8],
    ) -> Result<u64, Error> {
        let contract_root_path = contract_root_path(contract_id);
        let contract_id: &[u8] = contract.get("contractID").ok_or(Error::CorruptedData(String::from("unable to get contract id")))?;

        self.grove.insert(
            &[RootTree::ContractDocuments.into()],
            Vec::from(contract_id),
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
        let contract_documents_path = contract_documents_path(contract_id);
        for (type_key, indices) in contract_indices(contract) {
            self.grove.insert(
                &contract_documents_path,
                Vec::from(type_key),
                Element::empty_tree(),
            )?;

            let mut type_path = contract_documents_path.clone();
            type_path.push(type_key.as_bytes());

            // for each type we should insert the indices that are top level
            let top_level_indices = top_level_indices(indices);
            for index in top_level_indices {
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
        contract: HashMap<String, Vec<u8>>,
        contract_id: &[u8],
    ) -> Result<u64, Error> {
        let contract_root_path = contract_root_path(contract_id);
        // Will need a proper error enum
        let contract_id: &[u8] = contract
            .get("contractID")
            .ok_or(Error::CorruptedData(String::from("unable to get contract id")))?;

        self.grove.insert(
            &[RootTree::ContractDocuments.into()],
            Vec::from(contract_id),
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
        let contract_documents_path = contract_documents_path(contract_id);
        for (type_key, indices) in contract_indices(contract) {
            self.grove.insert(
                &contract_documents_path,
                Vec::from(type_key),
                Element::empty_tree(),
            )?;

            let mut type_path = contract_documents_path.clone();
            type_path.push(type_key.as_bytes());

            // for each type we should insert the indices that are top level
            let top_level_indices = top_level_indices(indices);
            for index in top_level_indices {
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

    pub fn apply_contract(&mut self, contract_cbor: &[u8]) -> Result<u64, Error> {
        // first we need to deserialize the contract
        let contract: HashMap<String, Vec<u8>> = minicbor::decode(contract_cbor.as_ref())
            .map_err(|_| Error::CorruptedData(String::from("unable to decode contract")))?;
        let contract_id: &[u8] = contract.get("contractID").ok_or(Error::CorruptedData(String::from("unable to get contract id")))?;

        let contract_bytes = Vec::from(contract_cbor);
        let contract_element = Element::Item(contract_bytes);

        // overlying structure
        let mut already_exists = false;
        let mut different_contract_data = false;
        match self.grove.get(&*contract_root_path(contract_id), b"0") {
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

        match already_exists {
            true => {
                match different_contract_data {
                    true => self.update_contract(contract_element, contract, contract_id),
                    false => {
                        // there is nothing to do, nothing was changed
                        // accept it, but return cost 0
                        Ok(0)
                    }
                }
            }
            false => self.insert_contract(contract_element, contract, contract_id),
        }
    }

    pub fn store_document(
        &mut self,
        document_cbor: &[u8],
        contract_indices_cbor: &[u8],
    ) -> Result<(), Error> {
        // first we need to deserialize the document and contract indices
        let document: HashMap<String, Vec<u8>> =
            minicbor::decode(document_cbor.as_ref())
                .map_err(|_| Error::CorruptedData(String::from("unable to decode contract")))?;
        let document_id: &[u8] = document.get("documentID").ok_or(Error::CorruptedData(String::from("unable to get document id")))?;
        let contract_id: &[u8] = document.get("contractID").ok_or(Error::CorruptedData(String::from("unable to get contract id")))?;

        let contract_indices: Vec<Index> = minicbor::decode(contract_indices_cbor.as_ref())
            .map_err(|_| Error::CorruptedData(String::from("unable to decode contract indices")))?;

        // second we need to construct the path for documents on the contract
        // the path is
        //  * Document and Contract root tree
        //  * Contract ID recovered from document
        //  * 0 to signify Documents and not Contract
        let contract_path = contract_documents_path(contract_id);

        // third we need to store the document for it's primary key
        let mut primary_key_path = contract_documents_primary_key_path(contract_id);
        let document_element = Element::Item(Vec::from(document_cbor));
        self.grove
            .insert(&primary_key_path, Vec::from(document_id), document_element)?;

        // fourth we need to store a reference to the document for each index
        for index in contract_indices {
            // at this point the contract path is to the contract documents
            // for each index the top index component will already have been added
            // when the contract itself was created
            let mut index_path = contract_path.clone();
            let top_index_property = index.indices.get(0)?;
            index_path.push(top_index_property.name.as_bytes());
            // with the example of the dashpay contract's first index
            // the index path is now something like Contracts/ContractID/Documents(1)/$ownerId

            let document_top_field: &[u8] = document.get(&top_index_property.name).ok_or(Error::CorruptedData(String::from("unable to get document top index field")))?;

            // here we are inserting an empty tree that will have a subtree of all other index properties
            self.grove.insert_if_not_exists(&index_path, Vec::from(document_top_field), Element::empty_tree())?;

            // we push the actual value of the index path
            index_path.push(document_top_field);
            // the index path is now something like Contracts/ContractID/Documents(1)/$ownerId/<ownerId>

            for i in 1..index.indices.len() {
                let index_property = index.indices.get(i)?;
                // here we are inserting an empty tree that will have a subtree of all other index properties
                self.grove.insert_if_not_exists(&index_path, Vec::from(&index_property.name), Element::empty_tree())?;

                index_path.push(index_property.name.as_bytes());
                // Iteration 1. the index path is now something like Contracts/ContractID/Documents(1)/$ownerId/<ownerId>/toUserId
                // Iteration 2. the index path is now something like Contracts/ContractID/Documents(1)/$ownerId/<ownerId>/toUserId/<ToUserId>/accountReference

                let document_top_field: &[u8] = document.get(&index_property.name).ok_or(Error::CorruptedData(String::from("unable to get document field")))?;

                // here we are inserting an empty tree that will have a subtree of all other index properties
                self.grove.insert_if_not_exists(&index_path, Vec::from(document_top_field), Element::empty_tree())?;

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
                self.grove.insert_if_not_exists(&index_path, Vec::from(b"0"), Element::empty_tree())?;
                index_path.push(b"0");
            }

            // here we should return an error if the element already exists
            self.grove.insert(&index_path, Vec::from(b"0"), document_reference)?;
        }
        Ok(cost)
    }

    pub fn delete_document(
        &mut self,
        contract_id: &[u8],
        document_id: &[u8],
        contract_indices_cbor: &[u8],
    ) -> Result<(), Error> {
        // first we need to construct the path for documents on the contract
        // the path is
        //  * Document and Contract root tree
        //  * Contract ID recovered from document
        //  * 0 to signify Documents and not Contract
        let contract_documents_primary_key_path = contract_documents_primary_key_path(contract_id);

        // next we need to get the document from storage
        let document_element : Element::Item = self.grove.get(&*contract_documents_primary_key_path, document_id)?;
        let document: HashMap<String, Vec<u8>> =
            minicbor::decode(document_element)
                .map_err(|_| Error::CorruptedData(String::from("unable to decode document")))?;

        // next we need to get the contract indices to be able to delete them
        let contract_indices: Vec<Index> = minicbor::decode(contract_indices_cbor.as_ref())
            .map_err(|_| Error::CorruptedData(String::from("unable to decode contract indices")))?;

        // third we need to delete the document for it's primary key
        self.grove.delete(&contract_documents_primary_key_path, Vec::from(document_id))?;

        // fourth we need delete all references to the document
        // to do this we need to go through each index
        for index in contract_indices {
            // at this point the contract path is to the contract documents
            // for each index the top index component will already have been added
            // when the contract itself was created
            let mut index_path = contract_path.clone();
            let top_index_property = index.indices.get(0)?;
            index_path.push(top_index_property.name.as_bytes());
            // with the example of the dashpay contract's first index
            // the index path is now something like Contracts/ContractID/Documents(1)/$ownerId

            let document_top_field: &[u8] = document.get(&top_index_property.name).ok_or(Error::CorruptedData(String::from("unable to get document top index field")))?;

            // we push the actual value of the index path
            index_path.push(document_top_field);
            // the index path is now something like Contracts/ContractID/Documents(1)/$ownerId/<ownerId>

            for i in 1..index.indices.len() {
                let index_property = index.indices.get(i)?;

                index_path.push(index_property.name.as_bytes());
                // Iteration 1. the index path is now something like Contracts/ContractID/Documents(1)/$ownerId/<ownerId>/toUserId
                // Iteration 2. the index path is now something like Contracts/ContractID/Documents(1)/$ownerId/<ownerId>/toUserId/<ToUserId>/accountReference

                let document_top_field: &[u8] = document.get(&index_property.name).ok_or(Error::CorruptedData(String::from("unable to get document field")))?;

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
        Ok(cost)
    }
}

#[cfg(test)]
mod tests {
    use crate::drive::Drive;
    use std::collections::HashMap;

    #[test]
    fn store_document_1() {
        let tmp_dir = TempDir::new("db").unwrap();
        drive = Drive::open(tmp_dir);
    }

    #[test]
    fn test_cbor_deserialization() {
        let document: HashMap<str, Vec<u8>> = minicbor::decode(document_cbor.as_ref())?;
        let tmp_dir = TempDir::new("db").unwrap();
        drive = Drive::open(tmp_dir);
    }
}
