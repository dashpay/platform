use anyhow::Result;
use dapi_grpc::platform::v0::{
    get_identity_balance_request, get_identity_balance_response, get_identity_request,
    get_identity_response, GetIdentityBalanceRequest, GetIdentityRequest,
};
use dpp::prelude::Identifier;
use rs_dapi_client::{Address, AddressList, DapiClient, DapiRequestExecutor, RequestSettings};
use std::str::FromStr;

#[tokio::main]
async fn main() -> Result<()> {
    println!("Dash Balance Checker - Testnet (Direct DAPI with Proofs)");
    println!("========================================================\n");

    let identity_id_string = "5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk";
    println!("Identity ID: {}", identity_id_string);

    // Parse the identity ID properly
    let identity_id = Identifier::from_string(
        identity_id_string,
        dpp::platform_value::string_encoding::Encoding::Base58,
    )?;
    println!("Parsed Identity ID: {}", identity_id);

    // Testnet addresses
    let addresses = vec![
        Address::from_str("https://52.13.132.146:1443")?,
        Address::from_str("https://52.89.154.48:1443")?,
        Address::from_str("https://44.227.137.77:1443")?,
    ];

    // Create DAPI client
    let dapi = DapiClient::new(
        AddressList::from_iter(addresses),
        RequestSettings::default(),
    );

    // Create request for identity (proved)
    let request = GetIdentityRequest {
        version: Some(get_identity_request::Version::V0(
            get_identity_request::GetIdentityRequestV0 {
                id: identity_id.to_vec(),
                prove: true,
            },
        )),
    };

    println!("Fetching identity (proved)...");
    match dapi.execute(request, RequestSettings::default()).await {
        Ok(response) => {
            // Check if we have a response with version
            if let Some(version) = response.inner.version {
                match version {
                    get_identity_response::Version::V0(v0) => {
                        if let Some(result) = v0.result {
                            match result {
                                get_identity_response::get_identity_response_v0::Result::Identity(identity_bytes) => {
                                    println!(
                                        "✓ Identity found! Raw response length: {} bytes",
                                        identity_bytes.len()
                                    );

                                    // Now get balance
                                    let balance_request = GetIdentityBalanceRequest {
                                        version: Some(get_identity_balance_request::Version::V0(
                                            get_identity_balance_request::GetIdentityBalanceRequestV0 {
                                                id: identity_id.to_vec(),
                                                prove: true,
                                            }
                                        )),
                                    };

                                    println!("\nFetching balance...");
                                    match dapi.execute(balance_request, RequestSettings::default()).await {
                                        Ok(balance_response) => {
                                            if let Some(version) = balance_response.inner.version {
                                                match version {
                                                    get_identity_balance_response::Version::V0(v0) => {
                                                        if let Some(result) = v0.result {
                                                            match result {
                                                                get_identity_balance_response::get_identity_balance_response_v0::Result::Balance(balance) => {
                                                                    println!("\n✓ Balance found!");
                                                                    println!("  Balance: {} credits", balance);
                                                                    println!(
                                                                        "  Balance in DASH: {} DASH",
                                                                        balance as f64 / 100_000_000.0
                                                                    );
                                                                }
                                                                _ => println!("✗ Unexpected response type")
                                                            }
                                                        } else {
                                                            println!("✗ No balance data in response");
                                                        }
                                                    }
                                                }
                                            } else {
                                                println!("✗ No version in balance response");
                                            }
                                        }
                                        Err(e) => {
                                            println!("✗ Error fetching balance: {}", e);
                                        }
                                    }
                                }
                                _ => println!("✗ Identity not found or proof returned")
                            }
                        } else {
                            println!("✗ No result in response");
                        }
                    }
                }
            } else {
                println!("✗ No version in response");
            }
        }
        Err(e) => {
            println!("✗ Error fetching identity: {}", e);
        }
    }

    Ok(())
}
