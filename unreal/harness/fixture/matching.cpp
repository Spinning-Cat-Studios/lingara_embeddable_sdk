// How `poll_until` and `expect.events` compare (conformance/ek/CONTRACT.md):
// an object matches when each non-null key it names does, an array when it
// has the same length and each element matches, anything else when equal.
#include "host_run.hpp"

namespace fixture {

namespace {

json object_without_nulls(const json& node) {
    json out = json::object();
    for (auto it = node.begin(); it != node.end(); ++it) {
        if (!it.value().is_null()) {
            out[it.key()] = without_nulls(it.value());
        }
    }
    return out;
}

json array_without_nulls(const json& node) {
    json out = json::array();
    for (const json& item : node) {
        out.push_back(item.is_null() ? item : without_nulls(item));
    }
    return out;
}

}  // namespace

json without_nulls(const json& node) {
    if (node.is_object()) {
        return object_without_nulls(node);
    }
    return node.is_array() ? array_without_nulls(node) : node;
}

namespace {

bool object_contains(const json& actual, const json& wanted) {
    for (auto it = wanted.begin(); it != wanted.end(); ++it) {
        if (it.value().is_null()) {
            continue;
        }
        if (!actual.contains(it.key()) || !contains(actual[it.key()], it.value())) {
            return false;
        }
    }
    return true;
}

bool array_contains(const json& actual, const json& wanted) {
    if (actual.size() != wanted.size()) {
        return false;
    }
    for (size_t i = 0; i < wanted.size(); ++i) {
        if (!contains(actual[i], wanted[i])) {
            return false;
        }
    }
    return true;
}

}  // namespace

bool contains(const json& actual, const json& wanted) {
    if (wanted.is_null()) {
        return true;
    }
    if (wanted.is_object()) {
        return actual.is_object() && object_contains(actual, wanted);
    }
    if (wanted.is_array()) {
        return actual.is_array() && array_contains(actual, wanted);
    }
    return actual == wanted;
}

}  // namespace fixture
