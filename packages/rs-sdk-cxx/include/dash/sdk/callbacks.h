// Copyright (c) 2026 The Dash Core developers
// Distributed under the MIT software license, see the accompanying
// file COPYING or http://www.opensource.org/licenses/mit-license.php.

#ifndef DASH_SDK_CXX_CALLBACKS_H
#define DASH_SDK_CXX_CALLBACKS_H

#include <rust/cxx.h>

#include <algorithm>
#include <array>
#include <cstdint>
#include <functional>
#include <span>
#include <utility>
#include <vector>

//! The C++ objects the SDK calls back into. Each wraps std::function
//! callbacks the embedder supplies, and each method is noexcept: a callback
//! that is empty, throws, or returns false is a refusal, and the SDK call it
//! serves fails. Answers are written into fixed-size buffers owned by Rust,
//! so nothing crosses the FFI by allocation.
//!
//! The SDK calls these on its own worker threads, possibly concurrently
//! with other calls on the same client, and may destroy them on any thread.
//! Callbacks must be thread-safe, take the locks they need themselves, and
//! must not wait on another call to the same client or on shutdown().
//! Private keys never cross the FFI.
namespace dash_sdk {

namespace detail {
//! Runs a callback body: a throw is a refusal.
template <typename Fn>
bool Guard(Fn&& body) noexcept
{
    try {
        return body();
    } catch (...) {
        return false;
    }
}
} // namespace detail

//! Signs Platform state transitions with keys the wallet holds.
//!
//! SignIdentityKey: `key_id`, `key_type` and `key_data` describe the
//! identity key the SDK signs with, as Platform records it (key_data is the
//! 33-byte public key for ECDSA_SECP256K1 = 0, the 20-byte key hash for
//! ECDSA_HASH160 = 2). ECDSA_HASH160 is a masternode voting key
//! (cast_masternode_vote): key_id is 0, the id on the voter identity, and
//! key_data is the voting key's HASH160. The wallet signs
//! SHA256d(signable) and answers a 65-byte compact recoverable signature.
//!
//! SignAssetLock: signs the 32-byte digest with the private key of the asset
//! lock's credit output. `key_ref` is the embedder's own handle, passed back
//! unchanged from register_identity / top_up_identity. The key is
//! compressed, so the header byte is 31 + recovery id.
class WalletSigner
{
public:
    using SignIdentityKeyFn = std::function<bool(uint32_t key_id, uint8_t key_type, std::span<const uint8_t> key_data,
                                                 std::span<const uint8_t> signable, std::array<uint8_t, 65>& sig_out)>;
    using SignAssetLockFn =
        std::function<bool(uint32_t key_ref, const std::array<uint8_t, 32>& sighash, std::array<uint8_t, 65>& sig_out)>;

    WalletSigner(SignIdentityKeyFn sign_identity_key, SignAssetLockFn sign_asset_lock)
        : m_sign_identity_key(std::move(sign_identity_key)), m_sign_asset_lock(std::move(sign_asset_lock))
    {
    }

    bool SignIdentityKey(uint32_t key_id, uint8_t key_type, rust::Slice<const uint8_t> key_data,
                         rust::Slice<const uint8_t> signable, std::array<uint8_t, 65>& sig_out) const noexcept
    {
        return detail::Guard([&] {
            return m_sign_identity_key &&
                   m_sign_identity_key(key_id, key_type, {key_data.data(), key_data.size()},
                                       {signable.data(), signable.size()}, sig_out);
        });
    }

    bool SignAssetLock(uint32_t key_ref, const std::array<uint8_t, 32>& sighash,
                       std::array<uint8_t, 65>& sig_out) const noexcept
    {
        return detail::Guard([&] { return m_sign_asset_lock && m_sign_asset_lock(key_ref, sighash, sig_out); });
    }

private:
    SignIdentityKeyFn m_sign_identity_key;
    SignAssetLockFn m_sign_asset_lock;
};

//! The wallet's half of a DIP-15 contact request.
//!
//! SharedSecret: the ECDH secret between our identity key `sender_key_id`
//! and the recipient's compressed public key, as libsecp256k1's default
//! ECDH computes it: SHA256(0x02|0x03 by y parity || x).
//!
//! CompactXpub: the 69-byte DIP-15 compact extended public key
//! (parent fingerprint || chain code || public key) of the account the
//! wallet opens to `recipient` under `account_reference`.
class ContactKeys
{
public:
    using SharedSecretFn = std::function<bool(uint32_t sender_key_id, const std::array<uint8_t, 33>& recipient_key,
                                              std::array<uint8_t, 32>& secret_out)>;
    using CompactXpubFn = std::function<bool(const std::array<uint8_t, 32>& recipient, uint32_t account_reference,
                                             std::array<uint8_t, 69>& xpub_out)>;

    ContactKeys(SharedSecretFn shared_secret, CompactXpubFn compact_xpub)
        : m_shared_secret(std::move(shared_secret)), m_compact_xpub(std::move(compact_xpub))
    {
    }

    bool SharedSecret(uint32_t sender_key_id, const std::array<uint8_t, 33>& recipient_key,
                      std::array<uint8_t, 32>& secret_out) const noexcept
    {
        return detail::Guard(
            [&] { return m_shared_secret && m_shared_secret(sender_key_id, recipient_key, secret_out); });
    }

    bool CompactXpub(const std::array<uint8_t, 32>& recipient, uint32_t account_reference,
                     std::array<uint8_t, 69>& xpub_out) const noexcept
    {
        return detail::Guard(
            [&] { return m_compact_xpub && m_compact_xpub(recipient, account_reference, xpub_out); });
    }

private:
    SharedSecretFn m_shared_secret;
    CompactXpubFn m_compact_xpub;
};

//! The embedder's chain state, which the SDK verifies every proof against.
//!
//! QuorumPublicKey: the BLS public key of the quorum of type `llmq_type`
//! with hash `quorum_hash`, from the embedder's own mined commitments. The
//! hash is in the embedder's internal uint256 byte order. The client only
//! asks for its configured Platform LLMQ type; the callback must match both
//! type and hash and answer false for any quorum it does not hold. Any key
//! that is not exactly 48 bytes is refused. There is no fallback source.
//!
//! PlatformActivationHeight: the core height Platform activated at.
class CoreContext
{
public:
    using QuorumPublicKeyFn = std::function<bool(uint8_t llmq_type, const std::array<uint8_t, 32>& quorum_hash,
                                                 uint32_t core_chain_locked_height, std::vector<uint8_t>& key_out)>;
    using PlatformActivationHeightFn = std::function<bool(uint32_t& height_out)>;

    CoreContext(QuorumPublicKeyFn quorum_public_key, PlatformActivationHeightFn platform_activation_height)
        : m_quorum_public_key(std::move(quorum_public_key)),
          m_platform_activation_height(std::move(platform_activation_height))
    {
    }

    bool QuorumPublicKey(uint8_t llmq_type, const std::array<uint8_t, 32>& quorum_hash,
                         uint32_t core_chain_locked_height, std::array<uint8_t, 48>& key_out) const noexcept
    {
        return detail::Guard([&] {
            std::vector<uint8_t> key;
            if (!m_quorum_public_key || !m_quorum_public_key(llmq_type, quorum_hash, core_chain_locked_height, key) ||
                key.size() != key_out.size()) {
                return false;
            }
            std::copy(key.begin(), key.end(), key_out.begin());
            return true;
        });
    }

    bool PlatformActivationHeight(uint32_t& height_out) const noexcept
    {
        return detail::Guard([&] { return m_platform_activation_height && m_platform_activation_height(height_out); });
    }

private:
    QuorumPublicKeyFn m_quorum_public_key;
    PlatformActivationHeightFn m_platform_activation_height;
};

} // namespace dash_sdk

#endif // DASH_SDK_CXX_CALLBACKS_H
