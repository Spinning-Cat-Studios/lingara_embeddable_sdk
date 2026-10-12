// What the automation specs share (ADR 1.10.26ac D10): K1's
// conformance/vectors/json-shape.json, and a listener whose UFUNCTIONs the
// subsystem's dynamic delegates can call. Compiled into the Runtime module
// but used only by the specs, which WITH_DEV_AUTOMATION_TESTS compiles out
// of a shipping build.
#pragma once

#include "CoreMinimal.h"
#include "Dom/JsonObject.h"
#include "LingaraEventTypes.h"
#include "UObject/Object.h"

#include "LingaraSpecSupport.generated.h"

UCLASS()
class ULingaraSpecListener : public UObject
{
	GENERATED_BODY()

public:
	TArray<FString> Seen;
	TArray<FLingaraEmbedError> Errors;
	TArray<FLingaraEmbedError> SayFailures;
	TArray<FString> UnknownTypes;

	UFUNCTION() void TokenNeeded(const FLingaraTokenNeeded& Event) { Seen.Add(TEXT("token_needed")); }
	UFUNCTION() void Error(const FLingaraEmbedError& Event) { Seen.Add(TEXT("error")); Errors.Add(Event); }
	UFUNCTION() void SayFailed(const FLingaraEmbedError& Event) { SayFailures.Add(Event); }
	UFUNCTION() void Unknown(const FString& Type, const FString& RawJson) { UnknownTypes.Add(Type); }
};

namespace LingaraSpec
{
	// LINGARA_JSON_SHAPE when smoke.sh sets it; else the repository beside the plugin.
	TSharedPtr<FJsonObject> LoadJsonShape(FString& OutPath);

	// json-shape.json's `tagged.<Union>` examples.
	TArray<TSharedPtr<FJsonObject>> Examples(const TSharedPtr<FJsonObject>& Shape, const TCHAR* Union);

	FString Serialize(const TSharedPtr<FJsonObject>& Object);
}
