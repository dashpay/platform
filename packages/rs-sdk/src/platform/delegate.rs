//! Macros to delegate TransportRequest and FromProof to an enum wrapper.
//!
//! Two enum wrappers are created using [delegate_enum!](crate::delegate_enum!):
//!
//! * request
//! * response
//!
//! Each of these enums has a variant for each request/response pair. Name of variant in request enum is the same as
//! the name of variant in response.

/// Delegate the execution of a transport request to the appropriate variant of an enum wrapper.
///
/// Given two enums, request and response, that wrap multiple requests/responses for one object type, this macro
/// implements [TransportRequest](crate::platform::dapi::transport::TransportRequest) for the request enum and
/// delegates the execution of the transport request to the appropriate variant.
///
/// Each variant in request enum must have a corresponding variant in response enum.
/// Variant names in request and response enums must match.
/// Variants must take exactly one argument that implements
/// [TransportRequest](crate::platform::dapi::transport::TransportRequest) (for request) and
/// [TransportResponse](crate::platform::dapi::transport::TransportResponse) (for response), where for a given variant,
/// response must be the response type of the request variant.
///
/// Macro [delegate_enum!](crate::delegate_enum!) can be used to generate these enums and implement required
/// traits.
#[macro_export]
macro_rules! delegate_transport_request_variant {
    ($request:ty, $response:ty, $($variant:ident),+) => {
        impl $crate::platform::dapi::transport::TransportRequest for $request {

            type Client = $crate::platform::dapi::transport::PlatformGrpcClient;

            type Response = $response;

            const SETTINGS_OVERRIDES: $crate::platform::dapi::RequestSettings = $crate::platform::dapi::RequestSettings::default();

            /// TODO: Not sure how to do that
            fn method_name(&self) -> &'static str {
                ""
            }

            fn execute_transport<'c>(
                self,
                client: &'c mut Self::Client,
                settings: &$crate::platform::dapi::transport::AppliedRequestSettings,
            ) -> $crate::platform::dapi::transport::BoxFuture<'c, Result<Self::Response, <Self::Client as $crate::platform::dapi::transport::TransportClient>::Error>> {
                use futures::FutureExt;
                use $request::*;

                let settings =settings.clone();

                // We need to build new async box because we have to map response to the $response type
                match self {$(
                        $variant(request) => async move {
                            request
                                .execute_transport(client, &settings)
                                .await
                                .map(Into::into)
                        }
                        .boxed(),
                )*}
            }
        }
    }
}

/// Delegate the execution of a [FromProof](drive_proof_verifier::FromProof) trait to an enum supporting multiple variants.
///
/// In order to support multiple request/response types for one object (like, GetIdentityRequest and
/// GetIdentityByFirstPublicKeyHashRequest for Identity), we need to wrap them in an enum and
/// delegate the execution of the transport request to the appropriate variant.
///
/// See [delegate_enum!](crate::delegate_enum!) for more details.
#[macro_export]
macro_rules! delegate_from_proof_variant {
    ($request:ty, $response:ty, $object:ty, $(($variant:ident, $req: ty, $resp: ty)),+) => {
        impl drive_proof_verifier::FromProof<$request> for $object {
            type Request = $request;
            type Response = $response;

            fn maybe_from_proof_with_metadata<'a, I: Into<Self::Request>, O: Into<Self::Response>>(
                request: I,
                response: O,
                version: &dpp::version::PlatformVersion,
                provider: &'a dyn drive_proof_verifier::ContextProvider,
            ) -> Result<(Option<Self>, ResponseMetadata), drive_proof_verifier::Error>
            where
                Self: Sized + 'a,
            {
                use $request as req;
                use $response as resp;

                let request: Self::Request = request.into();
                let response: Self::Response = response.into();

                match request {$(
                    req::$variant(request) => {
                        if let resp::$variant(response) = response {
                            <Self as drive_proof_verifier::FromProof<$req>>::maybe_from_proof_with_metadata(
                                request, response, version, provider,
                            )
                        } else {
                            Err(drive_proof_verifier::Error::ResponseDecodeError {
                                error: format!(
                                    "expected {}, got {}",
                                    "GetIdentityResponse",
                                    $crate::platform::delegate::type_name(&response),
                                ),
                            })
                        }
                    },
                )*
                }
            }
        }
    };
}

#[macro_export]
/// Define enums that will wrap multiple requests/responses for one object.
///
/// In order to support multiple request/response types for one object (like, GetIdentityRequest and
/// GetIdentityByPublicKeyHashRequest for Identity), we need to wrap them in an enum and
/// delegate [TransportRequest](crate::platform::dapi::transport::TransportRequest)
/// and [FromProof](drive_proof_verifier::FromProof) to the appropriate variant.
///
/// This macro creates enums for requests (`$request`) and responses (`$response`) and variants  (`$variant`) for
/// each request (`$req`) /response (`$req`) pair. Variant name in request and response enums are the same.
///
/// It also calls [delegate_transport_request_variant!](crate::delegate_transport_request_variant!) and
/// [delegate_from_proof_variant!](crate::delegate_from_proof_variant!) to delegate
/// [TransportRequest](crate::platform::dapi::transport::TransportRequest)
/// and [FromProof](drive_proof_verifier::FromProof)
/// traits to the appropriate variant.
macro_rules! delegate_enum {
    ($request:ident, $response:ident, $object:ty, $(($variant:ident, $req: ty, $resp: ty)),+) => {
        /// Wrapper around multiple requests for one object type.
        #[derive(Debug, Clone, derive_more::From, dapi_grpc_macros::Mockable)]
        #[cfg_attr(feature="mocks", derive(serde::Serialize, serde::Deserialize))]
        #[allow(missing_docs)]
        pub enum $request {
            $(
                $variant($req),
            )+
        }

        /// Wrapper around multiple responses for one object type.
        #[derive(Debug, Clone, Default, derive_more::From, dapi_grpc_macros::Mockable)]
        #[cfg_attr(feature="mocks", derive(serde::Serialize, serde::Deserialize))]
        #[allow(missing_docs)]
        pub enum $response {
            #[default]
            /// Unknown or unsupported request type.
            ///
            /// Used as default variant for the enum in mocks.
            ///
            /// Can cause panic.
            Unknown,
            $(
                $variant($resp),
            )+
        }

        $crate::delegate_transport_request_variant! {
            $request,
            $response,
            $($variant),+
        }

        $crate::delegate_from_proof_variant! {
            $request,
            $response,
            $object,
            $(($variant,$req,$resp)),+
        }
    };
}
/// Return type name of a variable as a String
pub(crate) fn type_name<T>(_v: &T) -> String {
    std::any::type_name::<T>().to_string()
}
