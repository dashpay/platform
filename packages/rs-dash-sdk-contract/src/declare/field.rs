//! Stored fields: their stable name and position, their type and their bounds.
//!
//! Every variable-length field declares an upper bound; integer bounds must
//! fit the declared Rust width. The width itself is part of the manifest and
//! is enforced natively by emitting the corresponding schema bounds, so a
//! `u32` field stays a `u32` after native inference.

use alloc::string::String;
use alloc::vec::Vec;

use crate::identity::{CollectionName, PropertyName, PropertyPath};

/// The Rust integer width of an integer field.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum IntegerWidth {
    /// `u8`
    U8,
    /// `u16`
    U16,
    /// `u32`
    U32,
    /// `u64`
    U64,
    /// `i8`
    I8,
    /// `i16`
    I16,
    /// `i32`
    I32,
    /// `i64`
    I64,
}

impl IntegerWidth {
    /// Every width.
    pub const ALL: &'static [IntegerWidth] = &[
        IntegerWidth::U8,
        IntegerWidth::U16,
        IntegerWidth::U32,
        IntegerWidth::U64,
        IntegerWidth::I8,
        IntegerWidth::I16,
        IntegerWidth::I32,
        IntegerWidth::I64,
    ];

    /// The smallest value of the width.
    pub fn min_value(&self) -> i128 {
        match self {
            IntegerWidth::U8 | IntegerWidth::U16 | IntegerWidth::U32 | IntegerWidth::U64 => 0,
            IntegerWidth::I8 => i8::MIN as i128,
            IntegerWidth::I16 => i16::MIN as i128,
            IntegerWidth::I32 => i32::MIN as i128,
            IntegerWidth::I64 => i64::MIN as i128,
        }
    }

    /// The largest value of the width.
    pub fn max_value(&self) -> i128 {
        match self {
            IntegerWidth::U8 => u8::MAX as i128,
            IntegerWidth::U16 => u16::MAX as i128,
            IntegerWidth::U32 => u32::MAX as i128,
            IntegerWidth::U64 => u64::MAX as i128,
            IntegerWidth::I8 => i8::MAX as i128,
            IntegerWidth::I16 => i16::MAX as i128,
            IntegerWidth::I32 => i32::MAX as i128,
            IntegerWidth::I64 => i64::MAX as i128,
        }
    }

    /// Whether the width is signed.
    pub fn is_signed(&self) -> bool {
        matches!(
            self,
            IntegerWidth::I8 | IntegerWidth::I16 | IntegerWidth::I32 | IntegerWidth::I64
        )
    }

    /// The Rust type name.
    pub fn rust_name(&self) -> &'static str {
        match self {
            IntegerWidth::U8 => "u8",
            IntegerWidth::U16 => "u16",
            IntegerWidth::U32 => "u32",
            IntegerWidth::U64 => "u64",
            IntegerWidth::I8 => "i8",
            IntegerWidth::I16 => "i16",
            IntegerWidth::I32 => "i32",
            IntegerWidth::I64 => "i64",
        }
    }
}

/// Author-declared integer bounds. `i128` so that a bound outside `i64` is
/// representable and can be rejected instead of truncated.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct IntegerBounds {
    /// Inclusive lower bound.
    pub min: Option<i128>,
    /// Inclusive upper bound.
    pub max: Option<i128>,
}

impl IntegerBounds {
    /// No bounds beyond the width's own range.
    pub const NONE: IntegerBounds = IntegerBounds {
        min: None,
        max: None,
    };

    /// Both bounds.
    pub fn new(min: i128, max: i128) -> Self {
        IntegerBounds {
            min: Some(min),
            max: Some(max),
        }
    }
}

/// What an identifier field refers to.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ReferenceTarget {
    /// An identity.
    Identity,
    /// A data contract.
    Contract,
    /// A token.
    Token,
    /// A document of a type that forbids deletion.
    PermanentDocument {
        /// The contract holding the referenced type; the declaring contract
        /// when absent.
        contract: Option<[u8; 32]>,
        /// The referenced document type.
        document_type: CollectionName,
        /// Referring property to referenced property equalities enforced at
        /// write time.
        agreement: Vec<(PropertyPath, PropertyPath)>,
    },
    /// An identity public key; the reference carries the identity id and the
    /// named sibling property carries the key id.
    IdentityPublicKey {
        /// The property of the same collection carrying the key id.
        key_id_field: PropertyPath,
    },
}

/// The type of a stored field.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum FieldType {
    /// `bool`
    Bool,
    /// A Rust integer with optional narrower bounds.
    Integer {
        /// The Rust width.
        width: IntegerWidth,
        /// Author bounds inside the width.
        bounds: IntegerBounds,
    },
    /// `f64`
    F64,
    /// A string with a required character bound.
    String {
        /// Minimum characters.
        min_chars: Option<u16>,
        /// Maximum characters; absent is an `UnboundedField` diagnostic.
        max_chars: Option<u16>,
    },
    /// A byte array with a required length bound.
    Bytes {
        /// Minimum bytes.
        min_len: Option<u16>,
        /// Maximum bytes; absent is an `UnboundedField` diagnostic.
        max_len: Option<u16>,
    },
    /// A 32-byte identifier with no declared target.
    Identifier,
    /// A 32-byte identifier referring to a native object.
    Reference(ReferenceTarget),
    /// One of a closed set of strings.
    Enum(Vec<String>),
    /// A nested object with its own positioned fields.
    Object(Vec<FieldSpec>),
}

impl FieldType {
    /// An integer of the given width with no extra bounds.
    pub fn integer(width: IntegerWidth) -> Self {
        FieldType::Integer {
            width,
            bounds: IntegerBounds::NONE,
        }
    }

    /// An integer of the given width bounded to `min..=max`.
    pub fn bounded_integer(width: IntegerWidth, min: i128, max: i128) -> Self {
        FieldType::Integer {
            width,
            bounds: IntegerBounds::new(min, max),
        }
    }

    /// A string of at most `max_chars` characters.
    pub fn string(max_chars: u16) -> Self {
        FieldType::String {
            min_chars: None,
            max_chars: Some(max_chars),
        }
    }

    /// A byte array of at most `max_len` bytes.
    pub fn bytes(max_len: u16) -> Self {
        FieldType::Bytes {
            min_len: None,
            max_len: Some(max_len),
        }
    }

    /// An identifier referring to an identity.
    pub fn identity() -> Self {
        FieldType::Reference(ReferenceTarget::Identity)
    }
}

/// A stored field.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FieldSpec {
    /// The serialized property name; part of the field's identity.
    pub name: PropertyName,
    /// The serialization position; part of the field's identity. Positions are
    /// contiguous from 0 at each nesting level and never renumbered.
    pub position: u32,
    /// The type and bounds.
    pub ty: FieldType,
    /// Whether the property is required, default true.
    pub required: bool,
    /// Whether the property is validated but not stored, default false.
    pub transient: bool,
    /// Human description.
    pub description: Option<String>,
}

impl FieldSpec {
    /// A required, stored field.
    pub fn new(name: PropertyName, position: u32, ty: FieldType) -> Self {
        FieldSpec {
            name,
            position,
            ty,
            required: true,
            transient: false,
            description: None,
        }
    }

    /// Makes the field optional.
    pub fn optional(mut self) -> Self {
        self.required = false;
        self
    }

    /// Makes the field transient.
    pub fn transient(mut self) -> Self {
        self.transient = true;
        self
    }

    /// Adds a description.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_give_each_width_its_rust_range() {
        assert_eq!(IntegerWidth::U8.min_value(), 0);
        assert_eq!(IntegerWidth::U8.max_value(), 255);
        assert_eq!(IntegerWidth::U64.max_value(), u64::MAX as i128);
        assert_eq!(IntegerWidth::I8.min_value(), -128);
        assert_eq!(IntegerWidth::I64.min_value(), i64::MIN as i128);
        assert_eq!(IntegerWidth::I64.max_value(), i64::MAX as i128);
        assert!(IntegerWidth::I16.is_signed());
        assert!(!IntegerWidth::U16.is_signed());
    }
}
