use clap::Parser;
use dapi_grpc::platform::v0 as platform_proto;
use dapi_grpc::platform::v0::platform_client::PlatformClient;
use dpp::data_contract::document_type::schema::allowed_top_level_properties::ALLOWED_TRANSITION_TO_DOCUMENT_SCHEMA_V1_PROPERTIES;
use dpp::data_contract::serialized_version::DataContractInSerializationFormat;
use dpp::platform_value::Identifier;
use serde::Deserialize;
use std::time::Duration;
use tonic::transport::Channel;

const EXPLORER_MAINNET: &str = "https://platform-explorer.pshenmic.dev";
const EXPLORER_TESTNET: &str = "https://testnet.platform-explorer.pshenmic.dev";

const MAX_VALIDATORS_TO_TRY: usize = 5;

#[derive(Parser)]
#[command(
    name = "check-contract-properties",
    about = "Fetch all contracts from mainnet/testnet and check for unknown top-level document schema properties"
)]
struct Args {
    /// Network: "mainnet", "testnet"
    #[arg(short, long, default_value = "mainnet")]
    network: String,

    /// Override the DAPI gRPC URI (e.g. "https://1.2.3.4:1443")
    #[arg(long)]
    dapi_uri: Option<String>,

    /// Override the Platform Explorer API URI (e.g. "https://platform-explorer.pshenmic.dev")
    #[arg(long)]
    explorer_uri: Option<String>,

    /// Contract IDs to check (base58 or hex). If none given, fetches all from the explorer.
    #[arg(trailing_var_arg = true)]
    contract_ids: Vec<String>,
}

#[derive(Deserialize)]
struct ExplorerResponse {
    #[serde(rename = "resultSet")]
    result_set: Vec<ExplorerContract>,
    pagination: ExplorerPagination,
}

#[derive(Deserialize)]
struct ExplorerContract {
    identifier: String,
    name: Option<String>,
}

#[derive(Deserialize)]
struct ExplorerPagination {
    #[allow(dead_code)]
    page: u32,
    total: u32,
}

/// Fetches DAPI gRPC URIs by querying active validators from the explorer API.
fn fetch_dapi_uris_from_explorer(explorer_base: &str) -> Result<Vec<String>, String> {
    let url = format!(
        "{}/validators?page=1&limit={}&isActive=true",
        explorer_base, MAX_VALIDATORS_TO_TRY
    );

    let body: String = ureq::get(&url)
        .call()
        .map_err(|e| format!("Explorer API request failed: {e}"))?
        .body_mut()
        .read_to_string()
        .map_err(|e| format!("Failed to read explorer response: {e}"))?;

    let resp: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| format!("Failed to parse JSON: {e}"))?;

    let mut uris = Vec::new();
    if let Some(result_set) = resp.get("resultSet").and_then(|v| v.as_array()) {
        for validator in result_set {
            if let Some(state) = validator.get("proTxInfo").and_then(|p| p.get("state")) {
                let service = state.get("service").and_then(|v| v.as_str());
                let http_port = state.get("platformHTTPPort").and_then(|v| v.as_u64());

                if let (Some(service), Some(port)) = (service, http_port) {
                    // service is "ip:core_port", we only need the IP
                    let ip = service
                        .rsplit_once(':')
                        .map(|(ip, _)| ip)
                        .unwrap_or(service);
                    uris.push(format!("https://{ip}:{port}"));
                }
            }
        }
    }

    if uris.is_empty() {
        return Err("No active validators found with platform HTTP ports".to_string());
    }
    Ok(uris)
}

/// Fetches all contract identifiers from the Platform Explorer API.
fn fetch_all_contract_ids_from_explorer(
    explorer_base: &str,
) -> Result<Vec<(String, Identifier)>, String> {
    let mut all = Vec::new();
    let mut page = 1u32;
    let limit = 100u32;

    loop {
        let url = format!(
            "{}/dataContracts?page={}&limit={}&order=asc&order_by=block_height",
            explorer_base, page, limit
        );

        let body: String = ureq::get(&url)
            .call()
            .map_err(|e| format!("Explorer API request failed: {e}"))?
            .body_mut()
            .read_to_string()
            .map_err(|e| format!("Failed to read explorer response: {e}"))?;

        let resp: ExplorerResponse = serde_json::from_str(&body)
            .map_err(|e| format!("Failed to parse explorer JSON: {e}"))?;

        for contract in &resp.result_set {
            let label = contract
                .name
                .clone()
                .unwrap_or_else(|| contract.identifier.clone());
            let id = Identifier::from_string(
                &contract.identifier,
                dpp::platform_value::string_encoding::Encoding::Base58,
            )
            .map_err(|e| format!("Failed to parse contract ID '{}': {e}", contract.identifier))?;
            all.push((label, id));
        }

        let fetched_so_far = page * limit;
        if fetched_so_far >= resp.pagination.total {
            break;
        }
        page += 1;
    }

    Ok(all)
}

fn parse_contract_id(raw: &str) -> Identifier {
    Identifier::from_string(raw, dpp::platform_value::string_encoding::Encoding::Base58)
        .or_else(|_| {
            let bytes = hex::decode(raw).unwrap_or_else(|e| {
                eprintln!("Cannot parse '{raw}' as base58 or hex: {e}");
                std::process::exit(1);
            });
            if bytes.len() != 32 {
                eprintln!("ID '{raw}' decoded to {} bytes, expected 32", bytes.len());
                std::process::exit(1);
            }
            let arr: [u8; 32] = bytes.try_into().unwrap();
            Ok::<_, dpp::ProtocolError>(Identifier::new(arr))
        })
        .unwrap()
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let explorer_base = args.explorer_uri.clone().unwrap_or_else(|| {
        match args.network.as_str() {
            "testnet" => EXPLORER_TESTNET,
            _ => EXPLORER_MAINNET,
        }
        .to_string()
    });

    let dapi_seeds: Vec<String> = if let Some(uri) = args.dapi_uri.clone() {
        vec![uri]
    } else {
        println!("Discovering DAPI nodes from explorer ({explorer_base}) ...");
        match fetch_dapi_uris_from_explorer(&explorer_base) {
            Ok(uris) => {
                println!("Found {} DAPI node(s).", uris.len());
                uris
            }
            Err(e) => {
                eprintln!("Error discovering DAPI nodes: {e}");
                std::process::exit(1);
            }
        }
    };

    // Build list of (label, id) pairs to check
    let contracts_to_check: Vec<(String, Identifier)> = if args.contract_ids.is_empty() {
        println!("Fetching all contract IDs from explorer ({explorer_base}) ...");
        match fetch_all_contract_ids_from_explorer(&explorer_base) {
            Ok(ids) => {
                println!("Found {} contracts.\n", ids.len());
                ids
            }
            Err(e) => {
                eprintln!("Error: {e}");
                std::process::exit(1);
            }
        }
    } else {
        args.contract_ids
            .iter()
            .map(|raw| (raw.clone(), parse_contract_id(raw)))
            .collect()
    };

    // Try each seed until one connects
    let mut client = None;
    for seed_uri in &dapi_seeds {
        println!("Trying DAPI seed {seed_uri} ...");

        let mut endpoint = Channel::from_shared(seed_uri.clone())
            .expect("invalid URI")
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(30));

        if seed_uri.starts_with("https://") {
            endpoint = endpoint
                .tls_config(tonic::transport::ClientTlsConfig::new().with_enabled_roots())
                .expect("TLS config failed");
        }

        match endpoint.connect().await {
            Ok(channel) => {
                println!("Connected to {seed_uri}");
                client = Some(PlatformClient::new(channel));
                break;
            }
            Err(e) => {
                eprintln!("  Failed: {e}");
            }
        }
    }

    let mut client = client.unwrap_or_else(|| {
        eprintln!("Could not connect to any DAPI seed node");
        std::process::exit(1);
    });

    println!(
        "Checking {} contract(s) for unknown document schema properties...\n",
        contracts_to_check.len()
    );

    let bincode_config = dpp::bincode::config::standard()
        .with_big_endian()
        .with_no_limit();

    let mut total_issues = 0usize;
    let mut checked = 0usize;
    let mut errors = 0usize;

    for (label, id) in &contracts_to_check {
        let idx = checked + errors + 1;
        let request = platform_proto::GetDataContractRequest {
            version: Some(platform_proto::get_data_contract_request::Version::V0(
                platform_proto::get_data_contract_request::GetDataContractRequestV0 {
                    id: id.to_vec(),
                    prove: false,
                },
            )),
        };

        let response = match client.get_data_contract(request).await {
            Ok(resp) => resp.into_inner(),
            Err(e) => {
                eprintln!(
                    "  [{idx}/{total}] [{label}] ({id}) - ERROR fetching: {e}",
                    total = contracts_to_check.len()
                );
                errors += 1;
                continue;
            }
        };

        let contract_bytes = match response.version {
            Some(platform_proto::get_data_contract_response::Version::V0(v0)) => match v0.result {
                Some(
                    platform_proto::get_data_contract_response::get_data_contract_response_v0::Result::DataContract(
                        bytes,
                    ),
                ) => bytes,
                Some(
                    platform_proto::get_data_contract_response::get_data_contract_response_v0::Result::Proof(
                        _,
                    ),
                ) => {
                    eprintln!("  [{label}] ({id}) - got proof instead of data");
                    errors += 1;
                    continue;
                }
                None => {
                    eprintln!("  [{label}] ({id}) - empty response");
                    errors += 1;
                    continue;
                }
            },
            None => {
                eprintln!("  [{label}] ({id}) - no version in response");
                errors += 1;
                continue;
            }
        };

        let serialization_format: DataContractInSerializationFormat =
            match dpp::bincode::borrow_decode_from_slice(contract_bytes.as_slice(), bincode_config)
            {
                Ok((format, _)) => format,
                Err(e) => {
                    eprintln!("  [{label}] ({id}) - deserialization error: {e}");
                    errors += 1;
                    continue;
                }
            };

        checked += 1;
        let contract_id = serialization_format.id();
        let mut contract_has_issues = false;

        for (doc_type_name, schema_value) in serialization_format.document_schemas() {
            let map = match schema_value {
                dpp::platform_value::Value::Map(map) => map,
                _ => continue,
            };

            let unknown_keys: Vec<&str> = map
                .iter()
                .filter_map(|(key, _)| {
                    let key_str = match key {
                        dpp::platform_value::Value::Text(s) => s.as_str(),
                        _ => return None,
                    };
                    if ALLOWED_TRANSITION_TO_DOCUMENT_SCHEMA_V1_PROPERTIES.contains(&key_str) {
                        None
                    } else {
                        Some(key_str)
                    }
                })
                .collect();

            if !unknown_keys.is_empty() {
                if !contract_has_issues {
                    println!("  [{label}] ({contract_id}) - UNKNOWN PROPERTIES FOUND:");
                    contract_has_issues = true;
                }
                println!("    document type \"{doc_type_name}\": {:?}", unknown_keys);
                total_issues += unknown_keys.len();
            }
        }

        if !contract_has_issues {
            println!(
                "  [{idx}/{total}] [{label}] ({contract_id}) - OK",
                total = contracts_to_check.len()
            );
        }
    }

    println!();
    if errors > 0 {
        println!("{errors} contract(s) could not be fetched.");
    }
    if total_issues > 0 {
        println!("Found {total_issues} unknown property occurrence(s) across {checked} checked contracts.");
        std::process::exit(1);
    } else {
        println!("All {checked} checked contracts are clean.");
    }
}
