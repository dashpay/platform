use anyhow::Result;
use dapi_grpc::platform::v0::{
    GetIdentityBalanceRequest, GetIdentityBalanceResponse, GetIdentityRequest, GetIdentityResponse,
};
use dpp::dashcore::Network;
use dpp::prelude::Identifier;
use rs_dapi_client::{Address, AddressList, DapiClient, RequestSettings};
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
        version: None,
        id: identity_id.to_vec(),
        prove: true,
    };

    println!("Fetching identity (proved)...");
    match dapi.platform.get_identity(request).await {
        Ok(response) => {
            if let Some(identity_bytes) = response.identity {
                println!(
                    "✓ Identity found! Raw response length: {} bytes",
                    identity_bytes.len()
                );

                // Now get balance
                let balance_request = GetIdentityBalanceRequest {
                    version: None,
                    id: identity_id.to_vec(),
                    prove: true,
                };

                println!("\nFetching balance...");
                match dapi.platform.get_identity_balance(balance_request).await {
                    Ok(balance_response) => {
                        if let Some(balance) = balance_response.balance {
                            let balance_value = balance.balance;
                            println!("\n✓ Balance found!");
                            println!("  Balance: {} credits", balance_value);
                            println!(
                                "  Balance in DASH: {} DASH",
                                balance_value as f64 / 100_000_000.0
                            );
                        } else {
                            println!("✗ No balance data in response");
                        }
                    }
                    Err(e) => {
                        println!("✗ Error fetching balance: {}", e);
                    }
                }
            } else {
                println!("✗ Identity not found");
            }
        }
        Err(e) => {
            println!("✗ Error fetching identity: {}", e);
        }
    }

    Ok(())
}
