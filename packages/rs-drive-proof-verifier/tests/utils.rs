use std::{fs::File, path::PathBuf};

use base64::engine::GeneralPurposeConfig;

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct TestMetadata {
    #[serde(with = "dapi_grpc::deserialization::hexstring")]
    pub quorum_public_key: Vec<u8>,
    pub data_contract: Option<dpp::prelude::DataContract>,
}

#[allow(unused)]
pub fn load<Req, Resp>(
    file: &str,
) -> (
    Req,
    Resp,
    TestMetadata,
    drive_proof_verifier::proof::from_proof::MockQuorumInfoProvider,
)
where
    Req: serde::de::DeserializeOwned, // dapi_grpc::Message
    Resp: serde::de::DeserializeOwned,
{
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join(file);

    let f = File::open(path).unwrap();
    // Use serde_bytes_repr to deserialize base64-encoded bytes
    let mut json_de = serde_json::Deserializer::new(serde_json::de::IoRead::new(&f));
    let base64_config = GeneralPurposeConfig::new();
    let b64_de = serde_bytes_repr::ByteFmtDeserializer::new_base64(
        &mut json_de,
        base64::alphabet::STANDARD,
        base64_config,
    );
    let (req, resp, metadata): (Req, Resp, TestMetadata) =
        serde::Deserialize::deserialize(b64_de).expect("deserialize test vector json");

    // println!("req: {:?}\nresp: {:?}\nquorum: {:?}\n", req, resp, quorum);

    let pubkey = metadata
        .quorum_public_key
        .clone()
        .try_into()
        .expect("pubkey size");
    let mut provider = drive_proof_verifier::proof::from_proof::MockQuorumInfoProvider::new();
    provider
        .expect_get_quorum_public_key()
        .return_once(move |_, _, _| Ok(pubkey));

    (req, resp, metadata, provider)
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
