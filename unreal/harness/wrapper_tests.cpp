// lingara_embed.hpp against K3's real host library, offline (ADR 1.10.26ac
// D4, D10). Run under ASan and UBSan on Linux and macOS, so "frees exactly
// once" is checked rather than read.
#include "doctest.h"
#include "harness_support.hpp"

#include <algorithm>
#include <thread>
#include <type_traits>
#include <utility>

using namespace harness;

namespace {

Client offline_client() {
    Status status = 0;
    Client client = Client::create(kOffline, status);
    REQUIRE(status == LINGARA_EMBED_OK);
    REQUIRE(client.valid());
    return client;
}

uint64_t opened_id(Client& client, std::vector<uint8_t>& answer) {
    Reply opened = execute(client, open_dialogue(), answer);
    REQUIRE(opened.status == LINGARA_EMBED_OK);
    REQUIRE(opened.body["type"] == "id");
    return opened.body["id"].get<uint64_t>();
}

// The status and text last_error answers on this thread.
std::pair<Status, std::string> last_error_now() {
    std::vector<uint8_t> buffer;
    std::string_view out;
    Status status = Client::last_error(buffer, out);
    return {status, std::string(out)};
}

}  // namespace

// 1.10.26ac AC1
TEST_CASE("the wrapper creates, executes, polls and frees one handle") {
    std::vector<uint8_t> answer(4096);
    std::vector<uint8_t> events(16384);
    Client client = offline_client();
    const uint64_t dialogue = opened_id(client, answer);
    Reply said = execute(client, say(dialogue), answer);
    REQUIRE(said.status == LINGARA_EMBED_OK);
    REQUIRE(said.body["type"] == "id");
    const uint64_t request = said.body["id"].get<uint64_t>();
    let_waiters_register();

    Reply needed = next_event(client, events);
    REQUIRE(needed.status == LINGARA_EMBED_OK);
    CHECK(needed.body["type"] == "token_needed");

    CHECK(execute(client, token_unavailable(), answer).status == LINGARA_EMBED_OK);
    Reply failed = next_event(client, events);
    REQUIRE(failed.status == LINGARA_EMBED_OK);
    CHECK(failed.body["type"] == "error");
    CHECK(failed.body["error"]["request"] == request);
    CHECK(quiet(client, events) == LINGARA_EMBED_EMPTY);
}

// 1.10.26ac AC2
TEST_CASE("the wrapper refuses another abi major or a lower minor") {
    using lingara_embed::abi_compatible;
    CHECK(abi_compatible((1u << 16) | 0u, 1, 0));
    CHECK(abi_compatible((1u << 16) | 3u, 1, 2));
    CHECK_FALSE(abi_compatible((2u << 16) | 0u, 1, 0));
    CHECK_FALSE(abi_compatible((1u << 16) | 1u, 1, 2));

    uint32_t foreign = static_cast<uint32_t>(LINGARA_EMBED_ABI_VERSION_MAJOR + 1) << 16;
    const std::string message = lingara_embed::abi_mismatch_message(foreign);
    CHECK(message.find(std::to_string(LINGARA_EMBED_ABI_VERSION_MAJOR + 1) + ".0") != std::string::npos);
    CHECK(message.find(std::to_string(LINGARA_EMBED_ABI_VERSION_MAJOR) + "." +
                       std::to_string(LINGARA_EMBED_ABI_VERSION_MINOR)) != std::string::npos);

    // On a fresh thread, so a status left by another test cannot pass for this one.
    std::thread([foreign] {
        Status status = 0;
        Client client = lingara_embed::detail::create_with_abi(foreign, kOffline, status);
        CHECK(status == LINGARA_EMBED_ERR_ABI_MISMATCH);
        CHECK_FALSE(client.valid());
        // lingara_embed_new was never reached, so it wrote nothing on this thread.
        CHECK(last_error_now().first == LINGARA_EMBED_EMPTY);
    }).join();
}

// 1.10.26ac AC3
TEST_CASE("an answer that does not fit is fetched once through last answer") {
    Client client = offline_client();
    std::vector<uint8_t> tiny(1);
    Reply first = execute(client, open_dialogue(), tiny);
    REQUIRE(first.status == LINGARA_EMBED_OK);
    REQUIRE(first.body["type"] == "id");
    const uint64_t id = first.body["id"].get<uint64_t>();

    std::vector<uint8_t> answer(4096);
    Reply second = execute(client, open_dialogue(), answer);
    REQUIRE(second.status == LINGARA_EMBED_OK);
    CHECK(second.body["id"] == id + 1);
}

// 1.10.26ac AC4
TEST_CASE("a core refusal is an embed error answer") {
    Client client = offline_client();
    std::vector<uint8_t> answer(4096);
    const auto before = last_error_now();
    Reply refused = execute(client, R"({"type":"practice_answer","session":999999,"answer":"x"})", answer);
    CHECK(refused.status == LINGARA_EMBED_REFUSED);
    CHECK(refused.body["kind"] == "unknown_id");
    CHECK(refused.body.contains("retryable"));
    CHECK(last_error_now() == before);
}

// 1.10.26ac AC5
TEST_CASE("the wrapper grows its poll buffer and keeps order") {
    Client client = offline_client();
    std::vector<uint8_t> answer(4096);
    std::vector<uint8_t> events(1);
    std::vector<uint64_t> requests;
    for (int n = 0; n < 3; ++n) {
        const uint64_t dialogue = opened_id(client, answer);
        Reply said = execute(client, say(dialogue), answer);
        REQUIRE(said.status == LINGARA_EMBED_OK);
        requests.push_back(said.body["id"].get<uint64_t>());
    }
    let_waiters_register();
    Reply needed = next_event(client, events);
    REQUIRE(needed.status == LINGARA_EMBED_OK);
    CHECK(needed.body["type"] == "token_needed");
    CHECK(events.size() > 1);
    CHECK(execute(client, token_unavailable(), answer).status == LINGARA_EMBED_OK);

    // The core wakes the three waiters together, so their order among
    // themselves is its own: each request is answered once, none lost.
    std::vector<uint64_t> answered;
    for (size_t n = 0; n < requests.size(); ++n) {
        Reply failed = next_event(client, events);
        REQUIRE(failed.status == LINGARA_EMBED_OK);
        CHECK(failed.body["type"] == "error");
        answered.push_back(failed.body["error"]["request"].get<uint64_t>());
    }
    std::sort(answered.begin(), answered.end());
    CHECK(answered == requests);
    CHECK(quiet(client, events) == LINGARA_EMBED_EMPTY);
}

// 1.10.26ac AC6
TEST_CASE("a moved-from client frees nothing") {
    static_assert(!std::is_copy_constructible_v<Client>, "Client is move-only");
    static_assert(!std::is_copy_assignable_v<Client>, "Client is move-only");
    Client first = offline_client();
    Client second = std::move(first);
    CHECK_FALSE(first.valid());  // NOLINT(bugprone-use-after-move): the point of the test

    std::vector<uint8_t> buffer;
    std::string_view status_json;
    CHECK(second.status(buffer, status_json) == LINGARA_EMBED_OK);
    CHECK(parse(status_json).contains("needs_token"));

    // Move-assigning over a live client frees the one it replaces, once.
    Client third = offline_client();
    third = std::move(second);
    CHECK(third.valid());
}
