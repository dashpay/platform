//! The keywords a document schema node may carry, grouped by what they apply to.

use strum::IntoEnumIterator;

/// A keyword a document meta-schema admits on a schema node.
///
/// The first six groups constrain a value on its own, and
/// [`SchemaNode`](super::SchemaNode) holds them. The last four do not: a relationship is
/// checked against more than the value, and resolution and storage keywords and
/// annotations constrain no value at all. [`SchemaKeyword::checked_by`] says what checks a
/// value against each.
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
    /// Says what kind of string or byte array a value is, beyond its JSON type.
    Format(FormatKeyword),
    /// Ties the value to something outside it: another document, identity or key, or
    /// other properties of the same document.
    Relationship(RelationshipKeyword),
    /// Changes how the JSON Schema validator resolves a `$ref`.
    Resolution(ResolutionKeyword),
    /// Says how the document type parser stores the property.
    Storage(StorageKeyword),
    /// Annotates, and constrains nothing.
    Annotation(AnnotationKeyword),
}

/// Keywords that apply to a value of any type.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, strum::EnumIter, strum::IntoStaticStr,
)]
#[strum(serialize_all = "camelCase")]
pub enum AnyKeyword {
    /// `$ref`: a reference to a schema the value must also match. The validation schema
    /// holds only JSON pointers into the document schema (`#` or `#/...`).
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
    /// `byteArray`: Platform's keyword for an array whose elements are bytes, integers from
    /// 0 to 255. The JSON Schema validator checks it as a keyword of its own, refusing a
    /// value of any other JSON type whatever `type` says, but inside `contains`, where it
    /// constrains nothing.
    ByteArray,
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
    /// `maxBytes`: Platform's bound on a string's UTF-8 bytes, which JSON Schema leaves
    /// unchecked and the document validator checks right after it.
    MaxBytes,
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

/// Keywords that say what kind of string or byte array a value is, beyond its JSON type.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, strum::EnumIter, strum::IntoStaticStr,
)]
#[strum(serialize_all = "camelCase")]
pub enum FormatKeyword {
    /// `format`: a named format, such as `uri` or `date-time`, that a value must follow.
    Format,
    /// `encryptedFor`: the byte array is ciphertext made with a named scheme, for a
    /// recipient other properties name. JSON Schema leaves it unchecked; the validation of a
    /// create or replace checks that the bytes have a length the scheme produces.
    EncryptedFor,
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
    /// `description`.
    Description,
    /// `examples`.
    Examples,
}

/// Keywords that tie a value to something outside it. Consensus checks each of them, but
/// not from the value alone.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, strum::EnumIter, strum::IntoStaticStr,
)]
#[strum(serialize_all = "camelCase")]
pub enum RelationshipKeyword {
    /// `refersTo`: the value names another document, an identity or an identity key, which
    /// must exist and meet the declared conditions.
    RefersTo,
    /// `distinctFrom`: the value must differ from another identifier property or the
    /// document's owner.
    DistinctFrom,
    /// `generatedFrom`: the value is generated from other properties of the document.
    GeneratedFrom,
}

/// Keywords that change how the JSON Schema validator resolves a `$ref`, and constrain no
/// value.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, strum::EnumIter, strum::IntoStaticStr,
)]
#[strum(serialize_all = "camelCase")]
pub enum ResolutionKeyword {
    /// `$id`: names the schema. The JSON Schema validator resolves a `$ref` through the
    /// schemas named this way before it follows the pointer, and against the `$id`s above
    /// it, which the validation schema does not represent. The builder refuses a document
    /// schema `$id`, and any other that is not a plain name (ASCII letters, digits, `_`, `-`
    /// and `.`), which leaves every `$ref` resolving to the schema at its pointer.
    #[strum(serialize = "$id")]
    Id,
}

/// Keywords that say how the document type parser stores the property. Consensus depends on
/// them, but none bounds a value: the parser stores what validation has accepted.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, strum::EnumIter, strum::IntoStaticStr,
)]
#[strum(serialize_all = "camelCase")]
pub enum StorageKeyword {
    /// `position`: the property's place in the stored document, whose values are written
    /// one after another without names.
    Position,
    /// `requiredSince`: the contract version from which the property is required. A document
    /// whose contract version has reached it stores the property without a presence flag;
    /// an earlier one stores the flag. A new document is validated against `required`, which
    /// must list the property.
    RequiredSince,
    /// `contentMediaType`: on a byte array, `application/x.dash.dpp.identifier` makes the
    /// property an identifier, a type the document type parser reads; the meta-schemas then
    /// require `byteArray` and exactly 32 items, which bound the value. JSON Schema draft
    /// 2020-12, which the document validator compiles, holds it an annotation and checks
    /// nothing for it.
    ContentMediaType,
}

/// What checks a document value against a keyword.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckedBy {
    /// The JSON Schema validator, wherever it compiles the keyword.
    JsonSchemaValidator,
    /// Consensus, on the property the document type parser builds from the schema: only
    /// where the parser reads a schema as a property, and for `maxBytes` also on its
    /// `items`.
    DocumentTypeParser,
    /// Consensus, against more than the value: other properties of the document, other
    /// documents, identities or keys.
    Relationship,
    /// Nothing: the keyword bounds no value.
    Nothing,
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
            .chain(RelationshipKeyword::iter().map(SchemaKeyword::Relationship))
            .chain(ResolutionKeyword::iter().map(SchemaKeyword::Resolution))
            .chain(StorageKeyword::iter().map(SchemaKeyword::Storage))
            .chain(AnnotationKeyword::iter().map(SchemaKeyword::Annotation))
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
            SchemaKeyword::Relationship(keyword) => keyword.into(),
            SchemaKeyword::Resolution(keyword) => keyword.into(),
            SchemaKeyword::Storage(keyword) => keyword.into(),
            SchemaKeyword::Annotation(keyword) => keyword.into(),
        }
    }

    /// The keyword a schema writes as `name`, if it is one.
    pub fn from_name(name: &str) -> Option<Self> {
        SchemaKeyword::iter().find(|keyword| keyword.name() == name)
    }

    /// What checks a document value against the keyword.
    pub fn checked_by(self) -> CheckedBy {
        match self {
            SchemaKeyword::String(StringKeyword::MaxBytes)
            | SchemaKeyword::Format(FormatKeyword::EncryptedFor) => CheckedBy::DocumentTypeParser,
            SchemaKeyword::Any(_)
            | SchemaKeyword::Object(_)
            | SchemaKeyword::Array(_)
            | SchemaKeyword::String(_)
            | SchemaKeyword::Number(_)
            | SchemaKeyword::Format(_) => CheckedBy::JsonSchemaValidator,
            SchemaKeyword::Relationship(_) => CheckedBy::Relationship,
            SchemaKeyword::Resolution(_)
            | SchemaKeyword::Storage(_)
            | SchemaKeyword::Annotation(_) => CheckedBy::Nothing,
        }
    }

    /// Whether the keyword constrains a value on its own, so that
    /// [`SchemaNode`](super::SchemaNode) holds it: whether the JSON Schema validator or the
    /// document type parser checks it.
    pub fn constrains_a_value(self) -> bool {
        matches!(
            self.checked_by(),
            CheckedBy::JsonSchemaValidator | CheckedBy::DocumentTypeParser
        )
    }
}
