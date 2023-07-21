use std::{fs::File, path::PathBuf};

#[derive(serde::Deserialize, Debug)]
struct QuorumInfo {
    #[serde(deserialize_with = "dapi_grpc::deserialization::from_hex")]
    quorum_public_key: Vec<u8>,
}

pub fn load<Req, Resp>(
    file: &str,
) -> (
    Req,
    Resp,
    drive_light_client::proof::from_proof::MockQuorumInfoProvider,
)
where
    Req: dapi_grpc::Message + serde::de::DeserializeOwned,
    Resp: dapi_grpc::Message + serde::de::DeserializeOwned,
{
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join(file);

    let f = File::open(path).unwrap();
    let (req, resp, quorum): (Req, Resp, QuorumInfo) = serde_json::from_reader(f).unwrap();

    println!("req: {:?}\nresp: {:?}\nquorum: {:?}\n", req, resp, quorum);

    let pubkey = quorum.quorum_public_key;
    let mut provider = drive_light_client::proof::from_proof::MockQuorumInfoProvider::new();
    provider
        .expect_get_quorum_public_key()
        .return_once(|_, _| Ok(pubkey));
    (req, resp, provider)
}

pub fn enable_logs() {
    tracing_subscriber::fmt::fmt()
        .pretty()
        .with_ansi(true)
        .with_max_level(tracing::Level::TRACE)
        .try_init()
        .ok();
}
