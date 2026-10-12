// Fetch Token From URL (ADR 1.10.26ac D6). The engine delivers the HTTP
// completion on the game thread, so ProvideToken runs where every other
// `execute` does.
#include "LingaraFetchTokenAsyncAction.h"

#include "Dom/JsonObject.h"
#include "Engine/Engine.h"
#include "Engine/GameInstance.h"
#include "HttpModule.h"
#include "Interfaces/IHttpResponse.h"
#include "LingaraEmbedLog.h"
#include "LingaraEmbedSubsystem.h"
#include "Serialization/JsonReader.h"
#include "Serialization/JsonSerializer.h"

ULingaraFetchTokenAsyncAction* ULingaraFetchTokenAsyncAction::FetchTokenFromUrl(UObject* WorldContextObject, const FString& Url, const TArray<FLingaraHeader>& Headers)
{
	ULingaraFetchTokenAsyncAction* Action = NewObject<ULingaraFetchTokenAsyncAction>();
	const UWorld* World = GEngine ? GEngine->GetWorldFromContextObject(WorldContextObject, EGetWorldErrorMode::LogAndReturnNull) : nullptr;
	const UGameInstance* GameInstance = World ? World->GetGameInstance() : nullptr;
	Action->Subsystem = GameInstance ? GameInstance->GetSubsystem<ULingaraEmbedSubsystem>() : nullptr;
	Action->Url = Url;
	Action->Headers = Headers;
	Action->RegisterWithGameInstance(WorldContextObject);
	return Action;
}

void ULingaraFetchTokenAsyncAction::Activate()
{
	const TSharedRef<IHttpRequest, ESPMode::ThreadSafe> Request = FHttpModule::Get().CreateRequest();
	Request->SetVerb(TEXT("POST"));
	Request->SetURL(Url);
	Request->SetHeader(TEXT("Content-Type"), TEXT("application/json"));
	for (const FLingaraHeader& Header : Headers)
	{
		Request->SetHeader(Header.Name, Header.Value);
	}
	Request->SetContentAsString(TEXT("{}"));
	Request->OnProcessRequestComplete().BindUObject(this, &ULingaraFetchTokenAsyncAction::HandleResponse);
	if (!Request->ProcessRequest())
	{
		Unavailable(TEXT("the token request could not be sent"));
	}
}

bool ULingaraFetchTokenAsyncAction::ReadHandoff(const FString& Body, FString& Token, FString& ExpiresAt)
{
	TSharedPtr<FJsonObject> Object;
	const TSharedRef<TJsonReader<TCHAR>> Reader = TJsonReaderFactory<TCHAR>::Create(Body);
	return FJsonSerializer::Deserialize(Reader, Object) && Object.IsValid()
		&& Object->TryGetStringField(TEXT("token"), Token) && Object->TryGetStringField(TEXT("expires_at"), ExpiresAt);
}

void ULingaraFetchTokenAsyncAction::HandleResponse(FHttpRequestPtr Request, FHttpResponsePtr Response, bool bConnected)
{
	FString Token;
	FString ExpiresAt;
	if (!bConnected || !Response.IsValid())
	{
		Unavailable(TEXT("the token endpoint could not be reached"));
	}
	else if (!EHttpResponseCodes::IsOk(Response->GetResponseCode()))
	{
		Unavailable(FString::Printf(TEXT("the token endpoint answered %d"), Response->GetResponseCode()));
	}
	else if (!ReadHandoff(Response->GetContentAsString(), Token, ExpiresAt))
	{
		Unavailable(TEXT("the token endpoint's body has no token and expires_at"));
	}
	else if (Subsystem != nullptr)
	{
		ELingaraCallResult Result;
		FLingaraEmbedError Error;
		Subsystem->ProvideToken(Token, ExpiresAt, Result, Error);
		if (Result == ELingaraCallResult::Ok)
		{
			OnProvided.Broadcast();
		}
		else
		{
			OnUnavailable.Broadcast(Error.Message);
		}
	}
	SetReadyToDestroy();
}

void ULingaraFetchTokenAsyncAction::Unavailable(const FString& Reason)
{
	UE_LOG(LogLingaraEmbed, Warning, TEXT("Fetch Token From URL: %s"), *Reason);
	if (Subsystem != nullptr)
	{
		ELingaraCallResult Result;
		FLingaraEmbedError Error;
		Subsystem->TokenUnavailable(Reason, Result, Error);
	}
	OnUnavailable.Broadcast(Reason);
}
