//! Historical fixture generator, invoked by scripts/generate_historical_test_vectors.sh.

use platform_encryption::*;
use secp256k1::{PublicKey, Secp256k1, SecretKey};
#[test]
#[ignore = "run through scripts/generate_historical_test_vectors.sh on the pinned revision"]
fn generate() {
    let secp = Secp256k1::new();
    let secret = SecretKey::from_slice(&[0xc0; 32]).unwrap();
    let recipient = SecretKey::from_slice(&[0x0d; 32]).unwrap();
    let public = PublicKey::from_secret_key(&secp, &recipient);
    let shared = derive_shared_key_ecdh(&secret, &public);
    let xpub = compact_xpub_bytes(
        [0x11, 0x22, 0x33, 0x44],
        [0xaa; 32],
        PublicKey::from_secret_key(&secp, &SecretKey::from_slice(&[7; 32]).unwrap()).serialize(),
    );
    let iv = [0x5a; 16];
    let out = super::output_dir("signing");
    for (name, value) in [
        ("shared-key", shared.to_vec()),
        ("compact-xpub", xpub.to_vec()),
        (
            "encrypted-xpub",
            encrypt_extended_public_key(&shared, &iv, &xpub),
        ),
        (
            "encrypted-label",
            encrypt_account_label(&shared, &iv, "My DashPay Account"),
        ),
        (
            "encrypted-empty-label",
            encrypt_account_label(&shared, &iv, ""),
        ),
        (
            "encrypted-id",
            encrypt_enc_to_user_id(&shared, &[0x23; 32]).to_vec(),
        ),
        (
            "encrypted-private-data",
            encrypt_private_data(&shared, &iv, b"historical contact information"),
        ),
    ] {
        std::fs::write(out.join(format!("{name}.bin")), value).unwrap();
    }
}
