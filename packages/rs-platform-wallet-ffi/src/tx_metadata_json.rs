use std::ffi::CString;
use std::os::raw::c_char;

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use platform_wallet::DecryptedEncryptedDocument;

use crate::error::{PlatformWalletFFIResult, PlatformWalletFFIResultCode};
use crate::types::zeroize_sensitive_bytes;

const IDENTIFIER_BASE58_CAPACITY: usize = 48;

fn serialization_error(message: &'static str) -> PlatformWalletFFIResult {
    PlatformWalletFFIResult::err(PlatformWalletFFIResultCode::ErrorSerialization, message)
}

fn arithmetic_error() -> PlatformWalletFFIResult {
    PlatformWalletFFIResult::err(
        PlatformWalletFFIResultCode::ErrorArithmeticOverflow,
        "decrypted document JSON length overflow",
    )
}

fn validate_ascii(bytes: &[u8]) -> Result<(), PlatformWalletFFIResult> {
    if !bytes.is_ascii() || bytes.contains(&0) {
        return Err(serialization_error(
            "decrypted document JSON contains non-ASCII or NUL bytes",
        ));
    }
    Ok(())
}

trait JsonWriter {
    fn write_ascii(&mut self, bytes: &[u8]) -> Result<(), PlatformWalletFFIResult>;
    fn write_payload(&mut self, payload: &[u8]) -> Result<(), PlatformWalletFFIResult>;
}

struct CountingWriter {
    written: usize,
}

impl CountingWriter {
    fn new() -> Self {
        Self { written: 0 }
    }

    fn written(&self) -> usize {
        self.written
    }
}

impl JsonWriter for CountingWriter {
    fn write_ascii(&mut self, bytes: &[u8]) -> Result<(), PlatformWalletFFIResult> {
        validate_ascii(bytes)?;
        self.written = self
            .written
            .checked_add(bytes.len())
            .ok_or_else(arithmetic_error)?;
        Ok(())
    }

    fn write_payload(&mut self, payload: &[u8]) -> Result<(), PlatformWalletFFIResult> {
        let encoded_len = base64::encoded_len(payload.len(), true).ok_or_else(arithmetic_error)?;
        self.written = self
            .written
            .checked_add(encoded_len)
            .ok_or_else(arithmetic_error)?;
        Ok(())
    }
}

struct FixedAsciiWriter<'a> {
    output: &'a mut [u8],
    written: usize,
}

impl<'a> FixedAsciiWriter<'a> {
    fn new(output: &'a mut [u8]) -> Self {
        Self { output, written: 0 }
    }

    fn written(&self) -> usize {
        self.written
    }

    fn remaining_mut(&mut self, len: usize) -> Result<&mut [u8], PlatformWalletFFIResult> {
        let end = self.written.checked_add(len).ok_or_else(arithmetic_error)?;
        if end > self.output.len() {
            return Err(serialization_error(
                "decrypted document JSON exceeded its fixed output buffer",
            ));
        }
        Ok(&mut self.output[self.written..end])
    }
}

impl JsonWriter for FixedAsciiWriter<'_> {
    fn write_ascii(&mut self, bytes: &[u8]) -> Result<(), PlatformWalletFFIResult> {
        validate_ascii(bytes)?;
        self.remaining_mut(bytes.len())?.copy_from_slice(bytes);
        self.written += bytes.len();
        Ok(())
    }

    fn write_payload(&mut self, payload: &[u8]) -> Result<(), PlatformWalletFFIResult> {
        let encoded_len = base64::encoded_len(payload.len(), true).ok_or_else(arithmetic_error)?;
        let written = STANDARD
            .encode_slice(payload, self.remaining_mut(encoded_len)?)
            .map_err(|_| {
                serialization_error("base64 payload did not fit its fixed output range")
            })?;
        if written != encoded_len {
            return Err(serialization_error(
                "base64 payload length differed from its counted length",
            ));
        }
        self.written += written;
        Ok(())
    }
}

fn write_identifier(
    writer: &mut impl JsonWriter,
    identifier: &[u8; 32],
) -> Result<(), PlatformWalletFFIResult> {
    let mut encoded = [0u8; IDENTIFIER_BASE58_CAPACITY];
    let len = bs58::encode(identifier)
        .onto(&mut encoded[..])
        .map_err(|_| serialization_error("identifier did not fit its base58 stack buffer"))?;
    writer.write_ascii(&encoded[..len])
}

fn write_u64(writer: &mut impl JsonWriter, mut value: u64) -> Result<(), PlatformWalletFFIResult> {
    let mut digits = [0u8; 20];
    let mut cursor = digits.len();
    loop {
        cursor -= 1;
        digits[cursor] = b'0' + (value % 10) as u8;
        value /= 10;
        if value == 0 {
            break;
        }
    }
    writer.write_ascii(&digits[cursor..])
}

fn write_documents(
    writer: &mut impl JsonWriter,
    documents: &[DecryptedEncryptedDocument],
) -> Result<(), PlatformWalletFFIResult> {
    writer.write_ascii(b"[")?;
    for (index, document) in documents.iter().enumerate() {
        if index > 0 {
            writer.write_ascii(b",")?;
        }
        writer.write_ascii(b"{\"id\":\"")?;
        write_identifier(writer, &document.document_id.to_buffer())?;
        writer.write_ascii(b"\",\"ownerId\":\"")?;
        write_identifier(writer, &document.owner_id.to_buffer())?;
        writer.write_ascii(b"\",\"keyIndex\":")?;
        write_u64(writer, u64::from(document.key_index))?;
        writer.write_ascii(b",\"encryptionKeyIndex\":")?;
        write_u64(writer, u64::from(document.encryption_key_index))?;
        writer.write_ascii(b",\"version\":")?;
        write_u64(writer, u64::from(document.version))?;
        writer.write_ascii(b",\"updatedAt\":")?;
        if let Some(updated_at_ms) = document.updated_at_ms {
            write_u64(writer, updated_at_ms)?;
        } else {
            writer.write_ascii(b"null")?;
        }
        writer.write_ascii(b",\"payload\":\"")?;
        writer.write_payload(&document.payload)?;
        writer.write_ascii(b"\"}")?;
    }
    writer.write_ascii(b"]")
}

pub(crate) struct SensitiveCString {
    inner: Option<Box<[u8]>>,
}

impl SensitiveCString {
    fn new(content_len: usize) -> Result<Self, PlatformWalletFFIResult> {
        let allocation_len = content_len.checked_add(1).ok_or_else(arithmetic_error)?;
        let mut bytes = vec![b' '; allocation_len];
        bytes[content_len] = 0;
        Ok(Self {
            inner: Some(bytes.into_boxed_slice()),
        })
    }

    fn content_mut(&mut self) -> &mut [u8] {
        let inner = self
            .inner
            .as_mut()
            .expect("sensitive bytes are owned until consuming transfer");
        let content_len = inner
            .len()
            .checked_sub(1)
            .expect("sensitive bytes include a NUL terminator");
        &mut inner[..content_len]
    }

    fn validate(&self) -> Result<(), PlatformWalletFFIResult> {
        let inner = self
            .inner
            .as_ref()
            .expect("sensitive bytes are owned until consuming transfer");
        let Some((&terminator, content)) = inner.split_last() else {
            return Err(serialization_error(
                "decrypted document JSON output buffer was empty",
            ));
        };
        if terminator != 0 {
            return Err(serialization_error(
                "decrypted document JSON lost its NUL terminator",
            ));
        }
        validate_ascii(content)
    }

    #[cfg(test)]
    fn as_c_str(&self) -> &std::ffi::CStr {
        let inner = self
            .inner
            .as_deref()
            .expect("test observes sensitive bytes before ownership transfer");
        std::ffi::CStr::from_bytes_with_nul(inner)
            .expect("validated sensitive bytes form a C string")
    }

    pub(crate) fn into_raw(mut self) -> *mut c_char {
        let bytes = self
            .inner
            .take()
            .expect("sensitive bytes are owned until consuming transfer")
            .into_vec();
        // SAFETY: serialization validates that the final byte remains the sole
        // NUL terminator. Converting an exact-length boxed slice into a Vec
        // gives it capacity equal to its length, so CString adopts the same
        // allocation without shrinking it.
        unsafe { CString::from_vec_with_nul_unchecked(bytes) }.into_raw()
    }
}

impl Drop for SensitiveCString {
    fn drop(&mut self) {
        if let Some(mut inner) = self.inner.take() {
            zeroize_sensitive_bytes(&mut inner);
        }
    }
}

pub(crate) fn serialize_decrypted_documents(
    documents: &[DecryptedEncryptedDocument],
) -> Result<SensitiveCString, PlatformWalletFFIResult> {
    let mut counter = CountingWriter::new();
    write_documents(&mut counter, documents)?;
    let expected_len = counter.written();

    let mut output = SensitiveCString::new(expected_len)?;
    let mut writer = FixedAsciiWriter::new(output.content_mut());
    write_documents(&mut writer, documents)?;
    if writer.written() != expected_len {
        return Err(serialization_error(
            "decrypted document JSON did not fill its fixed output buffer",
        ));
    }
    output.validate()?;

    Ok(output)
}

#[cfg(test)]
mod tests {
    use dpp::prelude::Identifier;
    use platform_wallet::DecryptedEncryptedDocument;

    use super::*;

    fn document(payload: &[u8]) -> DecryptedEncryptedDocument {
        DecryptedEncryptedDocument {
            document_id: Identifier::from([1; 32]),
            owner_id: Identifier::from([2; 32]),
            key_index: 3,
            encryption_key_index: 4,
            version: 1,
            updated_at_ms: Some(5),
            payload: payload.to_vec().into(),
        }
    }

    #[test]
    fn should_preserve_the_existing_sensitive_json_wire_shape() {
        let serialized =
            serialize_decrypted_documents(&[document(b"\x00\x01secret")]).expect("serialize");
        let id = bs58::encode([1; 32]).into_string();
        let owner_id = bs58::encode([2; 32]).into_string();
        let expected = format!(
            r#"[{{"id":"{id}","ownerId":"{owner_id}","keyIndex":3,"encryptionKeyIndex":4,"version":1,"updatedAt":5,"payload":"AAFzZWNyZXQ="}}]"#
        );

        assert_eq!(serialized.as_c_str().to_bytes(), expected.as_bytes());
    }

    #[test]
    fn should_serialize_empty_sensitive_json_as_an_ascii_array() {
        let serialized = serialize_decrypted_documents(&[]).expect("serialize");

        assert_eq!(serialized.as_c_str().to_bytes(), b"[]");
        assert!(serialized.as_c_str().to_bytes().is_ascii());
        assert!(!serialized.as_c_str().to_bytes().contains(&0));
    }

    #[test]
    fn should_preserve_sensitive_json_array_order_and_null_timestamps() {
        let first = document(b"first");
        let mut second = document(b"second");
        second.document_id = Identifier::from([9; 32]);
        second.updated_at_ms = None;

        let serialized =
            serialize_decrypted_documents(&[first, second]).expect("serialize documents");
        let json: serde_json::Value =
            serde_json::from_slice(serialized.as_c_str().to_bytes()).expect("valid JSON");

        assert_eq!(
            json[0]["id"],
            bs58::encode([1; 32]).into_string(),
            "fetch order must be preserved"
        );
        assert_eq!(
            json[1]["id"],
            bs58::encode([9; 32]).into_string(),
            "fetch order must be preserved"
        );
        assert!(json[1]["updatedAt"].is_null());
        assert_eq!(json[0]["payload"], "Zmlyc3Q=");
        assert_eq!(json[1]["payload"], "c2Vjb25k");
        assert!(serialized.as_c_str().to_bytes().is_ascii());
        assert!(!serialized.as_c_str().to_bytes().contains(&0));
    }

    #[test]
    fn should_reject_bounded_writer_overflow_without_growing() {
        let mut storage = [b' '; 3];
        let mut writer = FixedAsciiWriter::new(&mut storage);

        assert!(writer.write_ascii(b"four").is_err());
        assert_eq!(writer.written(), 0);
        assert_eq!(storage, [b' '; 3]);
    }

    #[test]
    fn should_zeroize_raw_pointer_bytes_before_release() {
        let serialized = serialize_decrypted_documents(&[document(b"secret")]).expect("serialize");
        let expected_len = serialized.as_c_str().to_bytes_with_nul().len();
        let raw = serialized.into_raw();

        let zeroized = unsafe { crate::types::zeroize_sensitive_string_into_bytes(raw) };

        assert_eq!(zeroized.len(), expected_len);
        assert!(zeroized.iter().all(|byte| *byte == 0));
    }

    #[test]
    fn should_write_into_mutable_owned_bytes_before_cstring_transfer() {
        // Deliberately `&Box<[u8]>` rather than `&[u8]`: the whole point is to
        // pin `inner`'s type as an OWNED, mutable heap allocation the serializer
        // writes into before ownership transfers to the C string. Taking a slice
        // here would accept a borrow of anything and assert nothing.
        #[allow(clippy::borrowed_box)]
        fn assert_mutable_byte_owner(_: &Box<[u8]>) {}

        let mut serialized = SensitiveCString::new(6).expect("allocate");
        let owned_bytes = serialized
            .inner
            .as_ref()
            .expect("sensitive bytes remain owned before transfer");
        assert_mutable_byte_owner(owned_bytes);
        let allocation_ptr = owned_bytes.as_ptr();
        serialized.content_mut().copy_from_slice(b"secret");

        let raw = serialized.into_raw();

        assert_eq!(raw.cast::<u8>().cast_const(), allocation_ptr);
        let zeroized = unsafe { crate::types::zeroize_sensitive_string_into_bytes(raw) };
        assert!(zeroized.iter().all(|byte| *byte == 0));
    }
}
