// The subsystem's 22 command methods, one per K1 command (ADR 1.10.26ac D6),
// each encode → `execute` → decode. `OK` is the decoded outcome, `REFUSED`
// is K1's EmbedError on the Refused pin, and a negative status (the call
// never reached the core) is mapped once, in NativeFailure.
#include "LingaraCommandEncoder.h"
#include "LingaraEmbedLog.h"
#include "LingaraEmbedNative.h"
#include "LingaraEmbedSubsystem.h"
#include "LingaraEventDecoder.h"

ELingaraCallResult ULingaraEmbedSubsystem::Run(const FString& Command, FLingaraOutcome& Outcome, FLingaraEmbedError& Error)
{
	if (!Native.IsValid())
	{
		return RefuseLocally(ELingaraErrorKind::Closed, TEXT("Lingara is not running: call Start first"), Error);
	}
	const FTCHARToUTF8 Utf8(*Command);
	std::string_view Out;
	const lingara_embed::Status Status = Native->Client.execute(std::string_view(Utf8.Get(), Utf8.Length()), Native->AnswerBuffer, Out);
	if (Status == LINGARA_EMBED_OK)
	{
		LingaraEventDecoder::DecodeOutcome(LingaraNative::ToFString(Out), Outcome);
		return ELingaraCallResult::Ok;
	}
	if (Status == LINGARA_EMBED_REFUSED)
	{
		LingaraEventDecoder::DecodeError(LingaraNative::ToFString(Out), Error);
		return ELingaraCallResult::Refused;
	}
	Error = NativeFailure(Status, false);
	return ELingaraCallResult::Refused;
}

void ULingaraEmbedSubsystem::ProvideToken(const FString& Token, const FString& ExpiresAt, ELingaraCallResult& Result, FLingaraEmbedError& Error)
{
	FLingaraOutcome Outcome;
	Result = Run(LingaraCommandEncoder::ProvideToken(Token, ExpiresAt), Outcome, Error);
}

void ULingaraEmbedSubsystem::ProvideTokenWithUpdate(const FString& Token, const FString& ExpiresAt, const FString& UpdateAvailableJson, ELingaraCallResult& Result, FLingaraEmbedError& Error)
{
	FLingaraOutcome Outcome;
	Result = Run(LingaraCommandEncoder::ProvideToken(Token, ExpiresAt, UpdateAvailableJson), Outcome, Error);
}

void ULingaraEmbedSubsystem::ProvideTokenFor(const FString& Token, int32 ExpiresInSecs, ELingaraCallResult& Result, FLingaraEmbedError& Error)
{
	FLingaraOutcome Outcome;
	Result = ExpiresInSecs < 0 ? RefuseLocally(ELingaraErrorKind::InvalidInput, TEXT("ProvideTokenFor: ExpiresInSecs is negative"), Error)
		: Run(LingaraCommandEncoder::ProvideTokenFor(Token, static_cast<uint32>(ExpiresInSecs)), Outcome, Error);
}

void ULingaraEmbedSubsystem::TokenUnavailable(const FString& Reason, ELingaraCallResult& Result, FLingaraEmbedError& Error)
{
	FLingaraOutcome Outcome;
	Result = Run(LingaraCommandEncoder::TokenUnavailable(Reason), Outcome, Error);
}

void ULingaraEmbedSubsystem::SetTokenEndpoint(const FString& Url, const TArray<FLingaraHeader>& Headers, ELingaraCallResult& Result, FLingaraEmbedError& Error)
{
	FLingaraOutcome Outcome;
	Result = Run(LingaraCommandEncoder::SetTokenEndpoint(Url, Headers), Outcome, Error);
}

void ULingaraEmbedSubsystem::StreamPause(ELingaraCallResult& Result, FLingaraEmbedError& Error)
{
	FLingaraOutcome Outcome;
	Result = Run(LingaraCommandEncoder::StreamPause(), Outcome, Error);
}

void ULingaraEmbedSubsystem::StreamResume(ELingaraCallResult& Result, FLingaraEmbedError& Error)
{
	FLingaraOutcome Outcome;
	Result = Run(LingaraCommandEncoder::StreamResume(), Outcome, Error);
}

int64 ULingaraEmbedSubsystem::SendContext(const FLingaraWorldContext& Context, ELingaraCallResult& Result, FLingaraEmbedError& Error)
{
	FLingaraOutcome Outcome;
	Result = Run(LingaraCommandEncoder::SendContext(Context), Outcome, Error);
	return Outcome.Id;
}

int64 ULingaraEmbedSubsystem::PracticeStart(const FLingaraPracticeSource& Source, ELingaraCallResult& Result, FLingaraEmbedError& Error)
{
	FLingaraOutcome Outcome;
	Result = Run(LingaraCommandEncoder::PracticeStart(Source), Outcome, Error);
	return Outcome.Id;
}

bool ULingaraEmbedSubsystem::GetPracticeSnapshot(int64 Session, FLingaraPracticeSnapshot& Snapshot, ELingaraCallResult& Result, FLingaraEmbedError& Error)
{
	FLingaraOutcome Outcome;
	Result = Run(LingaraCommandEncoder::PracticeSnapshot(Session), Outcome, Error);
	Snapshot = Outcome.PracticeSnapshot;
	return Outcome.bHasSnapshot;
}

FLingaraPracticeSnapshot ULingaraEmbedSubsystem::PracticeAnswer(int64 Session, const FString& Answer, ELingaraCallResult& Result, FLingaraEmbedError& Error)
{
	FLingaraOutcome Outcome;
	Result = Run(LingaraCommandEncoder::PracticeAnswer(Session, Answer), Outcome, Error);
	return Outcome.PracticeSnapshot;
}

FLingaraPracticeSnapshot ULingaraEmbedSubsystem::PracticeChoose(int64 Session, int32 Option, ELingaraCallResult& Result, FLingaraEmbedError& Error)
{
	FLingaraOutcome Outcome;
	Result = Option < 0 ? RefuseLocally(ELingaraErrorKind::InvalidInput, TEXT("PracticeChoose: Option is negative"), Error)
		: Run(LingaraCommandEncoder::PracticeChoose(Session, static_cast<uint32>(Option)), Outcome, Error);
	return Outcome.PracticeSnapshot;
}

FLingaraPracticeSnapshot ULingaraEmbedSubsystem::PracticeReveal(int64 Session, ELingaraCallResult& Result, FLingaraEmbedError& Error)
{
	FLingaraOutcome Outcome;
	Result = Run(LingaraCommandEncoder::PracticeReveal(Session), Outcome, Error);
	return Outcome.PracticeSnapshot;
}

FLingaraPracticeSnapshot ULingaraEmbedSubsystem::PracticeSelfGrade(int64 Session, bool bCorrect, ELingaraCallResult& Result, FLingaraEmbedError& Error)
{
	FLingaraOutcome Outcome;
	Result = Run(LingaraCommandEncoder::PracticeSelfGrade(Session, bCorrect), Outcome, Error);
	return Outcome.PracticeSnapshot;
}

FLingaraPracticeSnapshot ULingaraEmbedSubsystem::PracticeNext(int64 Session, ELingaraCallResult& Result, FLingaraEmbedError& Error)
{
	FLingaraOutcome Outcome;
	Result = Run(LingaraCommandEncoder::PracticeNext(Session), Outcome, Error);
	return Outcome.PracticeSnapshot;
}

FLingaraPracticeSnapshot ULingaraEmbedSubsystem::PracticeFinish(int64 Session, ELingaraCallResult& Result, FLingaraEmbedError& Error)
{
	FLingaraOutcome Outcome;
	Result = Run(LingaraCommandEncoder::PracticeFinish(Session), Outcome, Error);
	return Outcome.PracticeSnapshot;
}

int64 ULingaraEmbedSubsystem::DialogueOpen(const FLingaraDialogueConfig& Config, ELingaraCallResult& Result, FLingaraEmbedError& Error)
{
	FLingaraOutcome Outcome;
	Result = Run(LingaraCommandEncoder::DialogueOpen(Config), Outcome, Error);
	return Outcome.Id;
}

int64 ULingaraEmbedSubsystem::DialogueSay(int64 Dialogue, const FString& Line, ELingaraCallResult& Result, FLingaraEmbedError& Error)
{
	FLingaraOutcome Outcome;
	Result = Run(LingaraCommandEncoder::DialogueSay(Dialogue, Line), Outcome, Error);
	return Outcome.Id;
}

void ULingaraEmbedSubsystem::DialogueSetLevel(int64 Dialogue, uint8 Level, ELingaraCallResult& Result, FLingaraEmbedError& Error)
{
	FLingaraOutcome Outcome;
	Result = Run(LingaraCommandEncoder::DialogueSetLevel(Dialogue, Level), Outcome, Error);
}

void ULingaraEmbedSubsystem::DialogueSetScene(int64 Dialogue, const FString& Scene, ELingaraCallResult& Result, FLingaraEmbedError& Error)
{
	FLingaraOutcome Outcome;
	Result = Run(LingaraCommandEncoder::DialogueSetScene(Dialogue, Scene), Outcome, Error);
}

void ULingaraEmbedSubsystem::DialogueCancel(int64 Dialogue, ELingaraCallResult& Result, FLingaraEmbedError& Error)
{
	FLingaraOutcome Outcome;
	Result = Run(LingaraCommandEncoder::DialogueCancel(Dialogue), Outcome, Error);
}

void ULingaraEmbedSubsystem::DialogueClose(int64 Dialogue, ELingaraCallResult& Result, FLingaraEmbedError& Error)
{
	FLingaraOutcome Outcome;
	Result = Run(LingaraCommandEncoder::DialogueClose(Dialogue), Outcome, Error);
}

bool ULingaraEmbedSubsystem::GetDialogueSnapshot(int64 Dialogue, FLingaraDialogueSnapshot& Snapshot, ELingaraCallResult& Result, FLingaraEmbedError& Error)
{
	FLingaraOutcome Outcome;
	Result = Run(LingaraCommandEncoder::DialogueSnapshot(Dialogue), Outcome, Error);
	Snapshot = Outcome.DialogueSnapshot;
	return Outcome.bHasSnapshot;
}
