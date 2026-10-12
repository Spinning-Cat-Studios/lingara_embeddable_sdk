// lingara_embed.hpp: an engine-free C++17 wrapper over lingara_embed.h
// (ADR 1.10.26ac D4). Header-only, with no Unreal include, no exceptions and
// no RTTI. It moves bytes and never parses JSON: decoding is the caller's.
//
// The thread contract is the ABI's own (lingara_embed.h, K3 D4):
//   - one thread polls a client;
//   - `execute` and the `last_answer` it may need run on one thread, which
//     Client::execute guarantees by making both calls itself;
//   - the destructor races no other call on the same client.
//
// Every `out_json` is a view into the caller's buffer, valid until that
// buffer's next use. A negative status leaves it empty: read the reason with
// Client::last_error on the same thread.
#ifndef LINGARA_EMBED_HPP
#define LINGARA_EMBED_HPP

#include "lingara_embed.h"

#include <cstddef>
#include <cstdint>
#include <string>
#include <string_view>
#include <utility>
#include <vector>

namespace lingara_embed {

// Compared against the header's LINGARA_EMBED_* codes; no enum of our own.
using Status = int32_t;

// The library's `(major << 16) | minor` against the major and minor this
// header was built with: the same major, and a minor no lower (K3 D5).
constexpr bool abi_compatible(uint32_t library, uint32_t major, uint32_t minor) {
    return (library >> 16) == major && (library & 0xFFFFu) >= minor;
}

// The refusal never reaches the library, so last_error has nothing: this
// names both versions instead (K3 D5).
inline std::string abi_mismatch_message(uint32_t library) {
    return "ERR_ABI_MISMATCH: the library's ABI is " + std::to_string(library >> 16) + "." +
           std::to_string(library & 0xFFFFu) + ", and this header needs " +
           std::to_string(LINGARA_EMBED_ABI_VERSION_MAJOR) + "." +
           std::to_string(LINGARA_EMBED_ABI_VERSION_MINOR) + " or a later minor";
}

class Client;

namespace detail {

Client create_with_abi(uint32_t library, std::string_view config_json, Status& out);

inline const uint8_t* bytes(std::string_view text) {
    return reinterpret_cast<const uint8_t*>(text.data());
}

inline std::string_view view(const std::vector<uint8_t>& buffer, size_t length) {
    return std::string_view(reinterpret_cast<const char*>(buffer.data()), length);
}

inline size_t next_power_of_two(size_t n) {
    size_t size = 1;
    while (size < n) {
        size <<= 1;
    }
    return size;
}

// One fetch with the ABI's growth rule: on BUFFER_TOO_SMALL nothing was
// consumed, so grow to the next power of two at or above the size it asked
// for and fetch again, which returns the same message.
template <typename Fetch>
Status fetch_growing(std::vector<uint8_t>& buffer, std::string_view& out_json, Fetch fetch) {
    size_t length = 0;
    Status status = fetch(buffer.data(), buffer.size(), &length);
    if (status == LINGARA_EMBED_BUFFER_TOO_SMALL) {
        buffer.resize(next_power_of_two(length));
        status = fetch(buffer.data(), buffer.size(), &length);
    }
    const bool written = status == LINGARA_EMBED_OK || status == LINGARA_EMBED_REFUSED;
    out_json = written ? view(buffer, length) : std::string_view();
    return status;
}

}  // namespace detail

// One player's connection. Move-only: a moved-from client owns nothing and
// frees nothing.
class Client {
public:
    Client() = default;
    Client(const Client&) = delete;
    Client& operator=(const Client&) = delete;
    Client(Client&& other) noexcept : handle_(std::exchange(other.handle_, nullptr)) {}
    Client& operator=(Client&& other) noexcept {
        if (this != &other) {
            reset();
            handle_ = std::exchange(other.handle_, nullptr);
        }
        return *this;
    }
    ~Client() { reset(); }

    static uint32_t abi_version() { return lingara_embed_abi_version(); }
    static std::string_view library_version() { return lingara_embed_version(); }

    // `config_json` is K1's EmbedConfig; empty is `{}`. On REFUSED the
    // EmbedError is this thread's last_answer.
    static Client create(std::string_view config_json, Status& out) {
        return detail::create_with_abi(abi_version(), config_json, out);
    }

    bool valid() const { return handle_ != nullptr; }

    // Runs one K1 Command. On BUFFER_TOO_SMALL the command HAS run, so its
    // answer is fetched once through last_answer, never by running it again.
    Status execute(std::string_view command_json, std::vector<uint8_t>& answer,
                   std::string_view& out_json) {
        size_t length = 0;
        Status status = lingara_embed_execute(handle_, detail::bytes(command_json), command_json.size(),
                                              answer.data(), answer.size(), &length);
        if (status == LINGARA_EMBED_BUFFER_TOO_SMALL) {
            answer.resize(length);
            status = lingara_embed_last_answer(answer.data(), answer.size(), &length);
        }
        const bool written = status == LINGARA_EMBED_OK || status == LINGARA_EMBED_REFUSED;
        out_json = written ? detail::view(answer, length) : std::string_view();
        return status;
    }

    // One K1 EmbedEvent, or EMPTY.
    Status poll(std::vector<uint8_t>& buffer, std::string_view& out_json) {
        LingaraEmbed* handle = handle_;
        return detail::fetch_growing(buffer, out_json, [handle](uint8_t* b, size_t cap, size_t* len) {
            return lingara_embed_poll(handle, b, cap, len);
        });
    }

    // K1's EmbedStatus.
    Status status(std::vector<uint8_t>& buffer, std::string_view& out_json) const {
        const LingaraEmbed* handle = handle_;
        return detail::fetch_growing(buffer, out_json, [handle](uint8_t* b, size_t cap, size_t* len) {
            return lingara_embed_status(handle, b, cap, len);
        });
    }

    // This thread's last `create` or `execute` answer.
    static Status last_answer(std::vector<uint8_t>& buffer, std::string_view& out_json) {
        return detail::fetch_growing(buffer, out_json, lingara_embed_last_answer);
    }

    // This thread's last negative status, as English text that quotes no input.
    static Status last_error(std::vector<uint8_t>& buffer, std::string_view& out_json) {
        return detail::fetch_growing(buffer, out_json, lingara_embed_last_error);
    }

private:
    friend Client detail::create_with_abi(uint32_t, std::string_view, Status&);
    explicit Client(LingaraEmbed* handle) : handle_(handle) {}

    void reset() {
        if (handle_ != nullptr) {
            lingara_embed_free(std::exchange(handle_, nullptr));
        }
    }

    LingaraEmbed* handle_ = nullptr;
};

namespace detail {

// `create` with the library's ABI passed in, so a test can hand it a foreign
// value: a refusal returns before lingara_embed_new is reached.
inline Client create_with_abi(uint32_t library, std::string_view config_json, Status& out) {
    if (!abi_compatible(library, LINGARA_EMBED_ABI_VERSION_MAJOR, LINGARA_EMBED_ABI_VERSION_MINOR)) {
        out = LINGARA_EMBED_ERR_ABI_MISMATCH;
        return Client();
    }
    LingaraEmbed* handle = nullptr;
    out = lingara_embed_new(LINGARA_EMBED_ABI_VERSION_MAJOR, bytes(config_json), config_json.size(), &handle);
    return out == LINGARA_EMBED_OK ? Client(handle) : Client();
}

}  // namespace detail

}  // namespace lingara_embed

#endif  // LINGARA_EMBED_HPP
