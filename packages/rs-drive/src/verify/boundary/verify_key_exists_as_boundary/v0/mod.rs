use crate::drive::Drive;
use crate::error::proof::ProofError;
use crate::error::Error;
use grovedb::operations::proof::GroveDBProof;

impl Drive {
    pub(super) fn verify_key_exists_as_boundary_v0(
        proof: &[u8],
        path: &[&[u8]],
        key: &[u8],
    ) -> Result<bool, Error> {
        let bincode_config = bincode::config::standard()
            .with_big_endian()
            .with_no_limit();

        let grovedb_proof: GroveDBProof = bincode::decode_from_slice(proof, bincode_config)
            .map(|(p, _)| p)
            .map_err(|e| {
                Error::Proof(ProofError::CorruptedProof(format!(
                    "cannot decode GroveDBProof: {}",
                    e
                )))
            })?;

        grovedb_proof
            .key_exists_as_boundary(path, key)
            .map_err(|e| {
                Error::Proof(ProofError::CorruptedProof(format!(
                    "error checking boundary key: {}",
                    e
                )))
            })
    }
}
