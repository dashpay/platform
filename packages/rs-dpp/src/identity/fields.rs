pub mod property_names {
    pub const PUBLIC_KEYS: &str = "publicKeys";
    pub const ID_JSON: &str = "$id";
    pub const ID_RAW_OBJECT: &str = "id";
}

pub const IDENTITY_MAX_KEYS: u16 = 15000;

pub const IDENTIFIER_FIELDS_JSON: [&str; 1] = [property_names::ID_JSON];
pub const IDENTIFIER_FIELDS_RAW_OBJECT: [&str; 1] = [property_names::ID_RAW_OBJECT];
