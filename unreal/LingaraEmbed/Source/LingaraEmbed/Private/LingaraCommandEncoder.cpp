// K1's Command shapes (ADR 1.10.26ac D5). Every command `type` in
// ffi/schema/messages.schema.json appears below as a string literal;
// unreal/tools/check_coverage.sh fails the PR that adds one without it.
#include "LingaraCommandEncoder.h"

#include "Policies/CondensedJsonPrintPolicy.h"
#include "Serialization/JsonWriter.h"

namespace
{
	using FWriter = TJsonWriter<TCHAR, TCondensedJsonPrintPolicy<TCHAR>>;

	// One JSON object: `{"type": <Type>, …}`. Out is declared before the
	// writer that appends to it.
	struct FObject
	{
		FString Out;
		TSharedRef<FWriter> W;

		explicit FObject(const TCHAR* Type) : W(TJsonWriterFactory<TCHAR, TCondensedJsonPrintPolicy<TCHAR>>::Create(&Out))
		{
			W->WriteObjectStart();
			if (Type != nullptr)
			{
				W->WriteValue(TEXT("type"), Type);
			}
		}

		FString Close()
		{
			W->WriteObjectEnd();
			W->Close();
			return Out;
		}
	};

	void WriteStrings(FWriter& W, const TCHAR* Key, const TArray<FString>& Values)
	{
		W.WriteArrayStart(Key);
		for (const FString& Value : Values)
		{
			W.WriteValue(Value);
		}
		W.WriteArrayEnd();
	}

	void WriteHeaders(FWriter& W, const TArray<FLingaraHeader>& Headers)
	{
		W.WriteArrayStart(TEXT("headers"));
		for (const FLingaraHeader& Header : Headers)
		{
			W.WriteObjectStart();
			W.WriteValue(TEXT("name"), Header.Name);
			W.WriteValue(TEXT("value"), Header.Value);
			W.WriteObjectEnd();
		}
		W.WriteArrayEnd();
	}

	void WriteNpc(FWriter& W, const TCHAR* Key, const FLingaraNpc& Npc)
	{
		W.WriteObjectStart(Key);
		W.WriteValue(TEXT("name"), Npc.Name);
		if (Npc.Persona.IsEmpty())
		{
			W.WriteNull(TEXT("persona"));
		}
		else
		{
			W.WriteValue(TEXT("persona"), Npc.Persona);
		}
		W.WriteObjectEnd();
	}

	void WriteLanguages(FWriter& W, const FString& Source, const FString& Target, uint8 Level)
	{
		W.WriteValue(TEXT("source_lang"), Source);
		W.WriteValue(TEXT("target_lang"), Target);
		W.WriteValue(TEXT("level"), static_cast<int32>(Level));
	}

	FString SessionCommand(const TCHAR* Type, int64 Session)
	{
		FObject O(Type);
		O.W->WriteValue(TEXT("session"), Session);
		return O.Close();
	}

	FString DialogueCommand(const TCHAR* Type, int64 Dialogue)
	{
		FObject O(Type);
		O.W->WriteValue(TEXT("dialogue"), Dialogue);
		return O.Close();
	}

	const TCHAR* LogWord(ELingaraLogLevel Level)
	{
		static const TCHAR* const Words[] = {nullptr, TEXT("error"), TEXT("warn"), TEXT("info"), TEXT("debug"), TEXT("trace")};
		return Words[static_cast<uint8>(Level)];
	}

	void WriteConfigOverrides(FWriter& W, const FLingaraEmbedConfig& C)
	{
		if (C.bOverride_StreamOnStart) W.WriteValue(TEXT("stream_on_start"), C.bStreamOnStart);
		if (!C.ResumeCursor.IsEmpty()) W.WriteValue(TEXT("resume_cursor"), C.ResumeCursor);
		if (C.EventTypes.Num() > 0) WriteStrings(W, TEXT("event_types"), C.EventTypes);
		if (!C.ApiVersion.IsEmpty()) W.WriteValue(TEXT("api_version"), C.ApiVersion);
		if (!C.EmbedVersion.IsEmpty()) W.WriteValue(TEXT("embed_version"), C.EmbedVersion);
		if (C.QueueCapacity > 0) W.WriteValue(TEXT("queue_capacity"), C.QueueCapacity);
		if (C.StreamIdleTimeoutMs > 0) W.WriteValue(TEXT("stream_idle_timeout_ms"), C.StreamIdleTimeoutMs);
		if (C.TokenWaitTimeoutMs > 0) W.WriteValue(TEXT("token_wait_timeout_ms"), C.TokenWaitTimeoutMs);
	}
}

namespace LingaraCommandEncoder
{
	FString Config(const FLingaraEmbedConfig& C, const FString& UserAgentSuffix)
	{
		FObject O(nullptr);
		if (!C.BaseUrl.IsEmpty()) O.W->WriteValue(TEXT("base_url"), C.BaseUrl);
		if (const TCHAR* Level = LogWord(C.LogLevel)) O.W->WriteValue(TEXT("log_level"), Level);
		WriteConfigOverrides(*O.W, C);
		O.W->WriteValue(TEXT("user_agent_suffix"), UserAgentSuffix);
		if (!C.TokenEndpoint.Url.IsEmpty())
		{
			O.W->WriteObjectStart(TEXT("token_source"));
			O.W->WriteValue(TEXT("type"), TEXT("http"));
			O.W->WriteValue(TEXT("url"), C.TokenEndpoint.Url);
			WriteHeaders(*O.W, C.TokenEndpoint.Headers);
			O.W->WriteObjectEnd();
		}
		return O.Close();
	}

	FString ProvideToken(const FString& Token, const FString& ExpiresAt, const FString& UpdateAvailableJson)
	{
		FObject O(TEXT("provide_token"));
		O.W->WriteValue(TEXT("token"), Token);
		O.W->WriteValue(TEXT("expires_at"), ExpiresAt);
		if (!UpdateAvailableJson.IsEmpty())
		{
			// Verbatim: the core reads it leniently, so a malformed one costs nothing.
			O.W->WriteRawJSONValue(TEXT("update_available"), UpdateAvailableJson);
		}
		return O.Close();
	}

	FString ProvideTokenFor(const FString& Token, uint32 ExpiresInSecs)
	{
		FObject O(TEXT("provide_token_for"));
		O.W->WriteValue(TEXT("token"), Token);
		O.W->WriteValue(TEXT("expires_in_secs"), static_cast<int64>(ExpiresInSecs));
		return O.Close();
	}

	FString TokenUnavailable(const FString& Reason)
	{
		FObject O(TEXT("token_unavailable"));
		O.W->WriteValue(TEXT("reason"), Reason);
		return O.Close();
	}

	FString SetTokenEndpoint(const FString& Url, const TArray<FLingaraHeader>& Headers)
	{
		FObject O(TEXT("set_token_endpoint"));
		O.W->WriteValue(TEXT("url"), Url);
		WriteHeaders(*O.W, Headers);
		return O.Close();
	}

	FString StreamPause() { return FObject(TEXT("stream_pause")).Close(); }

	FString StreamResume() { return FObject(TEXT("stream_resume")).Close(); }

	FString SendContext(const FLingaraWorldContext& Context)
	{
		FObject O(TEXT("send_context"));
		O.W->WriteObjectStart(TEXT("context"));
		O.W->WriteValue(TEXT("scene"), Context.Scene);
		WriteLanguages(*O.W, Context.SourceLang, Context.TargetLang, Context.Level);
		if (Context.bHasNpc) WriteNpc(*O.W, TEXT("npc"), Context.Npc);
		WriteStrings(*O.W, TEXT("tags"), Context.Tags);
		O.W->WriteValue(TEXT("generate"), Context.bGenerate);
		O.W->WriteObjectEnd();
		return O.Close();
	}

	FString PracticeStart(const FLingaraPracticeSource& S)
	{
		static const TCHAR* const Types[] = {TEXT("request"), TEXT("create"), TEXT("plan")};
		FObject O(TEXT("practice_start"));
		O.W->WriteObjectStart(TEXT("source"));
		O.W->WriteValue(TEXT("type"), Types[static_cast<uint8>(S.Type)]);
		switch (S.Type)
		{
		case ELingaraPracticeSourceType::Request:
			O.W->WriteValue(TEXT("topic"), S.Topic);
			WriteLanguages(*O.W, S.SourceLang, S.TargetLang, S.Level);
			WriteStrings(*O.W, TEXT("tags"), S.Tags);
			break;
		case ELingaraPracticeSourceType::Create:
			O.W->WriteValue(TEXT("context"), S.Context);
			WriteLanguages(*O.W, S.SourceLang, S.TargetLang, S.Level);
			break;
		case ELingaraPracticeSourceType::Plan:
			O.W->WriteValue(TEXT("plan_id"), S.PlanId);
			O.W->WriteValue(TEXT("report"), S.bReport);
			break;
		}
		O.W->WriteObjectEnd();
		return O.Close();
	}

	FString PracticeSnapshot(int64 Session) { return SessionCommand(TEXT("practice_snapshot"), Session); }

	FString PracticeAnswer(int64 Session, const FString& Answer)
	{
		FObject O(TEXT("practice_answer"));
		O.W->WriteValue(TEXT("session"), Session);
		O.W->WriteValue(TEXT("answer"), Answer);
		return O.Close();
	}

	FString PracticeChoose(int64 Session, uint32 Option)
	{
		FObject O(TEXT("practice_choose"));
		O.W->WriteValue(TEXT("session"), Session);
		O.W->WriteValue(TEXT("option"), static_cast<int64>(Option));
		return O.Close();
	}

	FString PracticeReveal(int64 Session) { return SessionCommand(TEXT("practice_reveal"), Session); }

	FString PracticeSelfGrade(int64 Session, bool bCorrect)
	{
		FObject O(TEXT("practice_self_grade"));
		O.W->WriteValue(TEXT("session"), Session);
		O.W->WriteValue(TEXT("correct"), bCorrect);
		return O.Close();
	}

	FString PracticeNext(int64 Session) { return SessionCommand(TEXT("practice_next"), Session); }

	FString PracticeFinish(int64 Session) { return SessionCommand(TEXT("practice_finish"), Session); }

	FString DialogueOpen(const FLingaraDialogueConfig& C)
	{
		FObject O(TEXT("dialogue_open"));
		O.W->WriteObjectStart(TEXT("config"));
		WriteNpc(*O.W, TEXT("npc"), C.Npc);
		if (C.Scene.IsEmpty()) O.W->WriteNull(TEXT("scene"));
		else O.W->WriteValue(TEXT("scene"), C.Scene);
		WriteLanguages(*O.W, C.SourceLang, C.TargetLang, C.Level);
		O.W->WriteArrayStart(TEXT("window"));
		for (const FLingaraWindowEntry& Entry : C.Window)
		{
			O.W->WriteObjectStart();
			O.W->WriteValue(TEXT("speaker"), Entry.Speaker == ELingaraSpeaker::Npc ? TEXT("npc") : TEXT("player"));
			O.W->WriteValue(TEXT("text"), Entry.Text);
			O.W->WriteObjectEnd();
		}
		O.W->WriteArrayEnd();
		O.W->WriteObjectEnd();
		return O.Close();
	}

	FString DialogueSay(int64 Dialogue, const FString& Line)
	{
		FObject O(TEXT("dialogue_say"));
		O.W->WriteValue(TEXT("dialogue"), Dialogue);
		O.W->WriteValue(TEXT("line"), Line);
		return O.Close();
	}

	FString DialogueSetLevel(int64 Dialogue, uint8 Level)
	{
		FObject O(TEXT("dialogue_set_level"));
		O.W->WriteValue(TEXT("dialogue"), Dialogue);
		O.W->WriteValue(TEXT("level"), static_cast<int32>(Level));
		return O.Close();
	}

	FString DialogueSetScene(int64 Dialogue, const FString& Scene)
	{
		FObject O(TEXT("dialogue_set_scene"));
		O.W->WriteValue(TEXT("dialogue"), Dialogue);
		if (Scene.IsEmpty()) O.W->WriteNull(TEXT("scene"));
		else O.W->WriteValue(TEXT("scene"), Scene);
		return O.Close();
	}

	FString DialogueCancel(int64 Dialogue) { return DialogueCommand(TEXT("dialogue_cancel"), Dialogue); }

	FString DialogueClose(int64 Dialogue) { return DialogueCommand(TEXT("dialogue_close"), Dialogue); }

	FString DialogueSnapshot(int64 Dialogue) { return DialogueCommand(TEXT("dialogue_snapshot"), Dialogue); }

	FString DialogueLoad(const FString& IrJson, const FString& SessionJson)
	{
		FObject O(TEXT("dialogue_load"));
		O.W->WriteObjectStart(TEXT("source"));
		O.W->WriteValue(TEXT("type"), TEXT("ir"));
		O.W->WriteValue(TEXT("ir_json"), IrJson);
		O.W->WriteObjectEnd();
		O.W->WriteRawJSONValue(TEXT("session"), SessionJson);
		return O.Close();
	}

	FString DialogueLoadStored(const FString& DialogueId, const FString& SessionJson)
	{
		FObject O(TEXT("dialogue_load"));
		O.W->WriteObjectStart(TEXT("source"));
		O.W->WriteValue(TEXT("type"), TEXT("stored"));
		O.W->WriteValue(TEXT("dialogue_id"), DialogueId);
		O.W->WriteObjectEnd();
		O.W->WriteRawJSONValue(TEXT("session"), SessionJson);
		return O.Close();
	}

	FString DialogueSetFactBool(int64 Dialogue, const FString& Key, bool bValue)
	{
		FObject O(TEXT("dialogue_set_fact"));
		O.W->WriteValue(TEXT("dialogue"), Dialogue);
		O.W->WriteValue(TEXT("key"), Key);
		O.W->WriteValue(TEXT("value"), bValue);
		return O.Close();
	}

	FString DialogueSetFactNumber(int64 Dialogue, const FString& Key, double Value)
	{
		FObject O(TEXT("dialogue_set_fact"));
		O.W->WriteValue(TEXT("dialogue"), Dialogue);
		O.W->WriteValue(TEXT("key"), Key);
		O.W->WriteValue(TEXT("value"), Value);
		return O.Close();
	}

	FString DialogueSetFactText(int64 Dialogue, const FString& Key, const FString& Value)
	{
		FObject O(TEXT("dialogue_set_fact"));
		O.W->WriteValue(TEXT("dialogue"), Dialogue);
		O.W->WriteValue(TEXT("key"), Key);
		O.W->WriteValue(TEXT("value"), Value);
		return O.Close();
	}

	FString DialogueValidate(const FString& IrJson)
	{
		FObject O(TEXT("dialogue_validate"));
		O.W->WriteValue(TEXT("ir_json"), IrJson);
		return O.Close();
	}
}
