// Project Settings → Plugins → Lingara Embed (ADR 1.10.26ac D7).
//
// There is no client id, client secret or token here, and there never will
// be: anything in DefaultGame.ini ships inside the game. A native embed token
// is minted by your own server, without `origin`, and fetched at runtime.
// unreal/tools/check_plugin.sh fails this header if a property is ever named
// like one.
#pragma once

#include "CoreMinimal.h"
#include "Engine/DeveloperSettings.h"
#include "LingaraEmbedTypes.h"

#include "LingaraEmbedSettings.generated.h"

UCLASS(config = Game, defaultconfig, meta = (DisplayName = "Lingara Embed"))
class LINGARAEMBED_API ULingaraEmbedSettings : public UDeveloperSettings
{
	GENERATED_BODY()

public:
	ULingaraEmbedSettings();

	// Empty: the core's own default applies, and is never restated here.
	UPROPERTY(config, EditAnywhere, Category = "Lingara")
	FString BaseUrl;

	// Start the connection when the game instance starts.
	UPROPERTY(config, EditAnywhere, Category = "Lingara")
	bool bAutoStart = false;

	UPROPERTY(config, EditAnywhere, Category = "Lingara")
	ELingaraLogLevel LogLevel = ELingaraLogLevel::CoreDefault;

	// The platform embed version this build was registered against, e.g. 1.1.0
	// (ADR 30.10.26p). Sent to your token endpoint as Lingara-Embed-Version.
	UPROPERTY(config, EditAnywhere, Category = "Lingara")
	FString EmbedVersion;

	// A flood carries over to the next frame rather than stalling this one.
	UPROPERTY(config, EditAnywhere, Category = "Lingara", meta = (ClampMin = "1"))
	int32 MaxEventsPerTick = 256;

	// The settings as the config `Start` takes.
	FLingaraEmbedConfig ToConfig() const;
};
