//! Profile addresses extend a positional bincode record embedded in identities.
//! Tag existing rows as format 0 and new writes as format 1; legacy decoders keep
//! old databases readable without treating a corrupt new record as an old one.
pub fn migration() -> String {
    "ALTER TABLE identities ADD COLUMN entry_format INTEGER NOT NULL DEFAULT 0 CHECK(entry_format IN (0, 1));
     ALTER TABLE dashpay_profiles ADD COLUMN profile_format INTEGER NOT NULL DEFAULT 0 CHECK(profile_format IN (0, 1));".to_string()
}
