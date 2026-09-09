//! Pure cryptographic helpers used across the identity domain.
//!
//! No state, no network — just deterministic functions over keys,
//! paths, and bytes.

pub mod auto_accept;
pub mod contact_info;
pub mod dip14;
pub mod invitation;
pub mod validation;

pub use contact_info::{
    decode_private_data, derive_contact_info_keys, encode_private_data, ContactInfoKeys,
    ContactInfoPrivateData,
};
pub use dip14::{
    calculate_account_reference, derive_contact_xpub, unmask_account_reference, ContactXpubData,
};
pub use invitation::{
    encode_invitation_uri, parse_invitation_uri, voucher_output_index, wif_network_matches,
    InviterInfo, ParsedInvitation,
};
pub use validation::pubkey_binds_expected_key_data;
