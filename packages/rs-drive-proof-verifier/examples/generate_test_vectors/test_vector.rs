use async_trait::async_trait;
use dapi_grpc::platform::v0::Proof;
use dashcore_rpc::{
    dashcore::{hashes::Hash, QuorumHash},
    dashcore_rpc_json::QuorumType,
};
use dpp::prelude::DataContract;
use drive_abci::rpc::core::{CoreRPCLike, DefaultCoreRPC};
use serde::Serialize;

// Load TestMetadata
include!("../../tests/utils.rs");

macro_rules! get_proof {
    ($response:expr, $result_type:ty) => {{
        use $result_type as Result;
        let proof = if let Result::Proof(proof) = $response.result.as_ref().expect("result") {
            proof
        } else {
            panic!("missing proof in response")
        };

        proof
    }};
}

#[async_trait]
pub trait TestVector {
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
