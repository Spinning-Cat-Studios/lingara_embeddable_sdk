// K1's JSON into the plugin's USTRUCTs (ADR 1.10.26ac D5): the event half,
// one function per K1 event `type`, and the answer half (CommandOutcome,
// EmbedError, EmbedStatus). Hand-written over FJsonObject, never
// FJsonObjectConverter: K1's snake_case keys do not match PascalCase
// properties, and the converter would miss them silently.
#pragma once

#include "CoreMinimal.h"
#include "LingaraEventTypes.h"

enum class ELingaraDecodedType : uint8
{
	TokenNeeded, Stream, Lingara, WorldSent, Practice, NpcTurn, Error, Log, Closed, Unknown
};

// One polled event. Only the member its Type names is filled.
struct FLingaraDecodedEvent
{
	ELingaraDecodedType Type = ELingaraDecodedType::Unknown;
	FString TypeName;
	FLingaraTokenNeeded TokenNeeded;
	FLingaraStreamChange Stream;
	FLingaraEvent Lingara;
	FLingaraWorldSent WorldSent;
	FLingaraPracticeUpdate Practice;
	FLingaraNpcTurn NpcTurn;
	FLingaraEmbedError Error;
	ELingaraLogLevel LogLevel = ELingaraLogLevel::Info;
	FString LogTarget;
	FString LogMessage;
	ELingaraCloseReason CloseReason = ELingaraCloseReason::Unknown;
};

enum class ELingaraOutcomeType : uint8 { Done, Id, Practice, Dialogue, Unknown };

// K1's CommandOutcome.
struct FLingaraOutcome
{
	ELingaraOutcomeType Type = ELingaraOutcomeType::Unknown;
	int64 Id = 0;
	bool bHasSnapshot = false;
	FLingaraPracticeSnapshot PracticeSnapshot;
	FLingaraDialogueSnapshot DialogueSnapshot;
};

namespace LingaraEventDecoder
{
	// True with Type Unknown for a `type` this plugin does not know (the
	// caller routes it to OnUnknownEvent). False when the text is not a JSON
	// object, or a known type's body does not decode; TypeName is still set.
	LINGARAEMBED_API bool DecodeEvent(const FString& Json, FLingaraDecodedEvent& Out);

	LINGARAEMBED_API bool DecodeOutcome(const FString& Json, FLingaraOutcome& Out);
	LINGARAEMBED_API bool DecodeError(const FString& Json, FLingaraEmbedError& Out);
	LINGARAEMBED_API bool DecodeStatus(const FString& Json, FLingaraEmbedStatus& Out);
}
