use grovedb::Element;

/// Element info
pub enum ElementInfo {
    /// An element
    Element(Element),
    /// An element size
    ElementSize(u32),
}
