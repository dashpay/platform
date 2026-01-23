use crate::impl_wasm_type_info;
use dpp::prelude::{RecipientKeyIndex, SenderKeyIndex};
use dpp::tokens::SharedEncryptedNote;
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Debug, Clone, PartialEq)]
#[wasm_bindgen(js_name = "SharedEncryptedNote")]
pub struct SharedEncryptedNoteWasm(SharedEncryptedNote);

impl From<SharedEncryptedNote> for SharedEncryptedNoteWasm {
    fn from(note: SharedEncryptedNote) -> Self {
        Self(note)
    }
}

impl From<SharedEncryptedNoteWasm> for SharedEncryptedNote {
    fn from(note: SharedEncryptedNoteWasm) -> Self {
        note.0
    }
}

#[wasm_bindgen(js_class = SharedEncryptedNote)]
impl SharedEncryptedNoteWasm {
    #[wasm_bindgen(constructor)]
    pub fn constructor(
        #[wasm_bindgen(js_name = "senderKeyIndex")] sender_key_index: SenderKeyIndex,
        #[wasm_bindgen(js_name = "recipientKeyIndex")] recipient_key_index: RecipientKeyIndex,
        value: Vec<u8>,
    ) -> Self {
        SharedEncryptedNoteWasm((sender_key_index, recipient_key_index, value))
    }

    #[wasm_bindgen(getter = "senderKeyIndex")]
    pub fn sender_key_index(&self) -> SenderKeyIndex {
        self.0.0
    }

    #[wasm_bindgen(getter = "recipientKeyIndex")]
    pub fn recipient_key_index(&self) -> RecipientKeyIndex {
        self.0.1
    }

    #[wasm_bindgen(getter = "value")]
    pub fn value(&self) -> Vec<u8> {
        self.0.2.clone()
    }

    #[wasm_bindgen(setter = "senderKeyIndex")]
    pub fn set_sender_key_index(&mut self, index: SenderKeyIndex) {
        self.0.0 = index;
    }

    #[wasm_bindgen(setter = "recipientKeyIndex")]
    pub fn set_recipient_key_index(&mut self, index: RecipientKeyIndex) {
        self.0.1 = index;
    }

    #[wasm_bindgen(setter = "value")]
    pub fn set_value(&mut self, value: Vec<u8>) {
        self.0.2 = value;
    }
}

impl_wasm_type_info!(SharedEncryptedNoteWasm, SharedEncryptedNote);
