// One function per K1 D12 `host:` step, over lingara_embed::Client (ADR
// 1.10.26ac D10). The wrapper already speaks K1's JSON, so a `command` step
// is `execute` of the step's JSON as written; a step this fixture cannot
// interpret fails its case, since a harness never skips (R1 D6).
#include "host_run.hpp"

#include <chrono>
#include <thread>

namespace fixture {

using lingara_embed::Client;
using lingara_embed::Status;

void HostRun::run(const json& kase, const std::string& id) {
    const json steps = kase.value("host", json::array());
    for (size_t n = 0; n < steps.size(); ++n) {
        const auto& entry = steps[n].items().begin();
        Problem problem = step(entry.key(), entry.value());
        if (problem) {
            problems_.push_back(id + ": host[" + std::to_string(n) + "]." + entry.key() + ": " + *problem);
            break;
        }
    }
    client_ = Client();
}

Problem HostRun::step(const std::string& name, const json& body) {
    if (name == "start") return start(body);
    if (name == "provide_token") return provide(body);
    if (name == "command") return command(body);
    if (name == "poll_until") return poll_until(body);
    if (name == "expect_no_event") return expect_no_event(body);
    if (name == "stop_polling") {
        std::this_thread::sleep_for(std::chrono::milliseconds(body.value("for_ms", int64_t{0})));
        return std::nullopt;
    }
    if (name == "close") {
        client_ = Client();  // an ABI harness frees: no `closed` follows (EK9)
        return std::nullopt;
    }
    return "unknown step " + name;
}

// `start`: the case's config with `base_url` set to the server, then a
// `provide_token_for(token, 900)` when an initial token is given.
Problem HostRun::start(const json& body) {
    json config = body.value("config", json::object());
    config["base_url"] = server_;
    Status status = 0;
    client_ = Client::create(config.dump(), status);
    if (status != LINGARA_EMBED_OK) {
        return "create answered " + status_text(status);
    }
    const std::string token = body.value("initial_token", std::string("none"));
    if (token == "none") {
        return std::nullopt;
    }
    return refusable(json{{"type", "provide_token_for"}, {"token", token}, {"expires_in_secs", 900}});
}

Problem HostRun::provide(const json& body) {
    json command = {{"type", "provide_token"}, {"token", body.value("token", std::string())}};
    if (body.contains("expires_at")) {
        command["expires_at"] = body["expires_at"];
    } else if (body.contains("expires_in_secs")) {
        command["type"] = "provide_token_for";
        command["expires_in_secs"] = body["expires_in_secs"];
    } else {
        return std::string("no expires_at or expires_in_secs");
    }
    return refusable(command);
}

Problem HostRun::command(const json& body) {
    return refusable(substitute(body));
}

// A refusal is an event, seen before anything still queued (CONTRACT.md).
// A negative status never reached the core, and fails the case.
Problem HostRun::refusable(const json& command) {
    if (!client_.valid()) {
        return std::string("no start step ran");
    }
    std::string_view out;
    const Status status = client_.execute(command.dump(), answer_, out);
    if (status == LINGARA_EMBED_REFUSED) {
        local_.push_back(json{{"type", "error"}, {"error", json::parse(out, nullptr, false)}});
        return std::nullopt;
    }
    if (status != LINGARA_EMBED_OK) {
        return "execute answered " + status_text(status);
    }
    const json answer = json::parse(out, nullptr, false);
    const std::string type = command.value("type", std::string());
    const bool opens = type == "practice_start" || type == "dialogue_open" || type == "dialogue_load";
    if (answer.value("type", std::string()) == "id" && opens) {
        last_ = answer["id"].get<uint64_t>();
    }
    return std::nullopt;
}

// Any top-level value "$last" becomes the id the latest open answered.
json HostRun::substitute(const json& body) const {
    json copy = body;
    for (auto it = copy.begin(); it != copy.end(); ++it) {
        if (it.value().is_string() && it.value() == "$last" && last_) {
            it.value() = *last_;
        }
    }
    return copy;
}

std::optional<json> HostRun::next() {
    if (!local_.empty()) {
        json seen = local_.front();
        local_.pop_front();
        return seen;
    }
    std::string_view out;
    if (client_.valid() && client_.poll(events_, out) == LINGARA_EMBED_OK) {
        return json::parse(out, nullptr, false);
    }
    return std::nullopt;
}

std::optional<json> HostRun::find(const std::string& type, const json& wanted, int64_t within_ms) {
    const auto deadline = std::chrono::steady_clock::now() + std::chrono::milliseconds(within_ms);
    while (std::chrono::steady_clock::now() < deadline) {
        std::optional<json> seen = next();
        if (!seen) {
            std::this_thread::sleep_for(std::chrono::milliseconds(5));
        } else if (seen->value("type", std::string()) == type && contains(*seen, wanted)) {
            return seen;
        }
    }
    return std::nullopt;
}

Problem HostRun::poll_until(const json& body) {
    const std::string type = body.value("event", std::string());
    const json wanted = body.value("match", json());
    const int64_t within = body.value("within_ms", int64_t{0});
    std::optional<json> seen = find(type, wanted, within);
    if (!seen) {
        return "no " + type + " event matching " + wanted.dump() + " within " + std::to_string(within) + " ms";
    }
    matched_.push_back(*seen);
    return std::nullopt;
}

Problem HostRun::expect_no_event(const json& body) {
    const std::string type = body.value("event", std::string());
    std::optional<json> seen = find(type, body.value("match", json()), body.value("for_ms", int64_t{0}));
    if (seen) {
        return "an unexpected " + type + " event: " + seen->dump();
    }
    return std::nullopt;
}

// A negative status's name and last_error's English text (K3 D4).
std::string HostRun::status_text(Status status) const {
    std::vector<uint8_t> buffer;
    std::string_view text;
    if (status < 0 && Client::last_error(buffer, text) == LINGARA_EMBED_OK) {
        return std::string(text);
    }
    return "status " + std::to_string(status);
}

}  // namespace fixture
