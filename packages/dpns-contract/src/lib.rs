use platform_value::{Identifier, IdentifierBytes32};
use serde_json::{Error, Value};

pub const ID_BYTES: [u8; 32] = [
    230, 104, 198, 89, 175, 102, 174, 225, 231, 44, 24, 109, 222, 123, 91, 126, 10, 29, 113, 42, 9,
    196, 13, 87, 33, 246, 34, 191, 83, 197, 49, 85,
];

pub const OWNER_ID_BYTES: [u8; 32] = [
    48, 18, 193, 155, 152, 236, 0, 51, 173, 219, 54, 205, 100, 183, 245, 16, 103, 15, 42, 53, 26,
    67, 4, 181, 246, 153, 65, 68, 40, 110, 253, 172,
];

pub mod document_types {
    pub mod domain {
        pub const NAME: &str = "domain";

        pub mod properties {
            pub const LABEL: &str = "label";
            pub const NORMALIZED_LABEL: &str = "normalizedLabel";
            pub const PARENT_DOMAIN_NAME: &str = "parentDomainName";
            pub const NORMALIZED_PARENT_DOMAIN_NAME: &str = "normalizedParentDomainName";
            pub const PREORDER_SALT: &str = "preorderSalt";
            pub const ALLOW_SUBDOMAINS: &str = "subdomainRules.allowSubdomains";
            pub const RECORDS: &str = "records";
            pub const DASH_UNIQUE_IDENTITY_ID: &str = "dashUniqueIdentityId";
            pub const DASH_ALIAS_IDENTITY_ID: &str = "dashAliasIdentityId";
        }
    }
}

pub const ID: Identifier = Identifier(IdentifierBytes32(ID_BYTES));
pub const OWNER_ID: Identifier = Identifier(IdentifierBytes32(OWNER_ID_BYTES));

pub fn load_documents_schemas() -> Result<Value, Error> {
    serde_json::from_str(include_str!("../schema/dpns-contract-documents.json"))
}
