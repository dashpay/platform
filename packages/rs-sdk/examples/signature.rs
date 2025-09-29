//! Example showing how to use signature utilities

use dash_sdk::platform::signature_utils::{
    recover_public_key, sign_message_recoverable, RecoveryIdExt,
};
use dpp::dashcore::hashes::{sha256, Hash};
use dpp::dashcore::secp256k1::ecdsa::RecoveryId;
use dpp::dashcore::secp256k1::{PublicKey, Secp256k1, SecretKey};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Signature Utils ===\n");

    // Example secret key (in real usage, this would come from a wallet)
    let secret_key = SecretKey::from_slice(&[
        0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f,
        0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e,
        0x1f, 0x20,
    ])?;

    // Create the challenge to sign
    let mut challenge = Vec::new();
    challenge.extend_from_slice(b"document_id_here");
    challenge.extend_from_slice(b"contract_id_here");
    challenge.extend_from_slice(&1234567890u64.to_le_bytes()); // timestamp
    challenge.extend_from_slice(&[0x42; 32]); // nonce

    // Hash the challenge to get 32 bytes
    let message_hash = {
        let hash = sha256::Hash::hash(&challenge);
        let mut hash_bytes = [0u8; 32];
        hash_bytes.copy_from_slice(&hash.to_byte_array());
        hash_bytes
    };

    println!("1. Sign the challenge with identity key:");
    let (r, s, v) = sign_message_recoverable(&message_hash, &secret_key)?;
    println!("   signature_r: {}", hex::encode(r));
    println!("   signature_s: {}", hex::encode(s));
    println!("   signature_v: {}", v);

    // Get the public key (in real usage, this would be the identity key)
    let secp = Secp256k1::new();
    let public_key = PublicKey::from_secret_key(&secp, &secret_key);
    let public_key_bytes = public_key.serialize();
    println!("   public_key: {}", hex::encode(public_key_bytes));

    println!("\n2. Verify we can recover the public key:");
    let recovered = recover_public_key(&message_hash, &r, &s, v)?;
    println!("   Recovered: {}", hex::encode(&recovered));
    println!("   Matches: {}", recovered == public_key_bytes.to_vec());

    println!("\n3. Direct RecoveryId conversion (if needed):");
    // Show how to use RecoveryId conversion directly
    let recovery_id = RecoveryId::from_u8(v)?;
    println!("   Created RecoveryId from u8: {:?}", recovery_id);
    let v_back = recovery_id.to_u8();
    println!("   Converted back to u8: {}", v_back);

    // Alternative: Use the standard traits directly without our extension
    println!("\n4. Using standard traits (without RecoveryIdExt):");
    let recovery_id_2 = RecoveryId::try_from(2i32)?;
    let v_2: i32 = recovery_id_2.into();
    println!("   RecoveryId::try_from(2) -> into() -> {}", v_2);

    println!("\n✓ All components ready for proof!");

    Ok(())
}
