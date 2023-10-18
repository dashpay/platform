use rs_sdk::platform::dapi::transport::TransportRequest;

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct TestMetadata {
    #[serde(with = "dapi_grpc::deserialization::hexstring")]
    pub quorum_public_key: Vec<u8>,
    pub data_contract: Option<dpp::prelude::DataContract>,
}

#[allow(unused)]
pub fn load<R: TransportRequest, P: AsRef<std::path::Path>>(file: P) -> (R, R::Response)
where
    R: for<'de> serde::Deserialize<'de>,
    R::Response: for<'de> serde::Deserialize<'de>,
{
    let data = rs_sdk::platform::dapi::DumpData::load(file).expect("load test vector");

    (data.request, data.response)
}

#[allow(unused)]
pub fn enable_logs() {
    tracing_subscriber::fmt::fmt()
        .pretty()
        .with_ansi(true)
        .with_max_level(tracing::Level::TRACE)
        .try_init()
        .ok();
}
