// K1's configuration, status, error, practice and dialogue shapes as
// Blueprint types (ADR 1.10.26ac D6, D7). Ids are int64 throughout: K1 keeps
// them at or below i64::MAX, and Blueprint has no uint64. A struct the game
// fills (config, context, source) is BlueprintReadWrite; one the plugin fills
// is BlueprintReadOnly.
#pragma once

#include "CoreMinimal.h"

#include "LingaraEmbedTypes.generated.h"

UENUM(BlueprintType)
enum class ELingaraEmbedState : uint8 { Stopped, Running, Disabled };

UENUM(BlueprintType)
enum class ELingaraCallResult : uint8 { Ok, Refused };

UENUM(BlueprintType)
enum class ELingaraErrorKind : uint8
{
	Config, InvalidInput, TokenUnavailable, TokenRejected, Forbidden, Billing, NotFound, RateLimited,
	Unavailable, Network, Busy, InvalidState, UnknownId, Closed, Internal, Unknown
};

// `CoreDefault` is omitted from the config, so K1's own default applies.
UENUM(BlueprintType)
enum class ELingaraLogLevel : uint8 { CoreDefault, Error, Warn, Info, Debug, Trace };

UENUM(BlueprintType)
enum class ELingaraCloseReason : uint8 { Requested, Internal, Unknown };

UENUM(BlueprintType)
enum class ELingaraStreamStateType : uint8 { Connecting, Live, Paused, Down, Unknown };

UENUM(BlueprintType)
enum class ELingaraPracticeSourceType : uint8 { Request, Create, Plan };

UENUM(BlueprintType)
enum class ELingaraSpeaker : uint8 { Player, Npc };

UENUM(BlueprintType)
enum class ELingaraPracticePhase : uint8
{
	Requesting, Generating, Creating, Loading, Unanswered, AwaitingSelfGrade, Reviewed, SetComplete,
	Reporting, Reported, ReportFailed, Failed, Ended, Unknown
};

UENUM(BlueprintType)
enum class ELingaraPracticeOutcome : uint8 { None, Correct, Incorrect, Revealed, SelfCorrect, SelfIncorrect, Skipped, Unknown };

UENUM(BlueprintType)
enum class ELingaraDialoguePhase : uint8 { Idle, Sending, Streaming, Unknown };

USTRUCT(BlueprintType)
struct LINGARAEMBED_API FLingaraHeader
{
	GENERATED_BODY()
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Lingara") FString Name;
	// A game session is a credential too: never logged, never on a delegate.
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Lingara") FString Value;
};

// K1's Http token source: the plugin POSTs `{}` here and reads `{token, expires_at}`.
USTRUCT(BlueprintType)
struct LINGARAEMBED_API FLingaraTokenEndpoint
{
	GENERATED_BODY()
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Lingara") FString Url;
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Lingara") TArray<FLingaraHeader> Headers;
};

// K1's EmbedConfig. Anything left unset (empty, zero, an un-ticked override)
// is omitted from the JSON, so K1's default applies (D7).
USTRUCT(BlueprintType)
struct LINGARAEMBED_API FLingaraEmbedConfig
{
	GENERATED_BODY()
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Lingara") FString BaseUrl;
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Lingara") ELingaraLogLevel LogLevel = ELingaraLogLevel::CoreDefault;
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Lingara", meta = (InlineEditConditionToggle)) bool bOverride_StreamOnStart = false;
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Lingara", meta = (EditCondition = "bOverride_StreamOnStart")) bool bStreamOnStart = true;
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Lingara") FString ResumeCursor;
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Lingara") TArray<FString> EventTypes;
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Lingara") FString ApiVersion;
	// The platform embed version this build was registered against (ADR 30.10.26p).
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Lingara") FString EmbedVersion;
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Lingara") int32 QueueCapacity = 0;
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Lingara") int32 StreamIdleTimeoutMs = 0;
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Lingara") int32 TokenWaitTimeoutMs = 0;
	// Follows `lingara-unreal/<version> unreal/<engine>` (D5).
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Lingara") FString UserAgentSuffix;
	// Runtime only, never in settings: its header values are the game's session (D7).
	UPROPERTY(BlueprintReadWrite, Category = "Lingara") FLingaraTokenEndpoint TokenEndpoint;
	// Not sent: the subsystem's own drain cap (D5).
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Lingara") int32 MaxEventsPerTick = 256;
};

// K1's EmbedError. `Message` is English, for logs, and never holds a token.
USTRUCT(BlueprintType)
struct LINGARAEMBED_API FLingaraEmbedError
{
	GENERATED_BODY()
	UPROPERTY(BlueprintReadOnly, Category = "Lingara") ELingaraErrorKind Kind = ELingaraErrorKind::Internal;
	UPROPERTY(BlueprintReadOnly, Category = "Lingara") FString KindName;
	UPROPERTY(BlueprintReadOnly, Category = "Lingara") FString Message;
	UPROPERTY(BlueprintReadOnly, Category = "Lingara") int32 Status = 0;
	UPROPERTY(BlueprintReadOnly, Category = "Lingara") FString Code;
	UPROPERTY(BlueprintReadOnly, Category = "Lingara") int64 RetryAfterMs = 0;
	UPROPERTY(BlueprintReadOnly, Category = "Lingara") bool bRetryable = false;
	UPROPERTY(BlueprintReadOnly, Category = "Lingara") int64 Request = 0;
};

USTRUCT(BlueprintType)
struct LINGARAEMBED_API FLingaraStreamState
{
	GENERATED_BODY()
	UPROPERTY(BlueprintReadOnly, Category = "Lingara") ELingaraStreamStateType Type = ELingaraStreamStateType::Connecting;
	UPROPERTY(BlueprintReadOnly, Category = "Lingara") FLingaraEmbedError Error;
	UPROPERTY(BlueprintReadOnly, Category = "Lingara") int64 RetryInMs = 0;
};

// K1's EmbedStatus, read from the core each call: the plugin keeps no copy (D5).
USTRUCT(BlueprintType)
struct LINGARAEMBED_API FLingaraEmbedStatus
{
	GENERATED_BODY()
	UPROPERTY(BlueprintReadOnly, Category = "Lingara") FLingaraStreamState Stream;
	UPROPERTY(BlueprintReadOnly, Category = "Lingara") FString Cursor;
	UPROPERTY(BlueprintReadOnly, Category = "Lingara") bool bNeedsToken = false;
	UPROPERTY(BlueprintReadOnly, Category = "Lingara") bool bClosed = false;
};

USTRUCT(BlueprintType)
struct LINGARAEMBED_API FLingaraNpc
{
	GENERATED_BODY()
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Lingara") FString Name;
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Lingara") FString Persona;
};

USTRUCT(BlueprintType)
struct LINGARAEMBED_API FLingaraWindowEntry
{
	GENERATED_BODY()
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Lingara") ELingaraSpeaker Speaker = ELingaraSpeaker::Player;
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Lingara") FString Text;
};

USTRUCT(BlueprintType)
struct LINGARAEMBED_API FLingaraDialogueConfig
{
	GENERATED_BODY()
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Lingara") FLingaraNpc Npc;
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Lingara") FString Scene;
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Lingara") FString SourceLang;
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Lingara") FString TargetLang;
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Lingara") uint8 Level = 1;
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Lingara") TArray<FLingaraWindowEntry> Window;
};

USTRUCT(BlueprintType)
struct LINGARAEMBED_API FLingaraDialogueSnapshot
{
	GENERATED_BODY()
	UPROPERTY(BlueprintReadOnly, Category = "Lingara") ELingaraDialoguePhase Phase = ELingaraDialoguePhase::Idle;
	UPROPERTY(BlueprintReadOnly, Category = "Lingara") FLingaraNpc Npc;
	UPROPERTY(BlueprintReadOnly, Category = "Lingara") FString Scene;
	UPROPERTY(BlueprintReadOnly, Category = "Lingara") uint8 Level = 0;
	UPROPERTY(BlueprintReadOnly, Category = "Lingara") TArray<FLingaraWindowEntry> Window;
};

// `world.context_changed` (K1 D6).
USTRUCT(BlueprintType)
struct LINGARAEMBED_API FLingaraWorldContext
{
	GENERATED_BODY()
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Lingara") FString Scene;
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Lingara") FString SourceLang;
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Lingara") FString TargetLang;
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Lingara") uint8 Level = 1;
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Lingara") bool bHasNpc = false;
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Lingara") FLingaraNpc Npc;
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Lingara") TArray<FString> Tags;
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Lingara") bool bGenerate = false;
};

// K1's PracticeSource union: `Type` picks which fields are sent.
USTRUCT(BlueprintType)
struct LINGARAEMBED_API FLingaraPracticeSource
{
	GENERATED_BODY()
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Lingara") ELingaraPracticeSourceType Type = ELingaraPracticeSourceType::Plan;
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Lingara") FString Topic;
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Lingara") FString Context;
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Lingara") FString SourceLang;
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Lingara") FString TargetLang;
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Lingara") uint8 Level = 1;
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Lingara") TArray<FString> Tags;
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Lingara") FString PlanId;
	// A demo plan must be started with `false` (K1 D7).
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Lingara") bool bReport = true;
};

USTRUCT(BlueprintType)
struct LINGARAEMBED_API FLingaraPlanWord
{
	GENERATED_BODY()
	UPROPERTY(BlueprintReadOnly, Category = "Lingara") FString Word;
	UPROPERTY(BlueprintReadOnly, Category = "Lingara") FString Translation;
	UPROPERTY(BlueprintReadOnly, Category = "Lingara") FString Pronunciation;
};

USTRUCT(BlueprintType)
struct LINGARAEMBED_API FLingaraPlanSummary
{
	GENERATED_BODY()
	UPROPERTY(BlueprintReadOnly, Category = "Lingara") FString Id;
	UPROPERTY(BlueprintReadOnly, Category = "Lingara") FString Title;
	UPROPERTY(BlueprintReadOnly, Category = "Lingara") uint8 Level = 0;
	UPROPERTY(BlueprintReadOnly, Category = "Lingara") FString SourceLang;
	UPROPERTY(BlueprintReadOnly, Category = "Lingara") FString TargetLang;
	UPROPERTY(BlueprintReadOnly, Category = "Lingara") TArray<FLingaraPlanWord> Vocabulary;
};

USTRUCT(BlueprintType)
struct LINGARAEMBED_API FLingaraPracticeQuestion
{
	GENERATED_BODY()
	UPROPERTY(BlueprintReadOnly, Category = "Lingara") FString Kind;
	UPROPERTY(BlueprintReadOnly, Category = "Lingara") FString Prompt;
	// Non-empty on a multiple-choice question: answer with PracticeChoose.
	UPROPERTY(BlueprintReadOnly, Category = "Lingara") TArray<FString> Options;
	UPROPERTY(BlueprintReadOnly, Category = "Lingara") FString Hint;
	UPROPERTY(BlueprintReadOnly, Category = "Lingara") bool bSelfGraded = false;
};

USTRUCT(BlueprintType)
struct LINGARAEMBED_API FLingaraAnswerResult
{
	GENERATED_BODY()
	// None while a self-graded question waits for the learner's grade.
	UPROPERTY(BlueprintReadOnly, Category = "Lingara") ELingaraPracticeOutcome Outcome = ELingaraPracticeOutcome::None;
	UPROPERTY(BlueprintReadOnly, Category = "Lingara") FString Expected;
	UPROPERTY(BlueprintReadOnly, Category = "Lingara") FString Explanation;
};

USTRUCT(BlueprintType)
struct LINGARAEMBED_API FLingaraPracticeSnapshot
{
	GENERATED_BODY()
	UPROPERTY(BlueprintReadOnly, Category = "Lingara") ELingaraPracticePhase Phase = ELingaraPracticePhase::Requesting;
	UPROPERTY(BlueprintReadOnly, Category = "Lingara") bool bHasPlan = false;
	UPROPERTY(BlueprintReadOnly, Category = "Lingara") FLingaraPlanSummary Plan;
	UPROPERTY(BlueprintReadOnly, Category = "Lingara") int32 SetIndex = 0;
	UPROPERTY(BlueprintReadOnly, Category = "Lingara") int32 SetCount = 0;
	UPROPERTY(BlueprintReadOnly, Category = "Lingara") int32 QuestionIndex = 0;
	UPROPERTY(BlueprintReadOnly, Category = "Lingara") int32 QuestionCount = 0;
	UPROPERTY(BlueprintReadOnly, Category = "Lingara") bool bHasQuestion = false;
	UPROPERTY(BlueprintReadOnly, Category = "Lingara") FLingaraPracticeQuestion Question;
	UPROPERTY(BlueprintReadOnly, Category = "Lingara") bool bHasLastResult = false;
	UPROPERTY(BlueprintReadOnly, Category = "Lingara") FLingaraAnswerResult LastResult;
	UPROPERTY(BlueprintReadOnly, Category = "Lingara") bool bHasFailure = false;
	UPROPERTY(BlueprintReadOnly, Category = "Lingara") FLingaraEmbedError Failure;
};
