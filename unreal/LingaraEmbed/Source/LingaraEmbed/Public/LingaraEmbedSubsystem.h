// Get Game Instance Subsystem → Lingara Embed (ADR 1.10.26ac D5, D6).
//
// One client per game instance, so a conversation survives a level load.
// Everything runs on the game thread: Tick is the one poller, and every
// command method calls `execute` there, so the ABI's thread contract holds
// with no lock. The methods mirror K1's commands one for one; a refusal is
// the `Refused` exec pin with the core's error, never an exception.
#pragma once

#include "CoreMinimal.h"
#include "LingaraEventTypes.h"
#include "Subsystems/GameInstanceSubsystem.h"
#include "Tickable.h"

#include "LingaraEmbedSubsystem.generated.h"

struct FLingaraEmbedNative;
struct FLingaraDecodedEvent;
struct FLingaraOutcome;

DECLARE_DYNAMIC_MULTICAST_DELEGATE_OneParam(FOnLingaraTokenNeeded, const FLingaraTokenNeeded&, Event);
DECLARE_DYNAMIC_MULTICAST_DELEGATE_OneParam(FOnLingaraStreamChanged, const FLingaraStreamChange&, Event);
DECLARE_DYNAMIC_MULTICAST_DELEGATE_OneParam(FOnLingaraEvent, const FLingaraEvent&, Event);
DECLARE_DYNAMIC_MULTICAST_DELEGATE_OneParam(FOnLingaraWorldSent, const FLingaraWorldSent&, Event);
DECLARE_DYNAMIC_MULTICAST_DELEGATE_OneParam(FOnLingaraPractice, const FLingaraPracticeUpdate&, Event);
DECLARE_DYNAMIC_MULTICAST_DELEGATE_OneParam(FOnLingaraNpcTurn, const FLingaraNpcTurn&, Event);
DECLARE_DYNAMIC_MULTICAST_DELEGATE_OneParam(FOnLingaraError, const FLingaraEmbedError&, Error);
DECLARE_DYNAMIC_MULTICAST_DELEGATE_OneParam(FOnLingaraClosed, ELingaraCloseReason, Reason);
DECLARE_DYNAMIC_MULTICAST_DELEGATE_TwoParams(FOnLingaraStateChanged, ELingaraEmbedState, State, const FString&, Reason);
DECLARE_DYNAMIC_MULTICAST_DELEGATE_TwoParams(FOnLingaraUnknownEvent, const FString&, Type, const FString&, RawJson);

UCLASS()
class LINGARAEMBED_API ULingaraEmbedSubsystem : public UGameInstanceSubsystem, public FTickableGameObject
{
	GENERATED_BODY()

public:
	ULingaraEmbedSubsystem();
	virtual ~ULingaraEmbedSubsystem() override;

	virtual void Initialize(FSubsystemCollectionBase& Collection) override;
	virtual void Deinitialize() override;

	virtual void Tick(float DeltaTime) override;
	virtual ETickableTickType GetTickableTickType() const override { return ETickableTickType::Conditional; }
	virtual bool IsTickable() const override;
	// Pause menus still need token refreshes and a reply already streaming.
	virtual bool IsTickableWhenPaused() const override { return true; }
	virtual bool IsTickableInEditor() const override { return false; }
	virtual TStatId GetStatId() const override;

	// Lifecycle
	UFUNCTION(BlueprintCallable, Category = "Lingara", meta = (ExpandEnumAsExecs = "Result"))
	void Start(const FLingaraEmbedConfig& Config, ELingaraCallResult& Result, FLingaraEmbedError& Error);
	// Blocks up to 1 s while the core's I/O thread joins: stop at a loading screen.
	UFUNCTION(BlueprintCallable, Category = "Lingara")
	void Stop();

	// Token (K1 D4). ExpiresAt is the handoff body's `expires_at`, verbatim.
	UFUNCTION(BlueprintCallable, Category = "Lingara", meta = (ExpandEnumAsExecs = "Result"))
	void ProvideToken(const FString& Token, const FString& ExpiresAt, ELingaraCallResult& Result, FLingaraEmbedError& Error);
	UFUNCTION(BlueprintCallable, Category = "Lingara", meta = (ExpandEnumAsExecs = "Result"))
	void ProvideTokenFor(const FString& Token, int32 ExpiresInSecs, ELingaraCallResult& Result, FLingaraEmbedError& Error);
	UFUNCTION(BlueprintCallable, Category = "Lingara", meta = (ExpandEnumAsExecs = "Result"))
	void TokenUnavailable(const FString& Reason, ELingaraCallResult& Result, FLingaraEmbedError& Error);
	UFUNCTION(BlueprintCallable, Category = "Lingara", meta = (ExpandEnumAsExecs = "Result"))
	void SetTokenEndpoint(const FString& Url, const TArray<FLingaraHeader>& Headers, ELingaraCallResult& Result, FLingaraEmbedError& Error);

	// Stream and world (K1 D5, D6)
	UFUNCTION(BlueprintCallable, Category = "Lingara", meta = (ExpandEnumAsExecs = "Result"))
	void StreamPause(ELingaraCallResult& Result, FLingaraEmbedError& Error);
	UFUNCTION(BlueprintCallable, Category = "Lingara", meta = (ExpandEnumAsExecs = "Result"))
	void StreamResume(ELingaraCallResult& Result, FLingaraEmbedError& Error);
	UFUNCTION(BlueprintCallable, Category = "Lingara", meta = (ExpandEnumAsExecs = "Result"))
	int64 SendContext(const FLingaraWorldContext& Context, ELingaraCallResult& Result, FLingaraEmbedError& Error);

	// Practice (K1 D7)
	UFUNCTION(BlueprintCallable, Category = "Lingara", meta = (ExpandEnumAsExecs = "Result"))
	int64 PracticeStart(const FLingaraPracticeSource& Source, ELingaraCallResult& Result, FLingaraEmbedError& Error);
	UFUNCTION(BlueprintCallable, Category = "Lingara", meta = (ExpandEnumAsExecs = "Result"))
	bool GetPracticeSnapshot(int64 Session, FLingaraPracticeSnapshot& Snapshot, ELingaraCallResult& Result, FLingaraEmbedError& Error);
	UFUNCTION(BlueprintCallable, Category = "Lingara", meta = (ExpandEnumAsExecs = "Result"))
	FLingaraPracticeSnapshot PracticeAnswer(int64 Session, const FString& Answer, ELingaraCallResult& Result, FLingaraEmbedError& Error);
	UFUNCTION(BlueprintCallable, Category = "Lingara", meta = (ExpandEnumAsExecs = "Result"))
	FLingaraPracticeSnapshot PracticeChoose(int64 Session, int32 Option, ELingaraCallResult& Result, FLingaraEmbedError& Error);
	UFUNCTION(BlueprintCallable, Category = "Lingara", meta = (ExpandEnumAsExecs = "Result"))
	FLingaraPracticeSnapshot PracticeReveal(int64 Session, ELingaraCallResult& Result, FLingaraEmbedError& Error);
	UFUNCTION(BlueprintCallable, Category = "Lingara", meta = (ExpandEnumAsExecs = "Result"))
	FLingaraPracticeSnapshot PracticeSelfGrade(int64 Session, bool bCorrect, ELingaraCallResult& Result, FLingaraEmbedError& Error);
	UFUNCTION(BlueprintCallable, Category = "Lingara", meta = (ExpandEnumAsExecs = "Result"))
	FLingaraPracticeSnapshot PracticeNext(int64 Session, ELingaraCallResult& Result, FLingaraEmbedError& Error);
	UFUNCTION(BlueprintCallable, Category = "Lingara", meta = (ExpandEnumAsExecs = "Result"))
	FLingaraPracticeSnapshot PracticeFinish(int64 Session, ELingaraCallResult& Result, FLingaraEmbedError& Error);

	// Dialogue (K1 D8)
	UFUNCTION(BlueprintCallable, Category = "Lingara", meta = (ExpandEnumAsExecs = "Result"))
	int64 DialogueOpen(const FLingaraDialogueConfig& Config, ELingaraCallResult& Result, FLingaraEmbedError& Error);
	UFUNCTION(BlueprintCallable, Category = "Lingara", meta = (ExpandEnumAsExecs = "Result"))
	int64 DialogueSay(int64 Dialogue, const FString& Line, ELingaraCallResult& Result, FLingaraEmbedError& Error);
	UFUNCTION(BlueprintCallable, Category = "Lingara", meta = (ExpandEnumAsExecs = "Result"))
	void DialogueSetLevel(int64 Dialogue, uint8 Level, ELingaraCallResult& Result, FLingaraEmbedError& Error);
	// An empty scene clears it (K1's None).
	UFUNCTION(BlueprintCallable, Category = "Lingara", meta = (ExpandEnumAsExecs = "Result"))
	void DialogueSetScene(int64 Dialogue, const FString& Scene, ELingaraCallResult& Result, FLingaraEmbedError& Error);
	UFUNCTION(BlueprintCallable, Category = "Lingara", meta = (ExpandEnumAsExecs = "Result"))
	void DialogueCancel(int64 Dialogue, ELingaraCallResult& Result, FLingaraEmbedError& Error);
	UFUNCTION(BlueprintCallable, Category = "Lingara", meta = (ExpandEnumAsExecs = "Result"))
	void DialogueClose(int64 Dialogue, ELingaraCallResult& Result, FLingaraEmbedError& Error);
	UFUNCTION(BlueprintCallable, Category = "Lingara", meta = (ExpandEnumAsExecs = "Result"))
	bool GetDialogueSnapshot(int64 Dialogue, FLingaraDialogueSnapshot& Snapshot, ELingaraCallResult& Result, FLingaraEmbedError& Error);

	UFUNCTION(BlueprintPure, Category = "Lingara")
	ELingaraEmbedState GetState() const { return State; }
	// Read from the core on each call; the plugin keeps no copy (D5).
	UFUNCTION(BlueprintPure, Category = "Lingara")
	FLingaraEmbedStatus GetStatus() const;

	// One delegate per K1 event; `log` goes to LogLingaraEmbed instead.
	UPROPERTY(BlueprintAssignable, Category = "Lingara") FOnLingaraTokenNeeded OnTokenNeeded;
	UPROPERTY(BlueprintAssignable, Category = "Lingara") FOnLingaraStreamChanged OnStreamChanged;
	UPROPERTY(BlueprintAssignable, Category = "Lingara") FOnLingaraEvent OnLingaraEvent;
	UPROPERTY(BlueprintAssignable, Category = "Lingara") FOnLingaraWorldSent OnWorldSent;
	UPROPERTY(BlueprintAssignable, Category = "Lingara") FOnLingaraPractice OnPractice;
	UPROPERTY(BlueprintAssignable, Category = "Lingara") FOnLingaraNpcTurn OnNpcTurn;
	UPROPERTY(BlueprintAssignable, Category = "Lingara") FOnLingaraError OnError;
	UPROPERTY(BlueprintAssignable, Category = "Lingara") FOnLingaraClosed OnClosed;
	UPROPERTY(BlueprintAssignable, Category = "Lingara") FOnLingaraStateChanged OnStateChanged;
	UPROPERTY(BlueprintAssignable, Category = "Lingara") FOnLingaraUnknownEvent OnUnknownEvent;

	// Up to Max events, decoded and broadcast in queue order. Tick calls it
	// with MaxEventsPerTick; returns how many were taken.
	int32 Drain(int32 Max);

	// `lingara-unreal/<plugin> unreal/<engine>`, then the game's own suffix.
	static FString UserAgentSuffix(const FString& GameSuffix);

private:
	ELingaraCallResult Run(const FString& Command, FLingaraOutcome& Outcome, FLingaraEmbedError& Error);
	ELingaraCallResult RefuseLocally(ELingaraErrorKind Kind, const FString& Message, FLingaraEmbedError& Error) const;
	FLingaraEmbedError NativeFailure(int32 Status, bool bFromCreate);
	bool Dispatch(const FString& Json);
	void Broadcast(const FLingaraDecodedEvent& Event);
	void FreeClient();
	void SetState(ELingaraEmbedState Next, const FString& Reason);

	TUniquePtr<FLingaraEmbedNative> Native;
	ELingaraEmbedState State = ELingaraEmbedState::Stopped;
	int32 MaxEventsPerTick = 256;
};
