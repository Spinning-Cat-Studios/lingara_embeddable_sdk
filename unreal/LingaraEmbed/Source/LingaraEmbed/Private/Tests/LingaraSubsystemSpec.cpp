// The subsystem and the async actions, offline (ADR 1.10.26ac AC14): the
// drain cap and order, ProvideToken, Lingara Say's filtering and its Closed
// failure, the Win64 load failure, Deinitialize's bound and the user-agent
// suffix. Run only by `make unreal-smoke`, never by public CI.
#include "Misc/AutomationTest.h"

#if WITH_DEV_AUTOMATION_TESTS

#include "Engine/Engine.h"
#include "Engine/GameInstance.h"
#include "HAL/PlatformProcess.h"
#include "LingaraEmbedModule.h"
#include "LingaraEmbedSubsystem.h"
#include "LingaraSayAsyncAction.h"
#include "LingaraSpecSupport.h"

namespace
{
	FLingaraEmbedConfig Offline()
	{
		FLingaraEmbedConfig Config;
		Config.bOverride_StreamOnStart = true;
		Config.bStreamOnStart = false;
		return Config;
	}

	FLingaraDialogueConfig Mei()
	{
		FLingaraDialogueConfig Config;
		Config.Npc.Name = TEXT("Mei");
		Config.SourceLang = TEXT("en");
		Config.TargetLang = TEXT("zh");
		Config.Level = 2;
		return Config;
	}

	// A dialogue_say waits for its token on the core's I/O thread; give it
	// the time K3's suites give it before failing the need.
	void LetWaitersRegister() { FPlatformProcess::Sleep(0.3f); }
}

BEGIN_DEFINE_SPEC(FLingaraSubsystemSpec, "LingaraEmbed.Subsystem",
	EAutomationTestFlags::EngineFilter | EAutomationTestFlags_ApplicationContextMask)
	UGameInstance* GameInstance = nullptr;
	ULingaraEmbedSubsystem* Subsystem = nullptr;
	ULingaraSpecListener* Listener = nullptr;
	ELingaraCallResult Result = ELingaraCallResult::Ok;
	FLingaraEmbedError Error;

	void StartOffline(int32 MaxEventsPerTick)
	{
		FLingaraEmbedConfig Config = Offline();
		Config.MaxEventsPerTick = MaxEventsPerTick;
		Subsystem->Start(Config, Result, Error);
		TestEqual(TEXT("Start"), Result, ELingaraCallResult::Ok);
	}

	int64 Say(const FString& Line)
	{
		const int64 Dialogue = Subsystem->DialogueOpen(Mei(), Result, Error);
		return Subsystem->DialogueSay(Dialogue, Line, Result, Error);
	}

	void FailTheNeed()
	{
		LetWaitersRegister();
		Subsystem->TokenUnavailable(TEXT("offline"), Result, Error);
		LetWaitersRegister();
	}
END_DEFINE_SPEC(FLingaraSubsystemSpec)

void FLingaraSubsystemSpec::Define()
{
	BeforeEach([this]()
	{
		GameInstance = NewObject<UGameInstance>(GEngine);
		GameInstance->InitializeStandalone();
		Subsystem = GameInstance->GetSubsystem<ULingaraEmbedSubsystem>();
		Listener = NewObject<ULingaraSpecListener>();
		Listener->AddToRoot();
		Subsystem->OnTokenNeeded.AddDynamic(Listener, &ULingaraSpecListener::TokenNeeded);
		Subsystem->OnError.AddDynamic(Listener, &ULingaraSpecListener::Error);
	});

	AfterEach([this]()
	{
		Listener->RemoveFromRoot();
		GameInstance->Shutdown();
	});

	// 1.10.26ac AC14
	It("the subsystem drains in order and frees on deinitialize", [this]()
	{
		StartOffline(2);
		TArray<int64> Requests = {Say(TEXT("a")), Say(TEXT("b")), Say(TEXT("c"))};
		FailTheNeed();
		Subsystem->Tick(0.f);
		TestEqual(TEXT("one tick takes at most MaxEventsPerTick"), Listener->Seen.Num(), 2);
		Subsystem->Tick(0.f);
		TestEqual(TEXT("the rest arrive on the next tick"), Listener->Seen.Num(), 4);
		TestEqual(TEXT("token_needed is served first"), Listener->Seen[0], FString(TEXT("token_needed")));
		TArray<int64> Answered;
		for (const FLingaraEmbedError& Failed : Listener->Errors) Answered.Add(Failed.Request);
		Answered.Sort();
		TestEqual(TEXT("every say is answered once"), Answered, Requests);

		const double Before = FPlatformTime::Seconds();
		GameInstance->Shutdown();
		TestTrue(TEXT("Deinitialize frees the client within 1 s"), FPlatformTime::Seconds() - Before < 1.5);
		TestEqual(TEXT("stopped"), Subsystem->GetState(), ELingaraEmbedState::Stopped);
		GameInstance = NewObject<UGameInstance>(GEngine);
		GameInstance->InitializeStandalone();
	});

	It("provide token clears needs token", [this]()
	{
		StartOffline(256);
		TestTrue(TEXT("needs a token at start"), Subsystem->GetStatus().bNeedsToken);
		Subsystem->ProvideTokenFor(TEXT("lgr_et_") + FString::ChrN(43, TEXT('a')), 900, Result, Error);
		TestEqual(TEXT("accepted"), Result, ELingaraCallResult::Ok);
		TestFalse(TEXT("no longer needs one"), Subsystem->GetStatus().bNeedsToken);
	});

	It("lingara say fails on its own error and ignores another's", [this]()
	{
		StartOffline(256);
		const int64 Dialogue = Subsystem->DialogueOpen(Mei(), Result, Error);
		ULingaraSayAsyncAction* Action = ULingaraSayAsyncAction::LingaraSay(GameInstance->GetWorld(), Dialogue, TEXT("ni hao"));
		Action->OnFailed.AddDynamic(Listener, &ULingaraSpecListener::SayFailed);
		Action->Activate();
		const int64 Other = Say(TEXT("another"));
		FailTheNeed();
		Subsystem->Drain(256);
		TestEqual(TEXT("one failure"), Listener->SayFailures.Num(), 1);
		TestEqual(TEXT("for its own request"), Listener->SayFailures[0].Request, Action->GetRequest());
		TestNotEqual(TEXT("not the other's"), Listener->SayFailures[0].Request, Other);
	});

	It("lingara say fails closed when stop runs with its line in flight", [this]()
	{
		StartOffline(256);
		const int64 Dialogue = Subsystem->DialogueOpen(Mei(), Result, Error);
		ULingaraSayAsyncAction* Action = ULingaraSayAsyncAction::LingaraSay(GameInstance->GetWorld(), Dialogue, TEXT("ni hao"));
		Action->OnFailed.AddDynamic(Listener, &ULingaraSpecListener::SayFailed);
		Action->Activate();
		Subsystem->Stop();
		TestEqual(TEXT("one failure"), Listener->SayFailures.Num(), 1);
		TestEqual(TEXT("kind Closed"), Listener->SayFailures[0].Kind, ELingaraErrorKind::Closed);
	});

	It("names itself lingara-unreal first", [this]()
	{
		TestTrue(TEXT("the suffix begins lingara-unreal/"), ULingaraEmbedSubsystem::UserAgentSuffix(TEXT("game/1.0")).StartsWith(TEXT("lingara-unreal/")));
		TestTrue(TEXT("and ends with the game's own"), ULingaraEmbedSubsystem::UserAgentSuffix(TEXT("game/1.0")).EndsWith(TEXT(" game/1.0")));
	});

#if PLATFORM_WINDOWS
	It("a library that does not load leaves the subsystem disabled without asserting", [this]()
	{
		FLingaraEmbedModule::TryLoadLibrary(FString());
		UGameInstance* Fresh = NewObject<UGameInstance>(GEngine);
		Fresh->InitializeStandalone();
		ULingaraEmbedSubsystem* Disabled = Fresh->GetSubsystem<ULingaraEmbedSubsystem>();
		Disabled->Start(Offline(), Result, Error);
		TestEqual(TEXT("refused"), Result, ELingaraCallResult::Refused);
		TestEqual(TEXT("disabled"), Disabled->GetState(), ELingaraEmbedState::Disabled);
		Fresh->Shutdown();
		// Restore the real result, so the forced failure never disables a later spec.
		TestTrue(TEXT("the real library loads again"), FLingaraEmbedModule::TryLoadLibrary(FLingaraEmbedModule::LibraryDirectory()));
	});
#endif
}

#endif
