use std::collections::HashMap;
use std::path::Path;
use grovedb::{Element, Error, GroveDb};
use minicbor::decode::{Token, Tokenizer};

pub struct Drive {
    grove: GroveDb,
}

// split_contract_indices will take an array of indices and construct an array of group indices
// grouped indices will group on identical first indices then on identical second indices
// if the first index is common and so forth
pub fn split_contract_indices(contract_indices : Vec<Vec<Vec<u8>>>) -> HashMap<&[u8], V> {

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

    fn store(&mut self, document_cbor: &[u8], contract_indices_cbor: &[u8]) -> Result<(), Error> {
        // first we need to deserialize the document and contract indices
        let document : HashMap<str, V> = minicbor::decode(document_cbor.as_ref())?;
        let mut contract_id: &[u8] = &[];
        let mut document_id: &[u8] = &[];
        match document.get("contractID") {
            Some(recovered_contract_id) => {
                contract_id = recovered_contract_id;
            },
            None() => Err(())
        }
        match document.get("documentID") {
            Some(recovered_document_id) => {
                document_id = recovered_document_id;
            },
            None() => Err(())
        }

        let contract_indices : Vec<Vec<Vec<u8>>> = minicbor::decode(contract_indices_cbor.as_ref())?;

        // second we need to construct the path for documents on the contract
        // the path is
        //  * Document and Contract root tree
        //  * Contract ID recovered from document
        //  * 0 to signify Documents and not Contract
        let mut contract_path = &[b"5", contract_id, b"0"];

        // third we need to store the document for it's primary key
        let primary_key_path = contract_path.clone().append(document_id);
        let document_element = Element::Item(Vec::from(document_cbor));
        self.grove.insert(primary_key_path, Vec::from(document_id), document_element)?;

        // fourth we need to store a reference to the document for each index
        for (grouped_contract_index_key, grouped) in split_contract_indices(contract_indices) {

            // if there is a grouping on the contract index then we need to insert a tree
            let index_path = contract_path.clone().append(grouped_contract_index_key);
            let document_index = Element::Tree();
            self.grove.insert(index_path, Vec::from(document_id), document_index)?;

            let index_path = contract_path.clone().append(contract_index);
            let document_index = Element::Reference(primary_key_path);
            self.grove.insert(index_path, Vec::from(document_id), document_index)?;
        };
    }
}