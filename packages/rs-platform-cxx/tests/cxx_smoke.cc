// Link-and-run check of the installed interface: the generated bridge
// header, the runtime header it includes, the hand-written WalletSigner,
// and the static archive. Exercises one call of each kind the embedder
// makes: client construction and context setup, a query without endpoints
// (expected to throw rust::Error rather than abort), quorum-key and
// endpoint plumbing, and every builder driven through a WalletSigner
// callback (refused because the signer declines).

#include <dash/platform/ffi.h>
#include <dash/platform/signer.h>

#include <array>
#include <cstdint>
#include <cstdio>
#include <string>
#include <vector>

namespace {

int fail(const char* what)
{
    std::fprintf(stderr, "cxx_smoke: %s\n", what);
    return 1;
}

} // namespace

int main()
{
    rust::Box<platform_ffi::PlatformClient> client = platform_ffi::new_platform_client();
    try {
        client->set_context("test", std::uint32_t{106}, "dash-testnet-51", std::uint32_t{12},
                            std::uint32_t{0});
    } catch (const rust::Error& e) {
        return fail(e.what());
    }

    // A query before any endpoint is known must surface as an exception,
    // never an abort.
    const std::array<std::uint8_t, 32> id{};
    const rust::Slice<const std::uint8_t> id_slice(id.data(), id.size());
    bool threw = false;
    try {
        client->get_identity(id_slice);
    } catch (const rust::Error&) {
        threw = true;
    }
    if (!threw) return fail("query without endpoints succeeded");

    // Quorum keys and endpoints round-trip; a malformed key is refused.
    try {
        rust::Vec<platform_ffi::FfiQuorumKey> keys;
        platform_ffi::FfiQuorumKey key;
        for (int i = 0; i < 32; ++i) key.quorum_hash.push_back(static_cast<std::uint8_t>(i));
        for (int i = 0; i < 48; ++i) key.pubkey.push_back(static_cast<std::uint8_t>(i));
        keys.push_back(std::move(key));
        client->update_quorum_keys(std::move(keys));
        rust::Vec<rust::String> endpoints;
        endpoints.push_back("https://127.0.0.1:1443");
        client->set_endpoints(std::move(endpoints));
        client->set_core_chain_locked_height(std::uint32_t{1});
    } catch (const rust::Error& e) {
        return fail(e.what());
    }
    threw = false;
    try {
        rust::Vec<platform_ffi::FfiQuorumKey> keys;
        platform_ffi::FfiQuorumKey key;
        key.quorum_hash.push_back(0);
        keys.push_back(std::move(key));
        client->update_quorum_keys(std::move(keys));
    } catch (const rust::Error&) {
        threw = true;
    }
    if (!threw) return fail("malformed quorum key accepted");

    // A builder round trip through the WalletSigner callback type.
    const platform_ffi::WalletSigner signer(
        [](std::uint32_t, const std::array<std::uint8_t, 32>&, std::vector<std::uint8_t>&) {
            return false; // wallet declines
        });
    platform_ffi::FfiIdentityKey key;
    key.id = 1;
    key.purpose = 0;
    key.security_level = 2;
    key.key_type = 0;
    key.read_only = false;
    for (int i = 0; i < 33; ++i) key.data.push_back(static_cast<std::uint8_t>(i == 0 ? 2 : 0));
    key.has_disabled_at = false;
    key.disabled_at = 0;

    // Every builder must reach the same guarded, fallible path: a declined
    // signer or a rejected input surfaces as rust::Error from each of them.
    const auto expect_error = [](const char* what, auto&& call) {
        try {
            call();
        } catch (const rust::Error&) {
            return 0;
        }
        return fail(what);
    };
    if (expect_error("st_build_dpns_preorder", [&] {
            client->st_build_dpns_preorder(id_slice, std::uint64_t{1}, "alice", id_slice,
                                           std::uint32_t{1}, key, signer);
        }))
        return 1;
    if (expect_error("st_build_dpns_domain", [&] {
            client->st_build_dpns_domain(id_slice, std::uint64_t{1}, "alice", "a11ce", "dash",
                                         id_slice, std::uint32_t{1}, key, signer);
        }))
        return 1;
    if (expect_error("st_build_profile", [&] {
            client->st_build_profile(id_slice, std::uint64_t{1}, "name", "", "", id_slice,
                                     rust::Slice<const std::uint8_t>(id.data(), 8),
                                     std::uint64_t{1}, false, id_slice, id_slice,
                                     std::uint32_t{1}, key, signer);
        }))
        return 1;
    if (expect_error("st_build_contact_request", [&] {
            std::vector<std::uint8_t> encrypted(96, 0);
            client->st_build_contact_request(
                id_slice, std::uint64_t{1}, id_slice,
                rust::Slice<const std::uint8_t>(encrypted.data(), encrypted.size()), 0, 0, 0,
                rust::Slice<const std::uint8_t>(encrypted.data(), 0), id_slice, std::uint32_t{1},
                key, signer);
        }))
        return 1;
    if (expect_error("st_build_identity_create", [&] {
            rust::Vec<platform_ffi::FfiNewIdentityKey> keys;
            client->st_build_identity_create(false, id_slice, id_slice, 0, 0,
                                             rust::Slice<const std::uint8_t>(id.data(), 32),
                                             std::move(keys), signer);
        }))
        return 1;

    // Stored-document decoders reject garbage cleanly.
    if (expect_error("decode_dpns_domain", [&] { client->decode_dpns_domain(id_slice); })) return 1;

    client->shutdown();
    client->shutdown(); // idempotent
    return 0;
}
