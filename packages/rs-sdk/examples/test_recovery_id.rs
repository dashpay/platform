//! Test what methods are actually available on RecoveryId

use dpp::dashcore::secp256k1::ecdsa::{RecoverableSignature, RecoveryId};
use dpp::dashcore::secp256k1::{Message, Secp256k1, SecretKey};

fn main() {
    println!("Testing RecoveryId methods...");

    // Create a RecoveryId using from()
    let id = RecoveryId::from(0i32);
    println!("Created RecoveryId with from(0i32): {:?}", id);

    // Test with actual signature to see what we get
    let secp = Secp256k1::new();
    let secret_key = SecretKey::from_slice(&[
        0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f,
        0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e,
        0x1f, 0x20,
    ])
    .expect("Valid secret key");

    let message = Message::from_digest_slice(&[0x42; 32]).expect("Valid message");

    let recoverable_sig = secp.sign_ecdsa_recoverable(&message, &secret_key);
    let (recovery_id, sig_bytes) = recoverable_sig.serialize_compact();

    println!("\nSignature created!");
    println!("Recovery ID: {:?}", recovery_id);
    println!("Signature bytes length: {}", sig_bytes.len());

    // See if we can compare RecoveryIds
    for i in 0..4 {
        let test_id = RecoveryId::from(i);
        // Check if they're equal
        if format!("{:?}", test_id) == format!("{:?}", recovery_id) {
            println!("Recovery ID matches value: {}", i);
        }
    }

    // Try to recreate the signature
    let recreated = RecoverableSignature::from_compact(&sig_bytes, recovery_id);
    match recreated {
        Ok(_) => println!("✓ Successfully recreated signature"),
        Err(e) => println!("✗ Failed to recreate: {}", e),
    }
}
