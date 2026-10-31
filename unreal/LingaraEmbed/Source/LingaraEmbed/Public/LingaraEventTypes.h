// One USTRUCT per K1 event a delegate carries (ADR 1.10.26ac D6). The wire
// tags stay K1's; `error` carries FLingaraEmbedError, `closed` an enum, and
// `log` goes to UE_LOG rather than a delegate (D5). No struct here holds a
// token.
#pragma once

#include "CoreMinimal.h"
#include "LingaraEmbedTypes.h"

#include "LingaraEventTypes.generated.h"

UENUM(BlueprintType)
enum class ELingaraTokenReason : uint8 { Initial, Expiring, Rejected, Unknown };

UENUM(BlueprintType)
enum class ELingaraEventDetailType : uint8 { LessonPlanReady, LessonPlanFailed, Other };

UENUM(BlueprintType)
enum class ELingaraNpcTurnUpdateType : uint8 { Opened, Fragment, Notice, Finished, Cancelled, Unknown };

// `token_needed`: answer with ProvideToken, or TokenUnavailable.
USTRUCT(BlueprintType)
struct LINGARAEMBED_API FLingaraTokenNeeded
{
	GENERATED_BODY()
	UPROPERTY(BlueprintReadOnly, Category = "Lingara") int64 Request = 0;
	UPROPERTY(BlueprintReadOnly, Category = "Lingara") ELingaraTokenReason Reason = ELingaraTokenReason::Initial;
};

// `update_available` (ADR 30.10.26p): once per handle, this build's
// EmbedVersion has a newer or better one. For the developer's log, never the
// player's screen. A null `latest` or `message` is empty.
USTRUCT(BlueprintType)
struct LINGARAEMBED_API FLingaraUpdateAvailable
{
	GENERATED_BODY()
	UPROPERTY(BlueprintReadOnly, Category = "Lingara") FString Latest;
	UPROPERTY(BlueprintReadOnly, Category = "Lingara") bool bDeprecated = false;
	UPROPERTY(BlueprintReadOnly, Category = "Lingara") FString Message;
};

// `stream`: save Cursor at quit and pass it back as ResumeCursor.
USTRUCT(BlueprintType)
struct LINGARAEMBED_API FLingaraStreamChange
{
	GENERATED_BODY()
	UPROPERTY(BlueprintReadOnly, Category = "Lingara") FLingaraStreamState State;
	UPROPERTY(BlueprintReadOnly, Category = "Lingara") FString Cursor;
};

// K1's LingaraEventDetail union, flattened: `Type` says which fields are set.
USTRUCT(BlueprintType)
struct LINGARAEMBED_API FLingaraEventDetail
{
	GENERATED_BODY()
	UPROPERTY(BlueprintReadOnly, Category = "Lingara") ELingaraEventDetailType Type = ELingaraEventDetailType::Other;
	UPROPERTY(BlueprintReadOnly, Category = "Lingara") FString PlanId;
	UPROPERTY(BlueprintReadOnly, Category = "Lingara") FString Title;
	UPROPERTY(BlueprintReadOnly, Category = "Lingara") uint8 Level = 0;
	UPROPERTY(BlueprintReadOnly, Category = "Lingara") FString SourceLang;
	UPROPERTY(BlueprintReadOnly, Category = "Lingara") FString TargetLang;
	UPROPERTY(BlueprintReadOnly, Category = "Lingara") bool bComplete = false;
	UPROPERTY(BlueprintReadOnly, Category = "Lingara") FString Reason;
};

// `lingara`: one event off the stream. DataJson is the event's raw `data`.
USTRUCT(BlueprintType)
struct LINGARAEMBED_API FLingaraEvent
{
	GENERATED_BODY()
	UPROPERTY(BlueprintReadOnly, Category = "Lingara") FString Id;
	UPROPERTY(BlueprintReadOnly, Category = "Lingara") FString EventType;
	UPROPERTY(BlueprintReadOnly, Category = "Lingara") FString CreatedAt;
	UPROPERTY(BlueprintReadOnly, Category = "Lingara") FString DataJson;
	UPROPERTY(BlueprintReadOnly, Category = "Lingara") FLingaraEventDetail Detail;
};

USTRUCT(BlueprintType)
struct LINGARAEMBED_API FLingaraReaction
{
	GENERATED_BODY()
	UPROPERTY(BlueprintReadOnly, Category = "Lingara") FString Status;
	UPROPERTY(BlueprintReadOnly, Category = "Lingara") FString PlanId;
	UPROPERTY(BlueprintReadOnly, Category = "Lingara") FString PlanStatus;
	UPROPERTY(BlueprintReadOnly, Category = "Lingara") FString Code;
	UPROPERTY(BlueprintReadOnly, Category = "Lingara") FString Error;
};

// `world_sent`: SendContext's request, accepted.
USTRUCT(BlueprintType)
struct LINGARAEMBED_API FLingaraWorldSent
{
	GENERATED_BODY()
	UPROPERTY(BlueprintReadOnly, Category = "Lingara") int64 Request = 0;
	UPROPERTY(BlueprintReadOnly, Category = "Lingara") FString EventId;
	UPROPERTY(BlueprintReadOnly, Category = "Lingara") bool bHasReaction = false;
	UPROPERTY(BlueprintReadOnly, Category = "Lingara") FLingaraReaction Reaction;
};

// `practice`: a session's new snapshot.
USTRUCT(BlueprintType)
struct LINGARAEMBED_API FLingaraPracticeUpdate
{
	GENERATED_BODY()
	UPROPERTY(BlueprintReadOnly, Category = "Lingara") int64 Session = 0;
	UPROPERTY(BlueprintReadOnly, Category = "Lingara") FLingaraPracticeSnapshot Snapshot;
};

// K1's NpcTurnUpdate union: Text on Fragment and Finished, Code and Message on Notice.
USTRUCT(BlueprintType)
struct LINGARAEMBED_API FLingaraNpcTurnUpdate
{
	GENERATED_BODY()
	UPROPERTY(BlueprintReadOnly, Category = "Lingara") ELingaraNpcTurnUpdateType Type = ELingaraNpcTurnUpdateType::Opened;
	UPROPERTY(BlueprintReadOnly, Category = "Lingara") FString Text;
	UPROPERTY(BlueprintReadOnly, Category = "Lingara") FString Code;
	UPROPERTY(BlueprintReadOnly, Category = "Lingara") FString Message;
};

// `npc_turn`: one step of a DialogueSay reply.
USTRUCT(BlueprintType)
struct LINGARAEMBED_API FLingaraNpcTurn
{
	GENERATED_BODY()
	UPROPERTY(BlueprintReadOnly, Category = "Lingara") int64 Dialogue = 0;
	UPROPERTY(BlueprintReadOnly, Category = "Lingara") int64 Request = 0;
	UPROPERTY(BlueprintReadOnly, Category = "Lingara") FLingaraNpcTurnUpdate Update;
};
