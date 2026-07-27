//! Register a data contract on Dash Platform.
//!
//! Reads a contract JSON file (e.g. one of the fixtures under
//! `packages/rs-drive/tests/supporting_files/contract/`), takes the
//! identity that should own it and a private key, fetches the
//! identity from Platform, finds which of its public keys matches
//! the supplied private key, and broadcasts a `DataContractCreate`
//! state transition.
//!
//! The `id` and `ownerId` from the JSON file are intentionally
//! overridden: `DataContractCreateTransition::new_from_data_contract`
//! regenerates the contract id deterministically from
//! `(owner_identity_id, identity_nonce)` and sets the owner id to
//! the supplied identity, so any values in the fixture are dropped.

use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;
use dash_sdk::platform::transition::put_contract::PutContract;
use dash_sdk::platform::Fetch;
use dash_sdk::{Sdk, SdkBuilder};
use dpp::dashcore::Network;
use dpp::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::signer::Signer;
use dpp::identity::{Identity, IdentityPublicKey, KeyType, Purpose, SecurityLevel};
use dpp::platform_value::string_encoding::Encoding;
use dpp::platform_value::Identifier;
use dpp::prelude::DataContract;
use rs_dapi_client::AddressList;
use rs_sdk_trusted_context_provider::TrustedHttpContextProvider;
use simple_signer::single_key_signer::SingleKeySigner;

#[derive(Parser, Debug)]
#[command(
    name = "register-contract",
    about = "Register a data contract on Dash Platform.\n\
             \n\
             The contract JSON's `id` and `ownerId` fields are\n\
             overridden — the on-chain contract id is regenerated\n\
             from (identity_id, identity_nonce) and the owner is\n\
             set to the supplied --identity."
)]
struct Args {
    /// Path to the contract JSON file (e.g. a fixture under
    /// packages/rs-drive/tests/supporting_files/contract/).
    #[arg(short = 'c', long = "contract")]
    contract_file: PathBuf,

    /// Identity id (base58) that will own the new contract.
    #[arg(short = 'i', long = "identity")]
    identity_id: String,

    /// Private key for that identity — WIF or 64-char hex.
    /// Must correspond to a CRITICAL + AUTHENTICATION +
    /// ECDSA_SECP256K1 key on the identity (the only key shape
    /// DPP accepts on a contract-create signature).
    #[arg(short = 'k', long = "private-key")]
    private_key: String,

    /// DAPI address, e.g. https://52.12.176.90:1443.
    #[arg(short = 'a', long = "address")]
    address: String,

    /// Network: mainnet | testnet | devnet | regtest. Defaults
    /// to testnet.
    #[arg(short = 'n', long = "network", default_value = "testnet")]
    network: String,

    /// Optional devnet name (only when --network devnet).
    #[arg(long = "devnet")]
    devnet_name: Option<String>,
}

fn parse_network(s: &str) -> Result<Network, String> {
    match s.to_ascii_lowercase().as_str() {
        "mainnet" | "main" => Ok(Network::Mainnet),
        "testnet" | "test" => Ok(Network::Testnet),
        "devnet" | "dev" => Ok(Network::Devnet),
        "regtest" => Ok(Network::Regtest),
        other => Err(format!(
            "unknown network '{other}' (expected mainnet | testnet | devnet | regtest)"
        )),
    }
}

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("Error: {e}");
            ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<(), String> {
    let args = Args::parse();

    let network = parse_network(&args.network)?;

    let identity_id = Identifier::from_string(&args.identity_id, Encoding::Base58)
        .map_err(|e| format!("invalid --identity (expected base58): {e}"))?;

    let signer = SingleKeySigner::from_string(args.private_key.trim(), network)
        .map_err(|e| format!("invalid --private-key: {e}"))?;

    let json_bytes = std::fs::read(&args.contract_file)
        .map_err(|e| format!("failed to read {}: {e}", args.contract_file.display()))?;
    let json_value: serde_json::Value = serde_json::from_slice(&json_bytes).map_err(|e| {
        format!(
            "failed to parse {} as JSON: {e}",
            args.contract_file.display()
        )
    })?;

    // Non-validating deserialization: the file may carry an `id` /
    // `ownerId` from a fixture that we're about to overwrite, and
    // strict id-vs-owner checks would reject otherwise-valid
    // contracts. The schema itself is still parsed and shape-
    // checked. The on-chain `validate_basic_structure` runs server-
    // side during state-transition validation anyway. Uses the
    // canonical (no-validation) `serde_json::from_value::<DataContract>`
    // path — the validating opt-in is `DataContract::from_json`.
    let mut data_contract: DataContract = serde_json::from_value(json_value)
        .map_err(|e| format!("failed to build DataContract from JSON: {e}"))?;

    // Set owner so the SDK's PutContract path fetches the right
    // identity nonce. `new_from_data_contract` will also re-set
    // owner_id and regenerate the contract id from
    // (owner_id, identity_nonce) — but that runs after the nonce
    // fetch, so the owner must already be correct here.
    data_contract.set_owner_id(identity_id);

    let address = args
        .address
        .parse()
        .map_err(|e| format!("failed to parse --address {}: {e}", args.address))?;
    let address_list = AddressList::from_iter([address]);

    let context_provider = TrustedHttpContextProvider::new(
        network,
        args.devnet_name.clone(),
        NonZeroUsize::new(100).expect("non-zero cache size"),
    )
    .map_err(|e| format!("failed to build context provider: {e}"))?;

    let sdk: Sdk = SdkBuilder::new(address_list)
        .with_network(network)
        .with_context_provider(context_provider)
        .build()
        .map_err(|e| format!("failed to build SDK: {e}"))?;

    eprintln!(
        "Fetching identity {} from {}...",
        args.identity_id, args.address
    );
    let identity = Identity::fetch(&sdk, identity_id)
        .await
        .map_err(|e| format!("failed to fetch identity: {e}"))?
        .ok_or_else(|| format!("identity {} not found", args.identity_id))?;

    let signing_key = select_signing_key(&identity, &signer)?;

    eprintln!(
        "Signing with key id {} (purpose={:?}, security_level={:?}, key_type={:?})",
        signing_key.id(),
        signing_key.purpose(),
        signing_key.security_level(),
        signing_key.key_type()
    );

    eprintln!("Broadcasting contract create transition...");
    let confirmed = data_contract
        .put_to_platform_and_wait_for_response(&sdk, signing_key, &signer, None)
        .await
        .map_err(|e| format!("failed to register contract: {e}"))?;

    println!("Contract registered successfully.");
    println!("  contract_id: {}", confirmed.id());
    println!("  owner_id:    {}", confirmed.owner_id());
    println!("  version:     {}", confirmed.version());

    Ok(())
}

/// Find the identity's first public key that:
///   1. matches the supplied private key (so we can sign with it), AND
///   2. satisfies the DPP-mandated triple for a contract-create
///      signature: AUTHENTICATION purpose, CRITICAL security level,
///      ECDSA_SECP256K1 key type.
///
/// Disabled keys are skipped.
fn select_signing_key(
    identity: &Identity,
    signer: &SingleKeySigner,
) -> Result<IdentityPublicKey, String> {
    let mut matched_but_unusable: Vec<&IdentityPublicKey> = Vec::new();

    for public_key in identity.public_keys().values() {
        if !signer.can_sign_with(public_key) {
            continue;
        }
        if public_key.is_disabled() {
            matched_but_unusable.push(public_key);
            continue;
        }
        if public_key.purpose() == Purpose::AUTHENTICATION
            && public_key.security_level() == SecurityLevel::CRITICAL
            && public_key.key_type() == KeyType::ECDSA_SECP256K1
        {
            return Ok(public_key.clone());
        }
        matched_but_unusable.push(public_key);
    }

    if matched_but_unusable.is_empty() {
        Err(format!(
            "private key does not match any public key on identity {}",
            identity.id()
        ))
    } else {
        let details = matched_but_unusable
            .iter()
            .map(|pk| {
                format!(
                    "  id={} purpose={:?} security_level={:?} key_type={:?} disabled={}",
                    pk.id(),
                    pk.purpose(),
                    pk.security_level(),
                    pk.key_type(),
                    pk.is_disabled()
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        Err(format!(
            "private key matched a key on identity {} but no key meets the\n\
             contract-create requirements (AUTHENTICATION + CRITICAL + ECDSA_SECP256K1).\n\
             Matched keys:\n{}",
            identity.id(),
            details
        ))
    }
}
