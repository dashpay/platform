use crate::{prelude::Identifier, util::hash::hash_to_vec};

/// Generates the document ID
pub fn generate_document_id_v0(
    contract_id: &Identifier,
    owner_id: &Identifier,
    document_type: &str,
    entropy: &[u8],
) -> Identifier {
    let mut buf: Vec<u8> = vec![];

    buf.extend_from_slice(&contract_id.to_buffer());
    buf.extend_from_slice(&owner_id.to_buffer());
    buf.extend_from_slice(document_type.as_bytes());
    buf.extend_from_slice(entropy);

    Identifier::from_bytes(&hash_to_vec(&buf)).unwrap()
}
