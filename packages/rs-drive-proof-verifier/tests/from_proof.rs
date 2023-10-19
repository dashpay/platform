use dapi_grpc::platform::v0::{self as grpc};
use dpp::prelude::{DataContract, Identity};
use drive_proof_verifier::proof::from_proof::{DataContractHistory, FromProof, Length};
use rs_sdk::SdkBuilder;

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
/// * `$expected`: expected result of the test, either Ok(number or records) OR Err(Error)
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
/// ### Warning
///
/// 1. when retrieveing response using a tool like grpcui, the `result` element is missing
/// and must be added manually.
/// 2. chain_id is usually invalid, you need to replace it with the one from Tenderdash's genesis.json
/// 3. Fields like `limit` should contain int; sometimes they contain `"value"` field and must be flattened.
///
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
    ($name:ident,$req:ty,$resp:ty,$object:ty,$vector:expr,$expected:expr) => {
        #[test]
        fn $name()
        where
            $object: FromProof<$req> + Length,
        {
            enable_logs();

            let expected: Result<usize, drive_proof_verifier::Error> = $expected;

            let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests")
                .join($vector);
            let (request, response) = load::<$req, _>(&path);

            let mut sdk = SdkBuilder::new_mock().build().expect("build sdk");
            sdk.mock()
                .quorum_info_dir(&path.parent().expect("valid parent dir"));

            let ret = <$object as FromProof<$req>>::maybe_from_proof(request, response, &sdk);

            tracing::info!(?ret, ?expected, "object retrieved from proof");

            match ret {
                Err(ref e) => assert_eq!(
                    expected.expect_err("Expected Ok, got error").to_string(),
                    e.to_string(),
                ), // Note: not tested
                Ok(None) => assert_eq!(expected.expect("Expected Ok got Err"), 0),
                Ok(Some(ref o)) => {
                    assert_eq!(expected.expect("Expected Ok, got Err"), o.count_some());
                }
            }
        }
    };
}

// Identity

test_maybe_from_proof! {
    identity_not_found,
    grpc::GetIdentityRequest,
    grpc::GetIdentityResponse,
    Identity,
    "vectors/identity_not_found.json",
    Result::Ok(0)
}

test_maybe_from_proof! {
    identity_ok,
    grpc::GetIdentityRequest,
    grpc::GetIdentityResponse,
    Identity,
    "vectors/identity_ok.json",
    Ok(1)
}

// Identity by pubkey
// TODO: uncomment and implement TransportRequest
/*
test_maybe_from_proof! {
    identity_by_pubkey_ok,
    grpc::GetIdentityByPublicKeyHashesRequest,
    grpc::GetIdentityByPublicKeyHashesResponse,
    Identity,
    "vectors/identity_by_pubkey_ok.json",
    Ok(1)
}
 */
// TODO: GRPC request fails with:
// drive error: grovedb: path key not found: key not found in Merk for get
// TODO: uncomment and implement TransportRequest
/*
test_maybe_from_proof! {
    identity_by_pubkey_not_found,
    grpc::GetIdentityByPublicKeyHashesRequest,
    grpc::GetIdentityByPublicKeyHashesResponse,
    Identity,
    "vectors/identity_by_pubkey_not_found.json",
    Ok(0)
}
*/
// Identity Balance
// TODO: uncomment and implement TransportRequest
/*
test_maybe_from_proof! {
    identity_balance_ok,
    grpc::GetIdentityRequest,
    grpc::GetIdentityBalanceResponse,
    IdentityBalance,
    "vectors/identity_balance_ok.json",
    Ok(1)
}

test_maybe_from_proof! {
    identity_balance_not_found,
    grpc::GetIdentityRequest,
    grpc::GetIdentityBalanceResponse,
    IdentityBalance,
    "vectors/identity_balance_not_found.json",
    Ok(0)
}

// Identity balance and revision
test_maybe_from_proof! {
    identity_balance_and_revision_ok,
    grpc::GetIdentityRequest,
    grpc::GetIdentityBalanceAndRevisionResponse,
    IdentityBalanceAndRevision,
    "vectors/identity_balance_and_revision_ok.json",
    Ok(1)
}

test_maybe_from_proof! {
    identity_balance_and_revision_not_found,
    grpc::GetIdentityRequest,
    grpc::GetIdentityBalanceAndRevisionResponse,
    IdentityBalanceAndRevision,
    "vectors/identity_balance_and_revision_not_found.json",
    Ok(0)
}
*/
// Identity keys
// TODO: uncomment and implement TransportRequest
/*
test_maybe_from_proof! {
    identity_keys_identity_not_found,
    grpc::GetIdentityKeysRequest,
    grpc::GetIdentityKeysResponse,
    IdentityPublicKeys,
    "vectors/identity_keys_identity_not_found.json",
    Ok(0)
}

test_maybe_from_proof! {
    identity_keys_ok,
    grpc::GetIdentityKeysRequest,
    grpc::GetIdentityKeysResponse,
    IdentityPublicKeys,
    "vectors/identity_keys_ok.json",
    Ok(2)
}

test_maybe_from_proof! {
    identity_keys_good_identity_wrong_keys,
    grpc::GetIdentityKeysRequest,
    grpc::GetIdentityKeysResponse,
    IdentityPublicKeys,
    "vectors/identity_keys_good_identity_wrong_keys.json",
    Ok(0)
}

test_maybe_from_proof! {
    identity_keys_2_of_6_ok,
    grpc::GetIdentityKeysRequest,
    grpc::GetIdentityKeysResponse,
    IdentityPublicKeys,
    "vectors/identity_keys_2_of_6_ok.json",
    Ok(2)
}
*/
// Data Contract

test_maybe_from_proof! {
    data_contract_ok,
    grpc::GetDataContractRequest,
    grpc::GetDataContractResponse,
    DataContract,
    "vectors/data_contract_ok.json",
    Ok(1)
}

test_maybe_from_proof! {
    data_contract_not_found,
    grpc::GetDataContractRequest,
    grpc::GetDataContractResponse,
    DataContract,
    "vectors/data_contract_not_found.json",
    Ok(0)
}

// Data contract history

test_maybe_from_proof! {
    data_contract_history_ok,
    grpc::GetDataContractHistoryRequest,
    grpc::GetDataContractHistoryResponse,
    DataContractHistory,
    "vectors/data_contract_history_ok.json",
    Ok(2)
}

test_maybe_from_proof! {
    data_contract_history_not_found,
    grpc::GetDataContractHistoryRequest,
    grpc::GetDataContractHistoryResponse,
    DataContractHistory,
    "vectors/data_contract_history_not_found.json",
    Ok(0)
}

// Multiple data contracts

// One contract, with history enabled, requested and found
// TODO: uncomment and implement TransportRequest
/*
test_maybe_from_proof! {
    data_contracts_1_ok,
    grpc::GetDataContractsRequest,
    grpc::GetDataContractsResponse,
    DataContracts,
    "vectors/data_contracts_1_ok.json",
    Ok(1)
}

// Two contracts, with history enabled, requested and found
test_maybe_from_proof! {
    data_contracts_2_ok,
    grpc::GetDataContractsRequest,
    grpc::GetDataContractsResponse,
    DataContracts,
    "vectors/data_contracts_2_ok.json",
    Ok(2)
}

// One contract (without history) requested and found
test_maybe_from_proof! {
    data_contracts_no_history_1_ok,
    grpc::GetDataContractsRequest,
    grpc::GetDataContractsResponse,
    DataContracts,
    "vectors/data_contracts_no_history_1_ok.json",
    Ok(1)
}

// Two contracts (without history) requested and found
test_maybe_from_proof! {
    data_contracts_no_history_2_ok,
    grpc::GetDataContractsRequest,
    grpc::GetDataContractsResponse,
    DataContracts,
    "vectors/data_contracts_no_history_2_ok.json",
    Ok(2)
}

// 2 contracts: with and without history, requested and found
test_maybe_from_proof! {
    data_contracts_mixed_2_ok,
    grpc::GetDataContractsRequest,
    grpc::GetDataContractsResponse,
    DataContracts,
    "vectors/data_contracts_mixed_2_ok.json",
    Ok(2)
}

// Two existing contracts, one with history and one without, out of 3 requested
test_maybe_from_proof! {
    data_contracts_mixed_2_of_3,
    grpc::GetDataContractsRequest,
    grpc::GetDataContractsResponse,
    DataContracts,
    "vectors/data_contracts_mixed_2_of_3.json",
    Ok(2)
}

// One data contract requested and not found
test_maybe_from_proof! {
    data_contracts_1_not_found,
    grpc::GetDataContractsRequest,
    grpc::GetDataContractsResponse,
    DataContracts,
    "vectors/data_contracts_1_not_found.json",
    Ok(0)
}
*/
// As some IDEs don't support tests generted my macros, you might want to
// manually call all tests here.
// #[test]
// fn run_test() {
// data_contracts_no_history_1_ok()
// }
