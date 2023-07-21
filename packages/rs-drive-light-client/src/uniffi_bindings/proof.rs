use crate::uniffi_bindings::codec::DEFAULT_CODEC;
use dapi_grpc::platform::v0::*;

/// Generate a wrapper function that exports uniffi bindings for proof processing code
///
/// Generate function
/// `$name(req_proto: Vec<u8>,resp_cbor: Vec<u8>, callback: Box<dyn QuorumInfoProvider)-> Result<Vec<u8>, crate::Error>`
///  that will return CBOR-encoded `$result` object from protobuf-encoded DAPI GRPC request of type `$req` and response of type `$resp`.
///
/// # Arguments
///
/// * `$name` - name of wrapper function to generate
/// * `$req` - type of request message
/// * `$resp` - type of response message
/// * `$result` - type of result; must implement trait [`FromProof<$req,$resp>`](crate::proof::from_proof::FromProof).
///
/// # Example
///
/// The following code will generate function [`identity_proof_to_cbor`].
///
/// ```no_run
/// uniffi_proof_binding_wrapper!(
///     identity_proof_to_cbor,
///     GetIdentityRequest,
///     GetIdentityResponse,
///     dpp::identity::Identity
/// );
/// ```
#[macro_export]
macro_rules! uniffi_proof_binding_wrapper {
    ($name:ident,$req:ty,$resp:ty,$result:ty,$codec:expr) => {
        /// Given protobuf request and response, retrieve encapsulated objects from proof and encode it using CBOR
        ///
        /// # Arguments
        ///
        /// * req_proto - CBOR-encoded request sent to the server
        /// * resp_proto - CBOR-encoded response received from the server
        /// * callback - trait that should be implemented by the caller, that will retrieve additional quorum
        /// information (eg. public key) needed to verify the proof
        ///
        /// # Returns
        ///
        /// Returns CBOR-encoded object(s) retrieved from the server.
        #[no_mangle]
        #[uniffi::export]
        pub fn $name(
            request: Vec<u8>,
            response: Vec<u8>,
            callback: Box<dyn crate::proof::from_proof::QuorumInfoProvider>,
        ) -> Result<Vec<u8>, crate::Error>
        where
            $req: serde::Deserialize<'static>,
            $resp: serde::Deserialize<'static>,
            $result: serde::Serialize,
        {
            use crate::proof::from_proof::FromProof;
            use crate::uniffi_bindings::codec::Codec;
            use bytes::Bytes;

            let request = $codec
                .decode::<$req>(&mut Bytes::from(request))
                .map_err(move |e| crate::Error::RequestDecodeError {
                    error: e.to_string(),
                })?;

            let response = $codec
                .decode::<$resp>(&mut Bytes::from(response))
                .map_err(|e| crate::Error::ResponseDecodeError {
                    error: e.to_string(),
                })?;

            let result = <$result>::from_proof(&request, &response, callback)?;

            $codec
                .encode(&result)
                .map_err(|e| crate::Error::ProtocolError {
                    error: e.to_string(),
                })
        }
    };
}

uniffi_proof_binding_wrapper!(
    identity_proof_json,
    GetIdentityRequest,
    GetIdentityResponse,
    dpp::identity::Identity,
    DEFAULT_CODEC
);

uniffi_proof_binding_wrapper!(
    identity_by_pubkeys_proof_json,
    GetIdentityByPublicKeyHashesRequest,
    GetIdentityByPublicKeyHashesResponse,
    dpp::identity::Identity,
    DEFAULT_CODEC
);

// uniffi_proof_binding_wrapper!(
//     identities_proof_to_cbor,
//     GetIdentitiesRequest,
//     GetIdentitiesResponse,
//     Identities
// );

// uniffi_proof_binding_wrapper!(
//     identities_proof_to_cbor,
//     GetIdentitiesByPublicKeyHashesRequest,
//     GetIdentitiesByPublicKeyHashesResponse,
//     IdentitiesByPublicKeyHashes
// );

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
