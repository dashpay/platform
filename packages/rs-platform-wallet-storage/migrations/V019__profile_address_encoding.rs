//! Stamp the bincode encoding of `identities.entry_blob` and
//! `dashpay_profiles.profile_blob`.
//!
//! Payment addresses (core, platform, shielded) extend `DashPayProfile`, which
//! is embedded positionally in both blobs, so a record written before this
//! migration decodes only against the pre-address shape. The stamp says which
//! shape a row carries: 0 is the pre-address record, 1 the current one, and
//! `schema::identity_profile_encoding` holds the decoder for each.
//!
//! Every row present when the migration runs is stamped 0. The column DEFAULT
//! is 1, so a later insert that does not name the column is read as the shape
//! the current writer produces; the CHECK refuses a stamp no decoder knows.
pub fn migration() -> String {
    "ALTER TABLE identities ADD COLUMN entry_format INTEGER NOT NULL DEFAULT 1 CHECK(entry_format IN (0, 1));
     UPDATE identities SET entry_format = 0;
     ALTER TABLE dashpay_profiles ADD COLUMN profile_format INTEGER NOT NULL DEFAULT 1 CHECK(profile_format IN (0, 1));
     UPDATE dashpay_profiles SET profile_format = 0;"
        .to_string()
}
