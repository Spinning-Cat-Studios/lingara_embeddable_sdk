// The Unreal C++ snippet's actor (ADR 1.10.26ac D9). UnrealHeaderTool reads
// only headers, and a dynamic delegate binds UFUNCTIONs, not lambdas, so the
// class and its handlers are declared here; LingaraEmbedSnippet.cpp holds
// the code the docs show. Drop the actor in a level and set TokenUrl.
#pragma once

#include "CoreMinimal.h"
#include "GameFramework/Actor.h"
#include "Interfaces/IHttpRequest.h"
#include "LingaraEmbedSubsystem.h"

#include "LingaraEmbedSnippetActor.generated.h"

UCLASS()
class ALingaraEmbedSnippetActor : public AActor
{
	GENERATED_BODY()

public:
	// Your own server's endpoint: it mints a native embed token (no origin)
	// and returns { token, expires_at }. tools/demo-token-server while you develop.
	UPROPERTY(EditAnywhere, Category = "Lingara")
	FString TokenUrl = TEXT("http://127.0.0.1:8787/token");

	UPROPERTY(EditAnywhere, Category = "Lingara")
	FString PlanId;

	// The NPC's reply so far: show it in your text widget.
	UPROPERTY(BlueprintReadOnly, Category = "Lingara")
	FString Reply;

	void Say(const FString& Line);
	void StartPractice(bool bReport);
	void Answer(int32 Choice, const FString& Typed);
	void Next();

protected:
	virtual void BeginPlay() override;

private:
	UFUNCTION() void HandleTokenNeeded(const FLingaraTokenNeeded& Event);
	UFUNCTION() void HandleNpcTurn(const FLingaraNpcTurn& Turn);
	UFUNCTION() void HandlePractice(const FLingaraPracticeUpdate& Update);
	UFUNCTION() void HandleError(const FLingaraEmbedError& Error);
	UFUNCTION() void HandleUpdateAvailable(const FLingaraUpdateAvailable& Update);
	void HandleTokenResponse(FHttpRequestPtr Request, FHttpResponsePtr Response, bool bConnected);
	ULingaraEmbedSubsystem* Lingara() const;

	int64 Dialogue = 0;
	int64 Session = 0;
	FLingaraPracticeSnapshot Snapshot;
};
