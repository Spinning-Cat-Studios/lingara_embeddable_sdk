// The subsystem's native half, kept out of its public header so a game that
// includes LingaraEmbedSubsystem.h sees no std:: and no ABI types. One
// client, one poll buffer and one answer buffer per subsystem (D5).
#pragma once

#include "CoreMinimal.h"

THIRD_PARTY_INCLUDES_START
#include "lingara_embed.hpp"
THIRD_PARTY_INCLUDES_END

struct FLingaraEmbedNative
{
	lingara_embed::Client Client;
	std::vector<uint8_t> PollBuffer = std::vector<uint8_t>(16 * 1024);
	std::vector<uint8_t> AnswerBuffer = std::vector<uint8_t>(4 * 1024);
};

namespace LingaraNative
{
	// The ABI moves UTF-8; the engine's strings are TCHAR.
	inline FString ToFString(std::string_view Utf8)
	{
		return FString(FUTF8ToTCHAR(Utf8.data(), static_cast<int32>(Utf8.size())));
	}

	// This thread's last negative status, as the ABI's English text.
	inline FString LastErrorText()
	{
		std::vector<uint8_t> Buffer;
		std::string_view Text;
		return lingara_embed::Client::last_error(Buffer, Text) == LINGARA_EMBED_OK ? ToFString(Text) : FString();
	}
}
