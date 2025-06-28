//! # BIP39 Mnemonic Module
//!
//! This module provides BIP39 mnemonic functionality for seed phrase generation,
//! validation, and key derivation using the bip39 crate.

use bip39::{Language, Mnemonic as Bip39Mnemonic};
use js_sys::{Array, Uint8Array};
use wasm_bindgen::prelude::*;

/// BIP39 word list languages
#[wasm_bindgen]
#[derive(Clone, Copy, Debug)]
pub enum WordListLanguage {
    English,
    Japanese,
    Korean,
    Spanish,
    ChineseSimplified,
    ChineseTraditional,
    French,
    Italian,
    Czech,
    Portuguese,
}

impl From<WordListLanguage> for Language {
    fn from(lang: WordListLanguage) -> Self {
        match lang {
            WordListLanguage::English => Language::English,
            WordListLanguage::Japanese => Language::Japanese,
            WordListLanguage::Korean => Language::Korean,
            WordListLanguage::Spanish => Language::Spanish,
            WordListLanguage::ChineseSimplified => Language::SimplifiedChinese,
            WordListLanguage::ChineseTraditional => Language::TraditionalChinese,
            WordListLanguage::French => Language::French,
            WordListLanguage::Italian => Language::Italian,
            WordListLanguage::Czech => Language::Czech,
            WordListLanguage::Portuguese => Language::Portuguese,
        }
    }
}

/// BIP39 mnemonic strength
#[wasm_bindgen]
#[derive(Clone, Copy, Debug)]
pub enum MnemonicStrength {
    /// 12 words (128 bits)
    Words12 = 128,
    /// 15 words (160 bits)
    Words15 = 160,
    /// 18 words (192 bits)
    Words18 = 192,
    /// 21 words (224 bits)
    Words21 = 224,
    /// 24 words (256 bits)
    Words24 = 256,
}

/// BIP39 mnemonic wrapper
#[wasm_bindgen]
pub struct Mnemonic {
    inner: Bip39Mnemonic,
    _language: Language,
}

#[wasm_bindgen]
impl Mnemonic {
    /// Generate a new mnemonic with the specified strength and language
    #[wasm_bindgen(js_name = generate)]
    pub fn generate(
        strength: MnemonicStrength,
        language: Option<WordListLanguage>,
    ) -> Result<Mnemonic, JsError> {
        let lang = language.map(Language::from).unwrap_or(Language::English);
        let strength_bits = strength as usize;

        // Generate entropy
        let entropy_bytes = strength_bits / 8;
        let mut entropy = vec![0u8; entropy_bytes];
        getrandom::getrandom(&mut entropy)
            .map_err(|e| JsError::new(&format!("Failed to generate entropy: {}", e)))?;

        // Create mnemonic from entropy
        let inner = Bip39Mnemonic::from_entropy(&entropy)
            .map_err(|e| JsError::new(&format!("Failed to create mnemonic: {}", e)))?;

        Ok(Mnemonic {
            inner,
            _language: lang,
        })
    }

    /// Create a mnemonic from an existing phrase
    #[wasm_bindgen(js_name = fromPhrase)]
    pub fn from_phrase(
        phrase: &str,
        language: Option<WordListLanguage>,
    ) -> Result<Mnemonic, JsError> {
        let lang = language.map(Language::from).unwrap_or(Language::English);

        let inner = Bip39Mnemonic::parse_in(lang, phrase)
            .map_err(|e| JsError::new(&format!("Invalid mnemonic phrase: {}", e)))?;

        Ok(Mnemonic {
            inner,
            _language: lang,
        })
    }

    /// Create a mnemonic from entropy
    #[wasm_bindgen(js_name = fromEntropy)]
    pub fn from_entropy(
        entropy: &[u8],
        language: Option<WordListLanguage>,
    ) -> Result<Mnemonic, JsError> {
        let lang = language.map(Language::from).unwrap_or(Language::English);

        let inner = Bip39Mnemonic::from_entropy(entropy)
            .map_err(|e| JsError::new(&format!("Invalid entropy: {}", e)))?;

        Ok(Mnemonic {
            inner,
            _language: lang,
        })
    }

    /// Get the mnemonic phrase as a string
    #[wasm_bindgen(getter)]
    pub fn phrase(&self) -> String {
        self.inner.to_string()
    }

    /// Get the mnemonic words as an array
    #[wasm_bindgen(getter)]
    pub fn words(&self) -> Array {
        let words = self.inner.words().map(|w| JsValue::from_str(w));
        words.collect()
    }

    /// Get the number of words
    #[wasm_bindgen(getter, js_name = wordCount)]
    pub fn word_count(&self) -> u32 {
        self.inner.word_count() as u32
    }

    /// Get the entropy as bytes
    #[wasm_bindgen(getter)]
    pub fn entropy(&self) -> Uint8Array {
        Uint8Array::from(self.inner.to_entropy().as_slice())
    }

    /// Generate seed from the mnemonic with optional passphrase
    #[wasm_bindgen(js_name = toSeed)]
    pub fn to_seed(&self, passphrase: Option<String>) -> Uint8Array {
        let passphrase = passphrase.as_deref().unwrap_or("");
        let seed = self.inner.to_seed(passphrase);
        Uint8Array::from(&seed[..])
    }

    /// Validate a mnemonic phrase
    #[wasm_bindgen(js_name = validate)]
    pub fn validate(phrase: &str, language: Option<WordListLanguage>) -> bool {
        let lang = language.map(Language::from).unwrap_or(Language::English);
        Bip39Mnemonic::parse_in(lang, phrase).is_ok()
    }
}

/// Generate a new mnemonic phrase
#[wasm_bindgen(js_name = generateMnemonic)]
pub fn generate_mnemonic(
    strength: Option<MnemonicStrength>,
    language: Option<WordListLanguage>,
) -> Result<String, JsError> {
    let mnemonic = Mnemonic::generate(strength.unwrap_or(MnemonicStrength::Words12), language)?;
    Ok(mnemonic.phrase())
}

/// Validate a mnemonic phrase
#[wasm_bindgen(js_name = validateMnemonic)]
pub fn validate_mnemonic(phrase: &str, language: Option<WordListLanguage>) -> bool {
    Mnemonic::validate(phrase, language)
}

/// Convert mnemonic to seed
#[wasm_bindgen(js_name = mnemonicToSeed)]
pub fn mnemonic_to_seed(
    phrase: &str,
    passphrase: Option<String>,
    language: Option<WordListLanguage>,
) -> Result<Uint8Array, JsError> {
    let mnemonic = Mnemonic::from_phrase(phrase, language)?;
    Ok(mnemonic.to_seed(passphrase))
}

/// Get word list for a language
#[wasm_bindgen(js_name = getWordList)]
pub fn get_word_list(language: Option<WordListLanguage>) -> Array {
    let lang = language.map(Language::from).unwrap_or(Language::English);
    let word_list = lang.word_list();

    let array = Array::new();
    for word in word_list {
        array.push(&JsValue::from_str(word));
    }
    array
}

/// Generate entropy for mnemonic
#[wasm_bindgen(js_name = generateEntropy)]
pub fn generate_entropy(strength: Option<MnemonicStrength>) -> Result<Uint8Array, JsError> {
    let strength_bits = strength.unwrap_or(MnemonicStrength::Words12) as usize;
    let entropy_bytes = strength_bits / 8;

    let mut entropy = vec![0u8; entropy_bytes];
    getrandom::getrandom(&mut entropy)
        .map_err(|e| JsError::new(&format!("Failed to generate entropy: {}", e)))?;

    Ok(Uint8Array::from(&entropy[..]))
}
