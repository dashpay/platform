//! The verification of a proof of the records a moderated contract keeps by document type,
//! then document id: the removal records and the settled-deletion approvals are proved alike,
//! so both verify here, told apart by the record type ([`ContractDocumentRecord`]).

use crate::drive::contract::moderation::types::{
    decode_document_record_element, ContractDocumentRecord, ContractDocumentRemovalsQuery,
};
use crate::drive::Drive;
use crate::error::proof::ProofError;
use crate::error::Error;
use crate::verify::RootHash;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::GroveDb;

impl Drive {
    /// The root hash of the proof, and the records of `T` it holds for the query, in document
    /// id order; ids proved absent are left out.
    pub(super) fn verify_contract_document_records_v0<T: ContractDocumentRecord>(
        proof: &[u8],
        contract_id: Identifier,
        query: &ContractDocumentRemovalsQuery,
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Vec<T::Entry>), Error> {
        let path_query =
            Self::contract_document_records_query(contract_id.to_buffer(), T::RECORDS, query);
        let (root_hash, proved_key_values) = if verify_subset_of_proof {
            GroveDb::verify_subset_query(proof, &path_query, &platform_version.drive.grove_version)?
        } else {
            GroveDb::verify_query(proof, &path_query, &platform_version.drive.grove_version)?
        };

        let entries = proved_key_values
            .into_iter()
            .filter_map(|(_path, key, element)| element.map(|element| (key, element)))
            .map(|(key, element)| {
                decode_document_record_element::<T>(&key, &element).map_err(|description| {
                    Error::Proof(ProofError::CorruptedProof(format!(
                        "contract {}s proof is malformed: {}",
                        T::RECORDS.record_name(),
                        description
                    )))
                })
            })
            .collect::<Result<Vec<_>, Error>>()?;

        Ok((root_hash, entries))
    }
}
