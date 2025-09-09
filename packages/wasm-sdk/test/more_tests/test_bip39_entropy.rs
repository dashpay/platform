use bip39::{Mnemonic, Language};

fn main() {
    println!("Testing BIP39 entropy sizes...");

    let test_sizes = vec![
        (16, "128-bit (12 words)"),
        (20, "160-bit (15 words)"),
        (24, "192-bit (18 words)"),
        (28, "224-bit (21 words)"),
        (32, "256-bit (24 words)"),
    ];

    for (size, desc) in test_sizes {
        println!("\nTesting {}: ", desc);

        // Create entropy of the specified size
        let entropy = vec![0u8; size];

        // Try to create mnemonic from entropy
        match Mnemonic::from_entropy_in(Language::English, &entropy) {
            Ok(mnemonic) => {
                let words: Vec<&str> = mnemonic.word_iter().collect();
                println!("  ✓ Success! Generated {} words", words.len());
            }
            Err(e) => {
                println!("  ✗ Failed: {}", e);
            }
        }
    }

    // Also test if generate_in exists
    println!("\nChecking if generate_in method exists...");
    // This will fail to compile if generate_in doesn't exist
    // let _mnemonic = Mnemonic::generate_in(Language::English, 16);
}