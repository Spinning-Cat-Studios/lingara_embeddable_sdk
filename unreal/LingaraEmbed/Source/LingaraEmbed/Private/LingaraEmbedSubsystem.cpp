// Tick, drain, broadcast, lifecycle, status mapping and the user-agent
// suffix (ADR 1.10.26ac D5). The command methods are in
// LingaraEmbedCommands.cpp.
#include "LingaraEmbedSubsystem.h"

#include "Interfaces/IPluginManager.h"
#include "LingaraCommandEncoder.h"
#include "LingaraEmbedLog.h"
#include "LingaraEmbedModule.h"
#include "LingaraEmbedNative.h"
#include "LingaraEmbedSettings.h"
#include "LingaraEventDecoder.h"
#include "Misc/EngineVersion.h"

namespace
{
	void LogFromCore(ELingaraLogLevel Level, const FString& Target, const FString& Message)
	{
		switch (Level)
		{
		case ELingaraLogLevel::Error: UE_LOG(LogLingaraEmbed, Error, TEXT("%s: %s"), *Target, *Message); break;
		case ELingaraLogLevel::Warn: UE_LOG(LogLingaraEmbed, Warning, TEXT("%s: %s"), *Target, *Message); break;
		case ELingaraLogLevel::Debug: UE_LOG(LogLingaraEmbed, Verbose, TEXT("%s: %s"), *Target, *Message); break;
		case ELingaraLogLevel::Trace: UE_LOG(LogLingaraEmbed, VeryVerbose, TEXT("%s: %s"), *Target, *Message); break;
		default: UE_LOG(LogLingaraEmbed, Log, TEXT("%s: %s"), *Target, *Message); break;
		}
	}

	FLingaraEmbedError MakeError(ELingaraErrorKind Kind, const TCHAR* KindName, const FString& Message)
	{
		FLingaraEmbedError Error;
		Error.Kind = Kind;
		Error.KindName = KindName;
		Error.Message = Message;
		return Error;
	}
}

ULingaraEmbedSubsystem::ULingaraEmbedSubsystem() = default;

ULingaraEmbedSubsystem::~ULingaraEmbedSubsystem() = default;

void ULingaraEmbedSubsystem::Initialize(FSubsystemCollectionBase& Collection)
{
	Super::Initialize(Collection);
	const ULingaraEmbedSettings* Settings = GetDefault<ULingaraEmbedSettings>();
	if (Settings->bAutoStart)
	{
		ELingaraCallResult Result;
		FLingaraEmbedError Error;
		Start(Settings->ToConfig(), Result, Error);
	}
}

void ULingaraEmbedSubsystem::Deinitialize()
{
	Stop();
	Super::Deinitialize();
}

void ULingaraEmbedSubsystem::Start(const FLingaraEmbedConfig& Config, ELingaraCallResult& Result, FLingaraEmbedError& Error)
{
	if (State == ELingaraEmbedState::Running || State == ELingaraEmbedState::Disabled || !FLingaraEmbedModule::IsLibraryLoaded())
	{
		if (!FLingaraEmbedModule::IsLibraryLoaded())
		{
			SetState(ELingaraEmbedState::Disabled, FLingaraEmbedModule::LoadFailure());
		}
		const bool bRunning = State == ELingaraEmbedState::Running;
		Result = RefuseLocally(bRunning ? ELingaraErrorKind::InvalidState : ELingaraErrorKind::Closed,
			bRunning ? TEXT("Start: already running") : TEXT("Start: Lingara is disabled for this run"), Error);
		return;
	}
	const FString Json = LingaraCommandEncoder::Config(Config, UserAgentSuffix(Config.UserAgentSuffix));
	const FTCHARToUTF8 Utf8(*Json);
	lingara_embed::Status Status = 0;
	TUniquePtr<FLingaraEmbedNative> Next = MakeUnique<FLingaraEmbedNative>();
	Next->Client = lingara_embed::Client::create(std::string_view(Utf8.Get(), Utf8.Length()), Status);
	if (Status == LINGARA_EMBED_OK)
	{
		Native = MoveTemp(Next);
		MaxEventsPerTick = FMath::Max(1, Config.MaxEventsPerTick);
		SetState(ELingaraEmbedState::Running, FString());
		Result = ELingaraCallResult::Ok;
		return;
	}
	Result = ELingaraCallResult::Refused;
	if (Status == LINGARA_EMBED_REFUSED)
	{
		// K1 refused the config: the answer is this thread's last_answer, and the state stays Stopped.
		std::string_view Answer;
		lingara_embed::Client::last_answer(Next->AnswerBuffer, Answer);
		LingaraEventDecoder::DecodeError(LingaraNative::ToFString(Answer), Error);
		return;
	}
	Error = NativeFailure(Status, true);
}

void ULingaraEmbedSubsystem::Stop()
{
	if (State == ELingaraEmbedState::Running)
	{
		FreeClient();
		SetState(ELingaraEmbedState::Stopped, TEXT("stopped"));
	}
}

bool ULingaraEmbedSubsystem::IsTickable() const
{
	return Native.IsValid();
}

TStatId ULingaraEmbedSubsystem::GetStatId() const
{
	RETURN_QUICK_DECLARE_CYCLE_STAT(ULingaraEmbedSubsystem, STATGROUP_Tickables);
}

void ULingaraEmbedSubsystem::Tick(float DeltaTime)
{
	Drain(MaxEventsPerTick);
}

int32 ULingaraEmbedSubsystem::Drain(int32 Max)
{
	int32 Taken = 0;
	while (Taken < Max && Native.IsValid())
	{
		std::string_view Out;
		const lingara_embed::Status Status = Native->Client.poll(Native->PollBuffer, Out);
		if (Status == LINGARA_EMBED_EMPTY)
		{
			break;
		}
		if (Status != LINGARA_EMBED_OK)
		{
			OnError.Broadcast(NativeFailure(Status, false));
			break;
		}
		++Taken;
		if (!Dispatch(LingaraNative::ToFString(Out)))
		{
			break;
		}
	}
	return Taken;
}

// False when the event ends this drain: K1's `closed`.
bool ULingaraEmbedSubsystem::Dispatch(const FString& Json)
{
	FLingaraDecodedEvent Event;
	if (!LingaraEventDecoder::DecodeEvent(Json, Event))
	{
		UE_LOG(LogLingaraEmbed, Error, TEXT("an event of type '%s' did not decode"), *Event.TypeName);
		OnUnknownEvent.Broadcast(Event.TypeName, Json);
		return true;
	}
	if (Event.Type == ELingaraDecodedType::Closed)
	{
		// Free before broadcasting, so no handler can poll a freed handle.
		FreeClient();
		SetState(ELingaraEmbedState::Stopped, TEXT("closed"));
		OnClosed.Broadcast(Event.CloseReason);
		return false;
	}
	if (Event.Type == ELingaraDecodedType::Unknown)
	{
		OnUnknownEvent.Broadcast(Event.TypeName, Json);
		return true;
	}
	Broadcast(Event);
	return true;
}

void ULingaraEmbedSubsystem::Broadcast(const FLingaraDecodedEvent& Event)
{
	switch (Event.Type)
	{
	case ELingaraDecodedType::TokenNeeded: OnTokenNeeded.Broadcast(Event.TokenNeeded); break;
	case ELingaraDecodedType::Stream: OnStreamChanged.Broadcast(Event.Stream); break;
	case ELingaraDecodedType::Lingara: OnLingaraEvent.Broadcast(Event.Lingara); break;
	case ELingaraDecodedType::WorldSent: OnWorldSent.Broadcast(Event.WorldSent); break;
	case ELingaraDecodedType::Practice: OnPractice.Broadcast(Event.Practice); break;
	case ELingaraDecodedType::NpcTurn: OnNpcTurn.Broadcast(Event.NpcTurn); break;
	case ELingaraDecodedType::Error: OnError.Broadcast(Event.Error); break;
	case ELingaraDecodedType::Log: LogFromCore(Event.LogLevel, Event.LogTarget, Event.LogMessage); break;
	case ELingaraDecodedType::UpdateAvailable: OnUpdateAvailable.Broadcast(Event.Update); break;
	default: break;
	}
}

FLingaraEmbedStatus ULingaraEmbedSubsystem::GetStatus() const
{
	FLingaraEmbedStatus Status;
	Status.bClosed = !Native.IsValid();
	if (Native.IsValid())
	{
		std::vector<uint8_t> Buffer(512);
		std::string_view Out;
		if (Native->Client.status(Buffer, Out) == LINGARA_EMBED_OK)
		{
			LingaraEventDecoder::DecodeStatus(LingaraNative::ToFString(Out), Status);
		}
	}
	return Status;
}

// A negative status never reached the core (K3 D8's mapping, for C++).
FLingaraEmbedError ULingaraEmbedSubsystem::NativeFailure(int32 Status, bool bFromCreate)
{
	const uint32 Library = lingara_embed::Client::abi_version();
	const bool bOurRefusal = bFromCreate && Status == LINGARA_EMBED_ERR_ABI_MISMATCH
		&& !lingara_embed::abi_compatible(Library, LINGARA_EMBED_ABI_VERSION_MAJOR, LINGARA_EMBED_ABI_VERSION_MINOR);
	const FString Text = bOurRefusal ? FString(UTF8_TO_TCHAR(lingara_embed::abi_mismatch_message(Library).c_str())) : LingaraNative::LastErrorText();
	FLingaraEmbedError Error = Status == LINGARA_EMBED_ERR_ABI_MISMATCH ? MakeError(ELingaraErrorKind::Config, TEXT("config"), Text)
		: Status == LINGARA_EMBED_ERR_CLOSED ? MakeError(ELingaraErrorKind::Closed, TEXT("closed"), Text)
		: MakeError(ELingaraErrorKind::Internal, TEXT("internal"), Text);
	UE_LOG(LogLingaraEmbed, Error, TEXT("%s"), *Text);
	if (Status == LINGARA_EMBED_ERR_ABI_MISMATCH || Status == LINGARA_EMBED_ERR_CLOSED || Status == LINGARA_EMBED_ERR_PANIC)
	{
		FreeClient();
		SetState(ELingaraEmbedState::Disabled, Text);
	}
	return Error;
}

ELingaraCallResult ULingaraEmbedSubsystem::RefuseLocally(ELingaraErrorKind Kind, const FString& Message, FLingaraEmbedError& Error) const
{
	static const TCHAR* const Names[] = {
		TEXT("config"), TEXT("invalid_input"), TEXT("token_unavailable"), TEXT("token_rejected"), TEXT("forbidden"),
		TEXT("billing"), TEXT("not_found"), TEXT("rate_limited"), TEXT("unavailable"), TEXT("network"), TEXT("busy"),
		TEXT("invalid_state"), TEXT("unknown_id"), TEXT("closed"), TEXT("internal"), TEXT("internal")};
	Error = MakeError(Kind, Names[static_cast<uint8>(Kind)], Message);
	UE_LOG(LogLingaraEmbed, Warning, TEXT("%s"), *Message);
	return ELingaraCallResult::Refused;
}

void ULingaraEmbedSubsystem::FreeClient()
{
	// K3's `free` joins the I/O thread within 1 s, on this thread.
	Native.Reset();
}

void ULingaraEmbedSubsystem::SetState(ELingaraEmbedState Next, const FString& Reason)
{
	if (State == Next || State == ELingaraEmbedState::Disabled)
	{
		return;
	}
	State = Next;
	if (Next == ELingaraEmbedState::Disabled)
	{
		UE_LOG(LogLingaraEmbed, Error, TEXT("disabled: %s"), *Reason);
	}
	OnStateChanged.Broadcast(Next, Reason);
}

FString ULingaraEmbedSubsystem::UserAgentSuffix(const FString& GameSuffix)
{
	const TSharedPtr<IPlugin> Plugin = IPluginManager::Get().FindPlugin(TEXT("LingaraEmbed"));
	const FString Version = Plugin.IsValid() ? Plugin->GetDescriptor().VersionName : TEXT("0.0.0-dev");
	const FString Kit = FString::Printf(TEXT("lingara-unreal/%s unreal/%s"), *Version,
		*FEngineVersion::Current().ToString(EVersionComponent::Patch));
	return GameSuffix.IsEmpty() ? Kit : Kit + TEXT(" ") + GameSuffix;
}
