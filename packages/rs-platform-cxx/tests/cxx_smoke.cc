// Link-and-run check of the installed interface: the generated bridge
// header, the runtime header it includes, the hand-written WalletSigner and
// the static archive. Exercises one call of each kind the embedder makes:
// construction from a Config, the trust inputs, a read before any endpoint
// is known (a typed Unavailable status, never an exception or abort), a
// read with endpoints but no ChainLock anchor, every builder driven through
// a WalletSigner that declines, and the pure helpers.

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

bool is_kind(const platform_ffi::Status& status, platform_ffi::StatusKind kind)
{
    return status.kind == kind;
}

} // namespace

int main()
{
    platform_ffi::Config cfg;
    cfg.network = 1;
    cfg.tenderdash_chain_id = "dash-testnet-51";
    cfg.platform_llmq_type = 106;
    rust::Box<platform_ffi::PlatformClient> client = platform_ffi::new_platform_client(cfg);

    // A bad config is a rust::Error, not an abort.
    bool threw = false;
    try {
        platform_ffi::Config bad = cfg;
        bad.network = 42;
        platform_ffi::new_platform_client(bad);
    } catch (const rust::Error&) {
        threw = true;
    }
    if (!threw) return fail("bad network accepted");

    const std::array<std::uint8_t, 32> id{};
    const std::array<std::uint8_t, 20> hash160{};

    // Reads before any endpoint is known report Unavailable; nothing throws.
    if (!is_kind(client->get_identity(id).status, platform_ffi::StatusKind::Unavailable)) {
        return fail("read without endpoints is not Unavailable");
    }
    // A search prefix the network would refuse is refused here, whatever
    // the client's state.
    if (!is_kind(client->search_names("", 10, id).status, platform_ffi::StatusKind::Internal)) {
        return fail("empty search prefix is not refused");
    }
    if (!is_kind(client->resolve_name("alice").status, platform_ffi::StatusKind::Unavailable)) {
        return fail("resolve without endpoints is not Unavailable");
    }

    // Trust inputs round-trip. The slices view plain C++ containers: the
    // bridge takes `&[T]`, so no rust::Vec instantiation is needed.
    try {
        platform_ffi::QuorumKey key;
        for (std::size_t i = 0; i < key.hash.size(); ++i) key.hash[i] = static_cast<std::uint8_t>(i);
        for (std::size_t i = 0; i < key.pubkey.size(); ++i) key.pubkey[i] = static_cast<std::uint8_t>(i);
        const std::vector<platform_ffi::QuorumKey> keys{key};
        client->set_quorum_keys(rust::Slice<const platform_ffi::QuorumKey>(keys.data(), keys.size()));
        const std::vector<rust::String> endpoints{rust::String("https://127.0.0.1:1")};
        client->set_endpoints(rust::Slice<const rust::String>(endpoints.data(), endpoints.size()));
    } catch (const rust::Error& e) {
        return fail(e.what());
    }
    // Endpoints but no ChainLock anchor yet: proved reads are not
    // dispatched at all.
    if (!is_kind(client->get_identity(id).status, platform_ffi::StatusKind::Unavailable)) {
        return fail("read without a ChainLock anchor is not Unavailable");
    }
    client->set_chainlock_height(std::uint32_t{1});
    threw = false;
    try {
        const std::vector<rust::String> endpoints{rust::String("not a uri")};
        client->set_endpoints(rust::Slice<const rust::String>(endpoints.data(), endpoints.size()));
    } catch (const rust::Error&) {
        threw = true;
    }
    if (!threw) return fail("malformed endpoint accepted");

    // With an unreachable endpoint every read is a typed failure.
    const auto unreachable = client->get_identity_by_pubkey_hash(hash160);
    if (is_kind(unreachable.status, platform_ffi::StatusKind::Ok) ||
        is_kind(unreachable.status, platform_ffi::StatusKind::ProvenAbsent)) {
        return fail("unreachable endpoint produced a result");
    }

    // Before a verified read the network's protocol version is unknown on a
    // network whose floor is below this build's latest version: builders and
    // the contested fund amount refuse. A devnet's floor is the latest, so
    // its builders reach the WalletSigner.
    const auto expect_error = [](const char* what, auto&& call) {
        try {
            call();
        } catch (const rust::Error&) {
            return 0;
        }
        return fail(what);
    };
    if (expect_error("contested_vote_fund_credits before a verified read",
                     [&] { client->contested_vote_fund_credits(); }))
        return 1;
    platform_ffi::Config devnet_cfg;
    devnet_cfg.network = 2;
    devnet_cfg.tenderdash_chain_id = "devnet";
    devnet_cfg.platform_llmq_type = 106;
    rust::Box<platform_ffi::PlatformClient> devnet = platform_ffi::new_platform_client(devnet_cfg);
    if (devnet->contested_vote_fund_credits() == 0) return fail("contested_vote_fund_credits");

    // A builder round trip through the WalletSigner callback type: the
    // wallet declines, every builder surfaces that as rust::Error.
    bool signer_called = false;
    const platform_ffi::WalletSigner signer(
        [&](std::uint32_t, std::span<const std::uint8_t>, std::vector<std::uint8_t>&) {
            signer_called = true;
            return false;
        },
        [&](const std::array<std::uint8_t, 32>&, std::vector<std::uint8_t>&) {
            signer_called = true;
            return false;
        });
    platform_ffi::IdentityKey key;
    key.id = 1;
    key.purpose = 0;
    key.security_level = 2;
    key.key_type = 0;
    key.read_only = false;
    key.data.push_back(2);
    for (int i = 1; i < 33; ++i) key.data.push_back(0);
    key.disabled_at = 0;
    key.bounds.kind = platform_ffi::BoundsKind::NoBounds;

    if (expect_error("build_dpns_preorder before a verified read", [&] {
            client->build_dpns_preorder(id, std::uint64_t{1}, "alice", id, key, signer);
        }))
        return 1;
    if (expect_error("build_contact_request before a verified read", [&] {
            platform_ffi::Identity sender;
            sender.id = id;
            platform_ffi::ContactRequestInput input;
            input.to_user_id = id;
            client->build_contact_request(sender, sender, std::uint64_t{1}, input, key, signer);
        }))
        return 1;
    if (signer_called) return fail("the signer was called before the version was verified");
    if (expect_error("build_dpns_preorder", [&] {
            devnet->build_dpns_preorder(id, std::uint64_t{1}, "alice", id, key, signer);
        }))
        return 1;
    if (!signer_called) return fail("the devnet builder never reached the signer");
    if (expect_error("build_dpns_domain", [&] {
            devnet->build_dpns_domain(id, std::uint64_t{1}, "alice", id, key, signer);
        }))
        return 1;
    if (expect_error("build_profile", [&] {
            platform_ffi::Profile existing;
            existing.document_id = id;
            existing.owner = id;
            existing.revision = 1;
            platform_ffi::ProfileInput profile;
            profile.display_name = "name";
            devnet->build_profile(id, std::uint64_t{1}, existing, profile, key, signer);
        }))
        return 1;
    if (expect_error("build_contact_request", [&] {
            platform_ffi::Identity sender;
            sender.id = id;
            platform_ffi::Identity recipient = sender;
            platform_ffi::ContactRequestInput input;
            input.to_user_id = id;
            input.sender_key_index = 2;
            input.recipient_key_index = 2;
            input.account_reference = 0;
            devnet->build_contact_request(sender, recipient, std::uint64_t{1}, input, key, signer);
        }))
        return 1;
    if (expect_error("build_identity_create", [&] {
            platform_ffi::AssetLockProofInput proof;
            proof.is_instant = false;
            proof.core_chain_locked_height = 1;
            const std::vector<platform_ffi::NewIdentityKey> keys;
            devnet->build_identity_create(
                proof, rust::Slice<const platform_ffi::NewIdentityKey>(keys.data(), keys.size()),
                signer);
        }))
        return 1;

    // Pure helpers.
    if (std::string(platform_ffi::normalize_label("Alice")) != "a11ce") return fail("normalize_label");
    if (!platform_ffi::is_valid_username("alice")) return fail("is_valid_username");
    if (!platform_ffi::is_contested_username("alice")) return fail("is_contested_username");
    if (platform_ffi::credits_per_duff() != 1000) return fail("credits_per_duff");
    const auto dpns = platform_ffi::system_contract_id(platform_ffi::SystemContract::Dpns);
    const auto dashpay = platform_ffi::system_contract_id(platform_ffi::SystemContract::Dashpay);
    if (dpns == dashpay) return fail("system_contract_id");
    std::array<std::uint8_t, 32> mac{};
    const std::uint32_t reference = platform_ffi::dip15_account_reference_from_mac(mac, 5, 3);
    const auto unmasked = platform_ffi::dip15_unmask_account_reference_from_mac(mac, reference);
    if (unmasked.version != 3 || unmasked.account_index != 5) return fail("account reference");
    if (platform_ffi::dip15_receive_keys_acceptable(1, 2, 0)) return fail("key 0 accepted");
    if (!platform_ffi::dip15_receive_keys_acceptable(1, 2, 3)) return fail("keys refused");

    devnet->shutdown();
    client->shutdown();
    client->shutdown(); // idempotent
    if (!is_kind(client->get_identity(id).status, platform_ffi::StatusKind::Unavailable)) {
        return fail("read after shutdown is not Unavailable");
    }
    return 0;
}
