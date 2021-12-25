use std::collections::HashMap;
use grovedb::{Element, Error, GroveDb};

pub struct Drive {
    grove: GroveDb,
}

pub enum RootTree {
    // Input data errors
    Identities,
    ContractDocuments,
    PublicKeyHashesToIdentities,
}

impl From<RootTree> for &'static [u8]{
    fn from(root_tree: RootTree) -> Self {
        match root_tree {
            RootTree::Identities => {
                b"0"
            }
            RootTree::ContractDocuments => {
                b"1"
            }
            RootTree::PublicKeyHashesToIdentities => {
                b"2"
            }
        }
    }
}

// split_contract_indices will take an array of indices and construct an array of group indices
// grouped indices will group on identical first indices then on identical second indices
// if the first index is common and so forth
pub fn split_contract_indices(contract_indices : Vec<Vec<Vec<u8>>>) -> HashMap<&[u8], V> {
//    [firstName, lastName]
//    [firstName]
//    [firstName, lastName, age]
//    [age]
//    =>
//    [firstName : [b"0", {lastName : [b"0", {age : b"0" }]}], age: b"0"],
//
}

impl Drive {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        match GroveDb::open(path) {
            Ok(grove) => Ok(Drive {
                grove,
            }),
            Err(e) => Err(e),
        }
    }

    fn create_root_tree(&mut self) -> Result<(), Error> {
        self.grove.insert(&[], RootTree::Identities.into(), Element::empty_tree())?;
        self.grove.insert(&[], RootTree::ContractDocuments.into(), Element::empty_tree())?;
        self.grove.insert(&[], RootTree::PublicKeyHashesToIdentities.into(), Element::empty_tree())?;
        Ok(())
    }

    fn store(&mut self, document_cbor: &[u8], contract_indices_cbor: &[u8]) -> Result<(), Error> {
        // first we need to deserialize the document and contract indices
        let document : HashMap<str, Vec<u8>> = minicbor::decode(document_cbor.as_ref())?;
        let document_id : &[u8] = document.get("documentID")?;
        let contract_id : &[u8] = document.get("contractID")?;

        let contract_indices : Vec<Vec<Vec<u8>>> = minicbor::decode(contract_indices_cbor.as_ref())?;

        // second we need to construct the path for documents on the contract
        // the path is
        //  * Document and Contract root tree
        //  * Contract ID recovered from document
        //  * 0 to signify Documents and not Contract
        let contract_path = vec![RootTree::ContractDocuments.into(), contract_id, b"0"];

        // third we need to store the document for it's primary key
        let mut primary_key_path = contract_path.clone();
        primary_key_path.push(document_id);
        let document_element = Element::Item(Vec::from(document_cbor));
        self.grove.insert(&primary_key_path, Vec::from(document_id), document_element)?;

        // fourth we need to store a reference to the document for each index
        for (grouped_contract_index_key, grouped) in split_contract_indices(contract_indices) {

            // if there is a grouping on the contract index then we need to insert a tree
            let mut index_path = contract_path.clone();
            index_path.push(grouped_contract_index_key);
            let document_index = Element::empty_tree();
            self.grove.insert(&index_path, Vec::from(document_id), document_index)?;

            let mut index_path = contract_path.clone();
            index_path.push(contract_index);
            let document_index = Element::Reference(primary_key_path.iter().map(|x| x.to_vec()).collect());
            self.grove.insert(&index_path, Vec::from(document_id), document_index)?;
        };
    }
}

#[cfg(test)]
mod tests {
    use crate::drive::Drive;

    #[test]
    fn store_document_1() {
        let tmp_dir = TempDir::new("db").unwrap();
        drive = Drive::open(tmp_dir);
    }
}
