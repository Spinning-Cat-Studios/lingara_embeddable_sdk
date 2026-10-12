// The event half is a table from each K1 event `type` to its own function
// (ADR 1.10.26ac D5), so no function grows past the complexity budget. Every
// event `type` in ffi/schema/messages.schema.json appears below as a string
// literal; unreal/tools/check_coverage.sh fails the PR that adds one without
// it.
#include "LingaraEventDecoder.h"

#include "LingaraJsonShapes.h"
#include "Serialization/JsonReader.h"
#include "Serialization/JsonSerializer.h"

using namespace LingaraJsonShapes;

namespace
{
	using FDecode = bool (*)(const FObjectPtr&, FLingaraDecodedEvent&);

	FObjectPtr Parse(const FString& Json)
	{
		FObjectPtr Object;
		const TSharedRef<TJsonReader<TCHAR>> Reader = TJsonReaderFactory<TCHAR>::Create(Json);
		return FJsonSerializer::Deserialize(Reader, Object) ? Object : nullptr;
	}

	bool TokenNeeded(const FObjectPtr& O, FLingaraDecodedEvent& Out)
	{
		Out.TokenNeeded.Reason = TokenReason(OptString(O, TEXT("reason")));
		return ReadId(O, TEXT("request"), Out.TokenNeeded.Request);
	}

	bool Stream(const FObjectPtr& O, FLingaraDecodedEvent& Out)
	{
		Out.Stream.Cursor = OptString(O, TEXT("cursor"));
		return ReadStreamState(OptObject(O, TEXT("state")), Out.Stream.State);
	}

	bool Lingara(const FObjectPtr& O, FLingaraDecodedEvent& Out)
	{
		return ReadLingaraEvent(OptObject(O, TEXT("event")), Out.Lingara);
	}

	bool WorldSent(const FObjectPtr& O, FLingaraDecodedEvent& Out)
	{
		if (const FObjectPtr Reaction = OptObject(O, TEXT("reaction")))
		{
			Out.WorldSent.bHasReaction = ReadReaction(Reaction, Out.WorldSent.Reaction);
		}
		return ReadId(O, TEXT("request"), Out.WorldSent.Request) && O->TryGetStringField(TEXT("event_id"), Out.WorldSent.EventId);
	}

	bool Practice(const FObjectPtr& O, FLingaraDecodedEvent& Out)
	{
		return ReadId(O, TEXT("session"), Out.Practice.Session)
			&& ReadPracticeSnapshot(OptObject(O, TEXT("snapshot")), Out.Practice.Snapshot);
	}

	bool NpcTurn(const FObjectPtr& O, FLingaraDecodedEvent& Out)
	{
		return ReadId(O, TEXT("dialogue"), Out.NpcTurn.Dialogue) && ReadId(O, TEXT("request"), Out.NpcTurn.Request)
			&& ReadNpcTurnUpdate(OptObject(O, TEXT("update")), Out.NpcTurn.Update);
	}

	bool Error(const FObjectPtr& O, FLingaraDecodedEvent& Out)
	{
		return ReadError(OptObject(O, TEXT("error")), Out.Error);
	}

	bool Log(const FObjectPtr& O, FLingaraDecodedEvent& Out)
	{
		Out.LogLevel = LogLevel(OptString(O, TEXT("level")));
		Out.LogTarget = OptString(O, TEXT("target"));
		return O->TryGetStringField(TEXT("message"), Out.LogMessage);
	}

	bool Closed(const FObjectPtr& O, FLingaraDecodedEvent& Out)
	{
		Out.CloseReason = CloseReason(OptString(O, TEXT("reason")));
		return true;
	}

	struct FRow
	{
		const TCHAR* Tag;
		ELingaraDecodedType Type;
		FDecode Decode;
	};

	const FRow GEvents[] = {
		{TEXT("token_needed"), ELingaraDecodedType::TokenNeeded, &TokenNeeded},
		{TEXT("stream"), ELingaraDecodedType::Stream, &Stream},
		{TEXT("lingara"), ELingaraDecodedType::Lingara, &Lingara},
		{TEXT("world_sent"), ELingaraDecodedType::WorldSent, &WorldSent},
		{TEXT("practice"), ELingaraDecodedType::Practice, &Practice},
		{TEXT("npc_turn"), ELingaraDecodedType::NpcTurn, &NpcTurn},
		{TEXT("error"), ELingaraDecodedType::Error, &Error},
		{TEXT("log"), ELingaraDecodedType::Log, &Log},
		{TEXT("closed"), ELingaraDecodedType::Closed, &Closed},
	};

	bool ReadOutcomeBody(const FObjectPtr& O, FLingaraOutcome& Out)
	{
		const FObjectPtr Snapshot = OptObject(O, TEXT("snapshot"));
		switch (Out.Type)
		{
		case ELingaraOutcomeType::Id:
			return ReadId(O, TEXT("id"), Out.Id);
		case ELingaraOutcomeType::Practice:
			Out.bHasSnapshot = Snapshot.IsValid();
			return !Out.bHasSnapshot || ReadPracticeSnapshot(Snapshot, Out.PracticeSnapshot);
		case ELingaraOutcomeType::Dialogue:
			Out.bHasSnapshot = Snapshot.IsValid();
			return !Out.bHasSnapshot || ReadDialogueSnapshot(Snapshot, Out.DialogueSnapshot);
		default:
			return Out.Type == ELingaraOutcomeType::Done;
		}
	}
}

namespace LingaraEventDecoder
{
	bool DecodeEvent(const FString& Json, FLingaraDecodedEvent& Out)
	{
		const FObjectPtr O = Parse(Json);
		if (!O.IsValid() || !O->TryGetStringField(TEXT("type"), Out.TypeName))
		{
			return false;
		}
		for (const FRow& Row : GEvents)
		{
			if (Out.TypeName == Row.Tag)
			{
				Out.Type = Row.Type;
				return Row.Decode(O, Out);
			}
		}
		Out.Type = ELingaraDecodedType::Unknown;
		return true;
	}

	bool DecodeOutcome(const FString& Json, FLingaraOutcome& Out)
	{
		static const TCHAR* const Words[] = {TEXT("done"), TEXT("id"), TEXT("practice"), TEXT("dialogue")};
		const FObjectPtr O = Parse(Json);
		FString Type;
		if (!O.IsValid() || !O->TryGetStringField(TEXT("type"), Type))
		{
			return false;
		}
		Out.Type = ELingaraOutcomeType::Unknown;
		for (int32 Index = 0; Index < UE_ARRAY_COUNT(Words); ++Index)
		{
			if (Type == Words[Index])
			{
				Out.Type = static_cast<ELingaraOutcomeType>(Index);
			}
		}
		return ReadOutcomeBody(O, Out);
	}

	bool DecodeError(const FString& Json, FLingaraEmbedError& Out)
	{
		return ReadError(Parse(Json), Out);
	}

	bool DecodeStatus(const FString& Json, FLingaraEmbedStatus& Out)
	{
		const FObjectPtr O = Parse(Json);
		if (!O.IsValid() || !ReadStreamState(OptObject(O, TEXT("stream")), Out.Stream))
		{
			return false;
		}
		Out.Cursor = OptString(O, TEXT("cursor"));
		O->TryGetBoolField(TEXT("needs_token"), Out.bNeedsToken);
		O->TryGetBoolField(TEXT("closed"), Out.bClosed);
		return true;
	}
}
