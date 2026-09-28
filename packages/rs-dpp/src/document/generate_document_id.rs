use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::DocumentTypeRef;
use crate::document::{Document, DocumentV0Getters, DocumentV0Setters};
use crate::prelude::IdentityNonce;
use crate::ProtocolError;
use crate::{prelude::Identifier, util::hash::hash_double, util::hash::hash_double_to_vec};
use platform_version::version::PlatformVersion;

/// Domain tag of the v1 document id preimage. A v0 preimage starts with a
/// data contract id, which is itself a hash, so no v0 preimage can start with
/// these bytes and the two derivations can never produce the same id from
/// different inputs.
const DOCUMENT_ID_V1_DOMAIN_TAG: &[u8] = b"dash:document-id:v1";

impl Document {
    /// Derives the id of a document that is about to be created.
    ///
    /// From `generate_document_id` version 1 the identity contract nonce of
    /// the create transition is part of the id. A nonce is consumed at most
    /// once per identity and contract, so an id can be produced at most once:
    /// a document that was deleted can not be created again under the same
    /// id with different content, and whatever referenced the id keeps
    /// pointing at that one document or at nothing.
    ///
    /// The id therefore only exists once the nonce of the create transition
    /// is known, and it changes if the transition is rebuilt with another
    /// nonce.
    pub fn generate_document_id(
        contract_id: &Identifier,
        owner_id: &Identifier,
        document_type_name: &str,
        entropy: &[u8],
        identity_contract_nonce: IdentityNonce,
        platform_version: &PlatformVersion,
    ) -> Result<Identifier, ProtocolError> {
        match platform_version
            .dpp
            .document_versions
            .document_method_versions
            .generate_document_id
        {
            0 => Ok(Self::generate_document_id_v0(
                contract_id,
                owner_id,
                document_type_name,
                entropy,
            )),
            1 => Ok(Self::generate_document_id_v1(
                contract_id,
                owner_id,
                document_type_name,
                entropy,
                identity_contract_nonce,
            )),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "Document::generate_document_id".to_string(),
                known_versions: vec![0, 1],
                received: version,
            }),
        }
    }

    /// Gives a document that is about to be created the id its create
    /// transition will carry, from the entropy and the identity contract nonce
    /// that transition is going to use.
    ///
    /// A client that needs the id before it sends the document (to reference
    /// it from another document, or to act on it afterwards) assigns the nonce
    /// first and calls this. The create transition must then be built with the
    /// same entropy and nonce.
    pub fn set_id_for_creation(
        &mut self,
        document_type: DocumentTypeRef,
        entropy: &[u8; 32],
        identity_contract_nonce: IdentityNonce,
        platform_version: &PlatformVersion,
    ) -> Result<(), ProtocolError> {
        let id = Self::generate_document_id(
            &document_type.data_contract_id(),
            &self.owner_id(),
            document_type.name(),
            entropy,
            identity_contract_nonce,
            platform_version,
        )?;
        self.set_id(id);
        Ok(())
    }

    /// The number of SHA-256 blocks deriving the id takes, which is what
    /// validating the id of a create is billed for.
    ///
    /// The entropy only id has always been billed as 2 blocks and stays so.
    /// The nonce derived id is billed by what the double SHA-256 really
    /// hashes: the padded preimage (longer than before by the domain tag and
    /// the nonce) plus the one block of the second pass over the 32 byte
    /// digest. That is 4 blocks for a document type name of up to 60 bytes
    /// and 5 beyond that.
    pub fn generate_document_id_sha256_blocks(
        document_type_name: &str,
        platform_version: &PlatformVersion,
    ) -> Result<u16, ProtocolError> {
        match platform_version
            .dpp
            .document_versions
            .document_method_versions
            .generate_document_id
        {
            0 => Ok(2),
            1 => {
                // tag + contract id + owner id + name + entropy + nonce, then
                // the 0x80 byte and the 8 byte length SHA-256 pads with
                let padded_len = DOCUMENT_ID_V1_DOMAIN_TAG.len()
                    + 32
                    + 32
                    + document_type_name.len()
                    + 32
                    + 8
                    + 9;
                let first_pass_blocks = padded_len.div_ceil(64) as u16;
                // the second pass hashes the 32 byte digest of the first
                let second_pass_blocks = 1;
                Ok(first_pass_blocks + second_pass_blocks)
            }
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "Document::generate_document_id_sha256_blocks".to_string(),
                known_versions: vec![0, 1],
                received: version,
            }),
        }
    }

    /// Whether the id of a new document depends on the identity contract
    /// nonce of its create transition. When it does, the id a document
    /// carries before its create transition is built is only a placeholder.
    pub fn document_id_depends_on_nonce(
        platform_version: &PlatformVersion,
    ) -> Result<bool, ProtocolError> {
        match platform_version
            .dpp
            .document_versions
            .document_method_versions
            .generate_document_id
        {
            0 => Ok(false),
            1 => Ok(true),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "Document::document_id_depends_on_nonce".to_string(),
                known_versions: vec![0, 1],
                received: version,
            }),
        }
    }

    /// Generates the document ID
    pub fn generate_document_id_v0(
        contract_id: &Identifier,
        owner_id: &Identifier,
        document_type_name: &str,
        entropy: &[u8],
    ) -> Identifier {
        let mut buf: Vec<u8> = vec![];

        buf.extend_from_slice(&contract_id.to_buffer());
        buf.extend_from_slice(&owner_id.to_buffer());
        buf.extend_from_slice(document_type_name.as_bytes());
        buf.extend_from_slice(entropy);

        Identifier::from_bytes(&hash_double_to_vec(&buf)).unwrap()
    }

    /// Generates the document ID from the entropy and the identity contract
    /// nonce of the create transition
    pub fn generate_document_id_v1(
        contract_id: &Identifier,
        owner_id: &Identifier,
        document_type_name: &str,
        entropy: &[u8],
        identity_contract_nonce: IdentityNonce,
    ) -> Identifier {
        let mut buf: Vec<u8> = Vec::with_capacity(
            DOCUMENT_ID_V1_DOMAIN_TAG.len() + 64 + document_type_name.len() + entropy.len() + 8,
        );

        buf.extend_from_slice(DOCUMENT_ID_V1_DOMAIN_TAG);
        buf.extend_from_slice(contract_id.as_slice());
        buf.extend_from_slice(owner_id.as_slice());
        buf.extend_from_slice(document_type_name.as_bytes());
        buf.extend_from_slice(entropy);
        buf.extend_from_slice(&identity_contract_nonce.to_be_bytes());

        Identifier::from(hash_double(&buf))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ENTROPY: [u8; 32] = [7u8; 32];

    fn ids() -> (Identifier, Identifier) {
        (Identifier::from([1u8; 32]), Identifier::from([2u8; 32]))
    }

    #[test]
    fn should_derive_the_entropy_only_id_before_protocol_version_14() {
        let (contract_id, owner_id) = ids();
        let platform_version = PlatformVersion::get(13).expect("expected version 13");

        let first = Document::generate_document_id(
            &contract_id,
            &owner_id,
            "note",
            &ENTROPY,
            1,
            platform_version,
        )
        .expect("expected an id");
        let second = Document::generate_document_id(
            &contract_id,
            &owner_id,
            "note",
            &ENTROPY,
            2,
            platform_version,
        )
        .expect("expected an id");

        assert_eq!(
            first,
            Document::generate_document_id_v0(&contract_id, &owner_id, "note", &ENTROPY)
        );
        assert_eq!(first, second);
        assert!(
            !Document::document_id_depends_on_nonce(platform_version).expect("expected a version")
        );
    }

    #[test]
    fn should_derive_a_different_id_for_every_nonce() {
        let (contract_id, owner_id) = ids();
        let platform_version = PlatformVersion::latest();

        let first = Document::generate_document_id(
            &contract_id,
            &owner_id,
            "note",
            &ENTROPY,
            1,
            platform_version,
        )
        .expect("expected an id");
        let second = Document::generate_document_id(
            &contract_id,
            &owner_id,
            "note",
            &ENTROPY,
            2,
            platform_version,
        )
        .expect("expected an id");

        assert_ne!(first, second);
        assert_ne!(
            first,
            Document::generate_document_id_v0(&contract_id, &owner_id, "note", &ENTROPY)
        );
        assert!(
            Document::document_id_depends_on_nonce(platform_version).expect("expected a version")
        );
    }

    #[test]
    fn should_bill_the_nonce_derived_id_by_the_length_of_its_preimage() {
        let latest = PlatformVersion::latest();
        let version_13 = PlatformVersion::get(13).expect("expected version 13");
        let blocks = |name: &str, version| {
            Document::generate_document_id_sha256_blocks(name, version).expect("expected blocks")
        };

        // the entropy only id keeps the 2 blocks it was always billed
        assert_eq!(blocks("note", version_13), 2);
        assert_eq!(blocks(&"n".repeat(64), version_13), 2);

        assert_eq!(blocks("note", latest), 4);
        assert_eq!(blocks(&"n".repeat(60), latest), 4);
        assert_eq!(blocks(&"n".repeat(61), latest), 5);
    }

    #[test]
    fn should_keep_the_entropy_in_the_nonce_derived_id() {
        let (contract_id, owner_id) = ids();

        assert_ne!(
            Document::generate_document_id_v1(&contract_id, &owner_id, "note", &ENTROPY, 1),
            Document::generate_document_id_v1(&contract_id, &owner_id, "note", &[8u8; 32], 1),
        );
    }

    #[test]
    fn should_pin_the_nonce_derived_id() {
        let (contract_id, owner_id) = ids();

        // Every client derives this id on its own, so the preimage layout is
        // part of the protocol: a change here is a consensus change.
        assert_eq!(
            Document::generate_document_id_v1(&contract_id, &owner_id, "note", &ENTROPY, 1)
                .to_string(platform_value::string_encoding::Encoding::Hex),
            PINNED_V1_ID
        );
    }

    const PINNED_V1_ID: &str = "e574ae73396611a517691d1f89275b6e99642cb9c176ce8cf879b1665c50f15f";
}
