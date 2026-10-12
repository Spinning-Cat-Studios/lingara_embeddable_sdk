// K1's nested shapes (ADR 1.10.26ac D5). Word tables are in each enum's
// declaration order; a word K1 adds later reads as the enum's Unknown.
#include "LingaraJsonShapes.h"

namespace LingaraJsonShapes
{
	namespace
	{
		template <typename E, int32 N>
		E FromWord(const FString& Word, const TCHAR* const (&Words)[N], E Fallback)
		{
			for (int32 Index = 0; Index < N; ++Index)
			{
				if (Word == Words[Index])
				{
					return static_cast<E>(Index);
				}
			}
			return Fallback;
		}

		ELingaraPracticePhase PracticePhase(const FString& Word)
		{
			static const TCHAR* const Words[] = {
				TEXT("requesting"), TEXT("generating"), TEXT("creating"), TEXT("loading"), TEXT("unanswered"),
				TEXT("awaiting_self_grade"), TEXT("reviewed"), TEXT("set_complete"), TEXT("reporting"),
				TEXT("reported"), TEXT("report_failed"), TEXT("failed"), TEXT("ended")};
			return FromWord(Word, Words, ELingaraPracticePhase::Unknown);
		}

		ELingaraPracticeOutcome PracticeOutcome(const FString& Word)
		{
			static const TCHAR* const Words[] = {
				TEXT("<none>"), TEXT("correct"), TEXT("incorrect"), TEXT("revealed"), TEXT("self_correct"),
				TEXT("self_incorrect"), TEXT("skipped")};
			return FromWord(Word, Words, ELingaraPracticeOutcome::Unknown);
		}

		ELingaraDialoguePhase DialoguePhase(const FString& Word)
		{
			static const TCHAR* const Words[] = {TEXT("idle"), TEXT("sending"), TEXT("streaming")};
			return FromWord(Word, Words, ELingaraDialoguePhase::Unknown);
		}

		int32 Count(const FObjectPtr& O, const TCHAR* Key)
		{
			int32 Value = 0;
			O->TryGetNumberField(Key, Value);
			return Value;
		}

		void ReadPlan(const FObjectPtr& O, FLingaraPlanSummary& Out)
		{
			O->TryGetStringField(TEXT("id"), Out.Id);
			Out.Title = OptString(O, TEXT("title"));
			Out.Level = static_cast<uint8>(Count(O, TEXT("level")));
			O->TryGetStringField(TEXT("source_lang"), Out.SourceLang);
			O->TryGetStringField(TEXT("target_lang"), Out.TargetLang);
			const TArray<TSharedPtr<FJsonValue>>* Words = nullptr;
			if (O->TryGetArrayField(TEXT("vocabulary"), Words))
			{
				for (const TSharedPtr<FJsonValue>& Value : *Words)
				{
					const FObjectPtr W = Value->AsObject();
					FLingaraPlanWord& Word = Out.Vocabulary.AddDefaulted_GetRef();
					W->TryGetStringField(TEXT("word"), Word.Word);
					W->TryGetStringField(TEXT("translation"), Word.Translation);
					Word.Pronunciation = OptString(W, TEXT("pronunciation"));
				}
			}
		}

		void ReadQuestion(const FObjectPtr& O, FLingaraPracticeQuestion& Out)
		{
			O->TryGetStringField(TEXT("kind"), Out.Kind);
			O->TryGetStringField(TEXT("prompt"), Out.Prompt);
			O->TryGetStringArrayField(TEXT("options"), Out.Options);
			Out.Hint = OptString(O, TEXT("hint"));
			O->TryGetBoolField(TEXT("self_graded"), Out.bSelfGraded);
		}

		void ReadAnswerResult(const FObjectPtr& O, FLingaraAnswerResult& Out)
		{
			const FString Outcome = OptString(O, TEXT("outcome"));
			Out.Outcome = Outcome.IsEmpty() ? ELingaraPracticeOutcome::None : PracticeOutcome(Outcome);
			O->TryGetStringField(TEXT("expected"), Out.Expected);
			O->TryGetStringField(TEXT("explanation"), Out.Explanation);
		}
	}

	bool ReadId(const FObjectPtr& O, const TCHAR* Key, int64& Out)
	{
		return O.IsValid() && O->TryGetNumberField(Key, Out);
	}

	FString OptString(const FObjectPtr& O, const TCHAR* Key)
	{
		FString Value;
		O->TryGetStringField(Key, Value);
		return Value;
	}

	FObjectPtr OptObject(const FObjectPtr& O, const TCHAR* Key)
	{
		const TSharedPtr<FJsonValue> Value = O->TryGetField(Key);
		return Value.IsValid() && Value->Type == EJson::Object ? Value->AsObject() : nullptr;
	}

	bool ReadError(const FObjectPtr& O, FLingaraEmbedError& Out)
	{
		if (!O.IsValid() || !O->TryGetStringField(TEXT("kind"), Out.KindName) || !O->TryGetStringField(TEXT("message"), Out.Message))
		{
			return false;
		}
		Out.Kind = ErrorKind(Out.KindName);
		O->TryGetBoolField(TEXT("retryable"), Out.bRetryable);
		O->TryGetNumberField(TEXT("status"), Out.Status);
		Out.Code = OptString(O, TEXT("code"));
		O->TryGetNumberField(TEXT("retry_after_ms"), Out.RetryAfterMs);
		O->TryGetNumberField(TEXT("request"), Out.Request);
		return true;
	}

	bool ReadStreamState(const FObjectPtr& O, FLingaraStreamState& Out)
	{
		static const TCHAR* const Words[] = {TEXT("connecting"), TEXT("live"), TEXT("paused"), TEXT("down")};
		FString Type;
		if (!O.IsValid() || !O->TryGetStringField(TEXT("type"), Type))
		{
			return false;
		}
		Out.Type = FromWord(Type, Words, ELingaraStreamStateType::Unknown);
		if (Out.Type == ELingaraStreamStateType::Down)
		{
			O->TryGetNumberField(TEXT("retry_in_ms"), Out.RetryInMs);
			return ReadError(OptObject(O, TEXT("error")), Out.Error);
		}
		return true;
	}

	bool ReadNpc(const FObjectPtr& O, FLingaraNpc& Out)
	{
		Out.Persona = O.IsValid() ? OptString(O, TEXT("persona")) : FString();
		return O.IsValid() && O->TryGetStringField(TEXT("name"), Out.Name);
	}

	bool ReadWindow(const FObjectPtr& O, TArray<FLingaraWindowEntry>& Out)
	{
		const TArray<TSharedPtr<FJsonValue>>* Entries = nullptr;
		if (!O->TryGetArrayField(TEXT("window"), Entries))
		{
			return false;
		}
		for (const TSharedPtr<FJsonValue>& Value : *Entries)
		{
			const FObjectPtr E = Value->AsObject();
			FLingaraWindowEntry& Entry = Out.AddDefaulted_GetRef();
			Entry.Speaker = OptString(E, TEXT("speaker")) == TEXT("npc") ? ELingaraSpeaker::Npc : ELingaraSpeaker::Player;
			E->TryGetStringField(TEXT("text"), Entry.Text);
		}
		return true;
	}

	bool ReadPracticeSnapshot(const FObjectPtr& O, FLingaraPracticeSnapshot& Out)
	{
		FString Phase;
		if (!O.IsValid() || !O->TryGetStringField(TEXT("phase"), Phase))
		{
			return false;
		}
		Out.Phase = PracticePhase(Phase);
		Out.SetIndex = Count(O, TEXT("set_index"));
		Out.SetCount = Count(O, TEXT("set_count"));
		Out.QuestionIndex = Count(O, TEXT("question_index"));
		Out.QuestionCount = Count(O, TEXT("question_count"));
		if (const FObjectPtr Plan = OptObject(O, TEXT("plan"))) { Out.bHasPlan = true; ReadPlan(Plan, Out.Plan); }
		if (const FObjectPtr Q = OptObject(O, TEXT("question"))) { Out.bHasQuestion = true; ReadQuestion(Q, Out.Question); }
		if (const FObjectPtr R = OptObject(O, TEXT("last_result"))) { Out.bHasLastResult = true; ReadAnswerResult(R, Out.LastResult); }
		if (const FObjectPtr F = OptObject(O, TEXT("failure"))) { Out.bHasFailure = ReadError(F, Out.Failure); }
		return true;
	}

	bool ReadDialogueSnapshot(const FObjectPtr& O, FLingaraDialogueSnapshot& Out)
	{
		FString Phase;
		if (!O.IsValid() || !O->TryGetStringField(TEXT("phase"), Phase) || !ReadNpc(OptObject(O, TEXT("npc")), Out.Npc))
		{
			return false;
		}
		Out.Phase = DialoguePhase(Phase);
		Out.Scene = OptString(O, TEXT("scene"));
		Out.Level = static_cast<uint8>(Count(O, TEXT("level")));
		return ReadWindow(O, Out.Window);
	}

	bool ReadLingaraEvent(const FObjectPtr& O, FLingaraEvent& Out)
	{
		static const TCHAR* const Details[] = {TEXT("lesson_plan_ready"), TEXT("lesson_plan_failed"), TEXT("other")};
		const FObjectPtr D = O.IsValid() ? OptObject(O, TEXT("detail")) : nullptr;
		if (!D.IsValid() || !O->TryGetStringField(TEXT("id"), Out.Id) || !O->TryGetStringField(TEXT("event_type"), Out.EventType))
		{
			return false;
		}
		O->TryGetStringField(TEXT("created_at"), Out.CreatedAt);
		O->TryGetStringField(TEXT("data_json"), Out.DataJson);
		Out.Detail.Type = FromWord(OptString(D, TEXT("type")), Details, ELingaraEventDetailType::Other);
		Out.Detail.PlanId = OptString(D, TEXT("plan_id"));
		Out.Detail.Title = OptString(D, TEXT("title"));
		Out.Detail.Level = static_cast<uint8>(Count(D, TEXT("level")));
		Out.Detail.SourceLang = OptString(D, TEXT("source_lang"));
		Out.Detail.TargetLang = OptString(D, TEXT("target_lang"));
		D->TryGetBoolField(TEXT("complete"), Out.Detail.bComplete);
		Out.Detail.Reason = OptString(D, TEXT("reason"));
		return true;
	}

	bool ReadReaction(const FObjectPtr& O, FLingaraReaction& Out)
	{
		if (!O->TryGetStringField(TEXT("status"), Out.Status))
		{
			return false;
		}
		Out.PlanId = OptString(O, TEXT("plan_id"));
		Out.PlanStatus = OptString(O, TEXT("plan_status"));
		Out.Code = OptString(O, TEXT("code"));
		Out.Error = OptString(O, TEXT("error"));
		return true;
	}

	bool ReadNpcTurnUpdate(const FObjectPtr& O, FLingaraNpcTurnUpdate& Out)
	{
		static const TCHAR* const Words[] = {TEXT("opened"), TEXT("fragment"), TEXT("notice"), TEXT("finished"), TEXT("cancelled")};
		FString Type;
		if (!O.IsValid() || !O->TryGetStringField(TEXT("type"), Type))
		{
			return false;
		}
		Out.Type = FromWord(Type, Words, ELingaraNpcTurnUpdateType::Unknown);
		Out.Text = OptString(O, TEXT("text"));
		Out.Code = OptString(O, TEXT("code"));
		Out.Message = OptString(O, TEXT("message"));
		return true;
	}

	ELingaraErrorKind ErrorKind(const FString& Word)
	{
		static const TCHAR* const Words[] = {
			TEXT("config"), TEXT("invalid_input"), TEXT("token_unavailable"), TEXT("token_rejected"), TEXT("forbidden"),
			TEXT("billing"), TEXT("not_found"), TEXT("rate_limited"), TEXT("unavailable"), TEXT("network"), TEXT("busy"),
			TEXT("invalid_state"), TEXT("unknown_id"), TEXT("closed"), TEXT("internal")};
		return FromWord(Word, Words, ELingaraErrorKind::Unknown);
	}

	// ELingaraLogLevel leads with CoreDefault, so K1's five words start at 1.
	ELingaraLogLevel LogLevel(const FString& Word)
	{
		static const TCHAR* const Words[] = {TEXT("<default>"), TEXT("error"), TEXT("warn"), TEXT("info"), TEXT("debug"), TEXT("trace")};
		return FromWord(Word, Words, ELingaraLogLevel::Info);
	}

	ELingaraCloseReason CloseReason(const FString& Word)
	{
		static const TCHAR* const Words[] = {TEXT("requested"), TEXT("internal")};
		return FromWord(Word, Words, ELingaraCloseReason::Unknown);
	}

	ELingaraTokenReason TokenReason(const FString& Word)
	{
		static const TCHAR* const Words[] = {TEXT("initial"), TEXT("expiring"), TEXT("rejected")};
		return FromWord(Word, Words, ELingaraTokenReason::Unknown);
	}
}
