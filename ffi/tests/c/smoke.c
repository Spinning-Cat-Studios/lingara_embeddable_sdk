/* The ABI from C (ADR 1.10.26aa D12, AC12): this file includes only
 * lingara_embed.h, links the staticlib, and runs the offline lifecycle. It
 * is compiled at -std=c99 -Wall -Werror -pedantic (cc) or /std:c11 /W4 /WX
 * (MSVC), so a header that is not C fails here and never in a game. */
#include <stdio.h>
#include <string.h>

#include "lingara_embed.h"

static unsigned char buf[16384];

static int fail(const char *step, int32_t status) {
    size_t len = 0;
    char text[512];
    if (lingara_embed_last_error((uint8_t *)text, sizeof text - 1, &len) == LINGARA_EMBED_OK) {
        text[len] = '\0';
    } else {
        text[0] = '\0';
    }
    fprintf(stderr, "smoke: %s returned %d: %s\n", step, (int)status, text);
    return 1;
}

/* buf holds `len` bytes of JSON: does it contain `needle`? */
static int has(size_t len, const char *needle) {
    buf[len < sizeof buf ? len : sizeof buf - 1] = '\0';
    return strstr((const char *)buf, needle) != NULL;
}

static int32_t execute(LingaraEmbed *handle, const char *command, size_t *len) {
    return lingara_embed_execute(handle, (const uint8_t *)command, strlen(command), buf, sizeof buf - 1, len);
}

int main(void) {
    static const char config[] = "{\"base_url\": \"http://127.0.0.1:9\", \"stream_on_start\": false}";
    static const char dialogue[] =
        "{\"type\": \"dialogue_open\", \"config\": {\"npc\": {\"name\": \"Mei\"}, \"source_lang\": \"en\", \"target_lang\": \"zh\", \"level\": 2}}";
    static const char practice[] =
        "{\"type\": \"practice_start\", \"source\": {\"type\": \"plan\", \"plan_id\": \"8f0e2c1a-0000-4000-8000-000000000001\", \"report\": false}}";
    LingaraEmbed *handle = NULL;
    size_t len = 0;
    int32_t status;
    int polls;

    if ((lingara_embed_abi_version() >> 16) != LINGARA_EMBED_ABI_VERSION_MAJOR) {
        return fail("abi_version", -1);
    }
    status = lingara_embed_new(LINGARA_EMBED_ABI_VERSION_MAJOR, (const uint8_t *)config, strlen(config), &handle);
    if (status != LINGARA_EMBED_OK || handle == NULL) {
        return fail("new", status);
    }
    status = execute(handle, dialogue, &len);
    if (status != LINGARA_EMBED_OK || !has(len, "\"type\":\"id\"")) {
        return fail("execute dialogue_open", status);
    }
    status = execute(handle, practice, &len);
    if (status != LINGARA_EMBED_OK || !has(len, "\"type\":\"id\"")) {
        return fail("execute practice_start", status);
    }
    for (polls = 0; polls < 1000; polls++) {
        status = lingara_embed_poll(handle, buf, sizeof buf - 1, &len);
        if (status == LINGARA_EMBED_OK && has(len, "\"type\":\"token_needed\"")) {
            break;
        }
        if (status < 0) {
            return fail("poll", status);
        }
    }
    if (polls == 1000) {
        return fail("poll until token_needed", status);
    }
    status = lingara_embed_status(handle, buf, sizeof buf - 1, &len);
    if (status != LINGARA_EMBED_OK || !has(len, "\"needs_token\":true")) {
        return fail("status", status);
    }
    lingara_embed_free(handle);
    printf("smoke: lingara-embed %s, ABI %u.%u: ok\n", lingara_embed_version(), (unsigned)LINGARA_EMBED_ABI_VERSION_MAJOR,
           (unsigned)LINGARA_EMBED_ABI_VERSION_MINOR);
    return 0;
}
