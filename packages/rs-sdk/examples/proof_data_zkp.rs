//! Example demonstrating how to use proof data for zero-knowledge proof applications.
//!
//! This example shows how to fetch identities and documents with their associated
//! proof data, and use the new merkle proof APIs for ZK circuits.

use dash_sdk::platform::{FetchWithProof, Identity};
use dash_sdk::{platform::Identifier, Sdk};
use dpp::identity::accessors::IdentityGettersV0;

/// Example function to generate a zero-knowledge proof using identity key merkle proofs.
///
/// This demonstrates how a ZK application would use the merkle proofs to prove
/// key ownership without revealing which key was used.
fn demonstrate_identity_key_zk_proof(identity: &Identity) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== Identity Key Merkle Proofs for ZK ===");
    
    if identity.public_keys().is_empty() {
        println!("No keys found in identity");
        return Ok(());
    }
    
    // Build merkle tree of all identity keys
    let merkle_tree = identity.build_keys_merkle_tree()?;
    let keys_root = merkle_tree.root();
    
    println!("Keys merkle root: {}", hex::encode(keys_root));
    println!("Total keys: {}", identity.public_keys().len());
    
    // Get merkle proof for a specific key
    let first_key_id = *identity.public_keys().keys().next().unwrap();
    let key_proof = identity.get_key_merkle_proof(first_key_id)?;
    
    println!("\nMerkle proof for key {}:", key_proof.key_id);
    println!("  Purpose: {:?}", key_proof.key_purpose);
    println!("  Security level: {:?}", key_proof.key_security_level);
    println!("  Proof path length: {} nodes", key_proof.proof_path.len());
    
    // Show the proof path structure
    for (level, (sibling_hash, is_left)) in key_proof.proof_path.iter().enumerate() {
        println!("  Level {}: sibling position={}, hash={}...", 
            level,
            if *is_left { "left" } else { "right" },
            hex::encode(&sibling_hash[..4])
        );
    }
    
    println!("\nZK Proof Application:");
    println!("  1. Public input: keys_root = {}", hex::encode(&keys_root[..8]));
    println!("  2. Private inputs: key data and merkle path");
    println!("  3. Circuit proves: key exists in identity without revealing which key");
    
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the SDK
    let sdk = Sdk::new_mock();

    // Example 1: Fetch an identity with proof data
    println!("=== Fetching Identity with Proof Data ===\n");

    let identity_id = Identifier::from_bytes(&[
        0xf9, 0xc8, 0x5a, 0x89, 0x45, 0x3e, 0x67, 0x96, 0x87, 0xc7, 0xb1, 0xc4, 0x7a, 0xc9, 0x8a,
        0x7e, 0x6e, 0x68, 0xd0, 0x27, 0xd3, 0xb9, 0x64, 0x1a, 0xf6, 0x4f, 0x12, 0x56, 0x64, 0xf0,
        0xca, 0xf5,
    ])?;

    match Identity::fetch_with_proof(&sdk, identity_id).await {
        Ok((Some(identity), proof_data)) => {
            println!("Found identity: {}", identity.id());
            println!("Identity balance: {}", identity.balance());
            println!("\nProof Data from Platform:");
            println!(
                "  - GroveDB proof size: {} bytes",
                proof_data.grovedb_proof.len()
            );
            println!("  - Root hash: {}", hex::encode(proof_data.root_hash));
            println!("  - Quorum hash: {}", hex::encode(&proof_data.quorum_hash));
            println!("  - Block height: {}", proof_data.metadata.height);
            println!("  - Epoch: {}", proof_data.metadata.epoch);
            println!("  - Round: {}", proof_data.round);

            // Demonstrate the new identity key merkle proof functionality
            demonstrate_identity_key_zk_proof(&identity)?;
        }
        Ok((None, _)) => {
            println!("Identity not found");
        }
        Err(e) => {
            eprintln!("Error fetching identity: {}", e);
        }
    }

    println!("\n=== Fetching Documents with Proof Data ===\n");

    // Example 2: Using the new document proof functionality
    #[cfg(feature = "network-testing")]
    {
        use dash_sdk::platform::document_proof::fetch_document_with_proof;
        
        // In production, you would use real contract and document IDs
        let contract_id = Identifier::from_string(
            "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec",
            Default::default()
        )?;
        let document_id = Identifier::from_string(
            "4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF",
            Default::default()
        )?;
        
        match fetch_document_with_proof(&sdk, &contract_id, &document_id).await {
            Ok(doc_proof) => {
                println!("Document fetched with merkle proof:");
                println!("  Owner: {}", doc_proof.owner_id);
                println!("  State root: {}", hex::encode(doc_proof.state_root));
                println!("  Merkle path length: {} nodes", doc_proof.merkle_path.len());
                println!("  Block height: {}", doc_proof.block_height);
                
                // Show merkle path for ZK circuit
                for (level, (hash, is_left)) in doc_proof.merkle_path.iter().enumerate() {
                    println!("    Level {}: {} sibling, hash={}...",
                        level,
                        if *is_left { "left" } else { "right" },
                        hex::encode(&hash[..4])
                    );
                }
            }
            Err(e) => {
                println!("Could not fetch document: {}", e);
            }
        }
    }
    
    #[cfg(not(feature = "network-testing"))]
    {
        println!("Document fetching with the new API:");
        println!("```rust");
        println!("use dash_sdk::platform::document_proof::fetch_document_with_proof;");
        println!("");
        println!("let doc_proof = fetch_document_with_proof(&sdk, &contract_id, &document_id).await?;");
        println!("");
        println!("// doc_proof contains:");
        println!("// - document: The fetched document");
        println!("// - owner_id: Document owner identifier");
        println!("// - merkle_path: Path from document to state root");
        println!("// - state_root: Platform state root hash");
        println!("// - block_height: Block containing this state");
        println!("```");
    }

    println!("\n=== Use Cases for ZK Proofs ===\n");

    println!("1. Privacy-preserving identity verification:");
    println!("   - Prove you own an identity without revealing which one");
    println!("   - Prove your balance is above a threshold without revealing the exact amount");

    println!("\n2. Privacy-preserving document queries:");
    println!("   - Prove a document exists without revealing its contents");
    println!("   - Prove ownership of a domain name without revealing which one");

    println!("\n3. Cross-chain bridges:");
    println!("   - Prove Platform state to other blockchains");
    println!("   - Enable trustless cross-chain communication");

    println!("\n4. Light client applications:");
    println!("   - Verify Platform data without downloading full state");
    println!("   - Build mobile apps with cryptographic guarantees");

    Ok(())
}

// Note: To run this example, use:
// cargo run --example proof_data_zkp --features="network-testing"
