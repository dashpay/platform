use crate::{
    proof::from_proof::*, uniffi_bindings::codec::json::JsonCodec, uniffi_proof_binding_wrapper,
};
use dapi_grpc::platform::v0::*;

static CODEC: JsonCodec = JsonCodec {};

uniffi_proof_binding_wrapper!(
    identity_proof_json,
    GetIdentityRequest,
    GetIdentityResponse,
    dpp::identity::Identity,
    CODEC
);

uniffi_proof_binding_wrapper!(
    identity_by_pubkeys_proof_json,
    GetIdentityByPublicKeyHashesRequest,
    GetIdentityByPublicKeyHashesResponse,
    dpp::identity::Identity,
    CODEC
);

// uniffi_proof_binding_wrapper!(
//     identities_proof_to_cbor,
//     GetIdentitiesRequest,
//     GetIdentitiesResponse,
//     Identities
// );

uniffi_proof_binding_wrapper!(
    identities_by_pubkey_hashes_json,
    GetIdentitiesByPublicKeyHashesRequest,
    GetIdentitiesByPublicKeyHashesResponse,
    IdentitiesByPublicKeyHashes,
    CODEC
);

// uniffi_proof_binding_wrapper!(
//     identity_balance_proof_to_cbor,
//     GetIdentityRequest,
//     GetIdentityBalanceResponse,
//     IdentityBalance
// );

// uniffi_proof_binding_wrapper!(
//     identity_balance_and_revision_proof_to_cbor,
//     GetIdentityRequest,
//     GetIdentityBalanceAndRevisionResponse,
//     IdentityBalanceAndRevision
// );

// uniffi_proof_binding_wrapper!(
//     data_contract_proof_to_cbor,
//     GetDataContractRequest,
//     GetDataContractResponse,
//     DataContract
// );

// uniffi_proof_binding_wrapper!(
//     data_contracts_proof_to_cbor,
//     GetDataContractsRequest,
//     GetDataContractsResponse,
//     DataContracts
// );

// uniffi_proof_binding_wrapper!(
//     documents_proof_to_cbor,
//     DriveQuery,
//     GetDocumentsResponse,
//     Documents
// );
