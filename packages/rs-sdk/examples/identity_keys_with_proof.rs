//! Example showing how to fetch identity keys with proof data for ZK applications
//!
//! This example demonstrates how to use the rs-sdk to fetch identity public keys
//! along with their cryptographic proofs, which can be used in zero-knowledge
//! proof systems like GroveSTARK.

use dash_sdk::platform::{FetchMany, Identifier, IdentityKeysQuery, IdentityPublicKey, ProofData};
use dash_sdk::Sdk;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::KeyID;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the SDK (use Sdk::new() for mainnet/testnet)
    let sdk = Sdk::new_mock();

    // Example identity ID (replace with actual identity)
    let identity_id = Identifier::new([1; 32]);

    // Fetch all keys for the identity with proof data
    let (keys, metadata, proof) = IdentityPublicKey::fetch_many_with_metadata_and_proof(
        &sdk,
        identity_id,
        None, // Optional request settings
    )
    .await?;

    // Create ProofData structure for easier access
    let proof_data = ProofData::new(proof, metadata);

    println!("Identity keys fetched successfully!");
    println!("Number of keys: {}", keys.len());
    println!("Proof size: {} bytes", proof_data.grovedb_proof.len());
    println!("Root hash: {}", hex::encode(&proof_data.root_hash));
    println!("Block height: {}", proof_data.metadata.height);

    // Process each key with its proof
    for (key_id, maybe_key) in keys.iter() {
        if let Some(key) = maybe_key {
            println!("\nKey ID: {}", key_id);
            println!("  Purpose: {:?}", key.purpose());
            println!("  Security Level: {:?}", key.security_level());
            println!("  Key Type: {:?}", key.key_type());

            // The proof contains:
            // 1. Merkle path from this key → IdentityTreeKeys root
            // 2. Path from IdentityTreeKeys root → Identity root
            // 3. Path from Identity root → state root
            //
            // This is exactly what you need for ZK proofs!
        }
    }

    // For ZK proof implementation:
    println!("\n=== Data for Zero-Knowledge Proof ===");
    println!("1. Keys data: Available in 'keys' BTreeMap");
    println!("2. GroveDB proof bytes: Available in proof_data.grovedb_proof");
    println!("3. State root (public input): {:?}", proof_data.root_hash);
    println!(
        "4. Block metadata: height={}, timestamp={}",
        proof_data.metadata.height, proof_data.metadata.time_ms
    );

    // Example: Extract merkle paths from the GroveDB proof for ZK circuit
    // Note: You would use the extract_merkle_path_from_grove_proof function
    // from the document_proof module or implement similar parsing

    println!("\nThe GroveDB proof contains merkle paths that prove:");
    println!("- Each key belongs to the IdentityTreeKeys subtree");
    println!("- The IdentityTreeKeys subtree belongs to this identity");
    println!("- The identity belongs to the global state root");
    println!("\nThis allows you to prove key ownership without revealing the identity!");

    // NEW: Fetch SPECIFIC keys with proof (more efficient for ZK)
    println!("\n=== Fetching Specific Keys with Proof ===");

    // Let's say we only want to fetch keys with IDs 0 and 1
    let specific_key_ids: Vec<KeyID> = vec![0, 1];

    // Create a query for specific keys
    let specific_keys_query = IdentityKeysQuery::new(identity_id, specific_key_ids.clone());

    // Fetch only the specified keys with proof
    let (specific_keys, specific_metadata, specific_proof) =
        IdentityPublicKey::fetch_many_with_metadata_and_proof(&sdk, specific_keys_query, None)
            .await?;

    let specific_proof_data = ProofData::new(specific_proof, specific_metadata);

    println!("Fetched {} specific keys", specific_keys.len());
    println!(
        "Specific keys proof size: {} bytes",
        specific_proof_data.grovedb_proof.len()
    );

    // Process the specific keys
    for (key_id, maybe_key) in specific_keys.iter() {
        if let Some(key) = maybe_key {
            println!(
                "  Key ID {}: Type={:?}, Purpose={:?}",
                key_id,
                key.key_type(),
                key.purpose()
            );
        }
    }

    println!("\nBenefits of fetching specific keys:");
    println!("- Smaller proof size (only includes requested keys)");
    println!("- More efficient for ZK circuits");
    println!("- Reveals less information about the identity");
    println!("- Perfect for proving ownership of a single key!");

    Ok(())
}
