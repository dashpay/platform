use dapi_grpc::platform::v0::*;

/// Create function `$name` that will retrieve CBOR-encoded `$result` object from request `$req` and response `$resp`.
///
/// `$provider` must implement [`crate::proof::QuorumInfoProvider`].
///
/// `$result` must be a type that implements trait [`FromProof<$req,$resp,$provider>`](crate::proof::FromProof).
macro_rules! proof_to_cbor {
    ($name:ident,$req:ty,$resp:ty,$result:ty) => {
        /// Given protobuf request and response, retrieve encapsulated objects from proof and encode it using CBOR
        ///
        /// # Arguments
        ///
        /// * req_proto - protobuf-encoded request sent to the server
        /// * resp_proto - protobuf-encoded response received from the server
        ///
        /// # Returns
        ///
        /// Returns CBOR-encoded object(s) retrieved from the server.
        #[no_mangle]
        #[uniffi::export]
        pub fn $name(
            req_proto: Vec<u8>,
            resp_proto: Vec<u8>,
            provider: Box<dyn crate::proof::from_proof::QuorumInfoProvider>,
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

            let result = <$result>::from_proof(&request, &response, provider)?;

            result.to_cbor().map_err(|e| crate::Error::ProtocolError {
                error: e.to_string(),
            })
        }
    };
}

proof_to_cbor!(
    identity_proof_to_cbor,
    GetIdentityRequest,
    GetIdentityResponse,
    dpp::identity::Identity
);

#[uniffi::export]
pub fn hello() {
    println!("hello world")
}

#[cfg(test)]
mod test {
    #[cfg(feature = "mock")]
    #[test]
    fn test_get_identity_proof_to_cbor() {
        // let req =
        let req_proto = vec![0u8; 32];
        let resp_proto = vec![0u8; 32];

        super::identity_proof_to_cbor(
            req_proto,
            resp_proto,
            Box::new(crate::proof::from_proof::MockQuorumInfoProvider::new()),
        )
        .unwrap();
    }
}
