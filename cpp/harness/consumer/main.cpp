// K5's offline smoke (ADR 1.10.26ac AC1), through the installed package
// (ADR 8.10.26q D6): the library loads, the ABI check passes, and the core
// answers. It proves packaging, not behaviour: EK replays the wrapper in
// conformance-ek-unreal. Replies are matched as text, so the consumer needs
// nothing beyond the package and doctest.
#include "doctest.h"
#include "lingara_embed.hpp"

#include <chrono>
#include <cstdint>
#include <string>
#include <string_view>
#include <thread>
#include <vector>

namespace {

using lingara_embed::Client;
using lingara_embed::Status;

// With the stream off, nothing reaches a network (ADR 1.10.26aa D12).
constexpr std::string_view kOffline = R"({"stream_on_start":false})";

// The unsigned integer after `"<key>":` in a reply, or 0.
uint64_t number_after(std::string_view json, std::string_view key) {
    const std::string needle = "\"" + std::string(key) + "\":";
    const size_t at = json.find(needle);
    if (at == std::string_view::npos) {
        return 0;
    }
    uint64_t value = 0;
    for (size_t i = at + needle.size(); i < json.size() && json[i] >= '0' && json[i] <= '9'; ++i) {
        value = value * 10 + static_cast<uint64_t>(json[i] - '0');
    }
    return value;
}

bool has_type(std::string_view json, std::string_view type) {
    return json.find("\"type\":\"" + std::string(type) + "\"") != std::string_view::npos;
}

// Polls until an event arrives or `within` passes; EMPTY if none did.
Status next_event(Client& client, std::vector<uint8_t>& buffer, std::string_view& out,
                  std::chrono::milliseconds within = std::chrono::milliseconds(2000)) {
    const auto deadline = std::chrono::steady_clock::now() + within;
    Status status = client.poll(buffer, out);
    while (status == LINGARA_EMBED_EMPTY && std::chrono::steady_clock::now() < deadline) {
        std::this_thread::sleep_for(std::chrono::milliseconds(5));
        status = client.poll(buffer, out);
    }
    return status;
}

}  // namespace

// 8.10.26q AC3 AC5 AC6
TEST_CASE("the installed library creates, executes, polls and frees one handle") {
    std::vector<uint8_t> answer(4096);
    std::vector<uint8_t> events(16384);
    std::string_view out;

    Status status = 0;
    Client client = Client::create(kOffline, status);
    REQUIRE(status == LINGARA_EMBED_OK);
    REQUIRE(client.valid());

    REQUIRE(client.execute(R"({"type":"dialogue_open","config":{"npc":{"name":"Mei"},"source_lang":"en","target_lang":"zh","level":2}})",
                           answer, out) == LINGARA_EMBED_OK);
    REQUIRE(has_type(out, "id"));
    const uint64_t dialogue = number_after(out, "id");

    const std::string say = R"({"type":"dialogue_say","dialogue":)" + std::to_string(dialogue) + R"(,"line":"ni hao"})";
    REQUIRE(client.execute(say, answer, out) == LINGARA_EMBED_OK);
    REQUIRE(has_type(out, "id"));
    const uint64_t request = number_after(out, "id");
    // The say reaches the core's token wait on its I/O thread, not in
    // execute; K5's suite waits the same 300 ms.
    std::this_thread::sleep_for(std::chrono::milliseconds(300));

    REQUIRE(next_event(client, events, out) == LINGARA_EMBED_OK);
    CHECK(has_type(out, "token_needed"));

    CHECK(client.execute(R"({"type":"token_unavailable","reason":"offline"})", answer, out) == LINGARA_EMBED_OK);
    REQUIRE(next_event(client, events, out) == LINGARA_EMBED_OK);
    CHECK(has_type(out, "error"));
    CHECK(number_after(out, "request") == request);
    CHECK(next_event(client, events, out, std::chrono::milliseconds(200)) == LINGARA_EMBED_EMPTY);
}
