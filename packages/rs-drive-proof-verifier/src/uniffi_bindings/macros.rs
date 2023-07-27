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
        /// Given protobuf request and response, retrieve encapsulated objects from proof.
        ///
        /// # Arguments
        ///
        /// * request - encoded request sent to the server
        /// * response - encoded response received from the server
        /// * callback - trait that should be implemented by the caller, that will retrieve additional quorum
        /// information (eg. public key) needed to verify the proof
        ///
        /// # Returns
        ///
        /// Returns encoded object(s) retrieved from the server.
        ///
        /// # Encoding
        ///
        /// Input arguments (`request`, `response`) and returned data is serialized.
        /// Default serialization method is defined as [DEFAULT_CODEC](crate::uniffi_bindings::codec::DEFAULT_CODEC).
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
