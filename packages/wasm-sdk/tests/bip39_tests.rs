//! Unit tests for BIP39 mnemonic functionality

use wasm_bindgen_test::*;
use wasm_sdk::bip39::*;
use js_sys::Array;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn test_mnemonic_generation() {
    // Test 12-word mnemonic
    let mnemonic = Mnemonic::generate(MnemonicStrength::Words12, WordListLanguage::English)
        .expect("Should generate 12-word mnemonic");
    assert_eq!(mnemonic.word_count(), 12);
    assert!(!mnemonic.phrase().is_empty());
    
    // Test 24-word mnemonic
    let mnemonic = Mnemonic::generate(MnemonicStrength::Words24, WordListLanguage::English)
        .expect("Should generate 24-word mnemonic");
    assert_eq!(mnemonic.word_count(), 24);
}

#[wasm_bindgen_test]
fn test_mnemonic_from_phrase() {
    let phrase = "abandon ability able about above absent absorb abstract absurd abuse access accident";
    let mnemonic = Mnemonic::from_phrase(phrase, WordListLanguage::English)
        .expect("Should create mnemonic from phrase");
    
    assert_eq!(mnemonic.word_count(), 12);
    assert_eq!(mnemonic.phrase(), phrase);
    
    let words = mnemonic.words();
    assert_eq!(words.length(), 12);
}

#[wasm_bindgen_test]
fn test_invalid_mnemonic_length() {
    let phrase = "abandon ability able"; // Only 3 words
    let result = Mnemonic::from_phrase(phrase, WordListLanguage::English);
    assert!(result.is_err());
    
    let err = result.unwrap_err();
    let err_msg = format!("{:?}", err);
    assert!(err_msg.contains("Invalid mnemonic length"));
}

#[wasm_bindgen_test]
fn test_mnemonic_validation() {
    let mnemonic = Mnemonic::generate(MnemonicStrength::Words12, WordListLanguage::English)
        .expect("Should generate mnemonic");
    
    let is_valid = mnemonic.validate().expect("Should validate mnemonic");
    assert!(is_valid);
}

#[wasm_bindgen_test]
fn test_mnemonic_to_seed() {
    let phrase = "abandon ability able about above absent absorb abstract absurd abuse access accident";
    let mnemonic = Mnemonic::from_phrase(phrase, WordListLanguage::English)
        .expect("Should create mnemonic");
    
    // Test without passphrase
    let seed = mnemonic.to_seed(None).expect("Should generate seed");
    assert_eq!(seed.len(), 64);
    
    // Test with passphrase
    let seed_with_pass = mnemonic.to_seed(Some("test".to_string()))
        .expect("Should generate seed with passphrase");
    assert_eq!(seed_with_pass.len(), 64);
    
    // Seeds should be different
    assert_ne!(seed, seed_with_pass);
}

#[wasm_bindgen_test]
fn test_mnemonic_to_hd_private_key() {
    let mnemonic = Mnemonic::generate(MnemonicStrength::Words12, WordListLanguage::English)
        .expect("Should generate mnemonic");
    
    // Test mainnet
    let mainnet_key = mnemonic.to_hd_private_key(None, "mainnet")
        .expect("Should generate mainnet HD key");
    assert!(mainnet_key.starts_with("xprv"));
    
    // Test testnet
    let testnet_key = mnemonic.to_hd_private_key(None, "testnet")
        .expect("Should generate testnet HD key");
    assert!(testnet_key.starts_with("tprv"));
    
    // Test invalid network
    let result = mnemonic.to_hd_private_key(None, "invalid");
    assert!(result.is_err());
}

#[wasm_bindgen_test]
fn test_validate_mnemonic_function() {
    // Valid mnemonic
    let valid_phrase = "abandon ability able about above absent absorb abstract absurd abuse access accident";
    assert!(validate_mnemonic(valid_phrase, None));
    
    // Invalid length
    let invalid_phrase = "abandon ability able";
    assert!(!validate_mnemonic(invalid_phrase, None));
    
    // Empty phrase
    assert!(!validate_mnemonic("", None));
}

#[wasm_bindgen_test]
fn test_generate_entropy() {
    // Test different entropy sizes
    let entropy_128 = generate_entropy(MnemonicStrength::Words12)
        .expect("Should generate 128-bit entropy");
    assert_eq!(entropy_128.len(), 16); // 128 bits = 16 bytes
    
    let entropy_256 = generate_entropy(MnemonicStrength::Words24)
        .expect("Should generate 256-bit entropy");
    assert_eq!(entropy_256.len(), 32); // 256 bits = 32 bytes
}

#[wasm_bindgen_test]
fn test_mnemonic_from_entropy() {
    let entropy = vec![
        0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
        0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0x10,
    ]; // 16 bytes = 128 bits
    
    let mnemonic = mnemonic_from_entropy(entropy.clone(), WordListLanguage::English)
        .expect("Should create mnemonic from entropy");
    assert_eq!(mnemonic.word_count(), 12);
    
    // Test invalid entropy length
    let invalid_entropy = vec![0x01, 0x02, 0x03]; // 3 bytes
    let result = mnemonic_from_entropy(invalid_entropy, WordListLanguage::English);
    assert!(result.is_err());
}

#[wasm_bindgen_test]
fn test_get_word_list() {
    let word_list = get_word_list(WordListLanguage::English);
    assert!(word_list.length() > 0);
    
    // Check that entries are strings
    if word_list.length() > 0 {
        let first_word = word_list.get(0);
        assert!(first_word.is_string());
    }
}

#[wasm_bindgen_test]
fn test_suggest_words() {
    // Test basic suggestions
    let suggestions = suggest_words("ab", WordListLanguage::English, None);
    assert!(suggestions.length() > 0);
    
    // All suggestions should start with "ab"
    for i in 0..suggestions.length() {
        let word = suggestions.get(i);
        if let Some(word_str) = word.as_string() {
            assert!(word_str.starts_with("ab"));
        }
    }
    
    // Test with max suggestions
    let limited_suggestions = suggest_words("a", WordListLanguage::English, Some(3));
    assert!(limited_suggestions.length() <= 3);
}

#[wasm_bindgen_test]
fn test_mnemonic_to_seed_hex() {
    let phrase = "abandon ability able about above absent absorb abstract absurd abuse access accident";
    
    let seed_hex = mnemonic_to_seed_hex(phrase, None)
        .expect("Should convert mnemonic to seed hex");
    
    // Hex string should be 128 characters (64 bytes * 2)
    assert_eq!(seed_hex.len(), 128);
    
    // Should only contain hex characters
    assert!(seed_hex.chars().all(|c| c.is_ascii_hexdigit()));
}

#[wasm_bindgen_test]
fn test_derive_child_key() {
    let phrase = "abandon ability able about above absent absorb abstract absurd abuse access accident";
    
    // Valid derivation path
    let result = derive_child_key(phrase, None, "m/44'/5'/0'/0/0", "mainnet")
        .expect("Should derive child key");
    
    // Check result has expected fields
    let obj = result.dyn_ref::<js_sys::Object>().expect("Should be an object");
    assert!(js_sys::Reflect::has(obj, &"privateKey".into()).unwrap());
    assert!(js_sys::Reflect::has(obj, &"publicKey".into()).unwrap());
    assert!(js_sys::Reflect::has(obj, &"address".into()).unwrap());
    assert!(js_sys::Reflect::has(obj, &"path".into()).unwrap());
    
    // Invalid derivation path
    let invalid_result = derive_child_key(phrase, None, "invalid/path", "mainnet");
    assert!(invalid_result.is_err());
}

#[wasm_bindgen_test]
fn test_mnemonic_words_array() {
    let phrase = "abandon ability able about above absent";
    let mnemonic = Mnemonic::from_phrase(phrase, WordListLanguage::English)
        .expect("Should create mnemonic");
    
    let words = mnemonic.words();
    assert_eq!(words.length(), 6);
    
    // Verify each word
    let expected_words = ["abandon", "ability", "able", "about", "above", "absent"];
    for (i, expected) in expected_words.iter().enumerate() {
        let word = words.get(i as u32);
        assert_eq!(word.as_string().unwrap(), *expected);
    }
}

#[wasm_bindgen_test]
fn test_different_languages() {
    // Test generating mnemonics in different languages
    let languages = vec![
        WordListLanguage::English,
        WordListLanguage::Japanese,
        WordListLanguage::Spanish,
        WordListLanguage::French,
    ];
    
    for language in languages {
        let mnemonic = Mnemonic::generate(MnemonicStrength::Words12, language)
            .expect("Should generate mnemonic in language");
        assert_eq!(mnemonic.word_count(), 12);
        assert!(mnemonic.validate().unwrap());
    }
}