// One EK case's `host:` script through lingara_embed::Client (ADR 1.10.26ac
// D10, conformance/ek/CONTRACT.md). This is an ABI harness: `close` frees
// the client, and no `closed` event is expected after it (EK9).
#ifndef LINGARA_FIXTURE_HOST_RUN_HPP
#define LINGARA_FIXTURE_HOST_RUN_HPP

#include "lingara_embed.hpp"

#include "json.hpp"

#include <cstdint>
#include <deque>
#include <optional>
#include <string>
#include <vector>

namespace fixture {

using nlohmann::json;

// A step's failure, or none. The fixture is built without exceptions.
using Problem = std::optional<std::string>;

class HostRun {
public:
    explicit HostRun(std::string server) : server_(std::move(server)) {}

    // Runs every step in order, stopping at the first that fails.
    void run(const json& kase, const std::string& id);

    const std::vector<std::string>& problems() const { return problems_; }
    const json& matched() const { return matched_; }

private:
    Problem step(const std::string& name, const json& body);
    Problem start(const json& body);
    Problem provide(const json& body);
    Problem command(const json& body);
    Problem poll_until(const json& body);
    Problem expect_no_event(const json& body);
    Problem refusable(const json& command);
    json substitute(const json& body) const;
    std::optional<json> next();
    std::optional<json> find(const std::string& type, const json& wanted, int64_t within_ms);
    std::string status_text(lingara_embed::Status status) const;

    std::string server_;
    lingara_embed::Client client_;
    std::deque<json> local_;
    std::optional<uint64_t> last_;
    std::vector<uint8_t> answer_ = std::vector<uint8_t>(4096);
    std::vector<uint8_t> events_ = std::vector<uint8_t>(16384);
    std::vector<std::string> problems_;
    json matched_ = json::array();
};

// CONTRACT.md's matching: containment after dropping nulls.
json without_nulls(const json& node);
bool contains(const json& actual, const json& wanted);

}  // namespace fixture

#endif  // LINGARA_FIXTURE_HOST_RUN_HPP
