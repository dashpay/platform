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

//! Random Documents.
//!
//! This module defines the CreateRandomDocument trait and its functions, which
//! create various types of random documents.
//!

use crate::data_contract::document_type::property_names::{CREATED_AT, UPDATED_AT};
use crate::data_contract::document_type::DocumentType;
use crate::document::generate_document_id::generate_document_id;
use crate::document::{Document, DocumentV0};
use crate::identity::Identity;
use crate::ProtocolError;
use platform_value::{Bytes32, Identifier};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::time::{SystemTime, UNIX_EPOCH};

// TODO The factory is used in benchmark and tests. Probably it should be available under the test feature
/// Functions for creating various types of random documents.
pub trait CreateRandomDocument {
    /// Random documents
    fn random_documents(&self, count: u32, seed: Option<u64>) -> Vec<Document>;
    /// Random documents with rng
    fn random_documents_with_rng(&self, count: u32, rng: &mut StdRng) -> Vec<Document>;
    /// Creates `count` Documents with random data using the random number generator given.
    fn random_documents_with_params(
        &self,
        count: u32,
        identities: &Vec<Identity>,
        time_ms: u64,
        rng: &mut StdRng,
    ) -> Vec<(Document, Identity, Bytes32)>;
    /// Document from bytes
    fn document_from_bytes(&self, bytes: &[u8]) -> Result<Document, ProtocolError>;
    /// Random document
    fn random_document(&self, seed: Option<u64>) -> Document;
    /// Random document with rng
    fn random_document_with_rng(&self, rng: &mut StdRng) -> Document;
    /// Creates a document with a random id, owner id, and properties using StdRng.
    fn random_document_with_params(
        &self,
        owner_id: Identifier,
        entropy: Bytes32,
        time_ms: u64,
        rng: &mut StdRng,
    ) -> Document;
    /// Random filled documents
    fn random_filled_documents(&self, count: u32, seed: Option<u64>) -> Vec<Document>;
    /// Random filled document
    fn random_filled_document(&self, seed: Option<u64>) -> Document;
    /// Random filled document with rng
    fn random_filled_document_with_rng(&self, rng: &mut StdRng) -> Document;
}

impl CreateRandomDocument for DocumentType {
    /// Creates `count` Documents with random data using a seed if given, otherwise entropy.
    fn random_documents(&self, count: u32, seed: Option<u64>) -> Vec<Document> {
        let mut rng = match seed {
            None => StdRng::from_entropy(),
            Some(seed_value) => StdRng::seed_from_u64(seed_value),
        };
        self.random_documents_with_rng(count, &mut rng)
    }

    /// Creates `count` Documents with random data using the random number generator given.
    fn random_documents_with_rng(&self, count: u32, rng: &mut StdRng) -> Vec<Document> {
        let mut vec: Vec<Document> = vec![];
        for _i in 0..count {
            vec.push(self.random_document_with_rng(rng));
        }
        vec
    }

    /// Creates `count` Documents with random data using the random number generator given.
    fn random_documents_with_params(
        &self,
        count: u32,
        identities: &Vec<Identity>,
        time_ms: u64,
        rng: &mut StdRng,
    ) -> Vec<(Document, Identity, Bytes32)> {
        let mut vec = vec![];
        for _i in 0..count {
            let identity_num = rng.gen_range(0..identities.len());
            let identity = identities.get(identity_num).unwrap().clone();
            let entropy = Bytes32::random_with_rng(rng);
            vec.push((
                self.random_document_with_params(identity.id, entropy, time_ms, rng),
                identity,
                entropy,
            ));
        }
        vec
    }

    /// Creates a Document from a serialized Document.
    fn document_from_bytes(&self, bytes: &[u8]) -> Result<Document, ProtocolError> {
        Document::from_bytes(bytes, self)
    }

    /// Creates a random Document using a seed if given, otherwise entropy.
    fn random_document(&self, seed: Option<u64>) -> Document {
        let mut rng = match seed {
            None => StdRng::from_entropy(),
            Some(seed_value) => StdRng::seed_from_u64(seed_value),
        };
        self.random_document_with_rng(&mut rng)
    }

    /// Creates a document with a random id, owner id, and properties using StdRng.
    fn random_document_with_rng(&self, rng: &mut StdRng) -> Document {
        let owner_id = Identifier::random_with_rng(rng);
        let entropy = Bytes32::random_with_rng(rng);
        let now = SystemTime::now();
        let duration_since_epoch = now.duration_since(UNIX_EPOCH).expect("Time went backwards");
        let milliseconds = duration_since_epoch.as_millis() as u64;
        self.random_document_with_params(owner_id, entropy, milliseconds, rng)
    }

    /// Creates a document with a given owner id and entropy, and properties using StdRng.
    fn random_document_with_params(
        &self,
        owner_id: Identifier,
        entropy: Bytes32,
        time_ms: u64,
        rng: &mut StdRng,
    ) -> Document {
        let id = generate_document_id(
            &self.data_contract_id,
            &owner_id,
            self.name.as_str(),
            entropy.as_slice(),
        );
        // dbg!("gen", hex::encode(id), hex::encode(&self.data_contract_id), hex::encode(&owner_id), self.name.as_str(), hex::encode(entropy.as_slice()));
        let mut created_at = None;
        let mut updated_at = None;
        let properties = self
            .flattened_properties
            .iter()
            .filter_map(|(key, document_field)| {
                if key == CREATED_AT {
                    created_at = Some(time_ms);
                    None
                } else if key == UPDATED_AT {
                    updated_at = Some(time_ms);
                    None
                } else {
                    Some((key.clone(), document_field.document_type.random_value(rng)))
                }
            })
            .collect();

        let revision = if self.documents_mutable {
            Some(1)
        } else {
            None
        };

        DocumentV0 {
            id,
            properties,
            owner_id,
            revision,
            created_at,
            updated_at,
        }
        .into()
    }

    /// Creates `count` Documents with properties filled to max size with random data, along with
    /// a random id and owner id, using a seed if provided, otherwise entropy.
    fn random_filled_documents(&self, count: u32, seed: Option<u64>) -> Vec<Document> {
        let mut rng = match seed {
            None => rand::rngs::StdRng::from_entropy(),
            Some(seed_value) => rand::rngs::StdRng::seed_from_u64(seed_value),
        };
        let mut vec: Vec<Document> = vec![];
        for _i in 0..count {
            vec.push(self.random_filled_document_with_rng(&mut rng));
        }
        vec
    }

    /// Creates a Document with properties filled to max size with random data, along with
    /// a random id and owner id, using a seed if provided, otherwise entropy.
    fn random_filled_document(&self, seed: Option<u64>) -> Document {
        let mut rng = match seed {
            None => rand::rngs::StdRng::from_entropy(),
            Some(seed_value) => rand::rngs::StdRng::seed_from_u64(seed_value),
        };
        self.random_filled_document_with_rng(&mut rng)
    }

    /// Creates a Document with properties filled to max size with random data, along with
    /// a random id and owner id.
    fn random_filled_document_with_rng(&self, rng: &mut StdRng) -> Document {
        let id = Identifier::random_with_rng(rng);
        let owner_id = Identifier::random_with_rng(rng);
        let properties = self
            .flattened_properties
            .iter()
            .map(|(key, document_field)| {
                (
                    key.clone(),
                    document_field.document_type.random_filled_value(rng),
                )
            })
            .collect();

        let revision = if self.documents_mutable {
            Some(1)
        } else {
            None
        };

        DocumentV0 {
            id,
            properties,
            owner_id,
            revision,
            created_at: None,
            updated_at: None,
        }
        .into()
    }
}
