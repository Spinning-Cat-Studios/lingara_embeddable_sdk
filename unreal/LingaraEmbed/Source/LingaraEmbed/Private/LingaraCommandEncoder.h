// K1's Command JSON, one encode function per command `type` (ADR 1.10.26ac
// D5), plus the EmbedConfig the subsystem starts with. Written with
// TJsonWriter<TCHAR, TCondensedJsonPrintPolicy>, and converted to UTF-8 only
// at `execute`. Ids are written as int64, so none passes through a double.
#pragma once

#include "CoreMinimal.h"
#include "LingaraEmbedTypes.h"

namespace LingaraCommandEncoder
{
	// K1's EmbedConfig: a field left unset is omitted, so K1's default applies (D7).
	LINGARAEMBED_API FString Config(const FLingaraEmbedConfig& Config, const FString& UserAgentSuffix);

	LINGARAEMBED_API FString ProvideToken(const FString& Token, const FString& ExpiresAt);
	LINGARAEMBED_API FString ProvideTokenFor(const FString& Token, uint32 ExpiresInSecs);
	LINGARAEMBED_API FString TokenUnavailable(const FString& Reason);
	LINGARAEMBED_API FString SetTokenEndpoint(const FString& Url, const TArray<FLingaraHeader>& Headers);
	LINGARAEMBED_API FString StreamPause();
	LINGARAEMBED_API FString StreamResume();
	LINGARAEMBED_API FString SendContext(const FLingaraWorldContext& Context);
	LINGARAEMBED_API FString PracticeStart(const FLingaraPracticeSource& Source);
	LINGARAEMBED_API FString PracticeSnapshot(int64 Session);
	LINGARAEMBED_API FString PracticeAnswer(int64 Session, const FString& Answer);
	LINGARAEMBED_API FString PracticeChoose(int64 Session, uint32 Option);
	LINGARAEMBED_API FString PracticeReveal(int64 Session);
	LINGARAEMBED_API FString PracticeSelfGrade(int64 Session, bool bCorrect);
	LINGARAEMBED_API FString PracticeNext(int64 Session);
	LINGARAEMBED_API FString PracticeFinish(int64 Session);
	LINGARAEMBED_API FString DialogueOpen(const FLingaraDialogueConfig& Config);
	LINGARAEMBED_API FString DialogueSay(int64 Dialogue, const FString& Line);
	LINGARAEMBED_API FString DialogueSetLevel(int64 Dialogue, uint8 Level);
	// An empty scene is K1's None: it clears the scene.
	LINGARAEMBED_API FString DialogueSetScene(int64 Dialogue, const FString& Scene);
	LINGARAEMBED_API FString DialogueCancel(int64 Dialogue);
	LINGARAEMBED_API FString DialogueClose(int64 Dialogue);
	LINGARAEMBED_API FString DialogueSnapshot(int64 Dialogue);

	// A graph dialogue (ADR 28.10.26f): the IR as text, and K1's GraphSession
	// as JSON text. K2 gives both a typed surface; until then they pass as written.
	LINGARAEMBED_API FString DialogueLoad(const FString& IrJson, const FString& SessionJson);
	// A dialogue saved on Lingara, by its store id (ADR 28.10.26i): DialogueLoaded follows.
	LINGARAEMBED_API FString DialogueLoadStored(const FString& DialogueId, const FString& SessionJson);
	// One per fact type, so a literal never picks the wrong overload.
	LINGARAEMBED_API FString DialogueSetFactBool(int64 Dialogue, const FString& Key, bool bValue);
	LINGARAEMBED_API FString DialogueSetFactNumber(int64 Dialogue, const FString& Key, double Value);
	LINGARAEMBED_API FString DialogueSetFactText(int64 Dialogue, const FString& Key, const FString& Value);
	LINGARAEMBED_API FString DialogueValidate(const FString& IrJson);
}
