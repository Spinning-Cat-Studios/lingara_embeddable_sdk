#include <stdio.h>
#include <string.h>

#include "lingara_embed.h"

/* 8.10.26q AC7 */
int main(void) {
    const char *version = lingara_embed_version();
    unsigned major = (unsigned)(lingara_embed_abi_version() >> 16);
    printf("lingara_embed_version() = %s (want %s), ABI major %u\n", version, LINGARA_EXPECTED_VERSION, major);
    if (major != LINGARA_EMBED_ABI_VERSION_MAJOR) {
        return 1;
    }
    return strcmp(version, LINGARA_EXPECTED_VERSION) == 0 ? 0 : 1;
}
