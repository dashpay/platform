//! DashPay profile models.
//!
//! Two structs:
//!
//! - [`DashPayProfile`] — the persisted/displayed profile. Stored on
//!   [`ManagedIdentity`](crate::wallet::identity::ManagedIdentity),
//!   emitted through [`IdentityEntry`](crate::changeset::IdentityEntry)
//!   for round-trip via the persister. No raw avatar bytes — only
//!   the computed hashes survive after document creation.
//!
//! - [`ProfileUpdate`] — input for
//!   [`IdentityWallet::create_profile`](crate::wallet::identity::IdentityWallet)
//!   / [`IdentityWallet::update_profile`](crate::wallet::identity::IdentityWallet).
//!   Carries the user-provided fields plus raw avatar bytes
//!   (pre-downloaded by the app layer). Platform-wallet computes
//!   SHA-256 hash + DHash fingerprint from the bytes, includes them in
//!   the document, then drops the bytes.

use sha2::{Digest, Sha256};

/// User-facing DashPay profile data published via the DashPay data
/// contract. This is the **output/stored** model — no raw image bytes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DashPayProfile {
    /// Display name (publicly visible, max 25 chars per DIP-15).
    pub display_name: Option<String>,
    /// Biography / about-me text (publicMessage field, max 140 chars).
    pub bio: Option<String>,
    /// URL of the avatar image (HTTPS, max 2048 chars per DIP-15).
    pub avatar_url: Option<String>,
    /// SHA-256 hash of the avatar image bytes (32 bytes).
    /// Required by the DashPay contract whenever `avatar_url` is set.
    pub avatar_hash: Option<[u8; 32]>,
    /// Perceptual hash (dHash) of the avatar image (8 bytes).
    /// Required by the DashPay contract whenever `avatar_url` is set.
    pub avatar_fingerprint: Option<[u8; 8]>,
    /// Public message broadcast to contacts.
    pub public_message: Option<String>,
}

/// A cached **contact** profile, keyed by the contact's identity id on the
/// owning [`ManagedIdentity`](crate::wallet::identity::ManagedIdentity).
///
/// Unlike the owner's own `dashpay_profile`, this cache is relationship-
/// independent — it serves established contacts, pending incoming-request
/// senders, and (later) ignored senders from one map. Holds **only the public
/// profile fields** parsed from the on-chain `profile` document; it must never
/// receive anything derived from the encrypted `contactInfo` path.
///
/// `profile` distinguishes three states:
/// - `Some(p)` — fetched and present;
/// - `None` — **confirmed absent** (the contact published no profile). This is
///   the negative cache that, together with `checked_at_ms`, stops the sweep
///   from re-querying a profile-less contact every tick forever.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ContactProfileEntry {
    /// The fetched profile, or `None` for a confirmed-absent profile.
    pub profile: Option<DashPayProfile>,
    /// Wall-clock ms of the last fetch attempt — drives the self-heal backoff
    /// for absent profiles. (Gates re-query cost only, never correctness.)
    pub checked_at_ms: u64,
}

/// Input for profile create/update operations. Only caller-provided
/// fields — platform-wallet computes `avatar_hash` + `avatar_fingerprint`
/// from `avatar_bytes` internally.
#[derive(Debug, Clone, Default)]
pub struct ProfileUpdate {
    /// Display name (max 25 chars per DIP-15).
    pub display_name: Option<String>,
    /// Public message / bio (max 140 chars per DIP-15).
    pub public_message: Option<String>,
    /// Avatar URL (HTTPS, max 2048 chars per DIP-15).
    pub avatar_url: Option<String>,
    /// Raw image bytes pre-downloaded by the app layer.
    /// Platform-wallet computes SHA-256 hash + DHash fingerprint,
    /// includes them in the document, then drops the bytes.
    /// `None` = no avatar / remove avatar.
    pub avatar_bytes: Option<Vec<u8>>,
}

/// Compute SHA-256 hash of image bytes (DIP-15 `avatarHash` field).
pub fn calculate_avatar_hash(image_bytes: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(image_bytes);
    let result = hasher.finalize();
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&result);
    hash
}

/// Compute DHash (Difference Hash) perceptual fingerprint of an image
/// (DIP-15 `avatarFingerprint` field, 8 bytes / 64 bits).
///
/// Algorithm:
/// 1. Decode image from bytes
/// 2. Convert to grayscale, resize to 9x8
/// 3. Compare each pixel with its right neighbor
/// 4. Generate 64-bit hash from comparisons
///
/// Returns `Err` if the bytes are not a valid image.
pub fn calculate_dhash_fingerprint(image_bytes: &[u8]) -> Result<[u8; 8], String> {
    let img =
        image::load_from_memory(image_bytes).map_err(|e| format!("Failed to load image: {e}"))?;
    let grayscale = img.grayscale();
    let resized = grayscale.resize_exact(9, 8, image::imageops::FilterType::Lanczos3);

    let mut hash = 0u64;
    let mut bit_position = 0;
    for y in 0..8u32 {
        for x in 0..8u32 {
            let left = image::GenericImageView::get_pixel(&resized, x, y).0[0];
            let right = image::GenericImageView::get_pixel(&resized, x + 1, y).0[0];
            if left > right {
                hash |= 1 << bit_position;
            }
            bit_position += 1;
        }
    }
    Ok(hash.to_le_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_avatar_hash_deterministic() {
        let data = b"test image data";
        assert_eq!(calculate_avatar_hash(data), calculate_avatar_hash(data));
    }

    #[test]
    fn test_avatar_hash_different_data() {
        assert_ne!(
            calculate_avatar_hash(b"first"),
            calculate_avatar_hash(b"second")
        );
    }

    #[test]
    fn test_dhash_valid_png() {
        use image::{ImageBuffer, Rgb};
        let img: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::from_fn(100, 100, |x, y| {
            Rgb([(x % 256) as u8, (y % 256) as u8, 128])
        });
        let mut png = Vec::new();
        img.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .unwrap();
        let fp = calculate_dhash_fingerprint(&png).unwrap();
        assert_eq!(fp.len(), 8);
    }

    #[test]
    fn test_dhash_invalid_data() {
        assert!(calculate_dhash_fingerprint(b"not an image").is_err());
    }
}
