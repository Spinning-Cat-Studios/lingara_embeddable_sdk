// Every Command example in conformance/vectors/json-shape.json is JSON-equal
// to what its subsystem method's encoder writes from the same values (ADR
// 1.10.26ac AC13). Offline; run only by `make unreal-smoke`.
#include "Misc/AutomationTest.h"

#if WITH_DEV_AUTOMATION_TESTS

#include "LingaraCommandEncoder.h"
#include "LingaraSpecSupport.h"
#include "Serialization/JsonReader.h"
#include "Serialization/JsonSerializer.h"

namespace
{
	using FObj = TSharedPtr<FJsonObject>;
	using FEncode = TFunction<FString(const FObj&)>;

	int64 Num(const FObj& O, const TCHAR* Key) { return static_cast<int64>(O->GetNumberField(Key)); }
	FString Str(const FObj& O, const TCHAR* Key) { FString S; O->TryGetStringField(Key, S); return S; }

	FLingaraNpc Npc(const FObj& O) { FLingaraNpc N; N.Name = Str(O, TEXT("name")); N.Persona = Str(O, TEXT("persona")); return N; }

	TArray<FLingaraHeader> Headers(const FObj& O)
	{
		TArray<FLingaraHeader> Out;
		for (const TSharedPtr<FJsonValue>& V : O->GetArrayField(TEXT("headers")))
		{
			Out.Add(FLingaraHeader{Str(V->AsObject(), TEXT("name")), Str(V->AsObject(), TEXT("value"))});
		}
		return Out;
	}

	FLingaraWorldContext Context(const FObj& C)
	{
		FLingaraWorldContext W;
		W.Scene = Str(C, TEXT("scene"));
		W.SourceLang = Str(C, TEXT("source_lang"));
		W.TargetLang = Str(C, TEXT("target_lang"));
		W.Level = static_cast<uint8>(Num(C, TEXT("level")));
		W.bHasNpc = C->HasTypedField<EJson::Object>(TEXT("npc"));
		if (W.bHasNpc) W.Npc = Npc(C->GetObjectField(TEXT("npc")));
		C->TryGetStringArrayField(TEXT("tags"), W.Tags);
		C->TryGetBoolField(TEXT("generate"), W.bGenerate);
		return W;
	}

	FLingaraDialogueConfig Dialogue(const FObj& C)
	{
		FLingaraDialogueConfig D;
		D.Npc = Npc(C->GetObjectField(TEXT("npc")));
		D.Scene = Str(C, TEXT("scene"));
		D.SourceLang = Str(C, TEXT("source_lang"));
		D.TargetLang = Str(C, TEXT("target_lang"));
		D.Level = static_cast<uint8>(Num(C, TEXT("level")));
		for (const TSharedPtr<FJsonValue>& V : C->GetArrayField(TEXT("window")))
		{
			const bool bNpc = Str(V->AsObject(), TEXT("speaker")) == TEXT("npc");
			D.Window.Add(FLingaraWindowEntry{bNpc ? ELingaraSpeaker::Npc : ELingaraSpeaker::Player, Str(V->AsObject(), TEXT("text"))});
		}
		return D;
	}

	FLingaraPracticeSource Plan(const FObj& S)
	{
		FLingaraPracticeSource P;
		P.Type = ELingaraPracticeSourceType::Plan;
		P.PlanId = Str(S, TEXT("plan_id"));
		S->TryGetBoolField(TEXT("report"), P.bReport);
		return P;
	}

	// One row per K1 command: the subsystem method's encoder, fed the example's values.
	TMap<FString, FEncode> Table()
	{
		using namespace LingaraCommandEncoder;
		TMap<FString, FEncode> T;
		T.Add(TEXT("provide_token"), [](const FObj& O) { return ProvideToken(Str(O, TEXT("token")), Str(O, TEXT("expires_at"))); });
		T.Add(TEXT("provide_token_for"), [](const FObj& O) { return ProvideTokenFor(Str(O, TEXT("token")), static_cast<uint32>(Num(O, TEXT("expires_in_secs")))); });
		T.Add(TEXT("token_unavailable"), [](const FObj& O) { return TokenUnavailable(Str(O, TEXT("reason"))); });
		T.Add(TEXT("set_token_endpoint"), [](const FObj& O) { return SetTokenEndpoint(Str(O, TEXT("url")), Headers(O)); });
		T.Add(TEXT("stream_pause"), [](const FObj&) { return StreamPause(); });
		T.Add(TEXT("stream_resume"), [](const FObj&) { return StreamResume(); });
		T.Add(TEXT("send_context"), [](const FObj& O) { return SendContext(Context(O->GetObjectField(TEXT("context")))); });
		T.Add(TEXT("practice_start"), [](const FObj& O) { return PracticeStart(Plan(O->GetObjectField(TEXT("source")))); });
		T.Add(TEXT("practice_snapshot"), [](const FObj& O) { return PracticeSnapshot(Num(O, TEXT("session"))); });
		T.Add(TEXT("practice_answer"), [](const FObj& O) { return PracticeAnswer(Num(O, TEXT("session")), Str(O, TEXT("answer"))); });
		T.Add(TEXT("practice_choose"), [](const FObj& O) { return PracticeChoose(Num(O, TEXT("session")), static_cast<uint32>(Num(O, TEXT("option")))); });
		T.Add(TEXT("practice_reveal"), [](const FObj& O) { return PracticeReveal(Num(O, TEXT("session"))); });
		T.Add(TEXT("practice_self_grade"), [](const FObj& O) { return PracticeSelfGrade(Num(O, TEXT("session")), O->GetBoolField(TEXT("correct"))); });
		T.Add(TEXT("practice_next"), [](const FObj& O) { return PracticeNext(Num(O, TEXT("session"))); });
		T.Add(TEXT("practice_finish"), [](const FObj& O) { return PracticeFinish(Num(O, TEXT("session"))); });
		T.Add(TEXT("dialogue_open"), [](const FObj& O) { return DialogueOpen(Dialogue(O->GetObjectField(TEXT("config")))); });
		T.Add(TEXT("dialogue_say"), [](const FObj& O) { return DialogueSay(Num(O, TEXT("dialogue")), Str(O, TEXT("line"))); });
		T.Add(TEXT("dialogue_set_level"), [](const FObj& O) { return DialogueSetLevel(Num(O, TEXT("dialogue")), static_cast<uint8>(Num(O, TEXT("level")))); });
		T.Add(TEXT("dialogue_set_scene"), [](const FObj& O) { return DialogueSetScene(Num(O, TEXT("dialogue")), Str(O, TEXT("scene"))); });
		T.Add(TEXT("dialogue_cancel"), [](const FObj& O) { return DialogueCancel(Num(O, TEXT("dialogue"))); });
		T.Add(TEXT("dialogue_close"), [](const FObj& O) { return DialogueClose(Num(O, TEXT("dialogue"))); });
		T.Add(TEXT("dialogue_snapshot"), [](const FObj& O) { return DialogueSnapshot(Num(O, TEXT("dialogue"))); });
		T.Add(TEXT("dialogue_load"), [](const FObj& O) { return DialogueLoad(Str(O->GetObjectField(TEXT("source")), TEXT("ir_json")), LingaraSpec::Serialize(O->GetObjectField(TEXT("session")))); });
		T.Add(TEXT("dialogue_set_fact"), [](const FObj& O) { return DialogueSetFactNumber(Num(O, TEXT("dialogue")), Str(O, TEXT("key")), O->GetNumberField(TEXT("value"))); });
		T.Add(TEXT("dialogue_validate"), [](const FObj& O) { return DialogueValidate(Str(O, TEXT("ir_json"))); });
		return T;
	}
}

BEGIN_DEFINE_SPEC(FLingaraCommandEncodeSpec, "LingaraEmbed.CommandEncode",
	EAutomationTestFlags::EngineFilter | EAutomationTestFlags_ApplicationContextMask)
	TSharedPtr<FJsonObject> Shape;
	FString ShapePath;
END_DEFINE_SPEC(FLingaraCommandEncodeSpec)

void FLingaraCommandEncodeSpec::Define()
{
	BeforeEach([this]()
	{
		Shape = LingaraSpec::LoadJsonShape(ShapePath);
		TestTrue(FString::Printf(TEXT("json-shape.json is readable at %s"), *ShapePath), Shape.IsValid());
	});

	// 1.10.26ac AC13
	It("every command example equals its encoded method", [this]()
	{
		const TMap<FString, FEncode> Encoders = Table();
		const TArray<TSharedPtr<FJsonObject>> Commands = LingaraSpec::Examples(Shape, TEXT("Command"));
		TestEqual(TEXT("K1's 22 commands"), Commands.Num(), 22);
		for (const TSharedPtr<FJsonObject>& Example : Commands)
		{
			const FString Type = Example->GetStringField(TEXT("type"));
			const FEncode* Encode = Encoders.Find(Type);
			if (!TestNotNull(FString::Printf(TEXT("an encoder for %s"), *Type), Encode))
			{
				continue;
			}
			TSharedPtr<FJsonValue> Written;
			FJsonSerializer::Deserialize(TJsonReaderFactory<TCHAR>::Create((*Encode)(Example)), Written);
			const TSharedRef<FJsonValue> Expected = MakeShared<FJsonValueObject>(Example);
			TestTrue(FString::Printf(TEXT("%s is JSON-equal to the example"), *Type), Written.IsValid() && FJsonValue::CompareEqual(*Written, *Expected));
		}
	});
}

#endif
