// Readers for K1's nested shapes, shared by the event and answer halves of
// LingaraEventDecoder (ADR 1.10.26ac D5). Each returns false when a
// required key is missing or mistyped; unknown keys are ignored. Strings are
// already TCHAR (FJsonSerializer converted the UTF-8 once).
#pragma once

#include "CoreMinimal.h"
#include "Dom/JsonObject.h"
#include "LingaraEventTypes.h"

namespace LingaraJsonShapes
{
	using FObjectPtr = TSharedPtr<FJsonObject>;

	// K1's ids are at or below i64::MAX; the engine's Json module holds
	// numbers as double, which is exact for every id below 2^53.
	bool ReadId(const FObjectPtr& O, const TCHAR* Key, int64& Out);
	// A missing or null optional string reads as empty.
	FString OptString(const FObjectPtr& O, const TCHAR* Key);
	// A missing or null optional object reads as null.
	FObjectPtr OptObject(const FObjectPtr& O, const TCHAR* Key);

	bool ReadError(const FObjectPtr& O, FLingaraEmbedError& Out);
	bool ReadStreamState(const FObjectPtr& O, FLingaraStreamState& Out);
	bool ReadNpc(const FObjectPtr& O, FLingaraNpc& Out);
	bool ReadWindow(const FObjectPtr& O, TArray<FLingaraWindowEntry>& Out);
	bool ReadPracticeSnapshot(const FObjectPtr& O, FLingaraPracticeSnapshot& Out);
	bool ReadDialogueSnapshot(const FObjectPtr& O, FLingaraDialogueSnapshot& Out);
	bool ReadLingaraEvent(const FObjectPtr& O, FLingaraEvent& Out);
	bool ReadReaction(const FObjectPtr& O, FLingaraReaction& Out);
	bool ReadNpcTurnUpdate(const FObjectPtr& O, FLingaraNpcTurnUpdate& Out);

	ELingaraErrorKind ErrorKind(const FString& Word);
	ELingaraLogLevel LogLevel(const FString& Word);
	ELingaraCloseReason CloseReason(const FString& Word);
	ELingaraTokenReason TokenReason(const FString& Word);
}
