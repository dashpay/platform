use clap::{Args, Subcommand};
use dapi_grpc::platform::v0::get_identity_by_public_key_hash_request::GetIdentityByPublicKeyHashRequestV0;
use dapi_grpc::platform::v0::get_identity_by_public_key_hash_response::{
    self as get_identity_by_public_key_hash_response,
    get_identity_by_public_key_hash_response_v0::Result as ByKeyResult,
};
use dapi_grpc::platform::v0::{
    get_identity_by_public_key_hash_request, platform_client::PlatformClient,
    GetIdentityByPublicKeyHashRequest,
};
use dapi_grpc::tonic::{transport::Channel, Request};

use crate::error::{CliError, CliResult};

#[derive(Subcommand, Debug)]
pub enum IdentityCommand {
    /// Fetch identity by unique public key hash
    ByKey(ByKeyCommand),
}

#[derive(Args, Debug)]
pub struct ByKeyCommand {
    /// Public key hash (20-byte hex string)
    #[arg(value_name = "HEX")]
    pub public_key_hash: String,
    /// Request cryptographic proof alongside the identity
    #[arg(long, default_value_t = false)]
    pub prove: bool,
}

pub async fn run(url: &str, command: IdentityCommand) -> CliResult<()> {
    match command {
        IdentityCommand::ByKey(cmd) => by_key(url, cmd).await,
    }
}

async fn by_key(url: &str, cmd: ByKeyCommand) -> CliResult<()> {
    let pk_hash = hex::decode(&cmd.public_key_hash).map_err(|source| CliError::InvalidHash {
        hash: cmd.public_key_hash.clone(),
        source,
    })?;

    let channel = Channel::from_shared(url.to_string()).map_err(|source| CliError::InvalidUrl {
        url: url.to_string(),
        source: Box::new(source),
    })?;
    let mut client = PlatformClient::connect(channel).await?;

    let request = GetIdentityByPublicKeyHashRequest {
        version: Some(get_identity_by_public_key_hash_request::Version::V0(
            GetIdentityByPublicKeyHashRequestV0 {
                public_key_hash: pk_hash,
                prove: cmd.prove,
            },
        )),
    };

    let response = client
        .get_identity_by_public_key_hash(Request::new(request))
        .await?
        .into_inner();

    let Some(get_identity_by_public_key_hash_response::Version::V0(v0)) = response.version else {
        return Err(CliError::EmptyResponse("getIdentityByPublicKeyHash"));
    };

    print_metadata(v0.metadata.as_ref());

    match v0.result {
        Some(ByKeyResult::Identity(identity_bytes)) => {
            if identity_bytes.is_empty() {
                println!("❌ Identity not found for the provided public key hash");
            } else {
                println!(
                    "✅ Identity bytes: {} ({} bytes)",
                    hex::encode_upper(&identity_bytes),
                    identity_bytes.len()
                );
            }
        }
        Some(ByKeyResult::Proof(proof)) => {
            print_proof(&proof);
        }
        None => println!("ℹ️  Response did not include identity data"),
    }

    Ok(())
}

fn print_metadata(metadata: Option<&dapi_grpc::platform::v0::ResponseMetadata>) {
    if let Some(meta) = metadata {
        println!("ℹ️  Metadata:");
        println!("    height: {}", meta.height);
        println!(
            "    core_chain_locked_height: {}",
            meta.core_chain_locked_height
        );
        println!("    epoch: {}", meta.epoch);
        println!("    protocol_version: {}", meta.protocol_version);
        println!("    chain_id: {}", meta.chain_id);
        println!("    time_ms: {}", meta.time_ms);
    }
}

fn print_proof(proof: &dapi_grpc::platform::v0::Proof) {
    println!("🔐 Proof received:");
    println!("    quorum_hash: {}", hex::encode_upper(&proof.quorum_hash));
    println!("    signature bytes: {}", proof.signature.len());
    println!("    grovedb_proof bytes: {}", proof.grovedb_proof.len());
    println!("    round: {}", proof.round);
}
