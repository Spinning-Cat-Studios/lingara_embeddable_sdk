// **Fetch Token From URL** (ADR 1.10.26ac D6): POSTs `{}` to the game's own
// endpoint with the headers the game supplies (its session), reads exactly
// `token` and `expires_at` from the handoff body, and calls ProvideToken.
// Anything else (a transport failure, a non-2xx status, a body without both
// fields) calls TokenUnavailable, so the core fails its waiters at once. It
// holds no secret, knows no Lingara route, and never logs the body.
#pragma once

#include "CoreMinimal.h"
#include "Interfaces/IHttpRequest.h"
#include "Kismet/BlueprintAsyncActionBase.h"
#include "LingaraEmbedTypes.h"

#include "LingaraFetchTokenAsyncAction.generated.h"

class ULingaraEmbedSubsystem;

DECLARE_DYNAMIC_MULTICAST_DELEGATE(FLingaraTokenProvided);
DECLARE_DYNAMIC_MULTICAST_DELEGATE_OneParam(FLingaraTokenUnavailable, const FString&, Reason);

UCLASS()
class LINGARAEMBED_API ULingaraFetchTokenAsyncAction : public UBlueprintAsyncActionBase
{
	GENERATED_BODY()

public:
	UFUNCTION(BlueprintCallable, Category = "Lingara", meta = (BlueprintInternalUseOnly = "true", WorldContext = "WorldContextObject", DisplayName = "Fetch Token From URL"))
	static ULingaraFetchTokenAsyncAction* FetchTokenFromUrl(UObject* WorldContextObject, const FString& Url, const TArray<FLingaraHeader>& Headers);

	UPROPERTY(BlueprintAssignable) FLingaraTokenProvided OnProvided;
	UPROPERTY(BlueprintAssignable) FLingaraTokenUnavailable OnUnavailable;

	virtual void Activate() override;

	// The handoff body's two fields, or false. Any other field is ignored (V1 ES6).
	static bool ReadHandoff(const FString& Body, FString& Token, FString& ExpiresAt);

	// The handoff's optional `update_available` as compact JSON, or empty (ADR
	// 30.10.26p). Passed on verbatim: the core drops a malformed one.
	static FString ReadUpdate(const FString& Body);

private:
	void HandleResponse(FHttpRequestPtr Request, FHttpResponsePtr Response, bool bConnected);
	void Unavailable(const FString& Reason);

	UPROPERTY() TObjectPtr<ULingaraEmbedSubsystem> Subsystem;
	FString Url;
	TArray<FLingaraHeader> Headers;
};
