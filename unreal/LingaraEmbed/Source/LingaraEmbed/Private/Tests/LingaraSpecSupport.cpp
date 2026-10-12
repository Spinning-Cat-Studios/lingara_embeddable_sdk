// The specs' shared vector reader (ADR 1.10.26ac D10).
#include "LingaraSpecSupport.h"

#include "HAL/PlatformMisc.h"
#include "Interfaces/IPluginManager.h"
#include "Misc/FileHelper.h"
#include "Misc/Paths.h"
#include "Policies/CondensedJsonPrintPolicy.h"
#include "Serialization/JsonReader.h"
#include "Serialization/JsonSerializer.h"
#include "Serialization/JsonWriter.h"

namespace LingaraSpec
{
	TSharedPtr<FJsonObject> LoadJsonShape(FString& OutPath)
	{
		OutPath = FPlatformMisc::GetEnvironmentVariable(TEXT("LINGARA_JSON_SHAPE"));
		if (OutPath.IsEmpty())
		{
			const TSharedPtr<IPlugin> Plugin = IPluginManager::Get().FindPlugin(TEXT("LingaraEmbed"));
			OutPath = Plugin.IsValid() ? FPaths::Combine(Plugin->GetBaseDir(), TEXT("../../conformance/vectors/json-shape.json")) : FString();
		}
		FString Text;
		TSharedPtr<FJsonObject> Shape;
		if (FFileHelper::LoadFileToString(Text, *OutPath))
		{
			FJsonSerializer::Deserialize(TJsonReaderFactory<TCHAR>::Create(Text), Shape);
		}
		return Shape;
	}

	TArray<TSharedPtr<FJsonObject>> Examples(const TSharedPtr<FJsonObject>& Shape, const TCHAR* Union)
	{
		TArray<TSharedPtr<FJsonObject>> Out;
		const TSharedPtr<FJsonObject>* Tagged = nullptr;
		const TArray<TSharedPtr<FJsonValue>>* Values = nullptr;
		if (Shape.IsValid() && Shape->TryGetObjectField(TEXT("tagged"), Tagged) && (*Tagged)->TryGetArrayField(Union, Values))
		{
			for (const TSharedPtr<FJsonValue>& Value : *Values)
			{
				Out.Add(Value->AsObject());
			}
		}
		return Out;
	}

	FString Serialize(const TSharedPtr<FJsonObject>& Object)
	{
		FString Out;
		FJsonSerializer::Serialize(Object.ToSharedRef(), TJsonWriterFactory<TCHAR, TCondensedJsonPrintPolicy<TCHAR>>::Create(&Out));
		return Out;
	}
}
