use dapi_grpc::platform::v0::*;

/// Generate a wrapper function that exports uniffi bindings for proof processing code
///
/// Generate function
/// `$name(req_proto: Vec<u8>,resp_proto: Vec<u8>, callback: Box<dyn QuorumInfoProvider)-> Result<Vec<u8>, crate::Error>`
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
    ($name:ident,$req:ty,$resp:ty,$result:ty) => {
        /// Given protobuf request and response, retrieve encapsulated objects from proof and encode it using CBOR
        ///
        /// # Arguments
        ///
        /// * req_proto - protobuf-encoded request sent to the server
        /// * resp_proto - protobuf-encoded response received from the server
        /// * callback - trait that should be implemented by the caller, that will retrieve additional quorum
        /// information (eg. public key) needed to verify the proof
        ///
        /// # Returns
        ///
        /// Returns CBOR-encoded object(s) retrieved from the server.
        #[no_mangle]
        #[uniffi::export]
        pub fn $name(
            req_proto: Vec<u8>,
            resp_proto: Vec<u8>,
            callback: Box<dyn crate::proof::from_proof::QuorumInfoProvider>,
        ) -> Result<Vec<u8>, crate::Error> {
            use crate::proof::from_proof::FromProof;
            use dapi_grpc::Message;
            let request = <$req>::decode(bytes::Bytes::from(req_proto))
                .map_err(|e| crate::Error::ProtoEncodeError {
                    error: e.to_string(),
                })
                .map_err(|e| crate::Error::ProtoRequestDecodeError {
                    error: e.to_string(),
                })?;

            let response = <$resp>::decode(bytes::Bytes::from(resp_proto))
                .map_err(|e| crate::Error::ProtoEncodeError {
                    error: e.to_string(),
                })
                .map_err(|e| crate::Error::ProtoRequestDecodeError {
                    error: e.to_string(),
                })?;

            let result = <$result>::from_proof(&request, &response, callback)?;

            result.to_cbor().map_err(|e| crate::Error::ProtocolError {
                error: e.to_string(),
            })
        }
    };
}

uniffi_proof_binding_wrapper!(
    identity_proof_to_cbor,
    GetIdentityRequest,
    GetIdentityResponse,
    dpp::identity::Identity
);

#[cfg(test)]
mod test {
    #[cfg(feature = "mock")]
    #[test]
    fn test_get_identity_proof_to_cbor() {
        use dapi_grpc::{
            platform::v0::{GetIdentityRequest, GetIdentityResponse},
            Message,
        };

        let req_proto = GetIdentityRequest {
            // fill data here
            ..Default::default()
        }
        .encode_to_vec();

        let resp_proto = GetIdentityResponse {
            // fill data here
            ..Default::default()
        }
        .encode_to_vec();

        let ret = super::identity_proof_to_cbor(
            req_proto,
            resp_proto,
            Box::new(crate::proof::from_proof::MockQuorumInfoProvider::new()),
        )
        .unwrap();

        assert_ne!(ret.len(), 0)
    }
}
