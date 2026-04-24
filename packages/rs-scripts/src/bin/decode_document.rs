use base64::Engine;
use clap::Parser;
use data_contracts::SystemDataContract;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
use dpp::document::Document;
use dpp::document::DocumentV0Getters;
use dpp::platform_value::Identifier;
use dpp::system_data_contracts::load_system_data_contract;
use platform_version::version::PlatformVersion;

// Keep in sync with SystemDataContract enum in packages/data-contracts/src/lib.rs
const SYSTEM_CONTRACTS: &[(&str, SystemDataContract)] = &[
    ("withdrawals", SystemDataContract::Withdrawals),
    ("dpns", SystemDataContract::DPNS),
    ("dashpay", SystemDataContract::Dashpay),
    (
        "masternode-reward-shares",
        SystemDataContract::MasternodeRewards,
    ),
    ("wallet-utils", SystemDataContract::WalletUtils),
    ("token-history", SystemDataContract::TokenHistory),
    ("keyword-search", SystemDataContract::KeywordSearch),
];

#[derive(Parser)]
#[command(
    name = "decode-document",
    about = "Decode a platform document from hex or base64 bytes"
)]
struct Args {
    /// Document bytes (base64 or hex encoded)
    doc_bytes: String,

    /// System data contract: name (e.g. "withdrawals") or ID in base58/base64/hex
    #[arg(short, long)]
    contract: String,

    /// Document type name within the contract (e.g. "withdrawal", "domain")
    #[arg(short, long)]
    doc_type: String,

    /// Input encoding: "base64", "hex", or "auto" (default: auto, tries base64 then hex)
    #[arg(short, long, default_value = "auto")]
    format: String,
}

fn resolve_system_contract(input: &str) -> SystemDataContract {
    // Try by name first
    for (name, sc) in SYSTEM_CONTRACTS {
        if input.eq_ignore_ascii_case(name) {
            return *sc;
        }
    }

    // Try parsing as an identifier (base58, base64, or hex)
    let id = Identifier::from_string_unknown_encoding(input).unwrap_or_else(|_| {
        eprintln!("Unknown contract: '{input}'");
        eprintln!(
            "Must be a name ({}) or an ID in base58/base64/hex",
            SYSTEM_CONTRACTS
                .iter()
                .map(|(n, _)| *n)
                .collect::<Vec<_>>()
                .join(", ")
        );
        std::process::exit(1);
    });

    for (_, sc) in SYSTEM_CONTRACTS {
        if sc.id() == id {
            return *sc;
        }
    }

    eprintln!("No system contract found with ID {id}");
    std::process::exit(1);
}

fn main() {
    let args = Args::parse();

    let platform_version = PlatformVersion::latest();

    let system_contract = resolve_system_contract(&args.contract);

    let data_contract = match load_system_data_contract(system_contract, platform_version) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to load system data contract: {e}");
            std::process::exit(1);
        }
    };

    let document_type = match data_contract.document_type_for_name(&args.doc_type) {
        Ok(dt) => dt,
        Err(e) => {
            eprintln!("Unknown document type '{}': {e}", args.doc_type);
            eprintln!(
                "Available types: {}",
                data_contract
                    .document_types()
                    .keys()
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            std::process::exit(1);
        }
    };

    let bytes = match args.format.as_str() {
        "base64" => base64::engine::general_purpose::STANDARD
            .decode(&args.doc_bytes)
            .unwrap_or_else(|e| {
                eprintln!("Invalid base64: {e}");
                std::process::exit(1);
            }),
        "hex" => hex::decode(&args.doc_bytes).unwrap_or_else(|e| {
            eprintln!("Invalid hex: {e}");
            std::process::exit(1);
        }),
        _ => {
            // Try base64 first (most common — gRPC responses are base64),
            // then hex. This avoids misinterpreting hex-only base64 strings.
            if let Ok(b) = base64::engine::general_purpose::STANDARD.decode(&args.doc_bytes) {
                b
            } else if let Ok(b) = hex::decode(&args.doc_bytes) {
                b
            } else {
                eprintln!("Failed to decode document bytes as base64 or hex");
                eprintln!("Hint: use --format base64 or --format hex to force a specific encoding");
                std::process::exit(1);
            }
        }
    };

    let document = match Document::from_bytes(&bytes, document_type, platform_version) {
        Ok(doc) => doc,
        Err(e) => {
            eprintln!("Failed to deserialize document: {e}");
            std::process::exit(1);
        }
    };

    println!("id:         {}", document.id());
    println!("owner_id:   {}", document.owner_id());
    if let Some(created_at) = document.created_at() {
        println!("created_at: {} ({}ms)", format_ts(created_at), created_at);
    }
    if let Some(updated_at) = document.updated_at() {
        println!("updated_at: {} ({}ms)", format_ts(updated_at), updated_at);
    }
    if let Some(revision) = document.revision() {
        println!("revision:   {}", revision);
    }
    println!();
    println!("properties:");
    for (key, value) in document.properties() {
        println!("  {key}: {value}");
    }
}

fn format_ts(ms: u64) -> String {
    let secs = (ms / 1000) as i64;
    let nanos = ((ms % 1000) * 1_000_000) as u32;
    let dt = chrono::DateTime::from_timestamp(secs, nanos);
    match dt {
        Some(dt) => dt.format("%Y-%m-%d %H:%M:%S UTC").to_string(),
        None => format!("invalid timestamp: {ms}"),
    }
}
