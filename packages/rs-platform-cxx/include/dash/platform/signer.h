// Copyright (c) 2026 The Dash Core developers
// Distributed under the MIT software license, see the accompanying
// file COPYING or http://www.opensource.org/licenses/mit-license.php.

#ifndef DASH_PLATFORM_CXX_SIGNER_H
#define DASH_PLATFORM_CXX_SIGNER_H

#include <rust/cxx.h>

#include <array>
#include <cstdint>
#include <functional>
#include <span>
#include <utility>
#include <vector>

namespace platform_ffi {

//! Signer handed by reference into the Rust state-transition builders. The
//! builders run to completion on the calling thread, so both callbacks are
//! only ever invoked on the thread that called the builder. The type is
//! nonetheless required to be callable from any thread (the Rust side
//! declares it Send + Sync on that basis): the callbacks must take the
//! wallet's own lock and must not rely on thread-local state.
//!
//! SignForKey receives the id of the identity key to sign with and the full
//! signable preimage of the transition; the embedder hashes it (double
//! SHA256) itself and must answer with a 65-byte compact recoverable ECDSA
//! signature. SignAssetLockSighash is the one digest path: the asset-lock
//! outpoint key of an identity registration signs the 32-byte double SHA256
//! the builder computed; the key is compressed, so the header byte is
//! 31 + recovery id. Private keys never cross the FFI boundary.
class WalletSigner
{
public:
    using SignForKeyFn = std::function<bool(uint32_t key_id, std::span<const uint8_t> signable,
                                            std::vector<uint8_t>& sig_out)>;
    using SignAssetLockFn = std::function<bool(const std::array<uint8_t, 32>& sighash,
                                               std::vector<uint8_t>& sig_out)>;

    WalletSigner(SignForKeyFn sign_for_key, SignAssetLockFn sign_asset_lock)
        : m_sign_for_key(std::move(sign_for_key)), m_sign_asset_lock(std::move(sign_asset_lock))
    {
    }

    bool SignForKey(uint32_t key_id, rust::Slice<const uint8_t> signable,
                    rust::Vec<uint8_t>& sig_out) const
    {
        if (!m_sign_for_key) return false;
        std::vector<uint8_t> signature;
        // Called from Rust frames: a C++ exception must not unwind through
        // them (unsupported by cxx), so a throwing signer reads as a refusal.
        try {
            if (!m_sign_for_key(key_id, std::span<const uint8_t>(signable.data(), signable.size()),
                                signature)) {
                return false;
            }
        } catch (...) {
            return false;
        }
        Copy(signature, sig_out);
        return true;
    }

    bool SignAssetLockSighash(const std::array<uint8_t, 32>& sighash,
                              rust::Vec<uint8_t>& sig_out) const
    {
        if (!m_sign_asset_lock) return false;
        std::vector<uint8_t> signature;
        try {
            if (!m_sign_asset_lock(sighash, signature)) return false;
        } catch (...) {
            return false;
        }
        Copy(signature, sig_out);
        return true;
    }

private:
    static void Copy(const std::vector<uint8_t>& from, rust::Vec<uint8_t>& to)
    {
        to.clear();
        to.reserve(from.size());
        for (uint8_t byte : from) {
            to.push_back(byte);
        }
    }

    SignForKeyFn m_sign_for_key;
    SignAssetLockFn m_sign_asset_lock;
};

} // namespace platform_ffi

#endif // DASH_PLATFORM_CXX_SIGNER_H
