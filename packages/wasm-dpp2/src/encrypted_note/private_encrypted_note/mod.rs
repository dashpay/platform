use dpp::prelude::{DerivationEncryptionKeyIndex, RootEncryptionKeyIndex};
use dpp::tokens::PrivateEncryptedNote;
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Debug, Clone, PartialEq)]
#[wasm_bindgen(js_name = "PrivateEncryptedNote")]
pub struct PrivateEncryptedNoteWASM(PrivateEncryptedNote);

impl From<PrivateEncryptedNote> for PrivateEncryptedNoteWASM {
    fn from(value: PrivateEncryptedNote) -> Self {
        PrivateEncryptedNoteWASM(value)
    }
}

impl From<PrivateEncryptedNoteWASM> for PrivateEncryptedNote {
    fn from(value: PrivateEncryptedNoteWASM) -> Self {
        value.0
    }
}

#[wasm_bindgen(js_class = PrivateEncryptedNote)]
impl PrivateEncryptedNoteWASM {
    #[wasm_bindgen(getter = __type)]
    pub fn type_name(&self) -> String {
        "PrivateEncryptedNote".to_string()
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name() -> String {
        "PrivateEncryptedNote".to_string()
    }

    #[wasm_bindgen(constructor)]
    pub fn new(
        root_encryption_key_index: RootEncryptionKeyIndex,
        derivation_encryption_key_index: DerivationEncryptionKeyIndex,
        value: Vec<u8>,
    ) -> PrivateEncryptedNoteWASM {
        PrivateEncryptedNoteWASM((
            root_encryption_key_index,
            derivation_encryption_key_index,
            value,
        ))
    }

    #[wasm_bindgen(getter = "rootEncryptionKeyIndex")]
    pub fn root_encryption_key_index(&self) -> RootEncryptionKeyIndex {
        self.0.0
    }

    #[wasm_bindgen(getter = "derivationEncryptionKeyIndex")]
    pub fn derivation_encryption_key_index(&self) -> DerivationEncryptionKeyIndex {
        self.0.1
    }

    #[wasm_bindgen(getter = "value")]
    pub fn value(&self) -> Vec<u8> {
        self.0.2.clone()
    }

    #[wasm_bindgen(setter = "rootEncryptionKeyIndex")]
    pub fn set_root_encryption_key_index(&mut self, index: RootEncryptionKeyIndex) {
        self.0.0 = index;
    }

    #[wasm_bindgen(setter = "derivationEncryptionKeyIndex")]
    pub fn set_derivation_encryption_key_index(&mut self, index: DerivationEncryptionKeyIndex) {
        self.0.1 = index;
    }

    #[wasm_bindgen(setter = "value")]
    pub fn set_value(&mut self, value: Vec<u8>) {
        self.0.2 = value;
    }
}
