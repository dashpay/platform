use crate::{
    proof::from_proof::*, uniffi_bindings::codec::json::JsonCodec, uniffi_proof_binding_wrapper,
};
use dapi_grpc::platform::v0::*;
use dpp::prelude::DataContract;
use drive::query::DriveQuery;

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

uniffi_proof_binding_wrapper!(
    identities_proof_json,
    GetIdentitiesRequest,
    GetIdentitiesResponse,
    Identities,
    CODEC
);

uniffi_proof_binding_wrapper!(
    identities_by_pubkey_hashes_proof_json,
    GetIdentitiesByPublicKeyHashesRequest,
    GetIdentitiesByPublicKeyHashesResponse,
    IdentitiesByPublicKeyHashes,
    CODEC
);

uniffi_proof_binding_wrapper!(
    identity_balance_proof_json,
    GetIdentityRequest,
    GetIdentityBalanceResponse,
    IdentityBalance,
    CODEC
);

uniffi_proof_binding_wrapper!(
    identity_balance_and_revision_proof_json,
    GetIdentityRequest,
    GetIdentityBalanceAndRevisionResponse,
    IdentityBalanceAndRevision,
    CODEC
);

uniffi_proof_binding_wrapper!(
    data_contract_proof_json,
    GetDataContractRequest,
    GetDataContractResponse,
    DataContract,
    CODEC
);

uniffi_proof_binding_wrapper!(
    data_contracts_proof_json,
    GetDataContractsRequest,
    GetDataContractsResponse,
    DataContracts,
    CODEC
);

// uniffi_proof_binding_wrapper!(
//     documents_proof_json,
//     DriveQuery<'static>,
//     GetDocumentsResponse,
//     Documents,
//     CODEC
// );
