// Link-and-run check of the installed interface: the generated bridge
// header, the runtime header it includes, the hand-written WalletSigner,
// and the static archive. Exercises one call of each kind the embedder
// makes: context setup, a fallible verify (expected to throw rust::Error on
// garbage input rather than abort), and a builder driven through a
// WalletSigner callback (refused because the signer declines).

#include <dash/platform/ffi.h>
#include <dash/platform/signer.h>

#include <array>
#include <cstdint>
#include <cstdio>
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
    try {
        platform_ffi::set_context("test", std::uint32_t{106}, std::uint32_t{12}, std::uint32_t{0});
    } catch (const rust::Error& e) {
        return fail(e.what());
    }

    // Garbage bytes must surface as an exception, never an abort.
    bool threw = false;
    try {
        const std::array<std::uint8_t, 4> garbage{0xff, 0xff, 0xff, 0xff};
        platform_ffi::verify_get_identity_nonce(
            rust::Slice<const std::uint8_t>(garbage.data(), garbage.size()),
            rust::Slice<const std::uint8_t>(garbage.data(), garbage.size()));
    } catch (const rust::Error&) {
        threw = true;
    }
    if (!threw) return fail("garbage input verified");

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
    const std::array<std::uint8_t, 32> id{};
    threw = false;
    try {
        platform_ffi::st_build_dpns_preorder(
            rust::Slice<const std::uint8_t>(id.data(), id.size()), std::uint64_t{1}, "alice",
            rust::Slice<const std::uint8_t>(id.data(), id.size()), std::uint32_t{1}, key, signer);
    } catch (const rust::Error&) {
        threw = true;
    }
    if (!threw) return fail("declined signer produced a transition");

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
    const rust::Slice<const std::uint8_t> id_slice(id.data(), id.size());
    if (expect_error("st_build_dpns_domain", [&] {
            platform_ffi::st_build_dpns_domain(id_slice, std::uint64_t{1}, "alice", "a11ce", "dash",
                                               id_slice, std::uint32_t{1}, key, signer);
        }))
        return 1;
    if (expect_error("st_build_profile", [&] {
            platform_ffi::st_build_profile(id_slice, std::uint64_t{1}, "name", "", "", id_slice,
                                           rust::Slice<const std::uint8_t>(id.data(), 8),
                                           std::uint64_t{1}, false, id_slice, id_slice,
                                           std::uint32_t{1}, key, signer);
        }))
        return 1;
    if (expect_error("st_build_contact_request", [&] {
            std::vector<std::uint8_t> encrypted(96, 0);
            platform_ffi::st_build_contact_request(
                id_slice, std::uint64_t{1}, id_slice,
                rust::Slice<const std::uint8_t>(encrypted.data(), encrypted.size()), 0, 0, 0,
                rust::Slice<const std::uint8_t>(encrypted.data(), 0), id_slice, std::uint32_t{1},
                key, signer);
        }))
        return 1;
    if (expect_error("st_build_identity_create", [&] {
            rust::Vec<platform_ffi::FfiNewIdentityKey> keys;
            platform_ffi::st_build_identity_create(false, id_slice, id_slice, 0, 0,
                                                   rust::Slice<const std::uint8_t>(id.data(), 32),
                                                   std::move(keys), signer);
        }))
        return 1;
    return 0;
}
