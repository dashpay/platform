use crate::Error;
use serde_json::Value;

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

pub fn load_documents_schemas() -> Result<Value, Error> {
    serde_json::from_str(include_str!("../../schema/v1/dpns-contract-documents.json"))
        .map_err(Error::InvalidSchemaJson)
}
