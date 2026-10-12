// Lingara Say (ADR 1.10.26ac D6).
#include "LingaraSayAsyncAction.h"

#include "Engine/Engine.h"
#include "Engine/GameInstance.h"
#include "LingaraEmbedSubsystem.h"

namespace
{
	FLingaraEmbedError ClosedError(const TCHAR* Message)
	{
		FLingaraEmbedError Error;
		Error.Kind = ELingaraErrorKind::Closed;
		Error.KindName = TEXT("closed");
		Error.Message = Message;
		return Error;
	}
}

ULingaraSayAsyncAction* ULingaraSayAsyncAction::LingaraSay(UObject* WorldContextObject, int64 Dialogue, const FString& Line)
{
	ULingaraSayAsyncAction* Action = NewObject<ULingaraSayAsyncAction>();
	const UWorld* World = GEngine ? GEngine->GetWorldFromContextObject(WorldContextObject, EGetWorldErrorMode::LogAndReturnNull) : nullptr;
	const UGameInstance* GameInstance = World ? World->GetGameInstance() : nullptr;
	Action->Subsystem = GameInstance ? GameInstance->GetSubsystem<ULingaraEmbedSubsystem>() : nullptr;
	Action->Dialogue = Dialogue;
	Action->Line = Line;
	Action->RegisterWithGameInstance(WorldContextObject);
	return Action;
}

void ULingaraSayAsyncAction::Activate()
{
	if (Subsystem == nullptr)
	{
		Fail(ClosedError(TEXT("Lingara Say: no Lingara Embed subsystem")));
		return;
	}
	ELingaraCallResult Result;
	FLingaraEmbedError Error;
	Request = Subsystem->DialogueSay(Dialogue, Line, Result, Error);
	if (Result == ELingaraCallResult::Refused)
	{
		Fail(Error);
		return;
	}
	Subsystem->OnNpcTurn.AddDynamic(this, &ULingaraSayAsyncAction::HandleNpcTurn);
	Subsystem->OnError.AddDynamic(this, &ULingaraSayAsyncAction::HandleError);
	Subsystem->OnStateChanged.AddDynamic(this, &ULingaraSayAsyncAction::HandleStateChanged);
}

void ULingaraSayAsyncAction::HandleNpcTurn(const FLingaraNpcTurn& Turn)
{
	if (bEnded || Turn.Request != Request)
	{
		return;
	}
	switch (Turn.Update.Type)
	{
	case ELingaraNpcTurnUpdateType::Fragment: OnFragment.Broadcast(Turn.Update.Text); break;
	case ELingaraNpcTurnUpdateType::Notice: OnNotice.Broadcast(Turn.Update.Code, Turn.Update.Message); break;
	case ELingaraNpcTurnUpdateType::Finished: OnFinished.Broadcast(Turn.Update.Text); End(); break;
	case ELingaraNpcTurnUpdateType::Cancelled: Fail(ClosedError(TEXT("the turn was cancelled"))); break;
	default: break;
	}
}

void ULingaraSayAsyncAction::HandleError(const FLingaraEmbedError& Error)
{
	if (!bEnded && Error.Request == Request)
	{
		Fail(Error);
	}
}

void ULingaraSayAsyncAction::HandleStateChanged(ELingaraEmbedState State, const FString& Reason)
{
	if (!bEnded && State != ELingaraEmbedState::Running)
	{
		Fail(ClosedError(TEXT("Lingara stopped with the line in flight")));
	}
}

void ULingaraSayAsyncAction::Fail(const FLingaraEmbedError& Error)
{
	if (!bEnded)
	{
		OnFailed.Broadcast(Error);
		End();
	}
}

void ULingaraSayAsyncAction::End()
{
	bEnded = true;
	if (Subsystem != nullptr)
	{
		Subsystem->OnNpcTurn.RemoveAll(this);
		Subsystem->OnError.RemoveAll(this);
		Subsystem->OnStateChanged.RemoveAll(this);
	}
	SetReadyToDestroy();
}
