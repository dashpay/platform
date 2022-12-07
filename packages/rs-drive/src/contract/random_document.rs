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

use super::document::Document;
use crate::error::Error;
use dpp::data_contract::extra::DocumentType;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

// TODO The factory is used in benchmark and tests. Probably it should be available under the test feature
/// Functions for creating various types of random documents.
pub trait CreateRandomDocument {
    /// Random documents
    fn random_documents(&self, count: u32, seed: Option<u64>) -> Vec<Document>;
    /// Document from bytes
    fn document_from_bytes(&self, bytes: &[u8]) -> Result<Document, Error>;
    /// Random document
    fn random_document(&self, seed: Option<u64>) -> Document;
    /// Random document with rng
    fn random_document_with_rng(&self, rng: &mut StdRng) -> Document;
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
        let mut vec: Vec<Document> = vec![];
        for _i in 0..count {
            vec.push(self.random_document_with_rng(&mut rng));
        }
        vec
    }

    /// Creates a Document from a serialized Document.
    fn document_from_bytes(&self, bytes: &[u8]) -> Result<Document, Error> {
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
        let id = rng.gen::<[u8; 32]>();
        let owner_id = rng.gen::<[u8; 32]>();
        let properties = self
            .properties
            .iter()
            .map(|(key, document_field)| {
                (key.clone(), document_field.document_type.random_value(rng))
            })
            .collect();

        Document {
            id,
            properties,
            owner_id,
        }
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
        let id = rng.gen::<[u8; 32]>();
        let owner_id = rng.gen::<[u8; 32]>();
        let properties = self
            .properties
            .iter()
            .map(|(key, document_field)| {
                (
                    key.clone(),
                    document_field.document_type.random_filled_value(rng),
                )
            })
            .collect();

        Document {
            id,
            properties,
            owner_id,
        }
    }
}
