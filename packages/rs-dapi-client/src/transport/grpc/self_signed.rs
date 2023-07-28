use hyper::{client::connect::HttpConnector, Client};
use hyper_rustls::HttpsConnector;
use lazy_static::lazy_static;
use rustls::client::{ServerCertVerified, ServerCertVerifier};

lazy_static! {
    /// Setups a [connector](hyper::client::connect::Connect) using HTTPS, but bypassing
    /// server's cerificates validation for development use.
    pub(crate) static ref INSECURE_CONNECTOR: HttpsConnector<HttpConnector> = {
        struct InsecureVerifier {}

        impl ServerCertVerifier for InsecureVerifier {
            fn verify_server_cert(
                &self,
                _end_entity: &rustls::Certificate,
                _intermediates: &[rustls::Certificate],
                _server_name: &rustls::ServerName,
                _scts: &mut dyn Iterator<Item = &[u8]>,
                _ocsp_response: &[u8],
                _now: std::time::SystemTime,
            ) -> Result<rustls::client::ServerCertVerified, rustls::Error> {
                Ok(ServerCertVerified::assertion())
            }
        }

        let tls_config = rustls::ClientConfig::builder()
            .with_safe_defaults()
            .with_custom_certificate_verifier(std::sync::Arc::new(InsecureVerifier {}))
            .with_no_client_auth();

        hyper_rustls::HttpsConnectorBuilder::new()
            .with_tls_config(tls_config)
            .https_only()
            .enable_http1()
            .build()
    };
}
