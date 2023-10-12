use dapi_grpc::platform::v0::Proof;
use dpp::prelude::DataContract;
use drive_proof_verifier::QuorumInfoProvider;
use serde::Serialize;
// Load TestMetadata
include!("../../../rs-drive-proof-verifier/tests/utils.rs");

impl super::Api {
    /// Generate a test vector for a given request and response.
    ///
    /// # Arguments
    ///
    /// * `request` - The request sent to the RPC server.
    /// * `response` - The response received from the RPC server.
    /// * `proof` - The proof received from the RPC server.
    ///
    /// # Returns
    ///
    /// A JSON string containing the test vector.
    pub async fn test_vector<I, O>(
        &self,
        request: I,
        response: O,
        proof: &Proof,
        data_contract: Option<DataContract>,
    ) -> String
    where
        I: Serialize + Send,
        O: Serialize + Send,
    {
        let quorum_hash = proof
            .quorum_hash
            .clone()
            .try_into()
            .expect("quorum hash must have 32 bytes");

        let quorum_public_key = self
            .sdk
            .get_quorum_public_key(proof.quorum_type, quorum_hash, 0)
            .expect("quorum public key not found");

        let mtd = crate::test_vector::TestMetadata {
            quorum_public_key: quorum_public_key.to_vec(),
            data_contract,
        };
        let output = (request, response, mtd);

        serde_json::to_string_pretty(&output).expect("json generation failed")
    }
}
