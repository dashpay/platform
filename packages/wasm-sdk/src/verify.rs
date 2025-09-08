use dash_sdk::dpp::dashcore::Network;
use dash_sdk::dpp::data_contract::DataContract;
use dash_sdk::dpp::document::{Document, DocumentV0Getters};
use dash_sdk::dpp::identity::Identity;
use dash_sdk::dpp::platform_value::string_encoding::Encoding;
use dash_sdk::dpp::serialization::PlatformDeserializableWithPotentialValidationFromVersionedStructure;
use dash_sdk::dpp::version::PlatformVersion;
use dash_sdk::platform::proto::get_identity_request::{
    GetIdentityRequestV0, Version as GetIdentityRequestVersion,
};
use dash_sdk::platform::proto::get_identity_response::{
    get_identity_response_v0, GetIdentityResponseV0, Version,
};
use dash_sdk::platform::proto::{GetDocumentsResponse, GetIdentityRequest, Proof, ResponseMetadata};
use dash_sdk::platform::DocumentQuery;
use drive_proof_verifier::types::Documents;
use drive_proof_verifier::FromProof;
use wasm_bindgen::prelude::wasm_bindgen;

use crate::context_provider::WasmContext;
use crate::dpp::{DataContractWasm, IdentityWasm};

#[wasm_bindgen]
pub async fn verify_identity_response() -> Option<IdentityWasm> {
    let request = dash_sdk::dapi_grpc::platform::v0::GetIdentityRequest {
        version: Some(GetIdentityRequestVersion::V0(GetIdentityRequestV0 {
            id: vec![],
            prove: true,
        })),
    };

    let response = dash_sdk::dapi_grpc::platform::v0::GetIdentityResponse {
        version: Some(Version::V0(GetIdentityResponseV0 {
            result: Some(get_identity_response_v0::Result::Proof(Proof {
                grovedb_proof: vec![],
                quorum_hash: vec![],
                signature: vec![],
                round: 0,
                block_id_hash: vec![],
                quorum_type: 0,
            })),
            metadata: Some(ResponseMetadata {
                height: 0,
                core_chain_locked_height: 0,
                epoch: 0,
                time_ms: 0,
                protocol_version: 0,
                chain_id: "".to_string(),
            }),
        })),
    };

    let context = WasmContext {};

    let (response, _metadata, _proof) =
        <Identity as FromProof<GetIdentityRequest>>::maybe_from_proof_with_metadata(
            request,
            response,
            Network::Dash,
            PlatformVersion::latest(),
            &context,
        )
        .expect("parse proof");

    response.map(IdentityWasm::from)
}

#[wasm_bindgen]
pub async fn verify_data_contract() -> Option<DataContractWasm> {
    let request = dash_sdk::dapi_grpc::platform::v0::GetDataContractRequest {
        version: Some(
            dash_sdk::platform::proto::get_data_contract_request::Version::V0(
                dash_sdk::platform::proto::get_data_contract_request::GetDataContractRequestV0 {
                    id: vec![],
                    prove: true,
                },
            ),
        ),
    };

    let response = dash_sdk::dapi_grpc::platform::v0::GetDataContractResponse {
        version: Some(
            dash_sdk::platform::proto::get_data_contract_response::Version::V0(
                dash_sdk::platform::proto::get_data_contract_response::GetDataContractResponseV0 {
                    result: Some(
                        dash_sdk::platform::proto::get_data_contract_response::get_data_contract_response_v0::Result::Proof(
                            dash_sdk::platform::proto::Proof {
                                grovedb_proof: vec![],
                                quorum_hash: vec![],
                                signature: vec![],
                                round: 0,
                                block_id_hash: vec![],
                                quorum_type: 0,
                            },
                        ),
                    ),
                    metadata: Some(dash_sdk::platform::proto::ResponseMetadata {
                        height: 0,
                        core_chain_locked_height: 0,
                        epoch: 0,
                        time_ms: 0,
                        protocol_version: 0,
                        chain_id: "".to_string(),
                    }),
                },
            ),
        ),
    };

    let context = WasmContext {};

    let (response, _, _) = <DataContract as FromProof<
        dash_sdk::dapi_grpc::platform::v0::GetDataContractRequest,
    >>::maybe_from_proof_with_metadata(
        request,
        response,
        Network::Dash,
        PlatformVersion::latest(),
        &context,
    )
    .expect("parse proof");

    response.map(DataContractWasm::from)
}

#[wasm_bindgen]
pub async fn verify_documents() -> Vec<DocumentWasm> {
    // TODO: this is a dummy implementation, replace with actual verification
    let data_contract =
        DataContract::versioned_deserialize(&[13, 13, 13], false, PlatformVersion::latest())
            .expect("create data contract");

    let query = DocumentQuery::new(data_contract, "asd").expect("create query");

    let response = GetDocumentsResponse {
        version: Some(dash_sdk::platform::proto::get_documents_response::Version::V0(
            dash_sdk::platform::proto::get_documents_response::GetDocumentsResponseV0 {
                result: Some(
                    dash_sdk::platform::proto::get_documents_response::get_documents_response_v0::Result::Proof(
                        Proof {
                            grovedb_proof: vec![],
                            quorum_hash: vec![],
                            signature: vec![],
                            round: 0,
                            block_id_hash: vec![],
                            quorum_type: 0,
                        },
                    ),
                ),
                metadata: Some(ResponseMetadata {
                    height: 0,
                    core_chain_locked_height: 0,
                    epoch: 0,
                    time_ms: 0,
                    protocol_version: 0,
                    chain_id: "".to_string(),
                }),
            },
        )),
    };

    let (documents, _, _) =
        <Documents as FromProof<DocumentQuery>>::maybe_from_proof_with_metadata(
            query,
            response,
            Network::Dash,
            PlatformVersion::latest(),
            &WasmContext {},
        )
        .expect("parse proof");

    documents
        .unwrap()
        .into_iter()
        .filter(|(_, doc)| doc.is_some())
        .map(|(_, doc)| DocumentWasm(doc.unwrap()))
        .collect()
}

#[wasm_bindgen]
pub struct DocumentWasm(Document);
#[wasm_bindgen]
impl DocumentWasm {
    pub fn id(&self) -> String {
        self.0.id().to_string(Encoding::Base58)
    }
}
