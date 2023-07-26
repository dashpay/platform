use dapi_grpc::platform::v0 as grpc;
use dpp::{
    identity::PartialIdentity,
    prelude::{DataContract, Identity},
};
use drive_light_client::proof::from_proof::{
    DataContracts, FromProof, Identities, IdentitiesByPublicKeyHashes, IdentityBalance,
    IdentityBalanceAndRevision,
};

include!("utils.rs");

/// `test_maybe_from_proof` is a macro that generates test functions for different types of proofs.
///
/// # Parameters
///
/// * `$name`: The name of the test function to be generated.
/// * `$req`: The type of the request object.
/// * `$resp`: The type of the response object.
/// * `$object`: The type of object from which the proof may be derived.
/// * `$vector`: File containing request and response data, relative to `$CARGO_MANIFEST_DIR/tests`
/// * `$result`: The expected result pattern of the test.
///
/// # Usage
///
/// This macro is used in the following way:
///
/// ```rust
/// test_maybe_from_proof!(
///     test_name,
///     GetIdentityRequest,
///     GetIdentityResponse,
///     Identity,
///     "vectors/identity_not_found.json",
///     Ok(Some(Identity)),
/// );
/// ```
///
/// In the example above, `test_name` is the name of the generated test function,
/// `"vectors/identity_not_found.json"` is the file containing request and response data,
/// `GetIdentityRequest`, `GetIdentityResponse`, and `Identity` are the types of the request, response, and object respectively,
/// `Ok(Some(Identity))` is the expected result pattern of the test.
///
/// # Generated Function
///
/// The generated function will load the specified request and response data from the vector,
/// attempt to derive an instance of the specified object type from the loaded proofs,
/// and finally assert that the result matches the expected result pattern.
///
/// # Vector file format
///
/// Vector file should contain sequence of 3 objects:
///
/// * request
/// * response
/// * quorum public key
///
/// ## Request
///
/// Request should contain JSON-encoded request data structure
///
/// ## Response
///
/// Response should contain two elements: `result` and `metadata`.
/// `result` should contain `proof` structure.
/// `metadata` should directly contain returned metadata.
///
/// Note that, when retrieveing response using a tool like grpcui, the `result` element is missing
/// and must be added manually.
///
/// ## Quorum public key
///
/// Quorum public key should be **hex-encoded** value of `"quorum_public_key"` field.
///
/// /// ## Example
/// ```json
/// {
///    "id": "base64-encoded",
///    "prove": true
/// },
/// {
///    "result": {
///       "proof": {
///          "grovedb_proof": "base64-encoded",
///          ...
///       }
///    },
///    "metadata": {
///        "height": "365",
///        ...
///    }
/// },
/// {
///    "quorum_public_key": "hex-encoded"
/// }
/// ```
macro_rules! test_maybe_from_proof {
    ($name:ident,$req:ty,$resp:ty,$object:ty,$vector:expr,$result:pat) => {
        #[test]
        fn $name() {
            enable_logs();

            let (request, response, quorum_info_callback) = load::<$req, $resp>($vector);

            let ret =
                <$object>::maybe_from_proof(&request, &response, Box::new(quorum_info_callback));
            println!("Result: {:?}", ret);
            assert!(matches!(ret, $result));
        }
    };
}

test_maybe_from_proof! {
    identity_not_found,
    grpc::GetIdentityRequest,
    grpc::GetIdentityResponse,
    Identity,
    "vectors/identity_not_found.json",
    Ok(None)
}

test_maybe_from_proof! {
    get_identity_by_pubkeys_not_found,
    grpc::GetIdentityByPublicKeyHashesRequest,
    grpc::GetIdentityByPublicKeyHashesResponse,
    Identity,
    "vectors/TODO.json",
    Ok(None)
}

test_maybe_from_proof! {
    get_identities_not_found,
    grpc::GetIdentitiesRequest,
    grpc::GetIdentitiesResponse,
    Identities,
    "vectors/identities_not_found.json",
    Ok(Some(_)) // cannot write a match like `Ok(Some(Vec[None]))`
}

test_maybe_from_proof! {
    get_identities_by_hashes_not_found,
    grpc::GetIdentitiesByPublicKeyHashesRequest,
    grpc::GetIdentitiesByPublicKeyHashesResponse,
    IdentitiesByPublicKeyHashes,
    "vectors/identities_by_pubkeys_not_found.json",
    Ok(None)
}

// todo continue from here
test_maybe_from_proof! {
    get_identity_balance_not_found,
    grpc::GetIdentityRequest,
    grpc::GetIdentityBalanceResponse,
    IdentityBalance,
    "vectors/TODO.json",
    Ok(None)
}

test_maybe_from_proof! {
    get_identity_balance_and_revision_not_found,
    grpc::GetIdentityRequest,
    grpc::GetIdentityBalanceAndRevisionResponse,
    IdentityBalanceAndRevision,
    "vectors/TODO.json",
    Ok(None)
}

test_maybe_from_proof! {
    get_identity_keys_not_found,
    grpc::GetIdentityKeysRequest,
    grpc::GetIdentityKeysResponse,
    PartialIdentity,
    "vectors/identity_keys_not_found.json",
    Ok(None)
}

test_maybe_from_proof! {
    get_data_contract_not_found,
    grpc::GetDataContractRequest,
    grpc::GetDataContractResponse,
    DataContract,
    "vectors/TODO.json",
    Ok(None)
}

test_maybe_from_proof! {
    get_data_contracts_not_found,
    grpc::GetDataContractsRequest,
    grpc::GetDataContractsResponse,
    DataContracts,
    "vectors/TODO.json",
    Ok(None)
}

// test_maybe_from_proof! {
//     get_documents_not_found,
//     DriveQuery,
//     grpc::GetDocumentsResponse,
//     Documents,
//     "vectors/TODO.json",
//     Ok(None)
// }
