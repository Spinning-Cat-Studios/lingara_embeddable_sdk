// What the wrapper suites share (ADR 1.10.26ac D10): an offline config,
// K1 commands as JSON, and polling with a deadline. Tests parse JSON with
// the vendored nlohmann/json; the wrapper itself never does.
#ifndef LINGARA_HARNESS_SUPPORT_HPP
#define LINGARA_HARNESS_SUPPORT_HPP

#include "lingara_embed.hpp"

#include "json.hpp"

#include <chrono>
#include <cstdint>
#include <string>
#include <string_view>
#include <thread>
#include <vector>

namespace harness {

using lingara_embed::Client;
using lingara_embed::Status;
using nlohmann::json;

// K3 D12's offline shape: with the stream off, nothing reaches a network.
inline constexpr std::string_view kOffline = R"({"stream_on_start":false})";

struct Reply {
    Status status = 0;
    json body;
};

inline json parse(std::string_view text) {
    return json::parse(text.begin(), text.end(), nullptr, false);
}

inline Reply execute(Client& client, const std::string& command, std::vector<uint8_t>& answer) {
    std::string_view out;
    Status status = client.execute(command, answer, out);
    return Reply{status, parse(out)};
}

inline std::string open_dialogue() {
    return R"({"type":"dialogue_open","config":{"npc":{"name":"Mei"},"source_lang":"en","target_lang":"zh","level":2}})";
}

inline std::string say(uint64_t dialogue) {
    return json{{"type", "dialogue_say"}, {"dialogue", dialogue}, {"line", "ni hao"}}.dump();
}

inline std::string token_unavailable() {
    return R"({"type":"token_unavailable","reason":"offline"})";
}

// A `dialogue_say` reaches the core's token wait on its I/O thread, not in
// `execute`. A `token_unavailable` sent before it gets there fails nobody,
// and the say then raises a second need. K3's C# suite waits the same
// 300 ms (1.10.26aa AC17).
inline void let_waiters_register() {
    std::this_thread::sleep_for(std::chrono::milliseconds(300));
}

// Polls until an event arrives or `within` passes; EMPTY if none did.
inline Reply next_event(Client& client, std::vector<uint8_t>& buffer,
                        std::chrono::milliseconds within = std::chrono::milliseconds(2000)) {
    const auto deadline = std::chrono::steady_clock::now() + within;
    std::string_view out;
    Status status = client.poll(buffer, out);
    while (status == LINGARA_EMBED_EMPTY && std::chrono::steady_clock::now() < deadline) {
        std::this_thread::sleep_for(std::chrono::milliseconds(5));
        status = client.poll(buffer, out);
    }
    return Reply{status, status == LINGARA_EMBED_OK ? parse(out) : json()};
}

// Nothing more arrives within a short quiet period.
inline Status quiet(Client& client, std::vector<uint8_t>& buffer) {
    return next_event(client, buffer, std::chrono::milliseconds(200)).status;
}

}  // namespace harness

#endif  // LINGARA_HARNESS_SUPPORT_HPP
