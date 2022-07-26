use super::document::Document;
use crate::error::Error;
use dpp::data_contract::extra::DocumentType;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

// TODO The factory is used in benchmark and tests. Probably it should be available under the test feature
pub trait CreateRandomDocument {
    fn random_documents(&self, count: u32, seed: Option<u64>) -> Vec<Document>;
    fn document_from_bytes(&self, bytes: &[u8]) -> Result<Document, Error>;
    fn random_document(&self, seed: Option<u64>) -> Document;
    fn random_document_with_rng(&self, rng: &mut StdRng) -> Document;
    fn random_filled_documents(&self, count: u32, seed: Option<u64>) -> Vec<Document>;
    fn random_filled_document(&self, seed: Option<u64>) -> Document;
    fn random_filled_document_with_rng(&self, rng: &mut StdRng) -> Document;
}

impl CreateRandomDocument for DocumentType {
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

    fn document_from_bytes(&self, bytes: &[u8]) -> Result<Document, Error> {
        Document::from_bytes(bytes, self)
    }

    fn random_document(&self, seed: Option<u64>) -> Document {
        let mut rng = match seed {
            None => StdRng::from_entropy(),
            Some(seed_value) => StdRng::seed_from_u64(seed_value),
        };
        self.random_document_with_rng(&mut rng)
    }

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

    fn random_filled_document(&self, seed: Option<u64>) -> Document {
        let mut rng = match seed {
            None => rand::rngs::StdRng::from_entropy(),
            Some(seed_value) => rand::rngs::StdRng::seed_from_u64(seed_value),
        };
        self.random_filled_document_with_rng(&mut rng)
    }

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
