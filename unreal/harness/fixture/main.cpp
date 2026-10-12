// The EK harness for the Unreal surface, over the layer beneath the engine
// (ADR 1.10.26ac D10; R1 D6's rule for a binding whose engine cannot run in
// public CI). `conformance-server run` starts it with
// LINGARA_EMBED_SERVER_URL and LINGARA_EMBED_RESULTS; for each case it
// reads the case, arms it, drives lingara_embed::Client, and reports the
// server's verdict with its own mismatches as one JSON line.
#include "host_run.hpp"

#include "httplib.h"

#include <algorithm>
#include <cstdlib>
#include <filesystem>
#include <fstream>
#include <iostream>

namespace {

using fixture::json;
namespace fs = std::filesystem;

const char* const kLang = "unreal";

// `<group>/<slug>.yaml` → `<group>.<slug>`, sorted ordinally.
std::vector<std::string> case_ids(const fs::path& dir) {
    std::vector<std::string> ids;
    for (const auto& group : fs::directory_iterator(dir)) {
        if (!group.is_directory()) continue;
        for (const auto& file : fs::directory_iterator(group.path())) {
            if (file.path().extension() == ".yaml") {
                ids.push_back(group.path().filename().string() + "." + file.path().stem().string());
            }
        }
    }
    std::sort(ids.begin(), ids.end());
    return ids;
}

// One control call; a transport failure or a non-2xx status is a mismatch.
json control(httplib::Client& http, const std::string& path, const char* body, json& mismatches) {
    httplib::Result res = body == nullptr ? http.Get(path) : http.Post(path, body, "application/json");
    if (!res || res->status / 100 != 2) {
        mismatches.push_back("cannot reach " + path + ": " + (res ? std::to_string(res->status) : httplib::to_string(res.error())));
        return json();
    }
    return res->body.empty() ? json() : json::parse(res->body, nullptr, false);
}

// Arm, drive the host script, compare `expect.events`, then finish.
void drive(httplib::Client& http, const std::string& server, const std::string& id, const json& kase,
           json& mismatches) {
    control(http, "/__conformance/arm/" + id, "", mismatches);
    fixture::HostRun run(server);
    run.run(kase, id);
    for (const std::string& problem : run.problems()) mismatches.push_back(problem);
    const json want = kase.contains("expect") ? kase["expect"].value("events", json()) : json();
    if (want.is_array() && fixture::without_nulls(run.matched()) != fixture::without_nulls(want)) {
        mismatches.push_back(id + ": the host saw " + run.matched().dump() + ", expected " + want.dump());
    }
    const json verdict = control(http, "/__conformance/finish/" + id, "{}", mismatches);
    for (const json& m : verdict.value("mismatches", json::array())) mismatches.push_back(m);
}

json run_case(httplib::Client& http, const std::string& server, const std::string& id) {
    json mismatches = json::array();
    const json kase = control(http, "/__conformance/cases/" + id, nullptr, mismatches);
    if (kase.is_object()) {
        drive(http, server, id, kase, mismatches);
    }
    return json{{"case", id}, {"lang", kLang}, {"result", mismatches.empty() ? "pass" : "fail"},
                {"mismatches", mismatches}};
}

const char* env(const char* name, const char* fallback) {
    const char* value = std::getenv(name);  // NOLINT(concurrency-mt-unsafe): read once, before any thread
    return value != nullptr ? value : fallback;
}

}  // namespace

int main() {
    const std::string server = env("LINGARA_EMBED_SERVER_URL", "");
    const std::string results = env("LINGARA_EMBED_RESULTS", "");
    if (server.empty() || results.empty()) {
        std::cerr << "LINGARA_EMBED_SERVER_URL and LINGARA_EMBED_RESULTS must be set\n";
        return 2;
    }
    httplib::Client http(server);
    std::ofstream out(results);
    for (const std::string& id : case_ids(env("LINGARA_EMBED_CASES", "conformance/ek/cases"))) {
        out << run_case(http, server, id).dump() << "\n";
    }
    return 0;
}
