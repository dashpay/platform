use grovedb::batch::key_info::KeyInfo;
use grovedb::Element;

/// Key element info
pub enum KeyElementInfo<'a> {
    /// An element
    KeyElement((&'a [u8], Element)),
    /// An element size
    KeyElementSize((KeyInfo, Element)),
    /// An element size
    KeyUnknownElementSize((KeyInfo, u32)),
}
