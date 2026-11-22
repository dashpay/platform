//! Types specific to the DashPay data contract (DIP-15).

pub mod contact_request;
pub mod established_contact;
pub mod payment;
pub mod payment_address;
pub mod profile;

pub use contact_request::ContactRequest;
pub use established_contact::EstablishedContact;
pub use payment::{DashpayAddressMatch, PaymentDirection, PaymentEntry, PaymentStatus};
pub use payment_address::{
    profile_payment_address_from_string, profile_payment_address_to_string,
    ProfilePaymentAddressKind, ProfilePaymentAddressSupport,
};
pub use profile::{
    calculate_avatar_hash, calculate_dhash_fingerprint, normalize_persisted_profile,
    ContactProfileEntry, DashPayProfile, PaymentAddressUpdate, ProfileUpdate,
};
