// Every EmbedEvent example in conformance/vectors/json-shape.json decodes
// into its USTRUCT with every field equal to the example (ADR 1.10.26ac
// AC12). Offline; run only by `make unreal-smoke`, never by public CI.
#include "Misc/AutomationTest.h"

#if WITH_DEV_AUTOMATION_TESTS

#include "LingaraEventDecoder.h"
#include "LingaraSpecSupport.h"

BEGIN_DEFINE_SPEC(FLingaraEventDecodeSpec, "LingaraEmbed.EventDecode",
	EAutomationTestFlags::EngineFilter | EAutomationTestFlags_ApplicationContextMask)
	TSharedPtr<FJsonObject> Shape;
	FString ShapePath;

	void ExpectError(const TSharedPtr<FJsonObject>& O, const FLingaraEmbedError& E)
	{
		TestEqual(TEXT("kind"), E.KindName, O->GetStringField(TEXT("kind")));
		TestEqual(TEXT("message"), E.Message, O->GetStringField(TEXT("message")));
		TestEqual(TEXT("retryable"), E.bRetryable, O->GetBoolField(TEXT("retryable")));
		double Number = 0;
		TestEqual(TEXT("status"), E.Status, O->TryGetNumberField(TEXT("status"), Number) ? static_cast<int32>(Number) : 0);
		TestEqual(TEXT("retry_after_ms"), E.RetryAfterMs, O->TryGetNumberField(TEXT("retry_after_ms"), Number) ? static_cast<int64>(Number) : 0);
		TestEqual(TEXT("request"), E.Request, O->TryGetNumberField(TEXT("request"), Number) ? static_cast<int64>(Number) : 0);
		FString Code;
		O->TryGetStringField(TEXT("code"), Code);
		TestEqual(TEXT("code"), E.Code, Code);
	}

	void ExpectPractice(const TSharedPtr<FJsonObject>& O, const FLingaraPracticeUpdate& P)
	{
		const TSharedPtr<FJsonObject> S = O->GetObjectField(TEXT("snapshot"));
		TestEqual(TEXT("session"), P.Session, static_cast<int64>(O->GetNumberField(TEXT("session"))));
		TestEqual(TEXT("phase"), P.Snapshot.Phase, ELingaraPracticePhase::Unanswered);
		TestEqual(TEXT("set_count"), P.Snapshot.SetCount, static_cast<int32>(S->GetNumberField(TEXT("set_count"))));
		TestEqual(TEXT("question_count"), P.Snapshot.QuestionCount, static_cast<int32>(S->GetNumberField(TEXT("question_count"))));
		TestEqual(TEXT("plan.id"), P.Snapshot.Plan.Id, S->GetObjectField(TEXT("plan"))->GetStringField(TEXT("id")));
		TestEqual(TEXT("plan.vocabulary"), P.Snapshot.Plan.Vocabulary.Num(), 1);
		TestEqual(TEXT("vocabulary[0].pronunciation"), P.Snapshot.Plan.Vocabulary[0].Pronunciation, FString(TEXT("nǐ hǎo")));
		TestEqual(TEXT("question.options"), P.Snapshot.Question.Options.Num(), 2);
		TestEqual(TEXT("question.prompt"), P.Snapshot.Question.Prompt, S->GetObjectField(TEXT("question"))->GetStringField(TEXT("prompt")));
		TestEqual(TEXT("last_result.outcome"), P.Snapshot.LastResult.Outcome, ELingaraPracticeOutcome::Correct);
		TestFalse(TEXT("failure"), P.Snapshot.bHasFailure);
	}

	void ExpectEvent(const TSharedPtr<FJsonObject>& O, const FLingaraDecodedEvent& E)
	{
		const FString Type = O->GetStringField(TEXT("type"));
		if (Type == TEXT("token_needed")) { TestEqual(TEXT("request"), E.TokenNeeded.Request, int64{1}); TestEqual(TEXT("reason"), E.TokenNeeded.Reason, ELingaraTokenReason::Initial); }
		else if (Type == TEXT("stream")) { TestEqual(TEXT("state"), E.Stream.State.Type, ELingaraStreamStateType::Live); TestEqual(TEXT("cursor"), E.Stream.Cursor, O->GetStringField(TEXT("cursor"))); }
		else if (Type == TEXT("lingara")) { const TSharedPtr<FJsonObject> L = O->GetObjectField(TEXT("event")); TestEqual(TEXT("id"), E.Lingara.Id, L->GetStringField(TEXT("id"))); TestEqual(TEXT("data_json"), E.Lingara.DataJson, L->GetStringField(TEXT("data_json"))); TestEqual(TEXT("detail"), E.Lingara.Detail.Type, ELingaraEventDetailType::Other); }
		else if (Type == TEXT("world_sent")) { TestEqual(TEXT("event_id"), E.WorldSent.EventId, O->GetStringField(TEXT("event_id"))); TestTrue(TEXT("reaction"), E.WorldSent.bHasReaction); TestEqual(TEXT("plan_status"), E.WorldSent.Reaction.PlanStatus, FString(TEXT("generating"))); }
		else if (Type == TEXT("practice")) { ExpectPractice(O, E.Practice); }
		else if (Type == TEXT("npc_turn")) { TestEqual(TEXT("dialogue"), E.NpcTurn.Dialogue, int64{4}); TestEqual(TEXT("request"), E.NpcTurn.Request, int64{5}); TestEqual(TEXT("update"), E.NpcTurn.Update.Type, ELingaraNpcTurnUpdateType::Fragment); TestEqual(TEXT("text"), E.NpcTurn.Update.Text, FString(TEXT("Ni"))); }
		else if (Type == TEXT("error")) { ExpectError(O->GetObjectField(TEXT("error")), E.Error); }
		else if (Type == TEXT("log")) { TestEqual(TEXT("level"), E.LogLevel, ELingaraLogLevel::Warn); TestEqual(TEXT("target"), E.LogTarget, O->GetStringField(TEXT("target"))); TestEqual(TEXT("message"), E.LogMessage, O->GetStringField(TEXT("message"))); }
		else if (Type == TEXT("closed")) { TestEqual(TEXT("reason"), E.CloseReason, ELingaraCloseReason::Requested); }
		else { AddError(FString::Printf(TEXT("json-shape.json has an event type this spec does not check: %s"), *Type)); }
	}
END_DEFINE_SPEC(FLingaraEventDecodeSpec)

void FLingaraEventDecodeSpec::Define()
{
	BeforeEach([this]()
	{
		Shape = LingaraSpec::LoadJsonShape(ShapePath);
		TestTrue(FString::Printf(TEXT("json-shape.json is readable at %s"), *ShapePath), Shape.IsValid());
	});

	// 1.10.26ac AC12
	It("every event example decodes into its struct", [this]()
	{
		const TArray<TSharedPtr<FJsonObject>> Events = LingaraSpec::Examples(Shape, TEXT("EmbedEvent"));
		TestEqual(TEXT("K1's nine event types"), Events.Num(), 9);
		for (const TSharedPtr<FJsonObject>& Example : Events)
		{
			FLingaraDecodedEvent Decoded;
			const bool bDecoded = LingaraEventDecoder::DecodeEvent(LingaraSpec::Serialize(Example), Decoded);
			TestTrue(FString::Printf(TEXT("%s decodes"), *Decoded.TypeName), bDecoded && Decoded.Type != ELingaraDecodedType::Unknown);
			ExpectEvent(Example, Decoded);
		}
	});

	It("decodes every stream state and npc turn update", [this]()
	{
		for (const TSharedPtr<FJsonObject>& State : LingaraSpec::Examples(Shape, TEXT("StreamState")))
		{
			FLingaraDecodedEvent Decoded;
			const FString Json = FString::Printf(TEXT("{\"type\":\"stream\",\"state\":%s,\"cursor\":null}"), *LingaraSpec::Serialize(State));
			TestTrue(TEXT("a stream state decodes"), LingaraEventDecoder::DecodeEvent(Json, Decoded) && Decoded.Stream.State.Type != ELingaraStreamStateType::Unknown);
		}
		for (const TSharedPtr<FJsonObject>& Update : LingaraSpec::Examples(Shape, TEXT("NpcTurnUpdate")))
		{
			FLingaraDecodedEvent Decoded;
			const FString Json = FString::Printf(TEXT("{\"type\":\"npc_turn\",\"dialogue\":1,\"request\":2,\"update\":%s}"), *LingaraSpec::Serialize(Update));
			TestTrue(TEXT("an npc turn update decodes"), LingaraEventDecoder::DecodeEvent(Json, Decoded) && Decoded.NpcTurn.Update.Type != ELingaraNpcTurnUpdateType::Unknown);
		}
	});

	It("routes an unknown type to OnUnknownEvent with its raw json", [this]()
	{
		FLingaraDecodedEvent Decoded;
		TestTrue(TEXT("decodes"), LingaraEventDecoder::DecodeEvent(TEXT("{\"type\":\"weather\",\"n\":1}"), Decoded));
		TestEqual(TEXT("unknown"), Decoded.Type, ELingaraDecodedType::Unknown);
		TestEqual(TEXT("type name"), Decoded.TypeName, FString(TEXT("weather")));
	});

	It("decodes every command outcome", [this]()
	{
		for (const TSharedPtr<FJsonObject>& Example : LingaraSpec::Examples(Shape, TEXT("CommandOutcome")))
		{
			FLingaraOutcome Outcome;
			TestTrue(TEXT("an outcome decodes"), LingaraEventDecoder::DecodeOutcome(LingaraSpec::Serialize(Example), Outcome) && Outcome.Type != ELingaraOutcomeType::Unknown);
		}
	});
}

#endif
