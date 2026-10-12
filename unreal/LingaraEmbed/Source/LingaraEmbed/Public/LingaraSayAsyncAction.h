// **Lingara Say**: the node a Blueprint-only dialogue needs (ADR 1.10.26ac
// D6). It calls DialogueSay, keeps the request id, and listens for that
// request's npc_turn updates and errors only. It ends on the first of
// OnFinished or OnFailed, and fails with kind Closed if the subsystem stops
// running first, so no node waits on a client that is gone.
#pragma once

#include "CoreMinimal.h"
#include "Kismet/BlueprintAsyncActionBase.h"
#include "LingaraEventTypes.h"

#include "LingaraSayAsyncAction.generated.h"

class ULingaraEmbedSubsystem;

DECLARE_DYNAMIC_MULTICAST_DELEGATE_OneParam(FLingaraSayText, const FString&, Text);
DECLARE_DYNAMIC_MULTICAST_DELEGATE_TwoParams(FLingaraSayNotice, const FString&, Code, const FString&, Message);
DECLARE_DYNAMIC_MULTICAST_DELEGATE_OneParam(FLingaraSayFailed, const FLingaraEmbedError&, Error);

UCLASS()
class LINGARAEMBED_API ULingaraSayAsyncAction : public UBlueprintAsyncActionBase
{
	GENERATED_BODY()

public:
	UFUNCTION(BlueprintCallable, Category = "Lingara", meta = (BlueprintInternalUseOnly = "true", WorldContext = "WorldContextObject", DisplayName = "Lingara Say"))
	static ULingaraSayAsyncAction* LingaraSay(UObject* WorldContextObject, int64 Dialogue, const FString& Line);

	UPROPERTY(BlueprintAssignable) FLingaraSayText OnFragment;
	UPROPERTY(BlueprintAssignable) FLingaraSayNotice OnNotice;
	UPROPERTY(BlueprintAssignable) FLingaraSayText OnFinished;
	UPROPERTY(BlueprintAssignable) FLingaraSayFailed OnFailed;

	virtual void Activate() override;

	// The request this node listens for; 0 until Activate has said the line.
	int64 GetRequest() const { return Request; }

private:
	UFUNCTION() void HandleNpcTurn(const FLingaraNpcTurn& Turn);
	UFUNCTION() void HandleError(const FLingaraEmbedError& Error);
	UFUNCTION() void HandleStateChanged(ELingaraEmbedState State, const FString& Reason);

	void Fail(const FLingaraEmbedError& Error);
	void End();

	UPROPERTY() TObjectPtr<ULingaraEmbedSubsystem> Subsystem;
	int64 Dialogue = 0;
	FString Line;
	int64 Request = 0;
	bool bEnded = false;
};
