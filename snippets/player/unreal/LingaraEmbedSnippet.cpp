// Lingara from plain Unreal C++ (ADR 1.10.26ac D9): the subsystem, its
// delegates and the game's own token endpoint. The plugin installs by hand
// (unzip it into Plugins/), so there is no install file. Compiled by every
// `make unreal-smoke` inside unreal/smoke/.
#include "LingaraEmbedSnippetActor.h"

#include "Dom/JsonObject.h"
#include "Engine/GameInstance.h"
#include "HttpModule.h"
#include "Interfaces/IHttpResponse.h"
#include "Serialization/JsonReader.h"
#include "Serialization/JsonSerializer.h"

ULingaraEmbedSubsystem* ALingaraEmbedSnippetActor::Lingara() const
{
	return GetGameInstance()->GetSubsystem<ULingaraEmbedSubsystem>();
}

// lingara:begin poll
// The subsystem drains the core's queue from its own tick (at most
// MaxEventsPerTick a frame, from Project Settings): bind the events you use.
void ALingaraEmbedSnippetActor::BeginPlay()
{
	Super::BeginPlay();
	ULingaraEmbedSubsystem* Subsystem = Lingara();
	Subsystem->OnTokenNeeded.AddDynamic(this, &ALingaraEmbedSnippetActor::HandleTokenNeeded);
	Subsystem->OnNpcTurn.AddDynamic(this, &ALingaraEmbedSnippetActor::HandleNpcTurn);
	Subsystem->OnPractice.AddDynamic(this, &ALingaraEmbedSnippetActor::HandlePractice);
	Subsystem->OnError.AddDynamic(this, &ALingaraEmbedSnippetActor::HandleError);
	Subsystem->OnUpdateAvailable.AddDynamic(this, &ALingaraEmbedSnippetActor::HandleUpdateAvailable);

	FLingaraEmbedConfig Config;
	Config.MaxEventsPerTick = 256;
	// The embed version this build was registered against: your server is told it.
	Config.EmbedVersion = TEXT("1.1.0");
	ELingaraCallResult Result;
	FLingaraEmbedError Error;
	Subsystem->Start(Config, Result, Error);
}

void ALingaraEmbedSnippetActor::HandleError(const FLingaraEmbedError& Error)
{
	UE_LOG(LogTemp, Warning, TEXT("Lingara %s: %s"), *Error.KindName, *Error.Message);
}

// Once per session, for you: log it, never draw it for the player.
void ALingaraEmbedSnippetActor::HandleUpdateAvailable(const FLingaraUpdateAvailable& Update)
{
	UE_LOG(LogTemp, Warning, TEXT("Lingara update: latest %s, deprecated %d"), *Update.Latest, Update.bDeprecated);
}
// lingara:end

// lingara:begin tokenSource
// Your own server mints the token: never put a client secret in a game.
void ALingaraEmbedSnippetActor::HandleTokenNeeded(const FLingaraTokenNeeded& Event)
{
	const TSharedRef<IHttpRequest, ESPMode::ThreadSafe> Request = FHttpModule::Get().CreateRequest();
	Request->SetVerb(TEXT("POST"));
	Request->SetURL(TokenUrl);
	Request->SetHeader(TEXT("Content-Type"), TEXT("application/json"));
	Request->SetContentAsString(TEXT("{}"));
	Request->OnProcessRequestComplete().BindUObject(this, &ALingaraEmbedSnippetActor::HandleTokenResponse);
	Request->ProcessRequest();
}

void ALingaraEmbedSnippetActor::HandleTokenResponse(FHttpRequestPtr Request, FHttpResponsePtr Response, bool bConnected)
{
	ELingaraCallResult Result;
	FLingaraEmbedError Error;
	TSharedPtr<FJsonObject> Body;
	FString Token;
	FString ExpiresAt;
	const bool bOk = bConnected && Response.IsValid() && EHttpResponseCodes::IsOk(Response->GetResponseCode())
		&& FJsonSerializer::Deserialize(TJsonReaderFactory<TCHAR>::Create(Response->GetContentAsString()), Body)
		&& Body->TryGetStringField(TEXT("token"), Token) && Body->TryGetStringField(TEXT("expires_at"), ExpiresAt);
	if (bOk)
	{
		Lingara()->ProvideToken(Token, ExpiresAt, Result, Error);
	}
	else
	{
		// Fail the waiting calls now rather than at the core's timeout.
		Lingara()->TokenUnavailable(TEXT("the token endpoint failed"), Result, Error);
	}
}
// lingara:end

// lingara:begin dialogue
void ALingaraEmbedSnippetActor::Say(const FString& Line)
{
	ELingaraCallResult Result;
	FLingaraEmbedError Error;
	if (Dialogue == 0)
	{
		FLingaraDialogueConfig Config;
		Config.Npc.Name = TEXT("Mei");
		Config.Npc.Persona = TEXT("a tea seller at the night market");
		Config.SourceLang = TEXT("en");
		Config.TargetLang = TEXT("zh");
		Config.Level = 2;
		Dialogue = Lingara()->DialogueOpen(Config, Result, Error);
	}
	Reply.Reset();
	Lingara()->DialogueSay(Dialogue, Line, Result, Error);
}

// The reply streams back as npc_turn events: append each fragment.
void ALingaraEmbedSnippetActor::HandleNpcTurn(const FLingaraNpcTurn& Turn)
{
	if (Turn.Dialogue == Dialogue && Turn.Update.Type == ELingaraNpcTurnUpdateType::Fragment)
	{
		Reply += Turn.Update.Text;
	}
}
// lingara:end

// lingara:begin practice
// A demo plan reports nothing: start it with bReport = false.
void ALingaraEmbedSnippetActor::StartPractice(bool bReport)
{
	FLingaraPracticeSource Source;
	Source.Type = ELingaraPracticeSourceType::Plan;
	Source.PlanId = PlanId;
	Source.bReport = bReport;
	ELingaraCallResult Result;
	FLingaraEmbedError Error;
	Session = Lingara()->PracticeStart(Source, Result, Error);
}

void ALingaraEmbedSnippetActor::HandlePractice(const FLingaraPracticeUpdate& Update)
{
	if (Update.Session == Session)
	{
		Snapshot = Update.Snapshot;
	}
}

// Each answer returns the new snapshot at once: show its LastResult, then Next.
void ALingaraEmbedSnippetActor::Answer(int32 Choice, const FString& Typed)
{
	ELingaraCallResult Result;
	FLingaraEmbedError Error;
	Snapshot = Snapshot.Question.Options.Num() > 0 ? Lingara()->PracticeChoose(Session, Choice, Result, Error)
		: Lingara()->PracticeAnswer(Session, Typed, Result, Error);
}

void ALingaraEmbedSnippetActor::Next()
{
	ELingaraCallResult Result;
	FLingaraEmbedError Error;
	Snapshot = Lingara()->PracticeNext(Session, Result, Error);
}
// lingara:end
