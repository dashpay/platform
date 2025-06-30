//! Optimized getter implementations to avoid unnecessary cloning

/// Helper trait for converting Vec<u8> to Uint8Array without cloning
pub trait VecU8ToUint8Array {
    fn to_uint8array(&self) -> js_sys::Uint8Array;
}

impl VecU8ToUint8Array for Vec<u8> {
    fn to_uint8array(&self) -> js_sys::Uint8Array {
        js_sys::Uint8Array::from(&self[..])
    }
}

impl VecU8ToUint8Array for [u8] {
    fn to_uint8array(&self) -> js_sys::Uint8Array {
        js_sys::Uint8Array::from(self)
    }
}
