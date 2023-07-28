#[cfg(feature = "uniffi")]
pub mod uniffi_test {

    use bytes::Bytes;

    use drive_proof_verifier::{
        uniffi_bindings::codec::{json::JsonCodec, Codec},
        Error,
    };

    include!("utils.rs");

    /// `test_proof` is a macro that generates test functions for different types of proofs.
    ///
    /// # Parameters
    ///
    /// * `$name`: The name of the test function to be generated.
    /// * `$tested_fn`: The name of the function to be tested.
    /// * `$vector`: File containing request and response data, relative to `$CARGO_MANIFEST_DIR/tests`
    /// * `$req`: The type of the request object.
    /// * `$resp`: The type of the response object.
    /// * `$result`: The expected result pattern of the test.
    /// * `$codec`: The codec used for encoding and decoding the request and response objects.
    ///
    /// # Usage
    ///
    /// This macro is used in the following way:
    ///
    /// ```rust
    /// test_proof!(
    ///     test_name,
    ///     tested_function,
    ///     "vectors/identity_not_found.json",
    ///     GetIdentityRequest,
    ///     GetIdentityResponse,
    ///     Ok(Identity),
    ///     DEFAULT_CODEC
    /// );
    /// ```
    ///
    /// In the example above, `test_name` is the name of the generated function,
    /// `tested_function` is the function to be tested,
    /// `"vectors/identity_not_found.json"` is the file containing request and response data,
    /// `GetIdentityRequest` and `GetIdentityResponse` are the types of the request and response objects respectively,
    /// `Ok(Identity)` is the expected result pattern of the test, and `DEFAULT_CODEC` is the codec used for encoding
    ///  and decoding.
    ///
    /// # Generated Function
    ///
    /// The generated function will load the specified request and response data from the vector,
    /// encode them using the specified codec, decode the encoded data and assert that it matches the original data,
    /// drive the light client with the encoded request and response data and a new quorum info callback,
    /// and finally assert that the result matches the expected result pattern.
    macro_rules! test_proof {
        ($name:ident,$tested_fn:ident,$vector:expr,$req:ty,$resp:ty,$result:pat,$codec:expr) => {
            #[cfg(feature = "uniffi")]
            #[test]
            fn $name() {
                use dapi_grpc::platform::v0::{$req, $resp};
                use drive_proof_verifier::uniffi_bindings::json::proof::*;

                let (request, response, quorum_info_callback) = load::<$req, $resp>($vector);
                let req = $codec.encode(&request).unwrap();
                assert_eq!(
                    request,
                    $codec.decode(&mut Bytes::from(req.clone())).unwrap()
                );

                let resp = $codec.encode(&response).unwrap();
                assert_eq!(
                    response,
                    $codec.decode(&mut Bytes::from(resp.clone())).unwrap()
                );

                let ret = $tested_fn(req, resp, Box::new(quorum_info_callback));
                assert!(matches!(ret, $result));
            }
        };
    }

    static CODEC: JsonCodec = JsonCodec {};

    test_proof!(
        identity_proof_json_not_found,
        identity_proof_json,
        "vectors/identity_not_found.json",
        GetIdentityRequest,
        GetIdentityResponse,
        Result::Err(Error::DocumentMissingInProof),
        CODEC
    );

    test_proof!(
        identity_by_pubkeys_proof_json,
        identity_by_pubkeys_proof_json,
        "vectors/TODO.json",
        GetIdentityByPublicKeyHashesRequest,
        GetIdentityByPublicKeyHashesResponse,
        _,
        CODEC
    );

    test_proof!(
        identities_proof_json,
        identities_proof_json,
        "vectors/TODO.json",
        GetIdentitiesRequest,
        GetIdentitiesResponse,
        Ok(_),
        CODEC
    );

    test_proof!(
        identities_by_pubkey_hashes_proof_json,
        identities_by_pubkey_hashes_proof_json,
        "vectors/TODO.json",
        GetIdentitiesByPublicKeyHashesRequest,
        GetIdentitiesByPublicKeyHashesResponse,
        Ok(_),
        CODEC
    );

    test_proof!(
        identity_balance_proof_json,
        identity_balance_proof_json,
        "vectors/TODO.json",
        GetIdentityRequest,
        GetIdentityBalanceResponse,
        Ok(_),
        CODEC
    );

    test_proof!(
        identity_balance_and_revision_proof_json,
        identity_balance_and_revision_proof_json,
        "vectors/TODO.json",
        GetIdentityRequest,
        GetIdentityBalanceAndRevisionResponse,
        Ok(_),
        CODEC
    );

    test_proof!(
        data_contract_proof_json,
        data_contract_proof_json,
        "vectors/TODO.json",
        GetDataContractRequest,
        GetDataContractResponse,
        Ok(_),
        CODEC
    );

    test_proof!(
        data_contracts_proof_json,
        data_contracts_proof_json,
        "vectors/TODO.json",
        GetDataContractsRequest,
        GetDataContractsResponse,
        Ok(_),
        CODEC
    );

    // test_proof!(
    //     documents_proof_json,
    //     DriveQuery<'static>,
    //     GetDocumentsResponse,
    //     Documents,
    //     CODEC
    // );

    // test_proof!(
    //     identity_proof_json_not_found,
    //     identity_proof_json,
    //     "vectors/identity_not_found.json",
    //     GetIdentityRequest,
    //     GetIdentityResponse,
    //     Result::Err(Error::DocumentMissingInProof),
    //     DEFAULT_CODEC
    // );

    // test_proof!(
    //     get_identities_by_pubkey_hashes_not_found,
    //     identities_by_pubkey_hashes_json,
    //     "vectors/get_identities_by_hashes_not_found.json",
    //     GetIdentitiesByPublicKeyHashesRequest,
    //     GetIdentitiesByPublicKeyHashesResponse,
    //     Result::Err(Error::DocumentMissingInProof),
    //     DEFAULT_CODEC
    // );
}
