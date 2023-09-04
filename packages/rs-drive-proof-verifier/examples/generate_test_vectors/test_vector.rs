use async_trait::async_trait;
use dapi_grpc::platform::v0::Proof;
use dpp::prelude::DataContract;
use serde::Serialize;

// Load TestMetadata
include!("../../../rs-drive-proof-verifier/tests/utils.rs");

#[async_trait]
pub trait TestVector {
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
    async fn test_vector<I, O>(
        &self,
        request: I,
        response: O,
        proof: &Proof,
        data_contract: Option<DataContract>,
    ) -> String
    where
        I: Serialize + Send,
        O: Serialize + Send;
}

#[async_trait]
impl TestVector for super::Api {
    async fn test_vector<I, O>(
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
        let quorum_public_key = self
            .get_quorum_key(&proof.quorum_hash, proof.quorum_type)
            .await;

        let mtd = crate::test_vector::TestMetadata {
            quorum_public_key,
            data_contract: data_contract,
        };
        let output = (request, response, mtd);

        serde_json::to_string_pretty(&output).expect("json generation failed")
    }
}
