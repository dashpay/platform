//! The keywords a document schema node may carry, grouped by what they apply to.

use strum::IntoEnumIterator;

/// A keyword a document meta-schema admits on a schema node.
///
/// The first six groups constrain a value, and [`SchemaNode`](super::SchemaNode) holds
/// them. The last two constrain nothing the JSON Schema validation of a document checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SchemaKeyword {
    /// Applies to a value of any type.
    Any(AnyKeyword),
    /// Applies to an object.
    Object(ObjectKeyword),
    /// Applies to an array.
    Array(ArrayKeyword),
    /// Applies to a string.
    String(StringKeyword),
    /// Applies to a number.
    Number(NumberKeyword),
    /// Says what kind of string, number or byte array a value is, beyond its JSON type.
    Format(FormatKeyword),
    /// Annotates, and constrains nothing.
    Annotation(AnnotationKeyword),
    /// Platform's own, which other stages read from the document type.
    Platform(PlatformKeyword),
}

/// Keywords that apply to a value of any type.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, strum::EnumIter, strum::IntoStaticStr,
)]
#[strum(serialize_all = "camelCase")]
pub enum AnyKeyword {
    /// `$ref`: a JSON pointer to a schema the value must also match.
    #[strum(serialize = "$ref")]
    Ref,
    /// `type`.
    Type,
    /// `enum`.
    Enum,
    /// `const`.
    Const,
}

/// Keywords that apply to an object.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, strum::EnumIter, strum::IntoStaticStr,
)]
#[strum(serialize_all = "camelCase")]
pub enum ObjectKeyword {
    /// `properties`.
    Properties,
    /// `additionalProperties`.
    AdditionalProperties,
    /// `required`.
    Required,
    /// `minProperties`.
    MinProperties,
    /// `maxProperties`.
    MaxProperties,
    /// `dependentRequired`.
    DependentRequired,
}

/// Keywords that apply to an array.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, strum::EnumIter, strum::IntoStaticStr,
)]
#[strum(serialize_all = "camelCase")]
pub enum ArrayKeyword {
    /// `items`.
    Items,
    /// `contains`.
    Contains,
    /// `minItems`.
    MinItems,
    /// `maxItems`.
    MaxItems,
    /// `uniqueItems`.
    UniqueItems,
}

/// Keywords that apply to a string.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, strum::EnumIter, strum::IntoStaticStr,
)]
#[strum(serialize_all = "camelCase")]
pub enum StringKeyword {
    /// `minLength`.
    MinLength,
    /// `maxLength`.
    MaxLength,
    /// `pattern`.
    Pattern,
}

/// Keywords that apply to a number.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, strum::EnumIter, strum::IntoStaticStr,
)]
#[strum(serialize_all = "camelCase")]
pub enum NumberKeyword {
    /// `minimum`.
    Minimum,
    /// `maximum`.
    Maximum,
    /// `exclusiveMinimum`.
    ExclusiveMinimum,
    /// `exclusiveMaximum`.
    ExclusiveMaximum,
    /// `multipleOf`.
    MultipleOf,
}

/// Keywords that say what kind of string, number or byte array a value is, beyond its JSON
/// type.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, strum::EnumIter, strum::IntoStaticStr,
)]
#[strum(serialize_all = "camelCase")]
pub enum FormatKeyword {
    /// `format`: a named format, such as `uri` or `date-time`, that a value must follow.
    Format,
    /// `byteArray`: Platform's keyword for an array whose elements are bytes.
    ByteArray,
    /// `contentMediaType`: a string keyword in JSON Schema, which Platform writes only on
    /// byte arrays, where it marks an identifier.
    ContentMediaType,
}

/// Keywords that annotate a schema and constrain nothing.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, strum::EnumIter, strum::IntoStaticStr,
)]
#[strum(serialize_all = "camelCase")]
pub enum AnnotationKeyword {
    /// `$comment`.
    #[strum(serialize = "$comment")]
    Comment,
    /// `$id`. A `$ref` resolves by JSON pointer only, as the parser refuses any other
    /// form, so an `$id` changes nothing a document is checked against.
    #[strum(serialize = "$id")]
    Id,
    /// `description`.
    Description,
    /// `examples`.
    Examples,
}

/// Platform's own keywords, which other stages read from the document type.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, strum::EnumIter, strum::IntoStaticStr,
)]
#[strum(serialize_all = "camelCase")]
pub enum PlatformKeyword {
    /// `position`.
    Position,
    /// `maxBytes`.
    MaxBytes,
    /// `refersTo`.
    RefersTo,
    /// `distinctFrom`.
    DistinctFrom,
    /// `encryptedFor`.
    EncryptedFor,
    /// `generatedFrom`.
    GeneratedFrom,
    /// `requiredSince`.
    RequiredSince,
}

impl SchemaKeyword {
    /// Every keyword, group by group in the order of [`SchemaKeyword`]'s variants.
    pub fn iter() -> impl Iterator<Item = SchemaKeyword> {
        AnyKeyword::iter()
            .map(SchemaKeyword::Any)
            .chain(ObjectKeyword::iter().map(SchemaKeyword::Object))
            .chain(ArrayKeyword::iter().map(SchemaKeyword::Array))
            .chain(StringKeyword::iter().map(SchemaKeyword::String))
            .chain(NumberKeyword::iter().map(SchemaKeyword::Number))
            .chain(FormatKeyword::iter().map(SchemaKeyword::Format))
            .chain(AnnotationKeyword::iter().map(SchemaKeyword::Annotation))
            .chain(PlatformKeyword::iter().map(SchemaKeyword::Platform))
    }

    /// The keyword as a schema writes it.
    pub fn name(self) -> &'static str {
        match self {
            SchemaKeyword::Any(keyword) => keyword.into(),
            SchemaKeyword::Object(keyword) => keyword.into(),
            SchemaKeyword::Array(keyword) => keyword.into(),
            SchemaKeyword::String(keyword) => keyword.into(),
            SchemaKeyword::Number(keyword) => keyword.into(),
            SchemaKeyword::Format(keyword) => keyword.into(),
            SchemaKeyword::Annotation(keyword) => keyword.into(),
            SchemaKeyword::Platform(keyword) => keyword.into(),
        }
    }

    /// The keyword a schema writes as `name`, if it is one.
    pub fn from_name(name: &str) -> Option<Self> {
        SchemaKeyword::iter().find(|keyword| keyword.name() == name)
    }

    /// Whether the keyword constrains a value, so that [`SchemaNode`](super::SchemaNode)
    /// holds it.
    pub fn constrains_a_value(self) -> bool {
        !matches!(
            self,
            SchemaKeyword::Annotation(_) | SchemaKeyword::Platform(_)
        )
    }
}
