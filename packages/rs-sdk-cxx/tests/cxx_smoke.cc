// Copyright (c) 2026 The Dash Core developers
// Distributed under the MIT software license, see the accompanying
// file COPYING or http://www.opensource.org/licenses/mit-license.php.

// Links the staged headers and the static archive the way an embedder does
// and crosses every bridged entry point once against a closed port: each
// call must come back as a typed status, never an exception or an abort.
// Each callback method is called directly for success, refusal and
// exception containment, and shutdown must destroy the CoreContext before
// any concurrent caller returns.

#include <dash/sdk/callbacks.h>
#include <dash/sdk/ffi.h>

#include <array>
#include <atomic>
#include <cstdint>
#include <cstdio>
#include <memory>
#include <stdexcept>
#include <thread>
#include <vector>

using namespace dash_sdk;

namespace {

int failures = 0;

void check(bool ok, const char* what)
{
    if (!ok) {
        std::fprintf(stderr, "cxx_smoke: %s\n", what);
        ++failures;
    }
}

bool unproved(const Status& status)
{
    return status.kind != StatusKind::Ok && status.kind != StatusKind::NotFound;
}

//! Flags its destruction, so a test can see when the CoreContext holding it
//! is destroyed.
struct Sentinel {
    explicit Sentinel(std::atomic<bool>& flag) : destroyed(flag) {}
    ~Sentinel() { destroyed = true; }
    std::atomic<bool>& destroyed;
};

std::unique_ptr<CoreContext> context(std::shared_ptr<Sentinel> sentinel = nullptr)
{
    return std::make_unique<CoreContext>(
        [sentinel](uint8_t, const std::array<uint8_t, 32>&, uint32_t, std::vector<uint8_t>&) {
            return QuorumLookup::Unavailable;
        },
        [](uint32_t&) { return false; });
}

Config config()
{
    Config cfg{};
    cfg.network = Network::Testnet;
    cfg.platform_llmq_type = 106;
    cfg.proxy.kind = ProxyKind::None;
    cfg.endpoints.push_back("https://127.0.0.1:1");
    return cfg;
}

bool refused(Config cfg, std::unique_ptr<CoreContext> ctx = context())
{
    try {
        new_platform_client(cfg, std::move(ctx));
    } catch (const rust::Error&) {
        return true;
    }
    return false;
}

} // namespace

int main()
{
    static_assert(sizeof(Network) == 1 && sizeof(StatusKind) == 1 && sizeof(QuorumLookup) == 1);
    static_assert(static_cast<uint8_t>(StatusKind::NotFound) == 1);
    static_assert(static_cast<uint8_t>(StatusKind::Consensus) == 4);
    static_assert(static_cast<uint8_t>(StatusKind::Contested) == 6);
    static_assert(static_cast<uint8_t>(AssetLockProofKind::Chain) == 1);
    static_assert(static_cast<uint8_t>(QuorumLookup::Unavailable) == 2);
    static_assert(static_cast<uint8_t>(FundingOutcome::Unavailable) == 5);
    static_assert(static_cast<uint8_t>(FundingOutcome::Submitted) == 6);
    static_assert(static_cast<uint8_t>(FundingOutcome::FeeCapReached) == 7);
    static_assert(static_cast<uint8_t>(AssetLockState::FullyConsumed) == 2);
    static_assert(static_cast<uint8_t>(ContestPolicyKind::Cap) == 1);

    // Configs the client cannot honour are errors, never a direct fallback.
    {
        Config bad = config();
        bad.network = static_cast<Network>(42);
        check(refused(bad), "unknown network accepted");
        bad = config();
        bad.proxy.kind = ProxyKind::Tcp;
        bad.proxy.address = "localhost:9050";
        check(refused(bad), "proxy host name accepted");
        bad.proxy.address = "127.0.0.1:9050";
        bad.proxy.kind = ProxyKind::None;
        check(refused(bad), "proxy address without a proxy kind accepted");
        bad.proxy.kind = static_cast<ProxyKind>(7);
        check(refused(bad), "unknown proxy kind accepted");
        bad.proxy.kind = ProxyKind::Unix;
        bad.proxy.address = "";
        check(refused(bad), "Unix proxy without a path accepted");
        bad.proxy.address = "/run/tor/socks";
        check(!refused(bad), "Unix proxy with a path refused");
        bad.proxy.kind = ProxyKind::Tcp;
        bad.proxy.address = "127.0.0.1:9050";
        bad.endpoints.push_back("https://evo.example:443");
        check(refused(bad), "proxied endpoint host name accepted");
        const char* onion = "https://2gzyxa5ihm7nsggfxnu52rck2vv4rvmdlkiu3zzui5du4xyclen53wid.onion:443";
        for (const char* uri : {"https://evo.example:443", "http://1.1.1.1:443", "1.1.1.1:443", onion}) {
            bad = config();
            bad.endpoints.push_back(uri);
            check(refused(bad), "endpoint accepted: not https with an IP host, or an onion host without a proxy");
        }
        check(refused(config(), nullptr), "missing CoreContext accepted");
        bad = config();
        bad.proxy.kind = ProxyKind::Tcp;
        bad.proxy.address = "[::1]:9050";
        bad.proxy.isolate = true;
        bad.endpoints.push_back(onion);
        bad.endpoints.push_back("https://[2001:db8::1]:443");
        check(!refused(bad), "proxied client with onion and IPv6 endpoints refused");
    }

    rust::Box<PlatformClient> client = new_platform_client(config(), context());
    const std::array<uint8_t, 32> id{1};
    const std::array<uint8_t, 20> hash160{2};

    // Reads.
    check(unproved(client->get_identity(id).status), "get_identity");
    check(unproved(client->get_identity_by_pubkey_hash(hash160).status), "get_identity_by_pubkey_hash");
    check(unproved(client->resolve_name("alice").status), "resolve_name");
    check(unproved(client->names_of_identity(id, 100, 10).status), "names_of_identity");
    check(client->names_of_identity(id, 0, 10).status.kind == StatusKind::Internal, "page_limit 0 accepted");
    check(client->names_of_identity(id, 100, 0).status.kind == StatusKind::Internal, "max_pages 0 accepted");
    check(unproved(client->search_names("ali", 10).status), "search_names");
    check(client->search_names("ali", 0).status.kind == StatusKind::Internal, "search limit 0 accepted");
    check(unproved(client->pending_contests_for_identity(id).status), "pending_contests_for_identity");
    check(unproved(client->contested_vote_state("a11ce").status), "contested_vote_state");
    const auto requests = client->contact_requests(id, ContactDirection::Received, 0);
    check(unproved(requests.status) && !requests.complete, "contact_requests");
    const std::array<uint8_t, 32> lock_txid{};
    const auto lock_state = client->asset_lock_state(lock_txid, 1);
    check(unproved(lock_state.status) && lock_state.state == AssetLockState::NotPresent, "asset_lock_state");
    check(client->contact_requests(id, static_cast<ContactDirection>(9), 0).status.kind == StatusKind::Internal,
          "unknown contact direction accepted");

    // Writes. Identity registration and top-up sign before they send, so
    // the WalletSigner is reached with the key and the asset-lock key_ref.
    int identity_key_calls = 0;
    int asset_lock_calls = 0;
    const auto signer = [&](bool throw_on_asset_lock) {
        return std::make_unique<WalletSigner>(
            [&](uint32_t key_id, uint8_t key_type, std::span<const uint8_t> key_data, std::span<const uint8_t> signable,
                std::array<uint8_t, 65>& sig) {
                ++identity_key_calls;
                check(key_id == 0 && key_type == 0 && key_data.size() == 33 && !signable.empty(),
                      "identity key marshalled wrongly");
                sig.fill(0x01);
                return true;
            },
            [&, throw_on_asset_lock](uint32_t key_ref, const std::array<uint8_t, 32>&, std::array<uint8_t, 65>&) -> bool {
                ++asset_lock_calls;
                check(key_ref == 77, "asset lock key_ref not passed back");
                if (throw_on_asset_lock) throw std::runtime_error("wallet failure");
                return false;
            });
    };
    IdentityKey key{};
    key.id = 0;
    key.purpose = 0;        // AUTHENTICATION
    key.security_level = 0; // MASTER
    key.key_type = 0;       // ECDSA_SECP256K1
    key.data.push_back(0x02);
    for (int i = 0; i < 32; ++i) key.data.push_back(0x79);
    AssetLockProof proof{};
    proof.kind = AssetLockProofKind::Chain;
    proof.core_chain_locked_height = 1;
    const std::vector<IdentityKey> keys{key};
    const rust::Slice<const IdentityKey> key_slice(keys.data(), keys.size());

    // A refused signature settles nothing about the lock; the attempt that
    // asked for it is reported, so the next floor is above it.
    const auto created = client->register_identity(key_slice, proof, 77, 3, signer(true));
    check(created.status.kind == StatusKind::Internal, "a throwing asset-lock signer was not a refusal");
    check(created.funding.outcome == FundingOutcome::Unavailable && created.funding.last_user_fee_increase == 3,
          "a refused registration was reported wrongly");
    const auto topped_up = client->top_up_identity(id, proof, 77, 0, signer(false));
    check(topped_up.status.kind == StatusKind::Internal, "a refusing asset-lock signer was not a refusal");
    check(topped_up.funding.outcome == FundingOutcome::Unavailable && topped_up.funding.last_user_fee_increase == 0,
          "a refused top-up was reported wrongly");
    check(asset_lock_calls == 2, "the asset-lock signer was not reached");
    check(identity_key_calls <= 1, "the identity key was signed more than once");
    // A floor above the cap submits nothing, and says no fee step is left.
    const auto capped = client->top_up_identity(id, proof, 77, 101, signer(false));
    check(capped.funding.outcome == FundingOutcome::FeeCapReached && asset_lock_calls == 2,
          "a floor above the fee cap was submitted, or not reported as the cap");
    proof.kind = static_cast<AssetLockProofKind>(5);
    const auto bad_proof = client->register_identity(key_slice, proof, 77, 4, signer(false));
    check(bad_proof.status.kind == StatusKind::Internal && bad_proof.funding.outcome == FundingOutcome::Unavailable &&
              bad_proof.funding.last_user_fee_increase == 3,
          "unknown proof kind accepted, or reported as an attempt");
    check(client->register_identity(key_slice, proof, 77, 0, nullptr).status.kind == StatusKind::Internal,
          "missing signer accepted");

    // Refuse decides a contested label before any network call or write.
    const ContestPolicy refuse{ContestPolicyKind::Refuse, 0};
    check(client->register_dpns_name("alice", id, 1, refuse, signer(false)).status.kind == StatusKind::Contested,
          "a contested label was not refused");
    check(unproved(client->register_dpns_name("alice-2000", id, 1, refuse, signer(false)).status),
          "register_dpns_name");
    const ContestPolicy cap{ContestPolicyKind::Cap, 0};
    check(unproved(client->register_dpns_name("alice", id, 1, cap, signer(false)).status), "register_dpns_name cap");
    const ContestPolicy unknown{static_cast<ContestPolicyKind>(9), 0};
    check(client->register_dpns_name("alice", id, 1, unknown, signer(false)).status.kind == StatusKind::Internal,
          "unknown contest policy accepted");
    ContactRequestInput request{};
    request.sender = id;
    request.recipient = id;
    check(client->send_contact_request(request, signer(false), nullptr).status.kind == StatusKind::Internal,
          "missing ContactKeys accepted");
    auto contact_keys = std::make_unique<ContactKeys>(
        [](uint32_t, const std::array<uint8_t, 33>&, std::array<uint8_t, 32>&) -> bool { throw std::runtime_error("x"); },
        nullptr);
    check(unproved(client->send_contact_request(request, signer(false), std::move(contact_keys)).status),
          "send_contact_request");

    // No network.
    check(is_valid_username("alice") && !is_valid_username("-"), "is_valid_username");
    check(is_contested_username("alice") && !is_contested_username("alice12345678901234567890"),
          "is_contested_username");
    bool threw = false;
    try {
        const std::array<uint8_t, 32> secret{};
        const std::vector<uint8_t> blob(96, 0);
        decrypt_contact_xpub(secret, rust::Slice<const uint8_t>(blob.data(), blob.size()));
    } catch (const rust::Error&) {
        threw = true;
    }
    check(threw, "an undecryptable xpub was accepted");

    std::array<uint8_t, 32> txid{};
    for (size_t i = 0; i < txid.size(); ++i) txid[i] = static_cast<uint8_t>(i);
    const auto derived = identity_id_from_asset_lock_outpoint(txid, 1);
    check(derived.status.kind == StatusKind::Ok && derived.id[0] == 0x52 && derived.id[31] == 0xfd,
          "identity_id_from_asset_lock_outpoint");

    // The callback types contain exceptions and check sizes. A quorum answer
    // the embedder gets wrong is Unavailable, never Invalid.
    {
        const std::array<uint8_t, 32> hash{};
        std::array<uint8_t, 48> out{};
        const auto answer = [](QuorumLookup lookup, size_t size) {
            return CoreContext(
                [lookup, size](uint8_t, const std::array<uint8_t, 32>&, uint32_t, std::vector<uint8_t>& key) {
                    key.assign(size, 0x05);
                    return lookup;
                },
                [](uint32_t&) -> bool { throw std::runtime_error("x"); });
        };
        check(answer(QuorumLookup::Found, 47).QuorumPublicKey(106, hash, 1, out) == QuorumLookup::Unavailable,
              "a 47-byte quorum key accepted");
        check(answer(QuorumLookup::Found, 49).QuorumPublicKey(106, hash, 1, out) == QuorumLookup::Unavailable,
              "a 49-byte quorum key accepted");
        check(answer(QuorumLookup::Invalid, 48).QuorumPublicKey(106, hash, 1, out) == QuorumLookup::Invalid,
              "an invalid quorum not reported");
        check(answer(QuorumLookup::Unavailable, 48).QuorumPublicKey(106, hash, 1, out) == QuorumLookup::Unavailable,
              "an undecided quorum not reported");
        check(answer(static_cast<QuorumLookup>(7), 48).QuorumPublicKey(106, hash, 1, out) ==
                  QuorumLookup::Unavailable,
              "an unknown quorum answer was not Unavailable");
        check(out == std::array<uint8_t, 48>{}, "a quorum key was copied without Found");
        check(answer(QuorumLookup::Found, 48).QuorumPublicKey(106, hash, 1, out) == QuorumLookup::Found &&
                  out[47] == 0x05,
              "a 48-byte quorum key refused");
        uint32_t height = 0;
        check(!answer(QuorumLookup::Found, 48).PlatformActivationHeight(height),
              "a throwing callback was not a refusal");
        const CoreContext throwing(
            [](uint8_t, const std::array<uint8_t, 32>&, uint32_t, std::vector<uint8_t>&) -> QuorumLookup {
                throw std::runtime_error("x");
            },
            nullptr);
        check(throwing.QuorumPublicKey(106, hash, 1, out) == QuorumLookup::Unavailable &&
                  !throwing.PlatformActivationHeight(height),
              "a throwing CoreContext was not Unavailable and a refusal");
        const CoreContext no_callbacks(nullptr, nullptr);
        check(no_callbacks.QuorumPublicKey(106, hash, 1, out) == QuorumLookup::Unavailable,
              "an empty CoreContext was not Unavailable");
        std::array<uint8_t, 65> sig{};
        const WalletSigner empty(nullptr, nullptr);
        check(!empty.SignAssetLock(1, hash, sig), "an empty WalletSigner signed");

        // WalletSigner::SignIdentityKey and ContactKeys: success, refusal, and
        // a throw contained as a refusal.
        const std::array<uint8_t, 3> data{1, 2, 3};
        const rust::Slice<const uint8_t> bytes(data.data(), data.size());
        const auto signs = [](int mode) {
            return WalletSigner(
                [mode](uint32_t, uint8_t, std::span<const uint8_t> key, std::span<const uint8_t> signable,
                       std::array<uint8_t, 65>& out) -> bool {
                    if (mode == 2) throw std::runtime_error("x");
                    out.fill(static_cast<uint8_t>(key.size() + signable.size()));
                    return mode == 0;
                },
                nullptr);
        };
        check(signs(0).SignIdentityKey(1, 0, bytes, bytes, sig) && sig[64] == 6, "SignIdentityKey did not answer");
        check(!signs(1).SignIdentityKey(1, 0, bytes, bytes, sig), "a refused SignIdentityKey signed");
        check(!signs(2).SignIdentityKey(1, 0, bytes, bytes, sig), "a throwing SignIdentityKey signed");
        const auto keys_for = [](int mode) {
            return ContactKeys(
                [mode](uint32_t, const std::array<uint8_t, 33>& theirs, std::array<uint8_t, 32>& out) -> bool {
                    if (mode == 2) throw std::runtime_error("x");
                    out.fill(theirs[0]);
                    return mode == 0;
                },
                [mode](const std::array<uint8_t, 32>& recipient, uint32_t reference, std::array<uint8_t, 69>& out) -> bool {
                    if (mode == 2) throw std::runtime_error("x");
                    out.fill(static_cast<uint8_t>(recipient[0] + reference));
                    return mode == 0;
                });
        };
        const std::array<uint8_t, 33> theirs{9};
        std::array<uint8_t, 32> secret{};
        std::array<uint8_t, 69> xpub{};
        check(keys_for(0).SharedSecret(1, theirs, secret) && secret[31] == 9, "SharedSecret did not answer");
        check(keys_for(0).CompactXpub(id, 2, xpub) && xpub[68] == 3, "CompactXpub did not answer");
        for (int mode : {1, 2}) {
            check(!keys_for(mode).SharedSecret(1, theirs, secret), "a refused or throwing SharedSecret answered");
            check(!keys_for(mode).CompactXpub(id, 2, xpub), "a refused or throwing CompactXpub answered");
        }
    }

    client->shutdown();
    client->shutdown();
    // One representative call after shutdown.
    check(client->get_identity(id).status.kind == StatusKind::Unavailable, "a call after shutdown is not Unavailable");

    // Concurrent shutdowns: neither returns before the CoreContext is gone.
    // Timing-dependent; the guarantee itself is structural (client.rs).
    {
        std::atomic<bool> destroyed{false};
        auto sentinel = std::make_shared<Sentinel>(destroyed);
        rust::Box<PlatformClient> other = new_platform_client(config(), context(sentinel));
        sentinel.reset();
        std::atomic<int> early{0};
        const auto stop = [&] {
            other->shutdown();
            if (!destroyed) ++early;
        };
        std::thread a(stop), b(stop);
        a.join();
        b.join();
        check(destroyed && early == 0, "a shutdown returned before the CoreContext was destroyed");
    }

    if (failures == 0) std::puts("cxx_smoke: ok");
    return failures == 0 ? 0 : 1;
}
