#include <cstdio>
#include <string>

#include "lingara_embed.hpp"

/* 8.10.26q AC4 */
int main() {
    const unsigned packaged = lingara_embed::Client::abi_version() >> 16;
    std::printf("lingara-embed %s, ABI major %u (header %u)\n",
                std::string(lingara_embed::Client::library_version()).c_str(), packaged,
                static_cast<unsigned>(LINGARA_EMBED_ABI_VERSION_MAJOR));
    return packaged == LINGARA_EMBED_ABI_VERSION_MAJOR ? 0 : 1;
}
