//! Stamp the bincode encoding of `contacts.outgoing_request` and
//! `contacts.incoming_request`.
//!
//! `ContactRequest` gained `encrypted_shielded_address` as a trailing field,
//! and the request blobs are positional bincode records, so a record written
//! before this migration decodes only against the shape without it. The stamp
//! says which shape a blob carries: 0 is the record without the field, 1 the
//! current one, and `schema::contacts` holds the decoder for each. Each blob
//! has its own stamp because a row's two requests are written at different
//! times.
//!
//! Every row present when the migration runs is stamped 0. The column DEFAULT
//! is 1, so a later insert that does not name the column is read as the shape
//! the current writer produces; the CHECK refuses a stamp no decoder knows.
pub fn migration() -> String {
    "ALTER TABLE contacts ADD COLUMN outgoing_request_format INTEGER NOT NULL DEFAULT 1 CHECK(outgoing_request_format IN (0, 1));
     ALTER TABLE contacts ADD COLUMN incoming_request_format INTEGER NOT NULL DEFAULT 1 CHECK(incoming_request_format IN (0, 1));
     UPDATE contacts SET outgoing_request_format = 0, incoming_request_format = 0;"
        .to_string()
}
